// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `vis test --jobs <N>`: chạy **đơn vị** ca song song, **giữ nguyên thứ tự đầu ra**.
//!
//! Đơn vị = (ca đã chọn sau `--filter`/`--shuffle`/`--shard`, lần lặp `k` của `--repeat`).
//! Khoá hành vi:
//! - `N` nguyên ≥ 1, mặc định **1**; `--jobs 1` giữ **nguyên byte** TAP/JSON/thứ tự cũ.
//! - Kết quả in theo **đúng thứ tự cũ** (chỉ số ca, rồi k) dù worker xong lệch thứ tự.
//! - `--fail-fast`: ngừng **lập lịch** đơn vị mới ngay sau lần đỏ đầu tiên; đơn vị đang chạy
//!   hoàn tất; in theo thứ tự gốc; plan/cases đếm **đơn vị đã chạy**. Vì đơn vị đã lập lịch
//!   trước khi thấy đỏ vẫn chạy, plan có thể **lớn hơn** vị trí ca đỏ (khác bản tuần tự).
//! - `--list` bỏ qua `--jobs` (vẫn kiểm N); `0`, âm, không nguyên, thiếu giá trị, lặp cờ
//!   ⇒ exit **2** + usage nêu `--jobs`.
//!
//! **RED-BEFORE**: trên HEAD `11fdaec` cờ `--jobs` chưa tồn tại ⇒ mọi lệnh `vis test --jobs …`
//! in `vis test: không hỗ trợ cờ '--jobs'` và exit **2**; chạy tệp này trước khi sửa:
//! **7/8 ca đỏ** (ca `jobs_sai_exit_2` vẫn xanh vì cờ lạ cũng exit 2 và thông điệp có `--jobs`).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(unix)]
use std::time::Instant;

fn tam(tag: &str) -> PathBuf {
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-jobs-{tag}-{}-{n}", std::process::id()));
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

fn vis(args: &[&str]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(args)
        .output()
        .expect("chạy vis");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// TAP hợp lệ: dòng đầu `TAP version 13`, dòng kế là plan, plan **bằng** số dòng test.
fn kiem_tap(tap: &str) -> Vec<&str> {
    let ls: Vec<&str> = tap.lines().collect();
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

fn test_fn(ten: &str) -> String {
    format!("test fn {ten}():\n    assert true\n")
}

fn test_fn_do(ten: &str) -> String {
    format!("test fn {ten}():\n    assert 1 == 2\n")
}

/// 6 ca `ca00..ca05` (mỗi tệp một `test fn`) theo thứ tự tên tệp `f0.vis..f5.vis`.
fn sau_ca(d: &Path) -> Vec<String> {
    let mut tens = Vec::new();
    for i in 0..6 {
        let ten = format!("ca{i:02}");
        ghi(d, &format!("f{i}.vis"), &test_fn(&ten));
        tens.push(ten);
    }
    tens
}

/// (1) `--jobs 1` = hành vi cũ **từng byte** (TAP, JSON, JSON-out, thứ tự); usage nêu `--jobs`.
/// Đây là ca **đỏ trước** trên HEAD `11fdaec` (lúc đó exit 2 vì cờ lạ).
#[test]
fn jobs_1_giong_khong_co() {
    let d = tam("mot");
    ghi(&d, "a.vis", "test fn alpha():\n    assert 1 == 1\n");
    ghi(&d, "b.vis", "print(1)\n");
    ghi_out(&d, "b.out", "1\n");
    ghi(&d, "c.vis", "test fn gamma():\n    assert 2 == 2\n");
    let ds = d.to_str().unwrap();

    let (c0, o0, e0) = vis(&["test", ds]);
    assert_eq!(c0, 0, "{o0} {e0}");
    let (c1, o1, e1) = vis(&["test", "--jobs", "1", ds]);
    assert_eq!(c1, 0, "--jobs 1 phải như cũ: {o1} {e1}");
    assert!(!e1.contains("không hỗ trợ cờ"), "--jobs vẫn là cờ lạ: {e1}");
    assert_eq!(o1, o0, "--jobs 1 phải giữ nguyên TAP cũ");
    kiem_tap(&o1);

    let (cj0, oj0, ej0) = vis(&["test", "--json", ds]);
    assert_eq!(cj0, 0, "{oj0} {ej0}");
    let (cj1, oj1, ej1) = vis(&["test", "--json", "--jobs", "1", ds]);
    assert_eq!(cj1, 0, "{oj1} {ej1}");
    assert_eq!(oj1, oj0, "--jobs 1 phải giữ nguyên JSON cũ");

    // `--json-out` cùng shape/nội dung khi chạy 1 worker.
    let f0 = d.join("j0.json");
    let f1 = d.join("j1.json");
    let (co0, oo0, eo0) = vis(&[
        "test",
        "--json-out",
        f0.to_str().unwrap(),
        "--jobs",
        "1",
        ds,
    ]);
    assert_eq!(co0, 0, "{oo0} {eo0}");
    assert_eq!(oo0, o0, "--json-out vẫn in TAP ra stdout");
    let (co1, oo1, _) = vis(&["test", "--json-out", f1.to_str().unwrap(), ds]);
    assert_eq!(co1, 0);
    assert_eq!(oo1, oo0, "--json-out 1 worker phải giữ TAP");
    assert_eq!(
        std::fs::read_to_string(&f1).expect("đọc json-out"),
        std::fs::read_to_string(&f0).expect("đọc json-out"),
        "--json-out phải cùng nội dung"
    );

    let (ch, _, eh) = vis(&["test", "--help"]);
    assert_eq!(ch, 2, "--help vẫn là lỗi dùng");
    assert!(eh.contains("--jobs"), "usage phải nêu --jobs: {eh}");

    // Root usage (không có lệnh con) cũng phải nêu cờ mới.
    let (cr, _, er) = vis(&[]);
    assert_eq!(cr, 2);
    assert!(er.contains("--jobs"), "root usage: {er}");

    let _ = std::fs::remove_dir_all(&d);
}

/// (2) Toàn xanh, `--jobs 4` cho TAP + JSON **byte-identical** `--jobs 1`; gồm cả `test fn`
/// (thư mục tạm phải duy nhất theo đơn vị) lẫn fixture `.out`.
#[test]
fn jobs_4_toan_xanh_giong_jobs_1() {
    let d = tam("xanh");
    let tens = sau_ca(&d);
    ghi(&d, "g.vis", "print(7)\n");
    ghi_out(&d, "g.out", "7\n");
    let ds = d.to_str().unwrap();

    let (c1, o1, e1) = vis(&["test", "--jobs", "1", ds]);
    assert_eq!(c1, 0, "{o1} {e1}");
    let (c4, o4, e4) = vis(&["test", "--jobs", "4", ds]);
    assert_eq!(c4, 0, "--jobs 4: {o4} {e4}");
    assert_eq!(o4, o1, "TAP song song phải giống hệt tuần tự");
    let ls = kiem_tap(&o4);
    assert_eq!(ls[1], "1..7");
    for (i, ten) in tens.iter().enumerate() {
        assert_eq!(ls[2 + i], format!("ok {} - {ten}", i + 1));
    }
    assert_eq!(ls[8], "ok 7 - g.vis");

    let (cj1, oj1, _) = vis(&["test", "--json", "--jobs", "1", ds]);
    assert_eq!(cj1, 0);
    let (cj4, oj4, _) = vis(&["test", "--json", "--jobs", "4", ds]);
    assert_eq!(cj4, 0);
    assert_eq!(oj4, oj1, "JSON song song phải giống hệt tuần tự");

    let _ = std::fs::remove_dir_all(&d);
}

/// (3) Trộn xanh/đỏ, `--jobs 4` cùng **kết quả + thứ tự** với tuần tự (TAP và JSON).
#[test]
fn jobs_4_tron_xanh_do_giu_thu_tu() {
    let d = tam("tron");
    ghi(&d, "f0.vis", &test_fn("alpha"));
    ghi(&d, "f1.vis", &test_fn_do("beta"));
    ghi(&d, "f2.vis", &test_fn("gamma"));
    ghi(&d, "f3.vis", &test_fn_do("delta"));
    ghi(&d, "f4.vis", &test_fn("epsilon"));
    let ds = d.to_str().unwrap();

    let (c1, o1, e1) = vis(&["test", "--jobs", "1", ds]);
    assert_eq!(c1, 1, "{o1} {e1}");
    let (c4, o4, e4) = vis(&["test", "--jobs", "4", ds]);
    assert_eq!(c4, 1, "{o4} {e4}");
    assert_eq!(o4, o1, "TAP trộn đỏ phải giữ nguyên thứ tự");
    let ls = kiem_tap(&o4);
    assert_eq!(ls[1], "1..5");
    let dau: Vec<&str> = ls
        .iter()
        .copied()
        .filter(|l| l.starts_with("ok ") || l.starts_with("not ok "))
        .collect();
    assert_eq!(dau.len(), 5);
    assert_eq!(dau[0], "ok 1 - alpha");
    assert_eq!(dau[1], "not ok 2 - beta");
    assert_eq!(dau[2], "ok 3 - gamma");
    assert_eq!(dau[3], "not ok 4 - delta");
    assert_eq!(dau[4], "ok 5 - epsilon");

    let (cj1, oj1, _) = vis(&["test", "--json", "--jobs", "1", ds]);
    assert_eq!(cj1, 1);
    let (cj4, oj4, _) = vis(&["test", "--json", "--jobs", "4", ds]);
    assert_eq!(cj4, 1);
    assert_eq!(oj4, oj1, "JSON trộn đỏ phải giữ nguyên thứ tự");
    assert!(
        oj4.starts_with("{\"total\":5,\"ok\":3,\"failed\":2,"),
        "{oj4}"
    );

    let _ = std::fs::remove_dir_all(&d);
}

/// (4) `--repeat 2 --jobs 3` = tuần tự **từng byte**: ca-trước-lặp-sau, tên `[k/2]`,
/// đánh số TAP liên tục, plan `1..(số ca × 2)`.
#[test]
fn repeat_2_jobs_3_giong_tuan_tu() {
    let d = tam("repeat");
    ghi(&d, "f0.vis", &test_fn("mot"));
    ghi(&d, "f1.vis", &test_fn_do("hai"));
    ghi(&d, "f2.vis", &test_fn("ba"));
    let ds = d.to_str().unwrap();

    let (c1, o1, e1) = vis(&["test", "--repeat", "2", ds]);
    assert_eq!(c1, 1, "{o1} {e1}");
    let (c3, o3, e3) = vis(&["test", "--repeat", "2", "--jobs", "3", ds]);
    assert_eq!(c3, 1, "{o3} {e3}");
    assert_eq!(o3, o1, "TAP repeat + jobs phải giống hệt tuần tự");
    let ls = kiem_tap(&o3);
    assert_eq!(ls[1], "1..6");
    let dau: Vec<&str> = ls
        .iter()
        .copied()
        .filter(|l| l.starts_with("ok ") || l.starts_with("not ok "))
        .collect();
    let mong = [
        "ok 1 - mot [1/2]",
        "ok 2 - mot [2/2]",
        "not ok 3 - hai [1/2]",
        "not ok 4 - hai [2/2]",
        "ok 5 - ba [1/2]",
        "ok 6 - ba [2/2]",
    ];
    assert_eq!(dau, mong);

    let (cj1, oj1, _) = vis(&["test", "--json", "--repeat", "2", ds]);
    assert_eq!(cj1, 1);
    let (cj3, oj3, _) = vis(&["test", "--json", "--repeat", "2", "--jobs", "3", ds]);
    assert_eq!(cj3, 1);
    assert_eq!(oj3, oj1, "JSON repeat + jobs phải giống hệt tuần tự");

    let _ = std::fs::remove_dir_all(&d);
}

/// (5) `--fail-fast --jobs 4`: exit 1, TAP hợp lệ, plan/cases = **số đơn vị đã chạy**; các đơn
/// vị đã chạy là **tiền tố** theo thứ tự gốc (không lỗ thứ tự), và vì đã lập lịch trước khi thấy
/// đỏ nên plan có thể lớn hơn vị trí ca đỏ — khác bản tuần tự.
#[test]
fn fail_fast_jobs_4_plan_bang_don_vi_da_chay() {
    let d = tam("ff");
    let mut tens = Vec::new();
    for i in 0..8 {
        let ten = format!("ca{i:02}");
        let src = if i == 2 {
            test_fn_do(&ten)
        } else {
            test_fn(&ten)
        };
        ghi(&d, &format!("f{i}.vis"), &src);
        tens.push(ten);
    }
    let ds = d.to_str().unwrap();

    let (c, o, e) = vis(&["test", "--fail-fast", "--jobs", "4", ds]);
    assert_eq!(c, 1, "phải exit 1: {o} {e}");
    let ls = kiem_tap(&o);
    let plan: usize = ls[1].strip_prefix("1..").unwrap().parse().unwrap();
    assert!(
        (3..=8).contains(&plan),
        "phải chạy ít nhất 3 đơn vị (0,1,2) và tối đa 8: plan {plan}: {o}"
    );
    let dau: Vec<&str> = ls
        .iter()
        .copied()
        .filter(|l| l.starts_with("ok ") || l.starts_with("not ok "))
        .collect();
    assert_eq!(dau.len(), plan);
    for (i, ten) in tens.iter().take(plan).enumerate() {
        let trang_thai = if i == 2 { "not ok" } else { "ok" };
        assert_eq!(dau[i], format!("{trang_thai} {} - {ten}", i + 1));
    }
    assert!(o.contains("not ok 3 - ca02"), "ca đỏ phải có mặt: {o}");
    assert!(
        !o.contains("ca08"),
        "không bao giờ có đơn vị ngoài tập: {o}"
    );

    let (cj, oj, ej) = vis(&["test", "--fail-fast", "--jobs", "4", "--json", ds]);
    assert_eq!(cj, 1, "{oj} {ej}");
    // Lần chạy JSON là **một lần chạy riêng** — plan có thể khác lần TAP (lập lịch song song
    // không tất định), nhưng vẫn phải là tiền tố đã chạy và đếm đúng.
    let total: usize = oj
        .strip_prefix("{\"total\":")
        .and_then(|s| s.split(',').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("JSON thiếu total: {oj}"));
    assert!(
        (3..=8).contains(&total),
        "JSON phải đếm 3..=8 đơn vị đã chạy, được {total}: {oj}"
    );
    assert!(
        oj.starts_with(&format!(
            "{{\"total\":{total},\"ok\":{},\"failed\":1,",
            total - 1
        )),
        "JSON phải đếm đơn vị đã chạy (total {total}): {oj}"
    );
    assert_eq!(
        oj.matches("\"name\":").count(),
        total,
        "số cases phải bằng total: {oj}"
    );
    let mut vi_tri = 0;
    for (i, ten) in tens.iter().take(total).enumerate() {
        let mau = format!("\"name\":\"{ten}\",\"ok\":{}", i != 2);
        let j = oj[vi_tri..]
            .find(&mau)
            .unwrap_or_else(|| panic!("thiếu `{mau}` đúng thứ tự trong {oj}"));
        vi_tri += j + mau.len();
    }
    assert!(!oj.contains("ca08"), "{oj}");

    let _ = std::fs::remove_dir_all(&d);
}

/// (6) `--filter` + `--shard` áp **trước** đơn vị; `--jobs 4` cho cùng TAP/JSON với tuần tự;
/// `--list --jobs` không đổi.
#[test]
fn filter_shard_jobs_giu_ket_qua() {
    let d = tam("loc");
    for i in 0..8 {
        let ten = format!("alpha{i:02}");
        ghi(&d, &format!("f{i}.vis"), &test_fn(&ten));
    }
    for i in 0..3 {
        let ten = format!("beta{i:02}");
        ghi(&d, &format!("b{i}.vis"), &test_fn_do(&ten));
    }
    let ds = d.to_str().unwrap();

    let tham_so = ["test", "--filter", "alpha", "--shard", "1/2"];
    let mut tuan_tu = tham_so.to_vec();
    tuan_tu.push(ds);
    let (c1, o1, e1) = vis(&tuan_tu);
    assert_eq!(c1, 0, "{o1} {e1}");
    let mut song_song = tham_so.to_vec();
    song_song.extend(["--jobs", "4", ds]);
    let (c4, o4, e4) = vis(&song_song);
    assert_eq!(c4, 0, "{o4} {e4}");
    assert_eq!(o4, o1, "filter+shard+jobs phải giống hệt tuần tự");
    let ls = kiem_tap(&o4);
    assert_eq!(ls[1], "1..4", "8 alpha, shard 1/2 ⇒ 4 ca: {o4}");
    assert!(!o4.contains("beta"), "filter phải loại beta: {o4}");

    let (cl, ol, el) = vis(&["test", "--list", "--filter", "alpha", "--shard", "1/2", ds]);
    assert_eq!(cl, 0, "{ol} {el}");
    let (clj, olj, elj) = vis(&[
        "test", "--list", "--jobs", "4", "--filter", "alpha", "--shard", "1/2", ds,
    ]);
    assert_eq!(clj, 0, "{olj} {elj}");
    assert_eq!(olj, ol, "--list phải bỏ qua --jobs");

    let _ = std::fs::remove_dir_all(&d);
}

/// (7) `--list` bỏ qua `--jobs` (một dòng mỗi ca; JSON cùng shape) nhưng vẫn kiểm N.
#[test]
fn list_bo_qua_jobs() {
    let d = tam("list");
    let tens = sau_ca(&d);
    let ds = d.to_str().unwrap();

    let (c, o, e) = vis(&["test", "--list", ds]);
    assert_eq!(c, 0, "{o} {e}");
    let mong: Vec<&str> = tens.iter().map(String::as_str).collect();
    assert_eq!(o.lines().collect::<Vec<_>>(), mong);

    let (cj, oj, ej) = vis(&["test", "--list", "--jobs", "4", ds]);
    assert_eq!(cj, 0, "{oj} {ej}");
    assert_eq!(oj, o, "--list phải bỏ qua --jobs");

    let (cjj, ojj, ejj) = vis(&["test", "--list", "--json", ds]);
    assert_eq!(cjj, 0, "{ojj} {ejj}");
    let (cjj4, ojj4, _) = vis(&["test", "--list", "--json", "--jobs", "4", ds]);
    assert_eq!(cjj4, 0);
    assert_eq!(ojj4, ojj, "--list --json phải bỏ qua --jobs");

    let (c0, _, e0) = vis(&["test", "--list", "--jobs", "0", ds]);
    assert_eq!(c0, 2, "--list vẫn phải kiểm N: {e0}");
    assert!(e0.contains("--jobs"), "{e0}");

    let _ = std::fs::remove_dir_all(&d);
}

/// (8) N sai (`0`, âm, không nguyên, thiếu giá trị, lặp cờ) ⇒ exit **2**, stderr nêu `--jobs`.
#[test]
fn jobs_sai_exit_2() {
    let d = tam("sai");
    ghi(&d, "a.vis", &test_fn("mot"));
    let ds = d.to_str().unwrap();

    let thieu = vec!["test", "--jobs"];
    let khong = vec!["test", "--jobs", "0", ds];
    let am = vec!["test", "--jobs", "-2", ds];
    let khong_nguyen = vec!["test", "--jobs", "1.5", ds];
    let chu = vec!["test", "--jobs", "abc", ds];
    let lap = vec!["test", "--jobs", "2", "--jobs", "3", ds];
    for args in [thieu, khong, am, khong_nguyen, chu, lap] {
        let (c, _, e) = vis(&args);
        assert_eq!(c, 2, "{args:?}: {e}");
        assert!(e.contains("--jobs"), "{args:?}: {e}");
    }

    let _ = std::fs::remove_dir_all(&d);
}

/// **ST-324a (#88)** — fixture sinh **tiến trình cháu giữ đầu ghi ống**: `vis run` bị `--timeout`
/// giết nhưng `sleep 30` (cháu) phải chết theo, nếu không `vis test` treo / cụt output / rò tiến
/// trình. Chỉ Unix (`/bin/sh`, `kill`, `chmod`); nhánh Windows dùng `taskkill /T` trong source,
/// không có công cụ tương đương trong fixture tích hợp này.
#[cfg(unix)]
#[test]
fn timeout_giet_cay_chau_giu_ong() {
    use std::os::unix::fs::PermissionsExt;

    let d = tam("giet-cay");
    let pid_file = d.join("grandchild.pid");
    let script = d.join("hold.sh");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\nsleep 30 & echo $! > {}\necho HOLD-READY\nwait\n",
            pid_file.display()
        ),
    )
    .expect("ghi script");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
        .expect("chmod script");
    ghi(
        &d,
        "treo.vish",
        &format!(
            "# SPDX-License-Identifier: MIT OR Apache-2.0\n{}\n",
            script.display()
        ),
    );
    ghi(&d, "xanh.vis", "print(7)\n");
    ghi_out(&d, "xanh.out", "7\n");
    let ds = d.to_str().expect("đường dẫn tạm UTF-8");

    let t0 = Instant::now();
    let (c, o, e) = vis(&["test", "--timeout", "300", ds]);
    let dt = t0.elapsed();
    assert_eq!(c, 1, "phải exit 1: {o} {e}");
    assert!(dt.as_millis() < 3_000, "phải trả nhanh, không treo: {dt:?}");
    // Byte-shape TAP giữ nguyên: plan, dòng `not ok`, thông điệp `quá hạn`, ca kế vẫn chạy.
    assert_eq!(
        o,
        "TAP version 13\n1..2\nnot ok 1 - treo.vish\n# treo.vish: quá hạn 300 ms\nok 2 - xanh.vis\n",
        "TAP đổi byte: {o:?}"
    );
    kiem_tap(&o);

    // Hậu duệ phải chết trong vòng 500 ms sau khi `vis test` trả.
    let pid: u32 = std::fs::read_to_string(&pid_file)
        .expect("cháu phải kịp ghi pid")
        .trim()
        .parse()
        .expect("pid cháu là số");
    std::thread::sleep(std::time::Duration::from_millis(500));
    let con_song = |p: u32| {
        Command::new("/bin/sh")
            .arg("-c")
            .arg(format!("kill -0 {p} 2>/dev/null"))
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    };
    assert!(!con_song(pid), "cháu {pid} vẫn sống sau 500 ms (#88)");

    // Byte-shape JSON giữ nguyên, và lần chạy JSON cũng dọn sạch cháu của chính nó.
    let (cj, oj, ej) = vis(&["test", "--timeout", "300", "--json", ds]);
    assert_eq!(cj, 1, "{oj} {ej}");
    assert_eq!(
        oj,
        "{\"total\":2,\"ok\":1,\"failed\":1,\"cases\":[{\"name\":\"treo.vish\",\"ok\":false,\"message\":\"quá hạn 300 ms\"},{\"name\":\"xanh.vis\",\"ok\":true,\"message\":\"\"}]}\n",
        "JSON đổi byte: {oj:?}"
    );
    let pid2: u32 = std::fs::read_to_string(&pid_file)
        .expect("cháu lần JSON phải kịp ghi pid")
        .trim()
        .parse()
        .expect("pid cháu là số");
    assert!(
        !con_song(pid2),
        "cháu {pid2} (lần JSON) vẫn sống sau 500 ms"
    );

    let _ = std::fs::remove_dir_all(&d);
}
