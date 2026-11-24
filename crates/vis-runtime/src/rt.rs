// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-313a (VD-289)** — runtime tác vụ dùng chung interp + VM.
//!
//! `Scheduler` một luồng, kênh/select, fee/ngân sách, reactor frame và worker pool.
//! Orchestration worker theo từng engine nằm ở `vis-interp`/`vis-vm`; crate này chỉ giữ
//! trạng thái + hàm thuần engine-agnostic.

pub mod chan;
pub mod exec_state;
pub mod fee;
pub mod frame_test;
pub mod ops;
pub mod reactor_io;
pub mod sched;
pub mod select;

pub use frame_test::*;
pub use ops::*;
pub use sched::Scheduler;
pub(crate) use sched::{FRAME_CANCEL_BYTES, FRAME_CANCEL_TEST, FRAME_CANCEL_TURNS};

pub use vis_core::reactor::{
    FrameReactorBackend, ReactorH2Outcome, ReactorH2Response, ReactorIoOutcome, ReactorOp,
};
pub use vis_core::value::{Chan, ChanSend, ChanSendState, Pending, Task, TrangThai, Wait};

/// Chặn **lồng** `await` (mỗi tầng là một khung gọi thật) — 256 như chốt VD-95 (7).
pub const MAX_TASK_NEST: usize = 256;
