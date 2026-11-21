// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (split-terminator)** — `std.str.split_terminator(s, sep) -> list[str]`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM), **đúng** `str::split_terminator` của Rust:
//! - Như `str.split` nhưng **không** sinh phần rỗng cuối khi `s` kết thúc bằng `sep`:
//!   `split_terminator("a,b,c", ",") == ["a", "b", "c"]`;
//!   `split_terminator("a,b,", ",") == ["a", "b"]` (khác `str.split`: `["a", "b", ""]`).
//! - Chỉ bỏ **một** phần rỗng cuối: `split_terminator(",", ",") == [""]` (len 1),
//!   `split_terminator(",,", ",") == ["", ""]` (len 2),
//!   `split_terminator("a,b,,", ",") == ["a", "b", ""]` (len 3).
//! - Dấu ở **đầu** vẫn sinh phần rỗng: `split_terminator(",a", ",") == ["", "a"]`.
//! - Không khớp (hoặc `sep` dài hơn `s`) ⇒ `[s]`; `"abc"` cắt `"abc"` ⇒ `[""]` (cả chuỗi là
//!   dấu phân cách, phần rỗng cuối bị bỏ nên còn đúng một phần tử rỗng).
//! - **`s` rỗng + `sep` khác rỗng ⇒ `[]`** — probe `rustc 1.100.0` xác nhận
//!   `"".split_terminator(",")` trả iterator **rỗng** (khác `str.split` của VS trả `[""]`,
//!   và cũng khác `"".split(",")` của Rust trả `[""]`). Đây là khác biệt **có chủ ý** ngoài
//!   quy tắc "bỏ phần rỗng cuối"; hợp đồng bám probe.
//! - `sep` rỗng theo **đúng nghĩa Rust**: khớp rỗng tại **mọi biên scalar** kể cả đầu, và
//!   **bỏ** phần rỗng cuối nên `split_terminator("abc", "") == ["", "a", "b", "c"]`,
//!   `split_terminator("a", "") == ["", "a"]`, `split_terminator("đ😀", "") ==
//!   ["", "đ", "😀"]`, `split_terminator("", "") == [""]` (len 1). Khác `str.split` của VS
//!   (rỗng ⇒ tách scalar, **không** sinh phần rỗng đầu).
//! - Chồng lấn **không** được khớp lại: `split_terminator("aaa", "aa") == ["", "a"]`,
//!   `split_terminator("abab", "aba") == ["", "b"]`, `split_terminator("banana", "ana") ==
//!   ["b", "na"]`.
//! - Đơn vị khớp là **chuỗi con UTF-8** trên **biên scalar** (cùng `str.split`/`str.find`),
//!   **không bao giờ** cắt giữa scalar: `split_terminator("đ😀x😀", "😀") == ["đ", "x"]`;
//!   ZWJ trong emoji không phải biên đặc biệt: `split_terminator("👨‍👩‍👧x", "👩") ==
//!   ["👨‍", "‍👧x"]`, phần còn lại vẫn là UTF-8 hợp lệ.
//!
//! Không trap, không có trần riêng: số phần tử bị chặn bởi độ dài `s` + 1 (`sep` rỗng) nên
//! cấp phát tuyến tính theo `s`; mỗi phần tử được cấp phát đúng một chuỗi kết quả.

use vis_syntax::Span;

use crate::interp::R;
use crate::value::Value;

/// Tách `s` theo `sep`, **bỏ** phần rỗng cuối khi `s` kết thúc bằng `sep`.
/// Dùng **đúng** `str::split_terminator` của Rust nên `sep` rỗng và `s` rỗng cũng theo nghĩa
/// Rust (xem hợp đồng ở đầu tệp).
pub(super) fn tach(s: &str, sep: &str) -> Vec<String> {
    s.split_terminator(sep).map(str::to_string).collect()
}

/// Dispatch `str.split_terminator`: luôn `list[str]`; không trap.
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

    /// `str::split` của Rust — mốc so sánh trực tiếp với `split_terminator`.
    fn split(s: &str, sep: &str) -> Vec<String> {
        s.split(sep).map(str::to_string).collect()
    }

    #[test]
    fn bang_probe_rust_bo_dung_phan_rong_cuoi() {
        assert_eq!(tach("a,b,c", ","), ds(&["a", "b", "c"]));
        assert_eq!(tach("a,b,", ","), ds(&["a", "b"]));
        assert_eq!(tach(",a", ","), ds(&["", "a"]));
        assert_eq!(tach(",", ","), ds(&[""]));
        assert_eq!(tach(",,", ","), ds(&["", ""]));
        assert_eq!(tach("", ","), ds(&[]));
        assert_eq!(tach("a", ","), ds(&["a"]));
        assert_eq!(tach("a,,b", ","), ds(&["a", "", "b"]));
        assert_eq!(tach("a,b,,", ","), ds(&["a", "b", ""]));
        assert_eq!(tach("a--b--", "--"), ds(&["a", "b"]));
        assert_eq!(tach("abc", "abc"), ds(&[""]));
        assert_eq!(tach("aa", "aa"), ds(&[""]));
    }

    #[test]
    fn chi_khac_split_o_phan_rong_cuoi() {
        // Không kết thúc bằng `sep` (và `s` khác rỗng) ⇒ **giống hệt** `str::split`.
        for (s, sep) in [
            ("a,b,c", ","),
            (",a", ","),
            ("a,,b", ","),
            ("aaa", "aa"),
            ("abab", "aba"),
            ("banana", "ana"),
            ("a,b,c", "x"),
            ("abc", "abcd"),
            ("a", ","),
        ] {
            assert_eq!(
                tach(s, sep),
                split(s, sep),
                "({s:?}, {sep:?}) phải giống split"
            );
        }
        // Kết thúc bằng `sep` ⇒ đúng `split` bỏ **một** phần tử rỗng cuối.
        for (s, sep) in [
            ("a,b,", ","),
            (",", ","),
            (",,", ","),
            ("a,b,,", ","),
            ("a--b--", "--"),
            ("abc", "abc"),
            ("aa", "aa"),
            ("đ😀x😀", "😀"),
        ] {
            let mut mong = split(s, sep);
            assert_eq!(mong.pop(), Some(String::new()), "({s:?}, {sep:?})");
            assert_eq!(tach(s, sep), mong, "({s:?}, {sep:?})");
        }
        // `s` rỗng + `sep` khác rỗng: split ⇒ `[""]`, split_terminator ⇒ `[]` (probe Rust).
        assert_eq!(split("", ","), ds(&[""]));
        assert_eq!(tach("", ","), ds(&[]));
    }

    #[test]
    fn khong_khop_tra_nguyen_s() {
        assert_eq!(tach("abc", ","), ds(&["abc"]));
        assert_eq!(tach("a,b,c", "x"), ds(&["a,b,c"]));
        assert_eq!(tach("abc", "abcd"), ds(&["abc"]), "sep dài hơn s");
        assert_eq!(tach("ab", "abc"), ds(&["ab"]));
        assert_eq!(tach("abc", "ac"), ds(&["abc"]));
    }

    #[test]
    fn chong_lan_khong_khop_lai() {
        assert_eq!(tach("aaa", "aa"), ds(&["", "a"]));
        assert_eq!(tach("aaaa", "aa"), ds(&["", ""]));
        assert_eq!(tach("abab", "aba"), ds(&["", "b"]));
        assert_eq!(tach("banana", "ana"), ds(&["b", "na"]));
        assert_eq!(tach("aaaaa", "aa"), ds(&["", "", "a"]));
    }

    #[test]
    fn sep_rong_theo_nghia_rust() {
        assert_eq!(tach("abc", ""), ds(&["", "a", "b", "c"]));
        assert_eq!(tach("a", ""), ds(&["", "a"]));
        assert_eq!(tach("", ""), ds(&[""]));
        assert_eq!(tach("đ😀", ""), ds(&["", "đ", "😀"]));
        assert_eq!(tach("a\r\nb", ""), ds(&["", "a", "\r", "\n", "b"]));
        // So với `str::split` của Rust: luôn bỏ đúng phần rỗng cuối.
        for s in ["", "a", "abc", "đ😀"] {
            let mut mong = split(s, "");
            assert_eq!(mong.pop(), Some(String::new()), "{s:?}");
            assert_eq!(tach(s, ""), mong, "{s:?}");
        }
    }

    #[test]
    fn s_rong_theo_nghia_rust() {
        // Probe `rustc 1.100.0`: `"".split_terminator(sep)` với `sep` khác rỗng là **rỗng**
        // (khác `str.split` của VS trả `[""]`); đây là hợp đồng **có chủ ý**.
        assert_eq!(tach("", ","), ds(&[]));
        assert_eq!(tach("", "abc"), ds(&[]));
        assert_eq!(tach("", ""), ds(&[""]));
    }

    #[test]
    fn unicode_emoji_zwj_tren_bien_scalar() {
        assert_eq!(tach("đ,😀,x", ","), ds(&["đ", "😀", "x"]));
        assert_eq!(tach("a😀b😀c", "😀"), ds(&["a", "b", "c"]));
        assert_eq!(tach("a🌍b🌍", "🌍"), ds(&["a", "b"]));
        assert_eq!(tach("héllo,🌍,👨‍👩‍👧‍👦", ","), ds(&["héllo", "🌍", "👨‍👩‍👧‍👦"]));
        // ZWJ **không** phải biên đặc biệt: cắt đúng quanh scalar `👩`, phần sau vẫn hợp lệ.
        assert_eq!(tach("👨‍👩‍👧x", "👩"), ds(&["👨‍", "‍👧x"]));
        assert_eq!(tach("👨‍👩‍👧‍👦x", "x"), ds(&["👨‍👩‍👧‍👦"]), "không khớp ⇒ nguyên chuỗi");
        assert_eq!(tach("đ😀x", "😀a"), ds(&["đ😀x"]));
    }

    #[test]
    fn dau_phan_cach_dieu_khien_va_nhieu_byte() {
        assert_eq!(tach("a\r\nb\r\n", "\r\n"), ds(&["a", "b"]));
        assert_eq!(tach("a\0b\0", "\0"), ds(&["a", "b"]));
        assert_eq!(tach("a\tb\t", "\t"), ds(&["a", "b"]));
        assert_eq!(tach("a b ", " "), ds(&["a", "b"]));
    }
}
