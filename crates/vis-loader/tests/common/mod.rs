// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310e** — helper chung cho suite e2e chuyển từ `vis-cli/tests/`.
//!
//! `vis_bin()`/`test_child_bin()` tìm binary trong target workspace; xem
//! `test/support/vis_test_support.rs` để biết thứ tự ưu tiên.

#[path = "../../../../test/support/vis_test_support.rs"]
mod vis_test_support;

pub use vis_test_support::*;
