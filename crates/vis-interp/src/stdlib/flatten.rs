// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (flatten)** — nối các list con thành **một** list: `std.list.flatten`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//! - `xs: list[list[T]]` ⇒ `list[T]`: nối các list con **theo thứ tự**, **một cấp** duy nhất.
//! - `xs` rỗng ⇒ `[]`; list con rỗng không đóng góp phần tử nào (không sinh phần tử giả).
//! - Phần tử của list con **vốn là list** được giữ **nguyên** làm phần tử — **không** làm phẳng
//!   đệ quy: `[[[1], [2]], [[3]]]` ⇒ `[[1], [2], [3]]`.
//! - Không có trần riêng: một lượt quét, cấp phát đúng `tổng độ dài các list con`.
//!
//! `xs` do checker buộc là `list[list[T]]`; tầng chạy vẫn kiểm **động** từng phần tử phải là
//! `list` (dữ liệu không kiểu tới được đây) — sai ⇒ `Err(msg)` cho [`super::loi`] dựng trap
//! `VS0501`, không panic Rust.

use crate::value::Value;

/// Nối **một cấp** các list con của `xs`; `name` là tên đầy đủ `"list.flatten"`.
///
/// `Err` là câu lỗi (chưa kèm span) khi một phần tử không phải `Value::List`; caller bọc thành
/// trap `VS0501` với span lời gọi.
pub(super) fn goi(name: &str, xs: &[Value]) -> Result<Value, String> {
    let mut out: Vec<Value> = Vec::with_capacity(xs.iter().map(do_dai).sum());
    for x in xs {
        match x {
            Value::List(inner) => out.extend_from_slice(inner),
            other => {
                return Err(format!(
                    "`{name}`: mỗi phần tử cần `list`, được `{}`",
                    other.show()
                ));
            }
        }
    }
    Ok(Value::List(out))
}

/// Độ dài dự phòng cho cấp phát: list con ⇒ số phần tử, còn lại ⇒ 0 (sẽ báo lỗi sau).
fn do_dai(v: &Value) -> usize {
    match v {
        Value::List(inner) => inner.len(),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ds(xs: &[i64]) -> Value {
        Value::List(xs.iter().copied().map(Value::Int).collect())
    }

    fn phang(xs: Vec<Value>) -> Vec<Value> {
        match goi("list.flatten", &xs).expect("flatten hợp lệ") {
            Value::List(out) => out,
            other => panic!("phải trả list, được {other:?}"),
        }
    }

    /// `Value` không có `PartialEq` (so bằng [`crate::value::eq`] như `==` của ngôn ngữ).
    fn bang(out: &[Value], mong: &[Value]) {
        assert_eq!(out.len(), mong.len(), "độ dài lệch: {out:?} vs {mong:?}");
        for (i, (a, b)) in out.iter().zip(mong).enumerate() {
            assert!(crate::value::eq(a, b), "phần tử {i} lệch: {a:?} vs {b:?}");
        }
    }

    #[test]
    fn noi_theo_thu_tu_va_bo_list_con_rong() {
        let out = phang(vec![ds(&[1, 2]), ds(&[]), ds(&[3]), ds(&[4, 5])]);
        bang(
            &out,
            &[
                Value::Int(1),
                Value::Int(2),
                Value::Int(3),
                Value::Int(4),
                Value::Int(5),
            ],
        );
    }

    #[test]
    fn xs_rong_va_toan_list_con_rong() {
        bang(&phang(Vec::new()), &[]);
        bang(&phang(vec![ds(&[]), ds(&[])]), &[]);
    }

    #[test]
    fn chi_mot_cap_khong_de_quy() {
        // `[[[1], [2]], [[3]]]` ⇒ `[[1], [2], [3]]`: phần tử list được giữ nguyên.
        let out = phang(vec![
            Value::List(vec![ds(&[1]), ds(&[2])]),
            Value::List(vec![ds(&[3])]),
        ]);
        bang(&out, &[ds(&[1]), ds(&[2]), ds(&[3])]);
    }

    #[test]
    fn phan_tu_khong_phai_list_la_loi() {
        let e = goi("list.flatten", &[ds(&[1]), Value::Int(2)]).expect_err("phải báo lỗi");
        assert!(e.contains("list.flatten"), "câu lỗi thiếu tên hàm: {e}");
        assert!(e.contains("list"), "câu lỗi thiếu kiểu cần: {e}");
    }
}
