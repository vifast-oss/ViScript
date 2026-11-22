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
use std::time::{Duration, Instant};

use vis_syntax::Span;
use vis_types::{Hir, Symbols, TypeTable};

use crate::builtin::Host;
use crate::interp::{Interp, Trap, R, RT};
use crate::rt::fee::SharedBudget;
use crate::rt::Task;
use crate::value::worker::{OutcomeBoard, TaskHandle, TaskId, WaitError, WorkerError, WorkerPool};
use crate::value::Value;

/// Ảnh chương trình **bất biến** chia sẻ cho mọi worker (P2).
#[derive(Debug)]
pub(crate) struct HirImage {
    pub(crate) hir: Hir,
    pub(crate) symbols: Symbols,
    pub(crate) types: TypeTable,
    /// Nguồn để chẩn đoán (worker hiện chỉ dùng cho tương lai; không đổi semantics).
    pub(crate) text: Arc<str>,
}

/// Chế độ evaluator. `Parallel` chỉ nhận task **CPU thuần** trong P2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum WorkerMode {
    #[default]
    Off,
    Local,
    Parallel(usize),
}

/// Proxy `Task.id` (main) ↔ job của pool.
pub(crate) struct TaskProxy {
    pub(crate) task: Rc<Task>,
    pub(crate) pool_id: TaskId,
    pub(crate) handle: TaskHandle,
    pub(crate) flushed: bool,
}

/// Trạng thái điều phối tác vụ của một `Scheduler`.
#[derive(Default)]
pub(crate) struct ExecState {
    pub(crate) mode: WorkerMode,
    /// `Task.id` theo thứ tự spawn.
    pub(crate) order: Vec<u32>,
    /// Ảnh HIR dùng cho runner worker; dựng một lần mỗi lần chạy.
    pub(crate) image: Option<Arc<HirImage>>,
    /// Ngân sách atomic (P2); `None` ⇒ không charge.
    pub(crate) budget: Option<Arc<SharedBudget>>,
    /// Test hook: huỷ mọi task trước khi worker chạy (P2 cancel pre-run).
    pub(crate) cancel_before: bool,
    /// Test hook: huỷ các task đang chạy sau mốc thời gian (P2 cancel-after).
    pub(crate) cancel_deadline: Option<Instant>,
    pool: Option<WorkerPool>,
    pub(crate) outcome_board: Option<Arc<OutcomeBoard>>,
    pub(crate) proxy: HashMap<u32, TaskProxy>,
    pub(crate) pool_to_task: HashMap<TaskId, u32>,
}

impl fmt::Debug for ExecState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExecState")
            .field("mode", &self.mode)
            .field("order_len", &self.order.len())
            .field("proxy_len", &self.proxy.len())
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
    fn ensure_pool(&mut self) -> Result<(), String> {
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

/// Tạo tác vụ theo đúng đường cũ (`Scheduler::spawn`), ghi order khi `Local`.
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

/// **P2**: spawn task CPU lên pool. Snapshot top-level + args phải `Sendable`; nếu không ⇒
/// trap `VS0508` **trước khi** tạo job.
pub(crate) fn spawn_parallel(
    it: &mut Interp<'_>,
    than: u32,
    args: Vec<Value>,
    ten: String,
    span: Span,
) -> R<Value> {
    let snap = crate::interp::worker::snapshot_top(it).map_err(|m| {
        crate::interp::Abort::Trap(Trap::new(
            RT,
            format!("snapshot worker lỗi (VS0508): {m}"),
            span,
        ))
    })?;
    let mut args_send = Vec::with_capacity(args.len());
    for v in args {
        args_send.push(v.to_send().map_err(|m| {
            crate::interp::Abort::Trap(Trap::new(
                RT,
                format!("đối số không Sendable (VS0508): {m}"),
                span,
            ))
        })?);
    }

    let t = it.sched.borrow_mut().tao(than, Vec::new(), ten, span);
    let (image, budget, cancel_before, deadline) = {
        let s = it.sched.borrow();
        (
            s.exec.image.clone().ok_or_else(|| {
                crate::interp::Abort::Trap(loi("worker Parallel thiếu HirImage (P2)", span))
            })?,
            s.exec.budget.clone(),
            s.exec.cancel_before,
            s.exec.cancel_deadline,
        )
    };

    let mut s = it.sched.borrow_mut();
    s.exec
        .ensure_pool()
        .map_err(|m| crate::interp::Abort::Trap(loi(format!("worker pool: {m}"), span)))?;
    let handle = {
        let pool =
            s.exec.pool.as_mut().ok_or_else(|| {
                crate::interp::Abort::Trap(loi("worker Parallel thiếu pool", span))
            })?;
        let image = Arc::clone(&image);
        pool.spawn_shared_with_cancel(
            snap,
            move |ctx, snap| {
                crate::interp::worker::chay_than_worker(
                    &image, than, args_send, span, ctx, snap, budget, deadline,
                )
            },
            cancel_before,
        )
        .map_err(|e| crate::interp::Abort::Trap(loi_pool(e, span)))?
    };
    let pool_id = handle.id();
    s.exec.pool_to_task.insert(pool_id, t.id);
    s.exec.proxy.insert(
        t.id,
        TaskProxy {
            task: Rc::clone(&t),
            pool_id,
            handle,
            flushed: false,
        },
    );
    s.exec.order.push(t.id);
    Ok(Value::Task(t))
}

/// W=1: `await` chạy trên caller, dùng cache/trạng thái của đường cũ.
pub(crate) fn await_local(host: &mut dyn Host, t: &Rc<Task>, span: Span) -> R<Value> {
    crate::rt::await_task_off(host, t, span)
}

/// **P2**: `await` task worker — chờ outcome board, flush output tại điểm tiêu thụ, cache kết quả.
pub(crate) fn await_parallel(host: &mut dyn Host, t: &Rc<Task>, span: Span) -> R<Value> {
    let (pool_id, board) = {
        let s = host.sched().borrow();
        let Some(p) = s.exec.proxy.get(&t.id) else {
            return Err(crate::interp::Abort::Trap(loi(
                format!("task worker `{}` thiếu proxy", t.ten),
                span,
            )));
        };
        (p.pool_id, s.exec.outcome_board.clone())
    };
    let Some(board) = board else {
        return Err(crate::interp::Abort::Trap(loi(
            "worker Parallel thiếu outcome board",
            span,
        )));
    };
    let outcome = loop {
        match board.wait_timeout(pool_id, Duration::from_millis(50)) {
            Ok(o) => break o,
            Err(WaitError::Timeout) => continue,
            Err(WaitError::Closed) => {
                return Err(crate::interp::Abort::Trap(loi(
                    "worker pool đóng bất ngờ khi await",
                    span,
                )))
            }
        }
    };
    {
        let mut s = host.sched().borrow_mut();
        if let Some(p) = s.exec.proxy.get_mut(&t.id) {
            p.flushed = true;
        }
    }
    host.out().push_str(&outcome.output);

    let tr = if !outcome.dirty.is_empty() {
        Some(loi(
            format!(
                "ghi chéo worker (W>1): biến {} (task `{}`)",
                outcome.dirty.join(", "),
                t.ten
            ),
            span,
        ))
    } else if outcome.cancelled {
        Some(Trap::new(
            crate::interp::TASK_ERR,
            "tác vụ bị huỷ".to_string(),
            t.span,
        ))
    } else {
        None
    };
    if let Some(tr) = tr {
        *t.kq.borrow_mut() = Some(Err(tr.clone()));
        t.trang_thai.set(crate::rt::TrangThai::Xong);
        return Err(crate::interp::Abort::Trap(tr));
    }
    match outcome.result {
        Ok(sv) => {
            let v = sv.to_value();
            *t.kq.borrow_mut() = Some(Ok(v.clone()));
            t.trang_thai.set(crate::rt::TrangThai::Xong);
            Ok(v)
        }
        Err(m) => {
            let tr = Trap::new(
                crate::interp::TASK_ERR,
                format!("tác vụ `{}` lỗi: {m}", t.ten),
                t.span,
            );
            *t.kq.borrow_mut() = Some(Err(tr.clone()));
            t.trang_thai.set(crate::rt::TrangThai::Xong);
            Err(crate::interp::Abort::Trap(tr))
        }
    }
}

/// W=1: join mọi task theo thứ tự tạo, không thread/buffer.
pub(crate) fn join_local(host: &mut dyn Host) -> R<()> {
    crate::rt::join_all_off(host)
}

/// **P2**: join cuối — chờ pool, flush output task chưa await theo spawn order, cache, cross-write.
pub(crate) fn join_parallel(host: &mut dyn Host) -> R<()> {
    let (pool, proxy, pool_to_task) = {
        let mut s = host.sched().borrow_mut();
        let pool = s.exec.pool.take();
        let proxy = std::mem::take(&mut s.exec.proxy);
        let map = std::mem::take(&mut s.exec.pool_to_task);
        (pool, proxy, map)
    };
    let Some(pool) = pool else {
        return Ok(());
    };
    let outcomes = match pool.join() {
        Ok(r) => r.outcomes,
        Err(WorkerError::CrossWrite { vars, tasks }) => {
            return Err(crate::interp::Abort::Trap(loi(
                format!(
                    "ghi chéo worker (W>1): biến {} (task {:?})",
                    vars.join(", "),
                    tasks
                ),
                Span::new(0, 0),
            )))
        }
        Err(e) => {
            return Err(crate::interp::Abort::Trap(loi(
                format!("worker pool join: {e:?}"),
                Span::new(0, 0),
            )))
        }
    };
    for o in outcomes {
        let Some(main_id) = pool_to_task.get(&o.id).copied() else {
            continue;
        };
        let Some(p) = proxy.get(&main_id) else {
            continue;
        };
        if p.flushed {
            continue;
        }
        host.out().push_str(&o.output);
        let task = Rc::clone(&p.task);
        let tr = if !o.dirty.is_empty() {
            Some(loi(
                format!(
                    "ghi chéo worker (W>1): biến {} (task `{}`)",
                    o.dirty.join(", "),
                    task.ten
                ),
                task.span,
            ))
        } else if o.cancelled {
            Some(Trap::new(
                crate::interp::TASK_ERR,
                "tác vụ bị huỷ".to_string(),
                task.span,
            ))
        } else {
            None
        };
        if let Some(tr) = tr {
            *task.kq.borrow_mut() = Some(Err(tr.clone()));
            task.trang_thai.set(crate::rt::TrangThai::Xong);
            return Err(crate::interp::Abort::Trap(tr));
        }
        match o.result {
            Ok(sv) => {
                *task.kq.borrow_mut() = Some(Ok(sv.to_value()));
                task.trang_thai.set(crate::rt::TrangThai::Xong);
            }
            Err(m) => {
                let tr = Trap::new(
                    crate::interp::TASK_ERR,
                    format!("tác vụ `{}` lỗi: {m}", task.ten),
                    task.span,
                );
                *task.kq.borrow_mut() = Some(Err(tr.clone()));
                task.trang_thai.set(crate::rt::TrangThai::Xong);
                return Err(crate::interp::Abort::Trap(tr));
            }
        }
    }
    Ok(())
}
