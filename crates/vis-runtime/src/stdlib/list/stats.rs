// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — gom các module `stdlib` miền `list_b` (giữ nguyên nội dung/đường dẫn).

pub(crate) mod listmedian {
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

    use vis_core::trap::Abort;
    use vis_core::value::Value;

    use super::super::super::chung::loi;

    /// Trung vị `xs`; `Err` là trap `VS0501` khi phần tử không phải `float`.
    pub(crate) fn goi(xs: &[Value], span: Span) -> Result<Value, Abort> {
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
}

pub(crate) mod listminmaxfloat {
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

    use vis_core::trap::Abort;
    use vis_core::value::Value;

    use super::super::super::chung::loi;

    /// Cực trị `xs` theo `name` (`list.min_float`/`list.max_float`); `Err` là trap `VS0501` khi
    /// phần tử không phải `float`.
    pub(crate) fn goi(name: &str, xs: &[Value], span: Span) -> Result<Value, Abort> {
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
}

pub(crate) mod listsumfloat {
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

    use vis_core::trap::Abort;
    use vis_core::value::Value;

    use super::super::super::chung::loi;

    /// Tổng `xs` theo thứ tự **trái → phải**, hạt giống `+0.0`; `Err` là trap `VS0501` khi phần tử
    /// không phải `float`.
    pub(crate) fn goi(xs: &[Value], span: Span) -> Result<Value, Abort> {
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
}

pub(crate) mod mapcontainskey {
    //! **VD-270 (map-contains-key)** — `std.map.contains_key(m, key) -> bool`.
    //!
    //! Hợp đồng (một hiện thực dùng chung interp + VM):
    //! - `true` khi `m` có **ít nhất một** cặp khoá `==` `key`; `false` khi không có hoặc `m` rỗng
    //!   (không trap).
    //! - So bằng [`vis_core::value::eq`] — **đúng** phép so của `map.has`/`map.get`
    //!   (`Value::eq_value`: cấu trúc, list/map/record/enum lồng nhau; `int` ↔ `float` theo tỉ lệ
    //!   như `==`), nên `contains_key(m, k)` khớp `m.has(k)`.
    //! - Tra qua [`vis_core::value::VisMap::contains_key`]: chỉ mục băm khi map đủ lớn, quét
    //!   tuyến tính cho map nhỏ/khoá phức tạp; không cấp phát.
    //!
    //! `key` **không** bị ép kiểu ở tầng chạy: checker đã buộc `K` từ `m`; lời gọi thủ công vẫn
    //! so **cấu trúc** (khác kiểu ⇒ thường `false`, không trap). `m` không phải map ⇒ lỗi dữ liệu
    //! `VS0501` qua [`super::super::super::chung::ban_do`].

    use vis_syntax::Span;

    use super::super::super::chung::ban_do;
    use vis_core::trap::R;
    use vis_core::value::{Value, VisMap};

    /// `true` iff có khoá `==` `key`; map rỗng ⇒ `false`.
    pub(crate) fn co(m: &VisMap, key: &Value) -> bool {
        m.contains_key(key)
    }

    /// Điểm vào `std.map.contains_key` — kiểm `m` là map rồi so khoá.
    pub(crate) fn goi(name: &str, args: &[Value], span: Span) -> R<Value> {
        let m = ban_do(name, &args[0], 0, span)?;
        Ok(Value::Bool(co(m, &args[1])))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn bd(kv: &[(&str, i64)]) -> VisMap {
            VisMap::from_pairs(
                kv.iter()
                    .map(|(k, v)| (Value::Str((*k).into()), Value::Int(*v))),
            )
        }

        #[test]
        fn co_va_khong_co() {
            let m = bd(&[("a", 1), ("b", 2)]);
            assert!(co(&m, &Value::Str("a".into())));
            assert!(co(&m, &Value::Str("b".into())));
            assert!(!co(&m, &Value::Str("c".into())));
        }

        #[test]
        fn map_rong_la_false() {
            assert!(!co(&VisMap::new(), &Value::Str("a".into())));
            assert!(!co(&VisMap::new(), &Value::Int(1)));
        }

        #[test]
        fn cau_truc_long_nhau_va_khac_kieu() {
            // List làm khoá: cùng cấu trúc, khác thứ tự ⇒ false.
            let m = VisMap::from_pairs(vec![(
                Value::list(vec![Value::Int(1), Value::Int(2)]),
                Value::Int(1),
            )]);
            assert!(co(&m, &Value::list(vec![Value::Int(1), Value::Int(2)])));
            assert!(!co(&m, &Value::list(vec![Value::Int(2), Value::Int(1)])));

            // Bản ghi: so theo trường, không theo thứ tự khai báo.
            let m = VisMap::from_pairs(vec![(
                Value::record(
                    "P",
                    vec![("x".into(), Value::Int(1)), ("y".into(), Value::Int(2))],
                ),
                Value::Int(1),
            )]);
            let k = Value::record(
                "P",
                vec![("y".into(), Value::Int(2)), ("x".into(), Value::Int(1))],
            );
            assert!(co(&m, &k));

            // Khác kiểu (`int` vs `str`) chỉ là `false`, không trap.
            let m = VisMap::from_pairs(vec![(Value::Int(1), Value::Int(1))]);
            assert!(!co(&m, &Value::Str("1".into())));
        }
    }
}
