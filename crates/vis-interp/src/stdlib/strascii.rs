// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (str-is-ascii)** — `std.str.is_ascii`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//! - `is_ascii(s)` = **`true` khi và chỉ khi mọi byte** của `s` thuộc `0x00..=0x7F`.
//! - Chuỗi rỗng ⇒ `true`; một **scalar** non-ASCII (mọi scalar > `0x7F` đều mã hoá UTF-8
//!   bằng byte ≥ `0x80`) ⇒ `false`. Chuỗi `str` của VS luôn là UTF-8 hợp lệ nên không có
//!   trường hợp "byte rời" — điều kiện theo byte và theo scalar là **tương đương**.
//! - Không trap, không cấp phát, O(độ dài byte), dừng ở byte non-ASCII đầu tiên.
//!
//! `\0`, `\n`, `\t`, `\r` và DEL `0x7F` đều **là** ASCII (`true`); NUL không đặc biệt.

use crate::value::Value;

/// `true` ⇔ **mọi byte** của `s` nằm trong `0x00..=0x7F` (`str::is_ascii` của Rust).
pub(super) fn is_ascii(s: &str) -> bool {
    s.is_ascii()
}

/// Dispatch `str.is_ascii`; chỉ một hàm nên không cần `name`.
pub(super) fn goi(s: &str) -> Value {
    Value::Bool(is_ascii(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rong_va_ascii_day_du() {
        assert!(is_ascii(""));
        assert!(is_ascii("abcXYZ"));
        assert!(is_ascii("0123456789"));
        assert!(is_ascii("!@#$%^&*()_+-=[]{};':\",./<>?`~|\\"));
        assert!(is_ascii(" \t\r\n"));
        // NUL và DEL đều thuộc 0x00..=0x7F.
        assert!(is_ascii("\0"));
        assert!(is_ascii("\u{7f}"));
        assert!(is_ascii("a\0b\nc\u{7f}d"));
    }

    #[test]
    fn moi_scalar_ascii_deu_true() {
        let s: String = (0x00u32..=0x7F).filter_map(char::from_u32).collect();
        assert_eq!(s.chars().count(), 128);
        assert!(is_ascii(&s));
        assert!(s.bytes().all(|b| b <= 0x7F));
    }

    #[test]
    fn non_ascii_va_astral_deu_false() {
        // U+0080 là scalar đầu tiên ngoài ASCII (UTF-8: C2 80).
        assert!(!is_ascii("\u{80}"));
        assert!(!is_ascii("đ"));
        assert!(!is_ascii("xin chào"));
        assert!(!is_ascii("😀"));
        assert!(!is_ascii("\u{10FFFF}"));
        assert!(!is_ascii("👨‍👩‍👧"));
        // Dấu kết hợp (scalar riêng) cũng non-ASCII.
        assert!(!is_ascii("e\u{301}"));
    }

    #[test]
    fn tron_ascii_va_non_ascii_deu_false() {
        for s in ["abcđ", "đabc", "a😀b", "abc def\nđ", "0\u{80}1", "\0😀"] {
            assert!(!is_ascii(s), "phải false: {s:?}");
        }
    }

    #[test]
    fn tuong_duong_kiem_theo_byte() {
        for s in [
            "",
            "plain ascii",
            "\0\n\t\r\u{7f}",
            "đ",
            "😀",
            "abcđxyz",
            "e\u{301}",
        ] {
            assert_eq!(
                is_ascii(s),
                s.bytes().all(|b| b <= 0x7F),
                "lệch với kiểm byte: {s:?}"
            );
        }
    }
}
