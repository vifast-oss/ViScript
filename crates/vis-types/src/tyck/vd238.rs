// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-238** helper — tách khỏi `mod.rs` để giữ **R1** ≤ 1.500.
//!
//! `None`-trần (`HExpr::Var{name:"None"}`) chỉ hợp lệ khi **kiểu đích ngoài cùng** là `Option`
//! (`T?`, `T??`, `T???`, …) — **VD-241 §7 (A0, 2026-11-19)**.

use super::*;

pub(super) fn la_none_bare(e: &HExpr) -> bool {
    matches!(e, HExpr::Var { name, .. } if name == "None")
}

/// **VD-241 §7 (A0, 2026-11-19)** — `None` trần hợp lệ khi **kiểu đích ngoài cùng** là
/// `Type::Opt` (`T?`, `T??`, `T???`, …): `T?? = Option[Option[T]]` lồng thật, không flatten.
/// `Option` lồng **bên trong** một kiểu non-Option (`list[int?]`, `map[str, int?]`,
/// `Result[.., int?]`) **không** được nới — vẫn VS0412.
/// Alias/type-alias (`type X = Option[…]`), `Option[T]`, và `int?`-cú pháp đều phân giải về
/// `Type::Opt(_)` ở bước phân giải annot (`resolve_ann`/`resolve_named`/lower) trước khi tới đây
/// ⇒ chấp nhận sau bước phân giải đó. Tham số kiểu đang mở (`T` trong `fn f[T]`) vẫn là
/// `TypeRef` (chưa biết là Option) ⇒ từ chối. `Never`/`Error` giữ nguyên như VD-238.
pub(super) fn la_none_dich_opt(to: &Type) -> bool {
    matches!(to, Type::Opt(_) | Type::Never | Type::Error)
}

/// `true` nếu biểu thức là **literal số nguyên** (§3.5: chỉ literal được nâng `int → float`).
pub(crate) fn is_int_lit(e: &HExpr) -> bool {
    matches!(e, HExpr::Lit { lit, .. } if lit.kind == crate::hir::LitKind::Int)
}

/// Kiểu **không thuộc VS0** (VD-48(a)) — vẫn `VS0301` nhưng thông điệp nêu rõ vis-std/vòng 2.
pub(crate) fn la_kieu_ngoai_vs0(name: &str) -> bool {
    matches!(
        name,
        // **VD-222**: `AnyError` và `Json` nay là dựng sẵn — bỏ khỏi danh sách "ngoài VS0".
        "cap" | "ref" | "weak" | "set" | "stream" | "Future" | "datetime" | "bigint" | "decimal"
    )
}

/// **ST-60b**: gốc của **đích gán** — đi xuống qua `Field`/`Index` tới `Var`.
pub(super) fn goc_dich(t: &HExpr) -> Option<u32> {
    match t {
        HExpr::Var { sym, .. } => *sym,
        HExpr::Field { base, .. } | HExpr::Index { base, .. } => goc_dich(base),
        _ => None,
    }
}

/// **ST-60b**: đích mà **codegen** hạ được: biến; **một** tầng trường (`x.f`); **một** chỉ số trên
/// biến (`x[i]`); hoặc **một** chỉ số trên **một** tầng trường (`x.f[i]`). Sâu hơn ⇒ `GAN_LONG`.
pub(super) fn dich_ha_duoc(t: &HExpr) -> bool {
    match t {
        HExpr::Var { sym: Some(_), .. } => true,
        HExpr::Field { base, .. } => matches!(&**base, HExpr::Var { sym: Some(_), .. }),
        HExpr::Index { base, .. } => match &**base {
            HExpr::Var { sym: Some(_), .. } => true,
            HExpr::Field { base: b2, .. } => matches!(&**b2, HExpr::Var { sym: Some(_), .. }),
            _ => false,
        },
        _ => false,
    }
}
