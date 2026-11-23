// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-304 (VD-283/VD-284)** — `vish-edit`: ViShell MVP phần A.
//!
//! API tối thiểu cho `vis`/ViShell:
//! - [`Editor`]: buffer đa dòng/thụt lề, cursor, history, tìm mờ (`Ctrl-R`), `Ctrl-C` trả
//!   [`Action::Cancel`]; render màu bằng token **CST** của `vis-syntax`.
//! - [`History`]: append-only, tìm mờ subsequence.
//! - [`read_line`]: raw mode (`rustix::termios` Unix/macOS, `crossterm` Windows) + đọc phím.
//!
//! 0 `unsafe`; raw-mode guard khôi phục termios khi `Drop` (kể cả panic).

#![forbid(unsafe_code)]

pub mod editor;
pub mod history;
pub mod key;
pub mod render;
pub mod run;
pub mod term;

pub use editor::{Action, Editor};
pub use history::History;
pub use key::{parse_keys, read_key, Key};
pub use run::{read_line, Outcome};
