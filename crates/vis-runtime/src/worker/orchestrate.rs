// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-317b (§4)** — orchestration worker **dùng chung** interp ↔ VM.
//!
//! Trước đây `vis-interp/src/rt/exec.rs` và `vis-vm/src/vm/worker.rs` giữ hai bản gần sao của
//! cùng một máy trạng thái: áp outcome vào proxy + cache `Task`, flush output đúng một lần
//! (`VS0509` ghi chéo / `VS0506` huỷ), rút outcome từ `OutcomeBoard`. Tệp này là **một nguồn**
//! cho các mảnh thuần engine-agnostic; runner thân (cây HIR vs module VBC1), snapshot globals và
//! chỗ ghi stdout vẫn thuộc từng engine.
//!
//! Hành vi không đổi: cùng message/mã lỗi, cùng thứ tự áp outcome, flush đúng một lần.

use std::time::Duration;

use vis_core::trap::Trap;
use vis_core::value::{Task, TrangThai, Value};
use vis_syntax::Span;

use crate::codes::TASK_ERR;
use crate::rt::exec_state::TaskProxy;
use crate::worker::{OutcomeBoard, TaskId, TaskOutcome};

/// Mã **VS0509** — ghi chéo worker (ST-290 cấp, ST-292 dùng).
pub fn ma_ghi_cheo() -> &'static vis_diag::Code {
    crate::codes::CROSS
}

/// Trap từ lỗi chuỗi của worker: **luôn** bọc `VS0506`, giữ mã gốc trong message (kể cả
/// `VS0510`) — cùng hợp đồng task-error với W=1.
pub fn loi_task(task: &Task, m: String) -> Trap {
    Trap::new(
        TASK_ERR,
        format!("tác vụ `{}` lỗi: {m}", task.ten),
        task.span,
    )
}

/// Áp một outcome vào proxy + cache `Task`; **không** flush output.
///
/// Trả `(task.id của main, kết quả)` để engine gọi `Scheduler::frame_task_done` theo scheduler
/// của mình (interp giữ `Scheduler` trong `ExecState`, VM giữ `Rc<RefCell<Scheduler>>`).
pub fn ap_dung_outcome(p: &mut TaskProxy, o: TaskOutcome) -> (u32, Result<Value, Trap>) {
    p.done = true;
    p.output = o.output.clone();
    p.dirty = o.dirty.clone();
    p.cancelled = o.cancelled;
    let ket = match o.result {
        Ok(sv) => Ok(sv.to_value()),
        Err(m) => Err(loi_task(&p.task, m)),
    };
    *p.task.kq.borrow_mut() = Some(ket.clone());
    p.task.trang_thai.set(TrangThai::Xong);
    (p.task.id, ket)
}

/// Lấy (và đánh dấu đã lấy) output của proxy **đúng một lần**.
///
/// `None` nếu proxy đã flush trước đó. `Some((output, None))` khi sạch; `Some((output, Some(trap)))`
/// khi `dirty` (`VS0509`) hoặc `cancelled` (`VS0506`) — engine tự append `output` rồi trả trap.
pub fn flush_proxy(p: &mut TaskProxy, span: Span) -> Option<(String, Option<Trap>)> {
    if p.flushed {
        return None;
    }
    p.flushed = true;
    let output = std::mem::take(&mut p.output);
    let dirty = std::mem::take(&mut p.dirty);
    let cancelled = p.cancelled;
    let ten = p.task.ten.clone();
    let loi = if !dirty.is_empty() {
        Some(Trap::new(
            ma_ghi_cheo(),
            format!(
                "ghi chéo worker (W>1): biến {} (task `{ten}`)",
                dirty.join(", ")
            ),
            span,
        ))
    } else if cancelled {
        Some(Trap::new(TASK_ERR, "tác vụ bị huỷ".to_string(), span))
    } else {
        None
    };
    Some((output, loi))
}

/// Áp outcome (nếu có) rồi flush proxy — dùng chung cho `final_join` hai engine.
///
/// Không đụng stdout: trả `Some((output, Option<Trap>))` để engine tự ghi output của mình.
///
/// **ST-318a (#204)**: nếu outcome mang `Err(..)` (lỗi thân task mà **không ai await**), trap
/// `VS0506` đã cache trong `task.kq` được trả kèm — cùng message/mã/span như đường frame
/// (`frame_final_join` quét `kq`). Trap `dirty` (`VS0509`)/`cancelled` của `flush_proxy` vẫn
/// **thắng** để giữ thứ tự cũ; proxy đã flush (đã await) không bị báo lại.
pub fn ap_va_flush(p: &mut TaskProxy, o: Option<TaskOutcome>) -> Option<(String, Option<Trap>)> {
    let mut loi_ket = None;
    if let Some(o) = o {
        if !p.done {
            let (_, ket) = ap_dung_outcome(p, o);
            loi_ket = ket.err();
        }
    }
    if loi_ket.is_none() {
        loi_ket = match p.task.kq.borrow().as_ref() {
            Some(Err(t)) => Some(t.clone()),
            _ => None,
        };
    }
    let span = p.task.span;
    let (output, loi) = flush_proxy(p, span)?;
    Some((output, loi.or(loi_ket)))
}

/// Rút outcome đã sẵn sàng từ `OutcomeBoard`; `block` ⇒ chờ tối đa **50 ms** (như P4 cũ).
///
/// Không giữ borrow `RefCell` của engine khi chờ — caller lấy `ids` trước rồi gọi hàm này.
pub fn rut_outcome(board: &OutcomeBoard, ids: &[TaskId], block: bool) -> Vec<TaskOutcome> {
    if ids.is_empty() {
        return Vec::new();
    }
    let mut outs = Vec::new();
    if block {
        if let Ok((_, o)) = board.wait_any_timeout(ids, Duration::from_millis(50)) {
            outs.push(o);
        }
    }
    for id in ids {
        if let Some(o) = board.try_take(*id) {
            outs.push(o);
        }
    }
    outs
}

/// **ST-317b** — pump **một nguồn** cho hai engine: rút outcome rồi gọi `ap` cho từng cái.
///
/// `board`/`ids` lấy **trước** khi gọi (không giữ borrow `RefCell` của engine qua 50 ms chờ);
/// closure `ap` của engine tự báo scheduler/`frame_task_done`.
pub fn pump<F: FnMut(TaskOutcome)>(
    board: &OutcomeBoard,
    ids: &[TaskId],
    block: bool,
    mut ap: F,
) -> usize {
    let outs = rut_outcome(board, ids, block);
    let n = outs.len();
    for o in outs {
        ap(o);
    }
    n
}
