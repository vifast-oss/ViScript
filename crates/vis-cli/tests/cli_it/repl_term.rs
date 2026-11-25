// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310b (VD-285)** — `vis repl` dùng `vish-edit` + `vish-term`:
//! - **PTY giả** = chuỗi byte qua `run_with` (editor thật, không cần tty của HĐH) — xanh 3 OS.
//! - Non-TTY = ống thật: stdout **sạch** (không ANSI, không khung), stderr chỉ chẩn đoán.
//! - `--format viterm` = khung JSON Lines `vish-term/1` (`hello` … `bye`).

use std::io::{Cursor, Write};
use std::process::{Command, Stdio};

use vis_cli::repl::{run_with, Format};
use vish_edit::History;

fn chay_phien(input: &str, tty: bool, format: Format) -> (i32, String, String) {
    let mut inp = Cursor::new(input.as_bytes().to_vec());
    let mut out = Vec::new();
    let mut err = Vec::new();
    let ma = run_with(
        &mut inp,
        &mut out,
        &mut err,
        tty,
        format,
        History::default(),
    )
    .expect("phiên repl");
    (
        ma,
        String::from_utf8(out).expect("stdout UTF-8"),
        String::from_utf8(err).expect("stderr UTF-8"),
    )
}

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
fn pty_gia_editor_text_va_prompt_stderr() {
    let (ma, out, err) = chay_phien("print(1)\r:quit\r", true, Format::Text);
    assert_eq!(ma, 0);
    assert_eq!(out, "1\n", "stdout chỉ có kết quả chương trình");
    assert!(!out.contains('\x1b'), "stdout không được có ANSI: {out:?}");
    assert!(err.contains("vis> "), "prompt phải ở stderr: {err:?}");
    assert!(err.contains(":help"), "lời chào ở stderr: {err:?}");
}

#[test]
fn pty_gia_editor_giu_trang_thai_va_rollback() {
    let input =
        "let x = 1\rprint(x)\rlet y: int = \"s\"\rprint(x + 1)\r:type x\r:reset\rprint(2)\r:quit\r";
    let (ma, out, err) = chay_phien(input, true, Format::Text);
    assert_eq!(ma, 0);
    assert_eq!(
        out, "1\n2\nint\n(đã xoá trạng thái)\n2\n",
        "stdout: {out:?}"
    );
    assert!(err.contains("VS0412"), "lỗi kiểu phải ở stderr: {err:?}");
}

#[test]
fn pty_gia_editor_viterm_hello_text_diag_bye() {
    let input = "print(1)\rprint(1 +)\r:type 2\r:quit\r";
    let (ma, out, err) = chay_phien(input, true, Format::ViTerm);
    assert_eq!(ma, 0);
    let lines: Vec<&str> = out.lines().collect();
    assert!(
        lines[0].starts_with(r#"{"v":1,"t":"hello","proto":"vish-term/1""#),
        "khung đầu phải hello: {out:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains(r#""t":"text""#) && l.contains(r#""s":"1\n""#)),
        "thiếu text 1: {out:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains(r#""t":"diag""#) && l.contains("VS")),
        "thiếu diag: {out:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains(r#""t":"text""#) && l.contains(r#""s":"int\n""#)),
        "thiếu text int: {out:?}"
    );
    assert_eq!(lines.last().copied(), Some(r#"{"v":1,"t":"bye"}"#));
    assert!(!out.contains('\x1b'), "khung không được có ANSI");
    assert!(err.contains("vis> "), "editor vẫn prompt stderr: {err:?}");
}

#[test]
fn pty_gia_ctrl_c_thoat_130_va_dong_khung() {
    let (ma, out, _err) = chay_phien("print(1)\r\x03", true, Format::ViTerm);
    assert_eq!(ma, 130);
    assert_eq!(out.lines().last(), Some(r#"{"v":1,"t":"bye"}"#));
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
