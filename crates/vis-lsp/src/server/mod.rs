// SPDX-License-Identifier: MIT OR Apache-2.0
//! Tầng server LSP (tách khỏi `lib.rs` ở `ST-317b`): vòng lặp stdio + dispatch, kết quả
//! `initialize`, chuyển đổi URI. `mod.rs` chỉ khai báo + re-export.
//!
//! `run` được `lib.rs` re-export nguyên tên ⇒ `vis_lsp::run` không đổi.

mod khoi_tao;
mod run;
pub(crate) mod uri;

pub use run::run;
