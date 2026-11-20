// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (is-blank)** — `std.str.is_blank(s) -> bool`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//! - `is_blank(s)` = `true` **khi và chỉ khi** `s` rỗng **hoặc mọi scalar Unicode** của `s`
//!   đều là khoảng trắng theo `char::is_whitespace` (thuộc tính Unicode `White_Space`).
//! - Một scalar **không** phải khoảng trắng ở bất kỳ vị trí nào ⇒ `false`; không có ngoại lệ
//!   cho scalar vô hình — ZWSP `U+200B`, BOM `U+FEFF`, MVS `U+180E`, dấu kết hợp `U+0301`
//!   **không** thuộc `White_Space` nên đều làm kết quả `false`.
//! - Đơn vị là **scalar**, **không** phải byte và **không** phải grapheme: `"đ"`/`"😀"` là
//!   scalar không trắng ⇒ `false`; khoảng trắng nhiều scalar (NBSP `U+00A0`, ideographic
//!   space `U+3000`, `\r\n`, …) đều được chấp nhận.
//! - Không trap, không cấp phát, O(số scalar), dừng ở scalar không trắng đầu tiên. Rỗng ⇒
//!   `true` (không có scalar nào vi phạm).
//!
//! Cùng tập khoảng trắng với `std.str.title`/`std.str.trim` (`char::is_whitespace`), nên
//! `is_blank(s)` tương đương `trim(s).is_empty()` về mặt ngữ nghĩa — nhưng duyệt **một lượt**,
//! không cắt chuỗi và không cấp phát.

use crate::value::Value;

/// `true` ⇔ `s` rỗng hoặc **mọi scalar** là khoảng trắng Unicode.
pub(super) fn is_blank(s: &str) -> bool {
    s.chars().all(char::is_whitespace)
}

/// Dispatch `str.is_blank`; chỉ một hàm nên không cần `name`.
pub(super) fn goi(s: &str) -> Value {
    Value::Bool(is_blank(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rong_va_khoang_trang_ascii() {
        assert!(is_blank(""));
        assert!(is_blank(" "));
        assert!(is_blank("   "));
        assert!(is_blank("\t"));
        assert!(is_blank("\n"));
        assert!(is_blank("\r"));
        assert!(is_blank("\r\n"));
        assert!(is_blank("\t \r\n"));
        // VT (U+000B) và FF (U+000C) cũng là White_Space.
        assert!(is_blank("\u{000B}"));
        assert!(is_blank("\u{000C}"));
    }

    #[test]
    fn khoang_trang_unicode() {
        for s in [
            "\u{00A0}", // NBSP
            "\u{3000}", // ideographic space
            "\u{0085}", // NEL
            "\u{1680}", // ogham space mark
            "\u{2000}", // en quad
            "\u{2003}", // em space
            "\u{2028}", // line separator
            "\u{2029}", // paragraph separator
            "\u{202F}", // narrow no-break space
            "\u{205F}", // medium mathematical space
        ] {
            assert!(
                is_blank(s),
                "phải là khoảng trắng: U+{:04X}",
                s.chars().next().unwrap() as u32
            );
        }
        assert!(is_blank(" \t\r\n\u{00A0}\u{3000}"));
        assert!(is_blank("\u{3000}\u{00A0}\r\n\t "));
    }

    #[test]
    fn mot_scalar_khac_la_du() {
        for s in [
            "a",
            " a",
            "a ",
            " a ",
            "a b",
            " \t\n0\r\n",
            "  x  ",
            " - ",
            "\0",
        ] {
            assert!(!is_blank(s), "phải false: {s:?}");
        }
    }

    #[test]
    fn scalar_vo_hinh_va_dau_ket_hop_khong_phai_khoang_trang() {
        // ZWSP, BOM, MVS và dấu kết hợp đều KHÔNG thuộc White_Space.
        for c in ['\u{200B}', '\u{FEFF}', '\u{180E}', '\u{0301}'] {
            let s = c.to_string();
            assert!(!is_blank(&s), "U+{:04X} không phải khoảng trắng", c as u32);
            assert!(!is_blank(&format!(" \t{c} \n")));
        }
    }

    #[test]
    fn unicode_khong_trang_va_astral() {
        for s in [
            "đ",
            "đ ",
            " đ",
            "xin chào",
            "😀",
            " \u{1F600} ",
            "👨‍👩‍👧",
            "Σ",
            "ß",
        ] {
            assert!(!is_blank(s), "phải false: {s:?}");
        }
    }

    #[test]
    fn tuong_duong_trim_rong() {
        // Cùng `char::is_whitespace` với `str::trim`; kiểm chéo trên hỗn hợp.
        for s in [
            "",
            " ",
            "\r\n\t",
            "\u{00A0}\u{3000}",
            "a",
            " a ",
            "\u{200B}",
            "đ",
            "😀",
            "\u{0301}",
        ] {
            assert_eq!(is_blank(s), s.trim().is_empty(), "lệch với trim: {s:?}");
        }
    }
}
