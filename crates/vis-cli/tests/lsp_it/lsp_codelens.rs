// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `textDocument/codeLens`: mỗi `test fn` cấp tệp ⇒ một lens `Chạy test <tên>`.
//!
//! `range` là **đúng token tên** (0-based, UTF-16); `command.command` = `vis.test.run`, tham số
//! `[uri, "<tên>"]`. Không có lens cho `fn` thường/phương thức `impl`; giữ thứ tự nguồn, tối đa
//! 200. Tệp mở mà không có `test fn` ⇒ `[]`; URI chưa mở ⇒ `null`.
//!
//! Snapshot **nguyên văn** từng frame (thứ tự khoá cố định). **Đỏ trước VD-270**: request
//! `textDocument/codeLens` rơi vào nhánh method lạ ⇒ `-32601`; các ca dưới đây đỏ trước khi thêm
//! handler.

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use vis_interp::value::Value;

const URI: &str = "file:///a.vis";

/// Đóng một frame LSP: header + body (byte).
fn khung(body: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{body}", body.len())
}

/// Chạy `vis lsp --stdio`, bơm toàn bộ `dau_vao` rồi đóng stdin; **deadline 5 s** rồi kill.
fn chay(dau_vao: &str) -> (i32, String, String) {
    let mut con = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(["lsp", "--stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn `vis lsp`");
    con.stdin
        .take()
        .expect("stdin")
        .write_all(dau_vao.as_bytes())
        .expect("bơm stdin");
    let han = Instant::now() + Duration::from_secs(5);
    loop {
        if con.try_wait().expect("try_wait").is_some() {
            break;
        }
        if Instant::now() > han {
            let _ = con.kill();
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let o = con.wait_with_output().expect("đọc output");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

fn s(x: &str) -> Value {
    Value::Str(x.to_string())
}

fn m(kv: Vec<(&str, Value)>) -> Value {
    Value::Map(
        kv.into_iter()
            .map(|(k, v)| (Value::Str(k.to_string()), v))
            .collect(),
    )
}

fn so(v: &Value) -> String {
    let mut o = String::new();
    vis_interp::stdlib::json(v, false, &mut o);
    o
}

fn init() -> String {
    so(&m(vec![
        ("jsonrpc", s("2.0")),
        ("id", Value::Int(1)),
        ("method", s("initialize")),
        ("params", m(vec![])),
    ]))
}

fn did_open(uri: &str, text: &str) -> String {
    so(&m(vec![
        ("jsonrpc", s("2.0")),
        ("method", s("textDocument/didOpen")),
        (
            "params",
            m(vec![(
                "textDocument",
                m(vec![
                    ("uri", s(uri)),
                    ("languageId", s("viscript")),
                    ("version", Value::Int(1)),
                    ("text", s(text)),
                ]),
            )]),
        ),
    ]))
}

/// `textDocument/codeLens` cho một tài liệu.
fn code_lens(id: i64, uri: &str) -> String {
    so(&m(vec![
        ("jsonrpc", s("2.0")),
        ("id", Value::Int(id)),
        ("method", s("textDocument/codeLens")),
        (
            "params",
            m(vec![("textDocument", m(vec![("uri", s(uri))]))]),
        ),
    ]))
}

/// Frame **cuối** trong stdout (bỏ header).
fn frame_cuoi(out: &str) -> String {
    let i = out.rfind("Content-Length: ").expect("có frame");
    let j = out[i..].find("\r\n\r\n").expect("header") + i + 4;
    out[j..].to_string()
}

/// Ca 1 — một `test fn`: range đúng token tên (cột 8..16) + command `vis.test.run`.
#[test]
fn lsp_codelens_mot_test() {
    let text = "test fn kiem_tra():\n    assert 1 == 1\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&code_lens(2, URI))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(err, "", "stderr phải rỗng");
    let tra = r#"{"jsonrpc":"2.0","id":2,"result":[{"range":{"start":{"line":0,"character":8},"end":{"line":0,"character":16}},"command":{"title":"Chạy test kiem_tra","command":"vis.test.run","arguments":["file:///a.vis","kiem_tra"]}}]}"#;
    assert_eq!(frame_cuoi(&out), tra);
}

/// Ca 2 — nhiều `test fn` ⇒ lens theo **thứ tự nguồn**; `fn` thường, `type` và phương thức `impl`
/// đều **không** sinh lens.
#[test]
fn lsp_codelens_nhieu_test_thu_tu_va_bo_fn_thuong() {
    let text = "test fn dau():\n    assert 1 == 1\n\nfn thuong():\n    print(1)\n\ntype H:\n    x: int\n\nimpl H:\n    fn phuong(self) -> int:\n        self.x\n\ntest fn giua():\n    assert 1 == 1\n\ntest fn cuoi():\n    assert 1 == 1\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&code_lens(2, URI))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(err, "");
    let tra = r#"{"jsonrpc":"2.0","id":2,"result":[{"range":{"start":{"line":0,"character":8},"end":{"line":0,"character":11}},"command":{"title":"Chạy test dau","command":"vis.test.run","arguments":["file:///a.vis","dau"]}},{"range":{"start":{"line":13,"character":8},"end":{"line":13,"character":12}},"command":{"title":"Chạy test giua","command":"vis.test.run","arguments":["file:///a.vis","giua"]}},{"range":{"start":{"line":16,"character":8},"end":{"line":16,"character":12}},"command":{"title":"Chạy test cuoi","command":"vis.test.run","arguments":["file:///a.vis","cuoi"]}}]}"#;
    assert_eq!(frame_cuoi(&out), tra);
}

/// Ca 3 — **UTF-16**: tên chứa ký tự astral `𐐀` (1 scalar, **2** code unit UTF-16). `character`
/// cuối phải là 12 (8 + 4 code unit), không phải 11 (scalar) hay 14 (byte UTF-8); dòng trước cũng
/// có `𐐀` để mốc byte của range đi qua ký tự nhiều byte.
#[test]
fn lsp_codelens_utf16_ten_unicode() {
    let text = "fn 𐐀():\n    print(1)\n\ntest fn a𐐀b():\n    assert 1 == 1\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&code_lens(2, URI))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(err, "");
    let tra = r#"{"jsonrpc":"2.0","id":2,"result":[{"range":{"start":{"line":3,"character":8},"end":{"line":3,"character":12}},"command":{"title":"Chạy test a𐐀b","command":"vis.test.run","arguments":["file:///a.vis","a𐐀b"]}}]}"#;
    assert_eq!(frame_cuoi(&out), tra);
}

/// Ca 4 — tệp mở **rỗng** (không có `test fn`) ⇒ `result:[]`, không phải `null`.
#[test]
fn lsp_codelens_tep_rong_mang_rong() {
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, "")),
        khung(&code_lens(2, URI))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(err, "");
    assert_eq!(frame_cuoi(&out), r#"{"jsonrpc":"2.0","id":2,"result":[]}"#);
}

/// Ca 5 — URI **chưa** `didOpen` ⇒ `result:null` (như hover/definition), không lỗi.
#[test]
fn lsp_codelens_chua_mo_null() {
    let vao = format!("{}{}", khung(&init()), khung(&code_lens(2, URI)));
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(err, "");
    assert_eq!(
        frame_cuoi(&out),
        r#"{"jsonrpc":"2.0","id":2,"result":null}"#
    );
}

/// Ca 6 — **trần 200**: 201 `test fn` ⇒ đúng 200 lens, giữ 200 khai báo đầu (nguồn), bỏ cái cuối.
#[test]
fn lsp_codelens_toi_da_200() {
    let mut text = String::new();
    for i in 0..201 {
        text.push_str(&format!("test fn t_{i}():\n    assert 1 == 1\n\n"));
    }
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, &text)),
        khung(&code_lens(2, URI))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(err, "");
    let tra = frame_cuoi(&out);
    assert_eq!(
        tra.matches(r#""title":"Chạy test "#).count(),
        200,
        "phải đúng 200 lens"
    );
    assert!(tra.contains(r#""Chạy test t_199""#), "giữ 200 khai báo đầu");
    assert!(
        !tra.contains(r#""Chạy test t_200""#),
        "khai báo thứ 201 phải bị cắt"
    );
}

/// Ca 7 — **đỏ trước VD-270**: trước khi có handler, request rơi vào method lạ ⇒ `-32601`; nay là
/// `result` mảng có lens, không còn lỗi.
#[test]
fn lsp_codelens_khong_con_32601() {
    let text = "test fn mot():\n    assert 1 == 1\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&code_lens(2, URI))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(err, "");
    let tra = frame_cuoi(&out);
    assert!(
        !tra.contains("-32601"),
        "codeLens phải có handler riêng, không phải method lạ: {tra}"
    );
    assert_eq!(
        tra,
        r#"{"jsonrpc":"2.0","id":2,"result":[{"range":{"start":{"line":0,"character":8},"end":{"line":0,"character":11}},"command":{"title":"Chạy test mot","command":"vis.test.run","arguments":["file:///a.vis","mot"]}}]}"#
    );
}
