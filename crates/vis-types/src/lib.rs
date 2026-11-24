// SPDX-License-Identifier: MIT OR Apache-2.0
//! `vis-types` — hạ **CST → HIR** + kiểm kiểu VS0 (ST-11/ST-12).
//!
//! **ST-312c (VD-289)**: dữ liệu thuần HIR/`Type`/`TypeTable`/bảng chữ ký `stdlib` đã tách sang
//! crate [`vis_hir`]; crate này chỉ còn:
//!
//! - [`lower`] — hạ CST → HIR, giải tên giá trị, điền kiểu literal;
//! - [`tyck`] — kiểm kiểu VS0.
//!
//! Thiết kế §14.2: phụ thuộc `vis-hir` + `vis-syntax` + `vis-diag`, **không** phụ thuộc hệ điều
//! hành, không `unsafe`, tệp ≤ 1.500 dòng.
//!
//! # Ví dụ
//!
//! ```
//! use vis_syntax::{lex, parse, Mode};
//!
//! let src = "let x = 1\nlet y = x + 2\n";
//! let out = parse(src, Mode::Script);
//! assert!(out.diagnostics.is_empty());
//! let h = vis_types::lower(&out.cst, &lex(src, Mode::Script));
//! assert!(h.is_clean(), "chẩn đoán: {:?}", h.codes());
//! assert_eq!(h.hir.stmts.len(), 2);
//! let vis_hir::HStmt::Let { init, .. } = &h.hir.stmts[1] else {
//!     panic!("câu lệnh thứ hai phải là `let`");
//! };
//! let vis_hir::HExpr::Bin { lhs, .. } = init else {
//!     panic!("giá trị phải là phép cộng");
//! };
//! let vis_hir::HExpr::Var { name, sym, .. } = &**lhs else {
//!     panic!("vế trái phải là tên");
//! };
//! assert_eq!(name, "x");
//! assert!(sym.is_some(), "`x` phải phân giải được");
//! ```
//!
//! # Ví dụ lỗi (`VS0302`: `var` khai trùng cùng phạm vi)
//!
//! **R16 (VD-178)**: `let` **được che tên** trong cùng phạm vi; chỉ `var` mới báo trùng.
//!
//! ```
//! use vis_syntax::{lex, parse, Mode};
//!
//! let src = "var x = 1\nvar x = 2\n";
//! let out = parse(src, Mode::Script);
//! let h = vis_types::lower(&out.cst, &lex(src, Mode::Script));
//! assert_eq!(h.codes(), ["VS0302"]);
//! ```

pub mod lower;
pub mod tyck;

pub use lower::{
    is_builtin_value, lower, lower_into, lower_with, HirOutput, LowerError, LowerOptions,
    NamePolicy, TraitSig, BUILTIN_VALUE_NAMES,
};
pub use tyck::{check, CheckOutput};
