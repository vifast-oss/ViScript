// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `vis fmt --check --diff` (một hunk, `---`/`+++`/`@@`).

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam() -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-fmt-diff-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tmp");
    d
}

fn chay(args: &[&str], stdin: &[u8]) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("chạy vis");
    {
        let mut vao = c.stdin.take().expect("stdin");
        if let Err(e) = vao.write_all(stdin) {
            if e.kind() != std::io::ErrorKind::BrokenPipe {
                panic!("ghi stdin: {e}");
            }
        }
    }
    let o = c.wait_with_output().expect("chờ vis");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

#[test]
fn diff_tep_tab_thanh_bon_cach() {
    let d = tam();
    let p = d.join("a.vis");
    std::fs::write(&p, "fn main():\n\tprint(1)\n").expect("ghi");
    let f = p.to_string_lossy().to_string();
    let (ma, out, err) = chay(&["fmt", "--check", "--diff", &f], b"");
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(ma, 1, "stderr: {err}");
    assert!(out.contains(&format!("--- {f}")), "thiếu ---: {out}");
    assert!(out.contains(&format!("+++ {f}")), "thiếu +++: {out}");
    assert!(out.contains("@@ -2,1 +2,1 @@"), "thiếu hunk: {out}");
    assert!(out.contains("-\tprint(1)"), "thiếu dòng cũ: {out}");
    assert!(out.contains("+    print(1)"), "thiếu dòng mới: {out}");
}

#[test]
fn diff_tep_da_dung_rong_exit_0() {
    let d = tam();
    let p = d.join("a.vis");
    std::fs::write(&p, "fn main():\n    print(1)\n").expect("ghi");
    let f = p.to_string_lossy().to_string();
    let (ma, out, err) = chay(&["fmt", "--check", "--diff", &f], b"");
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(out, "");
}

#[test]
fn diff_stdin_check() {
    let (ma, out, err) = chay(
        &["fmt", "--stdin", "--check", "--diff"],
        b"fn main():\n\tprint(1)\n",
    );
    assert_eq!(ma, 1, "stderr: {err}");
    assert!(out.starts_with("--- -\n+++ -\n@@ -2,1 +2,1 @@"), "{out}");
    assert!(out.contains("-\tprint(1)"), "{out}");
    assert!(out.contains("+    print(1)"), "{out}");
}

#[test]
fn diff_stdin_da_dung_rong_exit_0() {
    let (ma, out, err) = chay(
        &["fmt", "--stdin", "--check", "--diff"],
        b"fn main():\n    print(1)\n",
    );
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(out, "");
}

#[test]
fn diff_cu_phap_hong_exit_2() {
    let d = tam();
    let p = d.join("a.vis");
    std::fs::write(&p, "fn main(:\n").expect("ghi");
    let f = p.to_string_lossy().to_string();
    let (ma, out, err) = chay(&["fmt", "--check", "--diff", &f], b"");
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(ma, 2, "stderr: {err}");
    assert_eq!(out, "");
    assert!(err.contains("VS0201"), "stderr: {err}");
}

#[test]
fn diff_to_hop_sai_exit_2() {
    for args in [
        &["fmt", "--diff", "a.vis"][..],
        &["fmt", "--check", "--diff", "--json", "a.vis"][..],
        &["fmt", "--check", "--diff", "--list", "a.vis"][..],
        &["fmt", "--diff", "--write", "a.vis"][..],
    ] {
        let (ma, out, err) = chay(args, b"");
        assert_eq!(ma, 2, "args {args:?} phải exit 2; stderr: {err}");
        assert_eq!(out, "", "args {args:?}");
        assert!(err.contains("vis fmt"), "args {args:?}: {err}");
    }
}
