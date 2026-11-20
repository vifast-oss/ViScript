// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-279** — `textDocument/formatting`.

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
fn formatting(id: i64, uri: &str) -> String {
    so(&m(vec![
        ("jsonrpc", s("2.0")),
        ("id", Value::Int(id)),
        ("method", s("textDocument/formatting")),
        (
            "params",
            m(vec![("textDocument", m(vec![("uri", s(uri))]))]),
        ),
    ]))
}
fn frame_cuoi(out: &str) -> String {
    let i = out.rfind("Content-Length: ").expect("có frame");
    let j = out[i..].find("\r\n\r\n").expect("header") + i + 4;
    out[j..].to_string()
}

const URI: &str = "file:///a.vis";

/// Ca 1 — tab trong thụt lề ⇒ formatter trả `newText` 4 dấu cách.
#[test]
fn formatting_tab_thanh_bon_cach() {
    let text = "fn main():\n\tprint(1)\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&formatting(2, URI))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let body = frame_cuoi(&out);
    assert!(
        body.contains(r#""newText":"fn main():\n    print(1)\n""#),
        "newText phải 4 cách: {body}"
    );
    assert!(
        body.starts_with(r#"{"jsonrpc":"2.0","id":2,"result":["#),
        "frame: {body}"
    );
}

/// Ca 2 — cú pháp hỏng ⇒ `[]`, không bịa bản đã format.
#[test]
fn formatting_cu_phap_hong_rong() {
    let text = "fn main(:\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&formatting(2, URI))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(frame_cuoi(&out), r#"{"jsonrpc":"2.0","id":2,"result":[]}"#);
}
