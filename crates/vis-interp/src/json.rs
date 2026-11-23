// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310a (VD-285)** — `json` chuyển sang `vis-core` (một bộ `json_esc` duy nhất, đóng #112).
//! Đường dẫn công khai `vis_interp::json::*` giữ nguyên qua re-export.

pub use vis_core::json::*;
