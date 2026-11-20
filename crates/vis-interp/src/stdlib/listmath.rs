// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (list-sum-product)** — gộp danh sách `int` bằng phép **có kiểm tràn**:
//! `std.list.sum` / `std.list.product`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//! - `sum(xs)`: gộp **trái → phải** bằng `+` checked; `xs` rỗng ⇒ `0` (phần tử đơn vị).
//! - `product(xs)`: gộp **trái → phải** bằng `*` checked; `xs` rỗng ⇒ `1`.
//! - Tràn `i64` ở **bất kỳ bước nào** ⇒ trap `VS0507` (đúng mã của phép `+`/`*`), **không
//!   wrap**, không bỏ qua bước tràn dù kết quả cuối biểu diễn được:
//!   `sum([MAX, 1, -1])` trap tại `MAX + 1`; `product([MAX, 2, 0])` trap tại `MAX * 2`.
//! - Phần tử không phải `int` (dữ liệu **không kiểu** tới được tầng chạy) ⇒ trap `VS0501`
//!   nêu vị trí; checker đã buộc `list[int]` nên đường thường không tới đây.
//!
//! **Tương tác `xs.sum()` (R5c)**: `sum` là **phương thức lõi** của `list` nên
//! [`crate::builtin::method`] xử lý trước R5c — `xs.sum()` **không** chuyển thành
//! `std.list.sum`. Với `list[int]`, phương thức lõi cộng bằng đúng `bin(Add)` checked và rỗng
//! ⇒ `0`, nên **cùng ngữ nghĩa** với `list.sum`; nó **vẫn** nhận `list[float]` (tổng float)
//! như trước (`test/sem/list/sum-float.vis`). `product` **không** có phương thức lõi nên
//! `xs.product()` đi R5c tới `list.product` (chỉ `list[int]`). Giữ nguyên phân chia này để
//! không hồi quy `sum()` lõi; muốn một ngữ nghĩa duy nhất cho mọi list số cần VD riêng.
//!
//! Không cấp phát danh sách trung gian: một lượt quét, một biến tích luỹ `i64`.

use vis_syntax::Span;

use crate::interp::{Abort, Trap, OVERFLOW};
use crate::value::Value;

use super::chung::loi;

/// Gộp `xs` bằng `+`/`*` checked; `name` là `"list.sum"` hoặc `"list.product"`.
///
/// `Err` là trap: `VS0507` khi tràn (kể cả tràn trung gian), `VS0501` khi phần tử không phải
/// `int`. Không có kết quả nửa vời — trap huỷ toàn bộ lời gọi.
pub(super) fn goi(name: &str, xs: &[Value], span: Span) -> Result<Value, Abort> {
    let tich = name == "list.product";
    let mut acc: i64 = if tich { 1 } else { 0 };
    for (i, x) in xs.iter().enumerate() {
        let Value::Int(n) = x else {
            return Err(loi(
                format!("`{name}`: phần tử {i} cần `int`, được `{}`", x.show()),
                span,
            ));
        };
        let moi = if tich {
            acc.checked_mul(*n)
        } else {
            acc.checked_add(*n)
        };
        acc = moi.ok_or_else(|| {
            Abort::Trap(Trap::new(
                OVERFLOW,
                format!(
                    "`{name}`: tràn i64 khi {} dồn (VD-270)",
                    if tich { "nhân" } else { "cộng" }
                ),
                span,
            ))
        })?;
    }
    Ok(Value::Int(acc))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ds(xs: &[i64]) -> Vec<Value> {
        xs.iter().copied().map(Value::Int).collect()
    }

    fn goi_ok(name: &str, xs: &[i64]) -> i64 {
        match goi(name, &ds(xs), Span::default()).expect("gộp hợp lệ") {
            Value::Int(n) => n,
            other => panic!("phải trả int, được {other:?}"),
        }
    }

    /// Mã của trap đầu tiên; panic nếu không phải trap (để ca âm không lọt).
    fn ma_trap(name: &str, xs: &[i64]) -> &'static str {
        match goi(name, &ds(xs), Span::default()) {
            Err(Abort::Trap(t)) => t.code.short,
            other => panic!("phải trap, được {other:?}"),
        }
    }

    #[test]
    fn rong_va_co_ban() {
        assert_eq!(goi_ok("list.sum", &[]), 0);
        assert_eq!(goi_ok("list.product", &[]), 1);
        assert_eq!(goi_ok("list.sum", &[1, 2, 3]), 6);
        assert_eq!(goi_ok("list.product", &[1, 2, 3]), 6);
        assert_eq!(goi_ok("list.sum", &[-4, -6]), -10);
        assert_eq!(goi_ok("list.product", &[-4, -6]), 24);
        assert_eq!(goi_ok("list.sum", &[10, -3, 7]), 14);
        assert_eq!(goi_ok("list.product", &[10, -3, 7]), -210);
        assert_eq!(goi_ok("list.product", &[5, 0, -5]), 0);
    }

    #[test]
    fn bien_i64_khong_tran() {
        let max = i64::MAX;
        let min = i64::MIN;
        assert_eq!(goi_ok("list.sum", &[max, 0]), max);
        assert_eq!(goi_ok("list.sum", &[min, 0]), min);
        assert_eq!(goi_ok("list.sum", &[min, max]), -1);
        assert_eq!(goi_ok("list.product", &[max, 1]), max);
        assert_eq!(goi_ok("list.product", &[min, 1]), min);
        assert_eq!(goi_ok("list.product", &[max, -1, 0]), 0);
        assert_eq!(goi_ok("list.product", &[0, max]), 0);
    }

    /// Tràn **trung gian** cũng trap: kết quả toán học đẹp không cứu được bước đã tràn.
    #[test]
    fn tran_cong_va_nhan_la_vs0507() {
        assert_eq!(ma_trap("list.sum", &[i64::MAX, 1]), "VS0507");
        assert_eq!(ma_trap("list.sum", &[i64::MIN, -1]), "VS0507");
        assert_eq!(ma_trap("list.sum", &[i64::MAX, 1, -1]), "VS0507");
        assert_eq!(ma_trap("list.product", &[i64::MAX, 2]), "VS0507");
        assert_eq!(ma_trap("list.product", &[i64::MIN, -1]), "VS0507");
        assert_eq!(ma_trap("list.product", &[i64::MAX, 2, 0]), "VS0507");
        assert_eq!(
            ma_trap("list.product", &[3_037_000_500, 3_037_000_500]),
            "VS0507"
        );
    }

    /// Dữ liệu không kiểu: phần tử lạ ⇒ `VS0501` (không panic Rust, không bỏ qua).
    #[test]
    fn phan_tu_khong_phai_int_la_vs0501() {
        let xs = vec![Value::Int(1), Value::Str("2".into())];
        match goi("list.sum", &xs, Span::default()) {
            Err(Abort::Trap(t)) => {
                assert_eq!(t.code.short, "VS0501");
                assert!(t.msg.contains("phần tử 1"), "{}", t.msg);
            }
            other => panic!("phải trap VS0501, được {other:?}"),
        }
    }
}
