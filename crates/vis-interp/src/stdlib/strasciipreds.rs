// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (ascii-preds)** — bốn vị từ ASCII của `std.str`:
//! `is_ascii_digit`, `is_ascii_alphabetic`, `is_ascii_alphanumeric`, `is_ascii_whitespace`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//! - Mỗi hàm `true` **khi và chỉ khi** `s` **không rỗng** và **mọi scalar** của `s` là ASCII
//!   thuộc nhóm tương ứng — đúng `char::is_ascii_digit` / `is_ascii_alphabetic` /
//!   `is_ascii_alphanumeric` / `is_ascii_whitespace` của Rust (các hàm này **chỉ** nhận
//!   `0x00..=0x7F`, nên điều kiện "ASCII" đã nằm trong phép thử nhóm).
//! - `digit` = `0-9`; `alphabetic` = `A-Z` + `a-z`; `alphanumeric` = hợp hai tập (62 scalar);
//!   chữ số/chữ Unicode (Arabic-Indic `١`, fullwidth `１`, `đ`, `Σ`, …) **không** thuộc nhóm.
//! - `whitespace` = **đúng** tập ASCII whitespace của Rust. Probe `rustc 1.100.0` (toolchain
//!   ghim) trên toàn bộ 128 scalar ASCII:
//!
//!   | scalar | ký tự | `is_ascii_whitespace` |
//!   |---|---|---|
//!   | `U+0009` | `\t` TAB | `true` |
//!   | `U+000A` | `\n` LF | `true` |
//!   | `U+000C` | `\x0C` FF | `true` |
//!   | `U+000D` | `\r` CR | `true` |
//!   | `U+0020` | space | `true` |
//!   | `U+000B` | `\x0B` VT | **`false`** |
//!
//!   Mọi scalar ASCII khác đều `false`; khoảng trắng Unicode (NBSP `U+00A0`, ideographic
//!   space `U+3000`, NEL, LS, PS, …) cũng `false` — khác `str.is_blank` (Unicode
//!   `White_Space`) và khác `char::is_whitespace`.
//! - Chuỗi **rỗng ⇒ `false`** cho cả bốn. Đây là điểm **cố ý khác** `str.is_ascii("")` ⇒
//!   `true`: `is_ascii` hỏi "mọi byte đều ASCII" (mệnh đề đúng với rỗng), còn bốn vị từ này
//!   hỏi "có ít nhất một scalar và **tất cả** thuộc nhóm" — rỗng không có scalar nào để
//!   thuộc nhóm nên `false`.
//! - NUL `\0` không thuộc nhóm nào ⇒ `false` cả bốn; trộn ASCII hợp lệ với **một** scalar sai
//!   ở bất kỳ vị trí nào ⇒ `false`; emoji/astral ⇒ `false`.
//! - Không trap, không cấp phát, O(số scalar), dừng ở scalar sai đầu tiên.

use crate::value::Value;

/// Nhóm ASCII cần kiểm — một biến thể cho mỗi hàm `std.str.is_ascii_*`.
#[derive(Clone, Copy)]
pub(super) enum Nhom {
    Digit,
    Alphabetic,
    Alphanumeric,
    Whitespace,
}

impl Nhom {
    /// `true` ⇔ scalar `c` thuộc nhóm này **và** là ASCII (các hàm `is_ascii_*` của Rust đã
    /// bao hàm điều kiện ASCII).
    fn thuoc(self, c: char) -> bool {
        match self {
            Nhom::Digit => c.is_ascii_digit(),
            Nhom::Alphabetic => c.is_ascii_alphabetic(),
            Nhom::Alphanumeric => c.is_ascii_alphanumeric(),
            Nhom::Whitespace => c.is_ascii_whitespace(),
        }
    }
}

/// `true` ⇔ `s` **không rỗng** và **mọi scalar** thuộc `nhom`.
pub(super) fn kiem(nhom: Nhom, s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| nhom.thuoc(c))
}

/// Dispatch một trong bốn vị từ; `nhom` do arm `stdlib::call` chọn theo tên hàm.
pub(super) fn goi(nhom: Nhom, s: &str) -> Value {
    Value::Bool(kiem(nhom, s))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Toàn bộ 128 scalar ASCII theo thứ tự.
    fn ascii_128() -> Vec<char> {
        (0x00u32..=0x7F).filter_map(char::from_u32).collect()
    }

    #[test]
    fn rong_false_ca_bon() {
        for nhom in [
            Nhom::Digit,
            Nhom::Alphabetic,
            Nhom::Alphanumeric,
            Nhom::Whitespace,
        ] {
            assert!(!kiem(nhom, ""), "rỗng phải false");
        }
        // Khác `str::is_ascii` của Rust: rỗng ⇒ true.
        assert!("".is_ascii());
    }

    #[test]
    fn digit_chi_0_den_9() {
        assert!(kiem(Nhom::Digit, "0"));
        assert!(kiem(Nhom::Digit, "9"));
        assert!(kiem(Nhom::Digit, "0123456789"));
        for s in [
            "a", "A", "12a", "a12", "1 2", " 12", "12 ", "-1", "1.0", "!", "_", "\0", "١", "１",
        ] {
            assert!(!kiem(Nhom::Digit, s), "digit phải false: {s:?}");
        }
    }

    #[test]
    fn alphabetic_chi_a_z_hoa_thuong() {
        assert!(kiem(Nhom::Alphabetic, "a"));
        assert!(kiem(Nhom::Alphabetic, "Z"));
        assert!(kiem(Nhom::Alphabetic, "abcXYZ"));
        for s in [
            "0", "aZ0", "0a", "a-b", "a b", "_", "\0", "đ", "Σ", "á", "١",
        ] {
            assert!(!kiem(Nhom::Alphabetic, s), "alphabetic phải false: {s:?}");
        }
    }

    #[test]
    fn alphanumeric_hop_hai_tap() {
        assert!(kiem(Nhom::Alphanumeric, "abc123"));
        assert!(kiem(Nhom::Alphanumeric, "A1z"));
        assert!(kiem(Nhom::Alphanumeric, "ABCxyz019"));
        for s in ["abc-123", "a_b", "a b", "abc!", "\0", "đ1", "١٢٣", "１２３"] {
            assert!(
                !kiem(Nhom::Alphanumeric, s),
                "alphanumeric phải false: {s:?}"
            );
        }
    }

    #[test]
    fn whitespace_dung_bang_probe_va_khong_co_vt() {
        // Bảng probe `rustc 1.100.0`: đúng 5 scalar ASCII whitespace.
        let dung = ['\u{0009}', '\u{000A}', '\u{000C}', '\u{000D}', '\u{0020}'];
        for c in dung {
            let s = c.to_string();
            assert!(kiem(Nhom::Whitespace, &s), "U+{:04X} phải true", c as u32);
        }
        assert!(kiem(Nhom::Whitespace, " \t\r\n\u{000C}"));
        // VT U+000B **không** phải ASCII whitespace của Rust.
        assert!(!kiem(Nhom::Whitespace, "\u{000B}"));
        assert!(!kiem(Nhom::Whitespace, "\u{000B} "));
        assert!(!kiem(Nhom::Whitespace, " \u{000B}"));
        // Khoảng trắng Unicode không thuộc tập ASCII.
        for s in [
            "\u{0085}", "\u{00A0}", "\u{1680}", "\u{2000}", "\u{2028}", "\u{2029}", "\u{202F}",
            "\u{205F}", "\u{3000}", "\u{FEFF}",
        ] {
            assert!(!kiem(Nhom::Whitespace, s), "phải false: {s:?}");
        }
        for s in ["a", " a", "a ", "a b", "\0"] {
            assert!(!kiem(Nhom::Whitespace, s), "phải false: {s:?}");
        }
    }

    #[test]
    fn tuong_duong_char_method_tren_toan_bo_ascii() {
        let tat_ca = ascii_128();
        assert_eq!(tat_ca.len(), 128);
        let mut dem = [0usize; 4];
        for c in tat_ca {
            let s = c.to_string();
            assert_eq!(kiem(Nhom::Digit, &s), c.is_ascii_digit(), "digit {c:?}");
            assert_eq!(
                kiem(Nhom::Alphabetic, &s),
                c.is_ascii_alphabetic(),
                "alphabetic {c:?}"
            );
            assert_eq!(
                kiem(Nhom::Alphanumeric, &s),
                c.is_ascii_alphanumeric(),
                "alphanumeric {c:?}"
            );
            assert_eq!(
                kiem(Nhom::Whitespace, &s),
                c.is_ascii_whitespace(),
                "whitespace {c:?}"
            );
            for (i, nhom) in [
                Nhom::Digit,
                Nhom::Alphabetic,
                Nhom::Alphanumeric,
                Nhom::Whitespace,
            ]
            .into_iter()
            .enumerate()
            {
                if kiem(nhom, &s) {
                    dem[i] += 1;
                }
            }
        }
        // Số scalar đúng của từng nhóm: 10 chữ số, 52 chữ cái, 62 chữ+số, 5 khoảng trắng.
        assert_eq!(dem, [10, 52, 62, 5]);
    }

    #[test]
    fn unicode_nbsp_emoji_va_nul_false() {
        for s in ["đ", "Σ", "á", "١", "１２３", "😀", "👨‍👩‍👧", "\u{10FFFF}"] {
            for nhom in [
                Nhom::Digit,
                Nhom::Alphabetic,
                Nhom::Alphanumeric,
                Nhom::Whitespace,
            ] {
                assert!(!kiem(nhom, s), "phải false: {s:?}");
            }
        }
        // NBSP/ideographic space là khoảng trắng Unicode nhưng **không** phải ASCII whitespace.
        for s in ["\u{00A0}", "\u{3000}"] {
            assert!(!kiem(Nhom::Whitespace, s), "phải false: {s:?}");
        }
        // NUL không thuộc nhóm nào.
        for nhom in [
            Nhom::Digit,
            Nhom::Alphabetic,
            Nhom::Alphanumeric,
            Nhom::Whitespace,
        ] {
            assert!(!kiem(nhom, "\0"), "NUL phải false");
            assert!(!kiem(nhom, "\0a"));
            assert!(!kiem(nhom, "a\0"));
        }
    }

    #[test]
    fn tron_ascii_hop_le_va_mot_scalar_sai() {
        for s in ["123a", "a123", "12 3"] {
            assert!(!kiem(Nhom::Digit, s), "digit phải false: {s:?}");
        }
        for s in ["abC1", "1abC", "abC-"] {
            assert!(!kiem(Nhom::Alphabetic, s), "alphabetic false: {s:?}");
        }
        for s in ["abc123!", "abc 123", "abc123_"] {
            assert!(!kiem(Nhom::Alphanumeric, s), "alphanumeric false: {s:?}");
        }
        for s in [" \t\nx", "x \t\n", " \t\n\u{000B}"] {
            assert!(!kiem(Nhom::Whitespace, s), "whitespace false: {s:?}");
        }
    }
}
