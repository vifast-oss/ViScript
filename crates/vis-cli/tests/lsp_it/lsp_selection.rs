// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `textDocument/selectionRange`: chuỗi chọn token → dòng → khai báo → tệp.
//!
//! Snapshot **nguyên văn** từng frame phản hồi (thứ tự khoá cố định) để chốt cả hình dạng JSON,
//! thứ tự `positions` và quan hệ `parent` lồng nhau. stdout phải sạch, stderr rỗng.

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

/// `textDocument/selectionRange` với nhiều vị trí (giữ đúng thứ tự gửi).
fn selection(id: i64, uri: &str, vi_tris: &[(i64, i64)]) -> String {
    let ds: Vec<Value> = vi_tris
        .iter()
        .map(|&(line, ch)| {
            m(vec![
                ("line", Value::Int(line)),
                ("character", Value::Int(ch)),
            ])
        })
        .collect();
    so(&m(vec![
        ("jsonrpc", s("2.0")),
        ("id", Value::Int(id)),
        ("method", s("textDocument/selectionRange")),
        (
            "params",
            m(vec![
                ("textDocument", m(vec![("uri", s(uri))])),
                ("positions", Value::List(ds)),
            ]),
        ),
    ]))
}

/// Frame **cuối** trong stdout (bỏ header).
fn frame_cuoi(out: &str) -> String {
    let i = out.rfind("Content-Length: ").expect("có frame");
    let j = out[i..].find("\r\n\r\n").expect("header") + i + 4;
    out[j..].to_string()
}

/// Ca 1 — trong thân `fn add`, chuỗi: token `a` → dòng 1 → cả `fn add` → toàn tệp.
#[test]
fn lsp_selection_token_dong_khai_bao() {
    let text = "fn add(a: int, b: int) -> int:\n    a + b\n\nfn main():\n    let x = add(1, 2)\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&selection(2, URI, &[(1, 4)]))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(err, "", "stderr phải rỗng");
    let tra = r#"{"jsonrpc":"2.0","id":2,"result":[{"range":{"start":{"line":1,"character":4},"end":{"line":1,"character":5}},"parent":{"range":{"start":{"line":1,"character":0},"end":{"line":1,"character":9}},"parent":{"range":{"start":{"line":0,"character":0},"end":{"line":3,"character":0}},"parent":{"range":{"start":{"line":0,"character":0},"end":{"line":5,"character":0}}}}}}]}"#;
    assert_eq!(frame_cuoi(&out), tra);
}

/// Ca 2 — vị trí trên **khoảng trắng** (không có token) ⇒ range rỗng → dòng → toàn tệp.
#[test]
fn lsp_selection_khoang_trang_range_rong() {
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, "  \n")),
        khung(&selection(2, URI, &[(0, 0)]))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(err, "");
    let tra = r#"{"jsonrpc":"2.0","id":2,"result":[{"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":0}},"parent":{"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":2}},"parent":{"range":{"start":{"line":0,"character":0},"end":{"line":1,"character":0}}}}}]}"#;
    assert_eq!(frame_cuoi(&out), tra);
}

/// Ca 3 — nhiều vị trí: `SelectionRange[]` đúng **thứ tự đầu vào** (dòng 1 trước, dòng 0 sau).
#[test]
fn lsp_selection_nhieu_vi_tri_giu_thu_tu() {
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, "abc\ndef\n")),
        khung(&selection(2, URI, &[(1, 1), (0, 1)]))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(err, "");
    let tra = r#"{"jsonrpc":"2.0","id":2,"result":[{"range":{"start":{"line":1,"character":0},"end":{"line":1,"character":3}},"parent":{"range":{"start":{"line":0,"character":0},"end":{"line":2,"character":0}}}},{"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":3}},"parent":{"range":{"start":{"line":0,"character":0},"end":{"line":2,"character":0}}}}]}"#;
    assert_eq!(frame_cuoi(&out), tra);
}

/// Ca 4 — trường `f` (một dòng) đứng **trước** dòng vật lý và trước `type U`: token → field →
/// dòng → tệp; đây là chuỗi bao hàm thật (field nằm trong dòng).
#[test]
fn lsp_selection_truong_truoc_type() {
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, "type U:\n    f: int\n")),
        khung(&selection(2, URI, &[(1, 4)]))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(err, "");
    let tra = r#"{"jsonrpc":"2.0","id":2,"result":[{"range":{"start":{"line":1,"character":4},"end":{"line":1,"character":5}},"parent":{"range":{"start":{"line":1,"character":4},"end":{"line":1,"character":10}},"parent":{"range":{"start":{"line":1,"character":0},"end":{"line":1,"character":10}},"parent":{"range":{"start":{"line":0,"character":0},"end":{"line":2,"character":0}}}}}}]}"#;
    assert_eq!(frame_cuoi(&out), tra);
}

/// Ca 5 — tệp rỗng: token/dòng/tệp đều rỗng ⇒ gộp còn **một** range rỗng tại `(0,0)`.
#[test]
fn lsp_selection_tep_rong_mot_range() {
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, "")),
        khung(&selection(2, URI, &[(0, 0)]))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(err, "");
    let tra = r#"{"jsonrpc":"2.0","id":2,"result":[{"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":0}}}]}"#;
    assert_eq!(frame_cuoi(&out), tra);
}

/// Ca 6 — URI chưa `didOpen` ⇒ mảng rỗng (không lỗi), đúng thứ tự frame.
#[test]
fn lsp_selection_chua_mo_rong() {
    let vao = format!("{}{}", khung(&init()), khung(&selection(2, URI, &[(0, 0)])));
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(err, "");
    let tra = r#"{"jsonrpc":"2.0","id":2,"result":[]}"#;
    assert_eq!(frame_cuoi(&out), tra);
}
