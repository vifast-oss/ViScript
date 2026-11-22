// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-274/ST-275** — hover biến thể enum + `textDocument/foldingRange`.
//!
//! Tách khỏi `lsp.rs` để giữ R1 (≤1500 dòng/tệp); harness tối thiểu giống `lsp.rs`.

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use vis_interp::value::Value;

fn khung(body: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{body}", body.len())
}

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
fn pos(line: i64, ch: i64) -> Value {
    m(vec![
        ("line", Value::Int(line)),
        ("character", Value::Int(ch)),
    ])
}
fn req_vi_tri(id: i64, method: &str, uri: &str, line: i64, ch: i64) -> String {
    so(&m(vec![
        ("jsonrpc", s("2.0")),
        ("id", Value::Int(id)),
        ("method", s(method)),
        (
            "params",
            m(vec![
                ("textDocument", m(vec![("uri", s(uri))])),
                ("position", pos(line, ch)),
            ]),
        ),
    ]))
}
fn hover(id: i64, uri: &str, line: i64, ch: i64) -> String {
    req_vi_tri(id, "textDocument/hover", uri, line, ch)
}
fn frame_cuoi(out: &str) -> String {
    let i = out.rfind("Content-Length: ").expect("có frame");
    let j = out[i..].find("\r\n\r\n").expect("header") + i + 4;
    out[j..].to_string()
}

const URI: &str = "file:///a.vis";

// ===== ST-274: hover biến thể enum =====

/// Ca 34 — hover `A` trong nhánh `match` ⇒ nêu biến thể của enum `E`.
#[test]
fn lsp_hover_bien_the_enum() {
    let text = "enum E:\n    A\n\nfn f(e: E) -> int:\n    match e:\n        A: 1\n\nfn main():\n    print(f(A))\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&hover(2, URI, 5, 9))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let body = frame_cuoi(&out);
    assert!(
        body.contains("biến thể của"),
        "hover phải nêu biến thể enum: {body}"
    );
    assert!(body.contains("E"), "hover phải nêu enum E: {body}");
}

// ===== ST-275: foldingRange =====

fn folding(id: i64, uri: &str) -> String {
    so(&m(vec![
        ("jsonrpc", s("2.0")),
        ("id", Value::Int(id)),
        ("method", s("textDocument/foldingRange")),
        (
            "params",
            m(vec![("textDocument", m(vec![("uri", s(uri))]))]),
        ),
    ]))
}

/// Ca 35 — hai hàm nhiều dòng ⇒ ít nhất 2 vùng gấp bắt đầu dòng 0 và 3.
#[test]
fn lsp_folding_range_hai_ham() {
    let text = "fn add(a: int) -> int:\n    a\n\nfn main():\n    print(add(1))\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&folding(2, URI))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let body = frame_cuoi(&out);
    assert!(
        body.starts_with(r#"{"jsonrpc":"2.0","id":2,"result":["#),
        "frame folding: {body}"
    );
    assert!(body.contains(r#""startLine":0"#), "thiếu fn add: {body}");
    assert!(body.contains(r#""startLine":3"#), "thiếu fn main: {body}");
}
// ===== ST-276: completion sau module std.* =====

/// Ca 36 — `fs.` ⇒ gợi ý hàm của `std.fs` (kể cả helper mới).
#[test]
fn lsp_completion_sau_module_fs() {
    let text = "use std.fs\n\nfs.";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&completion(2, URI, 2, 3))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let body = frame_cuoi(&out);
    assert!(
        body.contains(r#""label":"read_lines""#),
        "thiếu read_lines: {body}"
    );
    assert!(body.contains(r#""label":"append""#), "thiếu append: {body}");
}

/// Ca 37 — `sqlite.` ⇒ gợi ý `query_one`.
#[test]
fn lsp_completion_sau_module_sqlite() {
    let text = "use std.sqlite\n\nsqlite.";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&completion(2, URI, 2, 7))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let body = frame_cuoi(&out);
    assert!(
        body.contains(r#""label":"query_one""#),
        "thiếu query_one: {body}"
    );
}

fn completion(id: i64, uri: &str, line: i64, ch: i64) -> String {
    req_vi_tri(id, "textDocument/completion", uri, line, ch)
}

// ===== ST-280: prepareRename =====

fn prepare(id: i64, uri: &str, line: i64, ch: i64) -> String {
    req_vi_tri(id, "textDocument/prepareRename", uri, line, ch)
}

/// Ca — `add` là định danh hợp lệ ⇒ `{range, placeholder}`.
#[test]
fn lsp_prepare_rename_hop_le() {
    let text = "fn add(a: int) -> int:\n    a\n\nfn main():\n    add(1)\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&prepare(2, URI, 0, 4))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let body = frame_cuoi(&out);
    assert!(
        body.contains(r#""placeholder":"add""#),
        "thiếu placeholder: {body}"
    );
    assert!(body.contains(r#""range""#), "thiếu range: {body}");
}

/// Ca — từ khoá `fn` ⇒ `result:null`.
#[test]
fn lsp_prepare_rename_tu_khoa_null() {
    let text = "fn main():\n    print(1)\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&prepare(2, URI, 0, 0))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(
        frame_cuoi(&out),
        r#"{"jsonrpc":"2.0","id":2,"result":null}"#
    );
}

// ===== ST-282: documentHighlight =====

fn highlight(id: i64, uri: &str, line: i64, ch: i64) -> String {
    req_vi_tri(id, "textDocument/documentHighlight", uri, line, ch)
}

/// Ca — `add` xuất hiện 2 lần ⇒ 2 highlight kind 1.
#[test]
fn lsp_document_highlight_dem_hai() {
    let text = "fn add(a: int) -> int:\n    a\n\nfn main():\n    print(add(1))\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&highlight(2, URI, 4, 10))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let body = frame_cuoi(&out);
    assert!(
        body.starts_with(r#"{"jsonrpc":"2.0","id":2,"result":["#),
        "frame: {body}"
    );
    assert_eq!(
        body.matches(r#""kind":1"#).count(),
        2,
        "phải có 2 highlight: {body}"
    );
}
