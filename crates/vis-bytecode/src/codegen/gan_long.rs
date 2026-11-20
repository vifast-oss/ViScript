// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-268 — codegen gán *đường*** (`a.b.f = v`, `xs[0].f = v`, `a.bs[i].f += 1`, …).
//!
//! Tách khỏi `codegen.rs` để giữ **R1** ≤ 1.500 dòng. Dùng lại đúng các opcode sẵn có
//! (`Field`/`Index`/`With`/`Set`/`Dup`/`Swap`) nên **không** đổi định dạng VBC1/VM.
//!
//! Ngăn xếp trước khi gộp (n = số bước):
//! `[a0, khoá1, a1, khoá2, a2, …, khoá(n-1), a(n-1), khoá_n, rhs]`
//! rồi gộp **từ lá lên**: `Set`/`With(1)` thay `[a(k-1), khoá_k, giá_trị]` bằng `a(k-1)'`.
//! Nhờ vậy mỗi chỉ số được đánh giá **một lần**, theo đúng thứ tự đường đích, trước vế phải.
//!
//! Ngữ nghĩa **giá trị**: `With`/`Set` của VM clone rồi dựng lại (không aliasing), khớp
//! `vis-interp::gan::gan_duong`.

use vis_syntax::Span;
use vis_types::{AssignOp, HExpr};

use crate::cerr::CodegenError;
use crate::fee::FeeKind;
use crate::format::Instr;
use crate::util::assign_binop;

/// Một bước của đường đích, từ gốc ra lá.
enum Buoc<'a> {
    Truong(String),
    ChiSo(&'a HExpr),
}

/// Đích mà các nhánh **cũ** trong `codegen.rs` đã hạ (giữ **byte y hệt** VD-97/ST-60b):
/// `x.f`, `x[i]`, `x.f[i]` với gốc là biến. Sâu hơn (hoặc gán ghép) đi qua [`gan_duong`].
pub(super) fn mot_tang(t: &HExpr) -> bool {
    match t {
        HExpr::Field { base, .. } => matches!(&**base, HExpr::Var { sym: Some(_), .. }),
        HExpr::Index { base, .. } => match &**base {
            HExpr::Var { sym: Some(_), .. } => true,
            HExpr::Field { base: b2, .. } => matches!(&**b2, HExpr::Var { sym: Some(_), .. }),
            _ => false,
        },
        _ => false,
    }
}

/// Thu đường từ gốc ra lá; trả `(sym gốc, các bước)`.
fn thu_duong<'a>(t: &'a HExpr, span: Span) -> Result<(u32, Vec<Buoc<'a>>), CodegenError> {
    let mut buoc = Vec::new();
    let mut nut = t;
    loop {
        match nut {
            HExpr::Field {
                base,
                name,
                optional,
                ..
            } => {
                if *optional {
                    // Tyck đã chặn `?.` trong đích gán; đây là chốt 2 (không hạ bừa).
                    return Err(CodegenError::Unsupported("assign-optional", span));
                }
                buoc.push(Buoc::Truong(name.clone()));
                nut = base;
            }
            HExpr::Index { base, index, .. } => {
                buoc.push(Buoc::ChiSo(index));
                nut = base;
            }
            HExpr::Var { sym: Some(id), .. } => {
                buoc.reverse();
                return Ok((*id, buoc));
            }
            _ => return Err(CodegenError::Unsupported("assign-target", span)),
        }
    }
}

impl<'g> super::Emit<'g> {
    /// Hạ đích **đường** + vế phải. `op = Set` hoặc gán ghép trên **lá**.
    pub(super) fn gan_duong(
        &mut self,
        target: &HExpr,
        op: AssignOp,
        value: &HExpr,
        span: Span,
    ) -> Result<(), CodegenError> {
        let (id, buoc) = thu_duong(target, span)?;
        let slot = self.slot_cua(id)?;
        // `a0`: giá trị gốc, đọc **một lần** trước mọi chỉ số/vế phải (như gán một tầng).
        self.f.push(Instr::LoadLocal(slot), span, 1);
        let n = buoc.len();
        for (k, b) in buoc.iter().enumerate() {
            if k + 1 == n {
                // Lá: đẩy khoá lá + (giá trị đọc cũ nếu gán ghép) rồi vế phải.
                if matches!(op, AssignOp::Set) {
                    match b {
                        Buoc::Truong(name) => {
                            let c = self.g.cst_str(name);
                            self.f.push(Instr::Const(c), span, 1);
                        }
                        Buoc::ChiSo(idx) => self.expr(idx)?,
                    }
                    self.expr(value)?;
                    self.lift_assign_set(target, span)?;
                } else {
                    match b {
                        // `[a, tên, a.tên]` — `Field` dùng const, `Swap` đưa khoá vào giữa.
                        Buoc::Truong(name) => {
                            self.f.push(Instr::Dup, span, 1);
                            let c = self.g.cst_str(name);
                            self.f.push(Instr::Field(c), span, 0);
                            let c2 = self.g.cst_str(name);
                            self.f.push(Instr::Const(c2), span, 1);
                            self.f.push(Instr::Swap, span, 0);
                        }
                        // Chỉ số đánh giá **một lần** vào ô tạm (cần cho cả đọc cũ và `Set`).
                        Buoc::ChiSo(idx) => {
                            let tam = self.f.tam()?;
                            self.expr(idx)?;
                            self.f.push(Instr::StoreLocal(tam), span, -1);
                            self.f.push(Instr::Dup, span, 1);
                            self.f.push(Instr::LoadLocal(tam), span, 1);
                            self.f.push(Instr::Index, span, -1);
                            self.f.push(Instr::LoadLocal(tam), span, 1);
                            self.f.push(Instr::Swap, span, 0);
                        }
                    }
                    self.expr(value)?;
                    let bop = assign_binop(op)?;
                    self.f.push(Instr::Bin(bop), span, -1);
                }
            } else {
                // Tầng giữa: giữ `a(k-1)` và khoá, tính `a(k)`, xếp `[a(k-1), khoá, a(k)]`.
                match b {
                    Buoc::Truong(name) => {
                        self.f.push(Instr::Dup, span, 1);
                        let c = self.g.cst_str(name);
                        self.f.push(Instr::Field(c), span, 0);
                        let c2 = self.g.cst_str(name);
                        self.f.push(Instr::Const(c2), span, 1);
                        self.f.push(Instr::Swap, span, 0);
                    }
                    Buoc::ChiSo(idx) => {
                        // `Index` tiêu thụ **cả** base lẫn chỉ số ⇒ giữ chỉ số trong ô tạm để
                        // (a) không đánh giá lại, (b) còn khoá cho bước gộp `Set` ở trên.
                        let tam = self.f.tam()?;
                        self.expr(idx)?;
                        self.f.push(Instr::StoreLocal(tam), span, -1);
                        self.f.push(Instr::Dup, span, 1);
                        self.f.push(Instr::LoadLocal(tam), span, 1);
                        self.f.push(Instr::Index, span, -1);
                        self.f.push(Instr::LoadLocal(tam), span, 1);
                        self.f.push(Instr::Swap, span, 0);
                    }
                }
            }
        }
        // Gộp từ lá lên: mỗi `With`/`Set` thay bộ ba `[gốc, khoá, giá trị]` bằng gốc mới.
        for b in buoc.iter().rev() {
            match b {
                Buoc::Truong(_) => self.f.push(Instr::With(1), span, -2),
                Buoc::ChiSo(_) => self.f.push(Instr::Set, span, -2),
            }
        }
        // ST-276: E_ASSIGN sau giá trị + đích (mirror nhánh `Var`, và interp).
        self.f.charge(FeeKind::Assign, span);
        self.f.push(Instr::Assign(slot), span, -1);
        self.net_stack(span)
    }
}
