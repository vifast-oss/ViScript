// SPDX-License-Identifier: MIT OR Apache-2.0
//! Câu lệnh ở **ngữ cảnh có giá trị** (ST-20b/ST-60b/ST-238) — tách từ `codegen.rs` để giữ tệp
//! cha dưới ngưỡng R1. Dùng chung `Emit` + mọi phương thức riêng qua `super`.

use super::*;

impl Emit<'_> {
    /// Nhánh "có giá trị": câu lệnh cuối là `Expr` ⇒ **giữ** giá trị trên ngăn xếp.
    pub(super) fn nhanh_gia_tri(&mut self, body: &[HStmt]) -> Result<(), CodegenError> {
        if let Some((last, rest)) = body.split_last() {
            for s in rest {
                self.cau(s)?;
            }
            self.tra_gia_tri(last)
        } else {
            self.f.push(Instr::PushUnit, Span::new(0, 0), 1);
            Ok(())
        }
    }

    /// Như [`Emit::cau`] nhưng `HStmt::Expr` **không** bị `Pop` (để lại giá trị).
    pub(super) fn tra_gia_tri(&mut self, s: &HStmt) -> Result<(), CodegenError> {
        match s {
            HStmt::Expr(e) => {
                // **VD-257 (stream B)** — "câu cuối có giá trị" vẫn LÀ một câu nguồn; interp charge
                // `E_STMT` ở `exec_stmt`. VD-256 vá top-level nhưng sót `tra_gia_tri` (thân hàm,
                // nhánh `if` có giá trị) ⇒ parity lệch đúng 1 sự kiện mỗi lần. Không đổi opcode.
                self.f.charge(FeeKind::Stmt, s.span());
                self.expr(e)
            }
            HStmt::If {
                cond,
                then,
                elifs,
                else_,
                span,
            } => {
                let mut jend: Vec<usize> = Vec::new();
                let mut jf = self.cond(cond)?;
                // **R3a (VD-178)**: `if let` để giá trị cho nhánh `then` bind; nhánh `else`/`elif`
                // phải `Pop` nó trước.
                let mut cond_let = matches!(cond, HCond::Let { .. });
                self.nhanh_gia_tri(then)?;
                jend.push(self.f.here());
                self.f.push(Instr::Jump(0), *span, 0);
                let mut prev = jf.take();
                for (c, body) in elifs {
                    let at = self.f.here();
                    if let Some(p) = prev {
                        self.f.patch(p, at);
                    }
                    if cond_let {
                        self.f.push(Instr::Pop, *span, -1);
                    }
                    let mut j = self.cond(c)?;
                    cond_let = matches!(c, HCond::Let { .. });
                    self.nhanh_gia_tri(body)?;
                    jend.push(self.f.here());
                    self.f.push(Instr::Jump(0), *span, 0);
                    prev = j.take();
                }
                let at = self.f.here();
                if let Some(p) = prev {
                    self.f.patch(p, at);
                }
                if cond_let {
                    self.f.push(Instr::Pop, *span, -1);
                }
                match else_ {
                    Some(b) => self.nhanh_gia_tri(b)?,
                    // **ST-60b**: `if` **không** `else` ⇒ giá trị `()` thật (trước: `str("")`).
                    None => self.f.push(Instr::PushUnit, *span, 1),
                }
                let end = self.f.here();
                for j in jend {
                    self.f.patch(j, end);
                }
                Ok(())
            }
            HStmt::Match {
                subject,
                arms,
                span,
            } => {
                self.expr(subject)?;
                let mut jend = Vec::new();
                let mut guardas: Vec<usize> = Vec::new();
                let mut cuoi: Vec<(usize, Vec<usize>)> = Vec::new();
                for (i, arm) in arms.iter().enumerate() {
                    self.f.push(Instr::Dup, *span, 1);
                    let jf = self.test(&arm.pat, *span)?;
                    let br = self.f.here();
                    self.f.push(Instr::JumpIfFalse(0), *span, -1);
                    if let Some(j) = jf {
                        self.f.patch(j, br + 1);
                    }
                    // Bind **trước** guard (interp bind rồi mới xét guard); giữ subject để `Pop`.
                    self.bind(&arm.pat, *span, false)?;
                    if let Some(gd) = &arm.guard {
                        self.expr(gd)?;
                        let g = self.f.here();
                        self.f.push(Instr::JumpIfFalse(0), *span, -1);
                        guardas.push(g);
                    }
                    self.f.push(Instr::Pop, *span, -1);
                    self.nhanh_gia_tri(&arm.body)?;
                    jend.push(self.f.here());
                    self.f.push(Instr::Jump(0), *span, 0);
                    let next = self.f.here();
                    if i + 1 == arms.len() {
                        // Nhánh **cuối** trượt ⇒ `Pop` subject trước khi đẩy giá trị dự phòng.
                        cuoi.push((br, guardas.clone()));
                    } else {
                        self.f.patch(br, next);
                        for g in guardas.drain(..) {
                            self.f.patch(g, next);
                        }
                    }
                }
                let lpop = self.f.here();
                self.f.push(Instr::Pop, *span, -1);
                for (br, gs) in cuoi {
                    self.f.patch(br, lpop);
                    for g in gs {
                        self.f.patch(g, lpop);
                    }
                }
                // **ST-238 (VD-242, ca 7)** — ngữ cảnh giá trị: rơi nhánh cũng trap (interp làm
                // vậy ở `interp.rs:950`), không đẩy `()` rồi nuốt lỗi.
                self.trap_no_arm(*span, true)?;
                let end = self.f.here();
                for j in jend {
                    self.f.patch(j, end);
                }
                Ok(())
            }
            HStmt::Scope { body, .. } | HStmt::Block { body, .. } => self.nhanh_gia_tri(body),
            other => {
                // **ST-60b**: câu lệnh cuối **không** cho giá trị (vd `x = 5`) ⇒ `()` **thật** —
                // trước đây mã hoá `()` thành `Const(str "")` nên VM in chuỗi rỗng còn interp in `()`.
                self.cau(other)?;
                self.f.push(Instr::PushUnit, other.span(), 1);
                Ok(())
            }
        }
    }
}
