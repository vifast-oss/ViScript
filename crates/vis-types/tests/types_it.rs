// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-311c (VD-286)** — binary gộp cho bộ kiểm thử tích hợp của `vis-types`.
//!
//! `#[path]` giữ nguyên từng tệp ca gốc trong `types_it/`; `stdlib_ty.rs` giữ riêng.

#[path = "types_it/cmd_lower.rs"]
mod cmd_lower;
#[path = "types_it/exhaustive.rs"]
mod exhaustive;
#[path = "types_it/generic.rs"]
mod generic;
#[path = "types_it/generic_that.rs"]
mod generic_that;
#[path = "types_it/hir.rs"]
mod hir;
#[path = "types_it/names.rs"]
mod names;
#[path = "types_it/stdlib_strmatches.rs"]
mod stdlib_strmatches;
#[path = "types_it/trait.rs"]
mod r#trait;
#[path = "types_it/tyck.rs"]
mod tyck;
#[path = "types_it/tyck_methods.rs"]
mod tyck_methods;
#[path = "types_it/tyck_v2.rs"]
mod tyck_v2;
