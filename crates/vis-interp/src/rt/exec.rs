// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-313a (VD-289)** — orchestration worker của interp.
//!
//! Tách từ tệp cùng tên thời ST-306: **trạng thái** `ExecState`/pool/`WorkerMode` đã sang
//! `vis-runtime/src/rt/exec_state.rs`; ở đây chỉ còn orchestration cần runner cây
//! (`chay_than_worker`) và proxy theo engine.

use std::rc::Rc;
use std::sync::Arc;

use vis_core::trap::{Trap, R};
use vis_core::value::{Task, Value};
use vis_syntax::Span;

use vis_runtime::builtin::Host;

use crate::interp::Interp;
use vis_runtime::codes::RT;
pub(crate) use vis_runtime::rt::exec_state::{HirImage, PendingJob, TaskProxy, WorkerMode};
use vis_runtime::worker::{SendValue, TaskId, ToSend, WorkerCtx, WorkerError};

fn loi(msg: impl Into<String>, span: Span) -> Trap {
    Trap::new(RT, msg.into(), span)
}

fn loi_pool(e: WorkerError, span: Span) -> Trap {
    loi(format!("worker pool: {e:?}"), span)
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

    let mut s = it.sched.borrow_mut();
    s.exec
        .ensure_pool()
        .map_err(|m| crate::interp::Abort::Trap(loi(format!("worker pool: {m}"), span)))?;
    s.exec.order.push(t.id);
    if s.exec.worker_ids.len() <= t.id as usize {
        s.exec.worker_ids.resize(t.id as usize + 1, false);
    }
    s.exec.worker_ids[t.id as usize] = true;
    // **ST-291 (P2b-1)**: không gửi job ngay — gom vào lô, `flush_jobs` gửi trước await/join.
    s.exec.pending.push(PendingJob {
        task: Rc::clone(&t),
        than,
        args: args_send,
        snap,
        span,
    });
    Ok(Value::Task(t))
}

/// **ST-291 (P2b-1)** — gửi **toàn bộ** job đang chờ xuống pool theo một lô, giữ thứ tự spawn.
///
/// Gọi ở đầu `await_parallel` và `join_parallel`; như vậy chương trình spawn N rồi await/join
/// chỉ trả một lần chi phí gửi lô, còn spawn→await tuần tự vẫn flush đúng 1 job mỗi lần.
pub(crate) fn flush_jobs(host: &mut dyn Host) -> R<()> {
    let mut s = host.sched().borrow_mut();
    if s.exec.pending.is_empty() {
        return Ok(());
    }
    let span = s.exec.pending[0].span;
    s.exec
        .ensure_pool()
        .map_err(|m| crate::interp::Abort::Trap(loi(format!("worker pool: {m}"), span)))?;
    let (image, budget, cancel_before, deadline) = {
        (
            s.exec.image.clone().ok_or_else(|| {
                crate::interp::Abort::Trap(loi("worker Parallel thiếu HirImage (P2)", span))
            })?,
            s.exec.budget.clone(),
            s.exec.cancel_before,
            s.exec.cancel_deadline,
        )
    };
    let jobs = std::mem::take(&mut s.exec.pending);
    let mut batch = Vec::with_capacity(jobs.len());
    let mut tasks = Vec::with_capacity(jobs.len());
    for j in jobs {
        let PendingJob {
            task,
            than,
            args,
            snap,
            span,
        } = j;
        tasks.push(task);
        let image = Arc::clone(&image);
        let budget = budget.clone();
        batch.push((snap, move |ctx: &mut WorkerCtx, snap: &SendValue| {
            crate::interp::worker::chay_than_worker(
                &image, than, args, span, ctx, snap, budget, deadline,
            )
        }));
    }
    let handles = {
        let pool =
            s.exec.pool.as_mut().ok_or_else(|| {
                crate::interp::Abort::Trap(loi("worker Parallel thiếu pool", span))
            })?;
        pool.spawn_shared_batch(batch, cancel_before)
            .map_err(|e| crate::interp::Abort::Trap(loi_pool(e, span)))?
    };
    for (task, handle) in tasks.into_iter().zip(handles) {
        let idx = task.id as usize;
        if s.exec.proxy.len() <= idx {
            s.exec.proxy.resize_with(idx + 1, || None);
        }
        let pool_id = handle.id();
        s.exec.proxy[idx] = Some(TaskProxy {
            task,
            pool_id,
            handle,
            flushed: false,
            done: false,
            output: String::new(),
            dirty: Vec::new(),
            cancelled: false,
        });
        s.exec.pool_order.push(idx as u32);
    }
    Ok(())
}

/// W=1: `await` chạy trên caller, dùng cache/trạng thái của đường cũ.
pub(crate) fn await_local(host: &mut dyn Host, t: &Rc<Task>, span: Span) -> R<Value> {
    crate::rt::await_task_off(host, t, span)
}

/// **ST-292 (P4)/ST-317b**: áp outcome qua nguồn chung `vis_runtime::worker`, rồi báo
/// `frame_task_done` theo scheduler của interp.
fn apply_outcome(s: &mut crate::rt::Scheduler, o: vis_runtime::worker::TaskOutcome) {
    let Some(main_id) = s.exec.pool_order.get(o.id.as_u64() as usize).copied() else {
        return;
    };
    let Some(p) = s
        .exec
        .proxy
        .get_mut(main_id as usize)
        .and_then(|p| p.as_mut())
    else {
        return;
    };
    let (id, ket) = vis_runtime::worker::ap_dung_outcome(p, o);
    s.frame_task_done(id, ket);
}

/// Flush output của proxy đúng **một lần**; dirty ⇒ `VS0509`, cancelled ⇒ `VS0506`.
fn flush_proxy(host: &mut dyn Host, id: u32, span: Span) -> R<()> {
    let Some((output, loi)) = ({
        let mut s = host.sched().borrow_mut();
        let Some(p) = s.exec.proxy.get_mut(id as usize).and_then(|p| p.as_mut()) else {
            return Ok(());
        };
        vis_runtime::worker::flush_proxy(p, span)
    }) else {
        return Ok(());
    };
    host.out().push_str(&output);
    match loi {
        Some(t) => Err(crate::interp::Abort::Trap(t)),
        None => Ok(()),
    }
}

/// **ST-292 (P4)**: rút outcome đã sẵn sàng; `block` ⇒ chờ tối đa 50 ms. Trả số task vừa
/// được đánh thức. Không giữ borrow `RefCell` khi chờ.
pub(crate) fn pump(host: &mut dyn Host, block: bool) -> R<usize> {
    let (board, ids) = {
        let s = host.sched().borrow();
        let Some(board) = s.exec.outcome_board.clone() else {
            return Ok(0);
        };
        let ids: Vec<TaskId> = s
            .exec
            .proxy
            .iter()
            .flatten()
            .filter(|p| !p.done)
            .map(|p| p.pool_id)
            .collect();
        (board, ids)
    };
    if ids.is_empty() {
        return Ok(0);
    }
    let n = vis_runtime::worker::pump(&board, &ids, block, |o| {
        let mut s = host.sched().borrow_mut();
        apply_outcome(&mut s, o);
    });
    Ok(n)
}

/// Còn job/task worker chưa có outcome.
pub(crate) fn in_flight(s: &crate::rt::Scheduler) -> bool {
    !s.exec.pending.is_empty() || s.exec.proxy.iter().flatten().any(|p| !p.done)
}

/// `id` là worker task (spawn từ main frame) — fallback không được chạy lại thân.
pub(crate) fn is_worker_task(s: &crate::rt::Scheduler, id: u32) -> bool {
    s.exec.worker_ids.get(id as usize).copied().unwrap_or(false)
}

/// Huỷ hợp tác task worker (idempotent).
pub(crate) fn cancel_worker(s: &mut crate::rt::Scheduler, id: u32) {
    if let Some(p) = s.exec.proxy.get(id as usize).and_then(|p| p.as_ref()) {
        p.handle.cancel();
    }
}

/// **P2/P4**: `await` task worker. Frame mode ⇒ suspend (`Pending`) tới khi pump xong; non-frame
/// giữ nguyên chờ board. Output flush đúng điểm tiêu thụ; cache kết quả cho lần await sau.
pub(crate) fn await_parallel(host: &mut dyn Host, t: &Rc<Task>, span: Span) -> R<Value> {
    // **ST-291 (P2b-1)**: bảo đảm job của `t` (và mọi job đang chờ trước đó) đã xuống pool.
    flush_jobs(host)?;
    let (frame_on, has_proxy, done) = {
        let s = host.sched().borrow();
        let p = s.exec.proxy.get(t.id as usize).and_then(|p| p.as_ref());
        (
            s.frame.enabled,
            p.is_some(),
            p.map(|p| p.done).unwrap_or(false),
        )
    };
    if !has_proxy {
        return crate::rt::await_task_off(host, t, span);
    }
    if frame_on && !done {
        return Err(crate::interp::Abort::Pending(crate::rt::Pending {
            task: Some(Rc::clone(t)),
            wait: None,
            span,
        }));
    }
    // Non-frame: chờ board như cũ (pump block 50 ms tới khi có outcome).
    while !host
        .sched()
        .borrow()
        .exec
        .proxy
        .get(t.id as usize)
        .and_then(|p| p.as_ref())
        .map(|p| p.done)
        .unwrap_or(false)
    {
        let _ = pump(host, true)?;
    }
    flush_proxy(host, t.id, span)?;
    if let Some(kq) = t.kq.borrow().clone() {
        return kq.map_err(crate::interp::Abort::Trap);
    }
    Err(crate::interp::Abort::Trap(loi(
        "task worker thiếu kết quả sau pump",
        span,
    )))
}

/// W=1: join mọi task theo thứ tự tạo, không thread/buffer.
pub(crate) fn join_local(host: &mut dyn Host) -> R<()> {
    crate::rt::join_all_off(host)
}

/// **P2/P4**: final join — chờ pool, cache task chưa await, flush output theo **spawn order**
/// (outcome của pool đúng thứ tự gửi), surface dirty `VS0509`/cancel `VS0506`/lỗi task.
pub(crate) fn final_join(host: &mut dyn Host) -> R<()> {
    // Đường không song song: không có pool ⇒ no-op. Frame Off/W1 giữ nguyên hành vi trước P4
    // (`frame_final_join` lo phần task cache); non-frame đã gọi `Host::join_tasks` riêng.
    if !matches!(host.sched().borrow().exec.mode, WorkerMode::Parallel(_)) {
        return Ok(());
    }
    flush_jobs(host)?;
    let (pool, mut proxy, pool_order) = {
        let mut s = host.sched().borrow_mut();
        let pool = s.exec.pool.take();
        let proxy = std::mem::take(&mut s.exec.proxy);
        let order = std::mem::take(&mut s.exec.pool_order);
        s.exec.worker_ids.clear();
        s.exec.pending.clear();
        s.exec.outcome_board = None;
        (pool, proxy, order)
    };
    let Some(pool) = pool else {
        return Ok(());
    };
    let outcomes = match pool.join() {
        Ok(r) => r.outcomes,
        Err(WorkerError::CrossWrite { vars, tasks }) => {
            return Err(crate::interp::Abort::Trap(Trap::new(
                vis_runtime::worker::ma_ghi_cheo(),
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
        let Some(main_id) = pool_order.get(o.id.as_u64() as usize).copied() else {
            continue;
        };
        let Some(p) = proxy.get_mut(main_id as usize).and_then(|p| p.as_mut()) else {
            continue;
        };
        // Áp outcome (nếu chưa) + flush output **một lần**, kể cả task đã pump nhưng chưa await.
        if let Some((output, loi)) = vis_runtime::worker::ap_va_flush(p, Some(o)) {
            host.out().push_str(&output);
            if let Some(t) = loi {
                return Err(crate::interp::Abort::Trap(t));
            }
        }
    }
    Ok(())
}

/// Alias cũ (non-frame `join_all` gọi tên này).
pub(crate) fn join_parallel(host: &mut dyn Host) -> R<()> {
    final_join(host)
}
