// SPDX-License-Identifier: MIT OR Apache-2.0
//! **R3a (VD-178)**: `if` **biểu thức** — tách khỏi `codegen.rs` để giữ **R1** ≤ 1.500 dòng.
//! `if let` để giá trị trên ngăn xếp cho nhánh `then` bind; nhánh `else`/`elif` phải `Pop` trước.

use super::*;

impl Emit<'_> {
    pub(super) fn expr_if(
        &mut self,
        cond: &HCond,
        then: &[HStmt],
        elifs: &[(HCond, Vec<HStmt>)],
        else_: &[HStmt],
        span: Span,
    ) -> Result<(), CodegenError> {
        let mut jend: Vec<usize> = Vec::new();
        let mut jf = self.cond(cond)?;
        let mut cond_let = matches!(cond, HCond::Let { .. });
        self.nhanh_gia_tri(then)?;
        jend.push(self.f.here());
        self.f.push(Instr::Jump(0), span, 0);
        let mut prev = jf.take();
        for (c, body) in elifs {
            let at = self.f.here();
            if let Some(p) = prev {
                self.f.patch(p, at);
            }
            if cond_let {
                self.f.push(Instr::Pop, span, -1);
            }
            let mut j = self.cond(c)?;
            cond_let = matches!(c, HCond::Let { .. });
            self.nhanh_gia_tri(body)?;
            jend.push(self.f.here());
            self.f.push(Instr::Jump(0), span, 0);
            prev = j.take();
        }
        let at = self.f.here();
        if let Some(p) = prev {
            self.f.patch(p, at);
        }
        if cond_let {
            self.f.push(Instr::Pop, span, -1);
        }
        self.nhanh_gia_tri(else_)?;
        let end = self.f.here();
        for j in jend {
            self.f.patch(j, end);
        }
        Ok(())
    }
}
