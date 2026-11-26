// SPDX-License-Identifier: MIT OR Apache-2.0
//! HIR của **chế độ script** (ST-11) — mặt nạ *đã hạ* từ CST, giữ `Span`, đã giải tên **giá trị**
//! và đã điền kiểu cho literal (VD-44 #1: hạ **permissive**, biểu diễn được mọi thứ ở Phụ lục A;
//! giới hạn VS0 chỉ báo ở tầng kiểm kiểu).
//!
//! # Quy ước sexp HIR
//!
//! `sexp` là **dạng ngoặc hoá** dùng cho test/log, cùng quy ước với VD-34 (`golden.expr`) — chỉ
//! khác **một** điểm: HIR **không có** nút `paren` (ngoặc tròn là chuyện của CST), nên
//! `(1 + 2) * 3` ở CST là `(* (paren (+ 1 2)) 3)` thì ở HIR là `(* (+ 1 2) 3)`.
//! Literal in **nguyên văn nguồn** (`1`, `"a"`, `50pct`, `true`); các toán tử giữ nguyên chữ viết.
//!
//! **ST-318d**: nội dung tách theo nhóm — `item` (khai báo cấp tệp), `stmt` (câu lệnh), `pat`
//! (pattern), `expr` (biểu thức + toán tử), `lit` (literal). Tệp này chỉ giữ doc, khai báo module
//! và `pub use`; mọi đường dẫn `vis_hir::hir::*` **không đổi**.

mod expr;
mod item;
mod lit;
mod pat;
mod stmt;

// `hir/tests.rs` dùng `use super::*` và cần `Span`/`Type` như bản `hir.rs` cũ.
#[cfg(test)]
use crate::ty::Type;
#[cfg(test)]
use vis_syntax::Span;

pub use expr::{BinOp, HBody, HExpr, InterpPart, UnOp};
pub use item::{
    FnKind, HAlias, HConst, HEnum, HField, HFn, HImpl, HItem, HParam, HRec, HSig, HTrait, HUse,
    HVariant, Hir,
};
pub use lit::{Lit, LitKind};
pub use pat::HPat;
pub use stmt::{AssignOp, HArm, HCond, HElse, HStmt};

#[cfg(test)]
mod tests;
