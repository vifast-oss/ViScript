// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (list-minmax-float)** — nhỏ nhất/lớn nhất danh sách `float`:
//! `std.list.min_float` và `std.list.max_float`.
//!
//! Hợp đồng **dùng chung** cho hai hàm (một hiện thực, interp + VM):
//! - `xs` rỗng ⇒ `None` (không có cực trị, không trap).
//! - Ngược lại trả `Some(giá_trị_phần_tử)`: **đúng giá trị đã lưu**, không tính lại, nên
//!   `-0.0`/`+0.0` được giữ nguyên bit khi nó là cực trị **duy nhất**.
//! - **`NaN` đầu vào** ở **bất kỳ** vị trí ⇒ `None` — cùng chính sách tất định với
//!   `list.median` (không phụ thuộc thứ tự/so sánh từng phần), không trap.
//! - `±inf` **không** phải `NaN` nên so bình thường: `-inf` là nhỏ nhất, `+inf` là lớn nhất.
//! - So sánh bằng **đúng** toán tử IEEE `f64` (`<` cho `min_float`, `>` cho `max_float`) và
//!   chỉ **thay** khi nghiêm ngặt hơn, nên phần tử **bằng nhau** — kể cả cặp `-0.0`/`+0.0`
//!   (chúng bằng nhau theo `==`) — giữ **phần tử đầu tiên** theo thứ tự danh sách. Hệ quả:
//!   `min_float([0.0, -0.0])` ⇒ `Some(0)` (dấu **dương**, phần tử đầu), còn
//!   `min_float([-0.0, 0.0])` ⇒ `Some(-0)` (dấu **âm**, phần tử đầu); muốn phân biệt hai
//!   dấu zero, hợp đồng là **vị trí đầu** chứ không phải `total_cmp`.
//! - Phần tử không phải `float` (dữ liệu **không kiểu** tới được tầng chạy) ⇒ trap `VS0501`
//!   nêu vị trí; **quét hết** phần tử trước khi trả `None` vì `NaN`, nên `[NaN, "x"]` vẫn
//!   trap ở phần tử 1 thay vì trả `None` sớm. Checker đã buộc `list[float]` nên đường thường
//!   không tới đây.
//!
//! **Sugar R5c**: `min_float`/`max_float` không phải phương thức lõi của `list` (không đụng
//! `list.min`/`list.max` nhận `list[T]`), nên `xs.min_float()`/`xs.max_float()` đi R5c tới
//! `std.list.*` — cùng một hiện thực, không cần `use std.list`.
//!
//! Không cấp phát: một lượt quét, một biến tích luỹ `f64` và cờ `NaN`.

use vis_syntax::Span;

use crate::interp::Abort;
use crate::value::Value;

use super::chung::loi;

/// Cực trị `xs` theo `name` (`list.min_float`/`list.max_float`); `Err` là trap `VS0501` khi
/// phần tử không phải `float`.
pub(super) fn goi(name: &str, xs: &[Value], span: Span) -> Result<Value, Abort> {
    let la_min = name == "list.min_float";
    let mut chon: Option<f64> = None;
    let mut co_nan = false;
    for (i, x) in xs.iter().enumerate() {
        let Value::Float(f) = x else {
            return Err(loi(
                format!("`{name}`: phần tử {i} cần `float`, được `{}`", x.show()),
                span,
            ));
        };
        co_nan |= f.is_nan();
        match chon {
            // Chỉ thay khi **nghiêm ngặt** hơn ⇒ phần tử bằng nhau (kể cả `±0.0`) giữ phần
            // tử **đầu** theo thứ tự danh sách.
            Some(c) if (la_min && *f < c) || (!la_min && *f > c) => chon = Some(*f),
            Some(_) => {}
            None => chon = Some(*f),
        }
    }
    // `NaN` đầu vào ⇒ `None` **tất định**; kiểm sau khi đã quét hết để lỗi kiểu phần tử (nếu
    // có) luôn thắng, không phụ thuộc vị trí `NaN`.
    if co_nan {
        return Ok(Value::none());
    }
    Ok(match chon {
        Some(f) => Value::some(Value::Float(f)),
        None => Value::none(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ds(xs: &[f64]) -> Vec<Value> {
        xs.iter().copied().map(Value::Float).collect()
    }

    /// `Some(f64)` của kết quả; `None` cho danh sách rỗng/`NaN`; panic cho hình dạng khác.
    fn goi_ok(name: &str, xs: &[f64]) -> Option<f64> {
        let v = goi(name, &ds(xs), Span::default()).expect("min/max hợp lệ");
        match v.into_some() {
            Some(Value::Float(f)) => Some(f),
            None => None,
            Some(other) => panic!("`Some` phải bọc float, được {other:?}"),
        }
    }

    #[test]
    fn rong_tra_none_ca_hai() {
        assert_eq!(goi_ok("list.min_float", &[]), None);
        assert_eq!(goi_ok("list.max_float", &[]), None);
    }

    #[test]
    fn mot_phan_tu_tra_chinh_no() {
        assert_eq!(goi_ok("list.min_float", &[1.5]), Some(1.5));
        assert_eq!(goi_ok("list.max_float", &[1.5]), Some(1.5));
        assert_eq!(goi_ok("list.min_float", &[-2.5]), Some(-2.5));
        assert_eq!(goi_ok("list.max_float", &[-2.5]), Some(-2.5));
    }

    #[test]
    fn co_ban_va_khong_can_sap() {
        assert_eq!(goi_ok("list.min_float", &[3.0, 1.0, 2.0]), Some(1.0));
        assert_eq!(goi_ok("list.max_float", &[3.0, 1.0, 2.0]), Some(3.0));
        assert_eq!(goi_ok("list.min_float", &[-1.0, -5.0, -3.0]), Some(-5.0));
        assert_eq!(goi_ok("list.max_float", &[-1.0, -5.0, -3.0]), Some(-1.0));
        assert_eq!(
            goi_ok("list.min_float", &[10.0, -2.5, 3.5, 0.0]),
            Some(-2.5)
        );
        assert_eq!(
            goi_ok("list.max_float", &[10.0, -2.5, 3.5, 0.0]),
            Some(10.0)
        );
        assert_eq!(goi_ok("list.min_float", &[2.0, 2.0, 2.0]), Some(2.0));
        assert_eq!(goi_ok("list.max_float", &[2.0, 2.0, 2.0]), Some(2.0));
    }

    #[test]
    fn bang_nhau_giu_phan_tu_dau() {
        // Ba phần tử bằng nhau: cả hai trả **đầu tiên** (giá trị như nhau nên chỉ khoá vị trí
        // qua độ dài danh sách — dấu được kiểm riêng ở test zero).
        assert_eq!(goi_ok("list.min_float", &[7.0, 7.0, 7.0]), Some(7.0));
        assert_eq!(goi_ok("list.max_float", &[7.0, 7.0, 7.0]), Some(7.0));
        // Cực trị duy nhất vẫn đúng dù xuất hiện ở cuối.
        assert_eq!(goi_ok("list.min_float", &[7.0, 7.0, 6.5]), Some(6.5));
        assert_eq!(goi_ok("list.max_float", &[7.0, 7.0, 7.5]), Some(7.5));
    }

    #[test]
    fn nan_o_moi_vi_tri_tra_none() {
        for xs in [
            vec![f64::NAN],
            vec![f64::NAN, 1.0],
            vec![1.0, f64::NAN],
            vec![1.0, f64::NAN, 2.0],
            vec![1.0, 2.0, 3.0, f64::NAN],
            vec![f64::NEG_INFINITY, f64::NAN, f64::INFINITY],
        ] {
            assert_eq!(goi_ok("list.min_float", &xs), None, "min_float({xs:?})");
            assert_eq!(goi_ok("list.max_float", &xs), None, "max_float({xs:?})");
        }
    }

    #[test]
    fn inf_so_binh_thuong() {
        assert_eq!(
            goi_ok("list.min_float", &[1.0, f64::INFINITY, 2.0]),
            Some(1.0)
        );
        assert_eq!(
            goi_ok("list.max_float", &[1.0, f64::INFINITY, 2.0]),
            Some(f64::INFINITY)
        );
        assert_eq!(
            goi_ok("list.min_float", &[1.0, f64::NEG_INFINITY, 2.0]),
            Some(f64::NEG_INFINITY)
        );
        assert_eq!(
            goi_ok("list.max_float", &[1.0, f64::NEG_INFINITY, 2.0]),
            Some(2.0)
        );
        assert_eq!(
            goi_ok("list.min_float", &[f64::INFINITY, f64::NEG_INFINITY]),
            Some(f64::NEG_INFINITY)
        );
        assert_eq!(
            goi_ok("list.max_float", &[f64::INFINITY, f64::NEG_INFINITY]),
            Some(f64::INFINITY)
        );
        assert_eq!(
            goi_ok("list.min_float", &[f64::INFINITY, f64::INFINITY]),
            Some(f64::INFINITY)
        );
        assert_eq!(
            goi_ok("list.max_float", &[f64::NEG_INFINITY, f64::NEG_INFINITY]),
            Some(f64::NEG_INFINITY)
        );
    }

    #[test]
    fn zero_giu_dau_va_vi_tri_dau() {
        // `-0.0` là cực trị **duy nhất** (không có `+0.0`) ⇒ giữ dấu âm.
        let am = goi_ok("list.min_float", &[1.0, -0.0]).expect("Some");
        assert_eq!(am, 0.0);
        assert!(am.is_sign_negative(), "min_float([1.0, -0.0]) phải là -0.0");
        let am = goi_ok("list.max_float", &[-1.0, -0.0]).expect("Some");
        assert!(
            am.is_sign_negative(),
            "max_float([-1.0, -0.0]) phải là -0.0"
        );
        // `+0.0` là cực trị duy nhất ⇒ giữ dấu dương.
        let duong = goi_ok("list.max_float", &[-1.0, 0.0]).expect("Some");
        assert!(duong.is_sign_positive());
        let duong = goi_ok("list.min_float", &[0.0, 1.0]).expect("Some");
        assert!(duong.is_sign_positive());
        // Có **cả hai** dấu zero ⇒ trả **phần tử đầu** theo thứ tự danh sách (cả min lẫn max),
        // vì `-0.0 == 0.0` nên không bên nào "nghiêm ngặt hơn".
        for name in ["list.min_float", "list.max_float"] {
            let dau = goi_ok(name, &[-0.0, 0.0]).expect("Some");
            assert!(dau.is_sign_negative(), "{name}([-0.0, 0.0]) phải là -0.0");
            let dau = goi_ok(name, &[0.0, -0.0]).expect("Some");
            assert!(dau.is_sign_positive(), "{name}([0.0, -0.0]) phải là +0.0");
        }
        // Zero không phải cực trị thì dấu zero không ảnh hưởng kết quả.
        assert_eq!(goi_ok("list.min_float", &[0.0, -0.0, -1.0]), Some(-1.0));
        assert_eq!(goi_ok("list.max_float", &[0.0, -0.0, 1.0]), Some(1.0));
    }

    #[test]
    fn phan_tu_khong_phai_float_la_vs0501() {
        for (xs, vi_tri) in [
            (vec![Value::Int(1), Value::Float(2.0)], 0),
            (vec![Value::Float(1.0), Value::Str("2".into())], 1),
            // Lỗi kiểu thắng `NaN`: quét hết phần tử trước khi xét `NaN`.
            (vec![Value::Float(f64::NAN), Value::Str("2".into())], 1),
        ] {
            for name in ["list.min_float", "list.max_float"] {
                match goi(name, &xs, Span::default()) {
                    Err(Abort::Trap(t)) => {
                        assert_eq!(t.code.short, "VS0501");
                        assert!(
                            t.msg.contains(&format!("phần tử {vi_tri}")),
                            "{}: {}",
                            name,
                            t.msg
                        );
                        assert!(t.msg.contains(name), "{}", t.msg);
                    }
                    other => panic!("{name} phải trap VS0501, được {other:?}"),
                }
            }
        }
    }
}
