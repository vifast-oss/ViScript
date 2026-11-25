// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310b (VD-285)** — `vis repl` PTY giả qua `run_with` (API thư viện).
//!
//! Tách từ `vis-cli/tests/cli_it/repl_term.rs` (ST-313c/VD-289);
//! 4 ca CLI (non-TTY/viterm/format sai) giữ ở `vis-cli`.

use std::io::Cursor;

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
