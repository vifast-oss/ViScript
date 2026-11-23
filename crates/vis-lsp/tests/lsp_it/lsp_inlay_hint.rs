// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `textDocument/inlayHint` cho LSP.
//!
//! Chỉ `let`/`var` **tên đơn, không nhãn kiểu**; `label = ": " + kiểu`, `kind = 1` (Type), vị trí
//! **cuối tên** (UTF-16 0-based). Suy kiểu rẻ từ HIR (`Some`, list, lời gọi hàm trong tệp, biến
//! trước đó…); không chắc ⇒ không hint. Snapshot **nguyên văn** từng frame để chốt JSON + thứ tự.

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use vis_core::value::Value;

const URI: &str = "file:///a.vis";

/// Snapshot body phản hồi `initialize` — **VD-270** thêm `inlayHintProvider` sau
/// `selectionRangeProvider`.
const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"result":{"capabilities":{"textDocumentSync":{"openClose":true,"change":2},"positionEncoding":"utf-16","hoverProvider":true,"definitionProvider":true,"implementationProvider":true,"documentSymbolProvider":true,"codeLensProvider":{"resolveProvider":false},"foldingRangeProvider":true,"workspaceSymbolProvider":true,"documentFormattingProvider":true,"codeActionProvider":{"codeActionKinds":["source.fixAll"]},"referencesProvider":true,"documentHighlightProvider":true,"selectionRangeProvider":true,"inlayHintProvider":true,"typeDefinitionProvider":true,"renameProvider":{"prepareProvider":true},"signatureHelpProvider":{"triggerCharacters":["(",","]},"completionProvider":{"triggerCharacters":["."]},"semanticTokensProvider":{"legend":{"tokenTypes":["keyword","type","function","variable","string","number","comment","operator","parameter"],"tokenModifiers":[]},"full":true}},"serverInfo":{"name":"vis"}}}"#;

/// Đóng một frame LSP: header + body (byte).
fn khung(body: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{body}", body.len())
}

/// Chạy `vis lsp --stdio`, bơm toàn bộ `dau_vao` rồi đóng stdin; **deadline 5 s** rồi kill.
fn chay(dau_vao: &str) -> (i32, String, String) {
    let mut con = Command::new(crate::common::vis_bin())
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
    vis_core::json::json(v, false, &mut o);
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

/// `textDocument/inlayHint` — chỉ cần `textDocument.uri` (không `position`).
fn inlay(id: i64, uri: &str) -> String {
    so(&m(vec![
        ("jsonrpc", s("2.0")),
        ("id", Value::Int(id)),
        ("method", s("textDocument/inlayHint")),
        (
            "params",
            m(vec![("textDocument", m(vec![("uri", s(uri))]))]),
        ),
    ]))
}

/// Một `InlayHint` nguyên văn (thứ tự khoá `position`, `label`, `kind`).
fn hint(line: i64, ch: i64, nhan: &str) -> String {
    format!(r#"{{"position":{{"line":{line},"character":{ch}}},"label":"{nhan}","kind":1}}"#)
}

/// Body frame **cuối** trong stdout (bỏ header) — dùng khi không snapshot cả chuỗi publish.
fn frame_cuoi(out: &str) -> &str {
    let i = out.rfind("Content-Length: ").expect("có frame");
    let j = out[i..].find("\r\n\r\n").expect("header") + i + 4;
    &out[j..]
}

/// Ca 1 — `int`/`str`/`list[int]`/`Option` suy từ literal/`Some`; `None` trần bị bỏ.
/// Snapshot **cả stdout**: INIT + publish rỗng + frame kết quả.
#[test]
fn inlay_co_ban_int_str_list_option() {
    let nguon = "let a = 1\nlet b = \"hi\"\nlet xs = [1, 2]\nlet o = Some(3)\nlet khong = None\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, nguon)),
        khung(&inlay(2, URI))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(err, "", "stderr phải rỗng");
    let tra = format!(
        r#"{{"jsonrpc":"2.0","id":2,"result":[{},{},{},{}]}}"#,
        hint(0, 5, ": int"),
        hint(1, 5, ": str"),
        hint(2, 6, ": list[int]"),
        hint(3, 5, ": int?"),
    );
    let publish = "{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/publishDiagnostics\",\"params\":{\"uri\":\"file:///a.vis\",\"diagnostics\":[]}}";
    assert_eq!(
        out,
        format!("{}{}{}", khung(INIT), khung(publish), khung(&tra))
    );
}

/// Ca 2 — `let` có nhãn kiểu bị **bỏ**; `var` không nhãn **có** hint (quyết định VD-270);
/// `var` có nhãn bị bỏ.
#[test]
fn inlay_bo_nhan_va_co_nhan() {
    let nguon =
        "let a: int = 1\nlet b: str = \"x\"\nlet c = a + 1\nvar v: int = 2\nvar w = \"z\"\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, nguon)),
        khung(&inlay(2, URI))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let tra = format!(
        r#"{{"jsonrpc":"2.0","id":2,"result":[{},{}]}}"#,
        hint(2, 5, ": int"),
        hint(4, 5, ": str"),
    );
    assert_eq!(frame_cuoi(&out), tra);
}

/// Ca 3 — lời gọi hàm **trong tệp**: kiểu trả của `fn` → hint cho `let n`.
#[test]
fn inlay_ket_qua_goi_ham() {
    let nguon = "fn cong(a: int, b: int) -> int:\n    a + b\n\nlet n = cong(1, 2)\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, nguon)),
        khung(&inlay(2, URI))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let tra = format!(
        r#"{{"jsonrpc":"2.0","id":2,"result":[{}]}}"#,
        hint(3, 5, ": int")
    );
    assert_eq!(frame_cuoi(&out), tra);
}

/// Ca 4 — tên tiếng Việt + tên có ký tự **ngoài BMP** (`𐐨`, 2 code unit UTF-16): vị trí phải
/// đếm UTF-16, không phải byte/scalar.
#[test]
fn inlay_ten_unicode_utf16() {
    let nguon = "let tên = \"x\"\nlet 𐐨 = 2\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, nguon)),
        khung(&inlay(2, URI))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let tra = format!(
        r#"{{"jsonrpc":"2.0","id":2,"result":[{},{}]}}"#,
        hint(0, 7, ": str"),
        hint(1, 6, ": int"),
    );
    assert_eq!(frame_cuoi(&out), tra);
}

/// Ca 5 — thứ tự **nguồn** dù HIR tách `items`/`stmts` (fn khai giữa tệp).
#[test]
fn inlay_thu_tu_nguon() {
    let nguon = "let b = 2\nfn f():\n    let a = 1\n\nlet c = \"x\"\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, nguon)),
        khung(&inlay(2, URI))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let tra = format!(
        r#"{{"jsonrpc":"2.0","id":2,"result":[{},{},{}]}}"#,
        hint(0, 5, ": int"),
        hint(2, 9, ": int"),
        hint(4, 5, ": str"),
    );
    assert_eq!(frame_cuoi(&out), tra);
}

/// Ca 6 — cap **200**: 205 binding ⇒ chỉ 200 hint đầu (theo nguồn), hint thứ 201 bị cắt.
#[test]
fn inlay_cap_200() {
    let mut nguon = String::new();
    for i in 0..205 {
        nguon.push_str(&format!("let x{i} = 1\n"));
    }
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, &nguon)),
        khung(&inlay(2, URI))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let body = frame_cuoi(&out);
    // `x0` dài 2 ⇒ char 6; `x199` dài 4 ⇒ char 8; `x200` phải **không** xuất hiện.
    assert!(body.contains(r#"{"position":{"line":0,"character":6},"label":": int","kind":1}"#));
    assert!(body.contains(r#"{"position":{"line":199,"character":8},"label":": int","kind":1}"#));
    assert!(
        !body.contains(r#""line":200,"#),
        "cap phải cắt sau 200 hint"
    );
    let v = vis_core::json::decode(body).expect("JSON frame hợp lệ");
    let Value::Map(kv) = v else {
        panic!("frame phải là object");
    };
    let Some(Value::List(items)) = kv
        .iter()
        .find_map(|(k, v)| matches!(k, Value::Str(s) if s == "result").then_some(v))
    else {
        panic!("frame phải có result mảng");
    };
    assert_eq!(items.len(), 200, "cap 200");
}

/// Ca 7 — tệp **rỗng** ⇒ `result:[]` (không lỗi).
#[test]
fn inlay_tep_rong_tra_mang_rong() {
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, "")),
        khung(&inlay(2, URI))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(frame_cuoi(&out), r#"{"jsonrpc":"2.0","id":2,"result":[]}"#);
}

/// Ca 8 — URI **chưa mở** ⇒ `result:null` (khác `[]` của tệp rỗng); không có publish.
#[test]
fn inlay_uri_chua_mo_tra_null() {
    let vao = format!("{}{}", khung(&init()), khung(&inlay(2, "file:///b.vis")));
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(
        out,
        format!(
            "{}{}",
            khung(INIT),
            khung(r#"{"jsonrpc":"2.0","id":2,"result":null}"#)
        )
    );
}

/// Ca 9 (đỏ-trước) — trước khi có method này, `textDocument/inlayHint` rơi vào `-32601`.
/// Nay sau `initialize` phải trả **mảng** kết quả; method lạ vẫn `-32601`.
#[test]
fn inlay_khong_con_32601_va_method_la_van_32601() {
    let nguon = "let a = 1\n";
    let la = so(&m(vec![
        ("jsonrpc", s("2.0")),
        ("id", Value::Int(3)),
        ("method", s("textDocument/inlayHintKhongCo")),
        (
            "params",
            m(vec![("textDocument", m(vec![("uri", s(URI))]))]),
        ),
    ]));
    let vao = format!(
        "{}{}{}{}",
        khung(&init()),
        khung(&did_open(URI, nguon)),
        khung(&inlay(2, URI)),
        khung(&la)
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(
        out.contains(r#"{"jsonrpc":"2.0","id":2,"result":[{"position""#),
        "inlayHint phải trả mảng, không còn -32601: {out}"
    );
    assert_eq!(
        frame_cuoi(&out),
        r#"{"jsonrpc":"2.0","id":3,"error":{"code":-32601,"message":"method không tồn tại","data":"textDocument/inlayHintKhongCo"}}"#
    );
}

/// Ca 10 — request **trước** `initialize` ⇒ `-32002` (không frame kết quả hint).
#[test]
fn inlay_truoc_initialize_32002() {
    let (code, out, err) = chay(&khung(&inlay(1, URI)));
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(
        out,
        khung(
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32002,"message":"chưa `initialize`","data":null}}"#
        )
    );
}
