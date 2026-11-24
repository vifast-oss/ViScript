// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-61 (VD-96)** — kiểu của **kênh** `Chan[T]`: `chan[T](n)`, `send`/`recv`/`close`.
//!
//! Tách khỏi `tyck/expr.rs` để giữ **R1** ≤ 1.500 dòng (tiền lệ ST-60 tách `sig.rs`). Đây vẫn là
//! `impl Checker` đặt trong **mô-đun con** của `tyck::expr` nên dùng chung `diag`/`coerce`/`arity`
//! của cha — **một** nguồn kiểu, không nhân bản luật.

use vis_hir::hir::HExpr;
use vis_hir::ty::Type;
use vis_syntax::Span;

use super::super::{Checker, MISMATCH};

impl<'a> Checker<'a> {
    /// `chan[T](n)` — `n` phải `int` (âm là lỗi **lúc chạy** `VS0501`, VD-91); kết quả `Chan[T]`.
    pub(super) fn kiem_chan_new(&mut self, elem: &Type, cap: &HExpr, span: Span) -> Type {
        let et = self.resolve_ann(elem, span);
        let ct = self.kiem_expr(cap);
        if !matches!(ct, Type::Int | Type::Error) {
            self.diag(
                MISMATCH,
                format!(
                    "`chan[T](n)`: `n` phải `int` (dung lượng), được `{}`",
                    self.show(&ct)
                ),
                cap.span(),
            );
        }
        Type::TypeRef {
            name: "Chan".into(),
            args: vec![et],
        }
    }

    /// Phương thức kênh: `send(v)` đòi khớp `T` (**`VS0412` tĩnh**), `recv()` ⇒ `T?`, `close()` ⇒
    /// `()`; **VS3-2b**: `recv_timeout(ms)` ⇒ `T?`, `send_timeout(ms, v)` ⇒ `bool` (ms phải `int`;
    /// âm là lỗi lúc chạy `VS0501`). Tên khác ⇒ `None` (đường chung báo `VS0403`).
    pub(super) fn kiem_chan_method(
        &mut self,
        c_args: &[Type],
        name: &str,
        arg_tys: &[Type],
        args: &[HExpr],
        span: Span,
    ) -> Option<Type> {
        let t = c_args[0].clone();
        match name {
            "send" => {
                self.arity(args, 1, name, span);
                self.arg_ty(arg_tys, 0, &t, name, args, span);
                Some(Type::Unit)
            }
            "recv" => {
                self.arity(args, 0, name, span);
                Some(Type::Opt(Box::new(t)))
            }
            // Hợp đồng VS3-2b: None = timeout **hoặc** kênh đóng (không phân biệt).
            "recv_timeout" => {
                self.arity(args, 1, name, span);
                self.arg_ty(arg_tys, 0, &Type::Int, name, args, span);
                Some(Type::Opt(Box::new(t)))
            }
            // Hợp đồng VS3-2b: false = timeout **hoặc** kênh đóng.
            "send_timeout" => {
                self.arity(args, 2, name, span);
                self.arg_ty(arg_tys, 0, &Type::Int, name, args, span);
                self.arg_ty(arg_tys, 1, &t, name, args, span);
                Some(Type::Bool)
            }
            "close" => {
                self.arity(args, 0, name, span);
                Some(Type::Unit)
            }
            _ => None,
        }
    }

    /// **ST-293 (VD-274/VD-280)** — 4 `select_*` dựng sẵn: phần tử `T` suy từ `list[Chan[T]]`;
    /// trả **pseudo-record** `SelectRecv[T]` (chỉ `kenh.rs`/`truong.rs` biết hai trường
    /// `index:int`, `value:T?`) hoặc `int?` cho chiều gửi. Không đổi parser/EBNF.
    pub(super) fn kiem_select(&mut self, name: &str, args: &[HExpr], span: Span) -> Type {
        let want = match name {
            "select_recv" => 1,
            "select_recv_timeout" | "select_send" => 2,
            "select_send_timeout" => 3,
            _ => 0,
        };
        self.arity(args, want, name, span);
        let tys: Vec<Type> = args.iter().map(|a| self.kiem_expr(a)).collect();
        let elem = match tys.first() {
            Some(Type::Error) | None => Type::Error,
            Some(Type::List(inner)) => match &**inner {
                Type::TypeRef { name: n, args: a } if n == "Chan" && a.len() == 1 => a[0].clone(),
                Type::Error => Type::Error,
                other => {
                    self.diag(
                        MISMATCH,
                        format!(
                            "`{name}`: đối số 1 phải `list[Chan[T]]`, được `list[{}]`",
                            self.show(other)
                        ),
                        span,
                    );
                    Type::Error
                }
            },
            Some(other) => {
                self.diag(
                    MISMATCH,
                    format!(
                        "`{name}`: đối số 1 phải `list[Chan[T]]`, được `{}`",
                        self.show(other)
                    ),
                    span,
                );
                Type::Error
            }
        };
        match name {
            "select_recv" => Type::TypeRef {
                name: "SelectRecv".into(),
                args: vec![elem],
            },
            "select_recv_timeout" => {
                self.arg_ty(&tys, 1, &Type::Int, name, args, span);
                Type::Opt(Box::new(Type::TypeRef {
                    name: "SelectRecv".into(),
                    args: vec![elem],
                }))
            }
            "select_send" => {
                self.arg_ty(&tys, 1, &elem, name, args, span);
                Type::Opt(Box::new(Type::Int))
            }
            "select_send_timeout" => {
                self.arg_ty(&tys, 1, &elem, name, args, span);
                self.arg_ty(&tys, 2, &Type::Int, name, args, span);
                Type::Opt(Box::new(Type::Int))
            }
            _ => Type::Error,
        }
    }
}
