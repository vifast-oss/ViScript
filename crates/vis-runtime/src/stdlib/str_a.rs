// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — gom các module `stdlib` miền `str_a` (giữ nguyên nội dung/đường dẫn).

pub(crate) mod strascii {
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

    use vis_core::value::Value;

    /// `true` ⇔ **mọi byte** của `s` nằm trong `0x00..=0x7F` (`str::is_ascii` của Rust).
    pub(crate) fn is_ascii(s: &str) -> bool {
        s.is_ascii()
    }

    /// Dispatch `str.is_ascii`; chỉ một hàm nên không cần `name`.
    pub(crate) fn goi(s: &str) -> Value {
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
}

pub(crate) mod strasciipreds {
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

    use vis_core::value::Value;

    /// Nhóm ASCII cần kiểm — một biến thể cho mỗi hàm `std.str.is_ascii_*`.
    #[derive(Clone, Copy)]
    pub(crate) enum Nhom {
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
    pub(crate) fn kiem(nhom: Nhom, s: &str) -> bool {
        !s.is_empty() && s.chars().all(|c| nhom.thuoc(c))
    }

    /// Dispatch một trong bốn vị từ; `nhom` do arm `stdlib::call` chọn theo tên hàm.
    pub(crate) fn goi(nhom: Nhom, s: &str) -> Value {
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
            for s in ["đ", "Σ", "á", "١", "１２３", "😀", "👨‍👩‍👧", "\u{10FFFF}"]
            {
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
}

pub(crate) mod strcharat {
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

    use vis_core::trap::R;
    use vis_core::value::Value;

    /// Scalar Unicode tại chỉ số `i` (zero-based); âm hoặc vượt quá `char_count` ⇒ `None`.
    /// `i` là `i64` để giữ đúng miền âm/dương trước khi chuyển `usize`.
    pub(crate) fn char_at(s: &str, i: i64) -> Option<char> {
        let i = usize::try_from(i).ok()?;
        s.chars().nth(i)
    }

    /// Dispatch chung: kiểm kiểu lúc chạy (sai ⇒ `VS0501`) rồi trả `str?`.
    pub(crate) fn goi(name: &str, args: &[Value], span: Span) -> R<Value> {
        let s = super::super::chuoi(name, &args[0], 0, span)?;
        let i = super::super::so(name, &args[1], 1, span)?;
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
}

pub(crate) mod strfind {
    //! **VD-270 (strfind)** — tìm chuỗi con: `std.str.find` / `rfind` / `count`.
    //!
    //! Hợp đồng (một hiện thực dùng chung interp + VM):
    //! - Chỉ số là **offset scalar Unicode** (cùng đơn vị `chars()`), **không** phải byte:
    //!   `find("đ😀x", "x") == Some(2)`.
    //! - `find` trả vị trí **đầu tiên**, `rfind` vị trí **cuối cùng**; không khớp ⇒ `None`.
    //! - `count` đếm số lần xuất hiện **không chồng** (như Rust `matches`):
    //!   `count("aaa", "aa") == 1`, `count("aaaa", "aa") == 2`.
    //! - `needle` rỗng theo nghĩa Rust: `find("") == Some(0)`, `rfind("") == Some(len_scalar)`,
    //!   `count("") == len_scalar + 1` (`"".count("") == 1`).
    //!
    //! Không có trần riêng: mọi phép đều tuyến tính theo độ dài chuỗi, không cấp phát kết quả
    //! theo số lần khớp.

    use vis_core::value::Value;

    /// Vị trí **scalar** của lần xuất hiện **đầu tiên** của `needle`; `None` nếu không có.
    /// `needle` rỗng ⇒ `Some(0)` (Rust `str::find`).
    pub(crate) fn find(s: &str, needle: &str) -> Option<i64> {
        s.find(needle).map(|b| s[..b].chars().count() as i64)
    }

    /// Vị trí **scalar** của lần xuất hiện **cuối cùng** của `needle`; `None` nếu không có.
    /// `needle` rỗng ⇒ `Some(len_scalar)` (Rust `str::rfind`).
    pub(crate) fn rfind(s: &str, needle: &str) -> Option<i64> {
        if needle.is_empty() {
            return Some(s.chars().count() as i64);
        }
        s.rfind(needle).map(|b| s[..b].chars().count() as i64)
    }

    /// Số lần xuất hiện **không chồng** của `needle` (Rust `str::matches`); `needle` rỗng ⇒
    /// `len_scalar + 1` (khớp tại mọi biên scalar, kể cả cuối chuỗi).
    pub(crate) fn count(s: &str, needle: &str) -> i64 {
        s.matches(needle).count() as i64
    }

    /// Dispatch chung cho ba hàm; `name` là tên đầy đủ `"str.find"`/`"str.rfind"`/`"str.count"`.
    pub(crate) fn goi(name: &str, s: &str, needle: &str) -> Value {
        match name {
            "str.find" => Value::opt(find(s, needle).map(Value::Int)),
            "str.rfind" => Value::opt(rfind(s, needle).map(Value::Int)),
            _ => Value::Int(count(s, needle)),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn offset_la_scalar_khong_phai_byte() {
            assert_eq!(find("đ😀x", "x"), Some(2));
            assert_eq!(rfind("đ😀x😀", "😀"), Some(3));
            assert_eq!(find("aaa", "aa"), Some(0));
            assert_eq!(rfind("aaa", "aa"), Some(1));
        }

        #[test]
        fn khong_khop_va_needle_rong() {
            assert_eq!(find("abc", "z"), None);
            assert_eq!(rfind("abc", "z"), None);
            assert_eq!(find("", ""), Some(0));
            assert_eq!(rfind("", ""), Some(0));
            assert_eq!(find("đ😀", ""), Some(0));
            assert_eq!(rfind("đ😀", ""), Some(2));
            assert_eq!(count("đ😀", ""), 3);
        }

        #[test]
        fn count_khong_chong() {
            assert_eq!(count("aaa", "aa"), 1);
            assert_eq!(count("aaaa", "aa"), 2);
            assert_eq!(count("ababab", "ab"), 3);
            assert_eq!(count("banana", "na"), 2);
            assert_eq!(count("banana", "x"), 0);
        }
    }
}

pub(crate) mod strisblank {
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

    use vis_core::value::Value;

    /// `true` ⇔ `s` rỗng hoặc **mọi scalar** là khoảng trắng Unicode.
    pub(crate) fn is_blank(s: &str) -> bool {
        s.chars().all(char::is_whitespace)
    }

    /// Dispatch `str.is_blank`; chỉ một hàm nên không cần `name`.
    pub(crate) fn goi(s: &str) -> Value {
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
}
