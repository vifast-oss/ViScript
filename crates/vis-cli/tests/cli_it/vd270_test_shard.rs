// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `vis test --shard <I>/<N>`: chỉ chạy phần `I` của `N` phần rời nhau.
//!
//! Khoá hành vi:
//! - Chia **sau** `--filter` và `--shuffle`, **trước** `--list`/`--repeat`/`--json`.
//! - Phần `I` giữ ca có chỉ số 0-based `idx % N == I-1`; `N` phần rời nhau, hợp = đúng tập đã chọn.
//! - `1 ≤ I ≤ N`; `0/2`, `3/2`, `1/0`, thiếu `/`, thừa `/`, không nguyên, thiếu giá trị, lặp cờ
//!   ⇒ exit **2** + usage nêu `--shard`.
//! - `--list`/`--json`/`--repeat` áp **sau** shard; `--shard 1/1` giữ nguyên hành vi cũ.
//!
//! **RED-BEFORE**: trên HEAD `a1699d9` cờ chưa tồn tại ⇒ `vis test --shard 1/2 …` in
//! `vis test: không hỗ trợ cờ '--shard'` và exit **2**.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

fn tam(tag: &str) -> PathBuf {
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-shard-{tag}-{}-{n}", std::process::id()));
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

/// 5 ca `mot..nam` theo thứ tự tên tệp `a.vis..e.vis` (gom sắp theo đường dẫn).
fn nam_ca(d: &Path) {
    for (f, ten) in [
        ("a.vis", "mot"),
        ("b.vis", "hai"),
        ("c.vis", "ba"),
        ("d.vis", "bon"),
        ("e.vis", "nam"),
    ] {
        ghi(d, f, &format!("test fn {ten}():\n    assert true\n"));
    }
}

/// (1) `1/2` và `2/2` rời nhau, hợp đúng 5 ca; `1/1` = toàn bộ.
#[test]
fn shard_chia_roi_va_hop_du() {
    let d = tam("chia");
    nam_ca(&d);
    let ds = d.to_str().unwrap();

    let (c0, o0, e0) = vis(&["test", "--list", ds]);
    assert_eq!(c0, 0, "{o0} {e0}");
    let tat: Vec<String> = o0.lines().map(str::to_string).collect();
    assert_eq!(tat, ["mot", "hai", "ba", "bon", "nam"]);

    let (c1, o1, e1) = vis(&["test", "--list", "--shard", "1/2", ds]);
    assert_eq!(c1, 0, "{o1} {e1}");
    assert_eq!(o1.lines().collect::<Vec<_>>(), ["mot", "ba", "nam"]);

    let (c2, o2, e2) = vis(&["test", "--list", "--shard", "2/2", ds]);
    assert_eq!(c2, 0, "{o2} {e2}");
    assert_eq!(o2.lines().collect::<Vec<_>>(), ["hai", "bon"]);

    let (c11, o11, _) = vis(&["test", "--list", "--shard", "1/1", ds]);
    assert_eq!(c11, 0);
    assert_eq!(o11, o0, "--shard 1/1 phải giữ nguyên toàn bộ");

    let mut hop = o1
        .lines()
        .chain(o2.lines())
        .map(str::to_string)
        .collect::<Vec<_>>();
    hop.sort();
    let mut tat_sap = tat.clone();
    tat_sap.sort();
    assert_eq!(hop, tat_sap, "hợp hai phần phải bằng tập đã chọn");
    let _ = std::fs::remove_dir_all(&d);
}

/// (2) `--shard=1/2` tương đương dạng tách; `--json` đếm đúng phần đã chọn.
#[test]
fn shard_bang_va_json() {
    let d = tam("json");
    nam_ca(&d);
    let ds = d.to_str().unwrap();

    let (c, o, e) = vis(&["test", "--list", "--shard=1/2", ds]);
    assert_eq!(c, 0, "{o} {e}");
    assert_eq!(o.lines().collect::<Vec<_>>(), ["mot", "ba", "nam"]);

    let (cj, oj, ej) = vis(&["test", "--json", "--shard", "1/2", ds]);
    assert_eq!(cj, 0, "{oj} {ej}");
    assert_eq!(
        oj,
        "{\"total\":3,\"ok\":3,\"failed\":0,\"cases\":[{\"name\":\"mot\",\"ok\":true,\"message\":\"\"},{\"name\":\"ba\",\"ok\":true,\"message\":\"\"},{\"name\":\"nam\",\"ok\":true,\"message\":\"\"}]}\n"
    );

    let (c2, o2, _) = vis(&["test", "--json", "--shard", "2/2", ds]);
    assert_eq!(c2, 0);
    assert!(
        o2.starts_with("{\"total\":2,\"ok\":2,\"failed\":0,"),
        "{o2}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// (3) Cú pháp/miền sai và lặp cờ ⇒ exit 2, usage nêu `--shard`.
#[test]
fn shard_sai_exit_2() {
    let d = tam("sai");
    ghi(&d, "a.vis", "test fn mot():\n    assert true\n");
    let ds = d.to_str().unwrap();
    for v in ["0/2", "3/2", "1/0", "x/2", "1", "1/2/3", "", "/2", "1/"] {
        let (c, o, e) = vis(&["test", "--shard", v, ds]);
        assert_eq!(c, 2, "`{v}` phải exit 2: {o} {e}");
        assert!(e.contains("--shard"), "`{v}`: stderr phải nêu --shard: {e}");
    }
    let (c, o, e) = vis(&["test", "--shard", "1/2", "--shard", "2/2", ds]);
    assert_eq!(c, 2, "lặp cờ phải exit 2: {o} {e}");
    assert!(e.contains("lặp lại"), "{e}");
    let (cm, om, em) = vis(&["test", "--shard"]);
    assert_eq!(cm, 2, "{om} {em}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (4) `--filter` áp trước shard; `--repeat` áp sau shard; shard rỗng ⇒ `1..0` exit 0.
#[test]
fn shard_voi_filter_va_repeat() {
    let d = tam("filter");
    nam_ca(&d);
    let ds = d.to_str().unwrap();

    // Chỉ `mot` còn sau filter; shard 2/2 chọn chỉ số 1 ⇒ rỗng.
    let (c, o, e) = vis(&["test", "--filter", "mot", "--shard", "2/2", ds]);
    assert_eq!(c, 0, "{o} {e}");
    assert_eq!(o, "TAP version 13\n1..0\n");

    let (cr, or, er) = vis(&["test", "--shard", "1/2", "--repeat", "2", ds]);
    assert_eq!(cr, 0, "{or} {er}");
    assert!(or.contains("1..6"), "3 ca × 2 lần: {or}");
    for ten in [
        "mot [1/2]",
        "mot [2/2]",
        "ba [1/2]",
        "ba [2/2]",
        "nam [1/2]",
        "nam [2/2]",
    ] {
        assert!(or.contains(ten), "thiếu `{ten}`: {or}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// (5) `--shuffle --seed` tất định: 3 phần rời nhau, hợp đủ, chạy lại cùng seed cùng thứ tự.
#[test]
fn shard_shuffle_tat_dinh() {
    let d = tam("shuffle");
    nam_ca(&d);
    let ds = d.to_str().unwrap();

    let mut hop: Vec<String> = Vec::new();
    for i in 1..=3 {
        let s = format!("{i}/3");
        let (c, o, e) = vis(&[
            "test",
            "--list",
            "--shuffle",
            "--seed",
            "42",
            "--shard",
            &s,
            ds,
        ]);
        assert_eq!(c, 0, "{o} {e}");
        let (c2, o2, _) = vis(&[
            "test",
            "--list",
            "--shuffle",
            "--seed",
            "42",
            "--shard",
            &s,
            ds,
        ]);
        assert_eq!(c2, 0);
        assert_eq!(o, o2, "cùng seed + cùng shard phải cùng thứ tự");
        hop.extend(o.lines().map(str::to_string));
    }
    hop.sort();
    assert_eq!(hop, ["ba", "bon", "hai", "mot", "nam"]);
    let _ = std::fs::remove_dir_all(&d);
}

/// (6) Help và usage gốc đều nêu `--shard`.
#[test]
fn shard_usage() {
    let (c, o, e) = vis(&["test", "--help"]);
    assert_eq!(c, 2, "{o} {e}");
    assert!(e.contains("--shard"), "usage test phải nêu --shard: {e}");
    let (cr, or, er) = vis(&[]);
    assert_eq!(cr, 2, "{or} {er}");
    assert!(er.contains("--shard"), "usage gốc phải nêu --shard: {er}");
}
