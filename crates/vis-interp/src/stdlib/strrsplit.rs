// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (rsplit)** — `std.str.rsplit(s, sep) -> list[str]`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//! - Tách **phải → trái**, trả các phần theo **thứ tự ngược** (phần **cuối** trước), **đúng**
//!   nghĩa Rust `str::rsplit`: `rsplit("a,b,c", ",") == ["c", "b", "a"]`.
//! - Không khớp/`sep` dài hơn `s` ⇒ `[s]`; `s` rỗng + `sep` khác rỗng ⇒ `[""]` (`len` = 1).
//! - Dấu phân cách ở **cuối** sinh phần tử rỗng **đầu**: `rsplit("a,b,", ",") == ["", "b", "a"]`;
//!   ở **đầu** sinh phần tử rỗng **cuối**: `rsplit(",a", ",") == ["a", ""]`; dấu **liên tiếp**
//!   là phần tử rỗng riêng: `rsplit("a,,b", ",") == ["b", "", "a"]`.
//! - Chồng lấn **không** khớp lại (khớp không chồng, từ phải): `rsplit("aaa", "aa") == ["", "a"]`,
//!   `rsplit("aaaa", "aa") == ["", "", ""]`, `rsplit("abab", "aba") == ["b", ""]`,
//!   `rsplit("banana", "ana") == ["", "ban"]`.
//! - Đơn vị khớp là **chuỗi con UTF-8** trên **biên scalar** (cùng `str.split`/`str.find`),
//!   **không bao giờ** cắt giữa scalar: `rsplit("đ😀x😀", "😀") == ["", "x", "đ"]`; ZWJ trong
//!   emoji không phải biên đặc biệt: `rsplit("👨‍👩‍👧x", "👩") == ["‍👧x", "👨‍"]`.
//! - `sep` rỗng theo **đúng nghĩa Rust `str::rsplit`** (khác `str.split` của VS: rỗng ⇒ tách
//!   theo scalar và **không** sinh phần rỗng đầu/cuối). Bảng đã kiểm bằng probe Rust:
//!
//!   | biểu thức | kết quả |
//!   |---|---|
//!   | `"abc".rsplit("")` | `["", "c", "b", "a", ""]` |
//!   | `"".rsplit("")` | `["", ""]` |
//!   | `"a".rsplit("")` | `["", "a", ""]` |
//!   | `"đ😀".rsplit("")` | `["", "😀", "đ", ""]` |
//!   | `"".rsplit(",")` | `[""]` |
//!   | `"abc".rsplit(",")` | `["abc"]` |
//!
//! - Không trap; chỉ cấp phát **các phần** kết quả (mỗi phần một `String`), không cấp phát theo
//!   độ dài `s`; số phần bị chặn bởi `len(s) + 1` khi `sep` rỗng.

use vis_syntax::Span;

use crate::interp::R;
use crate::value::Value;

/// Tách `s` theo `sep` từ **phải → trái**; dùng **đúng** `str::rsplit` của Rust nên `sep` rỗng
/// cũng theo nghĩa Rust (khớp tại mọi biên scalar, kể cả đầu/cuối).
pub(super) fn phan(s: &str, sep: &str) -> Vec<String> {
    s.rsplit(sep).map(str::to_string).collect()
}

/// Dispatch `str.rsplit`; không trap, không kiểm `sep` rỗng.
pub(super) fn goi(name: &str, args: &[Value], span: Span) -> R<Value> {
    let s = super::chuoi(name, &args[0], 0, span)?;
    let sep = super::chuoi(name, &args[1], 1, span)?;
    Ok(Value::List(
        phan(s, sep).into_iter().map(Value::Str).collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ds(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn bang_probe_rust_thu_tu_nguoc() {
        assert_eq!(phan("a,b,c", ","), ds(&["c", "b", "a"]));
        assert_eq!(phan("a,b,", ","), ds(&["", "b", "a"]));
        assert_eq!(phan(",a", ","), ds(&["a", ""]));
        assert_eq!(phan(",", ","), ds(&["", ""]));
        assert_eq!(phan(",,", ","), ds(&["", "", ""]));
        assert_eq!(phan("a,,b", ","), ds(&["b", "", "a"]));
        assert_eq!(phan("a::b::c", "::"), ds(&["c", "b", "a"]));
        assert_eq!(phan("aXbXc", "X"), ds(&["c", "b", "a"]));
    }

    #[test]
    fn khong_khop_va_s_rong() {
        assert_eq!(phan("abc", ","), ds(&["abc"]));
        assert_eq!(phan("abc", "abcd"), ds(&["abc"]));
        assert_eq!(phan("", ","), ds(&[""]));
        assert_eq!(phan("", "::"), ds(&[""]));
    }

    #[test]
    fn sep_rong_theo_nghia_rust() {
        assert_eq!(phan("abc", ""), ds(&["", "c", "b", "a", ""]));
        assert_eq!(phan("", ""), ds(&["", ""]));
        assert_eq!(phan("a", ""), ds(&["", "a", ""]));
        assert_eq!(phan("đ😀", ""), ds(&["", "😀", "đ", ""]));
    }

    #[test]
    fn chong_lan_khong_khop_lai() {
        assert_eq!(phan("aaa", "aa"), ds(&["", "a"]));
        assert_eq!(phan("aaaa", "aa"), ds(&["", "", ""]));
        assert_eq!(phan("abab", "aba"), ds(&["b", ""]));
        assert_eq!(phan("banana", "ana"), ds(&["", "ban"]));
    }

    #[test]
    fn unicode_emoji_theo_chuoi_con() {
        assert_eq!(phan("đ😀x😀", "😀"), ds(&["", "x", "đ"]));
        assert_eq!(phan("a😀b😀c", "😀"), ds(&["c", "b", "a"]));
        assert_eq!(phan("đ,😀,x", ","), ds(&["x", "😀", "đ"]));
        assert_eq!(
            phan("👨‍👩‍👧x", "👩"),
            ds(&["‍👧x", "👨‍"]),
            "ZWJ là scalar thường; cắt quanh 👩 vẫn hợp lệ UTF-8"
        );
        assert_eq!(phan("👨‍👩‍👧", "👨‍👩‍👧"), ds(&["", ""]));
    }

    #[test]
    fn goi_tra_list_that() {
        use crate::value::eq;
        let args = |s: &str, sep: &str| vec![Value::Str(s.into()), Value::Str(sep.into())];
        let span = Span::new(0, 0);
        let ky_vong = Value::List(vec![
            Value::Str("c".into()),
            Value::Str("b".into()),
            Value::Str("a".into()),
        ]);
        assert!(eq(
            &goi("str.rsplit", &args("a,b,c", ","), span).unwrap(),
            &ky_vong
        ));
        let rong = Value::List(vec![Value::Str("".into())]);
        assert!(eq(&goi("str.rsplit", &args("", ","), span).unwrap(), &rong));
    }
}
