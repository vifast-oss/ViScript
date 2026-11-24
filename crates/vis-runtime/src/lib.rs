// SPDX-License-Identifier: MIT OR Apache-2.0
//! **`vis-runtime` — runtime dùng chung interp + VM (ST-313a/VD-289).**
//!
//! Tầng 2 của DAG đích: scheduler tác vụ, worker pool, bảng ngữ nghĩa `builtin`/`stdlib`,
//! và render. **Không** phụ thuộc `vis-interp`; interp/VM chỉ cài trait [`builtin::Host`]
//! và gọi vào đây.

pub mod builtin;
pub mod codes;
pub mod iter;
pub mod lift;
pub mod lit;
pub mod net;
pub mod render;
pub mod rt;
pub mod stdlib;
pub mod worker;

pub use builtin::Host;
pub use iter::{iter_items, ForIter};
pub use rt::Scheduler;
