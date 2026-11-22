// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-267** — `vis test --filter <substr>` và `--fail-fast`.
//!
//! Khoá hành vi:
//! - `--filter` chọn theo **tên `test fn`** hoặc **đường dẫn hiển thị** của fixture, phân biệt
//!   hoa/thường; lọc **trước khi chạy** nên ca bị loại không chạy (ca đỏ bị loại không làm đỏ).
//! - Không khớp (hoặc chuỗi rỗng) ⇒ `TAP version 13` + `1..0`, exit **0**.
//! - `--fail-fast` dừng sau ca `not ok` đầu tiên; kế hoạch TAP = **số ca đã chạy**, exit **1**.
//! - `--json` đếm cùng tập ca đã chạy; cờ lạ / thiếu giá trị `--filter` vẫn exit **2**.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

fn tam(tag: &str) -> PathBuf {
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd267-{tag}-{}-{n}", std::process::id()));
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

fn dong(s: &str) -> Vec<&str> {
    s.lines().collect()
}

/// TAP hợp lệ: dòng đầu `TAP version 13`, dòng kế là plan, và plan **bằng** số dòng test.
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

/// `a.vis`: `alpha_mot` xanh, `beta_hai` **đỏ nếu chạy**.
const A_TEST: &str =
    "test fn alpha_mot():\n    assert 1 == 1\n\ntest fn beta_hai():\n    assert 1 == 2\n";
/// `b.vis`: ca thứ hai khớp `alpha`.
const B_TEST: &str = "test fn alpha_hai():\n    assert 2 == 2\n";

/// (a) `--filter` theo tên `test fn`: khớp 1 và khớp nhiều; ca đỏ bị loại **không chạy**.
/// Chạy cả interp lẫn `--vm`, TAP phải **cùng byte**.
#[test]
fn loc_theo_ten_test_fn_mot_va_nhieu() {
    let d = tam("loc-ten");
    ghi(&d, "a.vis", A_TEST);
    ghi(&d, "b.vis", B_TEST);

    let (ci, oi, ei) = vis(&["test", "--filter", "alpha", d.to_str().unwrap()]);
    assert_eq!(ci, 0, "ca đỏ bị lọc không được chạy: {oi} {ei}");
    assert_eq!(
        oi,
        "TAP version 13\n1..2\nok 1 - alpha_mot\nok 2 - alpha_hai\n"
    );
    kiem_tap(&oi);

    let (cv, ov, ev) = vis(&["test", "--vm", "--filter", "alpha", d.to_str().unwrap()]);
    assert_eq!(cv, 0, "--vm: {ov} {ev}");
    assert_eq!(ov, oi, "TAP lệch interp/VM");

    let _ = std::fs::remove_dir_all(&d);
}

/// (b) `--filter` theo **đường dẫn hiển thị** của fixture; ca `.out` sai bị loại không chạy.
/// Một tệp đơn (gốc = chính tệp) vẫn lọc được theo tên tệp.
#[test]
fn loc_theo_duong_dan_fixture_va_tep_don() {
    let d = tam("loc-fixture");
    ghi(&d, "a_ok.vis", "print(1)\n");
    ghi_out(&d, "a_ok.out", "1\n");
    ghi(&d, "b_hong.vis", "print(1)\n");
    ghi_out(&d, "b_hong.out", "999\n");

    let (c, o, e) = vis(&["test", "--filter", "a_ok", d.to_str().unwrap()]);
    assert_eq!(c, 0, "{o} {e}");
    assert_eq!(o, "TAP version 13\n1..1\nok 1 - a_ok.vis\n");
    kiem_tap(&o);

    let (cv, ov, ev) = vis(&["test", "--vm", "--filter", "a_ok", d.to_str().unwrap()]);
    assert_eq!(cv, 0, "--vm: {ov} {ev}");
    assert_eq!(ov, o, "TAP lệch interp/VM");

    // Gốc là **một tệp**: tên hiển thị TAP vẫn như cũ (rỗng), nhưng lọc khớp tên tệp đã truyền.
    let p = d.join("a_ok.vis");
    let (c1, o1, e1) = vis(&["test", "--filter", "a_ok.vis", p.to_str().unwrap()]);
    assert_eq!(c1, 0, "lọc tệp đơn: {o1} {e1}");
    kiem_tap(&o1);
    assert_eq!(o1.lines().filter(|l| l.starts_with("ok ")).count(), 1);
    assert!(!o1.contains("b_hong"));

    let _ = std::fs::remove_dir_all(&d);
}

/// (c) Không khớp / chuỗi rỗng / sai hoa-thường ⇒ `1..0`, exit 0 (TAP hợp lệ).
#[test]
fn loc_khong_khop_ra_plan_rong_exit_0() {
    let d = tam("loc-rong");
    ghi(&d, "a_ok.vis", "print(1)\n");
    ghi_out(&d, "a_ok.out", "1\n");

    for f in ["khong_co_ca_nao", "A_OK", ""] {
        let (c, o, e) = vis(&["test", "--filter", f, d.to_str().unwrap()]);
        assert_eq!(c, 0, "filter {f:?}: {o} {e}");
        assert_eq!(o, "TAP version 13\n1..0\n", "filter {f:?}: {o:?}");
        kiem_tap(&o);
    }

    let _ = std::fs::remove_dir_all(&d);
}

/// (d) `--filter` + `--json`: `total` = số ca **đã chọn**; ca bị loại không có trong `cases`.
#[test]
fn loc_ket_hop_json() {
    let d = tam("loc-json");
    ghi(&d, "a.vis", A_TEST);
    ghi(&d, "b.vis", B_TEST);

    let (c, o, e) = vis(&["test", "--json", "--filter", "alpha", d.to_str().unwrap()]);
    assert_eq!(c, 0, "{o} {e}");
    assert_eq!(
        o,
        "{\"total\":2,\"ok\":2,\"failed\":0,\"cases\":[{\"name\":\"alpha_mot\",\"ok\":true,\"message\":\"\"},{\"name\":\"alpha_hai\",\"ok\":true,\"message\":\"\"}]}\n"
    );

    let (c0, o0, e0) = vis(&[
        "test",
        "--json",
        "--filter",
        "khong_co",
        d.to_str().unwrap(),
    ]);
    assert_eq!(c0, 0, "{o0} {e0}");
    assert_eq!(o0, "{\"total\":0,\"ok\":0,\"failed\":0,\"cases\":[]}\n");

    let _ = std::fs::remove_dir_all(&d);
}

/// `a_ok` xanh; `b_hong` và `c_hong` **đỏ** (để chứng minh `c_hong` sẽ chạy nếu không fail-fast).
fn thu_muc_fail_fast(tag: &str) -> PathBuf {
    let d = tam(tag);
    ghi(&d, "a_ok.vis", "print(1)\n");
    ghi_out(&d, "a_ok.out", "1\n");
    ghi(&d, "b_hong.vis", "print(1)\n");
    ghi_out(&d, "b_hong.out", "999\n");
    ghi(&d, "c_hong.vis", "print(1)\n");
    ghi_out(&d, "c_hong.out", "999\n");
    d
}

/// (e) `--fail-fast` dừng sau ca `not ok` **đầu tiên**: `1..2`, không có `c_hong`.
/// Đối chứng không cờ: `1..3` và **cả** `b_hong` lẫn `c_hong` đều `not ok`.
#[test]
fn fail_fast_dung_sau_ca_do_dau_tien() {
    let d = thu_muc_fail_fast("ff");

    let (c, o, e) = vis(&["test", d.to_str().unwrap()]);
    assert_eq!(c, 1, "đối chứng: {o} {e}");
    let ls = kiem_tap(&o);
    assert_eq!(ls[1], "1..3");
    assert!(o.contains("not ok 2 - b_hong.vis"), "{o}");
    assert!(
        o.contains("not ok 3 - c_hong.vis"),
        "đối chứng phải chạy c_hong: {o}"
    );

    let (cf, of, ef) = vis(&["test", "--fail-fast", d.to_str().unwrap()]);
    assert_eq!(cf, 1, "fail-fast vẫn exit 1: {of} {ef}");
    let lf = kiem_tap(&of);
    assert_eq!(lf[1], "1..2", "kế hoạch = số ca đã chạy: {of}");
    assert_eq!(lf[2], "ok 1 - a_ok.vis");
    assert!(of.contains("not ok 2 - b_hong.vis"), "{of}");
    assert!(!of.contains("c_hong"), "phải dừng trước c_hong: {of}");
    assert_eq!(
        lf.iter()
            .filter(|l| l.starts_with("ok ") || l.starts_with("not ok "))
            .count(),
        2,
        "chỉ được chạy 2 ca: {of}"
    );

    let (cv, ov, ev) = vis(&["test", "--vm", "--fail-fast", d.to_str().unwrap()]);
    assert_eq!(cv, 1, "--vm: {ov} {ev}");
    assert_eq!(ov, of, "TAP lệch interp/VM");
    kiem_tap(&ov);

    let _ = std::fs::remove_dir_all(&d);
}

/// (f) `--fail-fast` khi **toàn xanh** không cắt: đủ số ca đã chọn, exit 0.
#[test]
fn fail_fast_toan_xanh_van_chay_du() {
    let d = tam("ff-xanh");
    ghi(&d, "a.vis", "test fn mot():\n    assert 1 == 1\n");
    ghi(&d, "b.vis", "test fn hai():\n    assert 2 == 2\n");

    let (c, o, e) = vis(&["test", "--fail-fast", d.to_str().unwrap()]);
    assert_eq!(c, 0, "{o} {e}");
    assert_eq!(o, "TAP version 13\n1..2\nok 1 - mot\nok 2 - hai\n");
    kiem_tap(&o);

    let _ = std::fs::remove_dir_all(&d);
}

/// (g) `--fail-fast --json`: `total` = số ca **đã chạy** (2), ca thứ ba bị dừng không có mặt.
#[test]
fn fail_fast_ket_hop_json() {
    let d = thu_muc_fail_fast("ff-json");

    let (c, o, e) = vis(&["test", "--json", "--fail-fast", d.to_str().unwrap()]);
    assert_eq!(c, 1, "{o} {e}");
    assert_eq!(
        o,
        "{\"total\":2,\"ok\":1,\"failed\":1,\"cases\":[{\"name\":\"a_ok.vis\",\"ok\":true,\"message\":\"\"},{\"name\":\"b_hong.vis\",\"ok\":false,\"message\":\"VS0000 stdout khác kỳ vọng\"}]}\n"
    );

    let _ = std::fs::remove_dir_all(&d);
}

/// (h) Cờ lạ / thiếu giá trị `--filter` / `--help`: exit 2 và usage nêu **cả hai** cờ mới.
#[test]
fn co_moi_va_thieu_gia_tri_loc_van_exit_2() {
    let (c, _, e) = vis(&["test", "--bogus"]);
    assert_eq!(c, 2);
    assert!(e.contains("dùng: vis test"), "stderr: {e}");
    assert!(
        e.contains("--filter") && e.contains("--fail-fast"),
        "stderr: {e}"
    );

    let (c, _, e) = vis(&["test", "--filter"]);
    assert_eq!(c, 2, "thiếu giá trị --filter: {e}");
    assert!(e.contains("--filter"), "stderr: {e}");

    let (c, _, e) = vis(&["test", "--filter", "--fail-fast"]);
    assert_eq!(c, 2, "--filter nuốt cờ: {e}");
    assert!(e.contains("--filter"), "stderr: {e}");

    let (c, _, e) = vis(&["test", "--help"]);
    assert_eq!(c, 2);
    assert!(
        e.contains("--filter") && e.contains("--fail-fast"),
        "stderr: {e}"
    );
}
