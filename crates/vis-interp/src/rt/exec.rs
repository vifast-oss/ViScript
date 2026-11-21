// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-3b/W5 (P1)** — chế độ evaluator `Local` (W=1) **trong crate**, không bật mặc định.
//!
//! W=1 chạy thân tác vụ **trên caller**, dùng đúng `Scheduler`/`Host::goi_than` của đường cũ:
//! không snapshot, không buffer output, không thread, không đổi `Value`/`Task`/`VERSION`.
//! `ExecState` chỉ giữ chế độ + thứ tự spawn để P2 gắn pool/proxy mà không đổi chữ ký.
//!
//! `WorkerMode::Off` là mặc định và đi **nguyên văn** đường cũ; `Local` chỉ thêm ghi `order`
//! và gọi runner [`crate::interp::worker::chay_than_local`] thay cho `Host::goi_than` trực tiếp.
#![allow(dead_code)]

use std::rc::Rc;

use vis_syntax::Span;

use crate::builtin::Host;
use crate::interp::R;
use crate::rt::Task;
use crate::value::Value;

/// Chế độ evaluator. P1 chỉ cần `Off`/`Local`; P2 mới thêm `Parallel(usize)` cùng pool/board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum WorkerMode {
    #[default]
    Off,
    Local,
}

/// Trạng thái điều phối tác vụ của một `Scheduler`.
#[derive(Debug, Default)]
pub(crate) struct ExecState {
    pub(crate) mode: WorkerMode,
    /// `Task.id` theo thứ tự spawn — P2 dùng cho final join/flush; P1 chỉ ghi để test.
    pub(crate) order: Vec<u32>,
}

/// Tạo tác vụ theo đúng đường cũ (`Scheduler::spawn`), ghi order khi `Local`.
/// `Off` giữ nguyên `ready`/`id`/hành vi của `HExpr::Spawn` trước P1.
pub(crate) fn spawn_local(
    host: &mut dyn Host,
    than: u32,
    args: Vec<Value>,
    ten: String,
    span: Span,
) -> R<Value> {
    let mut s = host.sched().borrow_mut();
    let t = s.spawn(than, args, ten, span);
    if s.exec.mode == WorkerMode::Local {
        s.exec.order.push(t.id);
    }
    Ok(Value::Task(t))
}

/// W=1: `await` chạy trên caller, dùng cache/trạng thái của đường cũ.
pub(crate) fn await_local(host: &mut dyn Host, t: &Rc<Task>, span: Span) -> R<Value> {
    crate::rt::await_task_off(host, t, span)
}

/// W=1: join mọi task theo thứ tự tạo, không thread/buffer.
pub(crate) fn join_local(host: &mut dyn Host) -> R<()> {
    crate::rt::join_all_off(host)
}
