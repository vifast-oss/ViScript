// SPDX-License-Identifier: MIT OR Apache-2.0
//! Thân hàm: `HBody::Expr` ⇒ biểu thức; `HBody::Stmts` ⇒ câu lệnh (giá trị cuối giữ lại).
//! Tách khỏi `codegen.rs` để giữ **R1** ≤ 1.500 dòng. **VD-228**: `-> T?` ⇒ tự nâng `Some(…)`
//! ở biên hàm (mirror interp ở `call_hfn_trong`).

use super::*;

/// Số lớp `Opt` của một **đích kiểu** — đếm từ ngoài vào (mirror vis-interp `opt_depth_ty`).
pub(super) fn opt_depth(t: &vis_types::Type) -> usize {
    let mut d = 0;
    let mut cur = t;
    while let vis_types::Type::Opt(inner) = cur {
        d += 1;
        cur = inner;
    }
    d
}

impl Emit<'_> {
    /// **VD-241 §3 (A0, 2026-11-19)** — nâng giá trị Opt tới **≥ `depth` lớp** (hàm. nội bộ
    /// `$lift-opt`, interp/VM dùng chung; người dùng không viết được tên `$…`).
    /// Stack trước: [v] → sau: [v'] với `v'` có tối thiểu `depth` lớp `Opt` từ ngoài vào.
    pub(super) fn lift_opt(&mut self, depth: usize, span: Span) -> Result<(), CodegenError> {
        let d = self.g.cst(vis_types::LitKind::Int, &depth.to_string());
        self.f.push(Instr::Const(d), span, 1);
        let n = self.g.cst_str("$lift-opt");
        self.f.push(Instr::CallBuiltin(n, 2), span, -1);
        Ok(())
    }

    /// **VD-228** (siết theo VD-241 §3) — tự nâng `T → …T?…` ở biên hàm khi annot return là Optional.
    pub(super) fn lift_ret_opt(&mut self, span: Span) -> Result<(), CodegenError> {
        if self.ret_depth > 0 {
            self.lift_opt(self.ret_depth, span)?;
        }
        Ok(())
    }

    /// **VD-228b** — tự nâng `T → T?` ở biên **tham số** annot Optional: giá trị plain vào
    /// ô cục bộ tham số ⇒ gói `Some(…)` (mirror interp ở `call_hfn_trong`).
    pub(super) fn lift_params_opt(
        &mut self,
        params: &[vis_types::HParam],
        span: Span,
    ) -> Result<(), CodegenError> {
        for p in params {
            let depth = opt_depth(&p.ty);
            if depth > 0 {
                let slot = self
                    .f
                    .locals
                    .get(&p.sym)
                    .copied()
                    .ok_or(CodegenError::Unsupported(
                        "tham số không có ô cục bộ",
                        p.span,
                    ))?;
                self.f.push(Instr::LoadLocal(slot), span, 1);
                self.lift_opt(depth, span)?; // VD-241 §3: nâng tới đủ lớp tham số.
                self.f.push(Instr::StoreLocal(slot), span, -1);
            }
        }
        Ok(())
    }

    pub(super) fn than(&mut self, b: &HBody, _base: u16, span: Span) -> Result<(), CodegenError> {
        match b {
            HBody::Expr(e) => {
                self.expr(e)?;
                self.lift_ret_opt(span)?; // VD-228 (siết VD-241 §3): biên hàm `-> T?`
                Ok(())
            }
            HBody::Stmts(st) => {
                // Câu lệnh cuối là biểu thức ⇒ **giữ** giá trị làm kết quả hàm (giống interp).
                self.nhanh_gia_tri(st)?;
                self.lift_ret_opt(span)?; // VD-228 (siết VD-241 §3): biên hàm `-> T?`
                self.f.push(Instr::Return, span, 0);
                self.net_stack(span)
            }
        }
    }

    /// **VD-241 §3+ (A0, 2026-11-19)** — nâng giá trị ở biên **annot `Option`** (`let`/`var`):
    /// giá trị đã có đủ lớp giữ nguyên; thiếu thì bọc `Some(…)` tới đủ lớp. `None` trần được
    /// giữ nguyên bởi `lift_opt` (bước 1 — không bọc).
    pub(super) fn lift_to_annot(
        &mut self,
        ty: Option<&vis_types::Type>,
        span: Span,
    ) -> Result<(), CodegenError> {
        if let Some(t) = ty {
            let depth = opt_depth(t);
            if depth > 0 {
                self.lift_opt(depth, span)?;
            }
        }
        Ok(())
    }

    /// **VD-241 §3+ (A0, 2026-11-19)** — số lớp `Opt` của annot của `sym` (0 nếu không có).
    /// Dùng ở biên **gán lại** (`x = Some(7)` với `x: int??`).
    pub(super) fn sym_opt_depth(&self, sym: u32) -> usize {
        self.g.sym_ty.get(&sym).map(opt_depth).unwrap_or(0)
    }

    /// **VD-241 §3+ (A0, 2026-11-19)** — nâng một lớp ở biên gán lại khi đích là `var` có annot
    /// `Option`; đích trường/chỉ số **chưa** nâng (xem báo cáo).
    pub(super) fn lift_assign_set(
        &mut self,
        target: &vis_types::HExpr,
        span: Span,
    ) -> Result<(), CodegenError> {
        if let vis_types::HExpr::Var { sym: Some(id), .. } = target {
            let depth = self.sym_opt_depth(*id);
            if depth > 0 {
                self.lift_opt(depth, span)?;
            }
        }
        Ok(())
    }
}
