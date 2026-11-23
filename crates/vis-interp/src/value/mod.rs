// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310a (VD-285)** — `Value` chuyển sang `vis-core`; mô-đun này giữ phần **chỉ interp/VM**
//! dùng: bộ lặp `for` (cần trap runtime + trần vòng) và worker pool opt-in.
//!
//! Đường dẫn công khai `vis_interp::value::*` **không đổi**.

pub use vis_core::value::*;

mod iter;
pub use iter::{iter_items, ForIter};

/// **VS3-3b/W5** — worker pool thử nghiệm (chỉ test/opt-in), đường dẫn cũ
/// `vis_interp::value::worker::*`.
#[doc(hidden)]
pub mod worker;
pub use worker::TaskId;
