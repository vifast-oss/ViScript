// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310a (VD-285)** — hàm tác vụ/kênh cấp crate trên `Scheduler`, tách khỏi `rt.rs`.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use vis_syntax::Span;

use crate::builtin::Host;
use vis_core::trap::{Abort, Trap, R};

use crate::codes::{DEADLOCK, RT};

use vis_core::value::Value;

use super::{Chan, Pending, Task, TrangThai, MAX_TASK_NEST};

/// **Chỉ test (Lát 2)**: dựng `Result[str,str]` khớp đường `std.net` mặc định
/// (`hop_nhat_fs`: `Ok` → `Value::ok`, `Err` → `Value::err(Error)` dữ liệu).
pub(super) fn ok_str(s: impl Into<String>) -> Value {
    Value::ok(Value::Str(s.into()))
}
pub(super) fn err_str(s: impl Into<String>) -> Value {
    Value::err(Value::ban_ghi_error(s.into(), None))
}

/// **Chỉ test**: bọc lỗi thân tác vụ thành `VS0506` (dùng chung cho cache và driver).
#[doc(hidden)]
pub fn wrap_task_error_for_test(ten: &str, goc: Trap, span: Span) -> Trap {
    wrap_task_error(ten, goc, span)
}

/// **ST-61 (VD-96)**: tạo kênh đệm `cap` (âm ⇒ `VS0501`, **không** panic).
pub fn tao_kenh(host: &mut dyn Host, cap: i64, span: Span) -> R<Value> {
    if cap < 0 {
        return Err(Abort::Trap(Trap::new(
            RT,
            format!("`chan[T](n)`: dung lượng **âm** ({cap}) — cần `n ≥ 0`"),
            span,
        )));
    }
    // **ST-94 (VD-112)** / **ST-316b (`#145`)** — `chan(0)` (rendezvous) **không** chạy được trên
    // scheduler một luồng mặc định. Tyck **không** kiểm `cap`: `kiem_chan_new` chỉ đòi `n` là
    // `int` (VD-154 (c)) ⇒ `vis check` **xanh**; **runtime** mới từ chối bằng `VS0505` (exit 101 —
    // lỗi chạy có mã theo R9/C1, **không** phải lỗi tĩnh exit 2). **VS3-2a**: riêng đường frame
    // test đã có wait-set `chan(0)` (send chờ recv và ngược lại) ⇒ cho phép tạo; runtime mặc định
    // (`frame.enabled == false`) vẫn từ chối.
    if cap == 0 && !host.sched().borrow().frame.enabled {
        return Err(Abort::Trap(Trap::new(
            DEADLOCK,
            "`chan[T](0)`: kênh capacity 0 (rendezvous) cần scheduler thật — chưa hỗ trợ ở vòng này; dùng capacity ≥ 1",
            span,
        )));
    }
    let mut s = host.sched().borrow_mut();
    let id = s.kenh.len() as u32 + 1;
    let c = Rc::new(RefCell::new(Chan {
        id,
        cap: cap as usize,
        buf: VecDeque::new(),
        closed: false,
        send_waiters: VecDeque::new(),
        recv_waiters: VecDeque::new(),
        recv_values: HashMap::new(),
    }));
    s.kenh.push(Rc::clone(&c));
    Ok(Value::Chan(c))
}

/// **ST-61 (VD-96)**: chạy **một** tác vụ sẵn sàng trong hàng đợi (FIFO) **tới hoàn tất** — dùng
/// khi kênh **chặn** (`send` đầy, `recv` rỗng). `false` = **không còn** tác vụ nào chạy được.
///
/// Chỉ lấy task **`Moi`** (đã `spawn`, chưa chạy) — **không** chạy task `Moi` chưa `spawn` (đúng
/// VD-91: "hàng đợi sẵn sàng"). Hệ quả (nợ, ghi rõ): producer/consumer chỉ chạy khi **đệm đủ**
/// hoặc **thứ tự `await`** phù hợp — mô hình run-to-completion như async.
pub fn chay_mot(host: &mut dyn Host) -> R<bool> {
    loop {
        let idx = host.sched().borrow_mut().ready.pop_front();
        let Some(idx) = idx else { return Ok(false) };
        let t = {
            let s = host.sched().borrow();
            let Some(t) = s.tasks.get(idx).cloned() else {
                continue;
            };
            t
        };
        if t.trang_thai.get() != TrangThai::Moi {
            continue;
        }
        let span = t.span;
        host.await_task(&t, span)?;
        return Ok(true);
    }
}

/// Thân `await` của đường một luồng (Off) — P1 tách tên, **không đổi ngữ nghĩa**.
pub fn await_task_off(host: &mut dyn Host, t: &Rc<Task>, span: Span) -> R<Value> {
    match t.trang_thai.get() {
        TrangThai::Xong => {
            let kq = t.kq.borrow();
            return match kq.as_ref() {
                Some(Ok(v)) => Ok(v.clone()),
                Some(Err(tr)) => Err(Abort::Trap(tr.clone())),
                // Bất biến: `Xong` ⇒ có cache. Phòng thủ: bytecode đột biến ⇒ lỗi, **không** panic.
                None => Err(Abort::Trap(Trap::new(
                    RT,
                    format!(
                        "tác vụ `{}` đã xong mà không có kết quả (bất biến hỏng)",
                        t.ten
                    ),
                    span,
                ))),
            };
        }
        TrangThai::DangChay => {
            return Err(Abort::Trap(Trap::new(
                RT,
                format!("`await` chính tác vụ đang chạy `{}`", t.ten),
                span,
            )))
        }
        TrangThai::Moi => {}
        // `Pending` chỉ phát sinh trong đường frame; ở đó `await_task` không tự chạy thân.
        TrangThai::Pending => {}
        // VD-257/C: task bị huỷ. Nếu đã có cache (trước khi huỷ) thì trả cache đúng hợp đồng;
        // ngược lại dùng MÃ CÓ SẴN `VS0506` — không tạo mã huỷ mới (xem BLOCKER report).
        TrangThai::Huy => {
            let kq = t.kq.borrow();
            return match kq.as_ref() {
                Some(Ok(v)) => Ok(v.clone()),
                Some(Err(tr)) => Err(Abort::Trap(tr.clone())),
                None => Err(Abort::Trap(wrap_task_error(
                    &t.ten,
                    Trap::new(RT, "tác vụ đã bị huỷ trước khi có kết quả", t.span),
                    t.span,
                ))),
            };
        }
    }
    // **ST-267/VD-251 (chỉ test)**: không chạy thân tới hoàn tất. Báo driver treo frame,
    // driver lưu frame thật (Th/IFrame) rồi đăng ký wait + chạy frame khác. Đường cache
    // (`Xong`) ở trên vẫn đi nguyên văn ⇒ `await` lần hai trả cùng result/error.
    if host.sched().borrow().frame.enabled {
        return Err(Abort::Pending(Pending {
            task: Some(Rc::clone(t)),
            span,
            wait: None,
        }));
    }
    // Chặn lồng: mỗi tầng `await` là một khung gọi thật.
    {
        let mut s = host.sched().borrow_mut();
        s.sau += 1;
        if s.sau > MAX_TASK_NEST {
            s.sau -= 1;
            return Err(Abort::Trap(Trap::new(
                RT,
                format!("tác vụ lồng quá sâu (>{MAX_TASK_NEST} tầng `await`)"),
                span,
            )));
        }
    }
    t.trang_thai.set(TrangThai::DangChay);
    // Runner W=1 (P1): chạy thân trên caller, shared env; đã bọc `VS0506` đúng một lần.
    let kq = chay_than_local(host, t.than, t.args.clone(), &t.ten, t.span);
    t.trang_thai.set(TrangThai::Xong);
    let kq: Result<Value, Trap> = match kq {
        Ok(v) => Ok(v),
        // Runner đã bọc `Trap`; không bọc lần hai.
        Err(Abort::Trap(trap)) => Err(trap),
        // Hai nhánh dưới không xảy ra với runner P1; giữ phòng thủ như code cũ.
        Err(Abort::Ret(v)) => Ok(v),
        Err(Abort::Pending(_)) => Err(wrap_task_error(
            &t.ten,
            Trap::new(RT, "pending trong thân tác vụ", t.span),
            t.span,
        )),
    };
    *t.kq.borrow_mut() = Some(kq.clone());
    host.sched().borrow_mut().sau -= 1;
    kq.map_err(Abort::Trap)
}

/// Bọc lỗi thân tác vụ thành **`VS0506`** — giữ **mã gốc** + message gốc, `span` = **lúc tạo**.
pub fn wrap_task_error(ten: &str, goc: Trap, span: Span) -> Trap {
    Trap::new(
        crate::codes::TASK_ERR,
        format!("tác vụ `{ten}` lỗi: {}: {}", goc.code.short, goc.msg),
        span,
    )
}

/// Thân join của đường một luồng (Off) — P1 tách tên, **không đổi ngữ nghĩa**.
pub fn join_all_off(host: &mut dyn Host) -> R<()> {
    let ds: Vec<Rc<Task>> = host.sched().borrow().tasks.clone();
    for t in ds {
        let span = t.span;
        host.await_task(&t, span)?;
    }
    Ok(())
}

/// Chạy thân một tác vụ **trên caller** (W=1). Không snapshot/buffer/thread; mọi ghi đi vào env
/// chung của host. `ten` dùng cho thông điệp `VS0506`.
pub fn chay_than_local(
    host: &mut dyn Host,
    than: u32,
    args: Vec<Value>,
    ten: &str,
    span: Span,
) -> R<Value> {
    match host.goi_than(than, args, span) {
        Ok(v) => Ok(v),
        // `return` trong thân tác vụ: `Abort::Ret` là kết quả bình thường.
        Err(Abort::Ret(v)) => Ok(v),
        Err(Abort::Trap(goc)) => Err(Abort::Trap(crate::rt::ops::wrap_task_error(ten, goc, span))),
        // Đường frame không đi qua Local (P1 non-frame); phòng thủ thay vì panic.
        Err(Abort::Pending(_)) => Err(Abort::Trap(crate::rt::ops::wrap_task_error(
            ten,
            Trap::new(RT, "pending trong thân tác vụ", span),
            span,
        ))),
    }
}
