// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (strtrim)** — cắt theo **tập scalar**: `std.str.trim_matches` /
//! `trim_start_matches` / `trim_end_matches`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//! - `chars` là **tập** scalar Unicode: thứ tự **không** quan trọng, lặp **không** sao.
//! - Bỏ **lặp lại** scalar ở đầu/cuối (hoặc cả hai) khi scalar đó thuộc `chars`, tới scalar
//!   đầu tiên (từ phía tương ứng) **không** thuộc tập — như Rust `trim*_matches` với closure.
//! - `chars` rỗng ⇒ trả `s` **nguyên vẹn**; mọi scalar đều thuộc tập ⇒ `""`.
//! - Đơn vị là **scalar** (`char`), **không** phải byte và **không** phải grapheme: emoji
//!   nhiều scalar (vd. `👨‍👩‍👧`) chỉ bị cắt từng scalar có mặt trong `chars`.
//!
//! **Tối ưu (VD-270 trim-perf)**: `chars` ≤ [`NGUONG_BANG_TRA`] scalar ⇒ duyệt tuyến tính
//! (không cấp phát); lớn hơn ⇒ sắp xếp `Vec<char>` **một lần** rồi tìm nhị phân
//! O(log |chars|) mỗi scalar. Kết quả không đổi: bảng tra vẫn là **tập** scalar (lặp/bỏ
//! trùng như cũ), và ngưỡng chỉ chọn cách tra.

use vis_syntax::Span;

use crate::interp::R;
use crate::value::Value;

/// Tối đa số scalar của `chars` để còn duyệt tuyến tính; vượt ngưỡng thì dựng bảng tra
/// (sắp xếp một lần, đổi lại mỗi phép tra thành O(log |chars|)). Không ảnh hưởng ngữ nghĩa.
const NGUONG_BANG_TRA: usize = 8;

/// `true` nếu `c` thuộc **tập** scalar `chars` (duyệt tuyến tính, không cấp phát).
fn thuoc(chars: &str, c: char) -> bool {
    chars.chars().any(|x| x == c)
}

/// Cắt `s` theo `name` với predicate thành viên `thuoc`; dùng chung cho cả hai đường tra.
fn cat_voi(name: &str, s: &str, thuoc: impl Fn(char) -> bool) -> String {
    match name {
        "str.trim_start_matches" => s.trim_start_matches(thuoc).to_string(),
        "str.trim_end_matches" => s.trim_end_matches(thuoc).to_string(),
        _ => s.trim_matches(thuoc).to_string(),
    }
}

/// Cắt `s` theo `name` (`"str.trim_matches"` / `"str.trim_start_matches"` /
/// `"str.trim_end_matches"`); `chars` rỗng ⇒ `s` nguyên vẹn, toàn bộ thuộc tập ⇒ `""`.
pub(super) fn cat(name: &str, s: &str, chars: &str) -> String {
    if chars.is_empty() {
        return s.to_string();
    }
    if chars.chars().count() > NGUONG_BANG_TRA {
        let mut tap: Vec<char> = chars.chars().collect();
        tap.sort_unstable();
        tap.dedup();
        return cat_voi(name, s, |c| tap.binary_search(&c).is_ok());
    }
    cat_voi(name, s, |c| thuoc(chars, c))
}

/// Dispatch chung cho ba hàm; `name` là tên đầy đủ `"str.trim*_matches"`.
pub(super) fn goi(name: &str, args: &[Value], span: Span) -> R<Value> {
    let s = super::chuoi(name, &args[0], 0, span)?;
    let chars = super::chuoi(name, &args[1], 1, span)?;
    Ok(Value::Str(cat(name, s, chars)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tap_scalar_hai_phia() {
        assert_eq!(cat("str.trim_matches", "--x--", "-"), "x");
        assert_eq!(cat("str.trim_start_matches", "--x--", "-"), "x--");
        assert_eq!(cat("str.trim_end_matches", "--x--", "-"), "--x");
        assert_eq!(cat("str.trim_matches", "abbaHELLOabba", "ab"), "HELLO");
        assert_eq!(cat("str.trim_matches", "abbaHELLOabba", "ba"), "HELLO");
        assert_eq!(cat("str.trim_matches", "a-b", "-a"), "b");
    }

    #[test]
    fn rong_va_toan_bo() {
        assert_eq!(cat("str.trim_matches", "abc", ""), "abc");
        assert_eq!(cat("str.trim_matches", "xyx", "xy"), "");
        assert_eq!(cat("str.trim_start_matches", "xxx", "x"), "");
        assert_eq!(cat("str.trim_end_matches", "xxx", "x"), "");
    }

    #[test]
    fn scalar_khong_phai_grapheme() {
        assert_eq!(cat("str.trim_matches", "đ😀đxđ😀đ", "đ😀"), "x");
        assert_eq!(cat("str.trim_start_matches", "👨‍👩‍👧", "👨").chars().count(), 4);
        assert_eq!(cat("str.trim_matches", "👨‍👩‍👧", "👨👧").chars().count(), 3);
    }

    /// Bản tham chiếu **trước tối ưu** — duyệt tuyến tính mọi scalar của `chars`.
    fn cat_tuyen_tinh(name: &str, s: &str, chars: &str) -> String {
        if chars.is_empty() {
            return s.to_string();
        }
        let trong = |c: char| chars.chars().any(|x| x == c);
        match name {
            "str.trim_start_matches" => s.trim_start_matches(trong).to_string(),
            "str.trim_end_matches" => s.trim_end_matches(trong).to_string(),
            _ => s.trim_matches(trong).to_string(),
        }
    }

    const TEN: [&str; 3] = [
        "str.trim_matches",
        "str.trim_start_matches",
        "str.trim_end_matches",
    ];

    #[test]
    fn tap_lon_trung_va_astral() {
        // 22 scalar > ngưỡng: có lặp ('x' 20 lần) và astral ('😀', 'đ').
        let mut bo = String::new();
        for _ in 0..20 {
            bo.push('x');
        }
        bo.push('😀');
        bo.push('đ');
        assert!(bo.chars().count() > NGUONG_BANG_TRA);
        assert_eq!(cat("str.trim_matches", "xx😀đXđ😀xx", &bo), "X");
        assert_eq!(cat("str.trim_matches", "xđ😀x", &bo), "");
        assert_eq!(cat("str.trim_start_matches", "x😀X", &bo), "X");
        assert_eq!(cat("str.trim_end_matches", "X😀x", &bo), "X");
        // Tập lớn nhưng rỗng ⇒ nguyên vẹn (nhánh sớm, không dựng bảng).
        assert_eq!(cat("str.trim_matches", "xx😀", ""), "xx😀");
    }

    #[test]
    fn tap_lon_tuong_duong_ban_tuyen_tinh() {
        // 300 scalar thường + trùng + astral ⇒ chắc chắn đi nhánh bảng tra nhị phân.
        let mut bo = String::new();
        for c in (0x100u32..0x100 + 300).filter_map(char::from_u32) {
            bo.push(c);
        }
        for c in ['đ', '😀', 'đ', '😀', '\u{10FFFF}', '\u{10FFFF}'] {
            bo.push(c);
        }
        assert!(bo.chars().count() > NGUONG_BANG_TRA);
        let ca = [
            "",
            "x",
            "đ😀x😀đ",
            "đđ😀😀x",
            "xđ😀",
            "abcđ😀xyz",
            "\u{100}\u{12b}MID\u{12b}\u{100}",
            "\u{10FFFF}x\u{10FFFF}",
            "😀😀😀",
        ];
        for name in TEN {
            for s in ca {
                assert_eq!(
                    cat(name, s, &bo),
                    cat_tuyen_tinh(name, s, &bo),
                    "lệch kết quả: {name}({s:?}, tập lớn)"
                );
            }
        }
    }

    #[test]
    fn nguong_bang_tra_bien() {
        // 8 scalar ⇒ tuyến tính; 9 scalar ⇒ bảng tra — cả hai phải cùng kết quả.
        let tam = "abcdefgh";
        let chin = "abcdefghi";
        assert_eq!(tam.chars().count(), NGUONG_BANG_TRA);
        for (bo, goc, mong) in [
            (tam, "abcdefghXhgfedcba", ["X", "Xhgfedcba", "abcdefghX"]),
            (
                chin,
                "abcdefghiXihgfedcba",
                ["X", "Xihgfedcba", "abcdefghiX"],
            ),
        ] {
            assert_eq!(cat("str.trim_matches", goc, bo), mong[0]);
            assert_eq!(cat("str.trim_start_matches", goc, bo), mong[1]);
            assert_eq!(cat("str.trim_end_matches", goc, bo), mong[2]);
            assert_eq!(cat("str.trim_matches", "X", bo), "X");
        }
    }
}
