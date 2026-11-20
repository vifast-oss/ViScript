// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-275** — `textDocument/foldingRange` cho LSP.
//!
//! Gấp theo khai báo **nhiều dòng** trong cây ký hiệu (fn/type/enum/trait, trường/variant lồng).
//! `endLine` là dòng cuối thực sự có nội dung: span kết thúc ở đầu dòng kế ⇒ lùi một dòng.

use vis_interp::value::Value;
use vis_syntax::Span;

use super::jsonrpc::{m, phan_hoi};
use super::phan_tich::KetQua;
use super::vi_tri::vi_tri_utf16;

fn dong(text: &str, span: Span) -> (u32, u32) {
    let (sl, _) = vi_tri_utf16(text, span.start);
    let (mut el, ec) = vi_tri_utf16(text, span.end);
    if ec == 0 && el > sl {
        el -= 1;
    }
    (sl, el)
}

fn gom(text: &str, ds: &[super::ky_hieu::KyHieu], out: &mut Vec<Value>) {
    for k in ds {
        let (sl, el) = dong(text, k.span);
        if el > sl {
            out.push(m(vec![
                ("startLine", Value::Int(sl as i64)),
                ("endLine", Value::Int(el as i64)),
            ]));
            if out.len() >= 500 {
                return;
            }
        }
        gom(text, &k.con, out);
    }
}

/// Phản hồi `textDocument/foldingRange` (mảng `FoldingRange`).
pub fn tra_loi_folding(kq: &KetQua, id: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    gom(&kq.text, &kq.ky_hieu, &mut out);
    phan_hoi(id, Value::List(out))
}
