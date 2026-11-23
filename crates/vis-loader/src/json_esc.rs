// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310a (VD-285), đóng nợ #112** — escape JSON **một nguồn** nằm ở
//! `vis_core::json::esc`. Tệp này chỉ giữ đường dẫn cũ `vis_loader::json_esc::esc`
//! (dùng bởi `vis-cli check/test/doctor`); test/fuzz 100k đã chuyển sang crate lõi.

pub use vis_core::json::esc;
