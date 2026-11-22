// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-272** — `textDocument/rename` (đổi tên trong tệp) cho LSP.
//!
//! Chỉ đổi các token **định danh** trùng tên (lexer), nên không chạm chuỗi/chú thích/từ khoá.
//! Tên mới phải là định danh hợp lệ và không phải từ khoá; nếu không ⇒ `result:null`.
//! Trả `WorkspaceEdit` dạng `changes: { uri: TextEdit[] }`, cap 200.

use vis_interp::value::Value;
use vis_syntax::{is_keyword, lex, Kind, Mode, Span};

use super::goi_y::tu_tai;
use super::jsonrpc::{m, phan_hoi, s};
use super::phan_tich::KetQua;
use super::vi_tri::khoang;

/// Định danh ViScript: chữ cái/`_` đầu, sau đó chữ/số/`_`; không phải từ khoá.
fn ten_hop_le(ten: &str) -> bool {
    if ten.is_empty() || is_keyword(ten) {
        return false;
    }
    let mut it = ten.chars();
    match it.next() {
        Some(c) if c.is_alphabetic() || c == '_' => {}
        _ => return false,
    }
    it.all(|c| c.is_alphanumeric() || c == '_')
}

/// Phản hồi `textDocument/rename` (`WorkspaceEdit`) hoặc `result:null`.
pub fn tra_loi_rename(
    kq: &KetQua,
    uri: &str,
    byte: usize,
    ten_moi: &str,
    mode: Mode,
    id: &Value,
) -> Vec<u8> {
    if !ten_hop_le(ten_moi) {
        return phan_hoi(id, Value::none());
    }
    let (start, end) = tu_tai(&kq.text, byte);
    if start == end {
        return phan_hoi(id, Value::none());
    }
    let ten_cu = &kq.text[start..end];
    if ten_cu == ten_moi {
        return phan_hoi(id, Value::none());
    }
    let lexed = lex(&kq.text, mode);
    let pos = |line: u32, character: u32| {
        m(vec![
            ("line", Value::Int(line as i64)),
            ("character", Value::Int(character as i64)),
        ])
    };
    let mut edits: Vec<Value> = Vec::new();
    for t in &lexed.tokens {
        if t.kind != Kind::Ident || t.text != ten_cu {
            continue;
        }
        let ((l0, c0), (l1, c1)) = khoang(&kq.text, t.span);
        edits.push(m(vec![
            (
                "range",
                m(vec![("start", pos(l0, c0)), ("end", pos(l1, c1))]),
            ),
            ("newText", s(ten_moi)),
        ]));
        if edits.len() >= 200 {
            break;
        }
    }
    if edits.is_empty() {
        return phan_hoi(id, Value::none());
    }
    phan_hoi(id, m(vec![("changes", m(vec![(uri, Value::List(edits))]))]))
}

/// **ST-280**: `textDocument/prepareRename` — trả `{range, placeholder}` nếu từ tại con trỏ là
/// định danh hợp lệ (không phải từ khoá), ngược lại `null`.
pub fn tra_loi_prepare(kq: &KetQua, byte: usize, id: &Value) -> Vec<u8> {
    let (start, end) = tu_tai(&kq.text, byte);
    if start == end {
        return phan_hoi(id, Value::none());
    }
    let ten = &kq.text[start..end];
    if !ten_hop_le(ten) {
        return phan_hoi(id, Value::none());
    }
    let ((l0, c0), (l1, c1)) = khoang(&kq.text, Span::new(start, end));
    phan_hoi(
        id,
        m(vec![
            (
                "range",
                m(vec![
                    (
                        "start",
                        m(vec![
                            ("line", Value::Int(l0 as i64)),
                            ("character", Value::Int(c0 as i64)),
                        ]),
                    ),
                    (
                        "end",
                        m(vec![
                            ("line", Value::Int(l1 as i64)),
                            ("character", Value::Int(c1 as i64)),
                        ]),
                    ),
                ]),
            ),
            ("placeholder", s(ten)),
        ]),
    )
}
