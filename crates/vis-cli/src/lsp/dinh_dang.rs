// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-279** — `textDocument/formatting` cho LSP.
//!
//! Dùng đúng formatter của CLI (`vis_fmt::format_mode`) trên văn bản **đã chuẩn hoá** của LSP;
//! trả **một** `TextEdit` phủ toàn tệp, hoặc `[]` nếu không đổi/lỗi cú pháp (client giữ nguyên).

use vis_interp::value::Value;
use vis_syntax::Mode;

use super::jsonrpc::{m, phan_hoi, s};
use super::phan_tich::KetQua;
use super::vi_tri::vi_tri_utf16;

/// Phản hồi `textDocument/formatting` (`TextEdit[]`).
pub fn tra_loi_format(kq: &KetQua, mode: Mode, id: &Value) -> Vec<u8> {
    let Ok(out) = vis_fmt::format_mode(&kq.text, mode) else {
        return phan_hoi(id, Value::List(Vec::new()));
    };
    if out == kq.text {
        return phan_hoi(id, Value::List(Vec::new()));
    }
    let pos = |line: u32, character: u32| {
        m(vec![
            ("line", Value::Int(line as i64)),
            ("character", Value::Int(character as i64)),
        ])
    };
    let (l0, c0) = vi_tri_utf16(&kq.text, 0);
    let (l1, c1) = vi_tri_utf16(&kq.text, kq.text.len());
    phan_hoi(
        id,
        Value::List(vec![m(vec![
            (
                "range",
                m(vec![("start", pos(l0, c0)), ("end", pos(l1, c1))]),
            ),
            ("newText", s(&out)),
        ])]),
    )
}
