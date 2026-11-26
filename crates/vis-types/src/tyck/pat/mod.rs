// SPDX-License-Identifier: MIT OR Apache-2.0
//! Pattern: gán kiểu cho binding + **xét hết** `match` (`VS0402`, VD-43 #5).
//!
//! Luật xét hết: enum ⇒ **mọi** biến thể; `bool` ⇒ `true`/`false`; `T?` ⇒ `Some`/`None`
//! (`Some`/`None` cũng được); `Result` ⇒ `ok`/`err`; bản ghi ⇒ **đủ mọi trường, không `..`**;
//! kiểu vô hạn (`int`, `str`, `list`, `map`, …) ⇒ **bắt buộc `_`**; **guard không tính**;
//! or-pattern tính **cả hai** vế.
//!
//! **ST-317b**: tách `pat.rs` (1.140 dòng) thành `mod.rs` (khai báo + re-export) và các tệp
//! chuyên trách: [`gan`] (gán kiểu binding), [`xet_het`] (`match` xét hết), [`phu`] (helper phủ
//! dùng chung), [`or_bind`] (luật or-pattern), [`sym`] (duyệt use-set), [`phu_het`] (phủ hết cho
//! `let`/`for`). API + chẩn đoán không đổi.

mod gan;
mod or_bind;
mod phu;
mod phu_het;
mod sym;
mod xet_het;

pub(crate) use sym::{arm_uses, than_uses};
