// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (list-contains)** — `std.list.contains(xs, value) -> bool`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//! - `true` khi có **ít nhất một** phần tử `==` `value`; `false` khi không có hoặc `xs` rỗng
//!   (không trap).
//! - So bằng [`super::indexof::tim`] — **đúng** phép so của `index_of`/`last_index_of`
//!   (`Value::eq_value`: cấu trúc, list/map/record/enum lồng nhau; `int` ↔ `float` theo tỉ lệ
//!   như `==`), nên `contains(xs, v) == index_of(xs, v).is_some()`.
//! - Quét tuyến tính, dừng ở phần tử đầu tiên bằng; không cấp phát.
//!
//! `value` **không** bị ép kiểu ở tầng chạy: checker đã buộc `T` từ `xs`; lời gọi thủ công vẫn
//! so **cấu trúc** (khác kiểu ⇒ thường `false`, không trap).

use super::indexof;
use crate::value::Value;

/// `true` iff có phần tử `==` `value`; rỗng ⇒ `false`.
pub(super) fn goi(xs: &[Value], value: &Value) -> Value {
    Value::Bool(indexof::tim(xs, value, false).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ds(xs: &[i64]) -> Vec<Value> {
        xs.iter().copied().map(Value::Int).collect()
    }

    /// `goi` trả đúng `Value::Bool(true)` (không so `Value` bằng `==` — enum không `PartialEq`).
    fn co(xs: &[Value], v: &Value) -> bool {
        matches!(goi(xs, v), Value::Bool(true))
    }

    fn khong(xs: &[Value], v: &Value) -> bool {
        matches!(goi(xs, v), Value::Bool(false))
    }

    #[test]
    fn co_va_khong_co() {
        let xs = ds(&[10, 20, 30, 20]);
        assert!(co(&xs, &Value::Int(10)));
        assert!(co(&xs, &Value::Int(20)));
        assert!(co(&xs, &Value::Int(30)));
        assert!(khong(&xs, &Value::Int(9)));
    }

    #[test]
    fn danh_sach_rong_la_false() {
        assert!(khong(&[], &Value::Int(1)));
        assert!(khong(&[], &Value::Str("x".into())));
    }

    #[test]
    fn trung_lap_khong_doi_ket_qua() {
        let xs = ds(&[7, 7, 7]);
        assert!(co(&xs, &Value::Int(7)));
        assert!(khong(&xs, &Value::Int(8)));
    }

    #[test]
    fn cau_truc_long_nhau_va_khac_kieu() {
        let xs = vec![
            Value::List(vec![Value::Int(1), Value::Int(2)]),
            Value::List(vec![Value::Int(3), Value::Int(4)]),
        ];
        assert!(co(&xs, &Value::List(vec![Value::Int(1), Value::Int(2)])));
        assert!(khong(&xs, &Value::List(vec![Value::Int(2), Value::Int(1)])));

        let a = Value::Record(
            "P".into(),
            vec![("x".into(), Value::Int(1)), ("y".into(), Value::Int(2))],
        );
        let b = Value::Record(
            "P".into(),
            vec![("y".into(), Value::Int(2)), ("x".into(), Value::Int(1))],
        );
        assert!(co(&[a], &b));

        // Khác kiểu (`int` vs `str`) chỉ là `false`, không trap.
        assert!(khong(&ds(&[1, 2]), &Value::Str("1".into())));
    }

    #[test]
    fn khop_voi_index_of() {
        let xs = ds(&[1, 2, 3]);
        for v in [Value::Int(1), Value::Int(2), Value::Int(9)] {
            let tim = indexof::tim(&xs, &v, false).is_some();
            assert_eq!(co(&xs, &v), tim, "contains ≠ index_of cho {v:?}");
        }
    }
}
