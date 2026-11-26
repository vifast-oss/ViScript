// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — gom các module `stdlib` miền `list_a` (giữ nguyên nội dung/đường dẫn).

pub(crate) mod listchunksexact {
    //! **VD-270 (chunks-exact)** — `std.list.chunks_exact(xs, n) -> list[list[T]]`.
    //!
    //! Hợp đồng (một hiện thực dùng chung interp + VM):
    //! - Chia `xs` thành các nhóm **đúng `n`** phần tử theo thứ tự; **bỏ phần dư** cuối — khác
    //!   `list.chunk` (giữ nhóm cuối ngắn): `chunks_exact([1,2,3,4,5], 2)` ⇒ `[[1,2],[3,4]]`
    //!   (phần tử `5` bị bỏ).
    //! - `n == 1` ⇒ mỗi phần tử một nhóm; `n == len(xs)` ⇒ **đúng một** nhóm (khi `len > 0`);
    //!   `n > len(xs)` ⇒ `[]`; `xs` rỗng (với `n > 0`) ⇒ `[]`; list rỗng **không** sinh nhóm rỗng.
    //! - `n <= 0` ⇒ trap `VS0501` nêu `list.chunks_exact`; kiểm `n` **trước khi chia** nên
    //!   `chunks_exact([], 0)` cũng trap (không trả `[]`), không panic Rust.
    //! - Mỗi nhóm là **list mới** (`to_vec` sao giá trị từng phần tử) nên không alias `xs`; phần
    //!   tử giữ **nguyên giá trị** (list/record lồng nhau không bị làm phẳng hay đổi hình dạng).
    //! - Không có trần riêng: một lượt quét, cấp phát đúng `len(xs) / n` nhóm.
    //!
    //! `xs` do checker buộc là `list[T]`; tầng chạy vẫn kiểm **động** kiểu đối số (dữ liệu không
    //! kiểu tới được đây) — sai ⇒ trap `VS0501`, không panic Rust.

    use vis_syntax::Span;

    use vis_core::trap::R;
    use vis_core::value::Value;

    /// Chia `xs` thành các nhóm **đúng `n`** phần tử, bỏ phần dư cuối.
    ///
    /// `n` phải `>= 1` — caller ([`goi`]) kiểm `n <= 0` trước; `n == 0` sẽ panic trong
    /// `slice::chunks_exact` nên không được gọi trực tiếp với `0`.
    pub(crate) fn chia(xs: &[Value], n: usize) -> Vec<Value> {
        debug_assert!(n >= 1, "chia: n phải >= 1 (caller đã kiểm)");
        xs.chunks_exact(n)
            .map(|c| Value::list(c.to_vec()))
            .collect()
    }

    /// Dispatch `list.chunks_exact`: kiểm `n` **trước khi chia** (kể cả `xs` rỗng), rồi trả
    /// `list[list[T]]` với các nhóm là list mới.
    pub(crate) fn goi(name: &str, args: &[Value], span: Span) -> R<Value> {
        let xs = super::super::super::danh_sach(name, &args[0], 0, span)?;
        let n = super::super::super::so(name, &args[1], 1, span)?;
        if n <= 0 {
            return Err(super::super::super::loi(
                format!("`{name}`: kích thước khối phải ≥ 1, được {n}"),
                span,
            ));
        }
        Ok(Value::list(chia(xs, n as usize)))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use vis_core::trap::Abort;

        fn ds(xs: &[i64]) -> Vec<Value> {
            xs.iter().copied().map(Value::Int).collect()
        }

        fn l(xs: &[i64]) -> Value {
            Value::list(ds(xs))
        }

        fn goi_ok(name: &str, xs: &[Value], n: i64) -> Vec<Value> {
            match goi(
                name,
                &[Value::list(xs.to_vec()), Value::Int(n)],
                Span::default(),
            )
            .expect("chunks_exact hợp lệ")
            {
                Value::List(out) => std::rc::Rc::unwrap_or_clone(out),
                other => panic!("phải trả list, được {other:?}"),
            }
        }

        /// `Value` không có `PartialEq` (so bằng [`vis_core::value::eq`] như `==` của ngôn ngữ).
        fn bang(out: &[Value], mong: &[Value]) {
            assert_eq!(out.len(), mong.len(), "độ dài lệch: {out:?} vs {mong:?}");
            for (i, (a, b)) in out.iter().zip(mong).enumerate() {
                assert!(
                    vis_core::value::eq(a, b),
                    "phần tử {i} lệch: {a:?} vs {b:?}"
                );
            }
        }

        #[test]
        fn chia_dung_n_va_bo_phan_du_cuoi() {
            bang(&goi_ok("list.chunks_exact", &ds(&[1]), 1), &[l(&[1])]);
            bang(
                &goi_ok("list.chunks_exact", &ds(&[1, 2, 3, 4]), 2),
                &[l(&[1, 2]), l(&[3, 4])],
            );
            bang(
                &goi_ok("list.chunks_exact", &ds(&[1, 2, 3, 4, 5]), 2),
                &[l(&[1, 2]), l(&[3, 4])],
            );
            bang(
                &goi_ok("list.chunks_exact", &ds(&[1, 2, 3, 4, 5, 6, 7]), 3),
                &[l(&[1, 2, 3]), l(&[4, 5, 6])],
            );
            bang(
                &goi_ok("list.chunks_exact", &ds(&[1, 2, 3]), 3),
                &[l(&[1, 2, 3])],
            );
        }

        #[test]
        fn n_lon_hon_len_va_xs_rong() {
            bang(&goi_ok("list.chunks_exact", &ds(&[1, 2, 3]), 4), &[]);
            bang(&goi_ok("list.chunks_exact", &ds(&[1]), 2), &[]);
            bang(&goi_ok("list.chunks_exact", &[], 1), &[]);
            bang(&goi_ok("list.chunks_exact", &[], 9), &[]);
        }

        #[test]
        fn nhom_la_list_moi_khong_alias_va_giu_gia_tri() {
            let mut xs = ds(&[1, 2, 3, 4, 5]);
            let g = goi_ok("list.chunks_exact", &xs, 2);
            xs[0] = Value::Int(9);
            bang(&g, &[l(&[1, 2]), l(&[3, 4])]);

            // Phần tử vốn là list được giữ nguyên (không làm phẳng/đệ quy).
            let long = vec![Value::list(ds(&[1, 2])), Value::list(ds(&[3]))];
            bang(
                &goi_ok("list.chunks_exact", &long, 2),
                &[Value::list(long.clone())],
            );
        }

        #[test]
        fn n_khong_duong_la_trap_vs0501() {
            for n in [0, -1, i64::MIN] {
                match goi(
                    "list.chunks_exact",
                    &[Value::list(ds(&[1, 2, 3])), Value::Int(n)],
                    Span::default(),
                ) {
                    Err(Abort::Trap(t)) => {
                        assert_eq!(t.code.short, "VS0501", "n={n}");
                        assert!(t.msg.contains("list.chunks_exact"), "n={n}: {}", t.msg);
                        assert!(t.msg.contains("≥ 1"), "n={n}: {}", t.msg);
                    }
                    other => panic!("n={n}: phải trap VS0501, được {other:?}"),
                }
            }
            // `n <= 0` thắng cả khi `xs` rỗng (kiểm trước khi chia).
            match goi(
                "list.chunks_exact",
                &[Value::list(Vec::new()), Value::Int(0)],
                Span::default(),
            ) {
                Err(Abort::Trap(t)) => assert_eq!(t.code.short, "VS0501"),
                other => panic!("xs rỗng + n=0: phải trap, được {other:?}"),
            }
        }
    }
}

pub(crate) mod listcontains {
    //! **VD-270 (list-contains)** — `std.list.contains(xs, value) -> bool`.
    //!
    //! Hợp đồng (một hiện thực dùng chung interp + VM):
    //! - `true` khi có **ít nhất một** phần tử `==` `value`; `false` khi không có hoặc `xs` rỗng
    //!   (không trap).
    //! - So bằng [`super::super::super::indexof::tim`] — **đúng** phép so của `index_of`/`last_index_of`
    //!   (`Value::eq_value`: cấu trúc, list/map/record/enum lồng nhau; `int` ↔ `float` theo tỉ lệ
    //!   như `==`), nên `contains(xs, v) == index_of(xs, v).is_some()`.
    //! - Quét tuyến tính, dừng ở phần tử đầu tiên bằng; không cấp phát.
    //!
    //! `value` **không** bị ép kiểu ở tầng chạy: checker đã buộc `T` từ `xs`; lời gọi thủ công vẫn
    //! so **cấu trúc** (khác kiểu ⇒ thường `false`, không trap).

    use super::super::super::indexof;
    use vis_core::value::Value;

    /// `true` iff có phần tử `==` `value`; rỗng ⇒ `false`.
    pub(crate) fn goi(xs: &[Value], value: &Value) -> Value {
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
                Value::list(vec![Value::Int(1), Value::Int(2)]),
                Value::list(vec![Value::Int(3), Value::Int(4)]),
            ];
            assert!(co(&xs, &Value::list(vec![Value::Int(1), Value::Int(2)])));
            assert!(khong(&xs, &Value::list(vec![Value::Int(2), Value::Int(1)])));

            let a = Value::record(
                "P",
                vec![("x".into(), Value::Int(1)), ("y".into(), Value::Int(2))],
            );
            let b = Value::record(
                "P",
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
}

pub(crate) mod listmath {
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

    use crate::codes::OVERFLOW;
    use vis_core::trap::{Abort, Trap};
    use vis_core::value::Value;

    use super::super::super::chung::loi;

    /// Gộp `xs` bằng `+`/`*` checked; `name` là `"list.sum"` hoặc `"list.product"`.
    ///
    /// `Err` là trap: `VS0507` khi tràn (kể cả tràn trung gian), `VS0501` khi phần tử không phải
    /// `int`. Không có kết quả nửa vời — trap huỷ toàn bộ lời gọi.
    pub(crate) fn goi(name: &str, xs: &[Value], span: Span) -> Result<Value, Abort> {
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
}

pub(crate) mod listmean {
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

    use vis_core::trap::Abort;
    use vis_core::value::Value;

    use super::super::super::chung::loi;

    /// Trung bình cộng `xs`; `Err` là trap `VS0501` khi phần tử không phải `float`.
    pub(crate) fn goi(xs: &[Value], span: Span) -> Result<Value, Abort> {
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
}
