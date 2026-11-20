// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (replace-n)** — thay **tối đa `n` lần**: `std.str.replace_n`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//! - `replace_n(s, from, to, n)` thay **`n` lần khớp KHÔNG chồng** đầu tiên của `from` trong
//!   `s` bằng `to`, quét **trái → phải**, đúng nghĩa Rust `str::replacen(from, to, n)`.
//!   Cùng đơn vị khớp với `str.replace`/`replace_first`/`replace_last`/`find`
//!   (chuỗi con theo **scalar Unicode**, không phải byte).
//! - `n == 0` ⇒ `s` **nguyên vẹn**; `n` lớn hơn số lần khớp ⇒ thay **tất cả** (như
//!   `str.replace`); không khớp ⇒ `s` nguyên vẹn. Chồng lấn: sau mỗi lần thay, phép quét tiếp
//!   tục **sau** phần `from` vừa khớp (nên `replace_n("aaaa", "aa", "b", 2) == "bb"`).
//! - `from` rỗng theo **đúng nghĩa Rust `replacen`**: khớp rỗng tại **mọi biên scalar** kể cả
//!   đầu và cuối chuỗi; `n` lần khớp rỗng đầu tiên được chèn `to`:
//!
//!   | biểu thức | kết quả |
//!   |---|---|
//!   | `"abc".replacen("", "-", 0)` | `"abc"` |
//!   | `"abc".replacen("", "-", 1)` | `"-abc"` |
//!   | `"abc".replacen("", "-", 2)` | `"-a-bc"` |
//!   | `"abc".replacen("", "-", 3)` | `"-a-b-c"` |
//!   | `"abc".replacen("", "-", 4)` | `"-a-b-c-"` |
//!   | `"abc".replacen("", "-", 9)` | `"-a-b-c-"` |
//!   | `"".replacen("", "-", 1)` | `"-"` |
//!   | `"".replacen("", "-", 9)` | `"-"` |
//!   | `"đ".replacen("", "😀", 2)` | `"😀đ😀"` |
//!
//!   (bảng kiểm bằng probe Rust 1.100.0 — `str::replacen` của std.)
//! - `s` rỗng hoặc `to` rỗng theo cùng quy tắc; `to` rỗng nghĩa là **xoá** các lần khớp.
//! - `n < 0` là trap `VS0501` nêu `str.replace_n` (kiểm **trước** khi thay, cùng mẫu
//!   `str.splitn`/`str.repeat`).
//! - Không có trần riêng (như `str.replace`): số lần thay bị chặn bởi `n` **và** độ dài `s`
//!   (nhiều nhất `len(s) + 1` khi `from` rỗng), nên không cấp phát theo `n`.
//!
//! Không panic Rust: `replacen` chỉ cắt trên biên scalar (UTF-8) nên kết quả luôn hợp lệ.

/// Thay tối đa `n` lần khớp **không chồng** đầu tiên của `from` bằng `to`.
/// `n == 0` ⇒ bản sao nguyên vẹn của `s`; dùng **đúng** `str::replacen` của Rust nên
/// `from` rỗng cũng theo nghĩa Rust (chèn tại mọi biên scalar, kể cả đầu/cuối).
pub(super) fn replace_n(s: &str, from: &str, to: &str, n: usize) -> String {
    s.replacen(from, to, n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toi_da_n_lan_trai_sang_phai() {
        assert_eq!(replace_n("aaaa", "a", "b", 0), "aaaa");
        assert_eq!(replace_n("aaaa", "a", "b", 1), "baaa");
        assert_eq!(replace_n("aaaa", "a", "b", 2), "bbaa");
        assert_eq!(replace_n("aaaa", "a", "b", 3), "bbba");
        assert_eq!(replace_n("aaaa", "a", "b", 4), "bbbb");
        assert_eq!(replace_n("banana", "na", "X", 1), "baXna");
        assert_eq!(replace_n("banana", "na", "X", 2), "baXX");
        assert_eq!(replace_n("ababab", "ab", "X", 2), "XXab");
        assert_eq!(replace_n("x.y.z", ".", "/", 1), "x/y.z");
    }

    #[test]
    fn n_lon_hon_so_lan_khop_thay_tat_ca() {
        assert_eq!(replace_n("aaaa", "a", "b", 99), "bbbb");
        assert_eq!(replace_n("banana", "na", "X", 99), "baXX");
        assert_eq!(replace_n("ababab", "ab", "X", 99), "XXX");
        assert_eq!(replace_n("x.y.z", ".", "/", 99), "x/y/z");
        assert_eq!(replace_n("abc", "z", "X", 99), "abc");
    }

    #[test]
    fn khong_khop_tra_nguyen_ven() {
        assert_eq!(replace_n("abc", "z", "X", 2), "abc");
        assert_eq!(replace_n("abc", "abcd", "X", 5), "abc");
        assert_eq!(replace_n("", "a", "b", 3), "");
    }

    #[test]
    fn from_rong_theo_nghia_rust() {
        assert_eq!(replace_n("abc", "", "-", 0), "abc");
        assert_eq!(replace_n("abc", "", "-", 1), "-abc");
        assert_eq!(replace_n("abc", "", "-", 2), "-a-bc");
        assert_eq!(replace_n("abc", "", "-", 3), "-a-b-c");
        assert_eq!(replace_n("abc", "", "-", 4), "-a-b-c-");
        assert_eq!(replace_n("abc", "", "-", 9), "-a-b-c-");
        assert_eq!(replace_n("", "", "-", 1), "-");
        assert_eq!(replace_n("", "", "-", 9), "-");
        assert_eq!(replace_n("abc", "", "", 2), "abc");
        assert_eq!(replace_n("đ", "", "😀", 2), "😀đ😀");
    }

    #[test]
    fn s_rong_va_to_rong() {
        assert_eq!(replace_n("abc", "b", "", 1), "ac");
        assert_eq!(replace_n("abc", "b", "", 2), "ac");
        assert_eq!(replace_n("banana", "na", "", 2), "ba");
        assert_eq!(replace_n("a", "a", "", 1), "");
        assert_eq!(replace_n("đ😀", "😀", "", 5), "đ");
    }

    #[test]
    fn chong_lan_khong_chong_va_quet_tiep_sau_khop() {
        assert_eq!(replace_n("aaaa", "aa", "b", 1), "baa");
        assert_eq!(replace_n("aaaa", "aa", "b", 2), "bb");
        assert_eq!(replace_n("aaaa", "aa", "b", 3), "bb");
        assert_eq!(replace_n("aaa", "aa", "b", 2), "ba");
        assert_eq!(replace_n("ababa", "aba", "X", 2), "Xba");
        assert_eq!(replace_n("aaa", "a", "aa", 2), "aaaaa");
        assert_eq!(replace_n("aaa", "a", "aa", 9), "aaaaaa");
    }

    #[test]
    fn unicode_theo_scalar_khong_phai_byte() {
        assert_eq!(replace_n("đ😀x😀", "😀", "Z", 1), "đZx😀");
        assert_eq!(replace_n("đ😀x😀", "😀", "Z", 2), "đZxZ");
        assert_eq!(replace_n("đađ", "đ", "Đ", 2), "ĐaĐ");
        assert_eq!(replace_n("😀😀😀", "😀", "🙂", 2), "🙂🙂😀");
        assert_eq!(replace_n("đ😀x", "đ😀", "Q", 1), "Qx");
        assert_eq!(replace_n("đ😀x", "x", "đ", 1), "đ😀đ");
        assert_eq!(
            replace_n("👨‍👩‍👧x", "👩", "M", 1),
            "👨‍M‍👧x",
            "chỉ thay scalar giữa, không cắt ZWJ"
        );
        assert_eq!(replace_n("đ😀xđ😀y", "đ😀", "Q", 2), "QxQy");
    }
}
