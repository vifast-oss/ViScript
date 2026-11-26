// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-318c** — kiểm **toán tử nhị phân** (`kiem_bin`, nguyên văn từ `tyck/expr.rs`; 0 đổi
//! hành vi/chẩn đoán).

use vis_hir::hir::BinOp;
use vis_hir::ty::Type;

use super::super::{num, Checker, ARG_COUNT, MISMATCH};
use super::phu::{is_percent_float, la_cap};

impl<'a> Checker<'a> {
    pub(super) fn kiem_bin(
        &mut self,
        op: BinOp,
        (l, r): (&Type, &Type),
        (l_lit, r_lit): (bool, bool),
        (lspan, rspan): (vis_syntax::Span, vis_syntax::Span),
        span: vis_syntax::Span,
    ) -> Type {
        match op {
            BinOp::Add
            | BinOp::Sub
            | BinOp::Mul
            | BinOp::Div
            | BinOp::FloorDiv
            | BinOp::Rem
            | BinOp::Pow
            | BinOp::Range
            | BinOp::RangeIncl => match num::so_hoc(op, l, r, l_lit, r_lit) {
                Some(t) => t,
                None => {
                    // **R9 (VD-178)**: toán tử không hỗ trợ cho kiểu ⇒ `VS0414` (tách khỏi `VS0412`).
                    self.diag(
                        super::super::TOAN_TU,
                        format!(
                            "toán tử `{}`: `{}` và `{}` không cùng kiểu/đơn vị",
                            op.text(),
                            self.show(l),
                            self.show(r)
                        ),
                        span,
                    );
                    Type::Error
                }
            },
            BinOp::Coalesce => {
                let inner = match l {
                    Type::Opt(i) => (**i).clone(),
                    Type::Error => return Type::Error,
                    other => {
                        self.diag(
                            MISMATCH,
                            format!("`??` cần vế trái kiểu `T?`, được `{}`", self.show(other)),
                            lspan,
                        );
                        return Type::Error;
                    }
                };
                self.coerce_lit(r, &inner, rspan, "`??`", r_lit);
                inner
            }
            BinOp::Eq | BinOp::Ne => {
                // **ST-72 (VD-106 (1))**: `cap` là quyền — **không** so sánh được.
                if la_cap(l) || la_cap(r) {
                    self.diag(
                        MISMATCH,
                        format!("`cap` không so sánh được (`{}`)", self.show(l)),
                        span,
                    );
                    return Type::Bool;
                }
                if matches!(l, Type::Fn(..)) || matches!(r, Type::Fn(..)) {
                    self.diag(MISMATCH, "không so sánh được hàm (VD-43 #4)", span);
                    return Type::Bool;
                }
                if is_percent_float(l, r) {
                    return Type::Bool;
                }
                let (a, b) = num::promote(l, r, l_lit, r_lit);
                if !self.same(&a, &b) {
                    self.diag(
                        MISMATCH,
                        format!(
                            "so sánh khác kiểu: `{}` và `{}`",
                            self.show(l),
                            self.show(r)
                        ),
                        span,
                    );
                }
                Type::Bool
            }
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                // `{error}` lan truyền im lặng (lambda chưa có kiểu tham số ở VS0).
                if l.is_error() || r.is_error() {
                    return Type::Bool;
                }
                // ST-12b (MAJOR #6): `percent` so sánh `float` **theo tỉ lệ** (§3.1/§3.6).
                if is_percent_float(l, r) {
                    return Type::Bool;
                }
                let (a, b) = num::promote(l, r, l_lit, r_lit);
                if !self.same(&a, &b) || !num::ordered(&a) {
                    self.diag(
                        MISMATCH,
                        format!(
                            "so sánh thứ tự cần cùng kiểu sắp được: `{}` và `{}`",
                            self.show(l),
                            self.show(r)
                        ),
                        span,
                    );
                }
                Type::Bool
            }
            BinOp::And | BinOp::Or => {
                if !matches!(l, Type::Bool | Type::Error) || !matches!(r, Type::Bool | Type::Error)
                {
                    self.diag(
                        MISMATCH,
                        format!(
                            "`{}` cần `bool`, được `{}` và `{}`",
                            op.text(),
                            self.show(l),
                            self.show(r)
                        ),
                        span,
                    );
                }
                Type::Bool
            }
            BinOp::Pipe => match r {
                Type::Fn(params, ret) => {
                    if params.len() == 1 {
                        let p = params[0].clone();
                        self.coerce(l, &p, lspan, "`|>`");
                    } else if params.is_empty() {
                        // hàm không đối số: `x |> f()` — không kiểm thêm
                    } else {
                        self.diag(
                            ARG_COUNT,
                            format!("`|>` cần hàm **một** đối số, hàm này nhận {}", params.len()),
                            span,
                        );
                    }
                    (**ret).clone()
                }
                Type::Error => Type::Error,
                other => {
                    self.diag(
                        MISMATCH,
                        format!("`|>` cần hàm ở vế phải, được `{}`", self.show(other)),
                        span,
                    );
                    Type::Error
                }
            },
        }
    }
}
