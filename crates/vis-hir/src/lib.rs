// SPDX-License-Identifier: MIT OR Apache-2.0
//! **`vis-hir` — dữ liệu thuần HIR/Type + bảng chữ ký `stdlib` (ST-312c/VD-289).**
//!
//! Tầng 1 của DAG đích: **không** phụ thuộc `vis-types`, không OS, không `unsafe`.
//! `vis-types` chỉ còn `lower` + `tyck`; `vis-core`/`vis-bytecode`/`vis-vm` đọc thẳng crate này.
//!
//! - [`hir`] — HIR (`Hir`, `HExpr`, `HStmt`, `HItem`, pattern…).
//! - [`ty`] — `Type` + `TypeTable` (bảng kiểu người dùng).
//! - [`symbols`] — `Symbols`/`HSym`/khung tên + `EnumTmpl`.
//! - [`cmdsig`] — chữ ký lệnh (`cmd`) cho chế độ lệnh.
//! - [`method_ret`] — bảng kiểu trả phương thức lõi.
//! - [`stdlib`] — bảng chữ ký `stdlib` + tên module.

pub mod cmdsig;
pub mod hir;
pub mod method_ret;
pub mod prelude;
pub mod stdlib;
pub mod symbols;
pub mod ty;

pub use hir::{
    AssignOp, BinOp, FnKind, HAlias, HArm, HBody, HCond, HConst, HElse, HEnum, HExpr, HField, HFn,
    HImpl, HItem, HParam, HPat, HRec, HSig, HStmt, HTrait, HUse, HVariant, Hir, InterpPart, Lit,
    LitKind, UnOp,
};
pub use method_ret::MethodRet;
pub use symbols::{EnumTmpl, FrameKind, HFrame, HSym, ImplDangKy, ModInfo, SymKind, Symbols};
pub use ty::{
    nang_optional, EnumDef, EnumId, FieldDef, RecDef, RecId, Type, TypeTable, VariantDef,
};
