// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310a (VD-285)** — `Value` chuyển sang `vis-core`; mô-đun này giữ phần **chỉ interp/VM**
//! dùng: bộ lặp `for` (cần trap runtime + trần vòng) và worker pool opt-in.
//!
//! **ST-312a (VD-289)**: re-export `vis_core::value` chỉ còn `pub(crate)` — crate ngoài
//! dùng trực tiếp `vis_core::value::*`; tại đây chỉ còn `iter`/`worker` là API của interp.

pub(crate) use vis_core::value::*;

mod iter;
pub use iter::{iter_items, ForIter};

/// **VS3-3b/W5** — worker pool thử nghiệm (chỉ test/opt-in), đường dẫn cũ
/// `vis_interp::value::worker::*`.
#[doc(hidden)]
pub mod worker;
pub use worker::TaskId;
