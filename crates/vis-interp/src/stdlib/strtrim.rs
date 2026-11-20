// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (strtrim)** — cắt theo **tập scalar**: `std.str.trim_matches` /
//! `trim_start_matches` / `trim_end_matches`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//! - `chars` là **tập** scalar Unicode: thứ tự **không** quan trọng, lặp **không** sao.
//! - Bỏ **lặp lại** scalar ở đầu/cuối (hoặc cả hai) khi scalar đó thuộc `chars`, tới scalar
//!   đầu tiên (từ phía tương ứng) **không** thuộc tập — như Rust `trim*_matches` với closure.
//! - `chars` rỗng ⇒ trả `s` **nguyên vẹn**; mọi scalar đều thuộc tập ⇒ `""`.
//! - Đơn vị là **scalar** (`char`), **không** phải byte và **không** phải grapheme: emoji
//!   nhiều scalar (vd. `👨‍👩‍👧`) chỉ bị cắt từng scalar có mặt trong `chars`.
//!
//! Không cấp phát thêm ngoài chuỗi kết quả; mọi phép tuyến tính theo `s` × `chars`.

use vis_syntax::Span;

use crate::interp::R;
use crate::value::Value;

/// `true` nếu `c` thuộc **tập** scalar `chars` (duyệt tuyến tính, không cấp phát).
fn thuoc(chars: &str, c: char) -> bool {
    chars.chars().any(|x| x == c)
}

/// Cắt `s` theo `name` (`"str.trim_matches"` / `"str.trim_start_matches"` /
/// `"str.trim_end_matches"`); `chars` rỗng ⇒ `s` nguyên vẹn, toàn bộ thuộc tập ⇒ `""`.
pub(super) fn cat(name: &str, s: &str, chars: &str) -> String {
    if chars.is_empty() {
        return s.to_string();
    }
    let trong = |c: char| thuoc(chars, c);
    match name {
        "str.trim_start_matches" => s.trim_start_matches(trong).to_string(),
        "str.trim_end_matches" => s.trim_end_matches(trong).to_string(),
        _ => s.trim_matches(trong).to_string(),
    }
}

/// Dispatch chung cho ba hàm; `name` là tên đầy đủ `"str.trim*_matches"`.
pub(super) fn goi(name: &str, args: &[Value], span: Span) -> R<Value> {
    let s = super::chuoi(name, &args[0], 0, span)?;
    let chars = super::chuoi(name, &args[1], 1, span)?;
    Ok(Value::Str(cat(name, s, chars)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tap_scalar_hai_phia() {
        assert_eq!(cat("str.trim_matches", "--x--", "-"), "x");
        assert_eq!(cat("str.trim_start_matches", "--x--", "-"), "x--");
        assert_eq!(cat("str.trim_end_matches", "--x--", "-"), "--x");
        assert_eq!(cat("str.trim_matches", "abbaHELLOabba", "ab"), "HELLO");
        assert_eq!(cat("str.trim_matches", "abbaHELLOabba", "ba"), "HELLO");
        assert_eq!(cat("str.trim_matches", "a-b", "-a"), "b");
    }

    #[test]
    fn rong_va_toan_bo() {
        assert_eq!(cat("str.trim_matches", "abc", ""), "abc");
        assert_eq!(cat("str.trim_matches", "xyx", "xy"), "");
        assert_eq!(cat("str.trim_start_matches", "xxx", "x"), "");
        assert_eq!(cat("str.trim_end_matches", "xxx", "x"), "");
    }

    #[test]
    fn scalar_khong_phai_grapheme() {
        assert_eq!(cat("str.trim_matches", "đ😀đxđ😀đ", "đ😀"), "x");
        assert_eq!(cat("str.trim_start_matches", "👨‍👩‍👧", "👨").chars().count(), 4);
        assert_eq!(cat("str.trim_matches", "👨‍👩‍👧", "👨👧").chars().count(), 3);
    }
}
