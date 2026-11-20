// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — hover **phương thức `impl`/`trait`** (bổ sung cho hover item/biến/trường/ký hiệu).
//!
//! Trước VD-270 cả ba vị trí dưới đây trả `result:null` (đỏ): `u.lay` (nhận có kiểu), `self.lay`
//! trong `impl`, và đúng tên khai báo `fn lay`. Nay trả markdown ` ```vis\n<chữ ký>\n``` ` +
//! `phương thức của \`<chủ>\``; trùng tên mà **không** suy được chủ ⇒ `null` (không đoán).
//! Snapshot **nguyên văn** từng frame (thứ tự khoá cố định) để chốt cả hình dạng JSON.

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use vis_interp::value::Value;

const URI: &str = "file:///a.vis";

/// Snapshot body phản hồi `initialize` — **VD-270** thêm `implementationProvider` (sau
/// `definitionProvider`).
const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"result":{"capabilities":{"textDocumentSync":1,"positionEncoding":"utf-16","hoverProvider":true,"definitionProvider":true,"implementationProvider":true,"documentSymbolProvider":true,"codeLensProvider":{"resolveProvider":false},"foldingRangeProvider":true,"workspaceSymbolProvider":true,"documentFormattingProvider":true,"referencesProvider":true,"documentHighlightProvider":true,"selectionRangeProvider":true,"renameProvider":{"prepareProvider":true},"signatureHelpProvider":{"triggerCharacters":["(",","]},"completionProvider":{"triggerCharacters":["."]}},"serverInfo":{"name":"vis"}}}"#;

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

fn req_vi_tri(id: i64, method: &str, uri: &str, line: i64, ch: i64) -> String {
    so(&m(vec![
        ("jsonrpc", s("2.0")),
        ("id", Value::Int(id)),
        ("method", s(method)),
        (
            "params",
            m(vec![
                ("textDocument", m(vec![("uri", s(uri))])),
                (
                    "position",
                    m(vec![
                        ("line", Value::Int(line)),
                        ("character", Value::Int(ch)),
                    ]),
                ),
            ]),
        ),
    ]))
}

fn hover(id: i64, uri: &str, line: i64, ch: i64) -> String {
    req_vi_tri(id, "textDocument/hover", uri, line, ch)
}

fn completion(id: i64, uri: &str, line: i64, ch: i64) -> String {
    req_vi_tri(id, "textDocument/completion", uri, line, ch)
}

/// Frame **cuối** trong stdout (bỏ header) — dùng khi không snapshot cả chuỗi publish.
fn frame_cuoi(out: &str) -> &str {
    let i = out.rfind("Content-Length: ").expect("có frame");
    let j = out[i..].find("\r\n\r\n").expect("header") + i + 4;
    &out[j..]
}

/// Nguồn: `impl U` có `lay(self) -> int?`; con trỏ được hỏi ở `self.lay`, khai báo, `u.lay`.
const NGUON_IMPL: &str = "type U:\n    n: int\n\nimpl U:\n    fn lay(self) -> int?:\n        self.lay()\n\nfn f(u: U) -> int?:\n    u.lay()\n";

/// Ca 1 — `u.lay` với `u: U` ⇒ đúng chữ ký + chủ `U`; frame **nguyên văn** (kể cả publish rỗng).
#[test]
fn hover_phuong_thuc_impl_nhan_co_kieu() {
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, NGUON_IMPL)),
        khung(&hover(2, URI, 8, 6))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(err, "", "stderr phải rỗng");
    let tra = "{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"contents\":{\"kind\":\"markdown\",\"value\":\"```vis\\nfn lay(self) -> int?\\n```\\n\\nphương thức của `U`\"},\"range\":{\"start\":{\"line\":8,\"character\":6},\"end\":{\"line\":8,\"character\":9}}}}";
    let publish = "{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/publishDiagnostics\",\"params\":{\"uri\":\"file:///a.vis\",\"diagnostics\":[]}}";
    assert_eq!(
        out,
        format!("{}{}{}", khung(INIT), khung(publish), khung(tra))
    );
}

/// Ca 2 — `self.lay` trong `impl U` ⇒ suy chủ từ khối `impl` (không cần `self` có trong `bien`).
#[test]
fn hover_phuong_thuc_self_trong_impl() {
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, NGUON_IMPL)),
        khung(&hover(2, URI, 5, 13))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let tra = "{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"contents\":{\"kind\":\"markdown\",\"value\":\"```vis\\nfn lay(self) -> int?\\n```\\n\\nphương thức của `U`\"},\"range\":{\"start\":{\"line\":5,\"character\":13},\"end\":{\"line\":5,\"character\":16}}}}";
    assert_eq!(frame_cuoi(&out), tra);
}

/// Ca 3 — tên khai báo `fn lay` trong `impl` ⇒ cùng chữ ký; range đúng từ, không phải cả khối.
#[test]
fn hover_phuong_thuc_tai_khai_bao() {
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, NGUON_IMPL)),
        khung(&hover(2, URI, 4, 7))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let tra = "{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"contents\":{\"kind\":\"markdown\",\"value\":\"```vis\\nfn lay(self) -> int?\\n```\\n\\nphương thức của `U`\"},\"range\":{\"start\":{\"line\":4,\"character\":7},\"end\":{\"line\":4,\"character\":10}}}}";
    assert_eq!(frame_cuoi(&out), tra);
}

/// Nguồn trait: `trait Hien` + `impl Hien for V` cùng tên `hien` (hai chủ — phải phân biệt được).
const NGUON_TRAIT: &str = "trait Hien:\n    fn hien(self) -> int\n\ntype V:\n    n: int\n\nimpl Hien for V:\n    fn hien(self) -> int:\n        self.hien()\n\nfn f(v: V) -> int:\n    v.hien()\n";

/// Ca 4 — phương thức `trait` tại khai báo ⇒ chủ là tên trait (`Hien`), không lẫn `impl`.
#[test]
fn hover_phuong_thuc_trait_tai_khai_bao() {
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, NGUON_TRAIT)),
        khung(&hover(2, URI, 1, 7))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let tra = "{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"contents\":{\"kind\":\"markdown\",\"value\":\"```vis\\nfn hien(self) -> int\\n```\\n\\nphương thức của `Hien`\"},\"range\":{\"start\":{\"line\":1,\"character\":7},\"end\":{\"line\":1,\"character\":11}}}}";
    assert_eq!(frame_cuoi(&out), tra);
}

/// Ca 5 — `v.hien` với `v: V` ⇒ chủ **kiểu impl** (`V`), không phải trait `Hien` cùng tên.
#[test]
fn hover_phuong_thuc_trait_impl_nhan_co_kieu() {
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, NGUON_TRAIT)),
        khung(&hover(2, URI, 11, 6))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let tra = "{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"contents\":{\"kind\":\"markdown\",\"value\":\"```vis\\nfn hien(self) -> int\\n```\\n\\nphương thức của `V`\"},\"range\":{\"start\":{\"line\":11,\"character\":6},\"end\":{\"line\":11,\"character\":10}}}}";
    assert_eq!(frame_cuoi(&out), tra);
}

/// Ca 6 — **trùng tên** `lay` trên `A` và `B`, người nhận không suy được kiểu ⇒ `result:null`
/// (khử trùng theo **chủ**, không gộp hai chủ khác nhau dù chữ ký y hệt).
#[test]
fn hover_phuong_thuc_trung_ten_tra_null() {
    let text = "type A:\n    n: int\n\ntype B:\n    n: int\n\nimpl A:\n    fn lay(self) -> int:\n        self.n\n\nimpl B:\n    fn lay(self) -> int:\n        self.n\n\nfn g() -> int:\n    z.lay()\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&hover(2, URI, 15, 6))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(
        frame_cuoi(&out),
        r#"{"jsonrpc":"2.0","id":2,"result":null}"#
    );
}

/// Ca 7 — tên **không phải** phương thức của kiểu người nhận ⇒ `result:null` (không đoán bừa).
#[test]
fn hover_phuong_thuc_khong_ton_tai_tra_null() {
    let text = "type U:\n    n: int\n\nimpl U:\n    fn lay(self) -> int?:\n        self.n\n\nfn f(u: U) -> int?:\n    u.khong_co\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&hover(2, URI, 8, 6))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(
        frame_cuoi(&out),
        r#"{"jsonrpc":"2.0","id":2,"result":null}"#
    );
}

/// Ca 8 — completion **cấp tệp** không được nhận phương thức `impl` (`lay` không xuất hiện).
#[test]
fn completion_khong_them_phuong_thuc() {
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, NGUON_IMPL)),
        khung(&completion(2, URI, 9, 0))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let body = frame_cuoi(&out);
    assert!(
        body.starts_with(r#"{"jsonrpc":"2.0","id":2,"result":["#),
        "{body}"
    );
    assert!(
        !body.contains(r#""label":"lay""#),
        "completion cấp tệp không được có phương thức `lay`: {body}"
    );
}
