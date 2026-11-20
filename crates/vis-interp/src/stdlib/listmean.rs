// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (list-mean)** — trung bình cộng danh sách `float`: `std.list.mean`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//! - `mean(xs)`: `xs` rỗng ⇒ `None` (không chia 0, không trap).
//! - Ngược lại: cộng **trái → phải** bằng `f64` rồi chia cho `len as f64` ⇒ `Some(mean)`.
//!   Phần tử đầu là **hạt giống** của phép gộp — cùng quy ước với phương thức lõi
//!   `xs.sum()` trên `list[float]`, nên `list.mean(xs)` = `xs.sum() / len` (kể cả dấu `-0.0`).
//! - Ngữ nghĩa **IEEE 754**: `NaN` đầu vào ⇒ `NaN`; `+inf`/`-inf` lan truyền;
//!   `+inf + -inf` ⇒ `NaN`; chia cho `len > 0` không trap. Không kiểm hữu hạn, không bão hoà.
//! - Dấu `0.0`: theo số học `f64` bình thường — `mean([-0.0])` ⇒ `Some(-0.0)`,
//!   `mean([0.0, -0.0])` ⇒ `Some(0.0)` (`+0.0 + -0.0 = +0.0`).
//! - Phần tử không phải `float` (dữ liệu **không kiểu** tới được tầng chạy) ⇒ trap `VS0501`
//!   nêu vị trí; checker đã buộc `list[float]` nên đường thường không tới đây.
//!
//! **Sugar R5c**: `mean` không phải phương thức lõi của `list`, nên `xs.mean()` đi R5c tới
//! `std.list.mean` — cùng một hiện thực, không cần `use std.list`.
//!
//! Không cấp phát danh sách trung gian: một lượt quét, một biến tích luỹ `f64`.

use vis_syntax::Span;

use crate::interp::Abort;
use crate::value::Value;

use super::chung::loi;

/// Trung bình cộng `xs`; `Err` là trap `VS0501` khi phần tử không phải `float`.
pub(super) fn goi(xs: &[Value], span: Span) -> Result<Value, Abort> {
    let Some((dau, con_lai)) = xs.split_first() else {
        return Ok(Value::none());
    };
    let Value::Float(mut tong) = dau else {
        return Err(loi(
            format!("`list.mean`: phần tử 0 cần `float`, được `{}`", dau.show()),
            span,
        ));
    };
    for (i, x) in con_lai.iter().enumerate() {
        let Value::Float(f) = x else {
            return Err(loi(
                format!(
                    "`list.mean`: phần tử {} cần `float`, được `{}`",
                    i + 1,
                    x.show()
                ),
                span,
            ));
        };
        tong += f;
    }
    Ok(Value::some(Value::Float(tong / xs.len() as f64)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ds(xs: &[f64]) -> Vec<Value> {
        xs.iter().copied().map(Value::Float).collect()
    }

    /// `Some(f64)` của kết quả; `None` cho danh sách rỗng; panic cho hình dạng khác.
    fn goi_ok(xs: &[f64]) -> Option<f64> {
        let v = goi(&ds(xs), Span::default()).expect("mean hợp lệ");
        match v.into_some() {
            Some(Value::Float(f)) => Some(f),
            None => None,
            Some(other) => panic!("`Some` phải bọc float, được {other:?}"),
        }
    }

    #[test]
    fn rong_tra_none() {
        assert_eq!(goi_ok(&[]), None);
    }

    #[test]
    fn mot_phan_tu_va_dau() {
        assert_eq!(goi_ok(&[1.5]), Some(1.5));
        assert_eq!(goi_ok(&[-2.5]), Some(-2.5));
    }

    #[test]
    fn duong_am_tron() {
        assert_eq!(goi_ok(&[1.0, 2.0, 3.0]), Some(2.0));
        assert_eq!(goi_ok(&[-1.0, -2.0, -3.0]), Some(-2.0));
        assert_eq!(goi_ok(&[10.0, -3.0, 7.0]), Some(14.0 / 3.0));
        assert_eq!(goi_ok(&[0.0, 0.0]), Some(0.0));
    }

    #[test]
    fn do_chinh_xac_theo_f64_tra_phep_chia() {
        // Cùng phép cộng trái → phải `f64`, không dùng tổng chính xác rồi chia lại.
        let tong = 0.1f64 + 0.2f64;
        assert_eq!(goi_ok(&[0.1, 0.2]), Some(tong / 2.0));
        assert_ne!(goi_ok(&[0.1, 0.2]), Some(0.15));
    }

    #[test]
    fn nan_va_inf_lan_truyen() {
        assert!(goi_ok(&[f64::NAN, 1.0]).unwrap().is_nan());
        assert!(goi_ok(&[1.0, f64::NAN]).unwrap().is_nan());
        assert_eq!(goi_ok(&[f64::INFINITY, 1.0]), Some(f64::INFINITY));
        assert_eq!(goi_ok(&[f64::NEG_INFINITY, 1.0]), Some(f64::NEG_INFINITY));
        // `+inf + -inf = NaN` — không trap.
        assert!(goi_ok(&[f64::INFINITY, f64::NEG_INFINITY])
            .unwrap()
            .is_nan());
    }

    #[test]
    fn zero_co_dau_theo_f64() {
        let am = goi_ok(&[-0.0]).expect("Some");
        assert_eq!(am, 0.0);
        assert!(am.is_sign_negative(), "mean([-0.0]) phải giữ dấu âm");
        let duong = goi_ok(&[0.0, -0.0]).expect("Some");
        assert_eq!(duong, 0.0);
        assert!(duong.is_sign_positive(), "+0.0 + -0.0 phải là +0.0");
    }

    #[test]
    fn phan_tu_khong_phai_float_la_vs0501() {
        for (xs, vi_tri) in [
            (vec![Value::Int(1), Value::Float(2.0)], 0),
            (vec![Value::Float(1.0), Value::Str("2".into())], 1),
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
