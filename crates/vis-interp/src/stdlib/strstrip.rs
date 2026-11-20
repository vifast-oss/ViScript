// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (strstrip)** — bỏ **tiền tố/hậu tố**: `std.str.strip_prefix` / `strip_suffix`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//! - `strip_prefix(s, p)`: `s` **bắt đầu** bằng `p` ⇒ `Some(s[len(p)..])`, ngược lại ⇒ `None`.
//! - `strip_suffix(s, p)`: `s` **kết thúc** bằng `p` ⇒ `Some(s[..len(s)-len(p)])`, ngược lại
//!   ⇒ `None`.
//! - Chỉ bỏ **một lần**, và **chỉ** khi khớp đúng nguyên `p`; không cắt theo tập ký tự (khác
//!   `trim_matches`) và không tìm giữa chuỗi (khác `find`/`replace`).
//! - `p` rỗng ⇒ `Some(s)` nguyên vẹn (nghĩa Rust `str::strip_prefix`/`strip_suffix`).
//! - Đơn vị khớp là **byte** nhưng biên luôn rơi trên **ranh giới scalar Unicode**: cắt sau
//!   `len(p)` byte của một tiền tố khớp là an toàn, nên `"đ😀x"` bỏ tiền tố `"đ"` ⇒ `"😀x"`
//!   và bỏ hậu tố `"😀x"` ⇒ `"đ"`; không bao giờ trả chuỗi UTF-8 hỏng.
//! - Kiểu trả `str?` = `Option[str]` **thật** (`Some(Str)`/`None`), không phải `{error}`.
//! - Không có trần riêng: mọi phép là O(độ dài chuỗi), không cấp phát khi không khớp.

use crate::value::Value;

/// Phần **còn lại** sau khi bỏ tiền tố `prefix`; `None` nếu `s` không bắt đầu bằng `prefix`.
/// `prefix` rỗng ⇒ `Some(s)` (Rust `str::strip_prefix`).
pub(super) fn strip_prefix<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    s.strip_prefix(prefix)
}

/// Phần **còn lại** sau khi bỏ hậu tố `suffix`; `None` nếu `s` không kết thúc bằng `suffix`.
/// `suffix` rỗng ⇒ `Some(s)` (Rust `str::strip_suffix`).
pub(super) fn strip_suffix<'a>(s: &'a str, suffix: &str) -> Option<&'a str> {
    s.strip_suffix(suffix)
}

/// Dispatch chung cho hai hàm; `name` là `"str.strip_prefix"` hoặc `"str.strip_suffix"`.
pub(super) fn goi(name: &str, s: &str, moc: &str) -> Value {
    let con_lai = match name {
        "str.strip_prefix" => strip_prefix(s, moc),
        _ => strip_suffix(s, moc),
    };
    Value::opt(con_lai.map(|x| Value::Str(x.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_khop_va_khong_khop() {
        assert_eq!(strip_prefix("banana", "ban"), Some("ana"));
        assert_eq!(strip_prefix("banana", "x"), None);
        assert_eq!(strip_prefix("banana", "ana"), None, "chỉ khớp ở ĐẦU");
        assert_eq!(strip_suffix("banana", "ana"), Some("ban"));
        assert_eq!(strip_suffix("banana", "x"), None);
        assert_eq!(strip_suffix("banana", "ban"), None, "chỉ khớp ở CUỐI");
        assert_eq!(strip_prefix("abc", "abcd"), None, "tiền tố dài hơn");
        assert_eq!(strip_suffix("abc", "abcd"), None, "hậu tố dài hơn");
    }

    #[test]
    fn moc_rong_tra_nguyen_chuoi() {
        assert_eq!(strip_prefix("abc", ""), Some("abc"));
        assert_eq!(strip_suffix("abc", ""), Some("abc"));
        assert_eq!(strip_prefix("", ""), Some(""));
        assert_eq!(strip_suffix("", ""), Some(""));
        assert_eq!(strip_prefix("", "a"), None);
        assert_eq!(strip_suffix("", "a"), None);
    }

    #[test]
    fn chong_lan_chi_bo_mot_lan() {
        assert_eq!(strip_prefix("aaa", "a"), Some("aa"));
        assert_eq!(strip_suffix("aaa", "a"), Some("aa"));
        assert_eq!(strip_prefix("aaa", "aa"), Some("a"));
        assert_eq!(strip_suffix("aaa", "aa"), Some("a"));
        assert_eq!(strip_prefix("aaa", "aaa"), Some(""));
        assert_eq!(strip_suffix("aaa", "aaa"), Some(""));
        assert_eq!(strip_prefix("aaaa", "aa"), Some("aa"));
        assert_eq!(strip_suffix("aaaa", "aa"), Some("aa"));
    }

    #[test]
    fn unicode_theo_scalar_khong_phai_byte() {
        assert_eq!(strip_prefix("đ😀x", "đ"), Some("😀x"));
        assert_eq!(strip_suffix("đ😀x", "😀x"), Some("đ"));
        assert_eq!(strip_prefix("đ😀x", "đ😀"), Some("x"));
        assert_eq!(strip_suffix("đ😀x", "x"), Some("đ😀"));
        assert_eq!(strip_prefix("😀a", "😀"), Some("a"));
        assert_eq!(strip_suffix("a😀", "😀"), Some("a"));
        // "đ" (U+0111) là **một** scalar 2 byte; bỏ nó phải còn đúng phần sau, không lệch byte.
        assert_eq!(strip_prefix("đx", "đ"), Some("x"));
        assert_eq!(strip_suffix("xđ", "đ"), Some("x"));
    }

    #[test]
    fn goi_tra_option_that() {
        use crate::value::eq;
        let some = |s: &str| Value::opt(Some(Value::Str(s.into())));
        assert!(eq(&goi("str.strip_prefix", "abc", "a"), &some("bc")));
        assert!(eq(&goi("str.strip_suffix", "abc", "c"), &some("ab")));
        assert!(eq(&goi("str.strip_prefix", "abc", "b"), &Value::none()));
        assert!(eq(&goi("str.strip_suffix", "abc", "b"), &Value::none()));
    }
}
