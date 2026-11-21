// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `vis test --timeout <ms>` áp cho **fixture** `.out/.err` (mở rộng ST-105).
//!
//! Khoá hành vi:
//! - Fixture chạy quá hạn ⇒ **kill** tiến trình con, `not ok` + dòng
//!   `# <tên>: quá hạn <ms> ms`, đếm là đỏ, exit **1**; không `--fail-fast` thì chạy ca kế.
//! - `--json` mang **cùng** thông điệp `quá hạn <ms> ms`; TAP/plan/exit giữ đúng khuôn cũ.
//! - Không `--timeout` ⇒ **không** giới hạn như trước; fixture nhanh không bị đổi byte.
//! - `--list` **không chạy** ca; `--repeat` áp hạn **mỗi lần lặp**; `--fail-fast` dừng sau
//!   lần đỏ đầu.
//!
//! **RED-BEFORE**: trên HEAD `4babfb0` `--timeout` chỉ áp `test fn`, fixture vẫn dùng
//! `Command::output()` **không hạn**. Vì fixture treo vô hạn sẽ làm treo cả tiến trình test,
//! các ca ở đây dùng fixture **treo có hạn** (`test-child sleep 2`) để chạy trước khi sửa:
//! mỗi ca timeout đỏ vì nhận `ok`/exit 0 sau ~2 s thay vì `quá hạn`; riêng ca treo vô hạn có
//! **guard** phía test (kill + `pkill`) và đỏ với thông điệp “không trả trong 5 s”.
//!
//! Fixture `test-child` lấy từ PATH do test đặt (cùng cách `vis_test.rs`).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

fn tam(tag: &str) -> PathBuf {
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!(
        "vis-vd270-timeout-{tag}-{}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    d
}

fn ghi(d: &Path, ten: &str, src: &str) {
    std::fs::write(d.join(ten), src).expect("ghi nguồn");
}

/// Ghi `.out` kèm header SPDX (như fixture của bộ `test/sem`).
fn ghi_out(d: &Path, ten: &str, body: &str) {
    std::fs::write(
        d.join(ten),
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{body}"),
    )
    .expect("ghi .out");
}

/// PATH có thêm thư mục chứa `test-child` — fixture treo có hạn gọi `test-child` trần.
fn path_voi_tc() -> std::ffi::OsString {
    let tc = std::path::Path::new(env!("CARGO_BIN_EXE_test-child"));
    let mut muc: Vec<PathBuf> = vec![tc.parent().expect("test-child parent").to_path_buf()];
    let cur = std::env::var_os("PATH").unwrap_or_default();
    muc.extend(std::env::split_paths(&cur));
    std::env::join_paths(muc).expect("join PATH")
}

fn vis(args: &[&str]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .env("PATH", path_voi_tc())
        .args(args)
        .output()
        .expect("chạy vis");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Dọn tiến trình con còn sống nếu guard phải kill `vis` **trước khi** bản sửa chạy:
/// Unix dùng `pkill -f <marker>` (marker = thư mục tạm duy nhất), Windows dùng
/// `taskkill /T`. Không có công cụ thì bỏ qua — chỉ ảnh hưởng lần chạy đỏ-trước.
fn don_con(_pid: u32, marker: &str) {
    #[cfg(unix)]
    {
        let _ = Command::new("pkill").args(["-KILL", "-f", marker]).output();
    }
    #[cfg(windows)]
    {
        // `marker` chỉ dùng ở nhánh Unix (`pkill -f`); trên Windows dọn theo PID.
        // Giữ tham số cho cùng chữ ký gọi hai nền mà không để clippy `-D warnings` báo unused.
        let _ = marker;
        let _ = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &_pid.to_string()])
            .output();
    }
}

/// Chạy `vis` có **guard** phía test: quá `han_ms` mà chưa thoát ⇒ kill (và dọn cây con),
/// trả `None`. Dùng cho fixture treo **vô hạn**: trước bản sửa, lệnh `vis test` sẽ không bao
/// giờ tự trả, nên guard giữ cho lần chạy đỏ-trước không treo cả suite.
fn vis_co_han(args: &[&str], han_ms: u64, marker: &str) -> Option<(i32, String, String)> {
    let mut con = Command::new(env!("CARGO_BIN_EXE_vis"))
        .env("PATH", path_voi_tc())
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn vis");
    let pid = con.id();
    let bat = Instant::now();
    loop {
        match con.try_wait().expect("try_wait") {
            Some(_) => {
                let o = con.wait_with_output().expect("gom output");
                return Some((
                    o.status.code().unwrap_or(-1),
                    String::from_utf8_lossy(&o.stdout).to_string(),
                    String::from_utf8_lossy(&o.stderr).to_string(),
                ));
            }
            None if bat.elapsed() >= Duration::from_millis(han_ms) => {
                let _ = con.kill();
                let _ = con.wait();
                don_con(pid, marker);
                return None;
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    }
}

fn dong(s: &str) -> Vec<&str> {
    s.lines().collect()
}

/// TAP hợp lệ: dòng đầu `TAP version 13`, dòng kế là plan, plan **bằng** số dòng test.
fn kiem_tap(tap: &str) -> Vec<&str> {
    let ls = dong(tap);
    assert_eq!(ls.first().copied(), Some("TAP version 13"), "TAP: {tap:?}");
    let plan: usize = ls
        .get(1)
        .and_then(|l| l.strip_prefix("1.."))
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("thiếu plan `1..N`: {tap:?}"));
    let so_test = ls
        .iter()
        .filter(|l| l.starts_with("ok ") || l.starts_with("not ok "))
        .count();
    assert_eq!(
        plan, so_test,
        "plan {plan} ≠ số dòng test {so_test}: {tap:?}"
    );
    ls
}

/// Fixture **treo có hạn** (~2 s): đủ để chứng minh `--timeout 50` phải cắt ngang, nhưng
/// không làm treo suite khi chạy đỏ-trước (timeout bị bỏ qua thì ca chỉ chậm ~2 s).
const TREO_CO_HAN: &str =
    "# SPDX-License-Identifier: MIT OR Apache-2.0\ntest-child sleep 2\necho xong\n";

/// Fixture treo **vô hạn** — chỉ dùng qua [`vis_co_han`] để guard kill nếu bản sửa vắng mặt.
const TREO_THAT: &str = "var i = 0\nwhile true:\n    i = i + 1\n";

const NHANH: &str = "print(2)\n";

/// (1) Fixture nhanh (`.out` + `.err`) + `--timeout` lớn ⇒ TAP **nguyên byte** như không cờ,
/// exit 0 — áp hạn cho fixture không được đổi kết quả ca bình thường.
#[test]
fn fixture_thuong_khong_bi_anh_huong() {
    let d = tam("thuong");
    ghi(&d, "ok.vis", "print(\"chao\")\n");
    ghi_out(&d, "ok.out", "chao\n");
    ghi(&d, "loi.vis", "let x: int = \"a\"\n");
    std::fs::write(d.join("loi.err"), "VS0412\n").expect("ghi .err");
    let ds = d.to_str().unwrap();

    let (c0, o0, e0) = vis(&["test", ds]);
    assert_eq!(c0, 0, "baseline: {o0} {e0}");

    let (c1, o1, e1) = vis(&["test", "--timeout", "5000", ds]);
    assert_eq!(c1, 0, "--timeout 5000: {o1} {e1}");
    assert_eq!(
        o1, o0,
        "--timeout không được đổi byte TAP của fixture thường"
    );
    assert!(!o1.contains("quá hạn"), "{o1}");
    kiem_tap(&o1);

    let (_, oj, ej) = vis(&["test", "--timeout", "5000", "--json", ds]);
    assert!(!oj.contains("quá hạn"), "--json: {oj} {ej}");

    let _ = std::fs::remove_dir_all(&d);
}

/// (2) Fixture treo + `--timeout 50` ⇒ `not ok` + `# <tên>: quá hạn 50 ms`, exit 1; ca kế
/// **vẫn chạy**. Fixture thứ hai xanh để chứng minh “tiếp tục sau ca treo”.
#[test]
fn fixture_treo_bi_giet_va_tiep_tuc() {
    let d = tam("treo");
    ghi(&d, "a_treo.vish", TREO_CO_HAN);
    ghi_out(&d, "a_treo.out", "[]\nxong\n");
    ghi(&d, "b_nhanh.vis", NHANH);
    ghi_out(&d, "b_nhanh.out", "2\n");
    let ds = d.to_str().unwrap();

    let (c, o, e) = vis(&["test", "--timeout", "50", ds]);
    assert_eq!(c, 1, "phải exit 1: {o} {e}");
    let ls = kiem_tap(&o);
    assert_eq!(ls[1], "1..2", "{o}");
    assert_eq!(ls[2], "not ok 1 - a_treo.vish", "{o}");
    assert_eq!(ls[3], "# a_treo.vish: quá hạn 50 ms", "{o}");
    assert!(
        o.contains("ok 2 - b_nhanh.vis"),
        "ca kế phải chạy tiếp: {o}"
    );

    let _ = std::fs::remove_dir_all(&d);
}

/// (3) Fixture **treo vô hạn** ⇒ vẫn phải bị kill và trả trong hạn guard (5 s). Ca này là
/// bằng chứng “kill tiến trình con”, không chỉ là “kết quả đỏ”.
#[test]
fn fixture_treo_that_su_bi_giet() {
    let d = tam("treo-that");
    ghi(&d, "treo.vis", TREO_THAT);
    let ds = d.to_str().unwrap();

    let r = vis_co_han(&["test", "--timeout", "50", ds], 5_000, ds);
    let Some((c, o, e)) = r else {
        panic!("`vis test` không trả trong 5 s — `--timeout` chưa áp fixture treo vô hạn");
    };
    assert_eq!(c, 1, "phải exit 1: {o} {e}");
    assert!(o.contains("not ok 1 - treo.vis"), "{o}");
    assert!(o.contains("# treo.vis: quá hạn 50 ms"), "{o}");

    let _ = std::fs::remove_dir_all(&d);
}

/// (4) `--vm` cũng áp hạn cho fixture (không chỉ đường interp).
#[test]
fn fixture_treo_vm_cung_bi_giet() {
    let d = tam("vm");
    ghi(&d, "a_treo.vish", TREO_CO_HAN);
    ghi_out(&d, "a_treo.out", "[]\nxong\n");
    let ds = d.to_str().unwrap();

    let (c, o, e) = vis(&["test", "--vm", "--timeout", "50", ds]);
    assert_eq!(c, 1, "--vm phải exit 1: {o} {e}");
    assert!(o.contains("not ok 1 - a_treo.vish"), "{o}");
    assert!(o.contains("# a_treo.vish: quá hạn 50 ms"), "{o}");

    let _ = std::fs::remove_dir_all(&d);
}

/// (5) `--json`: `message` mang **đúng** câu `quá hạn <ms> ms`; failed=1, name là đường dẫn
/// hiển thị của fixture.
#[test]
fn json_message_qua_han() {
    let d = tam("json");
    ghi(&d, "a_treo.vish", TREO_CO_HAN);
    ghi_out(&d, "a_treo.out", "[]\nxong\n");
    let ds = d.to_str().unwrap();

    let (c, o, e) = vis(&["test", "--timeout", "50", "--json", ds]);
    assert_eq!(c, 1, "phải exit 1: {o} {e}");
    assert_eq!(
        o,
        "{\"total\":1,\"ok\":0,\"failed\":1,\"cases\":[{\"name\":\"a_treo.vish\",\"ok\":false,\"message\":\"quá hạn 50 ms\"}]}\n"
    );

    let _ = std::fs::remove_dir_all(&d);
}

/// (6) `--fail-fast`: dừng **ngay** sau fixture treo đầu tiên; plan = số ca đã chạy (1),
/// ca sau không chạy.
#[test]
fn fail_fast_dung_sau_fixture_treo() {
    let d = tam("fail-fast");
    ghi(&d, "a_treo.vish", TREO_CO_HAN);
    ghi_out(&d, "a_treo.out", "[]\nxong\n");
    ghi(&d, "b_nhanh.vis", NHANH);
    ghi_out(&d, "b_nhanh.out", "2\n");
    let ds = d.to_str().unwrap();

    let (c, o, e) = vis(&["test", "--timeout", "50", "--fail-fast", ds]);
    assert_eq!(c, 1, "phải exit 1: {o} {e}");
    let ls = kiem_tap(&o);
    assert_eq!(ls[1], "1..1", "--fail-fast: plan = số ca đã chạy: {o}");
    assert_eq!(ls[2], "not ok 1 - a_treo.vish", "{o}");
    assert!(
        !o.contains("b_nhanh.vis"),
        "--fail-fast không được chạy ca sau: {o}"
    );

    let _ = std::fs::remove_dir_all(&d);
}

/// (7) `--list` không chạy gì: fixture treo vẫn được liệt kê, không `not ok`, trả nhanh.
#[test]
fn list_khong_chay_fixture() {
    let d = tam("list");
    ghi(&d, "a_treo.vish", TREO_CO_HAN);
    ghi_out(&d, "a_treo.out", "[]\nxong\n");
    ghi(&d, "b_nhanh.vis", NHANH);
    ghi_out(&d, "b_nhanh.out", "2\n");
    let ds = d.to_str().unwrap();

    let t0 = Instant::now();
    let (c, o, e) = vis(&["test", "--timeout", "50", "--list", ds]);
    let dt = t0.elapsed();
    assert_eq!(c, 0, "--list: {o} {e}");
    assert_eq!(o, "a_treo.vish\nb_nhanh.vis\n");
    assert!(!o.contains("not ok"), "{o}");
    assert!(dt.as_millis() < 2_000, "--list phải không chạy: {dt:?}");

    let _ = std::fs::remove_dir_all(&d);
}

/// (8) Fixture có `.err` (kỳ vọng lỗi) mà **treo** ⇒ hạn thắng: báo `quá hạn`, không so `.err`.
#[test]
fn fixture_treo_co_err_van_bao_qua_han() {
    let d = tam("err");
    ghi(&d, "a_treo.vish", TREO_CO_HAN);
    std::fs::write(d.join("a_treo.err"), "VS0412\n").expect("ghi .err");
    let ds = d.to_str().unwrap();

    let (c, o, e) = vis(&["test", "--timeout", "50", ds]);
    assert_eq!(c, 1, "phải exit 1: {o} {e}");
    assert!(o.contains("not ok 1 - a_treo.vish"), "{o}");
    assert!(o.contains("# a_treo.vish: quá hạn 50 ms"), "{o}");
    assert!(!o.contains("VS0412"), "hạn phải thắng kỳ vọng .err: {o}");

    let _ = std::fs::remove_dir_all(&d);
}

/// (9) `--repeat` vẫn ca-trước-lặp-sau và hạn áp **mỗi lần lặp** với fixture.
#[test]
fn repeat_ap_han_moi_lan_fixture() {
    let d = tam("repeat");
    ghi(&d, "a_treo.vish", TREO_CO_HAN);
    ghi_out(&d, "a_treo.out", "[]\nxong\n");
    let ds = d.to_str().unwrap();

    let (c, o, e) = vis(&["test", "--timeout", "50", "--repeat", "2", ds]);
    assert_eq!(c, 1, "phải exit 1: {o} {e}");
    let ls = kiem_tap(&o);
    assert_eq!(ls[1], "1..2", "{o}");
    assert_eq!(ls[2], "not ok 1 - a_treo.vish [1/2]", "{o}");
    assert_eq!(ls[4], "not ok 2 - a_treo.vish [2/2]", "{o}");
    assert_eq!(
        o.matches("quá hạn 50 ms").count(),
        2,
        "mỗi lần lặp một hạn: {o}"
    );

    let _ = std::fs::remove_dir_all(&d);
}
