// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (split-inclusive)** — `std.str.split_inclusive(s, sep) -> list[str]`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM), **đúng** `str::split_inclusive` của Rust:
//! - Tách **trái → phải** tại mỗi lần xuất hiện **không chồng** của `sep`; **mỗi phần tử**
//!   **giữ luôn dấu phân cách** đã tách (trừ phần cuối khi `s` không kết thúc bằng `sep`):
//!   `split_inclusive("a,b,c", ",") == ["a,", "b,", "c"]`.
//! - Không khớp (hoặc `sep` dài hơn `s`) ⇒ `[s]`, **không** rỗng.
//! - Dấu phân cách ở **cuối** ⇒ phần tử cuối **kết thúc bằng** `sep`, **không** sinh phần rỗng
//!   thêm: `split_inclusive("a,b,", ",") == ["a,", "b,"]`.
//! - Dấu phân cách **liên tiếp** ⇒ mỗi dấu là một phần tử riêng (phần tử có thể chỉ là `sep`):
//!   `split_inclusive("a,b,,c", ",") == ["a,", "b,", ",", "c"]`.
//! - Chồng lấn **không** được khớp lại: `split_inclusive("aaa", "aa") == ["aa", "a"]`,
//!   `split_inclusive("abab", "aba") == ["aba", "b"]`.
//! - `sep` rỗng theo **đúng nghĩa Rust**: khớp rỗng tại **mọi biên scalar** kể cả đầu/cuối,
//!   nên `split_inclusive("abc", "") == ["", "a", "b", "c"]` (**không** có phần rỗng cuối);
//!   `split_inclusive("", "") == [""]`; `split_inclusive("a", "") == ["", "a"]`. Khác
//!   `str.split` của VS (rỗng ⇒ tách scalar, **không** sinh phần rỗng đầu).
//! - `s` rỗng + `sep` **khác** rỗng ⇒ **`[]`** — probe `rustc 1.100.0` xác nhận
//!   `"".split_inclusive(",")` trả iterator **rỗng** (KHÔNG phải `[""]`); giữ đúng nghĩa Rust.
//! - Đơn vị khớp là **chuỗi con UTF-8** trên **biên scalar** (cùng `str.split`/`str.find`),
//!   **không bao giờ** cắt giữa scalar: `split_inclusive("đ😀x😀", "😀") == ["đ😀", "x😀"]`;
//!   ZWJ trong emoji không phải biên đặc biệt: `split_inclusive("👨‍👩‍👧x", "👩") ==
//!   ["👨‍👩", "‍👧x"]`, phần còn lại vẫn là UTF-8 hợp lệ.
//!
//! Không trap, không có trần riêng: số phần tử bị chặn bởi độ dài `s` + 1 (`sep` rỗng) nên
//! cấp phát tuyến tính theo `s`; mỗi phần tử được cấp phát đúng một chuỗi kết quả.

use vis_syntax::Span;

use crate::interp::R;
use crate::value::Value;

/// Tách `s` theo `sep`, **giữ** dấu phân cách ở cuối mỗi phần (trừ phần cuối nếu không có dấu).
/// Dùng **đúng** `str::split_inclusive` của Rust nên `sep` rỗng và `s` rỗng cũng theo nghĩa Rust.
pub(super) fn tach(s: &str, sep: &str) -> Vec<String> {
    s.split_inclusive(sep).map(str::to_string).collect()
}

/// Dispatch `str.split_inclusive`: luôn `list[str]`; không trap.
pub(super) fn goi(name: &str, args: &[Value], span: Span) -> R<Value> {
    let s = super::chuoi(name, &args[0], 0, span)?;
    let sep = super::chuoi(name, &args[1], 1, span)?;
    Ok(Value::List(
        tach(s, sep).into_iter().map(Value::Str).collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ds(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn giu_dau_phan_cach_va_khong_sinh_phan_rong_cuoi() {
        assert_eq!(tach("a,b,c", ","), ds(&["a,", "b,", "c"]));
        assert_eq!(tach("aXbXc", "X"), ds(&["aX", "bX", "c"]));
        assert_eq!(tach("a--b--", "--"), ds(&["a--", "b--"]));
        // Dấu cuối **không** sinh phần rỗng thêm.
        assert_eq!(tach("a,b,", ","), ds(&["a,", "b,"]));
        assert_eq!(tach(",", ","), ds(&[","]));
        assert_eq!(tach(",,", ","), ds(&[",", ","]));
        assert_eq!(tach(",a", ","), ds(&[",", "a"]));
        assert_eq!(
            tach("a,b,,c", ","),
            ds(&["a,", "b,", ",", "c"]),
            "dấu liên tiếp là phần tử riêng"
        );
    }

    #[test]
    fn khong_khop_tra_nguyen_s() {
        assert_eq!(tach("abc", ","), ds(&["abc"]));
        assert_eq!(tach("a,b,c", "x"), ds(&["a,b,c"]));
        assert_eq!(tach("abc", "abcd"), ds(&["abc"]), "sep dài hơn s");
        assert_eq!(tach("ab", "abc"), ds(&["ab"]));
        assert_eq!(tach("abc", "abc"), ds(&["abc"]));
        assert_eq!(tach("abc", "ac"), ds(&["abc"]));
    }

    #[test]
    fn chong_lan_khong_khop_lai() {
        assert_eq!(tach("aaa", "aa"), ds(&["aa", "a"]));
        assert_eq!(tach("aaaa", "aa"), ds(&["aa", "aa"]));
        assert_eq!(tach("abab", "aba"), ds(&["aba", "b"]));
        assert_eq!(tach("banana", "ana"), ds(&["bana", "na"]));
        assert_eq!(tach("aaaaa", "aa"), ds(&["aa", "aa", "a"]));
    }

    #[test]
    fn sep_rong_theo_nghia_rust() {
        assert_eq!(tach("abc", ""), ds(&["", "a", "b", "c"]));
        assert_eq!(tach("a", ""), ds(&["", "a"]));
        assert_eq!(tach("", ""), ds(&[""]));
        assert_eq!(tach("đ😀", ""), ds(&["", "đ", "😀"]));
        assert_eq!(tach("a\r\nb", ""), ds(&["", "a", "\r", "\n", "b"]));
    }

    #[test]
    fn s_rong_theo_nghia_rust() {
        // Probe `rustc 1.100.0`: `"".split_inclusive(sep)` với `sep` khác rỗng là **rỗng**
        // (khác `str.split` của VS trả `[""]`); đây là hợp đồng **có chủ ý**.
        assert_eq!(tach("", ","), ds(&[]));
        assert_eq!(tach("", "abc"), ds(&[]));
        assert_eq!(tach("", ""), ds(&[""]));
    }

    #[test]
    fn unicode_emoji_zwj_tren_bien_scalar() {
        assert_eq!(tach("đ,😀,x", ","), ds(&["đ,", "😀,", "x"]));
        assert_eq!(tach("a😀b😀c", "😀"), ds(&["a😀", "b😀", "c"]));
        assert_eq!(tach("a🌍b🌍", "🌍"), ds(&["a🌍", "b🌍"]));
        assert_eq!(tach("héllo,🌍,👨‍👩‍👧‍👦", ","), ds(&["héllo,", "🌍,", "👨‍👩‍👧‍👦"]));
        // ZWJ **không** phải biên đặc biệt: cắt đúng quanh scalar `👩`, phần sau vẫn hợp lệ.
        assert_eq!(tach("👨‍👩‍👧x", "👩"), ds(&["👨‍👩", "‍👧x"]));
        assert_eq!(tach("👨‍👩‍👧‍👦x", "x"), ds(&["👨‍👩‍👧‍👦x"]), "không khớp ⇒ nguyên chuỗi");
        assert_eq!(tach("đ😀x", "😀a"), ds(&["đ😀x"]));
    }

    #[test]
    fn dau_phan_cach_dieu_khien_va_nhieu_byte() {
        assert_eq!(tach("a\r\nb\r\n", "\r\n"), ds(&["a\r\n", "b\r\n"]));
        assert_eq!(tach("a\0b\0", "\0"), ds(&["a\0", "b\0"]));
        assert_eq!(tach("a\tb\t", "\t"), ds(&["a\t", "b\t"]));
        assert_eq!(tach("a b ", " "), ds(&["a ", "b "]));
    }
}
