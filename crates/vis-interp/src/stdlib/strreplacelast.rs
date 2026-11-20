// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (replace-last)** — thay **một lần** cuối cùng: `std.str.replace_last`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//! - `replace_last(s, from, to)` thay **đúng lần xuất hiện cuối cùng** của `from` trong `s`
//!   bằng `to`; mọi lần xuất hiện trước giữ nguyên. Cùng đơn vị khớp với
//!   `str.replace`/`replace_first`/`rfind` (chuỗi con theo **scalar Unicode**, không phải byte).
//! - Không khớp ⇒ `s` nguyên vẹn.
//! - `from` rỗng ⇒ **nối** `to` một lần ở **cuối** (lần khớp rỗng cuối ở vị trí `len(s)`,
//!   đúng nghĩa Rust `s.rfind("")`): `replace_last("abc", "", "-") == "abc-"`,
//!   `replace_last("", "", "-") == "-"`.
//! - `s` rỗng hoặc `to` rỗng theo cùng quy tắc; `to` rỗng nghĩa là **xoá** lần khớp cuối.
//! - `to` được phép dài tùy ý: không có trần riêng (như `str.replace`), vì chỉ một phép thay
//!   và kết quả nhiều nhất là `s` + `to` (không nhân theo số lần khớp).
//!
//! Không panic Rust: `rfind` chỉ trả offset trên biên scalar, nên phép cắt `s[..i]` /
//! `s[i + from.len()..]` luôn là UTF-8 hợp lệ.

/// Thay lần xuất hiện **cuối cùng** của `from` bằng `to`; không khớp ⇒ bản sao nguyên vẹn của
/// `s`. `from` rỗng ⇒ `s + to` (nối một lần ở cuối), đúng nghĩa Rust `s.rfind("") == len(s)`.
pub(super) fn replace_last(s: &str, from: &str, to: &str) -> String {
    // `rfind` là đúng phép khớp của `str.rfind` (scalar Unicode, biên UTF-8); `from` rỗng
    // ⇒ `Some(s.len())` — lần khớp rỗng **cuối** — nên kết quả là `s + to`.
    match s.rfind(from) {
        Some(i) => {
            let mut out = String::with_capacity(s.len() + to.len());
            out.push_str(&s[..i]);
            out.push_str(to);
            out.push_str(&s[i + from.len()..]);
            out
        }
        None => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chi_thay_lan_cuoi() {
        assert_eq!(replace_last("aaa", "a", "b"), "aab");
        assert_eq!(replace_last("banana", "na", "X"), "banaX");
        assert_eq!(replace_last("ababab", "ab", "X"), "ababX");
        assert_eq!(
            replace_last("hello world hello", "hello", "hi"),
            "hello world hi"
        );
    }

    #[test]
    fn khong_khop_tra_nguyen_ven() {
        assert_eq!(replace_last("abc", "z", "X"), "abc");
        assert_eq!(replace_last("abc", "abcd", "X"), "abc");
        assert_eq!(replace_last("", "a", "b"), "");
    }

    #[test]
    fn from_rong_noi_cuoi_mot_lan() {
        assert_eq!(replace_last("abc", "", "-"), "abc-");
        assert_eq!(replace_last("", "", "-"), "-");
        assert_eq!(replace_last("abc", "", ""), "abc");
        assert_eq!(replace_last("đ", "", "😀"), "đ😀");
    }

    #[test]
    fn s_rong_va_to_rong() {
        assert_eq!(replace_last("abc", "b", ""), "ac");
        assert_eq!(replace_last("abc", "abc", ""), "");
        assert_eq!(replace_last("a", "a", ""), "");
        assert_eq!(replace_last("đ😀", "😀", ""), "đ");
    }

    #[test]
    fn unicode_theo_scalar_khong_phai_byte() {
        assert_eq!(replace_last("đ😀x😀", "😀", "Z"), "đ😀xZ");
        assert_eq!(replace_last("đađ", "đ", "Đ"), "đaĐ");
        assert_eq!(replace_last("😀😀😀", "😀", "🙂"), "😀😀🙂");
        assert_eq!(replace_last("đ😀x", "đ😀", "Q"), "Qx");
        assert_eq!(replace_last("đ😀x", "x", "đ"), "đ😀đ");
        assert_eq!(replace_last("đ😀xđ😀y", "đ😀", "Q"), "đ😀xQy");
    }

    #[test]
    fn chong_lan_chon_lan_cuoi() {
        assert_eq!(replace_last("aaa", "aa", "b"), "ab");
        assert_eq!(replace_last("aaaa", "aa", "b"), "aab");
        assert_eq!(replace_last("aaa", "a", "aa"), "aaaa");
        assert_eq!(replace_last("aaaa", "aaa", "b"), "ab");
        assert_eq!(replace_last("ababa", "aba", "X"), "abX");
    }
}
