// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (rsplit-once)** — `std.str.rsplit_once(s, sep) -> list[str]?`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//! - Tách tại **lần xuất hiện cuối cùng** của `sep`, **không** tách các lần trước:
//!   `rsplit_once("a,b,c", ",") == Some(["a,b", "c"])`, `rsplit_once("aXbXc", "X") ==
//!   Some(["aXb", "c"])`.
//! - Không có `sep` trong `s` (hoặc `sep` dài hơn `s`) ⇒ `None`.
//! - `sep` rỗng ⇒ `Some([s, ""])` — **biên cuối** của `s` (khác `split_once`: `Some(["", s])`);
//!   `s` rỗng + `sep` rỗng ⇒ `Some(["", ""])`; `s` rỗng + `sep` khác rỗng ⇒ `None`. Đúng nghĩa
//!   Rust `str::rsplit_once` (probe xác nhận; **không** trap).
//! - Dấu phân cách ở **đầu** ⇒ phần đầu rỗng: `rsplit_once("abc", "a") == Some(["", "bc"])`;
//!   ở **cuối** ⇒ phần sau rỗng: `rsplit_once("abc", "c") == Some(["ab", ""])`.
//! - Đơn vị khớp là **chuỗi con UTF-8** trên **biên scalar** (cùng `str.split`/`str.find`),
//!   **không bao giờ** cắt giữa scalar: `rsplit_once("đ😀x", "😀") == Some(["đ", "x"])`;
//!   ZWJ trong emoji không phải biên đặc biệt: `rsplit_once("👨‍👩‍👧x", "👩")` cắt đúng
//!   quanh `👩` **cuối** và phần còn lại vẫn là UTF-8 hợp lệ.
//! - `Some` luôn có **đúng 2** phần tử; chỉ cấp phát **hai** chuỗi kết quả (không cắt thêm,
//!   không cấp phát theo độ dài `s`).

use vis_syntax::Span;

use crate::value::Value;

/// Tách `s` tại lần khớp **cuối** của `sep`; `None` nếu không khớp. Dùng **đúng**
/// `str::rsplit_once` của Rust nên `sep` rỗng cũng theo nghĩa Rust.
pub(super) fn tach<'a>(s: &'a str, sep: &str) -> Option<(&'a str, &'a str)> {
    s.rsplit_once(sep)
}

/// Dispatch `str.rsplit_once`: `Some(list)` với **đúng 2** phần, hoặc `None`; không trap.
pub(super) fn goi(name: &str, args: &[Value], span: Span) -> crate::interp::R<Value> {
    let s = super::chuoi(name, &args[0], 0, span)?;
    let sep = super::chuoi(name, &args[1], 1, span)?;
    Ok(Value::opt(tach(s, sep).map(|(dau, sau)| {
        Value::List(vec![
            Value::Str(dau.to_string()),
            Value::Str(sau.to_string()),
        ])
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hai(a: &str, b: &str) -> Option<(String, String)> {
        tach(a, b).map(|(x, y)| (x.to_string(), y.to_string()))
    }

    #[test]
    fn chi_tach_lan_cuoi() {
        assert_eq!(hai("a,b,c", ","), Some(("a,b".into(), "c".into())));
        assert_eq!(hai("a::b::c", "::"), Some(("a::b".into(), "c".into())));
        assert_eq!(hai("aXbXc", "X"), Some(("aXb".into(), "c".into())));
        assert_eq!(hai("aaaa", "aa"), Some(("aa".into(), "".into())));
        assert_eq!(hai("aaa", "aa"), Some(("a".into(), "".into())));
        assert_eq!(hai("banana", "na"), Some(("bana".into(), "".into())));
    }

    #[test]
    fn dau_va_cuoi_sinh_phan_rong() {
        assert_eq!(hai("abc", "a"), Some(("".into(), "bc".into())));
        assert_eq!(hai("abc", "c"), Some(("ab".into(), "".into())));
        assert_eq!(hai("đ", "đ"), Some(("".into(), "".into())));
        assert_eq!(hai(",", ","), Some(("".into(), "".into())));
        assert_eq!(hai(",a,", ","), Some((",a".into(), "".into())));
    }

    #[test]
    fn khong_khop_tra_none() {
        assert_eq!(hai("a,b,c", "x"), None);
        assert_eq!(hai("abc", "abcd"), None, "sep dài hơn s");
        assert_eq!(hai("", "a"), None, "s rỗng + sep khác rỗng");
        assert_eq!(hai("", "ab"), None);
        assert_eq!(hai("abc", "ac"), None);
        assert_eq!(hai("đ😀x", "😀a"), None);
    }

    #[test]
    fn sep_rong_theo_nghia_rust() {
        assert_eq!(hai("abc", ""), Some(("abc".into(), "".into())));
        assert_eq!(hai("", ""), Some(("".into(), "".into())));
        assert_eq!(hai("đ😀", ""), Some(("đ😀".into(), "".into())));
    }

    #[test]
    fn unicode_emoji_theo_chuoi_con() {
        assert_eq!(hai("đ😀x", "😀"), Some(("đ".into(), "x".into())));
        assert_eq!(hai("ađb", "đ"), Some(("a".into(), "b".into())));
        assert_eq!(hai("đ,😀,x", ","), Some(("đ,😀".into(), "x".into())));
        assert_eq!(hai("a😀b😀c", "😀"), Some(("a😀b".into(), "c".into())));
        assert_eq!(
            hai("👨‍👩‍👧x", "👩"),
            Some(("👨‍".into(), "‍👧x".into())),
            "ZWJ là scalar thường; cắt quanh 👩 vẫn hợp lệ UTF-8"
        );
        assert_eq!(hai("👨‍👩‍👧", "👨‍👩‍👧"), Some(("".into(), "".into())));
    }

    #[test]
    fn goi_tra_option_that_hai_phan() {
        use crate::value::eq;
        let some = |a: &str, b: &str| {
            Value::opt(Some(Value::List(vec![
                Value::Str(a.into()),
                Value::Str(b.into()),
            ])))
        };
        let args = |s: &str, sep: &str| vec![Value::Str(s.into()), Value::Str(sep.into())];
        let span = Span::new(0, 0);
        assert!(eq(
            &goi("str.rsplit_once", &args("a,b,c", ","), span).unwrap(),
            &some("a,b", "c")
        ));
        assert!(eq(
            &goi("str.rsplit_once", &args("abc", ""), span).unwrap(),
            &some("abc", "")
        ));
        assert!(eq(
            &goi("str.rsplit_once", &args("abc", "x"), span).unwrap(),
            &Value::none()
        ));
    }
}
