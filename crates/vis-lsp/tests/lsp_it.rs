// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296a** — binary gộp miền: Language server LSP/JSON-RPC.
//!
//! Các tệp ca gốc nằm trong thư mục cùng tên; `#[path]` giữ nguyên nội dung, không copy test.
//! Tổng số ca/assertion không đổi — chỉ đổi crate test binary.

mod common;

#[path = "lsp_it/lsp.rs"]
mod lsp;
#[path = "lsp_it/lsp_code_action.rs"]
mod lsp_code_action;
#[path = "lsp_it/lsp_codelens.rs"]
mod lsp_codelens;
#[path = "lsp_it/lsp_da_tep.rs"]
mod lsp_da_tep;
#[path = "lsp_it/lsp_extra.rs"]
mod lsp_extra;
#[path = "lsp_it/lsp_format.rs"]
mod lsp_format;
#[path = "lsp_it/lsp_host.rs"]
mod lsp_host;
#[path = "lsp_it/lsp_hover_builtin.rs"]
mod lsp_hover_builtin;
#[path = "lsp_it/lsp_hover_method.rs"]
mod lsp_hover_method;
#[path = "lsp_it/lsp_implementation.rs"]
mod lsp_implementation;
#[path = "lsp_it/lsp_incremental.rs"]
mod lsp_incremental;
#[path = "lsp_it/lsp_inlay_hint.rs"]
mod lsp_inlay_hint;
#[path = "lsp_it/lsp_selection.rs"]
mod lsp_selection;
#[path = "lsp_it/lsp_semantic_tokens.rs"]
mod lsp_semantic_tokens;
#[path = "lsp_it/lsp_type_definition.rs"]
mod lsp_type_definition;
#[path = "lsp_it/lsp_workspace.rs"]
mod lsp_workspace;
