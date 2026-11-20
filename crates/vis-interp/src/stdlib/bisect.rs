// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (binary-search)** — tìm kiếm nhị phân trong danh sách **đã sắp tăng**:
//! `std.list.binary_search`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//! - **Điều kiện tiên quyết**: `xs` đã sắp **tăng dần** theo **cùng** thứ tự `list.sort`/
//!   `list.min`/`list.max` (so bằng [`Value::cmp_value`]). Hàm **không** kiểm lại thứ tự:
//!   danh sách chưa sắp là **hành vi không xác định** — vẫn chạy, không trap, nhưng kết quả có
//!   thể `None` dù phần tử có mặt hoặc trả một vị trí bất kỳ.
//! - Trả **`Some(i)`** với `i` là **biên dưới** (lower-bound): vị trí **đầu tiên** trong dãy
//!   phần tử bằng `value`. Có trùng lặp ⇒ luôn là vị trí **đầu** của cụm bằng nhau; không có
//!   ⇒ `None`; danh sách rỗng ⇒ `None` (không trap).
//! - **Thuật toán tất định**: vòng `while lo < hi`, `mid = lo + (hi - lo) / 2`; nếu
//!   `xs[mid] < value` thì `lo = mid + 1`, ngược lại (bằng **hoặc** lớn hơn) `hi = mid`.
//!   Kết thúc kiểm `xs[lo] == value` theo **thứ tự** (`cmp_value == Equal`), không dùng `eq`
//!   cấu trúc — nhờ vậy `int` ↔ `float` so được như `list.sort` (`1` và `1.0` là cùng một giá
//!   trị theo thứ tự).
//! - Phần tử **không so được** (NaN hoặc kiểu chưa có thứ tự như list/bản ghi) tại một điểm
//!   giữa ⇒ `None` ngay, **không** trap: điều kiện "sắp theo thứ tự" đã không thể đúng, nên
//!   không có vị trí đáng tin để trả.
//! - Chi phí `O(log n)` phép so, không cấp phát.

use crate::value::Value;

/// Vị trí **biên dưới** của `value` trong `xs` (đã sắp tăng theo `cmp_value`); `None` nếu
/// không có hoặc gặp phần tử không so được.
pub(super) fn tim(xs: &[Value], value: &Value) -> Option<usize> {
    let mut lo = 0usize;
    let mut hi = xs.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        match xs[mid].cmp_value(value)? {
            std::cmp::Ordering::Less => lo = mid + 1,
            // Bằng **hoặc** lớn hơn đều thu hẹp về nửa trái để tìm **biên dưới**.
            std::cmp::Ordering::Equal | std::cmp::Ordering::Greater => hi = mid,
        }
    }
    // `lo` là vị trí chèn đầu tiên; chỉ là khớp khi phần tử tại đó **bằng** `value`.
    (lo < xs.len() && xs[lo].cmp_value(value) == Some(std::cmp::Ordering::Equal)).then_some(lo)
}

/// Dispatch chung: `xs` đã sắp tăng, trả `Some(vị trí biên dưới)` hoặc `None`.
pub(super) fn goi(xs: &[Value], value: &Value) -> Value {
    Value::opt(tim(xs, value).map(|i| Value::Int(i as i64)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ds(xs: &[i64]) -> Vec<Value> {
        xs.iter().copied().map(Value::Int).collect()
    }

    #[test]
    fn dau_giua_cuoi() {
        let xs = ds(&[10, 20, 30, 40, 50]);
        assert_eq!(tim(&xs, &Value::Int(10)), Some(0));
        assert_eq!(tim(&xs, &Value::Int(30)), Some(2));
        assert_eq!(tim(&xs, &Value::Int(50)), Some(4));
    }

    #[test]
    fn khong_co_va_rong() {
        let xs = ds(&[10, 20, 30]);
        assert_eq!(tim(&xs, &Value::Int(5)), None);
        assert_eq!(tim(&xs, &Value::Int(35)), None);
        assert_eq!(tim(&xs, &Value::Int(25)), None);
        assert_eq!(tim(&[], &Value::Int(1)), None);
    }

    #[test]
    fn trung_lap_tra_bien_duoi() {
        let xs = ds(&[1, 2, 2, 2, 3]);
        assert_eq!(tim(&xs, &Value::Int(2)), Some(1));
        assert_eq!(tim(&ds(&[2, 2, 2]), &Value::Int(2)), Some(0));
        assert_eq!(tim(&ds(&[1, 2, 2]), &Value::Int(3)), None);
    }

    #[test]
    fn so_am_va_str() {
        let am = ds(&[-9, -5, -5, -1, 0]);
        assert_eq!(tim(&am, &Value::Int(-5)), Some(1));
        assert_eq!(tim(&am, &Value::Int(-6)), None);
        let chu = vec![
            Value::Str("a".into()),
            Value::Str("b".into()),
            Value::Str("b".into()),
        ];
        assert_eq!(tim(&chu, &Value::Str("b".into())), Some(1));
        assert_eq!(tim(&chu, &Value::Str("c".into())), None);
    }

    #[test]
    fn khong_so_duoc_tra_none_khong_trap() {
        // List lồng nhau không có thứ tự trong `cmp_value` ⇒ `None`, không panic.
        let xs = vec![Value::List(vec![Value::Int(1)])];
        assert_eq!(tim(&xs, &Value::List(vec![Value::Int(1)])), None);
    }

    #[test]
    fn ket_qua_la_option_that() {
        let v = goi(&ds(&[5]), &Value::Int(5));
        assert!(v.is_some());
        assert!(matches!(v.opt_payload(), Some(Value::Int(0))));
        let n = goi(&ds(&[5]), &Value::Int(6));
        assert!(n.is_none());
    }
}
