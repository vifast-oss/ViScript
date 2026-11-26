// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-268 — đích gán là *đường***: `a.b.f`, `xs[0].f`, `m["k"].f`, `a.bs[i].f`, …
//! (tách khỏi `mod.rs` để giữ R1 ≤ 1.500).
//!
//! Luật:
//!   * Gốc **phải** là biến `var` (kể cả `var self`); `let`/tham số bất biến, biểu thức tạm
//!     (`f().x`), và `?.` trong đường đều bị từ chối **tĩnh** bằng `VS0412` (thông điệp riêng).
//!   * Mỗi bước `Field` đi qua **bản ghi** (`VS0405` nếu thiếu trường); mỗi bước `Index` đi qua
//!     `list`/`map` và **kiểm kiểu chỉ số/khoá** — lá trả về là kiểu đích để `HStmt::Assign`
//!     ép vế phải / kiểm `+=` … như gán biến.
//!   * `?.` **không** nằm trong đích gán: bóc `Option` trước (`match`/`??`/`?`) rồi gán.
//!
//! Ngữ nghĩa runtime là **giá trị** (đọc bản sao, dựng lại từng tầng) nên không aliasing; xem
//! `vis-interp::gan` và `vis-bytecode::codegen::gan_long`.

use vis_hir::hir::{AssignOp, HExpr};
use vis_hir::ty::Type;

use super::phuong_thuc::{GAN_PHAN_TU_LET, GAN_TRUONG_LET};
use super::{Checker, GAN_BAT_BIEN, MISMATCH};

/// Gốc là biểu thức không phải biến (lời gọi, literal, `with`, `if`, …) — không có ô để ghi.
pub(super) const GAN_TAM: &str = "không gán được cho biểu thức tạm — đích gán phải bắt đầu bằng biến `var` (hoặc `var self`), §4.1";
/// `?.` trong **đường** đích gán — `Option` không phải ô, không gán xuyên được.
pub(super) const GAN_OPTIONAL: &str =
    "đích gán không nhận `?.` — bóc `Option` bằng `match`/`??`/`?` rồi mới gán (§4.1)";

/// Span của **đoạn `?.`** đầu tiên trong đường đích (nếu có).
pub(super) fn diem_optional(t: &HExpr) -> Option<vis_syntax::Span> {
    match t {
        HExpr::Field {
            base,
            optional,
            span,
            ..
        } => {
            if *optional {
                Some(*span)
            } else {
                diem_optional(base)
            }
        }
        HExpr::Index { base, .. } => diem_optional(base),
        _ => None,
    }
}

impl<'a> Checker<'a> {
    /// Kiểu của một **đích gán** (chỉ đọc, không báo lỗi gán-cho-`let`).
    pub(crate) fn kiem_dich(&mut self, target: &HExpr, _op: AssignOp) -> Type {
        match target {
            HExpr::Var {
                sym: Some(id),
                span,
                ..
            } => {
                let ty = self.ty_of(*id);
                // **ST-316b (`#186`)**: `let` **cấp tệp** được đánh dấu trước khi kiểm thân hàm
                // ⇒ báo `VS0413` dù `ty_of` còn `{error}` (thân hàm kiểm trước câu lệnh cấp tệp).
                // Thông điệp/mã/vị trí trùng đường interp (`lvalue_set`).
                let let_cap_tep = self.let_cap_tep.get(*id as usize).copied().unwrap_or(false);
                if !self.is_mutable(*id) && (!ty.is_error() || let_cap_tep) {
                    let kind = self.symbols.get(*id).map(|s| s.kind);
                    if !matches!(kind, Some(crate::lower::SymKind::Var)) {
                        self.diag(
                            GAN_BAT_BIEN,
                            "không gán được cho `let`/tham số (chỉ `var` mới gán được, §4.1)",
                            *span,
                        );
                    }
                }
                ty
            }
            HExpr::Var { sym: None, .. } => Type::Error,
            HExpr::Field { .. } | HExpr::Index { .. } => {
                // Gốc: biến `var`/`var self`; không có gốc ⇒ tạm; bất biến ⇒ đúng loại lá.
                let Some(id) = super::vd238::goc_dich(target) else {
                    self.diag(MISMATCH, GAN_TAM, target.span());
                    return Type::Error;
                };
                if !self.is_mutable(id) {
                    // Giữ **nguyên** chẩn đoán của các dạng một tầng (sem fixture + VD-97):
                    // `let xs[0] = …` ⇒ `VS0413`; `let p.x = …` ⇒ "bản ghi bất biến" `VS0412`;
                    // đích **sâu** ⇒ thông điệp "`let`/tham số" `VS0412` (không báo oan "lồng").
                    let la_index = matches!(target, HExpr::Index { .. });
                    if super::vd238::dich_ha_duoc(target) {
                        if la_index {
                            self.diag(GAN_BAT_BIEN, GAN_PHAN_TU_LET, target.span());
                        } else {
                            self.diag(
                                MISMATCH,
                                "bản ghi là **bất biến** — dùng `with` để tạo bản sao rồi gán lại `var` (§3.2/§4.1)",
                                target.span(),
                            );
                        }
                    } else {
                        let msg = if la_index {
                            GAN_PHAN_TU_LET
                        } else {
                            GAN_TRUONG_LET
                        };
                        self.diag(MISMATCH, msg, target.span());
                    }
                    return Type::Error;
                }
                // `?.` không phải ô: từ chối trước khi kiểm kiểu từng bước.
                if let Some(sp) = diem_optional(target) {
                    self.diag(MISMATCH, GAN_OPTIONAL, sp);
                    return Type::Error;
                }
                let goc_ty = self.ty_of(id);
                self.kiem_duong(goc_ty, target)
            }
            _ => {
                // Literal/biểu thức (`1 = 2`, `a + b = …`) — không phải ô để ghi.
                self.diag(MISMATCH, GAN_TAM, target.span());
                Type::Error
            }
        }
    }

    /// Đi từ gốc ra lá: mỗi `Field`/`Index` thay kiểu hiện tại bằng kiểu trường/phần tử.
    fn kiem_duong(&mut self, ty: Type, t: &HExpr) -> Type {
        match t {
            HExpr::Field {
                base, name, span, ..
            } => {
                let bt = self.kiem_duong(ty, base);
                self.kiem_truong_dich(bt, name, *span)
            }
            HExpr::Index { base, index, span } => {
                let bt = self.kiem_duong(ty, base);
                self.kiem_chi_so_dich(bt, index, *span)
            }
            _ => ty,
        }
    }

    fn kiem_truong_dich(&mut self, bt: Type, name: &str, span: vis_syntax::Span) -> Type {
        match bt {
            Type::Error => Type::Error,
            Type::Record(id) => match self.types.field(id, name) {
                Some(f) => f.ty.clone(),
                None => {
                    self.diag(
                        vis_diag::ma_truong(),
                        format!("bản ghi không có trường `{name}` (VD-43 #6)"),
                        span,
                    );
                    Type::Error
                }
            },
            Type::Opt(_) => {
                let ten = self.show(&bt);
                self.diag(
                    MISMATCH,
                    format!(
                        "`{ten}` là optional — không gán xuyên `Option` trong đích gán; bóc bằng `match`/`??`/`?` (§4.1)"
                    ),
                    span,
                );
                Type::Error
            }
            other => {
                let ten = self.show(&other);
                self.diag(
                    MISMATCH,
                    format!("không gán được trường của `{ten}` (§4.1)"),
                    span,
                );
                Type::Error
            }
        }
    }

    fn kiem_chi_so_dich(&mut self, bt: Type, index: &HExpr, span: vis_syntax::Span) -> Type {
        // Luôn kiểm biểu thức chỉ số (bắt tên lạ / lỗi bên trong) một lần.
        let it = self.kiem_expr(index);
        match bt {
            Type::List(t) => {
                self.coerce(&it, &Type::Int, index.span(), "chỉ số `list`");
                *t
            }
            Type::Map(k, v) => {
                self.coerce(&it, &k, index.span(), "khoá `map`");
                *v
            }
            Type::Error => Type::Error,
            other => {
                let ten = self.show(&other);
                self.diag(
                    MISMATCH,
                    format!("không chỉ số được kiểu `{ten}` trong đích gán (§4.1)"),
                    span,
                );
                Type::Error
            }
        }
    }
}
