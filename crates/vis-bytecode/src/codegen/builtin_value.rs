// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-208 (2)** — vai trò dựng **giá trị dựng sẵn** của `Emit` (tách ra để giữ R1 ≤ 1.500 dòng).
//!
//! ISA không có mã `MakeBuiltin` nên giá trị dựng sẵn được dựng qua builtin nội bộ
//! `$value:<tên>` (host dựng `Value::Builtin`); `None` vẫn là literal của enum `Option`.

use super::*;

impl Emit<'_> {
    /// Builtin là giá trị dựng sẵn ở cả hai đường — dựng qua `$value:<tên>`; `None` là literal.
    pub(super) fn gia_tri_builtin_var(
        &mut self,
        id: u32,
        span: Span,
    ) -> Result<bool, CodegenError> {
        match self.g.builtins.get(&id) {
            Some(name) if name != "None" && !self.g.fmap.contains_key(&id) => {
                let n = self.g.cst_str(&format!("$value:{name}"));
                self.f.push(Instr::CallBuiltin(n, 0), span, 1);
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
