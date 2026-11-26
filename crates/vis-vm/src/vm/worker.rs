// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-3b/W5 (P3)** — runner VM cho evaluator worker **CPU-only non-frame**.
//!
//! Mỗi job dựng `Vm` riêng trên `Arc<ModuleImage>`; globals là snapshot `SendValue::Map`
//! (`ô global → value`), output buffer merge theo spawn order, budget charge sau thân qua
//! `WorkerCtx` lease, cancel tại safe point 1.024 lệnh, cross-write phát hiện bằng so globals.
//!
//! Không host/`chan(0)`/frame: worker `Vm` không có host; `chan(0)` non-frame giữ `VS0505`.
//! `Off`/`Local` không đổi: `join_tasks`/`spawn_task`/`await_task` chỉ rẽ nhánh khi
//! `WorkerState.mode == Parallel`.
#![allow(dead_code)]

use vis_runtime::worker::ToSend;

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use vis_bytecode::Module;
use vis_core::trap::{Abort, Trap};
use vis_core::value::Task;
use vis_core::value::Value;
use vis_runtime::builtin::Host;
use vis_runtime::codes::RT;
use vis_runtime::rt::exec_state::TaskProxy;
use vis_runtime::rt::fee::SharedBudget;
use vis_runtime::worker::{OutcomeBoard, SendValue, TaskId, WorkerError, WorkerPool};
use vis_syntax::Span;

use super::Vm;

/// Ảnh module bất biến chia sẻ cho worker.
#[derive(Debug)]
pub(crate) struct ModuleImage {
    module: Module,
    text: Arc<str>,
}

/// Chế độ worker của VM; `Off` là mặc định.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Mode {
    #[default]
    Off,
    Parallel(usize),
}

/// Trạng thái evaluator worker của một `Vm`. `Vm` không `Send`; pool/board là opt-in.
#[derive(Default)]
pub(crate) struct WorkerState {
    pub(crate) mode: Mode,
    image: Option<Arc<ModuleImage>>,
    budget: Option<Arc<SharedBudget>>,
    cancel_before: bool,
    cancel_after: Option<Instant>,
    /// Cờ huỷ của task đang chạy trong worker VM (do `WorkerCtx` cấp).
    pub(crate) task_cancel: Option<Arc<std::sync::atomic::AtomicBool>>,
    /// Hạn huỷ của task (test hook cancel-after).
    pub(crate) task_deadline: Option<Instant>,
    pool: Option<WorkerPool>,
    board: Option<Arc<OutcomeBoard>>,
    proxy: HashMap<u32, TaskProxy>,
    pool_to_task: HashMap<TaskId, u32>,
}

fn loi(msg: impl Into<String>, span: Span) -> Trap {
    Trap::new(RT, msg.into(), span)
}

fn loi_abort(msg: impl Into<String>, span: Span) -> Abort {
    Abort::Trap(loi(msg, span))
}

fn image_and_flags(
    w: &WorkerState,
    span: Span,
) -> Result<(Arc<ModuleImage>, bool, Option<Instant>), Abort> {
    let image = w
        .image
        .clone()
        .ok_or_else(|| loi_abort("worker VM thiếu ModuleImage (P3)", span))?;
    Ok((image, w.cancel_before, w.cancel_after))
}

fn ensure_pool(w: &mut WorkerState) -> Result<(), String> {
    if w.pool.is_some() {
        return Ok(());
    }
    let Mode::Parallel(n) = w.mode else {
        return Ok(());
    };
    let root: HashMap<String, SendValue> = HashMap::new();
    let mut pool = match &w.budget {
        Some(b) => WorkerPool::with_budget(n, root, Arc::clone(b)),
        None => WorkerPool::new(n, root),
    }
    .map_err(|e| format!("worker pool: {e:?}"))?;
    let board = Arc::new(OutcomeBoard::new());
    pool.attach_outcome_board(Arc::clone(&board));
    w.pool = Some(pool);
    w.board = Some(board);
    Ok(())
}

/// Cấu hình VM chạy `Parallel(n)`; gọi trước `Vm::run`.
pub(crate) fn cau_hinh(
    vm: &mut Vm<'_>,
    module: &Module,
    text: &str,
    workers: usize,
    budget: Option<Arc<SharedBudget>>,
    cancel_after_ms: Option<u64>,
) {
    let mut w = vm.worker.borrow_mut();
    w.mode = Mode::Parallel(workers);
    w.image = Some(Arc::new(ModuleImage {
        module: module.clone(),
        text: Arc::from(text),
    }));
    w.budget = budget;
    w.cancel_before = false;
    w.cancel_after = cancel_after_ms.map(|ms| Instant::now() + Duration::from_millis(ms));
}

/// Snapshot globals (`None` bỏ qua) thành Map ô→value.
fn snapshot_globals(vm: &Vm<'_>) -> Result<Arc<SendValue>, String> {
    let mut pairs = Vec::new();
    for (i, o) in vm.globals.iter().enumerate() {
        let Some(v) = o else { continue };
        let sv = v
            .to_send()
            .map_err(|e| format!("global {i} không Sendable (VS0508): {e}"))?;
        pairs.push((SendValue::Int(i as i64), sv));
    }
    Ok(Arc::new(SendValue::Map(pairs)))
}

/// `$spawn` của VM: Off giữ scheduler cũ; Parallel đẩy job lên pool.
pub(crate) fn spawn_task(
    vm: &mut Vm<'_>,
    fi: u32,
    args: Vec<Value>,
    ten: String,
    span: Span,
) -> vis_core::trap::R<Value> {
    let mode = vm.worker.borrow().mode;
    // **ST-292 (P4a)**: chỉ main frame đẩy spawn lên pool; frame task chạy local.
    let frame_task = vm.sched.borrow().frame.current != 0;
    if mode == Mode::Off || frame_task {
        let t = vm.sched.borrow_mut().spawn(fi, args, ten, span);
        return Ok(Value::Task(t));
    }
    let snap = snapshot_globals(vm)
        .map_err(|m| loi_abort(format!("snapshot worker lỗi (VS0508): {m}"), span))?;
    let mut args_send = Vec::with_capacity(args.len());
    for v in args {
        args_send.push(
            v.to_send()
                .map_err(|m| loi_abort(format!("đối số không Sendable (VS0508): {m}"), span))?,
        );
    }
    let t = vm.sched.borrow_mut().tao(fi, Vec::new(), ten, span);
    let (image, cancel_before, deadline) = {
        let w = vm.worker.borrow();
        image_and_flags(&w, span)?
    };
    let mut w = vm.worker.borrow_mut();
    ensure_pool(&mut w).map_err(|m| loi_abort(format!("worker pool: {m}"), span))?;
    let handle = {
        let pool = w
            .pool
            .as_mut()
            .ok_or_else(|| loi_abort("worker VM thiếu pool", span))?;
        let image = Arc::clone(&image);
        pool.spawn_shared(snap, move |ctx, snap| {
            chay_than_vm(&image, fi, args_send, span, ctx, snap, deadline)
        })
        .map_err(|e| loi_abort(format!("worker pool: {e:?}"), span))?
    };
    if cancel_before {
        handle.cancel();
    }
    let pool_id = handle.id();
    w.pool_to_task.insert(pool_id, t.id);
    w.proxy.insert(
        t.id,
        TaskProxy {
            task: Rc::clone(&t),
            pool_id,
            handle,
            flushed: false,
            done: false,
            output: String::new(),
            dirty: Vec::new(),
            cancelled: false,
        },
    );
    Ok(Value::Task(t))
}

fn code_budget() -> &'static vis_diag::Code {
    &vis_diag::RUNTIME_CODES[9]
}

/// **ST-317b**: áp outcome qua nguồn chung `vis_runtime::worker`, rồi báo `frame_task_done`
/// theo scheduler của VM.
fn apply_outcome(vm: &mut Vm<'_>, o: vis_runtime::worker::TaskOutcome) {
    let mut w = vm.worker.borrow_mut();
    let Some(main_id) = w.pool_to_task.get(&o.id).copied() else {
        return;
    };
    let Some(p) = w.proxy.get_mut(&main_id) else {
        return;
    };
    let (id, ket) = vis_runtime::worker::ap_dung_outcome(p, o);
    drop(w);
    vm.sched.borrow_mut().frame_task_done(id, ket);
}

/// **ST-292 (P4)**: rút outcome đã sẵn sàng; `block` ⇒ chờ tối đa 50 ms.
pub(crate) fn pump(vm: &mut Vm<'_>, block: bool) -> vis_core::trap::R<usize> {
    let (board, ids) = {
        let w = vm.worker.borrow();
        let Some(board) = w.board.clone() else {
            return Ok(0);
        };
        let ids: Vec<TaskId> = w
            .proxy
            .values()
            .filter(|p| !p.done)
            .map(|p| p.pool_id)
            .collect();
        (board, ids)
    };
    if ids.is_empty() {
        return Ok(0);
    }
    let n = vis_runtime::worker::pump(&board, &ids, block, |o| apply_outcome(vm, o));
    Ok(n)
}

/// Còn proxy chưa có outcome.
pub(crate) fn in_flight(vm: &Vm<'_>) -> bool {
    vm.worker.borrow().proxy.values().any(|p| !p.done)
}

/// `id` là worker task (spawn từ main frame).
pub(crate) fn is_worker_task(vm: &Vm<'_>, id: u32) -> bool {
    let w = vm.worker.borrow();
    w.mode != Mode::Off && w.proxy.contains_key(&id)
}

/// Huỷ hợp tác task worker (idempotent).
pub(crate) fn cancel_worker(vm: &mut Vm<'_>, id: u32) {
    if let Some(p) = vm.worker.borrow().proxy.get(&id) {
        p.handle.cancel();
    }
}

/// Flush output của proxy đúng một lần; dirty ⇒ `VS0509`, cancelled ⇒ `VS0506`.
fn flush_proxy(vm: &mut Vm<'_>, id: u32, span: Span) -> vis_core::trap::R<()> {
    let Some((output, loi)) = ({
        let mut w = vm.worker.borrow_mut();
        let Some(p) = w.proxy.get_mut(&id) else {
            return Ok(());
        };
        vis_runtime::worker::flush_proxy(p, span)
    }) else {
        return Ok(());
    };
    vm.out.push_str(&output);
    match loi {
        Some(t) => Err(Abort::Trap(t)),
        None => Ok(()),
    }
}

/// `$await` của VM: frame mode ⇒ suspend; non-frame chờ board; flush output tại tiêu thụ.
pub(crate) fn await_task(vm: &mut Vm<'_>, t: &Rc<Task>, span: Span) -> vis_core::trap::R<Value> {
    let (frame_on, has_proxy, done) = {
        let w = vm.worker.borrow();
        let p = w.proxy.get(&t.id);
        (
            vm.sched.borrow().frame.enabled,
            p.is_some(),
            p.map(|p| p.done).unwrap_or(false),
        )
    };
    if !has_proxy {
        return vis_runtime::rt::await_task_off(vm, t, span);
    }
    if frame_on && !done {
        return Err(Abort::Pending(vis_core::value::Pending {
            task: Some(Rc::clone(t)),
            wait: None,
            span,
        }));
    }
    while !vm
        .worker
        .borrow()
        .proxy
        .get(&t.id)
        .map(|p| p.done)
        .unwrap_or(false)
    {
        let _ = pump(vm, false)?;
        if !vm
            .worker
            .borrow()
            .proxy
            .get(&t.id)
            .map(|p| p.done)
            .unwrap_or(false)
        {
            let _ = pump(vm, true)?;
        }
    }
    flush_proxy(vm, t.id, span)?;
    if let Some(kq) = t.kq.borrow().clone() {
        return kq.map_err(Abort::Trap);
    }
    Err(loi_abort("task worker thiếu kết quả sau pump", span))
}

/// Final join VM: chờ pool, cache task chưa await, flush output theo spawn order.
pub(crate) fn final_join(vm: &mut Vm<'_>) -> vis_core::trap::R<()> {
    let (pool, mut proxy, map) = {
        let mut w = vm.worker.borrow_mut();
        if w.mode == Mode::Off {
            // Frame Off/W1: no-op (giữ nguyên hành vi trước P4); non-frame đã gọi
            // `Host::join_tasks` riêng.
            return Ok(());
        }
        (
            w.pool.take(),
            std::mem::take(&mut w.proxy),
            std::mem::take(&mut w.pool_to_task),
        )
    };
    let Some(pool) = pool else {
        return Ok(());
    };
    let outcomes = match pool.join() {
        Ok(r) => r.outcomes,
        Err(WorkerError::CrossWrite { vars, tasks }) => {
            return Err(Abort::Trap(Trap::new(
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
            return Err(loi_abort(
                format!("worker pool join: {e:?}"),
                Span::new(0, 0),
            ))
        }
    };
    // **ST-318a F1/P2-B**: flush **hết** output của mọi task (kể cả task đứng sau task lỗi)
    // rồi mới trả trap **đầu** theo spawn order — trước đây `return` ngay ở trap đầu làm mất
    // stdout của các task phía sau. Mã/message/span của trap giữ nguyên.
    let mut trap = None;
    for o in outcomes {
        let Some(main_id) = map.get(&o.id).copied() else {
            continue;
        };
        let Some(p) = proxy.get_mut(&main_id) else {
            continue;
        };
        if let Some((output, loi)) = vis_runtime::worker::ap_va_flush(p, Some(o)) {
            vm.out.push_str(&output);
            if trap.is_none() {
                trap = loi;
            }
        }
    }
    match trap {
        Some(t) => Err(Abort::Trap(t)),
        None => Ok(()),
    }
}

/// Alias cũ.
pub(crate) fn join_tasks(vm: &mut Vm<'_>) -> vis_core::trap::R<()> {
    // Non-frame default: giữ nguyên `join_all` cũ (kể cả task không ai await).
    if vm.worker.borrow().mode == Mode::Off {
        return vis_runtime::rt::join_all_off(vm);
    }
    final_join(vm)
}

impl Vm<'_> {
    pub(crate) fn worker_tick(&self, span: Span) -> Option<Abort> {
        if self.steps & 1023 != 0 {
            return None;
        }
        let w = self.worker.borrow();
        if let Some(d) = w.task_deadline {
            if Instant::now() >= d {
                return Some(Abort::Trap(loi("tác vụ bị huỷ", span)));
            }
        }
        let c = w.task_cancel.as_ref()?;
        if c.load(Ordering::Acquire) {
            Some(Abort::Trap(loi("tác vụ bị huỷ", span)))
        } else {
            None
        }
    }
}

/// Runner VM trong worker: globals snapshot, nested join, output, dirty, budget lease.
#[allow(clippy::too_many_arguments)]
fn chay_than_vm(
    image: &ModuleImage,
    fi: u32,
    args: Vec<SendValue>,
    span: Span,
    ctx: &mut vis_runtime::worker::WorkerCtx,
    snap: &SendValue,
    deadline: Option<Instant>,
) -> Result<SendValue, String> {
    let mut vm = Vm::new(&image.module, &image.text);
    vm.init_globals();
    let SendValue::Map(pairs) = snap else {
        return Err("snapshot VM không phải Map".to_string());
    };
    for (k, v) in pairs {
        let SendValue::Int(i) = k else {
            return Err("snapshot VM có khoá không phải Int(ô)".to_string());
        };
        if let Some(slot) = vm.globals.get_mut(*i as usize) {
            *slot = Some(v.to_value());
        }
    }
    vm.worker.borrow_mut().task_cancel = Some(ctx.cancel_flag());
    vm.worker.borrow_mut().task_deadline = deadline;
    let before = vm.globals.clone();
    let args: Vec<Value> = args.iter().map(SendValue::to_value).collect();
    let res: Result<Value, String> = match Host::goi_than(&mut vm, fi, args, span) {
        Ok(v) => Ok(v),
        Err(Abort::Ret(v)) => Ok(v),
        Err(Abort::Trap(t)) => Err(format!("{}: {}", t.code.short, t.msg)),
        Err(Abort::Pending(_)) => Err("pending trong thân tác vụ VM worker".to_string()),
    };
    // Nested task không ai await vẫn chạy hết trong worker (plan D9).
    let res = if res.is_ok() {
        match join_tasks(&mut vm) {
            Ok(()) => res,
            Err(Abort::Trap(t)) => Err(format!("{}: {}", t.code.short, t.msg)),
            Err(_) => Err("join tác vụ lồng VM worker lỗi".to_string()),
        }
    } else {
        res
    };
    let out = vm.take_stdout();
    if !out.is_empty() {
        ctx.print(&out);
    }
    let after = vm.globals.clone();
    let n = before.len().max(after.len());
    for i in 0..n {
        let old = before.get(i).and_then(|o| o.as_ref());
        let new = after.get(i).and_then(|o| o.as_ref());
        let same = match (old, new) {
            (Some(a), Some(b)) => a.eq_value(b),
            (None, None) => true,
            _ => false,
        };
        if !same {
            ctx.mark_dirty(format!("g{i}"));
        }
    }
    // Charge theo chunk 256 để lease ghi `used` từng phần (một weight 1M vượt chunk
    // sẽ bị BudgetLease từ chối nguyên khối, không ghi used).
    let mut con = vm.steps as i64;
    while con > 0 {
        let w = con.min(256);
        ctx.charge(w)?;
        con -= w;
    }
    res.and_then(|v| {
        v.to_send()
            .map_err(|e| format!("kết quả task VM không Sendable (VS0508): {e}"))
    })
}
