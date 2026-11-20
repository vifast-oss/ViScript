// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (list-median)** — trung vị danh sách `float`: `std.list.median`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//! - `xs` rỗng ⇒ `None` (không có phần tử giữa, không trap).
//! - Ngược lại: **sao chép** các phần tử thành snapshot `f64` rồi sắp **tăng** bằng
//!   [`f64::total_cmp`] (thứ tự toàn phần, tất định) — danh sách gốc **không** bị đổi.
//! - Số phần tử **lẻ** ⇒ phần tử giữa (chỉ số `len / 2` sau khi sắp), giữ nguyên dấu `-0.0`.
//! - Số phần tử **chẵn** ⇒ trung bình cộng `f64` của hai phần tử giữa: đúng công thức
//!   `(a + b) / 2.0`. Hệ quả số học `f64` (không trap, không bão hoà):
//!   `[-0.0, 0.0]` ⇒ `+0.0`; hai số hữu hạn **cùng dấu cực lớn** có thể tràn thành `±inf`
//!   (`median([1e308, 1e308])` ⇒ `Some(inf)`); cặp giữa `[-inf, +inf]` ⇒ `NaN`.
//! - **`NaN` đầu vào** ở **bất kỳ** vị trí ⇒ `None` — kiểm **trước** khi sắp nên kết quả
//!   **tất định**, không phụ thuộc thứ tự/`total_cmp`; không trap. `±inf` **không** phải `NaN`
//!   nên sắp bình thường (`-inf` < hữu hạn < `+inf`).
//! - Phần tử không phải `float` (dữ liệu **không kiểu** tới được tầng chạy) ⇒ trap `VS0501`
//!   nêu vị trí; **quét hết** phần tử trước khi xét `NaN`, nên `[NaN, "x"]` vẫn trap ở phần tử 1
//!   thay vì trả `None` sớm. Checker đã buộc `list[float]` nên đường thường không tới đây.
//!
//! **Sugar R5c**: `median` không phải phương thức lõi của `list`, nên `xs.median()` đi R5c tới
//! `std.list.median` — cùng một hiện thực, không cần `use std.list`.
//!
//! Cấp phát **một** `Vec<f64>` cỡ `len` (snapshot để sắp); không đổi giá trị `Value` nào.

use vis_syntax::Span;

use crate::interp::Abort;
use crate::value::Value;

use super::chung::loi;

/// Trung vị `xs`; `Err` là trap `VS0501` khi phần tử không phải `float`.
pub(super) fn goi(xs: &[Value], span: Span) -> Result<Value, Abort> {
    if xs.is_empty() {
        return Ok(Value::none());
    }
    let mut so: Vec<f64> = Vec::with_capacity(xs.len());
    let mut co_nan = false;
    for (i, x) in xs.iter().enumerate() {
        let Value::Float(f) = x else {
            return Err(loi(
                format!(
                    "`list.median`: phần tử {i} cần `float`, được `{}`",
                    x.show()
                ),
                span,
            ));
        };
        co_nan |= f.is_nan();
        so.push(*f);
    }
    // `NaN` đầu vào ⇒ `None` **tất định**; kiểm sau khi đã quét hết để lỗi kiểu phần tử
    // (nếu có) luôn thắng, không phụ thuộc vị trí `NaN`.
    if co_nan {
        return Ok(Value::none());
    }
    so.sort_unstable_by(|a, b| a.total_cmp(b));
    let n = so.len();
    let giua = n / 2;
    let med = if n % 2 == 1 {
        so[giua]
    } else {
        (so[giua - 1] + so[giua]) / 2.0
    };
    Ok(Value::some(Value::Float(med)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ds(xs: &[f64]) -> Vec<Value> {
        xs.iter().copied().map(Value::Float).collect()
    }

    /// `Some(f64)` của kết quả; `None` cho danh sách rỗng/`NaN`; panic cho hình dạng khác.
    fn goi_ok(xs: &[f64]) -> Option<f64> {
        let v = goi(&ds(xs), Span::default()).expect("median hợp lệ");
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
    fn le_lay_phan_tu_giua_sau_khi_sap() {
        assert_eq!(goi_ok(&[1.5]), Some(1.5));
        assert_eq!(goi_ok(&[-2.5]), Some(-2.5));
        assert_eq!(goi_ok(&[3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(goi_ok(&[5.0, 9.0, 1.0, 7.0, 3.0]), Some(5.0));
        assert_eq!(goi_ok(&[2.0, 1.0, 1.0]), Some(1.0));
        assert_eq!(goi_ok(&[1.0, 2.0, 2.0]), Some(2.0));
    }

    #[test]
    fn chan_lay_trung_binh_cong_hai_phan_tu_giua() {
        assert_eq!(goi_ok(&[1.0, 2.0]), Some(1.5));
        assert_eq!(goi_ok(&[4.0, 1.0, 3.0, 2.0]), Some(2.5));
        assert_eq!(goi_ok(&[-4.0, -3.0, -2.0, -1.0]), Some(-2.5));
        assert_eq!(goi_ok(&[2.0, 2.0, 2.0, 2.0]), Some(2.0));
        // Đúng `(0.1 + 0.2) / 2.0` trong `f64`, không format lại thành `0.15`.
        assert_eq!(goi_ok(&[0.1, 0.2]), Some((0.1f64 + 0.2) / 2.0));
        assert_ne!(goi_ok(&[0.1, 0.2]), Some(0.15));
        assert_eq!(goi_ok(&[1.0 / 3.0, 1.0 / 3.0]), Some(1.0 / 3.0));
    }

    #[test]
    fn nan_o_moi_vi_tri_tra_none() {
        assert_eq!(goi_ok(&[f64::NAN]), None);
        assert_eq!(goi_ok(&[f64::NAN, 1.0]), None);
        assert_eq!(goi_ok(&[1.0, f64::NAN, 2.0]), None);
        assert_eq!(goi_ok(&[1.0, 2.0, 3.0, f64::NAN]), None);
        // `NaN` không lọt vào `sort_unstable_by` — kết quả không phụ thuộc `total_cmp`.
        assert_eq!(goi_ok(&[f64::NAN, f64::INFINITY, f64::NEG_INFINITY]), None);
    }

    #[test]
    fn inf_sap_binh_thuong_con_hai_dau_co_the_ra_nan() {
        // `±inf` sắp đúng hai đầu: `[inf, 1, 2]` ⇒ giữa là `2`, `[1, inf, -inf]` ⇒ giữa là `1`.
        assert_eq!(goi_ok(&[f64::INFINITY, 1.0, 2.0]), Some(2.0));
        assert_eq!(goi_ok(&[1.0, f64::INFINITY, f64::NEG_INFINITY]), Some(1.0));
        assert_eq!(
            goi_ok(&[f64::INFINITY, f64::INFINITY, 1.0]),
            Some(f64::INFINITY)
        );
        assert_eq!(
            goi_ok(&[f64::NEG_INFINITY, f64::NEG_INFINITY, 1.0]),
            Some(f64::NEG_INFINITY)
        );
        assert_eq!(goi_ok(&[1.0, f64::INFINITY]), Some(f64::INFINITY));
        assert_eq!(goi_ok(&[f64::NEG_INFINITY, 1.0]), Some(f64::NEG_INFINITY));
        assert_eq!(goi_ok(&[f64::INFINITY, f64::INFINITY]), Some(f64::INFINITY));
        assert_eq!(
            goi_ok(&[f64::NEG_INFINITY, f64::NEG_INFINITY]),
            Some(f64::NEG_INFINITY)
        );
        // Số học `(a + b) / 2.0`: `-inf + inf = NaN`, không phải `NaN` đầu vào.
        assert!(goi_ok(&[f64::NEG_INFINITY, f64::INFINITY])
            .unwrap()
            .is_nan());
    }

    #[test]
    fn dau_zero_theo_f64() {
        let am = goi_ok(&[-0.0]).expect("Some");
        assert_eq!(am, 0.0);
        assert!(am.is_sign_negative(), "median([-0.0]) phải giữ dấu âm");
        // `total_cmp` xếp `-0.0` trước `+0.0`; cặp giữa cộng lại thành `+0.0`.
        let duong = goi_ok(&[-0.0, 0.0]).expect("Some");
        assert_eq!(duong, 0.0);
        assert!(duong.is_sign_positive(), "-0.0 + 0.0 phải là +0.0");
    }

    #[test]
    fn hai_so_huu_han_cuc_lon_tran_thanh_inf() {
        // Hệ quả của đúng công thức `(a + b) / 2.0`: `1e308 + 1e308` tràn `f64` ⇒ `inf`.
        assert_eq!(goi_ok(&[1.0e308, 1.0e308]), Some(f64::INFINITY));
    }

    #[test]
    fn phan_tu_khong_phai_float_la_vs0501() {
        for (xs, vi_tri) in [
            (vec![Value::Int(1), Value::Float(2.0)], 0),
            (vec![Value::Float(1.0), Value::Str("2".into())], 1),
            // Lỗi kiểu thắng `NaN`: quét hết phần tử trước khi xét `NaN`.
            (vec![Value::Float(f64::NAN), Value::Str("2".into())], 1),
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
