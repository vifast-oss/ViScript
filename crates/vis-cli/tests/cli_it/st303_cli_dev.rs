// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-303 (VD-283)** — CLI dev: `vis new`, `vis doc`, `vis repl` trên Linux.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-st303-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tmp");
    d
}

fn chay(d: &Path, args: &[&str]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .current_dir(d)
        .args(args)
        .output()
        .expect("chạy vis");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

fn kiem_du_an(p: &Path) -> (i32, String, String) {
    let (c, o, e) = chay(p, &["check", "main.vis"]);
    assert_eq!(c, 0, "check stderr: {e}");
    assert!(o.contains("ok 1 - main.vis"), "{o}");
    let (c, o, e) = chay(p, &["test"]);
    assert_eq!(c, 0, "test stderr: {e}");
    assert!(o.contains("ok 1"), "{o}");
    let (c, o, e) = chay(p, &["run", "main.vis"]);
    assert_eq!(c, 0, "run stderr: {e}");
    assert_eq!(o, "hello from ViScript\n");
    (c, o, e)
}

#[test]
fn new_bin_du_an_chay_check_test_run() {
    let d = tam("new-bin");
    let (c, _o, e) = chay(&d, &["new", "proj", "--template", "bin"]);
    assert_eq!(c, 0, "new stderr: {e}");
    let p = d.join("proj");
    assert!(p.join("main.vis").is_file());
    assert!(p.join("test/sem/smoke.vis").is_file());
    kiem_du_an(&p);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn new_lib_du_an_chay_check_test_run() {
    let d = tam("new-lib");
    let (c, _o, e) = chay(&d, &["new", "proj", "--template", "lib"]);
    assert_eq!(c, 0, "new stderr: {e}");
    let p = d.join("proj");
    assert!(p.join("lib.vis").is_file());
    kiem_du_an(&p);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn new_tu_choi_thu_muc_ton_tai() {
    let d = tam("new-existing");
    std::fs::create_dir(d.join("co")).unwrap();
    let (c, _o, e) = chay(&d, &["new", "co"]);
    assert_eq!(c, 2);
    assert!(e.contains("đã tồn tại"), "{e}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn new_tu_choi_template_la() {
    let d = tam("new-template");
    let (c, _o, e) = chay(&d, &["new", "proj", "--template", "la"]);
    assert_eq!(c, 2);
    assert!(e.contains("--template"), "{e}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn doc_md_va_json_bang_cst() {
    let d = tam("doc");
    let src = "## Tài liệu hàm\n## Dòng hai\nfn cong(a: int, b: int) -> int:\n    a + b\n\n## Kiểu điểm\ntype Point:\n    x: int\n";
    std::fs::write(d.join("a.vis"), src).unwrap();
    let (c, o, e) = chay(&d, &["doc", "a.vis"]);
    assert_eq!(c, 0, "stderr: {e}");
    assert!(o.contains("Tài liệu hàm") && o.contains("Dòng hai"), "{o}");
    assert!(o.contains("fn cong") && o.contains("Kiểu điểm"), "{o}");
    let (c, o, e) = chay(&d, &["doc", "a.vis", "--format", "json"]);
    assert_eq!(c, 0, "stderr: {e}");
    assert!(o.starts_with('{'), "{o}");
    assert!(o.contains(r#""items""#), "{o}");
    assert!(o.contains(r#""doc":["Tài liệu hàm","Dòng hai"]"#), "{o}");
    let (c, _o, e) = chay(&d, &["doc", "a.vis", "--format", "xml"]);
    assert_eq!(c, 2);
    assert!(e.contains("--format"), "{e}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn doc_tu_choi_file_khong_ton_tai() {
    let d = tam("doc-missing");
    let (c, _o, e) = chay(&d, &["doc", "khong-co.vis"]);
    assert_eq!(c, 2);
    assert!(e.contains("không đọc được"), "{e}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn repl_giu_trang_thai_va_rollback() {
    let d = tam("repl");
    let mut con = Command::new(env!("CARGO_BIN_EXE_vis"))
        .current_dir(&d)
        .arg("repl")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn repl");
    con.stdin
        .take()
        .unwrap()
        .write_all(
            b"let x = 1\nprint(x)\nlet y: int = \"s\"\nprint(x + 1)\n:type x\n:reset\nprint(2)\n:quit\n",
        )
        .unwrap();
    let o = con.wait_with_output().expect("repl");
    assert_eq!(o.status.code(), Some(0));
    let out = String::from_utf8_lossy(&o.stdout);
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(out.contains("1\n2\nint\n"), "stdout: {out}");
    assert!(out.trim_end().ends_with('2'), "stdout: {out}");
    assert!(err.contains("VS0412"), "stderr: {err}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn repl_help_va_quit() {
    let d = tam("repl-help");
    let mut con = Command::new(env!("CARGO_BIN_EXE_vis"))
        .current_dir(&d)
        .arg("repl")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn repl");
    con.stdin
        .take()
        .unwrap()
        .write_all(b":help\n:quit\n")
        .unwrap();
    let o = con.wait_with_output().expect("repl");
    assert_eq!(o.status.code(), Some(0));
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(out.contains(":type") && out.contains(":reset") && out.contains(":quit"));
    let _ = std::fs::remove_dir_all(&d);
}
