// SPDX-License-Identifier: MIT OR Apache-2.0
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

use crate::interp::R;
use crate::value::Value;

/// Chia `xs` thành các nhóm **đúng `n`** phần tử, bỏ phần dư cuối.
///
/// `n` phải `>= 1` — caller ([`goi`]) kiểm `n <= 0` trước; `n == 0` sẽ panic trong
/// `slice::chunks_exact` nên không được gọi trực tiếp với `0`.
pub(super) fn chia(xs: &[Value], n: usize) -> Vec<Value> {
    debug_assert!(n >= 1, "chia: n phải >= 1 (caller đã kiểm)");
    xs.chunks_exact(n)
        .map(|c| Value::List(c.to_vec()))
        .collect()
}

/// Dispatch `list.chunks_exact`: kiểm `n` **trước khi chia** (kể cả `xs` rỗng), rồi trả
/// `list[list[T]]` với các nhóm là list mới.
pub(super) fn goi(name: &str, args: &[Value], span: Span) -> R<Value> {
    let xs = super::danh_sach(name, &args[0], 0, span)?;
    let n = super::so(name, &args[1], 1, span)?;
    if n <= 0 {
        return Err(super::loi(
            format!("`{name}`: kích thước khối phải ≥ 1, được {n}"),
            span,
        ));
    }
    Ok(Value::List(chia(xs, n as usize)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interp::Abort;

    fn ds(xs: &[i64]) -> Vec<Value> {
        xs.iter().copied().map(Value::Int).collect()
    }

    fn l(xs: &[i64]) -> Value {
        Value::List(ds(xs))
    }

    fn goi_ok(name: &str, xs: &[Value], n: i64) -> Vec<Value> {
        match goi(
            name,
            &[Value::List(xs.to_vec()), Value::Int(n)],
            Span::default(),
        )
        .expect("chunks_exact hợp lệ")
        {
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
        let long = vec![Value::List(ds(&[1, 2])), Value::List(ds(&[3]))];
        bang(
            &goi_ok("list.chunks_exact", &long, 2),
            &[Value::List(long.clone())],
        );
    }

    #[test]
    fn n_khong_duong_la_trap_vs0501() {
        for n in [0, -1, i64::MIN] {
            match goi(
                "list.chunks_exact",
                &[Value::List(ds(&[1, 2, 3])), Value::Int(n)],
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
            &[Value::List(Vec::new()), Value::Int(0)],
            Span::default(),
        ) {
            Err(Abort::Trap(t)) => assert_eq!(t.code.short, "VS0501"),
            other => panic!("xs rỗng + n=0: phải trap, được {other:?}"),
        }
    }
}
