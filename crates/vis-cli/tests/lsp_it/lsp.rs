// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-67/ST-80/ST-300b** — test LSP; tách 2 module con để `lsp.rs` < 800 dòng (R1).

#[path = "lsp/lsp_part1.rs"]
mod part1;
#[path = "lsp/lsp_part2.rs"]
mod part2;

pub(crate) use part1::*;
