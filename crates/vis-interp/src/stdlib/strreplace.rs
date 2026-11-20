// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (replace-first)** — thay **một lần** đầu tiên: `std.str.replace_first`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//! - `replace_first(s, from, to)` thay **đúng lần xuất hiện đầu tiên** của `from` trong `s`
//!   bằng `to`; mọi lần xuất hiện sau giữ nguyên. Cùng đơn vị khớp với `str.replace`/`find`
//!   (chuỗi con theo **scalar Unicode**, không phải byte).
//! - Không khớp ⇒ `s` nguyên vẹn.
//! - `from` rỗng ⇒ **chèn** `to` một lần ở **đầu** (Rust `str::replacen(…, 1)`):
//!   `replace_first("abc", "", "-") == "-abc"`, `replace_first("", "", "-") == "-"`.
//! - `s` rỗng hoặc `to` rỗng theo cùng quy tắc; `to` rỗng nghĩa là **xoá** lần khớp đầu.
//! - `to` được phép dài tùy ý: không có trần riêng (như `str.replace`), vì chỉ một phép thay
//!   và kết quả nhiều nhất là `s` + `to` (không nhân theo số lần khớp).
//!
//! Không panic Rust: mọi đầu vào `str` hợp lệ đều có kết quả; `replacen` bảo toàn biên scalar.

/// Thay lần xuất hiện **đầu tiên** của `from` bằng `to`; không khớp ⇒ bản sao nguyên vẹn của `s`.
/// `from` rỗng ⇒ `to + s` (chèn một lần ở đầu), đúng nghĩa Rust `s.replacen(from, to, 1)`.
pub(super) fn replace_first(s: &str, from: &str, to: &str) -> String {
    s.replacen(from, to, 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chi_thay_lan_dau() {
        assert_eq!(replace_first("aaa", "a", "b"), "baa");
        assert_eq!(replace_first("banana", "na", "X"), "baXna");
        assert_eq!(replace_first("ababab", "ab", "X"), "Xabab");
        assert_eq!(
            replace_first("hello world hello", "hello", "hi"),
            "hi world hello"
        );
    }

    #[test]
    fn khong_khop_tra_nguyen_ven() {
        assert_eq!(replace_first("abc", "z", "X"), "abc");
        assert_eq!(replace_first("abc", "abcd", "X"), "abc");
        assert_eq!(replace_first("", "a", "b"), "");
    }

    #[test]
    fn from_rong_chen_dau_mot_lan() {
        assert_eq!(replace_first("abc", "", "-"), "-abc");
        assert_eq!(replace_first("", "", "-"), "-");
        assert_eq!(replace_first("abc", "", ""), "abc");
        assert_eq!(replace_first("đ", "", "😀"), "😀đ");
    }

    #[test]
    fn s_rong_va_to_rong() {
        assert_eq!(replace_first("abc", "b", ""), "ac");
        assert_eq!(replace_first("abc", "abc", ""), "");
        assert_eq!(replace_first("a", "a", ""), "");
        assert_eq!(replace_first("đ😀", "😀", ""), "đ");
    }

    #[test]
    fn unicode_theo_scalar_khong_phai_byte() {
        assert_eq!(replace_first("đ😀x😀", "😀", "Z"), "đZx😀");
        assert_eq!(replace_first("đađ", "đ", "Đ"), "Đađ");
        assert_eq!(replace_first("😀😀😀", "😀", "🙂"), "🙂😀😀");
        assert_eq!(replace_first("đ😀x", "đ😀", "Q"), "Qx");
        assert_eq!(replace_first("đ😀x", "x", "đ"), "đ😀đ");
    }

    #[test]
    fn chong_lan_chi_mot_lan() {
        assert_eq!(replace_first("aaa", "aa", "b"), "ba");
        assert_eq!(replace_first("aaaa", "aa", "b"), "baa");
        assert_eq!(replace_first("aaa", "a", "aa"), "aaaa");
        assert_eq!(replace_first("aaaa", "aaa", "b"), "ba");
    }
}
