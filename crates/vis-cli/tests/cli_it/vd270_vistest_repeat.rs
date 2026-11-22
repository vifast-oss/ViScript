// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `vis test --repeat <N>`: chạy mỗi ca đã chọn **N lần**.
//!
//! Khoá hành vi:
//! - `N ≥ 1`, mặc định **1**; `N = 1` giữ **nguyên byte** TAP/JSON cũ (không hậu tố).
//! - `N > 1` ⇒ mỗi lần lặp là **một ca riêng**: TAP `ok/not ok` + tên `"<tên> [k/N]"`
//!   (k 1-based), đánh số TAP liên tục; plan `1..(số lần đã chạy)`.
//! - Đỏ ở **bất kỳ** lần lặp nào ⇒ exit **1**; không `--fail-fast` thì vẫn giữ **mọi** lần lặp.
//! - `--fail-fast` dừng ngay sau lần lặp đỏ **đầu tiên** (plan = số lần đã chạy).
//! - `--json` liệt kê từng lần lặp trong `cases`, `total/ok/failed` đếm **lần lặp**.
//! - `--list` bỏ qua `--repeat` (liệt kê ca đã chọn **một lần**) nhưng vẫn kiểm N.
//! - `--timeout` áp **mỗi lần lặp** của `test fn`.
//! - `0`, số âm, không phải số nguyên, thiếu giá trị ⇒ exit **2** + usage nêu `--repeat`.
//!
//! **RED-BEFORE**: trên HEAD `e8e5332` cờ `--repeat` chưa tồn tại ⇒ `vis test --repeat 1 …`
//! in `vis test: không hỗ trợ cờ '--repeat'` và exit **2**; chạy tệp này trước khi sửa:
//! **10/11 ca đỏ** (ca `repeat_sai_exit_2` vẫn xanh vì cờ lạ cũng exit 2).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

fn tam(tag: &str) -> PathBuf {
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-repeat-{tag}-{}-{n}", std::process::id()));
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

const MOT: &str = "test fn mot():\n    assert 1 == 1\n";
const HAI: &str = "test fn hai():\n    assert 2 == 2\n";
const HONG: &str = "test fn hong():\n    assert 1 == 2\n";
const XANH: &str = "test fn xanh():\n    assert true\n";
const SAU: &str = "test fn sau():\n    assert 3 == 3\n";

/// (1) `--repeat 1` = hành vi cũ **từng byte**; `--repeat` không còn là cờ lạ.
/// Đây là ca **đỏ trước** trên HEAD `e8e5332` (lúc đó exit 2 vì cờ lạ).
#[test]
fn repeat_1_khong_con_la_co_la_va_tap_cu_khong_doi() {
    let d = tam("mot");
    ghi(&d, "a.vis", MOT);
    ghi(&d, "b.vis", HAI);
    let ds = d.to_str().unwrap();

    let (c0, o0, e0) = vis(&["test", ds]);
    assert_eq!(c0, 0, "{o0} {e0}");
    assert_eq!(o0, "TAP version 13\n1..2\nok 1 - mot\nok 2 - hai\n");

    let (c1, o1, e1) = vis(&["test", "--repeat", "1", ds]);
    assert_eq!(c1, 0, "--repeat 1 phải như cũ: {o1} {e1}");
    assert!(
        !e1.contains("không hỗ trợ cờ"),
        "--repeat vẫn là cờ lạ: {e1}"
    );
    assert_eq!(o1, o0, "--repeat 1 phải giữ nguyên TAP cũ");

    let (cj, oj, ej) = vis(&["test", "--repeat", "1", "--json", ds]);
    assert_eq!(cj, 0, "{oj} {ej}");
    assert_eq!(
        oj,
        "{\"total\":2,\"ok\":2,\"failed\":0,\"cases\":[{\"name\":\"mot\",\"ok\":true,\"message\":\"\"},{\"name\":\"hai\",\"ok\":true,\"message\":\"\"}]}\n",
        "--repeat 1 phải giữ nguyên JSON cũ"
    );

    let (ch, _, eh) = vis(&["test", "--help"]);
    assert_eq!(ch, 2, "--help vẫn là lỗi dùng");
    assert!(eh.contains("--repeat"), "usage phải nêu --repeat: {eh}");

    // Root usage (không có lệnh con) cũng phải nêu cờ mới.
    let (cr, _, er) = vis(&[]);
    assert_eq!(cr, 2);
    assert!(er.contains("--repeat"), "root usage: {er}");

    let _ = std::fs::remove_dir_all(&d);
}

/// (2) `N = 3` toàn xanh: đủ 3 lần lặp mỗi ca, tên có `[k/3]`, đánh số liên tục, plan
/// `1..(2 ca × 3)`; interp và `--vm` cho TAP **cùng byte**.
#[test]
fn repeat_3_toan_xanh_du_lan_va_co_hau_to() {
    let d = tam("xanh");
    ghi(&d, "a.vis", MOT);
    ghi(&d, "b.vis", HAI);
    let ds = d.to_str().unwrap();

    let (c, o, e) = vis(&["test", "--repeat", "3", ds]);
    assert_eq!(c, 0, "{o} {e}");
    assert_eq!(
        o,
        "TAP version 13\n1..6\n\
         ok 1 - mot [1/3]\n\
         ok 2 - mot [2/3]\n\
         ok 3 - mot [3/3]\n\
         ok 4 - hai [1/3]\n\
         ok 5 - hai [2/3]\n\
         ok 6 - hai [3/3]\n"
    );
    kiem_tap(&o);

    let (cv, ov, ev) = vis(&["test", "--vm", "--repeat", "3", ds]);
    assert_eq!(cv, 0, "--vm: {ov} {ev}");
    assert_eq!(ov, o, "TAP lệch interp/VM");

    let _ = std::fs::remove_dir_all(&d);
}

/// (3) Fixture `.out` cũng được lặp (không chỉ `test fn`): tên `a_ok.vis [k/2]`.
#[test]
fn repeat_fixture_out_cung_lap() {
    let d = tam("fixture");
    ghi(&d, "a_ok.vis", "print(1)\n");
    ghi_out(&d, "a_ok.out", "1\n");
    let ds = d.to_str().unwrap();

    let (c, o, e) = vis(&["test", "--repeat", "2", ds]);
    assert_eq!(c, 0, "{o} {e}");
    assert_eq!(
        o,
        "TAP version 13\n1..2\nok 1 - a_ok.vis [1/2]\nok 2 - a_ok.vis [2/2]\n"
    );
    kiem_tap(&o);

    let _ = std::fs::remove_dir_all(&d);
}

/// (4) Một ca đỏ, `N = 3`: **cả ba** lần lặp đều `not ok` (không dừng), mỗi dòng có dòng
/// chẩn đoán `# hong:`; exit **1**, plan `1..3`.
#[test]
fn mot_ca_do_repeat_3_giu_du_ket_qua_va_exit_1() {
    let d = tam("do");
    ghi(&d, "a.vis", HONG);
    let ds = d.to_str().unwrap();

    let (c, o, e) = vis(&["test", "--repeat", "3", ds]);
    assert_eq!(c, 1, "đỏ ở lần lặp nào cũng exit 1: {o} {e}");
    let ls = kiem_tap(&o);
    assert_eq!(ls[1], "1..3");
    for k in 1..=3 {
        // Mỗi `not ok` kèm đúng một dòng `# …` ⇒ cặp (test, chẩn đoán) cách nhau 2 dòng.
        let i = 2 + (k - 1) * 2;
        assert_eq!(
            ls[i],
            format!("not ok {k} - hong [{k}/3]"),
            "thiếu lần lặp {k}: {o}"
        );
        let chi_tiet = ls.get(i + 1).copied().unwrap_or("");
        assert!(
            chi_tiet.starts_with("# hong:"),
            "lần lặp {k} phải có chẩn đoán: {o}"
        );
    }
    assert_eq!(
        o.matches("not ok ").count(),
        3,
        "không được dừng sau lần đỏ đầu: {o}"
    );

    let _ = std::fs::remove_dir_all(&d);
}

/// (5) Trộn xanh/đỏ: ca xanh lặp 3 `ok`, ca đỏ lặp 3 `not ok`; đỏ **không** cắt ca sau,
/// exit **1**, plan `1..6`.
#[test]
fn repeat_3_tron_xanh_do_khong_dung_ca_sau() {
    let d = tam("tron");
    ghi(&d, "a.vis", XANH);
    ghi(&d, "b.vis", HONG);
    let ds = d.to_str().unwrap();

    let (c, o, e) = vis(&["test", "--repeat", "3", ds]);
    assert_eq!(c, 1, "{o} {e}");
    let ls = kiem_tap(&o);
    assert_eq!(ls[1], "1..6", "{o}");
    let dl: Vec<&str> = ls
        .iter()
        .copied()
        .filter(|l| l.starts_with("ok ") || l.starts_with("not ok "))
        .collect();
    assert_eq!(dl.len(), 6, "{o}");
    for (i, ten) in ["xanh", "hong"].iter().enumerate() {
        for k in 1..=3 {
            let n = i * 3 + k;
            let dau = if *ten == "hong" { "not ok" } else { "ok" };
            assert_eq!(dl[n - 1], format!("{dau} {n} - {ten} [{k}/3]"), "{o}");
        }
    }
    assert_eq!(dl.iter().filter(|l| l.starts_with("ok ")).count(), 3);
    assert_eq!(dl.iter().filter(|l| l.starts_with("not ok ")).count(), 3);

    let _ = std::fs::remove_dir_all(&d);
}

/// (6) `--fail-fast`: dừng ngay sau **lần lặp đỏ đầu tiên**; plan = số lần đã chạy.
/// Ca đỏ ở lần 1/3 ⇒ plan `1..4` (3 xanh trước + 1 đỏ), không có `[2/3]`/`[3/3]` hay ca sau.
#[test]
fn fail_fast_dung_sau_lan_lap_do_dau_tien() {
    let d = tam("ff");
    ghi(&d, "a.vis", XANH);
    ghi(&d, "b.vis", HONG);
    ghi(&d, "c.vis", SAU);
    let ds = d.to_str().unwrap();

    let (c, o, e) = vis(&["test", "--repeat", "3", "--fail-fast", ds]);
    assert_eq!(c, 1, "{o} {e}");
    let ls = kiem_tap(&o);
    assert_eq!(ls[1], "1..4", "plan = số lần đã chạy: {o}");
    assert_eq!(ls[2], "ok 1 - xanh [1/3]");
    assert_eq!(ls[3], "ok 2 - xanh [2/3]");
    assert_eq!(ls[4], "ok 3 - xanh [3/3]");
    assert_eq!(ls[5], "not ok 4 - hong [1/3]");
    assert!(!o.contains("hong [2/3]"), "phải dừng sau lần đỏ đầu: {o}");
    assert!(!o.contains("hong [3/3]"), "phải dừng sau lần đỏ đầu: {o}");
    assert!(!o.contains("sau"), "không được chạy ca sau: {o}");
    assert_eq!(
        ls.iter()
            .filter(|l| l.starts_with("ok ") || l.starts_with("not ok "))
            .count(),
        4,
        "chỉ được chạy 4 lần lặp: {o}"
    );

    // Ca đỏ ngay lần 1/3 ⇒ plan `1..1`, chỉ một dòng `not ok`.
    let d1 = tam("ff-mot");
    ghi(&d1, "a.vis", HONG);
    let ds1 = d1.to_str().unwrap();
    let (c1, o1, e1) = vis(&["test", "--repeat", "3", "--fail-fast", ds1]);
    assert_eq!(c1, 1, "{o1} {e1}");
    let l1 = kiem_tap(&o1);
    assert_eq!(l1[1], "1..1");
    assert_eq!(l1[2], "not ok 1 - hong [1/3]");
    assert!(!o1.contains("[2/3]") && !o1.contains("[3/3]"), "{o1}");

    let _ = std::fs::remove_dir_all(&d);
    let _ = std::fs::remove_dir_all(&d1);
}

/// (7) `--json` liệt kê **từng lần lặp** là một case (tên cùng hậu tố) và đếm lần lặp.
#[test]
fn json_repeat_va_fail_fast_dem_lan_lap() {
    let d = tam("json");
    ghi(&d, "a.vis", MOT);
    let ds = d.to_str().unwrap();

    let (c, o, e) = vis(&["test", "--json", "--repeat", "3", ds]);
    assert_eq!(c, 0, "{o} {e}");
    assert_eq!(
        o,
        "{\"total\":3,\"ok\":3,\"failed\":0,\"cases\":[\
         {\"name\":\"mot [1/3]\",\"ok\":true,\"message\":\"\"},\
         {\"name\":\"mot [2/3]\",\"ok\":true,\"message\":\"\"},\
         {\"name\":\"mot [3/3]\",\"ok\":true,\"message\":\"\"}]}\n"
    );

    let dh = tam("json-do");
    ghi(&dh, "a.vis", HONG);
    let dhs = dh.to_str().unwrap();
    let (cd, od, ed) = vis(&["test", "--json", "--repeat", "3", dhs]);
    assert_eq!(cd, 1, "{od} {ed}");
    assert!(
        od.starts_with("{\"total\":3,\"ok\":0,\"failed\":3,\"cases\":["),
        "JSON đỏ: {od}"
    );
    assert_eq!(od.matches("\"ok\":false").count(), 3, "{od}");
    for k in 1..=3 {
        assert!(od.contains(&format!("\"name\":\"hong [{k}/3]\"")), "{od}");
    }

    // `--fail-fast` + JSON: 3 lần xanh rồi 1 lần đỏ ⇒ total = 4 (không phải 6).
    let df = tam("json-ff");
    ghi(&df, "a.vis", XANH);
    ghi(&df, "b.vis", HONG);
    ghi(&df, "c.vis", SAU);
    let dfs = df.to_str().unwrap();
    let (cf, of, ef) = vis(&["test", "--json", "--repeat", "3", "--fail-fast", dfs]);
    assert_eq!(cf, 1, "{of} {ef}");
    assert!(
        of.starts_with("{\"total\":4,\"ok\":3,\"failed\":1,\"cases\":["),
        "JSON fail-fast phải đếm lần đã chạy: {of}"
    );
    assert!(of.contains("\"name\":\"xanh [3/3]\""), "{of}");
    assert!(of.contains("\"name\":\"hong [1/3]\""), "{of}");
    assert!(!of.contains("hong [2/3]"), "{of}");
    assert!(!of.contains("sau"), "{of}");

    let _ = std::fs::remove_dir_all(&d);
    let _ = std::fs::remove_dir_all(&dh);
    let _ = std::fs::remove_dir_all(&df);
}

/// (8) `--list` **bỏ qua** `--repeat` (liệt kê một lần) nhưng vẫn kiểm N (0 ⇒ exit 2).
#[test]
fn list_bo_qua_repeat_va_van_kiem_n() {
    let d = tam("list");
    ghi(&d, "a.vis", MOT);
    ghi(&d, "b.vis", HAI);
    let ds = d.to_str().unwrap();

    let (c, o, e) = vis(&["test", "--list", "--repeat", "3", ds]);
    assert_eq!(c, 0, "{o} {e}");
    assert_eq!(o, "mot\nhai\n", "chỉ liệt kê một lần: {o:?}");

    let (cj, oj, ej) = vis(&["test", "--list", "--json", "--repeat", "3", ds]);
    assert_eq!(cj, 0, "{oj} {ej}");
    assert_eq!(oj, "{\"tests\":[\"mot\",\"hai\"]}\n");

    let (c0, _, e0) = vis(&["test", "--list", "--repeat", "0", ds]);
    assert_eq!(c0, 2, "--list vẫn phải kiểm N: {e0}");
    assert!(e0.contains("--repeat"), "{e0}");

    let _ = std::fs::remove_dir_all(&d);
}

/// (9) `--filter` + `--repeat`: chỉ ca được chọn mới lặp; số/plan tính trên ca đã chọn.
#[test]
fn loc_ket_hop_repeat() {
    let d = tam("loc");
    ghi(&d, "a.vis", "test fn alpha_mot():\n    assert 1 == 1\n");
    ghi(&d, "b.vis", "test fn beta_hai():\n    assert 1 == 2\n");
    let ds = d.to_str().unwrap();

    let (c, o, e) = vis(&["test", "--filter", "alpha", "--repeat", "2", ds]);
    assert_eq!(c, 0, "ca đỏ bị lọc không chạy: {o} {e}");
    assert_eq!(
        o,
        "TAP version 13\n1..2\nok 1 - alpha_mot [1/2]\nok 2 - alpha_mot [2/2]\n"
    );
    assert!(!o.contains("beta_hai"), "{o}");
    kiem_tap(&o);

    let (c0, o0, e0) = vis(&["test", "--filter", "khong_co", "--repeat", "3", ds]);
    assert_eq!(c0, 0, "{o0} {e0}");
    assert_eq!(o0, "TAP version 13\n1..0\n");

    let _ = std::fs::remove_dir_all(&d);
}

/// (10) `--timeout` áp **mỗi lần lặp**: ca treo, N = 2 ⇒ hai `not ok`, hai “quá hạn 300 ms”,
/// tổng vẫn dưới 5 s (không cộng dồn thành một hạn chung).
#[test]
fn timeout_ap_moi_lan_lap() {
    let d = tam("timeout");
    ghi(
        &d,
        "a.vis",
        "test fn treo():\n    var i = 0\n    while true:\n        i = i + 1\n",
    );
    let ds = d.to_str().unwrap();

    let t0 = Instant::now();
    let (c, o, e) = vis(&["test", "--timeout", "300", "--repeat", "2", ds]);
    let dt = t0.elapsed();
    assert_eq!(c, 1, "{o} {e}");
    let ls = kiem_tap(&o);
    assert_eq!(ls[1], "1..2", "{o}");
    assert_eq!(ls[2], "not ok 1 - treo [1/2]");
    assert_eq!(ls[4], "not ok 2 - treo [2/2]");
    assert_eq!(
        o.matches("quá hạn 300 ms").count(),
        2,
        "mỗi lần lặp một hạn: {o}"
    );
    assert!(dt.as_millis() < 5_000, "tổng {dt:?} ≥ 5 s");

    let _ = std::fs::remove_dir_all(&d);
}

/// (11) N sai (`0`, âm, không nguyên, thiếu giá trị) ⇒ exit **2**, stderr nêu `--repeat`.
#[test]
fn repeat_sai_exit_2() {
    let d = tam("sai");
    ghi(&d, "a.vis", MOT);
    let ds = d.to_str().unwrap();

    let thieu = vec!["test", "--repeat"];
    let am = vec!["test", "--repeat", "-2", ds];
    let khong_nguyen = vec!["test", "--repeat", "1.5", ds];
    let chu = vec!["test", "--repeat", "abc", ds];
    let khong = vec!["test", "--repeat", "0", ds];
    for args in [thieu, am, khong_nguyen, chu, khong] {
        let (c, _, e) = vis(&args);
        assert_eq!(c, 2, "{args:?}: {e}");
        assert!(e.contains("--repeat"), "{args:?}: {e}");
    }

    let _ = std::fs::remove_dir_all(&d);
}
