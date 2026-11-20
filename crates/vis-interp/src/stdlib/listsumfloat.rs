// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (list-sum-float)** — tổng danh sách `float`: `std.list.sum_float`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//! - Cộng **trái → phải** bằng `f64`, **hạt giống `0.0`**; `xs` rỗng ⇒ `0.0`.
//! - Ngữ nghĩa **IEEE 754**, không kiểm hữu hạn, không bão hoà, **không trap**:
//!   `NaN` lan truyền; `+inf`/`-inf` lan truyền; `+inf + -inf` ⇒ `NaN`; hai số hữu hạn lớn
//!   cộng tràn ⇒ `±inf`.
//! - **Dấu `0.0`**: hạt giống là `+0.0` và IEEE (làm tròn gần nhất) cho
//!   `+0.0 + -0.0 = +0.0`, nên **mọi** tổng bằng 0 — kể cả `[-0.0]` và `[-0.0, -0.0]` —
//!   đều là `+0.0`. Hàm **không** trả `-0.0` cho đầu vào hữu hạn. Đây là khác biệt **có chủ ý**
//!   với `mean`/`median` (lấy **phần tử đầu** làm hạt giống nên `mean([-0.0])` giữ `-0.0`);
//!   muốn giữ dấu của danh sách một phần tử, dùng phương thức lõi `xs.sum()`.
//! - Phần tử không phải `float` (dữ liệu **không kiểu** tới được tầng chạy) ⇒ trap `VS0501`
//!   nêu vị trí; checker đã buộc `list[float]` nên đường thường không tới đây.
//!
//! **Sugar R5c**: `sum_float` không phải phương thức lõi của `list`, nên `xs.sum_float()` đi R5c
//! tới `std.list.sum_float` — cùng một hiện thực, không cần `use std.list`. Phương thức lõi
//! `xs.sum()` **không** đổi (vẫn nhận `list[float]` với hạt giống phần tử đầu).
//!
//! Không cấp phát danh sách trung gian: một lượt quét, một biến tích luỹ `f64`.

use vis_syntax::Span;

use crate::interp::Abort;
use crate::value::Value;

use super::chung::loi;

/// Tổng `xs` theo thứ tự **trái → phải**, hạt giống `+0.0`; `Err` là trap `VS0501` khi phần tử
/// không phải `float`.
pub(super) fn goi(xs: &[Value], span: Span) -> Result<Value, Abort> {
    let mut tong = 0.0f64;
    for (i, x) in xs.iter().enumerate() {
        let Value::Float(f) = x else {
            return Err(loi(
                format!(
                    "`list.sum_float`: phần tử {i} cần `float`, được `{}`",
                    x.show()
                ),
                span,
            ));
        };
        tong += f;
    }
    Ok(Value::Float(tong))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ds(xs: &[f64]) -> Vec<Value> {
        xs.iter().copied().map(Value::Float).collect()
    }

    /// `f64` của kết quả; panic cho hình dạng khác.
    fn goi_ok(xs: &[f64]) -> f64 {
        match goi(&ds(xs), Span::default()).expect("sum_float hợp lệ") {
            Value::Float(f) => f,
            other => panic!("phải trả float, được {other:?}"),
        }
    }

    #[test]
    fn rong_tra_zero_duong() {
        let z = goi_ok(&[]);
        assert_eq!(z, 0.0);
        assert!(z.is_sign_positive(), "hạt giống phải là +0.0");
    }

    #[test]
    fn co_ban_va_thu_tu_trai_sang_phai() {
        assert_eq!(goi_ok(&[1.5]), 1.5);
        assert_eq!(goi_ok(&[-2.5]), -2.5);
        assert_eq!(goi_ok(&[1.0, 2.0, 3.0]), 6.0);
        assert_eq!(goi_ok(&[-1.0, -2.0, -3.0]), -6.0);
        assert_eq!(goi_ok(&[-2.5, 2.5]), 0.0);
        assert_eq!(goi_ok(&[10.0, -3.0, 7.0]), 14.0);
        // Thứ tự trái → phải: `1e-16` bị nuốt khi cộng sau `1.0`, nhưng sống sót khi cộng
        // sau `1.0 - 1.0`.
        assert_eq!(goi_ok(&[1e-16, 1.0, -1.0]), 0.0);
        assert_eq!(goi_ok(&[1.0, -1.0, 1e-16]), 1e-16);
    }

    #[test]
    fn do_chinh_xac_f64() {
        assert_eq!(goi_ok(&[0.1, 0.2]), 0.1f64 + 0.2);
        assert_ne!(goi_ok(&[0.1, 0.2]), 0.3);
        assert_eq!(goi_ok(&[0.1, 0.2, 0.3]), 0.1f64 + 0.2 + 0.3);
        assert_eq!(goi_ok(&[1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0]), 1.0);
    }

    #[test]
    fn nan_va_inf_lan_truyen() {
        assert!(goi_ok(&[f64::NAN]).is_nan());
        assert!(goi_ok(&[f64::NAN, 1.0]).is_nan());
        assert!(goi_ok(&[1.0, f64::NAN]).is_nan());
        assert_eq!(goi_ok(&[f64::INFINITY, 1.0]), f64::INFINITY);
        assert_eq!(goi_ok(&[f64::NEG_INFINITY, 1.0]), f64::NEG_INFINITY);
        assert!(goi_ok(&[f64::INFINITY, f64::NEG_INFINITY]).is_nan());
        assert_eq!(goi_ok(&[f64::INFINITY, f64::INFINITY]), f64::INFINITY);
        assert_eq!(
            goi_ok(&[f64::NEG_INFINITY, f64::NEG_INFINITY]),
            f64::NEG_INFINITY
        );
        assert_eq!(goi_ok(&[1.0e308, 1.0e308]), f64::INFINITY);
    }

    #[test]
    fn zero_co_dau_theo_hat_giong_duong() {
        // `+0.0 + -0.0 = +0.0` (IEEE, làm tròn gần nhất) ⇒ mọi tổng 0 đều dương.
        for xs in [
            vec![-0.0],
            vec![-0.0, -0.0],
            vec![0.0, -0.0],
            vec![-0.0, 0.0],
            vec![-0.0, 0.0, -0.0],
        ] {
            let z = goi_ok(&xs);
            assert_eq!(z, 0.0);
            assert!(
                z.is_sign_positive(),
                "sum_float({xs:?}) phải là +0.0, được {z:?}"
            );
        }
        // Tổng khác 0 âm vẫn giữ dấu bình thường.
        assert!(goi_ok(&[-1.0, 0.0]).is_sign_negative());
    }

    #[test]
    fn phan_tu_khong_phai_float_la_vs0501() {
        for (xs, vi_tri) in [
            (vec![Value::Int(1), Value::Float(2.0)], 0),
            (vec![Value::Float(1.0), Value::Str("2".into())], 1),
            (vec![Value::Float(1.0), Value::Bool(true)], 1),
        ] {
            match goi(&xs, Span::default()) {
                Err(Abort::Trap(t)) => {
                    assert_eq!(t.code.short, "VS0501");
                    assert!(t.msg.contains(&format!("phần tử {vi_tri}")), "{}", t.msg);
                }
                other => panic!("phải trap VS0501, được {other:?}"),
            }
        }
    }
}
