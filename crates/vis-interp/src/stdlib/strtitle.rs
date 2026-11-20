// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (str-title)** — `std.str.title(s) -> str`: chữ hoa đầu **từ**, phần còn lại chữ thường.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//! - Tách **từ** theo **khoảng trắng Unicode** (`char::is_whitespace`), **không** theo dấu câu:
//!   `"hello-world"` ⇒ `"Hello-world"` (gạch nối là ký tự thường **trong** từ), còn
//!   `"hello, world!"` ⇒ `"Hello, World!"` (dấu cách mới là biên).
//! - Với mỗi từ, **scalar đầu** đổi sang **chữ hoa** (`char::to_uppercase`) và **mọi scalar sau**
//!   đổi sang **chữ thường** (`char::to_lowercase`). Đơn vị là **scalar** Unicode, không phải
//!   byte/grapheme; phép đổi có thể **nở** thành nhiều scalar (`ß` ⇒ `SS`, `İ` ⇒ `i` + dấu chấm).
//! - Khoảng trắng **giữ nguyên từng scalar, đúng thứ tự** (`"a  \tb\nc"` ⇒ `"A  \tB\nC"`); chuỗi
//!   rỗng hoặc **chỉ** khoảng trắng ⇒ trả **nguyên vẹn**.
//! - Không trap, không cấp phát thừa ngoài chuỗi kết quả; O(số scalar của `s`).

/// Chữ hoa đầu từ, chữ thường phần còn lại; khoảng trắng Unicode giữ nguyên từng scalar.
///
/// `dau_tu` bật lại **chỉ** khi gặp scalar `is_whitespace` — dấu câu **không** mở từ mới, nên
/// `"hello-world"` ⇒ `"Hello-world"` (chỉ `h` được viết hoa). Scalar đầu từ được đổi hoa bằng
/// `char::to_uppercase` (có thể nở: `ß` ⇒ `SS`), các scalar sau bằng `char::to_lowercase`.
pub(super) fn title(s: &str) -> String {
    let mut ra = String::with_capacity(s.len());
    let mut dau_tu = true;
    for c in s.chars() {
        if c.is_whitespace() {
            ra.push(c);
            dau_tu = true;
        } else if dau_tu {
            ra.extend(c.to_uppercase());
            dau_tu = false;
        } else {
            ra.extend(c.to_lowercase());
        }
    }
    ra
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_va_tron_hoa_thuong() {
        assert_eq!(title("hello world"), "Hello World");
        assert_eq!(title("hello"), "Hello");
        assert_eq!(title("a b c"), "A B C");
        assert_eq!(title("hELLo wORLD"), "Hello World");
        assert_eq!(title("HELLO"), "Hello");
        assert_eq!(title("mIxEd CaSe TeXt"), "Mixed Case Text");
    }

    #[test]
    fn khoang_trang_giu_nguyen_tung_scalar() {
        assert_eq!(title("a  b\tc\nd"), "A  B\tC\nD");
        assert_eq!(title("  a\r\n b  "), "  A\r\n B  ");
        // Unicode whitespace: NBSP (U+00A0) và ideographic space (U+3000) cũng là biên từ.
        assert_eq!(
            title("a\u{A0}b\u{3000}c"),
            "A\u{A0}B\u{3000}C",
            "khoảng trắng Unicode phải giữ nguyên và tách từ"
        );
    }

    #[test]
    fn rong_va_chi_khoang_trang_nguyen_ven() {
        assert_eq!(title(""), "");
        assert_eq!(title("   "), "   ");
        assert_eq!(title("\t\n\r"), "\t\n\r");
        assert_eq!(title("\u{A0}\u{3000}"), "\u{A0}\u{3000}");
    }

    #[test]
    fn dau_cau_khong_mo_tu_moi() {
        assert_eq!(title("hello-world"), "Hello-world");
        assert_eq!(title("hello, world!"), "Hello, World!");
        assert_eq!(title("don't STOP"), "Don't Stop");
        // Dấu câu **đầu** từ là scalar đầu tiên: nó được "viết hoa" (không đổi) rồi các scalar
        // sau thành chữ thường — đúng hợp đồng "biên từ chỉ là khoảng trắng".
        assert_eq!(title("(hello)"), "(hello)");
        assert_eq!(title("-Hello"), "-hello");
    }

    #[test]
    fn viet_va_astral_theo_scalar() {
        assert_eq!(title("đẶNG văn hÙNG"), "Đặng Văn Hùng");
        assert_eq!(title("XIN CHÀO THẾ GIỚI"), "Xin Chào Thế Giới");
        assert_eq!(title("😀abc 😀dEF"), "😀abc 😀def");
        assert_eq!(title("👨‍👩‍👧 FAMILY"), "👨‍👩‍👧 Family");
        // Emoji là **một** scalar astral: không bị tách byte, char_count không đổi.
        assert_eq!(title("😀ABC").chars().count(), 4);
    }

    #[test]
    fn ss_no_thanh_hai_scalar() {
        assert_eq!(title("ßig ß"), "SSig SS");
        assert_eq!(title("ẞIG"), "ẞig");
        assert_eq!(title("aß"), "Aß", "ß không phải scalar đầu thì giữ nguyên");
    }
}
