// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `textDocument/semanticTokens/full` cho LSP.
//!
//! Legend **cố định** 9 loại (`keyword`, `type`, `function`, `variable`, `string`, `number`,
//! `comment`, `operator`, `parameter`), không modifier. Dữ liệu là mảng phẳng 5 số/token
//! (`deltaLine`, `deltaStartChar`, `length`, `tokenType`, `tokenModifiers`) theo **0-based
//! UTF-16**; token đầu `deltaLine` tuyệt đối, token sau là hiệu so với token trước.
//!
//! Tệp rỗng/khoảng trắng ⇒ `{"data":[]}`; URI chưa mở ⇒ lỗi `-32602` (InvalidParams). Snapshot
//! `initialize` giữ nguyên thứ tự capability cũ, thêm `semanticTokensProvider` ở cuối.

use std::collections::BTreeSet;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use vis_core::value::Value;

const URI: &str = "file:///a.vis";

/// Snapshot body phản hồi `initialize` — **VD-270** thêm `semanticTokensProvider` sau
/// `completionProvider` (giữ nguyên thứ tự capability cũ).
const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"result":{"capabilities":{"textDocumentSync":{"openClose":true,"change":2},"positionEncoding":"utf-16","hoverProvider":true,"definitionProvider":true,"implementationProvider":true,"documentSymbolProvider":true,"codeLensProvider":{"resolveProvider":false},"foldingRangeProvider":true,"workspaceSymbolProvider":true,"documentFormattingProvider":true,"codeActionProvider":{"codeActionKinds":["source.fixAll"]},"referencesProvider":true,"documentHighlightProvider":true,"selectionRangeProvider":true,"inlayHintProvider":true,"typeDefinitionProvider":true,"renameProvider":{"prepareProvider":true},"signatureHelpProvider":{"triggerCharacters":["(",","]},"completionProvider":{"triggerCharacters":["."]},"semanticTokensProvider":{"legend":{"tokenTypes":["keyword","type","function","variable","string","number","comment","operator","parameter"],"tokenModifiers":[]},"full":true}},"serverInfo":{"name":"vis"}}}"#;

/// Mẫu đủ 9 loại: khai báo `type`/trường, `const`, `fn` + tham số, chú thích, chuỗi, số, toán tử,
/// `test fn`, `assert` (soft keyword) và lời gọi hàm.
const NGUON: &str = "type Diem:\n    x: int\n    y: int\n\nconst GOC: int = 0\n\nfn cong(a: int, b: int) -> int:\n    # cộng hai số\n    let tong = a + b\n    let ten = \"xin chào\"\n    return tong\n\ntest fn kiem_tra():\n    assert cong(1, 2) == 3\n";

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
    Value::map(
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

/// `textDocument/semanticTokens/full` — chỉ cần `textDocument.uri`.
fn semantic_full(id: i64, uri: &str) -> String {
    so(&m(vec![
        ("jsonrpc", s("2.0")),
        ("id", Value::Int(id)),
        ("method", s("textDocument/semanticTokens/full")),
        (
            "params",
            m(vec![("textDocument", m(vec![("uri", s(uri))]))]),
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

/// Mảng `data` (số nguyên) của phản hồi `semanticTokens/full`.
fn data_cua(body: &str) -> Vec<i64> {
    let v = vis_core::json::decode(body).expect("JSON frame hợp lệ");
    let Value::Map(kv) = &v else {
        panic!("frame phải là object")
    };
    let Some(Value::Map(kq)) = lay(kv, "result") else {
        panic!("thiếu result: {body}")
    };
    let Some(Value::List(data)) = lay(kq, "data") else {
        panic!("thiếu data: {body}")
    };
    data.iter()
        .map(|v| match v {
            Value::Int(n) => *n,
            _ => panic!("data phải là số nguyên"),
        })
        .collect()
}

/// Giải mã delta → danh sách `(line, char, length, tokenType, modifiers)`.
fn giai_ma(data: &[i64]) -> Vec<(i64, i64, i64, i64, i64)> {
    let mut out = Vec::new();
    let mut dong = 0i64;
    let mut cot = 0i64;
    for (i, c) in data.as_chunks::<5>().0.iter().enumerate() {
        let (dl, dc) = (c[0], c[1]);
        if i == 0 {
            dong = dl;
            cot = dc;
        } else {
            dong += dl;
            cot = if dl == 0 { cot + dc } else { dc };
        }
        out.push((dong, cot, c[2], c[3], c[4]));
    }
    out
}

fn co_token(ds: &[(i64, i64, i64, i64, i64)], dong: i64, cot: i64, dai: i64, loai: i64) -> bool {
    ds.iter()
        .any(|t| t.0 == dong && t.1 == cot && t.2 == dai && t.3 == loai && t.4 == 0)
}

/// Ca 1 — `initialize` khai báo provider + legend **nguyên văn** và đúng cấu trúc.
#[test]
fn lsp_semtok_initialize_khai_bao_legend() {
    let (code, out, err) = chay(&khung(&init()));
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(err, "", "stderr phải rỗng");
    assert_eq!(frame_cuoi(&out), INIT);

    let v = vis_core::json::decode(frame_cuoi(&out)).expect("JSON frame hợp lệ");
    let Value::Map(kv) = &v else {
        panic!("frame phải là object")
    };
    let Some(Value::Map(kq)) = lay(kv, "result") else {
        panic!("thiếu result")
    };
    let Some(Value::Map(caps)) = lay(kq, "capabilities") else {
        panic!("thiếu capabilities")
    };
    let Some(Value::Map(st)) = lay(caps, "semanticTokensProvider") else {
        panic!("thiếu semanticTokensProvider")
    };
    let Some(Value::Map(legend)) = lay(st, "legend") else {
        panic!("thiếu legend")
    };
    let Some(Value::List(types)) = lay(legend, "tokenTypes") else {
        panic!("thiếu tokenTypes")
    };
    let ten: Vec<&str> = types
        .iter()
        .map(|v| match v {
            Value::Str(s) => s.as_str(),
            _ => panic!("tokenTypes phải là chuỗi"),
        })
        .collect();
    assert_eq!(
        ten,
        [
            "keyword",
            "type",
            "function",
            "variable",
            "string",
            "number",
            "comment",
            "operator",
            "parameter"
        ]
    );
    assert!(matches!(lay(legend, "tokenModifiers"), Some(Value::List(l)) if l.is_empty()));
    assert!(matches!(lay(st, "full"), Some(Value::Bool(true))));
}

/// Ca 2 — mẫu đủ loại: bội số 5, không âm, delta hợp lệ, đúng thứ tự, không chồng lấn, UTF-16.
#[test]
fn lsp_semtok_full_mau_day_du() {
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, NGUON)),
        khung(&semantic_full(2, URI))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(err, "");
    let body = body_id(&out, 2);
    let data = data_cua(body);
    assert!(!data.is_empty(), "mẫu phải sinh token");
    assert_eq!(data.len() % 5, 0, "mảng phẳng 5 số/token");

    for c in data.as_chunks::<5>().0 {
        assert!(c[0] >= 0, "deltaLine không âm");
        assert!(c[1] >= 0, "deltaStartChar không âm");
        assert!(c[2] > 0, "bỏ token rỗng");
        assert!((0..9).contains(&c[3]), "tokenType trong legend");
        assert_eq!(c[4], 0, "tokenModifiers = 0");
    }

    let ds = giai_ma(&data);
    let dong: Vec<&str> = NGUON.lines().collect();
    for (i, t) in ds.iter().enumerate() {
        assert!((t.0 as usize) < dong.len(), "dòng trong tệp: {t:?}");
        let dai_dong: i64 = dong[t.0 as usize]
            .chars()
            .map(|c| c.len_utf16() as i64)
            .sum();
        assert!(
            t.1 + t.2 <= dai_dong,
            "token vượt dòng (UTF-16): {t:?} dài {dai_dong}"
        );
        if let Some(tr) = ds.get(i + 1) {
            assert!(
                tr.0 > t.0 || (tr.0 == t.0 && t.1 + t.2 <= tr.1),
                "token phải tăng và không chồng lấn: {t:?} → {tr:?}"
            );
        }
    }

    // Đủ **cả 9** loại trong legend.
    let loai: BTreeSet<i64> = ds.iter().map(|t| t.3).collect();
    assert_eq!(loai, (0..9).collect(), "phải có đủ 9 loại: {ds:?}");

    // Neo vài token then chốt (0-based, UTF-16).
    assert!(co_token(&ds, 0, 0, 4, 0), "`type` = keyword");
    assert!(co_token(&ds, 0, 5, 4, 1), "`Diem` = type");
    assert!(co_token(&ds, 1, 4, 1, 3), "trường `x` = variable");
    assert!(co_token(&ds, 4, 6, 3, 3), "`GOC` = variable");
    assert!(co_token(&ds, 6, 3, 4, 2), "`cong` = function");
    assert!(co_token(&ds, 6, 8, 1, 8), "tham số `a` = parameter");
    assert!(co_token(&ds, 7, 4, 13, 6), "chú thích = comment");
    assert!(co_token(&ds, 8, 17, 1, 7), "`+` = operator");
    assert!(
        co_token(&ds, 9, 14, 10, 4),
        "`\"xin chào\"` = string (UTF-16)"
    );
    assert!(co_token(&ds, 12, 0, 4, 0), "`test` = keyword mềm");
    assert!(co_token(&ds, 12, 8, 8, 2), "`kiem_tra` = function");
    assert!(co_token(&ds, 13, 4, 6, 0), "`assert` = keyword mềm");
    assert!(co_token(&ds, 13, 11, 4, 2), "callee `cong` = function");
    assert!(co_token(&ds, 13, 25, 1, 5), "`3` = number");
}

/// Ca 3 — tệp rỗng và tệp chỉ khoảng trắng ⇒ `{"data":[]}`.
#[test]
fn lsp_semtok_rong_va_khoang_trang() {
    for nguon in ["", "   \n\t\n\n"] {
        let vao = format!(
            "{}{}{}",
            khung(&init()),
            khung(&did_open(URI, nguon)),
            khung(&semantic_full(2, URI))
        );
        let (code, out, err) = chay(&vao);
        assert_eq!(code, 0, "stderr: {err}");
        assert_eq!(err, "");
        assert_eq!(
            frame_cuoi(&out),
            r#"{"jsonrpc":"2.0","id":2,"result":{"data":[]}}"#,
            "nguồn: {nguon:?}"
        );
    }
}

/// Ca 4 — request lặp lại trên cùng văn bản ⇒ dữ liệu **y hệt** (ổn định, không phụ thuộc cache).
#[test]
fn lsp_semtok_lap_lai_on_dinh() {
    let vao = format!(
        "{}{}{}{}",
        khung(&init()),
        khung(&did_open(URI, NGUON)),
        khung(&semantic_full(2, URI)),
        khung(&semantic_full(3, URI))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(err, "");
    let a = data_cua(body_id(&out, 2));
    let b = data_cua(body_id(&out, 3));
    assert_eq!(a, b, "hai lần trả cùng dữ liệu");
    assert!(!a.is_empty());
}

/// Ca 5 — URI chưa mở ⇒ lỗi chuẩn `-32602` (không đoán nội dung); trước `initialize` ⇒ `-32002`.
#[test]
fn lsp_semtok_uri_chua_mo_va_truoc_initialize() {
    let la = "file:///khong-mo.vis";
    let vao = format!("{}{}", khung(&init()), khung(&semantic_full(2, la)));
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(
        frame_cuoi(&out),
        r#"{"jsonrpc":"2.0","id":2,"error":{"code":-32602,"message":"tài liệu chưa mở","data":"file:///khong-mo.vis"}}"#
    );

    let (code, out, err) = chay(&khung(&semantic_full(2, la)));
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(
        frame_cuoi(&out),
        r#"{"jsonrpc":"2.0","id":2,"error":{"code":-32002,"message":"chưa `initialize`","data":null}}"#
    );
}
