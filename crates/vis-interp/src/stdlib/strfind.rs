// SPDX-License-Identifier: MIT OR Apache-2.0
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

use crate::value::Value;

/// Vị trí **scalar** của lần xuất hiện **đầu tiên** của `needle`; `None` nếu không có.
/// `needle` rỗng ⇒ `Some(0)` (Rust `str::find`).
pub(super) fn find(s: &str, needle: &str) -> Option<i64> {
    s.find(needle).map(|b| s[..b].chars().count() as i64)
}

/// Vị trí **scalar** của lần xuất hiện **cuối cùng** của `needle`; `None` nếu không có.
/// `needle` rỗng ⇒ `Some(len_scalar)` (Rust `str::rfind`).
pub(super) fn rfind(s: &str, needle: &str) -> Option<i64> {
    if needle.is_empty() {
        return Some(s.chars().count() as i64);
    }
    s.rfind(needle).map(|b| s[..b].chars().count() as i64)
}

/// Số lần xuất hiện **không chồng** của `needle` (Rust `str::matches`); `needle` rỗng ⇒
/// `len_scalar + 1` (khớp tại mọi biên scalar, kể cả cuối chuỗi).
pub(super) fn count(s: &str, needle: &str) -> i64 {
    s.matches(needle).count() as i64
}

/// Dispatch chung cho ba hàm; `name` là tên đầy đủ `"str.find"`/`"str.rfind"`/`"str.count"`.
pub(super) fn goi(name: &str, s: &str, needle: &str) -> Value {
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
