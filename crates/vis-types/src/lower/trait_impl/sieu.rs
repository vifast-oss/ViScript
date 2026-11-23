// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-311d (VD-286)** — kiểm **siêu trait** một tầng cho `impl Trait for Type` (ST-57/VD-98).
//!
//! Tách **nguyên văn** khỏi `lower/trait_impl.rs` để giữ mỗi tệp ≤ 1.150 dòng.

use super::*;

// ---------------------------------------------------------------------------
// ST-57 (VD-98 (2)) — siêu trait
// ---------------------------------------------------------------------------

impl<'a> Lowerer<'a> {
    /// **ST-57 (VD-98 (2))**: mỗi `impl Trait for Type`, mọi **siêu trait** của `Trait` phải có
    /// `impl` cùng kiểu — kiểm **một tầng** (bắc cầu = nợ), `VS0403` (không cấp mã mới).
    pub(crate) fn kiem_sieu_trait(&mut self) {
        let pairs = self.impl_pairs.clone();
        for (ty, tn, span) in &pairs {
            let sieu: Vec<String> = self
                .traits
                .iter()
                .find(|t| &t.name == tn)
                .or_else(|| self.traits_ngoai.iter().find(|t| &t.name == tn))
                .map(|t| t.supertraits.clone())
                .unwrap_or_default();
            for st in sieu {
                if !pairs.iter().any(|(t2, n2, _)| n2 == &st && t2 == ty) {
                    self.diag(
                        &vis_diag::TYPE_CODES[2],
                        format!(
                            "`impl {tn} for …` thiếu `impl {st} for …` (siêu trait `{st}` — kiểm **một tầng**, VD-98)"
                        ),
                        *span,
                    );
                }
            }
        }
    }
}
