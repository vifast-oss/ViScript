// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (splitn)** — `std.str.splitn(s, sep, n) -> list[str]`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//! - Tách **trái → phải**, tối đa `n` phần; **phần cuối** là **phần còn lại nguyên văn** của
//!   `s`, kể cả các dấu phân cách chưa dùng: `splitn("a,b,c", ",", 2) == ["a", "b,c"]`.
//! - `n == 0` ⇒ `[]`; `n == 1` ⇒ `[s]`; `n > số phần` ⇒ **toàn bộ** phần như `str.split`.
//! - Dấu phân cách ở **cuối** `s` sinh phần tử rỗng cuối: `splitn("a,b,", ",", 3) == ["a", "b", ""]`.
//! - Đơn vị khớp là **chuỗi con UTF-8** (cùng `str.split`/`str.find`), **không bao giờ** cắt giữa
//!   scalar: `splitn("đ😀x", "😀", 2) == ["đ", "x"]`; kết quả luôn là UTF-8 hợp lệ.
//! - `sep` rỗng theo **đúng nghĩa Rust `str::splitn`** (khác `str.split` của VS: `str.split` rỗng
//!   tách theo **scalar** và **không** sinh phần rỗng đầu/cuối). Bảng đã kiểm bằng probe Rust:
//!
//!   | biểu thức | kết quả |
//!   |---|---|
//!   | `"abc".splitn(2, "")` | `["", "abc"]` |
//!   | `"abc".splitn(3, "")` | `["", "a", "bc"]` |
//!   | `"abc".splitn(1, "")` | `["abc"]` |
//!   | `"abc".splitn(0, "")` | `[]` |
//!   | `"".splitn(2, "")` | `["", ""]` |
//!   | `"".splitn(1, "")` | `[""]` |
//!   | `"a".splitn(3, "")` | `["", "a", ""]` |
//!   | `"đ😀".splitn(3, "")` | `["", "đ", "😀"]` |
//!
//! - `n < 0` là trap `VS0501` nêu `str.splitn` (kiểm **trước** khi tách); `n` là `int`.
//!
//! Không có trần riêng: số phần bị chặn bởi `n` **và** độ dài `s` (nhiều nhất `len(s) + 2` khi
//! `sep` rỗng), nên không cấp phát theo `n`; mọi phép tuyến tính theo `s`.

use vis_syntax::Span;

use crate::interp::R;
use crate::value::Value;

/// Tách `s` theo `sep`, tối đa `n` phần; `n <= 0` ⇒ `[]`. Dùng **đúng** `str::splitn` của Rust
/// nên `sep` rỗng cũng theo nghĩa Rust (khớp tại mọi biên scalar, kể cả đầu/cuối).
pub(super) fn phan(s: &str, sep: &str, n: i64) -> Vec<String> {
    if n <= 0 {
        return Vec::new();
    }
    // `n` dương; trên nền 32-bit, `i64` vượt `usize` ⇒ coi như vô hạn (mọi phần).
    let n = usize::try_from(n).unwrap_or(usize::MAX);
    s.splitn(n, sep).map(str::to_string).collect()
}

/// Dispatch `str.splitn`; `n < 0` ⇒ trap `VS0501` (cùng mẫu `str.repeat`/`list.windows`).
pub(super) fn goi(name: &str, args: &[Value], span: Span) -> R<Value> {
    let s = super::chuoi(name, &args[0], 0, span)?;
    let sep = super::chuoi(name, &args[1], 1, span)?;
    let n = super::so(name, &args[2], 2, span)?;
    if n < 0 {
        return Err(super::loi(
            format!("`{name}`: số phần `n` phải ≥ 0, được {n}"),
            span,
        ));
    }
    Ok(Value::List(
        phan(s, sep, n).into_iter().map(Value::Str).collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ds(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn toi_da_n_phan_va_phan_cuoi_giu_nguyen() {
        assert_eq!(phan("a,b,c", ",", 2), ds(&["a", "b,c"]));
        assert_eq!(phan("a,b,c", ",", 1), ds(&["a,b,c"]));
        assert_eq!(phan("a,b,c", ",", 0), ds(&[]));
        assert_eq!(phan("a,b,c", ",", 3), ds(&["a", "b", "c"]));
        assert_eq!(phan("a,b,c", ",", 99), ds(&["a", "b", "c"]));
        assert_eq!(phan("abc", ",", 2), ds(&["abc"]));
        assert_eq!(phan("", ",", 2), ds(&[""]));
        assert_eq!(phan("", ",", 0), ds(&[]));
    }

    #[test]
    fn dau_phan_cach_cuoi_sinh_phan_rong() {
        assert_eq!(phan("a,b,", ",", 2), ds(&["a", "b,"]));
        assert_eq!(phan("a,b,", ",", 3), ds(&["a", "b", ""]));
        assert_eq!(phan("a,b,", ",", 4), ds(&["a", "b", ""]));
        assert_eq!(phan(",", ",", 1), ds(&[","]));
        assert_eq!(phan(",", ",", 2), ds(&["", ""]));
        assert_eq!(phan(",,", ",", 2), ds(&["", ","]));
        assert_eq!(phan(",,", ",", 3), ds(&["", "", ""]));
    }

    #[test]
    fn sep_rong_theo_nghia_rust() {
        assert_eq!(phan("abc", "", 2), ds(&["", "abc"]));
        assert_eq!(phan("abc", "", 3), ds(&["", "a", "bc"]));
        assert_eq!(phan("abc", "", 1), ds(&["abc"]));
        assert_eq!(phan("abc", "", 0), ds(&[]));
        assert_eq!(phan("abc", "", 9), ds(&["", "a", "b", "c", ""]));
        assert_eq!(phan("", "", 1), ds(&[""]));
        assert_eq!(phan("", "", 2), ds(&["", ""]));
        assert_eq!(phan("", "", 3), ds(&["", ""]));
        assert_eq!(phan("a", "", 2), ds(&["", "a"]));
        assert_eq!(phan("a", "", 3), ds(&["", "a", ""]));
        assert_eq!(phan("a", "", 9), ds(&["", "a", ""]));
    }

    #[test]
    fn unicode_theo_bien_scalar() {
        assert_eq!(phan("đ😀", "", 3), ds(&["", "đ", "😀"]));
        assert_eq!(phan("đ😀", "", 4), ds(&["", "đ", "😀", ""]));
        assert_eq!(phan("đ,😀,x", ",", 2), ds(&["đ", "😀,x"]));
        assert_eq!(phan("a😀b😀c", "😀", 2), ds(&["a", "b😀c"]));
        assert_eq!(phan("đ😀x", "😀", 2), ds(&["đ", "x"]));
        assert_eq!(phan("👨‍👩‍👧x", "👩", 2), ds(&["👨‍", "‍👧x"]));
    }
}
