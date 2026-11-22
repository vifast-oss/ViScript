// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `vis test --shuffle [--seed <u64>]`: xáo **thứ tự ca đã chọn**.
//!
//! Khoá hành vi:
//! - `--shuffle` xáo thứ tự ca **sau `--filter`**, **trước** `--repeat` (lặp vẫn ca-trước-lặp-sau)
//!   và **trước** `--list` (liệt kê theo thứ tự đã xáo).
//! - Cùng seed + cùng tập chọn ⇒ **cùng thứ tự** (tất định); `--shuffle` không seed = seed **0**.
//! - Mỗi ca được chọn xuất hiện **đúng một lần**; TAP/JSON/plan/exit giữ nguyên định dạng.
//! - Không cờ ⇒ **nguyên byte** TAP/JSON/thứ tự cũ.
//! - `--seed` thiếu giá trị / không phải u64 / tràn / **không kèm `--shuffle`** ⇒ exit **2**,
//!   stderr nêu `--seed` (và `--shuffle` khi thiếu cờ kèm).
//!
//! **RED-BEFORE**: trên HEAD `8916490` hai cờ chưa tồn tại ⇒ `vis test --shuffle …` in
//! ``vis test: không hỗ trợ cờ `--shuffle` `` + usage, exit **2** (tương tự `--seed`);
//! chạy tệp này trước khi sửa: **6/6 ca đỏ** (ca `seed_sai_exit_2` chỉ xanh một phần vì cờ lạ
//! cũng exit 2 — nhưng stderr cũ không nêu `--shuffle` khi thiếu cặp, nên vẫn đỏ).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

fn tam(tag: &str) -> PathBuf {
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!(
        "vis-vd270-shuffle-{tag}-{}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    d
}

fn ghi(d: &Path, ten: &str, src: &str) {
    std::fs::write(d.join(ten), src).expect("ghi nguồn");
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

/// Sáu ca `test fn` — thứ tự tự nhiên (tên tệp đã sắp) là `mot, hai, ba, bon, nam, sau`.
const SAU: [&str; 6] = ["mot", "hai", "ba", "bon", "nam", "sau"];

fn sau_ca(d: &Path) {
    for (i, ten) in SAU.iter().enumerate() {
        ghi(
            d,
            &format!("{i}.vis"),
            &format!("test fn {ten}():\n    assert {i} == {i}\n"),
        );
    }
}

fn tu_nhien() -> Vec<String> {
    SAU.iter().map(|s| s.to_string()).collect()
}

fn da_sap(v: &[String]) -> Vec<String> {
    let mut x = v.to_vec();
    x.sort();
    x
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

/// Tên ca theo **thứ tự chạy**, đọc từ TAP (`ok N - <tên>` / `not ok N - <tên>`).
fn ten_tap(tap: &str) -> Vec<String> {
    tap.lines()
        .filter(|l| l.starts_with("ok ") || l.starts_with("not ok "))
        .map(|l| {
            l.split_once(" - ")
                .unwrap_or_else(|| panic!("dòng TAP thiếu ` - `: {l:?}"))
                .1
                .to_string()
        })
        .collect()
}

/// Tên ca theo thứ tự trong `--json` (`"name":"…"`; tên fixture test không chứa escape).
fn ten_json(json: &str) -> Vec<String> {
    json.split("\"name\":\"")
        .skip(1)
        .map(|s| s.split('"').next().unwrap_or("").to_string())
        .collect()
}

/// Tên ca theo thứ tự trong `--list --json`: `{"tests":["a","b"]}`.
fn ten_list_json(json: &str) -> Vec<String> {
    let body = json
        .trim()
        .strip_prefix("{\"tests\":[")
        .and_then(|s| s.strip_suffix("]}"))
        .unwrap_or_else(|| panic!("JSON --list sai dạng: {json:?}"));
    if body.is_empty() {
        return Vec::new();
    }
    body.split("\",\"")
        .map(|s| s.trim_matches('"').to_string())
        .collect()
}

fn thu_tu_voi_seed(ds: &str, seed: &str) -> Vec<String> {
    let (c, o, e) = vis(&["test", "--shuffle", "--seed", seed, ds]);
    assert_eq!(c, 0, "seed {seed}: {o} {e}");
    kiem_tap(&o);
    ten_tap(&o)
}

/// (1) Không cờ = **nguyên byte** TAP cũ; cùng seed ⇒ cùng thứ tự; đủ ca đúng một lần;
/// `--shuffle` không seed = seed 0; JSON cùng thứ tự TAP. Đây cũng là ca **đỏ trước**
/// (trên HEAD `--shuffle` là cờ lạ ⇒ exit 2).
#[test]
fn cung_seed_cung_thu_tu_va_du_ca() {
    let d = tam("cung-seed");
    sau_ca(&d);
    let ds = d.to_str().unwrap();

    let (c0, o0, e0) = vis(&["test", ds]);
    assert_eq!(c0, 0, "{o0} {e0}");
    assert_eq!(
        o0,
        "TAP version 13\n1..6\n\
         ok 1 - mot\n\
         ok 2 - hai\n\
         ok 3 - ba\n\
         ok 4 - bon\n\
         ok 5 - nam\n\
         ok 6 - sau\n",
        "không cờ phải giữ nguyên TAP cũ"
    );

    let (c1, o1, e1) = vis(&["test", "--shuffle", "--seed", "12345", ds]);
    assert_eq!(c1, 0, "--shuffle không còn là cờ lạ: {o1} {e1}");
    let (c2, o2, e2) = vis(&["test", "--shuffle", "--seed", "12345", ds]);
    assert_eq!(c2, 0, "{o2} {e2}");
    assert_eq!(o1, o2, "cùng seed + cùng tập chọn phải cùng byte");
    kiem_tap(&o1);
    let t1 = ten_tap(&o1);
    assert_eq!(t1.len(), 6, "phải chạy đúng 6 ca: {o1}");
    assert_eq!(da_sap(&t1), da_sap(&tu_nhien()), "mỗi ca đúng một lần");
    assert_ne!(t1, tu_nhien(), "seed 12345 phải xáo trộn thứ tự cũ");

    // `--shuffle` thiếu `--seed` ⇒ seed mặc định 0 (tài liệu ở usage).
    let (c3, o3, e3) = vis(&["test", "--shuffle", ds]);
    assert_eq!(c3, 0, "{o3} {e3}");
    let (c4, o4, e4) = vis(&["test", "--shuffle", "--seed", "0", ds]);
    assert_eq!(c4, 0, "{o4} {e4}");
    assert_eq!(o3, o4, "`--shuffle` không seed phải bằng `--seed 0`");

    let (cj, oj, ej) = vis(&["test", "--json", "--shuffle", "--seed", "12345", ds]);
    assert_eq!(cj, 0, "{oj} {ej}");
    assert_eq!(ten_json(&oj), t1, "JSON phải cùng thứ tự với TAP");
    assert!(
        oj.starts_with("{\"total\":6,\"ok\":6,\"failed\":0,\"cases\":["),
        "JSON phải giữ định dạng cũ: {oj}"
    );

    let _ = std::fs::remove_dir_all(&d);
}

/// (2) Hai seed khác nhau trên **tập 6 ca** ⇒ thứ tự khác nhau; mỗi bên vẫn đủ ca.
#[test]
fn seed_khac_thu_tu_khac() {
    let d = tam("seed-khac");
    sau_ca(&d);
    let ds = d.to_str().unwrap();

    let t1 = thu_tu_voi_seed(ds, "1");
    let t2 = thu_tu_voi_seed(ds, "2");
    assert_ne!(t1, t2, "seed 1 và 2 phải cho thứ tự khác: {t1:?} / {t2:?}");
    for t in [&t1, &t2] {
        assert_eq!(da_sap(t), da_sap(&tu_nhien()), "mỗi ca đúng một lần");
    }

    let _ = std::fs::remove_dir_all(&d);
}

/// (3) `--filter` chọn **trước** rồi mới xáo: chỉ ca khớp chạy, mỗi ca một lần, cùng seed
/// tất định, và `--list` cùng seed cho đúng thứ tự đó.
#[test]
fn filter_roi_moi_xao() {
    let d = tam("loc");
    ghi(&d, "0.vis", "test fn ca_mot():\n    assert 1 == 1\n");
    ghi(&d, "1.vis", "test fn ca_hai():\n    assert 2 == 2\n");
    ghi(&d, "2.vis", "test fn ca_ba():\n    assert 3 == 3\n");
    ghi(&d, "3.vis", "test fn khac_bon():\n    assert 4 == 4\n");
    ghi(&d, "4.vis", "test fn khac_nam():\n    assert 5 == 5\n");
    ghi(&d, "5.vis", "test fn khac_sau():\n    assert 6 == 6\n");
    let ds = d.to_str().unwrap();

    let (c, o, e) = vis(&["test", "--filter", "ca_", "--shuffle", "--seed", "3", ds]);
    assert_eq!(c, 0, "{o} {e}");
    kiem_tap(&o);
    let t = ten_tap(&o);
    assert_eq!(
        da_sap(&t),
        vec![
            "ca_ba".to_string(),
            "ca_hai".to_string(),
            "ca_mot".to_string()
        ],
        "chỉ 3 ca khớp `ca_`, mỗi ca một lần: {o}"
    );
    assert!(!o.contains("khac_"), "ca bị lọc không được chạy: {o}");

    let (c2, o2, e2) = vis(&["test", "--filter", "ca_", "--shuffle", "--seed", "3", ds]);
    assert_eq!(c2, 0, "{o2} {e2}");
    assert_eq!(o, o2, "cùng seed + cùng tập lọc phải cùng byte");

    let (cl, ol, el) = vis(&[
        "test",
        "--list",
        "--filter",
        "ca_",
        "--shuffle",
        "--seed",
        "3",
        ds,
    ]);
    assert_eq!(cl, 0, "{ol} {el}");
    let ds_list: Vec<String> = ol.lines().map(str::to_string).collect();
    assert_eq!(ds_list, t, "`--list` phải theo thứ tự đã xáo");

    // Lọc rỗng + xáo: vẫn TAP hợp lệ `1..0`, exit 0.
    let (cr, or, er) = vis(&[
        "test",
        "--filter",
        "khong_co",
        "--shuffle",
        "--seed",
        "9",
        ds,
    ]);
    assert_eq!(cr, 0, "{or} {er}");
    assert_eq!(or, "TAP version 13\n1..0\n");

    let _ = std::fs::remove_dir_all(&d);
}

/// (4) `--list` (và `--list --json`) in **thứ tự đã xáo**, khớp thứ tự chạy cùng seed; không
/// cờ `--list` vẫn giữ thứ tự cũ.
#[test]
fn list_theo_thu_tu_da_xao() {
    let d = tam("list");
    sau_ca(&d);
    let ds = d.to_str().unwrap();

    let (c0, o0, e0) = vis(&["test", "--list", ds]);
    assert_eq!(c0, 0, "{o0} {e0}");
    assert_eq!(
        o0.lines().collect::<Vec<_>>(),
        tu_nhien(),
        "không cờ phải giữ thứ tự list cũ"
    );

    let (c1, o1, e1) = vis(&["test", "--list", "--shuffle", "--seed", "7", ds]);
    assert_eq!(c1, 0, "{o1} {e1}");
    let ls: Vec<String> = o1.lines().map(str::to_string).collect();
    let chay = thu_tu_voi_seed(ds, "7");
    assert_eq!(ls, chay, "`--list` phải cùng thứ tự đã xáo với lần chạy");

    let (cj, oj, ej) = vis(&["test", "--list", "--json", "--shuffle", "--seed", "7", ds]);
    assert_eq!(cj, 0, "{oj} {ej}");
    assert_eq!(ten_list_json(&oj), chay, "`--list --json` cùng thứ tự");

    let _ = std::fs::remove_dir_all(&d);
}

/// (5) `--repeat 2` vẫn **ca-trước-lặp-sau** trên thứ tự đã xáo: mỗi ca hai lần liền nhau
/// `[1/2]`, `[2/2]`, rồi ca kế; plan `1..12`; thứ tự ca = lần chạy không lặp cùng seed.
#[test]
fn repeat_van_ca_truoc_lap_sau() {
    let d = tam("repeat");
    sau_ca(&d);
    let ds = d.to_str().unwrap();

    let thu_tu = thu_tu_voi_seed(ds, "99");
    let (c, o, e) = vis(&["test", "--shuffle", "--seed", "99", "--repeat", "2", ds]);
    assert_eq!(c, 0, "{o} {e}");
    let ls = kiem_tap(&o);
    assert_eq!(ls[1], "1..12", "plan = 6 ca × 2 lần: {o}");

    let mut mong: Vec<String> = Vec::new();
    for ten in &thu_tu {
        for k in 1..=2 {
            mong.push(format!("{ten} [{k}/2]"));
        }
    }
    assert_eq!(ten_tap(&o), mong, "lặp phải ca-trước-lặp-sau: {o}");

    let (cj, oj, ej) = vis(&[
        "test",
        "--json",
        "--shuffle",
        "--seed",
        "99",
        "--repeat",
        "2",
        ds,
    ]);
    assert_eq!(cj, 0, "{oj} {ej}");
    assert_eq!(ten_json(&oj), mong, "JSON phải cùng thứ tự lặp");
    assert!(
        oj.starts_with("{\"total\":12,\"ok\":12,\"failed\":0,"),
        "JSON đếm lần lặp: {oj}"
    );

    let _ = std::fs::remove_dir_all(&d);
}

/// (6) Seed sai/thiếu/tràn/không kèm `--shuffle` ⇒ exit **2**; stderr nêu `--seed` (và
/// `--shuffle` khi thiếu cặp). `--help` + root usage nêu cả hai cờ.
#[test]
fn seed_sai_exit_2() {
    let d = tam("sai");
    sau_ca(&d);
    let ds = d.to_str().unwrap();

    let thieu = vec!["test", "--shuffle", "--seed"];
    let chu = vec!["test", "--shuffle", "--seed", "abc", ds];
    let am = vec!["test", "--shuffle", "--seed", "-1", ds];
    let thap_phan = vec!["test", "--shuffle", "--seed", "1.5", ds];
    let tran = vec!["test", "--shuffle", "--seed", "18446744073709551616", ds];
    for args in [thieu, chu, am, thap_phan, tran] {
        let (c, _, e) = vis(&args);
        assert_eq!(c, 2, "{args:?}: {e}");
        assert!(e.contains("--seed"), "{args:?}: {e}");
    }

    // `--seed` không kèm `--shuffle` ⇒ exit 2 (cả dạng `--seed N` và `--seed=N`).
    for args in [
        vec!["test", "--seed", "5", ds],
        vec!["test", "--seed=5", ds],
        vec!["test", "--seed", "0", "--list", ds],
    ] {
        let (c, _, e) = vis(&args);
        assert_eq!(c, 2, "{args:?}: {e}");
        assert!(e.contains("--seed"), "{args:?}: {e}");
        assert!(e.contains("--shuffle"), "phải nêu cờ cần kèm: {e}");
    }

    // u64 lớn nhất vẫn hợp lệ.
    let (cm, om, em) = vis(&[
        "test",
        "--shuffle",
        "--seed",
        "18446744073709551615",
        "--list",
        ds,
    ]);
    assert_eq!(cm, 0, "{om} {em}");
    assert_eq!(om.lines().count(), 6);

    // `--help` và root usage nêu cờ mới.
    let (ch, _, eh) = vis(&["test", "--help"]);
    assert_eq!(ch, 2, "--help vẫn là lỗi dùng");
    assert!(eh.contains("--shuffle"), "usage phải nêu --shuffle: {eh}");
    assert!(eh.contains("--seed"), "usage phải nêu --seed: {eh}");
    assert!(
        eh.contains("seed 0"),
        "usage phải nêu seed mặc định 0: {eh}"
    );

    let (cr, _, er) = vis(&[]);
    assert_eq!(cr, 2);
    assert!(er.contains("--shuffle"), "root usage: {er}");
    assert!(er.contains("--seed"), "root usage: {er}");

    let _ = std::fs::remove_dir_all(&d);
}
