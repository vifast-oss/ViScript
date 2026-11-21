// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (matches)** — `std.str.matches(s, pat) -> list[str]`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM), **đúng** `str::matches` của Rust:
//! - Trả **các chuỗi con** khớp `pat`, theo thứ tự **trái → phải**, khớp **không chồng** (sau
//!   mỗi lần khớp, vị trí tìm tiếp nằm **sau** phần đã khớp): `matches("banana", "ana")` ⇒
//!   `["ana"]`; `matches("aaaa", "aa")` ⇒ `["aa", "aa"]`; `matches("aaa", "aa")` ⇒ `["aa"]`.
//! - Không khớp/`pat` dài hơn `s`/`s` rỗng + `pat` khác rỗng ⇒ `[]` (khác `str.split` trả
//!   `[s]`).
//! - `pat` rỗng theo **đúng nghĩa Rust**: khớp rỗng tại **mọi biên scalar** kể cả đầu/cuối ⇒
//!   `matches("abc", "")` ⇒ `["", "", "", ""]` (`len` = 4), `matches("", "")` ⇒ `[""]`
//!   (`len` = 1), `matches("đ😀x😀", "")` ⇒ 5 phần tử rỗng.
//! - Đơn vị khớp là **chuỗi con UTF-8** trên **biên scalar** (cùng `str.find`/`str.split`),
//!   **không bao giờ** cắt giữa scalar: `matches("đ😀x😀", "😀")` ⇒ `["😀", "😀"]`. ZWJ trong
//!   emoji không phải biên đặc biệt: `matches("👨‍👩‍👧x", "👩")` ⇒ `["👩"]`; cả cụm ZWJ vẫn
//!   khớp nguyên vẹn.
//! - Không trap, không có trần riêng: mỗi lần khớp cấp phát **đúng một** chuỗi kết quả; số
//!   phần tử bị chặn bởi `len_scalar(s) + 1` khi `pat` rỗng.
//!
//! Bảng probe `rustc 1.100.0` (toolchain ghim; chạy lại trong unit test bên dưới):
//!
//! | biểu thức | kết quả |
//! |---|---|
//! | `"banana".matches("ana")` | `["ana"]` |
//! | `"aaaa".matches("aa")` | `["aa", "aa"]` |
//! | `"abab".matches("aba")` | `["aba"]` |
//! | `"abc".matches("abcd")` | `[]` |
//! | `"abc".matches("x")` | `[]` |
//! | `"".matches("")` | `[""]` |
//! | `"abc".matches("")` | `["", "", "", ""]` |
//! | `"đ😀x😀".matches("😀")` | `["😀", "😀"]` |
//! | `"👨‍👩‍👧‍👦x👨‍👩‍👧‍👦".matches("👨‍👩‍👧‍👦")` | `["👨‍👩‍👧‍👦", "👨‍👩‍👧‍👦"]` |
//! | `"👨‍👩‍👧x".matches("👩")` | `["👩"]` |
//! | `"đ😀x😀".matches("")` | `["", "", "", "", ""]` |
//! | `"aaa".matches("aa")` | `["aa"]` |
//! | `"ababa".matches("aba")` | `["aba"]` |
//! | `"abcabc".matches("abc")` | `["abc", "abc"]` |
//! | `"😀😀😀".matches("😀😀")` | `["😀😀"]` |

use vis_syntax::Span;

use crate::interp::R;
use crate::value::Value;

/// Các chuỗi con khớp `pat` theo **đúng** `str::matches` của Rust (không chồng, trái → phải);
/// `pat` rỗng khớp tại mọi biên scalar.
pub(super) fn khop(s: &str, pat: &str) -> Vec<String> {
    s.matches(pat).map(str::to_string).collect()
}

/// Dispatch `str.matches`: luôn `list[str]`; không trap.
pub(super) fn goi(name: &str, args: &[Value], span: Span) -> R<Value> {
    let s = super::chuoi(name, &args[0], 0, span)?;
    let pat = super::chuoi(name, &args[1], 1, span)?;
    Ok(Value::List(
        khop(s, pat).into_iter().map(Value::Str).collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ds(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn bang_probe_rust() {
        assert_eq!(khop("banana", "ana"), ds(&["ana"]));
        assert_eq!(khop("aaaa", "aa"), ds(&["aa", "aa"]));
        assert_eq!(khop("abab", "aba"), ds(&["aba"]));
        assert_eq!(khop("abc", "abcd"), ds(&[]));
        assert_eq!(khop("abc", "x"), ds(&[]));
        assert_eq!(khop("", ""), ds(&[""]));
        assert_eq!(khop("abc", ""), ds(&["", "", "", ""]));
        assert_eq!(khop("đ😀x😀", "😀"), ds(&["😀", "😀"]));
        assert_eq!(khop("👨‍👩‍👧‍👦x👨‍👩‍👧‍👦", "👨‍👩‍👧‍👦"), ds(&["👨‍👩‍👧‍👦", "👨‍👩‍👧‍👦"]));
        assert_eq!(khop("👨‍👩‍👧x", "👩"), ds(&["👩"]));
        assert_eq!(khop("đ😀x😀", ""), ds(&["", "", "", "", ""]));
        assert_eq!(khop("aaa", "aa"), ds(&["aa"]));
        assert_eq!(khop("ababa", "aba"), ds(&["aba"]));
        assert_eq!(khop("abcabc", "abc"), ds(&["abc", "abc"]));
        assert_eq!(khop("😀😀😀", "😀😀"), ds(&["😀😀"]));
    }

    #[test]
    fn khong_chong_va_khong_quay_lai() {
        assert_eq!(khop("aaaaa", "aa"), ds(&["aa", "aa"]));
        assert_eq!(khop("banana", "ana"), ds(&["ana"]));
        assert_eq!(khop("banana", "na"), ds(&["na", "na"]));
        assert_eq!(khop("aaaa", "aaa"), ds(&["aaa"]));
        assert_eq!(khop("ababab", "ab"), ds(&["ab", "ab", "ab"]));
    }

    #[test]
    fn khong_khop_va_s_rong() {
        assert_eq!(khop("abc", "z"), ds(&[]));
        assert_eq!(khop("abc", "abcd"), ds(&[]));
        assert_eq!(khop("", "a"), ds(&[]));
        assert_eq!(khop("", "abc"), ds(&[]));
        // `pat` rỗng là ca duy nhất `s` rỗng vẫn khớp (một phần tử rỗng).
        assert_eq!(khop("", ""), ds(&[""]));
    }

    #[test]
    fn pat_rong_theo_nghia_rust() {
        assert_eq!(khop("abc", ""), ds(&["", "", "", ""]));
        assert_eq!(khop("a", ""), ds(&["", ""]));
        assert_eq!(khop("", ""), ds(&[""]));
        assert_eq!(khop("đ😀", ""), ds(&["", "", ""]));
        assert_eq!(khop("a\r\nb", ""), ds(&["", "", "", "", ""]));
    }

    #[test]
    fn unicode_emoji_zwj_tren_bien_scalar() {
        assert_eq!(khop("đ😀x😀", "😀"), ds(&["😀", "😀"]));
        assert_eq!(khop("a😀b😀c", "😀"), ds(&["😀", "😀"]));
        assert_eq!(khop("héllo,🌍,👨‍👩‍👧‍👦", ","), ds(&[",", ","]));
        // ZWJ không phải biên đặc biệt; phần khớp vẫn là UTF-8 hợp lệ.
        assert_eq!(khop("👨‍👩‍👧x", "👩"), ds(&["👩"]));
        assert_eq!(khop("👨‍👩‍👧x", "👨‍"), ds(&["👨‍"]));
        assert_eq!(khop("👨‍👩‍👧‍👦x👨‍👩‍👧‍👦", "👨‍👩‍👧‍👦"), ds(&["👨‍👩‍👧‍👦", "👨‍👩‍👧‍👦"]));
        assert_eq!(khop("😀😀😀", "😀😀"), ds(&["😀😀"]));
    }

    #[test]
    fn khong_cat_giua_scalar() {
        // `pat` luôn là chuỗi UTF-8 hợp lệ nên mọi lần khớp bắt đầu/kết thúc trên biên scalar;
        // dấu kết hợp là scalar **riêng** (không phải grapheme) nên khớp được một mình.
        assert_eq!(khop("e\u{301}", "\u{301}"), ds(&["\u{301}"]));
        assert_eq!(khop("đ", "đ"), ds(&["đ"]));
        assert_eq!(khop("😀", "\u{1F600}"), ds(&["😀"]));
        assert_eq!(khop("xđy", "đ"), ds(&["đ"]));
        assert_eq!(khop("đ", "x"), ds(&[]));
    }

    #[test]
    fn goi_tra_list_that() {
        let v = goi(
            "str.matches",
            &[Value::Str("abcabc".into()), Value::Str("abc".into())],
            Span::new(0, 0),
        )
        .expect("không trap");
        match v {
            Value::List(xs) => {
                assert_eq!(xs.len(), 2);
                assert!(matches!(&xs[0], Value::Str(s) if s == "abc"));
                assert!(matches!(&xs[1], Value::Str(s) if s == "abc"));
            }
            other => panic!("phải là list, được {}", other.show()),
        }
    }
}
