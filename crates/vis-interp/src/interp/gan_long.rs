// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-268 — interp gán *đường*** (`a.b.f = v`, `xs[0].f = v`, `a.bs[i].f += 1`, …).
//!
//! Tách khỏi `interp.rs` để giữ **R1** ≤ 1.500 dòng. Ngữ nghĩa **giá trị**: đọc gốc một lần,
//! đánh giá chỉ số/khoá **một lần** theo thứ tự đường, đọc lá cũ (nếu gán ghép), rồi
//! `gan::gan_duong` dựng lại từng tầng — khớp `With`/`Set` của VM và không aliasing.

use vis_hir::hir::{AssignOp, BinOp, HExpr};
use vis_syntax::Span;

use crate::gan::{doc_chi_so, doc_duong, doc_truong, gan_duong, Buoc};
use crate::interp::{Abort, Interp, Trap, MISMATCH, R, RT};
use crate::rt::fee::FeeKind;

/// Gốc của đích nếu là **biến** (kể cả giữa đường), ngược lại `None`.
pub(crate) fn goc_var(t: &HExpr) -> Option<u32> {
    match t {
        HExpr::Var { sym, .. } => *sym,
        HExpr::Field { base, .. } | HExpr::Index { base, .. } => goc_var(base),
        _ => None,
    }
}

/// Thu các **nút** bước của đường, từ gốc ra lá.
///
/// **VD-270**: `pub(crate)` để frame evaluator dùng **cùng** luật/ thông điệp `?.`/gốc.
pub(crate) fn thu_duong<'a>(t: &'a HExpr, ra: &mut Vec<&'a HExpr>, span: Span) -> R<()> {
    match t {
        HExpr::Field { base, optional, .. } => {
            if *optional {
                return Err(Abort::Trap(Trap::new(
                    MISMATCH,
                    "đích gán không nhận `?.` — bóc `Option` bằng `match`/`??`/`?` rồi mới gán (§4.1)",
                    span,
                )));
            }
            thu_duong(base, ra, span)?;
            ra.push(t);
            Ok(())
        }
        HExpr::Index { base, .. } => {
            thu_duong(base, ra, span)?;
            ra.push(t);
            Ok(())
        }
        HExpr::Var { .. } => Ok(()),
        _ => Err(Abort::Trap(Trap::new(
            RT,
            "đích gán phải bắt đầu bằng biến `var` (hoặc `var self`), §4.1",
            span,
        ))),
    }
}

impl<'a> Interp<'a> {
    /// Thi hành `target = value` / `target += value` cho đích **đường** có gốc `id`.
    pub(crate) fn gan_long(
        &mut self,
        id: u32,
        target: &HExpr,
        op: AssignOp,
        value: &HExpr,
        span: Span,
    ) -> R<()> {
        // Gốc đọc **một lần**, trước chỉ số và vế phải (mirror codegen `LoadLocal`).
        let cu = self.doc_o(id, target.span())?;
        let mut nut = Vec::new();
        thu_duong(target, &mut nut, span)?;
        let mut duong: Vec<Buoc> = Vec::with_capacity(nut.len());
        // Đọc **tầng giữa** đúng thứ tự codegen: `a.b` trước chỉ số kế, `xs[i]` ngay sau khi
        // đánh giá `i`, tất cả **trước vế phải**. Lần đọc này cũng chốt trap ngoài phạm vi/sai
        // hình dạng **trước** khi vế phải có tác dụng phụ; `gan_duong` đọc lại (thuần) để gộp.
        let mut con = cu.clone();
        for (k, n) in nut.iter().enumerate() {
            let cuoi = k + 1 == nut.len();
            match n {
                HExpr::Field { name, .. } => {
                    if !cuoi {
                        con = doc_truong(con, name, span)?;
                    }
                    duong.push(Buoc::Truong(name.clone()));
                }
                HExpr::Index { index, .. } => {
                    let iv = self.eval(index)?;
                    if !cuoi {
                        con = doc_chi_so(con, iv.clone(), span)?;
                    }
                    duong.push(Buoc::ChiSo(iv));
                }
                _ => {}
            }
        }
        let moi = if matches!(op, AssignOp::Set) {
            // Cùng biên nâng `Option` như gán một tầng (`x.f = …`, `x[i] = …`).
            let dep = self.assign_depth(target, &cu);
            let rhs = crate::lift::lift_opt(self.eval(value)?, dep);
            gan_duong(cu, &duong, rhs, span)?
        } else {
            // Gán ghép: đọc **lá cũ** trước vế phải, rồi `lá = lá <op> v`.
            let old = doc_duong(&cu, &duong, span)?;
            let rhs = self.eval(value)?;
            let bop = match op {
                AssignOp::Add => BinOp::Add,
                AssignOp::Sub => BinOp::Sub,
                AssignOp::Mul => BinOp::Mul,
                AssignOp::Div => BinOp::Div,
                // **R7 (VD-178)**: `//=` — chia lấy phần nguyên.
                AssignOp::FloorDiv => BinOp::FloorDiv,
                AssignOp::Rem => BinOp::Rem,
                AssignOp::Set => unreachable!("Set đã xử lý ở nhánh trên"),
            };
            let new = crate::builtin::bin(bop, old, rhs, span)?;
            gan_duong(cu, &duong, new, span)?
        };
        if let Some(a) = self.charge(FeeKind::Assign, span) {
            return Err(a);
        }
        self.ghi_o(id, moi, span)
    }
}
