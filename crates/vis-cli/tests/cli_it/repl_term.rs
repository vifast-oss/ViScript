// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310b (VD-285)** — `vis repl` dùng `vish-edit` + `vish-term`:
//! - **PTY giả** = chuỗi byte qua `run_with` (editor thật, không cần tty của HĐH) — xanh 3 OS.
//! - Non-TTY = ống thật: stdout **sạch** (không ANSI, không khung), stderr chỉ chẩn đoán.
//! - `--format viterm` = khung JSON Lines `vish-term/1` (`hello` … `bye`).

use std::io::Write;
use std::process::{Command, Stdio};

fn chay_vis(args: &[&str], stdin: &str) -> (i32, String, String) {
    let mut con = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn vis");
    con.stdin
        .take()
        .expect("stdin")
        .write_all(stdin.as_bytes())
        .expect("ghi stdin");
    let o = con.wait_with_output().expect("chờ vis");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

#[test]
fn non_tty_pipe_sach_khong_prompt_khong_ansi() {
    let (ma, out, err) = chay_vis(&["repl"], "print(1)\n:type 1\n:quit\n");
    assert_eq!(ma, 0);
    assert_eq!(out, "1\nint\n", "stdout pipe: {out:?}");
    assert!(!out.contains('\x1b'));
    assert!(!out.contains("\"t\":"));
    assert!(!err.contains("vis> "), "non-TTY không prompt: {err:?}");
    assert!(!err.contains(":help"), "non-TTY không lời chào: {err:?}");
}

#[test]
fn format_viterm_qua_ong_mo_hello_dong_bye() {
    let (ma, out, err) = chay_vis(
        &["repl", "--format", "viterm"],
        "print(1)\nprint(1 +)\n:quit\n",
    );
    assert_eq!(ma, 0, "stderr: {err}");
    let lines: Vec<&str> = out.lines().collect();
    assert!(
        lines[0].starts_with(r#"{"v":1,"t":"hello","proto":"vish-term/1""#),
        "khung đầu phải hello: {out:?}"
    );
    assert!(lines.iter().any(|l| l.contains(r#""t":"text""#)));
    assert!(lines.iter().any(|l| l.contains(r#""t":"diag""#)));
    assert_eq!(lines.last().copied(), Some(r#"{"v":1,"t":"bye"}"#));
    assert!(
        lines.iter().all(|l| l.starts_with(r#"{"v":1,"t":""#)),
        "mọi dòng stdout phải là khung: {out:?}"
    );
    assert!(err.is_empty(), "viterm không rò stderr: {err:?}");
}

#[test]
fn format_viterm_dang_dau_bang() {
    let (ma, out, _err) = chay_vis(&["repl", "--format=viterm"], "print(1)\n:quit\n");
    assert_eq!(ma, 0);
    assert!(out.starts_with(r#"{"v":1,"t":"hello""#), "{out:?}");
}

#[test]
fn format_la_la_loi_dung_exit_2() {
    let (ma, out, err) = chay_vis(&["repl", "--format", "xml"], ":quit\n");
    assert_eq!(ma, 2);
    assert!(out.is_empty(), "{out:?}");
    assert!(err.contains("--format"), "{err:?}");
    assert!(err.contains("text|viterm"), "{err:?}");
}
