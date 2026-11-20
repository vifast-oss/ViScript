// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (char-at)** — scalar Unicode tại chỉ số: `std.str.char_at`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//! - `char_at(s, i)` trả **scalar Unicode** thứ `i` (**zero-based**, cùng đơn vị
//!   `char_count`/`find`) dưới dạng `str` bọc `Some`.
//! - `i < 0`, `i >= char_count(s)`, hoặc `s` rỗng ⇒ `None` — **không trap**, **không** cắt
//!   byte: chỉ số luôn đếm theo `char`, không phải byte UTF-8.
//! - Kết quả là **một scalar** (`char`), **không** phải grapheme: `char_at("e\u{301}", 1)`
//!   trả scalar dấu kết hợp `"\u{301}"`; emoji ZWJ (`👨‍👩‍👧`) tách theo từng scalar.
//! - Không có trần riêng: duyệt `chars().nth(i)` là O(i), không cấp phát theo độ dài `s`.

use vis_syntax::Span;

use crate::interp::R;
use crate::value::Value;

/// Scalar Unicode tại chỉ số `i` (zero-based); âm hoặc vượt quá `char_count` ⇒ `None`.
/// `i` là `i64` để giữ đúng miền âm/dương trước khi chuyển `usize`.
pub(super) fn char_at(s: &str, i: i64) -> Option<char> {
    let i = usize::try_from(i).ok()?;
    s.chars().nth(i)
}

/// Dispatch chung: kiểm kiểu lúc chạy (sai ⇒ `VS0501`) rồi trả `str?`.
pub(super) fn goi(name: &str, args: &[Value], span: Span) -> R<Value> {
    let s = super::chuoi(name, &args[0], 0, span)?;
    let i = super::so(name, &args[1], 1, span)?;
    Ok(Value::opt(char_at(s, i).map(|c| Value::Str(c.to_string()))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_theo_chi_so_zero_based() {
        assert_eq!(char_at("abc", 0), Some('a'));
        assert_eq!(char_at("abc", 1), Some('b'));
        assert_eq!(char_at("abc", 2), Some('c'));
    }

    #[test]
    fn unicode_va_emoji_theo_scalar_khong_phai_byte() {
        assert_eq!(char_at("đ😀x", 0), Some('đ'));
        assert_eq!(char_at("đ😀x", 1), Some('😀'));
        assert_eq!(char_at("đ😀x", 2), Some('x'));
        assert_eq!(char_at("😀", 0), Some('😀'));
        // Dấu kết hợp là scalar riêng — không phải grapheme.
        assert_eq!(char_at("e\u{301}", 0), Some('e'));
        assert_eq!(char_at("e\u{301}", 1), Some('\u{301}'));
        // ZWJ và từng emoji thành phần đều là scalar riêng.
        assert_eq!(char_at("👨‍👩‍👧", 0), Some('👨'));
        assert_eq!(char_at("👨‍👩‍👧", 1), Some('\u{200d}'));
        assert_eq!(char_at("👨‍👩‍👧", 2), Some('👩'));
    }

    #[test]
    fn am_qua_dai_va_rong_tra_none_khong_trap() {
        assert_eq!(char_at("abc", -1), None);
        assert_eq!(char_at("abc", i64::MIN), None);
        assert_eq!(char_at("abc", 3), None);
        assert_eq!(char_at("abc", i64::MAX), None);
        assert_eq!(char_at("", 0), None);
        assert_eq!(char_at("", -1), None);
        assert_eq!(char_at("", 1), None);
    }
}
