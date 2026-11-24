// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-269** — `recv?.m(…)`: gọi phương thức qua **một** lớp `Option`.
//!
//! Lowering **không** dispatch được callee `HExpr::Field { optional: true }` (kiểu nhận là
//! `Option[T]`), nên checker tự phân giải:
//!
//! 1. Bóc **đúng một** lớp `Option` của base; base không phải `Option` ⇒ `VS0412` (kể cả
//!    `Result` — `?.` chỉ có nghĩa trên `Option`).
//! 2. Phương thức `impl` trước (bảng [`vis_hir::MethodRet`]), rồi phương thức lõi `list`/`str`/
//!    `map`/`Chan`, rồi `std.<module>.<fn>` (R5c). Thiếu phương thức trên bản ghi/enum ⇒
//!    `VS0405`; kiểu khác ⇒ `VS0403`.
//! 3. Kiểu kết quả = [`vis_hir::nang_optional`]: `U` ⇒ `U?`, `U?` ⇒ `U?` (không hai lớp),
//!    `()`/`{error}`/`never` giữ nguyên.
//!
//! Đối số được kiểm **một lần** đúng nhánh phân giải được (không kiểm trùng).

use vis_hir::hir::HExpr;
use vis_hir::ty::Type;
use vis_syntax::Span;

use super::{Checker, MISMATCH, NO_FIELD};

impl<'a> Checker<'a> {
    /// **VD-269** — callee dạng `Field`: `recv.m(…)` (lõi/std) hoặc `recv?.m(…)` (qua Option).
    /// Trả `None` nếu callee không phải `Field` (để `kiem_call` đi tiếp hàm/biến).
    pub(crate) fn kiem_goi_field(
        &mut self,
        callee: &HExpr,
        args: &[HExpr],
        named: &[(String, HExpr)],
        span: Span,
    ) -> Option<Type> {
        let HExpr::Field {
            base,
            name,
            optional,
            ..
        } = callee
        else {
            return None;
        };
        if *optional {
            return Some(self.kiem_goi_tuy_chon(base, name, args, named, span));
        }
        let recv = self.kiem_expr(base);
        if let Some(t) = self.kiem_method(&recv, name, args, span) {
            return Some(t);
        }
        // **R5c (VD-178)**: `x.f(a) ≡ std.<mod>.f(x, a)` — tham số đầu có kiểu của `x`.
        super::expr::phu::kiem_std_method(self, &recv, name, args, span)
    }

    /// Kiểu của `recv?.name(args)` với `recv: Option[T]` (đã kiểm `recv` ở caller).
    pub(crate) fn kiem_goi_tuy_chon(
        &mut self,
        base: &HExpr,
        name: &str,
        args: &[HExpr],
        named: &[(String, HExpr)],
        span: Span,
    ) -> Type {
        let recv = self.kiem_expr(base);
        let inner = match &recv {
            Type::Error => return Type::Error,
            Type::Opt(t) => (**t).clone(),
            other => {
                // **VD-269**: cùng luật với `?.` trường (VD-242 §3.2): base phải `Option[U]`.
                // `Result` rơi vào đây ⇒ bị từ chối, không có `Result?.m()`.
                self.diag(
                    MISMATCH,
                    format!(
                        "`?.` cần base `Option[U]`, được `{}` — dùng `.` cho base thường (VD-269)",
                        self.show(other)
                    ),
                    span,
                );
                return Type::Error;
            }
        };
        // **VD-270**: tra **kèm `TypeTable`** — biến nhận có thể là **instantiation**
        // (`Hop[int]`), chỉ bảng mới ánh xạ `RecId` instantiation về tên `impl`. Trước đây
        // `sym_voi_types` (không bảng) bỏ sót ⇒ `VS0405` oan dù `impl` có thật.
        let ret = if let Some(mut sig) = self
            .mret
            .sym(&inner, name, self.types)
            .and_then(|sym| self.sigs.get(sym as usize).and_then(|s| s.clone()))
        {
            // Chữ ký phương thức có `self` đầu; call-site **không** truyền `self`.
            if sig
                .params
                .first()
                .is_some_and(|p| p.0 == "self" || p.0 == "dt")
            {
                sig.params.remove(0);
                if !sig.raw_params.is_empty() {
                    sig.raw_params.remove(0);
                }
            }
            // Phương thức `impl`: chữ ký đầy đủ như gọi hàm cấp tệp (`kiem_args` kiểm **một**
            // lần), kể cả hợp nhất generic để kiểu trả không còn `T`.
            let arg_tys = self.kiem_args(&sig, args, named, span);
            if sig.is_async {
                // **VD-284/ST-302 (#13)**: gọi method async trả `Task[ret]` (không chạy thân).
                return Type::TypeRef {
                    name: "Task".into(),
                    args: vec![sig.ret],
                };
            }
            if sig.generics.is_empty() {
                sig.ret
            } else {
                let mut bang: Vec<(String, Type)> = Vec::new();
                for (i, t) in arg_tys.iter().enumerate() {
                    if let Some(raw) = sig.raw_params.get(i) {
                        let raw = raw.clone();
                        self.hop_nhat(&raw, t, &sig.generics, &mut bang, &sig.name, args[i].span());
                    }
                }
                super::thay_gen(&sig.ret_raw(), &bang)
            }
        } else if let Some(t) = self.kiem_method(&inner, name, args, span) {
            t
        } else if let Some(t) = super::expr::phu::kiem_std_method(self, &inner, name, args, span) {
            t
        } else {
            if inner.is_error() {
                return Type::Error;
            }
            // Bản ghi/enum: giữ mã "không có trường" `VS0405` (cùng sổ với `?.f` lạ) nhưng nói
            // rõ **phương thức** để không gợi ý sai; kiểu khác giữ `VS0403` như gọi phương thức
            // thường trên kiểu không có phương thức.
            let (code, what) = match &inner {
                Type::Record(_) | Type::Enum(_) => (vis_diag::ma_truong(), "phương thức"),
                _ => (NO_FIELD, "phương thức"),
            };
            self.diag(
                code,
                format!(
                    "`{}` không có {what} `{name}` (VD-269 — kiểm tra `impl`/tên)",
                    self.show(&inner)
                ),
                span,
            );
            return Type::Error;
        };
        vis_hir::nang_optional(ret)
    }
}
