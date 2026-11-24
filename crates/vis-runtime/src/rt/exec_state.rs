// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-3b/W5 (P1/P2)** — chế độ evaluator `Local` (W=1) và `Parallel(n)` (W>1).
//!
//! `Off` mặc định đi nguyên văn đường cũ. `Local` chạy trên caller, shared env (P1).
//! `Parallel` chạy task CPU trên `WorkerPool`: mỗi job dựng `Interp` riêng từ `Arc<HirImage>`,
//! nạp snapshot top-level (`SendValue::Map`), in vào buffer rồi merge **spawn order**, lease
//! `SharedBudget` theo task, cancel hợp tác và từ chối ghi top-level (cross-write).
#![allow(dead_code)]

use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use vis_hir::{Hir, Symbols, TypeTable};
use vis_syntax::Span;

use vis_core::trap::Trap;

use crate::codes::RT;
use crate::rt::fee::SharedBudget;
use crate::worker::{OutcomeBoard, SendValue, TaskHandle, TaskId, WorkerError, WorkerPool};
use vis_core::value::Task;

/// Ảnh chương trình **bất biến** chia sẻ cho mọi worker (P2).
#[derive(Debug)]
pub struct HirImage {
    pub hir: Hir,
    pub symbols: Symbols,
    pub types: TypeTable,
    /// Nguồn để chẩn đoán (worker hiện chỉ dùng cho tương lai; không đổi semantics).
    pub text: Arc<str>,
}

/// Chế độ evaluator. `Parallel` chỉ nhận task **CPU thuần** trong P2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkerMode {
    #[default]
    Off,
    Local,
    Parallel(usize),
}

/// Proxy `Task.id` (main) ↔ job của pool.
pub struct TaskProxy {
    pub task: Rc<Task>,
    pub pool_id: TaskId,
    pub handle: TaskHandle,
    /// Output đã append vào `host.out` (flush đúng một lần).
    pub flushed: bool,
    /// **ST-292 (P4)**: outcome đã được pump lấy (output/dirty nằm trong proxy).
    pub done: bool,
    pub output: String,
    pub dirty: Vec<String>,
    pub cancelled: bool,
}

/// **ST-291 (P2b-1)** — job đã tạo `Task` nhưng **chưa gửi** pool.
///
/// Snapshot top-level vẫn chụp tại thời điểm `spawn` (đúng semantics P2); chỉ việc gửi
/// xuống `WorkerPool` được hoãn tới `await`/final join để gửi **một lô**.
pub struct PendingJob {
    pub task: Rc<Task>,
    pub than: u32,
    pub args: Vec<SendValue>,
    pub snap: Arc<SendValue>,
    pub span: Span,
}

/// Trạng thái điều phối tác vụ của một `Scheduler`.
#[derive(Default)]
pub struct ExecState {
    pub mode: WorkerMode,
    /// `Task.id` theo thứ tự spawn.
    pub order: Vec<u32>,
    /// Ảnh HIR dùng cho runner worker; dựng một lần mỗi lần chạy.
    pub image: Option<Arc<HirImage>>,
    /// Ngân sách atomic (P2); `None` ⇒ không charge.
    pub budget: Option<Arc<SharedBudget>>,
    /// Test hook: huỷ mọi task trước khi worker chạy (P2 cancel pre-run).
    pub cancel_before: bool,
    /// Test hook: huỷ các task đang chạy sau mốc thời gian (P2 cancel-after).
    pub cancel_deadline: Option<Instant>,
    pub pool: Option<WorkerPool>,
    pub outcome_board: Option<Arc<OutcomeBoard>>,
    /// **ST-291 (P2b-2)**: proxy theo `Task.id` (dày, bắt đầu từ 1) — thay `HashMap`.
    pub proxy: Vec<Option<TaskProxy>>,
    /// **ST-291 (P2b-1)**: job đã tạo `Task` nhưng chưa gửi pool.
    pub pending: Vec<PendingJob>,
    /// **ST-291 (P2b-2)**: `Task.id` theo đúng thứ tự gửi pool — thay `HashMap` task↔pool.
    pub pool_order: Vec<u32>,
    /// **ST-292 (P4)**: `Task.id` là worker task (spawn từ **main frame**) — fallback frame
    /// không được dựng frame chạy lại thân.
    pub worker_ids: Vec<bool>,
}

impl fmt::Debug for ExecState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExecState")
            .field("mode", &self.mode)
            .field("order_len", &self.order.len())
            .field(
                "proxy_len",
                &self.proxy.iter().filter(|p| p.is_some()).count(),
            )
            .field("pending_len", &self.pending.len())
            .field("pool_order_len", &self.pool_order.len())
            .field(
                "worker_ids",
                &self.worker_ids.iter().filter(|w| **w).count(),
            )
            .field("budget", &self.budget.is_some())
            .finish()
    }
}

fn loi(msg: impl Into<String>, span: Span) -> Trap {
    Trap::new(RT, msg.into(), span)
}

fn loi_pool(e: WorkerError, span: Span) -> Trap {
    loi(format!("worker pool: {e:?}"), span)
}

impl ExecState {
    pub fn ensure_pool(&mut self) -> Result<(), String> {
        if self.pool.is_some() {
            return Ok(());
        }
        let WorkerMode::Parallel(n) = self.mode else {
            return Ok(());
        };
        let mut pool =
            WorkerPool::new(n, HashMap::new()).map_err(|e| format!("worker pool: {e:?}"))?;
        let board = Arc::new(OutcomeBoard::new());
        pool.attach_outcome_board(Arc::clone(&board));
        self.pool = Some(pool);
        self.outcome_board = Some(board);
        Ok(())
    }
}
