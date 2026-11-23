// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-311c (VD-286)** — binary gộp cho bộ kiểm thử tích hợp của `vis-syntax`.
//!
//! `#[path]` giữ nguyên từng tệp ca gốc trong `syntax_it/`; 0 test bị mất, giảm số lần link.

#[path = "syntax_it/cmd.rs"]
mod cmd;
#[path = "syntax_it/col_scalar.rs"]
mod col_scalar;
#[path = "syntax_it/doc_cmd.rs"]
mod doc_cmd;
#[path = "syntax_it/errors.rs"]
mod errors;
#[path = "syntax_it/expr.rs"]
mod expr;
#[path = "syntax_it/golden.rs"]
mod golden;
#[path = "syntax_it/invariants.rs"]
mod invariants;
#[path = "syntax_it/lex_float_dot.rs"]
mod lex_float_dot;
#[path = "syntax_it/nfc.rs"]
mod nfc;
#[path = "syntax_it/regress.rs"]
mod regress;
#[path = "syntax_it/vd37.rs"]
mod vd37;
