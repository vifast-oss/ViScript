// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (indexof)** — vị trí phần tử trong danh sách: `std.list.index_of` /
//! `std.list.last_index_of`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//! - Chỉ số **0-based**; so bằng [`eq`] — **cùng** ngữ nghĩa `==` (cấu trúc: list/map/record/
//!   enum lồng nhau, `int` ↔ `float` theo tỉ lệ như `==`).
//! - `index_of` trả vị trí **đầu tiên**, `last_index_of` vị trí **cuối cùng**; không có phần
//!   tử bằng `value` ⇒ `None`; danh sách rỗng ⇒ `None` (không trap).
//! - Quét tuyến tính, không cấp phát danh sách kết quả; không có trần riêng.
//!
//! `value` **không** bị ép kiểu ở tầng chạy: checker đã buộc `T` từ `xs`; lời gọi thủ công vẫn
//! so **cấu trúc** và trả `None` khi không bằng.

use crate::value::{eq, Value};

/// Vị trí **đầu tiên** (`cuoi == false`) hoặc **cuối cùng** (`cuoi == true`) có phần tử `==`
/// `value`; `None` nếu không có (kể cả danh sách rỗng).
pub(super) fn tim(xs: &[Value], value: &Value, cuoi: bool) -> Option<usize> {
    if cuoi {
        xs.iter().rposition(|x| eq(x, value))
    } else {
        xs.iter().position(|x| eq(x, value))
    }
}

/// Dispatch chung; `name` là `"list.index_of"` hoặc `"list.last_index_of"`.
pub(super) fn goi(name: &str, xs: &[Value], value: &Value) -> Value {
    let i = tim(xs, value, name == "list.last_index_of");
    Value::opt(i.map(|i| Value::Int(i as i64)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ds(xs: &[i64]) -> Vec<Value> {
        xs.iter().copied().map(Value::Int).collect()
    }

    #[test]
    fn dau_cuoi_va_trung_lap() {
        let xs = ds(&[10, 20, 30, 20]);
        assert_eq!(tim(&xs, &Value::Int(20), false), Some(1));
        assert_eq!(tim(&xs, &Value::Int(20), true), Some(3));
        assert_eq!(tim(&xs, &Value::Int(10), true), Some(0));
        assert_eq!(tim(&xs, &Value::Int(30), false), Some(2));
    }

    #[test]
    fn khong_co_va_danh_sach_rong() {
        assert_eq!(tim(&ds(&[1, 2, 3]), &Value::Int(9), false), None);
        assert_eq!(tim(&ds(&[1, 2, 3]), &Value::Int(9), true), None);
        assert_eq!(tim(&[], &Value::Int(1), false), None);
        assert_eq!(tim(&[], &Value::Int(1), true), None);
    }

    #[test]
    fn so_cau_truc_long_nhau() {
        // Cùng `eq_value` với `==`: list lồng nhau và record so theo **trường**, không theo
        // thứ tự khai báo.
        let xs = vec![
            Value::List(vec![Value::Int(1), Value::Int(2)]),
            Value::List(vec![Value::Int(3), Value::Int(4)]),
            Value::List(vec![Value::Int(1), Value::Int(2)]),
        ];
        let can = Value::List(vec![Value::Int(1), Value::Int(2)]);
        assert_eq!(tim(&xs, &can, false), Some(0));
        assert_eq!(tim(&xs, &can, true), Some(2));

        let a = Value::Record(
            "P".into(),
            vec![("x".into(), Value::Int(1)), ("y".into(), Value::Int(2))],
        );
        let b = Value::Record(
            "P".into(),
            vec![("y".into(), Value::Int(2)), ("x".into(), Value::Int(1))],
        );
        assert_eq!(tim(&[a], &b, false), Some(0));
    }

    #[test]
    fn ket_qua_la_option_that() {
        let v = goi("list.index_of", &ds(&[5]), &Value::Int(5));
        assert!(v.is_some());
        assert!(matches!(v.opt_payload(), Some(Value::Int(0))));
        let n = goi("list.last_index_of", &ds(&[5]), &Value::Int(6));
        assert!(n.is_none());
    }
}
