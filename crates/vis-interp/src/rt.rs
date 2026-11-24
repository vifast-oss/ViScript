// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-60 (VD-90/VD-95) · ST-313a (VD-289)** — cầu nối runtime tác vụ của interp.
//!
//! `Scheduler`/kênh/select/fee/worker pool nay ở `vis-runtime`; tệp này chỉ giữ
//! **orchestration worker của interp** (`exec`) + dispatcher `await/join` (Local/Parallel cần
//! runner cây) và re-export nội bộ để mã interp giữ đường gọi ngắn.

pub(crate) use vis_core::reactor::FrameReactorBackend;
pub(crate) use vis_core::value::{Pending, Task, TrangThai};

pub(crate) mod exec;
mod ops;

pub use vis_runtime::rt::chan;
pub use vis_runtime::rt::fee;
pub use vis_runtime::rt::frame_test::*;
pub use vis_runtime::rt::{tao_kenh, wrap_task_error};

pub use ops::{await_task, join_all};
pub use vis_runtime::rt::ops::{await_task_off, chay_mot, join_all_off, wrap_task_error_for_test};
pub use vis_runtime::rt::{Scheduler, MAX_TASK_NEST};
