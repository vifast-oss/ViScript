// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-114b** — chẩn đoán đa tệp phải gắn đúng **tệp + dòng/cột** (nợ ST-112).
//! Mỗi ca chạy `vis check --json`; parse bằng python3 độc lập; ca TAP kiểm dòng `not ok`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn tam(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!(
        "vis-da-tep-{tag}-{}-{}",
        std::process::id(),
        DEM.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tmp");
    d
}

fn ghi(d: &Path, ten: &str, src: &str) {
    std::fs::write(d.join(ten), src).expect("ghi");
}

fn vis(args: &[&str]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(args)
        .output()
        .expect("vis");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// In mỗi tệp một dòng: `tên|số_diag|code|line|col` (rỗng nếu không lỗi).
fn parse_json(json: &str) -> String {
    use std::io::Write;
    use std::process::Stdio;
    let mut c = Command::new("python3")
        .arg("-c")
        .arg(
            r#"import json,sys
d=json.load(sys.stdin)
for f in d["files"]:
    n=f["file"].rsplit("/",1)[-1]
    ds=f["diagnostics"]
    if ds:
        x=ds[0]
        print(f"{n}|{len(ds)}|{x['code']}|{x['line']}|{x['col']}|{x['severity']}")
    else:
        print(f"{n}|0|||")"#,
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("python3");
    c.stdin.take().unwrap().write_all(json.as_bytes()).unwrap();
    let o = c.wait_with_output().expect("python3");
    assert!(
        o.status.success(),
        "python3: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8_lossy(&o.stdout).to_string()
}

fn check_json(d: &Path) -> String {
    let (c, o, e) = vis(&["check", "--json", d.join("a.vis").to_str().unwrap()]);
    assert_eq!(c, 1, "stderr: {e}");
    assert_eq!(e, "");
    o
}

/// (a) lỗi **cấp tệp** trong a.vis ⇒ a.vis đúng `2:14`; b.vis rỗng.
#[test]
fn loi_cap_tep_a() {
    let d = tam("a");
    ghi(&d, "a.vis", "use ./b\nlet x: int = \"s\"\n");
    ghi(&d, "b.vis", "pub fn f() -> int:\n    return 1\n");
    let out = parse_json(&check_json(&d));
    assert_eq!(out, "a.vis|1|VS0412|2|14|error\nb.vis|0|||\n", "{out}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (b) lỗi trong `fn main` của a.vis ⇒ a.vis.
#[test]
fn loi_trong_fn_a() {
    let d = tam("fn");
    ghi(&d, "a.vis", "use ./b\nfn main():\n    let x: int = \"s\"\n");
    ghi(&d, "b.vis", "pub fn f() -> int:\n    return 1\n");
    let out = parse_json(&check_json(&d));
    assert_eq!(out, "a.vis|1|VS0412|3|18|error\nb.vis|0|||\n", "{out}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (c) lỗi trong b.vis ở dòng ≥5 ⇒ b.vis, line/col đúng trên b (dòng 5, cột 5).
#[test]
fn loi_trong_b_dong_sau() {
    let d = tam("b");
    ghi(&d, "a.vis", "use ./b\nprint(1)\n");
    ghi(&d, "b.vis", "pub fn f() -> int:\n\n\n\n    return \"s\"\n");
    let out = parse_json(&check_json(&d));
    assert_eq!(out, "a.vis|0|||\nb.vis|1|VS0412|5|5|error\n", "{out}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (d) lỗi **cả hai** ⇒ mỗi tệp đúng dòng/cột của nó.
#[test]
fn loi_ca_hai_tep() {
    let d = tam("hai");
    ghi(&d, "a.vis", "use ./b\nlet x: int = \"s\"\n");
    ghi(&d, "b.vis", "pub fn f() -> int:\n\n\n\n    return \"t\"\n");
    let out = parse_json(&check_json(&d));
    assert_eq!(
        out, "a.vis|1|VS0412|2|14|error\nb.vis|1|VS0412|5|5|error\n",
        "{out}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// (e) TAP (`check` không `--json`) in đúng tệp cho ca (a) và (d).
#[test]
fn tap_dung_tep() {
    let d = tam("tap");
    ghi(&d, "a.vis", "use ./b\nlet x: int = \"s\"\n");
    ghi(&d, "b.vis", "pub fn f() -> int:\n    return 1\n");
    let (c, o, e) = {
        let o = Command::new(env!("CARGO_BIN_EXE_vis"))
            .args(["check", "a.vis"])
            .current_dir(&d)
            .output()
            .expect("vis");
        (
            o.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&o.stdout).to_string(),
            String::from_utf8_lossy(&o.stderr).to_string(),
        )
    };
    assert_eq!(c, 1, "{e}");
    assert_eq!(e, "");
    assert!(o.starts_with("TAP version 13\n1..1\nnot ok 1 - "), "{o}");
    assert!(
        o.lines().any(|l| l.contains("not ok 1 - a.vis")) && !o.contains("(b.vis:"),
        "TAP phải in a.vis, không b.vis: {o}"
    );
    // (d) cả hai: chẩn đoán **đầu** phải là a.vis cấp tệp, không phải b.vis.
    ghi(&d, "b.vis", "pub fn f() -> int:\n\n\n\n    return \"t\"\n");
    let (c2, o2, _e2) = {
        let o = Command::new(env!("CARGO_BIN_EXE_vis"))
            .args(["check", "a.vis"])
            .current_dir(&d)
            .output()
            .expect("vis");
        (
            o.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&o.stdout).to_string(),
            String::from_utf8_lossy(&o.stderr).to_string(),
        )
    };
    assert_eq!(c2, 1);
    assert!(
        o2.lines().any(|l| l.contains("not ok 1 - a.vis"))
            && !o2.lines().next().unwrap_or("").contains("b.vis"),
        "TAP đầu phải a.vis: {o2}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// (f) b.vis **dài ≥100 dòng** + lỗi a.vis cấp tệp ⇒ vẫn a.vis (ca bắt heuristic span cũ).
#[test]
fn b_dai_van_dung_a() {
    let d = tam("dai");
    ghi(&d, "a.vis", "use ./b\nlet x: int = \"s\"\n");
    let mut b = String::from("pub fn f() -> int:\n");
    for i in 0..120 {
        b.push_str(&format!("    let v{i} = {i}\n"));
    }
    b.push_str("    return 1\n");
    ghi(&d, "b.vis", &b);
    let out = parse_json(&check_json(&d));
    assert_eq!(out, "a.vis|1|VS0412|2|14|error\nb.vis|0|||\n", "{out}");
    let _ = std::fs::remove_dir_all(&d);
}
