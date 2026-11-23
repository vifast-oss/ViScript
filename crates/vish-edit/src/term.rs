// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310b** — `vish-edit` dùng raw mode của tầng Terminal (`vish_term::RawMode`).
//!
//! Trước ST-310b tệp này giữ bản sao `rustix`/`crossterm`; nay chỉ còn **re-export** để
//! `vish_edit::term::RawMode` (đường cũ) vẫn biên dịch, còn hiện thực duy nhất nằm ở
//! `vish-term/src/raw.rs`.

pub use vish_term::RawMode;
