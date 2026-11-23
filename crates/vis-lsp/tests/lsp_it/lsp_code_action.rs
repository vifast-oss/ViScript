// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `textDocument/codeAction` cho LSP.
//!
//! Hợp đồng: `initialize` quảng cáo `codeActionProvider.codeActionKinds = ["source.fixAll"]`;
//! request trả **mảng** action, hiện chỉ một action "Định dạng tài liệu" (`kind: source.fixAll`)
//! với `edit.changes[uri]` = **một** `TextEdit` phủ toàn tài liệu bằng bản `vis fmt` — chỉ khi
//! bản format **khác** nội dung hiện tại. Đã đúng format/cú pháp hỏng/tài liệu rỗng ⇒ `[]`.
//! `range` + `context` bị bỏ qua có tài liệu hoá; URI chưa mở ⇒ `-32602` như semanticTokens.

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use vis_interp::value::Value;

const URI: &str = "file:///a.vis";
const URI_VISH: &str = "file:///a.vish";

fn khung(body: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{body}", body.len())
}

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

fn vi_tri(line: i64, character: i64) -> Value {
    m(vec![
        ("line", Value::Int(line)),
        ("character", Value::Int(character)),
    ])
}

fn khoang(l0: i64, c0: i64, l1: i64, c1: i64) -> Value {
    m(vec![("start", vi_tri(l0, c0)), ("end", vi_tri(l1, c1))])
}

/// Request `textDocument/codeAction` đầy đủ `textDocument.uri` + `range` + `context` như client.
fn code_action(id: i64, uri: &str, range: Value, context: Value) -> String {
    so(&m(vec![
        ("jsonrpc", s("2.0")),
        ("id", Value::Int(id)),
        ("method", s("textDocument/codeAction")),
        (
            "params",
            m(vec![
                ("textDocument", m(vec![("uri", s(uri))])),
                ("range", range),
                ("context", context),
            ]),
        ),
    ]))
}

/// Body frame **cuối** trong stdout (bỏ header).
fn frame_cuoi(out: &str) -> &str {
    let i = out.rfind("Content-Length: ").expect("có frame");
    let j = out[i..].find("\r\n\r\n").expect("header") + i + 4;
    &out[j..]
}

/// Body của frame có `"id":<id>` (bỏ qua publishDiagnostics).
fn body_id(out: &str, id: i64) -> &str {
    let needle = format!(r#"{{"jsonrpc":"2.0","id":{id},"#);
    let i = out.find(&needle).expect("có frame với id");
    let j = out[i..]
        .find("Content-Length: ")
        .map_or(out.len(), |k| i + k);
    out[i..j].trim_end()
}

fn lay<'a>(kv: &'a [(Value, Value)], khoa: &str) -> Option<&'a Value> {
    kv.iter().find_map(|(k, v)| match k {
        Value::Str(s) if s == khoa => Some(v),
        _ => None,
    })
}

fn truong_map<'a>(kv: &'a [(Value, Value)], khoa: &str) -> &'a [(Value, Value)] {
    match lay(kv, khoa) {
        Some(Value::Map(x)) => x,
        _ => panic!("thiếu object `{khoa}`"),
    }
}

fn truong_list<'a>(kv: &'a [(Value, Value)], khoa: &str) -> &'a [Value] {
    match lay(kv, khoa) {
        Some(Value::List(x)) => x,
        _ => panic!("thiếu mảng `{khoa}`"),
    }
}

fn truong_chuoi<'a>(kv: &'a [(Value, Value)], khoa: &str) -> &'a str {
    match lay(kv, khoa) {
        Some(Value::Str(x)) => x,
        _ => panic!("thiếu chuỗi `{khoa}`"),
    }
}

fn truong_so(kv: &[(Value, Value)], khoa: &str) -> i64 {
    match lay(kv, khoa) {
        Some(Value::Int(n)) => *n,
        _ => panic!("thiếu số `{khoa}`"),
    }
}

/// `result` của frame (mảng `CodeAction[]`).
fn ket_qua(body: &str) -> Vec<Value> {
    let v = vis_interp::json::decode(body).expect("JSON frame hợp lệ");
    let Value::Map(kv) = &v else {
        panic!("frame phải là object: {body}")
    };
    truong_list(kv, "result").to_vec()
}

/// `(line, character)` UTF-16 → byte offset (bản test của `lsp::vi_tri::byte_tu_vi_tri`).
fn byte_tu(text: &str, line: i64, character: i64) -> usize {
    let mut l = 0i64;
    let mut c = 0i64;
    for (i, ch) in text.char_indices() {
        if l == line && c >= character {
            return i;
        }
        if ch == '\n' {
            if l == line {
                return i;
            }
            l += 1;
            c = 0;
        } else if l == line {
            c += ch.len_utf16() as i64;
        }
    }
    text.len()
}

fn doc_vi_tri(range: &[(Value, Value)], khoa: &str) -> (i64, i64) {
    let p = truong_map(range, khoa);
    (truong_so(p, "line"), truong_so(p, "character"))
}

/// Áp dụng **một** `TextEdit` lên văn bản gốc (vị trí UTF-16) — kiểm edit thật sự cho ra bản format.
fn ap_dung(text: &str, edit: &Value) -> String {
    let Value::Map(kv) = edit else {
        panic!("TextEdit phải là object")
    };
    let range = truong_map(kv, "range");
    let (l0, c0) = doc_vi_tri(range, "start");
    let (l1, c1) = doc_vi_tri(range, "end");
    let a = byte_tu(text, l0, c0);
    let b = byte_tu(text, l1, c1);
    let moi = truong_chuoi(kv, "newText");
    format!("{}{}{}", &text[..a], moi, &text[b..])
}

/// Ca 1 — `initialize` quảng cáo đúng `codeActionProvider` + vẫn giữ provider format cũ.
#[test]
fn lsp_code_action_initialize_quang_cao() {
    let (code, out, err) = chay(&khung(&init()));
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(err, "", "stderr phải rỗng");

    let v = vis_interp::json::decode(frame_cuoi(&out)).expect("JSON frame hợp lệ");
    let Value::Map(kv) = &v else {
        panic!("frame phải là object")
    };
    let kq = truong_map(kv, "result");
    let caps = truong_map(kq, "capabilities");
    let ca = truong_map(caps, "codeActionProvider");
    let kinds: Vec<&str> = truong_list(ca, "codeActionKinds")
        .iter()
        .map(|v| match v {
            Value::Str(s) => s.as_str(),
            _ => panic!("codeActionKinds phải là chuỗi"),
        })
        .collect();
    assert_eq!(kinds, ["source.fixAll"]);
    assert!(
        matches!(
            lay(caps, "documentFormattingProvider"),
            Some(Value::Bool(true))
        ),
        "giữ documentFormattingProvider"
    );
    assert!(
        lay(caps, "semanticTokensProvider").is_some(),
        "không đổi capability cũ"
    );
}

/// Ca 2 — tab trong thụt lề ⇒ đúng **một** action `source.fixAll`; edit phủ toàn tệp và áp dụng
/// cho ra **đúng** bản format của `vis fmt`.
#[test]
fn lsp_code_action_sua_khi_can_format() {
    let text = "fn main():\n\tprint(1)\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&code_action(
            2,
            URI,
            khoang(0, 0, 2, 0),
            m(vec![("diagnostics", Value::List(vec![]))])
        ))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(err, "");

    let ds = ket_qua(body_id(&out, 2));
    assert_eq!(ds.len(), 1, "phải có đúng một action: {ds:?}");
    let Value::Map(action) = &ds[0] else {
        panic!("action phải là object")
    };
    assert_eq!(truong_chuoi(action, "title"), "Định dạng tài liệu");
    assert_eq!(truong_chuoi(action, "kind"), "source.fixAll");

    let edit = truong_map(action, "edit");
    let changes = truong_map(edit, "changes");
    let edits = truong_list(changes, URI);
    assert_eq!(edits.len(), 1, "một TextEdit toàn tệp");
    let Value::Map(e) = &edits[0] else {
        panic!("TextEdit phải là object")
    };
    let range = truong_map(e, "range");
    assert_eq!(doc_vi_tri(range, "start"), (0, 0));
    assert_eq!(
        doc_vi_tri(range, "end"),
        (2, 0),
        "kết thúc tệp: dòng 2 (sau newline cuối), cột 0"
    );
    let moi = truong_chuoi(e, "newText");
    assert_eq!(moi, "fn main():\n    print(1)\n");
    assert_ne!(moi, text, "chỉ trả action khi bản format khác nội dung");
    assert_eq!(ap_dung(text, &edits[0]), moi, "áp dụng edit = bản format");
}

/// Ca 3 — tài liệu **đã đúng** format ⇒ `[]` (formatter là điểm bất động), không action rỗng.
#[test]
fn lsp_code_action_da_format_khong_action() {
    for text in [
        "fn main():\n    print(1)\n",
        "fn cong(a: int, b: int) -> int:\n    return a + b\n",
    ] {
        let vao = format!(
            "{}{}{}",
            khung(&init()),
            khung(&did_open(URI, text)),
            khung(&code_action(2, URI, khoang(0, 0, 0, 0), m(vec![])))
        );
        let (code, out, err) = chay(&vao);
        assert_eq!(code, 0, "stderr: {err}");
        assert_eq!(
            frame_cuoi(&out),
            r#"{"jsonrpc":"2.0","id":2,"result":[]}"#,
            "nguồn đã format: {text:?}"
        );
    }
}

/// Ca 4 — tài liệu rỗng: formatter thêm newline cuối ⇒ action `newText: "\n"` (đúng `vis fmt`,
/// không special-case); tệp `"\n"` là điểm bất động ⇒ `[]`.
#[test]
fn lsp_code_action_tai_lieu_rong() {
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, "")),
        khung(&code_action(2, URI, khoang(0, 0, 0, 0), m(vec![])))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(err, "");
    let ds = ket_qua(body_id(&out, 2));
    assert_eq!(ds.len(), 1, "formatter thêm newline cuối: {ds:?}");
    let Value::Map(action) = &ds[0] else {
        panic!("action phải là object")
    };
    let edits = truong_list(truong_map(truong_map(action, "edit"), "changes"), URI);
    assert_eq!(edits.len(), 1);
    assert_eq!(ap_dung("", &edits[0]), "\n");

    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, "\n")),
        khung(&code_action(2, URI, khoang(0, 0, 0, 0), m(vec![])))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(frame_cuoi(&out), r#"{"jsonrpc":"2.0","id":2,"result":[]}"#);
}

/// Ca 5 — cú pháp hỏng ⇒ `[]` (formatter từ chối, không sửa dở dang).
#[test]
fn lsp_code_action_cu_phap_hong_rong() {
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, "fn main(:\n")),
        khung(&code_action(2, URI, khoang(0, 0, 0, 0), m(vec![])))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(frame_cuoi(&out), r#"{"jsonrpc":"2.0","id":2,"result":[]}"#);
}

/// Ca 6 — URI chưa mở ⇒ lỗi chuẩn `-32602`; trước `initialize` ⇒ `-32002` (không đoán nội dung).
#[test]
fn lsp_code_action_uri_chua_mo_va_truoc_initialize() {
    let la = "file:///khong-mo.vis";
    let vao = format!(
        "{}{}",
        khung(&init()),
        khung(&code_action(2, la, khoang(0, 0, 0, 0), m(vec![])))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(
        frame_cuoi(&out),
        r#"{"jsonrpc":"2.0","id":2,"error":{"code":-32602,"message":"tài liệu chưa mở","data":"file:///khong-mo.vis"}}"#
    );

    let (code, out, err) = chay(&khung(&code_action(2, la, khoang(0, 0, 0, 0), m(vec![]))));
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(
        frame_cuoi(&out),
        r#"{"jsonrpc":"2.0","id":2,"error":{"code":-32002,"message":"chưa `initialize`","data":null}}"#
    );
}

/// Ca 7 — `range`/`context` tuỳ ý (ngoài tệp, chỉ `quickfix`) vẫn **bỏ qua** ⇒ action toàn tệp.
#[test]
fn lsp_code_action_bo_qua_range_va_context() {
    let text = "fn main():\n\tprint(1)\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&code_action(
            2,
            URI,
            khoang(99, 42, 100, 7),
            m(vec![
                ("only", Value::List(vec![s("quickfix")])),
                ("diagnostics", Value::List(vec![])),
            ])
        ))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let ds = ket_qua(body_id(&out, 2));
    assert_eq!(ds.len(), 1, "range/context không lọc action full-document");
    let Value::Map(action) = &ds[0] else {
        panic!("action phải là object")
    };
    let edits = truong_list(truong_map(truong_map(action, "edit"), "changes"), URI);
    assert_eq!(edits.len(), 1);
    assert_eq!(ap_dung(text, &edits[0]), "fn main():\n    print(1)\n");
}

/// Ca 8 — URI `.vish` ⇒ chế độ **lệnh**: `|` hợp lệ, bản format chỉ bỏ khoảng trắng cuối dòng;
/// tệp `.vish` đã sạch ⇒ không action. (Nếu dispatch nhầm chế độ script, `|` là lỗi cú pháp.)
#[test]
fn lsp_code_action_vish_dung_che_do_lenh() {
    let text = "ls -la | wc -l   \n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI_VISH, text)),
        khung(&code_action(2, URI_VISH, khoang(0, 0, 0, 0), m(vec![])))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let ds = ket_qua(body_id(&out, 2));
    assert_eq!(ds.len(), 1, "`.vish` hợp lệ phải có action: {ds:?}");
    let Value::Map(action) = &ds[0] else {
        panic!("action phải là object")
    };
    let edits = truong_list(truong_map(truong_map(action, "edit"), "changes"), URI_VISH);
    assert_eq!(edits.len(), 1);
    assert_eq!(ap_dung(text, &edits[0]), "ls -la | wc -l\n");

    let sach = "ls -la | wc -l\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI_VISH, sach)),
        khung(&code_action(2, URI_VISH, khoang(0, 0, 0, 0), m(vec![])))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(frame_cuoi(&out), r#"{"jsonrpc":"2.0","id":2,"result":[]}"#);
}
