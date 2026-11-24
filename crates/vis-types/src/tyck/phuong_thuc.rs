// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-262** — thân **phương thức `impl`** được checker đi qua với **cùng ngữ cảnh** như hàm cấp
//! tệp: kiểu trả, tham số/`self`, phạm vi cục bộ, độ sâu vòng lặp (`break`/`continue`) và tên
//! chưa khai. Tách khỏi `mod.rs` để giữ R1 (≤1.500 dòng/tệp).
//!
//! Hai lượt:
//!   1. [`Checker::kiem_impl`] (lượt khai báo) — **đích gán** như ST-56b/VD-97, chạy **trước**
//!      codegen để interp ↔ `--vm` không lệch.
//!   2. [`Checker::kiem_impl_than`] (lượt kiểm) — toàn thân phương thức qua `flow.rs`/`EnvSnap`;
//!      phép gán đã do lượt 1 kiểm nên lượt 2 **lược chẩn đoán gán** (`bo_qua_gan_pt`) để không
//!      báo trùng, nhưng vẫn kiểm biểu thức hai bên.

use std::collections::HashSet;

use vis_hir::hir::{HExpr, HImpl, HStmt};

use super::{pat, vd238, Checker, GAN_BAT_BIEN, MISMATCH};

impl<'a> Checker<'a> {
    /// **ST-56b (VD-97)**: thân **phương thức `impl`** (khung VS1) — **phép gán** phải chặn
    /// **trước** để interp ↔ `--vm` không lệch:
    ///   * `self` **không** `var` là **tham số bất biến** ⇒ `self.x = v` / `self[i] = v` ⇒ **`VS0412`**
    ///     tại **phép gán**; `var self` **được** (rebind cục bộ, VD-97 (2)).
    ///   * `let`/tham số thường bất biến; `var` (kể cả biến cục bộ trong thân) **được**.
    ///   * gán ghép `+=` trên trường/phần tử **chưa** hỗ trợ ⇒ `VS0412`.
    pub(crate) fn kiem_impl(&mut self, im: &HImpl) {
        for m in &im.methods {
            let mut duoc_gan: HashSet<u32> = HashSet::new();
            for p in &m.params {
                if p.is_var_self {
                    duoc_gan.insert(p.sym);
                }
            }
            self.kiem_gan_than(&m.body, &mut duoc_gan);
        }
    }

    /// Duyệt thân phương thức, chỉ kiểm **đích gán**; theo dõi biến `var` để **không** báo oan.
    fn kiem_gan_than(&mut self, body: &[HStmt], duoc_gan: &mut HashSet<u32>) {
        for s in body {
            match s {
                HStmt::Var { sym, .. } => {
                    duoc_gan.insert(*sym);
                }
                HStmt::Assign { target, span, .. } => {
                    // **ST-60b**: gốc đi **xuống** qua `Field`/`Index` (không chỉ `Var` trực tiếp).
                    let goc_la_var = |t: &HExpr| {
                        vd238::goc_dich(t)
                            .map(|id| duoc_gan.contains(&id))
                            .unwrap_or(false)
                    };
                    // **VD-268**: đích lồng (`a.b.f`, `xs[0].f`, …) và gán ghép trên lá **được**
                    // hỗ trợ; ở lượt khai báo chỉ chặn **hình dạng gốc** (bất biến/tạm/`?.`).
                    // Kiểu từng bước: lượt kiểm thân (`kiem_impl_than`, `kiem_expr` trên đích)
                    // hoặc runtime — hai engine dùng **cùng** thông điệp nên không lệch.
                    match target {
                        HExpr::Var { sym: Some(id), .. } => {
                            if !duoc_gan.contains(id) {
                                self.diag(
                                    GAN_BAT_BIEN,
                                    "không gán được cho `let`/tham số (chỉ `var` mới gán được, §4.1)",
                                    *span,
                                );
                            }
                        }
                        HExpr::Field { .. } | HExpr::Index { .. } => {
                            if let Some(sp) = super::gan::diem_optional(target) {
                                self.diag(MISMATCH, super::gan::GAN_OPTIONAL, sp);
                            } else if !goc_la_var(target) {
                                if vd238::goc_dich(target).is_some() {
                                    let msg = match target {
                                        HExpr::Field { .. } => GAN_TRUONG_LET,
                                        _ => GAN_PHAN_TU_LET,
                                    };
                                    self.diag(GAN_BAT_BIEN, msg, *span);
                                } else {
                                    self.diag(MISMATCH, super::gan::GAN_TAM, *span);
                                }
                            }
                        }
                        HExpr::Error { .. } => {}
                        _ => self.diag(MISMATCH, super::gan::GAN_TAM, *span),
                    }
                }
                HStmt::If {
                    then, elifs, else_, ..
                } => {
                    self.kiem_gan_than(then, duoc_gan);
                    for (_, b) in elifs {
                        self.kiem_gan_than(b, duoc_gan);
                    }
                    if let Some(b) = else_ {
                        self.kiem_gan_than(b, duoc_gan);
                    }
                }
                HStmt::For { body, .. }
                | HStmt::While { body, .. }
                | HStmt::Loop { body, .. }
                | HStmt::Scope { body, .. }
                | HStmt::Block { body, .. } => self.kiem_gan_than(body, duoc_gan),
                HStmt::Match { arms, .. } => {
                    for a in arms {
                        self.kiem_gan_than(&a.body, duoc_gan);
                    }
                }
                _ => {}
            }
        }
    }

    /// **VD-262**: kiểm **toàn thân** phương thức với ngữ cảnh hàm — tham số/`self`, kiểu trả,
    /// phạm vi cục bộ (`EnvSnap`), `loop_depth = 0` (break không xuyên biên hàm) và use-set riêng.
    /// Phép gán đã do [`Self::kiem_impl`] kiểm ở lượt khai báo; ở đây chỉ kiểm biểu thức để không
    /// báo trùng (`bo_qua_gan_pt`). `self` giữ kiểu đã hạ (thường `{error}`) — định kiểu receiver
    /// đầy đủ vẫn là nợ riêng, không mở rộng trong VD-262.
    pub(crate) fn kiem_impl_than(&mut self, im: &HImpl) {
        for m in &im.methods {
            let luu = std::mem::take(&mut self.generic_scope);
            self.generic_scope = m.generics.clone();
            for p in &m.params {
                let raw = self.resolve_ann(&p.ty, p.span);
                let ty = self.chuan(&super::erase_gen(&raw, &m.generics));
                self.set_ty(p.sym, ty.clone());
                if p.is_var_self {
                    self.set_mutable(p.sym);
                }
                if let Some(d) = &p.default {
                    let dty = self.kiem_expr(d);
                    self.coerce_e(d, &dty, &ty, "giá trị mặc định");
                }
            }
            let raw_ret = self.resolve_ann(&m.ret, m.span);
            let ret = self.chuan(&super::erase_gen(&raw_ret, &m.generics));
            self.generic_scope = luu;

            let saved_ret = self.cur_ret.replace(ret.clone());
            // **VD-259/VD-262**: `break`/`continue` không xuyên biên hàm — đặt lại ngữ cảnh.
            let saved_loop = std::mem::replace(&mut self.loop_depth, 0);
            let saved_value = std::mem::replace(&mut self.in_value, false);
            let saved_uses = std::mem::replace(&mut self.fn_uses, pat::than_uses(&m.body));
            let saved_gan = std::mem::replace(&mut self.bo_qua_gan_pt, true);
            self.kiem_than(&m.body, &ret, m.span);
            self.bo_qua_gan_pt = saved_gan;
            self.fn_uses = saved_uses;
            self.in_value = saved_value;
            self.loop_depth = saved_loop;
            self.cur_ret = saved_ret;
        }
    }
}

/// **ST-56b (VD-97)**: thông điệp gán **trường** của `let`/tham số.
pub(crate) const GAN_TRUONG_LET: &str =
    "không gán được trường của `let`/tham số — chỉ `var` mới gán được (§4.1)";
/// Thông điệp gán **phần tử** của `let`/tham số.
pub(crate) const GAN_PHAN_TU_LET: &str =
    "chỉ gán được phần tử của `var` — `let`/tham số là bất biến (§4.1)";
