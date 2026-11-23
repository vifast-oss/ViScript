// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-273** — `textDocument/signatureHelp` cho LSP.
//!
//! Chỉ hỗ trợ **hàm cấp tệp** đã có trong `items` (detail `fn …`) — đủ cho đa số lời gọi; gọi
//! phương thức/builtin không có chữ ký trong HIR tệp ⇒ `result:null` (không đoán).
//!
//! Ngữ cảnh xác định bằng **token** (lexer), nên dấu ngoặc trong chuỗi/chú thích không đánh lừa.
//! `activeParameter` = số dấu phẩy ở độ sâu 0 giữa `(` và con trỏ.

use vis_core::value::Value;
use vis_syntax::{lex, Kind, Mode};

use super::jsonrpc::{m, phan_hoi, s};
use super::phan_tich::KetQua;

/// Tìm `(` chưa đóng gần nhất trước `byte` + tên hàm ngay trước nó.
fn ngu_canh(text: &str, byte: usize, mode: Mode) -> Option<(String, usize, usize)> {
    let lexed = lex(text, mode);
    let mut stack: Vec<usize> = Vec::new();
    let mut open = None;
    let mut open_idx = 0usize;
    for (i, t) in lexed.tokens.iter().enumerate() {
        if t.span.start >= byte {
            break;
        }
        if t.kind == Kind::Punct && t.text == "(" {
            stack.push(i);
        } else if t.kind == Kind::Punct && t.text == ")" {
            stack.pop();
        }
        if let Some(&idx) = stack.last() {
            open = Some(idx);
            open_idx = i;
        }
    }
    let idx = open?;
    let _ = open_idx;
    let open_tok = &lexed.tokens[idx];
    // Tên hàm: token Ident ngay trước `(`, bỏ qua khoảng trắng (token DEDENT/EOF không có).
    let mut j = idx;
    while j > 0 {
        j -= 1;
        let t = &lexed.tokens[j];
        if t.kind == Kind::Ident {
            return Some((t.text.clone(), open_tok.span.start, byte));
        }
        if t.kind == Kind::Newline || t.kind == Kind::Indent || t.kind == Kind::Dedent {
            return None;
        }
    }
    None
}

/// Đếm dấu phẩy độ sâu 0 giữa `(` và con trỏ (dùng token).
fn chi_so_tham_so(text: &str, open: usize, byte: usize, mode: Mode) -> usize {
    let lexed = lex(text, mode);
    let mut depth = 0i32;
    let mut n = 0usize;
    for t in &lexed.tokens {
        if t.span.start <= open {
            continue;
        }
        if t.span.start >= byte {
            break;
        }
        if t.kind == Kind::Punct {
            match t.text.as_str() {
                "(" | "[" | "{" => depth += 1,
                ")" | "]" | "}" => depth -= 1,
                "," if depth == 0 => n += 1,
                _ => {}
            }
        }
    }
    n
}

/// Tách danh sách tham số trong detail `fn name(a: T, b: U) -> R` (độ sâu ngoặc 0).
fn tach_tham_so(detail: &str, ten: &str) -> Vec<String> {
    let Some(p) = detail.find('(') else {
        return Vec::new();
    };
    let mut depth = 0i32;
    let mut end = None;
    for (i, c) in detail[p..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(p + i);
                    break;
                }
            }
            _ => {}
        }
    }
    let Some(end) = end else {
        return Vec::new();
    };
    let body = &detail[p + 1..end];
    if body.trim().is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (i, c) in body.char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                out.push(body[start..i].trim().to_string());
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(body[start..].trim().to_string());
    let _ = ten;
    out
}

/// Phản hồi `textDocument/signatureHelp` (`SignatureHelp`) hoặc `result:null`.
pub fn tra_loi_signature(kq: &KetQua, byte: usize, mode: Mode, id: &Value) -> Vec<u8> {
    let Some((ten, open, _)) = ngu_canh(&kq.text, byte, mode) else {
        return phan_hoi(id, Value::none());
    };
    let Some(it) = kq.items.iter().find(|g| g.label == ten) else {
        return phan_hoi(id, Value::none());
    };
    let Some(detail) = it.detail.as_deref() else {
        return phan_hoi(id, Value::none());
    };
    if !detail.starts_with("fn ") {
        return phan_hoi(id, Value::none());
    }
    let tham_so = tach_tham_so(detail, &ten);
    let params: Vec<Value> = tham_so.iter().map(|p| m(vec![("label", s(p))])).collect();
    let active = chi_so_tham_so(&kq.text, open, byte, mode);
    phan_hoi(
        id,
        m(vec![
            (
                "signatures",
                Value::List(vec![m(vec![
                    ("label", s(detail)),
                    ("parameters", Value::List(params)),
                ])]),
            ),
            ("activeSignature", Value::Int(0)),
            (
                "activeParameter",
                Value::Int(active.min(tham_so.len().saturating_sub(1)) as i64),
            ),
        ]),
    )
}
