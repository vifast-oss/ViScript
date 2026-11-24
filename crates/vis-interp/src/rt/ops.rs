// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-313a (VD-289)** — dispatcher tác vụ của interp (Local/Parallel cần runner cây).
//!
//! Đường `Off` + tạo kênh nằm ở `vis-runtime`; tệp này chỉ chọn runner theo
//! `WorkerMode` và giữ API `crate::rt::{await_task, join_all}` cho interp.

use std::rc::Rc;

use vis_core::trap::R;
use vis_core::value::{Task, Value};
use vis_syntax::Span;

use vis_runtime::builtin::Host;
use vis_runtime::rt::exec_state::WorkerMode;
use vis_runtime::rt::{await_task_off, join_all_off};

use super::exec;

/// Chạy một tác vụ **tới hoàn tất** (hoặc trả cache) — dispatch theo `WorkerMode`.
pub fn await_task(host: &mut dyn Host, t: &Rc<Task>, span: Span) -> R<Value> {
    let mode = host.sched().borrow().exec.mode;
    match mode {
        WorkerMode::Local => exec::await_local(host, t, span),
        WorkerMode::Parallel(_) => exec::await_parallel(host, t, span),
        WorkerMode::Off => await_task_off(host, t, span),
    }
}

/// **Join** mọi tác vụ theo **thứ tự tạo** — dispatch theo `WorkerMode`.
pub fn join_all(host: &mut dyn Host) -> R<()> {
    let mode = host.sched().borrow().exec.mode;
    match mode {
        WorkerMode::Local => exec::join_local(host),
        WorkerMode::Parallel(_) => exec::join_parallel(host),
        WorkerMode::Off => join_all_off(host),
    }
}
