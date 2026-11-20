// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (strlen)** — độ dài chuỗi và rỗng: `std.str.byte_len` / `char_count` / `is_empty`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//! - `byte_len(s)` = độ dài **byte UTF-8** (`str::len` của Rust) — `"đ"` = 2, `"😀"` = 4.
//! - `char_count(s)` = số **scalar Unicode** (`chars().count()`), **không** phải byte và
//!   **không** phải grapheme: `"đ😀x"` = 3; `"e\u{301}"` (e + dấu) = **2**.
//! - `is_empty(s)` = `s` rỗng **byte** (tương đương 0 scalar) ⇒ `bool`.
//! - Không có trần riêng: mọi phép là O(độ dài chuỗi), không cấp phát.
//!
//! **Không** có `grapheme_count`: đếm cụm grapheme cần bảng phân đoạn Unicode
//! (`unicode-segmentation`) — một dependency mới, ngoài phạm vi VD-270 (zero-dep). Đếm
//! grapheme sẽ đến khi A0 chốt dependency; `char_count` chỉ đếm scalar.

use crate::value::Value;

/// Độ dài **byte UTF-8** của `s` (`"đ😀x"` ⇒ 7: 2 + 4 + 1).
pub(super) fn byte_len(s: &str) -> i64 {
    s.len() as i64
}

/// Số **scalar Unicode** của `s` (`"đ😀x"` ⇒ 3), cùng đơn vị `chars()`.
pub(super) fn char_count(s: &str) -> i64 {
    s.chars().count() as i64
}

/// `s` rỗng hay không (0 byte ⇔ 0 scalar).
pub(super) fn is_empty(s: &str) -> bool {
    s.is_empty()
}

/// Dispatch chung cho ba hàm; `name` là tên đầy đủ `"str.byte_len"`/`"str.char_count"`/
/// `"str.is_empty"`.
pub(super) fn goi(name: &str, s: &str) -> Value {
    match name {
        "str.byte_len" => Value::Int(byte_len(s)),
        "str.char_count" => Value::Int(char_count(s)),
        _ => Value::Bool(is_empty(s)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_len_la_utf8_khong_phai_scalar() {
        assert_eq!(byte_len(""), 0);
        assert_eq!(byte_len("abc"), 3);
        assert_eq!(byte_len("đ"), 2);
        assert_eq!(byte_len("😀"), 4);
        assert_eq!(byte_len("đ😀x"), 7);
    }

    #[test]
    fn char_count_la_scalar_unicode() {
        assert_eq!(char_count(""), 0);
        assert_eq!(char_count("abc"), 3);
        assert_eq!(char_count("đ"), 1);
        assert_eq!(char_count("😀"), 1);
        assert_eq!(char_count("đ😀x"), 3);
        // Dấu kết hợp là scalar **riêng** — char_count không phải grapheme count.
        assert_eq!(char_count("e\u{301}"), 2);
        assert_eq!(byte_len("e\u{301}"), 3);
    }

    #[test]
    fn is_empty_theo_byte_va_scalar() {
        assert!(is_empty(""));
        assert!(!is_empty(" "));
        assert!(!is_empty("đ"));
        assert!(!is_empty("😀"));
    }
}
