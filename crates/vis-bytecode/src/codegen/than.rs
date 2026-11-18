// SPDX-License-Identifier: MIT OR Apache-2.0
//! Thân hàm: `HBody::Expr` ⇒ biểu thức; `HBody::Stmts` ⇒ câu lệnh (giá trị cuối giữ lại).
//! Tách khỏi `codegen.rs` để giữ **R1** ≤ 1.500 dòng. **VD-228**: `-> T?` ⇒ tự nâng `Some(…)`
//! ở biên hàm (mirror interp ở `call_hfn_trong`).

use super::*;

impl Emit<'_> {
    /// **VD-228** — tự nâng `T → T?` ở biên hàm khi annot return là Optional.
    pub(super) fn lift_ret_opt(&mut self, span: Span) {
        if self.ret_opt {
            self.f.push(Instr::MakeSome, span, 0); // không bọc 2 lớp (Option ⇒ giữ phẳng)
        }
    }

    pub(super) fn than(&mut self, b: &HBody, _base: u16, span: Span) -> Result<(), CodegenError> {
        match b {
            HBody::Expr(e) => {
                self.expr(e)?;
                self.lift_ret_opt(span); // VD-228: biên hàm `-> T?`
                Ok(())
            }
            HBody::Stmts(st) => {
                // Câu lệnh cuối là biểu thức ⇒ **giữ** giá trị làm kết quả hàm (giống interp).
                self.nhanh_gia_tri(st)?;
                self.lift_ret_opt(span); // VD-228: biên hàm `-> T?`
                self.f.push(Instr::Return, span, 0);
                self.net_stack(span)
            }
        }
    }
}
