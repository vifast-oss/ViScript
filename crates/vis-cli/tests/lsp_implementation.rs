// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `textDocument/implementation` cho LSP.
//!
//! Nguồn: `KetQua.phuong` (impl/trait method đã gom cho hover). Chỉ trả **phương thức `impl` trùng
//! tên**; lọc theo kiểu tĩnh của người nhận khi biết (`u.lay`, `self.lay`, `v.hien`), còn lại trả
//! mọi phương thức `impl` cùng tên (không đoán quan hệ kiểu). Không trùng ⇒ `result:null`.
//! Snapshot **nguyên văn** từng frame để chốt hình dạng JSON + thứ tự khoá.

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use vis_interp::value::Value;

const URI: &str = "file:///a.vis";

/// Snapshot body phản hồi `initialize` — **VD-270** thêm `implementationProvider` sau
/// `definitionProvider`.
const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"result":{"capabilities":{"textDocumentSync":1,"positionEncoding":"utf-16","hoverProvider":true,"definitionProvider":true,"implementationProvider":true,"documentSymbolProvider":true,"codeLensProvider":{"resolveProvider":false},"foldingRangeProvider":true,"workspaceSymbolProvider":true,"documentFormattingProvider":true,"codeActionProvider":{"codeActionKinds":["source.fixAll"]},"referencesProvider":true,"documentHighlightProvider":true,"selectionRangeProvider":true,"inlayHintProvider":true,"typeDefinitionProvider":true,"renameProvider":{"prepareProvider":true},"signatureHelpProvider":{"triggerCharacters":["(",","]},"completionProvider":{"triggerCharacters":["."]},"semanticTokensProvider":{"legend":{"tokenTypes":["keyword","type","function","variable","string","number","comment","operator","parameter"],"tokenModifiers":[]},"full":true}},"serverInfo":{"name":"vis"}}}"#;

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

fn implementation(id: i64, uri: &str, line: i64, ch: i64) -> String {
    req_vi_tri(id, "textDocument/implementation", uri, line, ch)
}

/// Frame **cuối** trong stdout (bỏ header) — dùng khi không snapshot cả chuỗi publish.
fn frame_cuoi(out: &str) -> &str {
    let i = out.rfind("Content-Length: ").expect("có frame");
    let j = out[i..].find("\r\n\r\n").expect("header") + i + 4;
    &out[j..]
}

/// Một `Location` nguyên văn (thứ tự khoá `uri`, `range.start`, `range.end`).
fn loc(uri: &str, l0: i64, c0: i64, l1: i64, c1: i64) -> String {
    format!(
        r#"{{"uri":"{uri}","range":{{"start":{{"line":{l0},"character":{c0}}},"end":{{"line":{l1},"character":{c1}}}}}}}"#
    )
}

/// Nguồn: `impl U` có `lay(self) -> int?`; con trỏ được hỏi ở `u.lay`, `self.lay`, khai báo.
const NGUON_IMPL: &str = "type U:\n    n: int\n\nimpl U:\n    fn lay(self) -> int?:\n        self.lay()\n\nfn f(u: U) -> int?:\n    u.lay()\n";

/// Nguồn trait: `trait Hien` + `impl Hien for V` cùng tên `hien`.
const NGUON_TRAIT: &str = "trait Hien:\n    fn hien(self) -> int\n\ntype V:\n    n: int\n\nimpl Hien for V:\n    fn hien(self) -> int:\n        self.hien()\n\nfn f(v: V) -> int:\n    v.hien()\n";

/// Ca 1 — `u.lay` với `u: U` ⇒ đúng vị trí **tên** phương thức trong `impl U`; frame nguyên văn.
#[test]
fn implementation_u_lay_tra_impl() {
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, NGUON_IMPL)),
        khung(&implementation(2, URI, 8, 6))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(err, "", "stderr phải rỗng");
    let tra = format!(
        r#"{{"jsonrpc":"2.0","id":2,"result":[{}]}}"#,
        loc(URI, 4, 7, 4, 10)
    );
    let publish = "{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/publishDiagnostics\",\"params\":{\"uri\":\"file:///a.vis\",\"diagnostics\":[]}}";
    assert_eq!(
        out,
        format!("{}{}{}", khung(INIT), khung(publish), khung(&tra))
    );
}

/// Ca 2 — `self.lay` trong `impl U` ⇒ suy chủ từ khối `impl`, cùng một vị trí.
#[test]
fn implementation_self_lay_trong_impl() {
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, NGUON_IMPL)),
        khung(&implementation(2, URI, 5, 13))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let tra = format!(
        r#"{{"jsonrpc":"2.0","id":2,"result":[{}]}}"#,
        loc(URI, 4, 7, 4, 10)
    );
    assert_eq!(frame_cuoi(&out), tra);
}

/// Ca 3 — khai báo phương thức `trait Hien.hien` ⇒ mọi phương thức `impl` cùng tên.
#[test]
fn implementation_trait_tai_khai_bao() {
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, NGUON_TRAIT)),
        khung(&implementation(2, URI, 1, 7))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let tra = format!(
        r#"{{"jsonrpc":"2.0","id":2,"result":[{}]}}"#,
        loc(URI, 7, 7, 7, 11)
    );
    assert_eq!(frame_cuoi(&out), tra);
}

/// Ca 4 — gọi phương thức trait với người nhận có kiểu `v: V` ⇒ lọc đúng `impl ... for V`.
#[test]
fn implementation_trait_call_loc_theo_kieu() {
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, NGUON_TRAIT)),
        khung(&implementation(2, URI, 11, 6))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let tra = format!(
        r#"{{"jsonrpc":"2.0","id":2,"result":[{}]}}"#,
        loc(URI, 7, 7, 7, 11)
    );
    assert_eq!(frame_cuoi(&out), tra);
}

/// Nguồn trait: `trait Hien` + `impl Hien for V`, nhưng lời gọi trên biến **không rõ kiểu**.
const NGUON_TRAIT_MO_HO: &str = "trait Hien:\n    fn hien(self) -> int\n\ntype V:\n    n: int\n\nimpl Hien for V:\n    fn hien(self) -> int:\n        self.n\n\nfn f() -> int:\n    z.hien()\n";

/// Ca 5 — gọi phương thức `trait` mà không suy được kiểu người nhận ⇒ `result:null` (không đoán).
#[test]
fn implementation_trait_call_mo_ho_tra_null() {
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, NGUON_TRAIT_MO_HO)),
        khung(&implementation(2, URI, 11, 6))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(
        frame_cuoi(&out),
        r#"{"jsonrpc":"2.0","id":2,"result":null}"#
    );
}

/// Nguồn: hai `impl` cùng phương thức `lay`; `fn f(a: A)` hỏi `a.lay`.
const NGUON_HAI: &str = "type A:\n    n: int\n\ntype B:\n    n: int\n\nimpl A:\n    fn lay(self) -> int:\n        self.n\n\nimpl B:\n    fn lay(self) -> int:\n        self.n\n\nfn f(a: A) -> int:\n    a.lay()\n";

/// Ca 6 — tại tên khai báo `impl A.lay` ⇒ chính phương thức đó (lọc chủ `A`), không kéo `B.lay`.
#[test]
fn implementation_tai_khai_bao_impl() {
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, NGUON_HAI)),
        khung(&implementation(2, URI, 7, 7))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let tra = format!(
        r#"{{"jsonrpc":"2.0","id":2,"result":[{}]}}"#,
        loc(URI, 7, 7, 7, 10)
    );
    assert_eq!(frame_cuoi(&out), tra);
}

/// Ca 7 — người nhận có kiểu tĩnh `A` ⇒ **chỉ** `impl A.lay`, không trả `impl B.lay` cùng tên.
#[test]
fn implementation_loc_theo_kieu_nguoi_nhan() {
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, NGUON_HAI)),
        khung(&implementation(2, URI, 15, 6))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let tra = format!(
        r#"{{"jsonrpc":"2.0","id":2,"result":[{}]}}"#,
        loc(URI, 7, 7, 7, 10)
    );
    assert_eq!(frame_cuoi(&out), tra);
}

/// Nguồn: hai `impl` cùng tên `lay`; `fn g()` hỏi `z.lay` với `z` **không có kiểu** (chưa khai).
const NGUON_MO_HO: &str = "type A:\n    n: int\n\ntype B:\n    n: int\n\nimpl A:\n    fn lay(self) -> int:\n        self.n\n\nimpl B:\n    fn lay(self) -> int:\n        self.n\n\nfn g() -> int:\n    z.lay()\n";

/// Ca 8 — không suy được kiểu người nhận ⇒ trả **mọi** `impl` cùng tên (không đoán, không rỗng).
#[test]
fn implementation_mo_ho_tra_moi_cung_ten() {
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, NGUON_MO_HO)),
        khung(&implementation(2, URI, 15, 6))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let tra = format!(
        r#"{{"jsonrpc":"2.0","id":2,"result":[{},{}]}}"#,
        loc(URI, 7, 7, 7, 10),
        loc(URI, 11, 7, 11, 10)
    );
    assert_eq!(frame_cuoi(&out), tra);
}

/// Ca 9 — từ không phải phương thức `impl` nào ⇒ `result:null` (không đoán bừa).
#[test]
fn implementation_khong_co_tra_null() {
    let text = "type U:\n    n: int\n\nimpl U:\n    fn lay(self) -> int?:\n        self.n\n\nfn f(u: U) -> int?:\n    u.khong_co\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&implementation(2, URI, 8, 6))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(
        frame_cuoi(&out),
        r#"{"jsonrpc":"2.0","id":2,"result":null}"#
    );
}

/// Ca 10 — khai báo trait không có `impl` nào ⇒ `result:null`.
#[test]
fn implementation_trait_khong_impl_tra_null() {
    let text = "trait Hien:\n    fn hien(self) -> int\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&implementation(2, URI, 1, 7))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(
        frame_cuoi(&out),
        r#"{"jsonrpc":"2.0","id":2,"result":null}"#
    );
}

/// Dựng `n` cặp `type`+`impl` cùng tên phương thức `lay`, thêm lời gọi trên biến không rõ kiểu.
fn nguon_nhieu_impl(n: usize) -> String {
    let mut text = String::new();
    for i in 0..n {
        text.push_str(&format!(
            "type T{i}:\n    n: int\n\nimpl T{i}:\n    fn lay(self) -> int:\n        self.n\n\n"
        ));
    }
    text.push_str("fn g() -> int:\n    z.lay()\n");
    text
}

/// Ca 11 — 51 `impl` cùng tên ⇒ cap **50** vị trí, giữ thứ tự khai.
#[test]
fn implementation_cap_50() {
    let n = 51usize;
    let text = nguon_nhieu_impl(n);
    let dong_goi = n * 7 + 1;
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, &text)),
        khung(&implementation(2, URI, dong_goi as i64, 6))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let body = frame_cuoi(&out);
    let v = vis_interp::json::decode(body).expect("JSON frame cuối");
    let Value::Map(kv) = &v else {
        panic!("frame không phải object")
    };
    let Some(Value::List(locs)) = lay(kv, "result") else {
        panic!("thiếu result: {body}")
    };
    assert_eq!(locs.len(), 50, "cap 50 vị trí");
    let Some(Value::Map(cuoi)) = locs.last() else {
        panic!("Location không phải object")
    };
    let Some(Value::Map(range)) = lay(cuoi, "range") else {
        panic!("thiếu range")
    };
    let Some(Value::Map(start)) = lay(range, "start") else {
        panic!("thiếu start")
    };
    // `impl` thứ 50 (T49) nằm ở dòng 4 + 49*7 = 347.
    assert!(matches!(lay(start, "line"), Some(Value::Int(347))));
}

fn lay<'a>(kv: &'a [(Value, Value)], khoa: &str) -> Option<&'a Value> {
    kv.iter().find_map(|(k, v)| match k {
        Value::Str(s) if s == khoa => Some(v),
        _ => None,
    })
}
