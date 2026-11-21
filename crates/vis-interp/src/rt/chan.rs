// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-2a/2b** — phương thức kênh + wait-set rendezvous `chan(0)` + **timeout kênh**, tất cả
//! **chỉ đường frame test** (thêm `recv_timeout`/`send_timeout`).
//!
//! Tách khỏi `builtin.rs` để giữ R1. Runtime mặc định (`frame.enabled == false`) giữ nguyên
//! hoàn toàn: `chan(0)` vẫn bị `tao_kenh` từ chối, blocking `send`/`recv` vẫn drain task rồi
//! `VS0505`; timeout trả lỗi rõ (chưa hỗ trợ đồng bộ). Chỉ khi frame test bật mới dùng
//! wait-set/timer bên dưới.
//!
//! Bất biến:
//! - sender/receiver `cap(0)` chờ theo FIFO; ghép đúng một lần;
//! - `close` đánh thức receiver (nhận `None` sau khi hết giá trị đã ghép) và sender
//!   (**`VS0504`** — không gửi vào kênh đã đóng);
//! - không còn task chạy được ⇒ `frame_stuck_trap` trả `VS0505`, không treo;
//! - cancellation gỡ sender/receiver khỏi kênh (xem `Chan::huy_frame`).

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use vis_syntax::Span;

use super::{
    Chan, ChanSend, ChanSendState, Pending, ReactorEvent, ReactorOp, ReactorOutcome, ReactorReg,
    Wait,
};
use crate::builtin::Host;
use crate::interp::{Abort, Trap, DEADLOCK, R, RT, SEND_CLOSED};
use crate::value::Value;

/// Điểm vào duy nhất của `send`/`recv`/`close` trên kênh.
pub(crate) fn method(
    host: &mut dyn Host,
    c: Rc<RefCell<Chan>>,
    name: &str,
    args: Vec<Value>,
    span: Span,
) -> R<Value> {
    let frame = host.sched().borrow().frame.enabled;
    let rendezvous = frame && c.borrow().cap == 0;
    match name {
        "send" => {
            let Some(v) = args.into_iter().next() else {
                return Err(trap(RT, "`send` cần 1 đối số", span));
            };
            if rendezvous {
                send_rendezvous(host, c, v, span)
            } else if frame {
                send_buf_frame(host, c, v, span)
            } else {
                send_buffered(host, c, v, span)
            }
        }
        "recv" => {
            if rendezvous {
                recv_rendezvous(host, c, span)
            } else if frame {
                recv_buf_frame(host, c, span)
            } else {
                recv_buffered(host, c, span)
            }
        }
        // **VS3-2b**: timeout hợp đồng None = timeout **hoặc** đóng.
        "recv_timeout" => recv_timeout(host, c, args, span),
        // **VS3-2b**: timeout hợp đồng false = timeout **hoặc** đóng.
        "send_timeout" => send_timeout(host, c, args, span),
        // `close` **idempotent** (VD-96 (3)): lần hai **không** lỗi.
        "close" => {
            if frame {
                close_rendezvous(host, c)
            } else {
                c.borrow_mut().closed = true;
                wake_chan_timeouts(host, c.borrow().id, true);
                wake_chan_timeouts(host, c.borrow().id, false);
                Ok(Value::Unit)
            }
        }
        _ => Err(trap(
            RT,
            format!(
                "kênh không có phương thức `{name}` (chỉ `send`, `recv`, `recv_timeout`, `send_timeout`, `close`)"
            ),
            span,
        )),
    }
}

fn trap(code: &'static vis_diag::Code, msg: impl Into<String>, span: Span) -> Abort {
    Abort::Trap(Trap::new(code, msg, span))
}

fn pending(id: u32, send: bool, span: Span) -> Abort {
    Abort::Pending(Pending {
        task: None,
        span,
        wait: Some(Wait::Chan { id, send }),
    })
}

/// Trạng thái chờ kênh hiện tại của frame (id, frame).
fn hien_tai(host: &dyn Host, c: &Rc<RefCell<Chan>>) -> (u32, u32) {
    let frame = host.sched().borrow().frame.current;
    (c.borrow().id, frame)
}

/// `send` trên `chan(0)` đường frame: ghép với receiver đang chờ, nếu không thì treo.
fn send_rendezvous(host: &mut dyn Host, c: Rc<RefCell<Chan>>, v: Value, span: Span) -> R<Value> {
    let (id, frame) = hien_tai(host, &c);
    {
        let mut ch = c.borrow_mut();
        // Resume của chính sender này: `Taken` = đã ghép xong; `Closed` = kênh đóng khi chờ.
        if let Some(pos) = ch.send_waiters.iter().position(|s| s.frame == frame) {
            match ch.send_waiters[pos].state {
                ChanSendState::Taken => {
                    ch.send_waiters.remove(pos);
                    return Ok(Value::Unit);
                }
                ChanSendState::Pending => return Err(pending(id, true, span)),
                ChanSendState::Closed => {
                    ch.send_waiters.remove(pos);
                    return Err(trap(
                        SEND_CLOSED,
                        "`send` vào kênh đã `close` (không nhận thêm phần tử)",
                        span,
                    ));
                }
            }
        }
        if ch.closed {
            return Err(trap(
                SEND_CLOSED,
                "`send` vào kênh đã `close` (không nhận thêm phần tử)",
                span,
            ));
        }
        // Receiver chờ trước ⇒ ghép ngay, đánh thức receiver.
        if let Some(rf) = ch.recv_waiters.pop_front() {
            ch.recv_values.insert(rf, v);
            drop(ch);
            host.sched().borrow_mut().frame_wake_chan(id, false, rf);
            wake_chan_timeouts(host, id, false);
            return Ok(Value::Unit);
        }
        ch.send_waiters.push_back(ChanSend {
            frame,
            value: v,
            state: ChanSendState::Pending,
        });
    }
    Err(pending(id, true, span))
}

/// `recv` trên `chan(0)` đường frame: lấy giá trị đã ghép, ghép sender đang chờ, hoặc treo.
fn recv_rendezvous(host: &mut dyn Host, c: Rc<RefCell<Chan>>, span: Span) -> R<Value> {
    let (id, frame) = hien_tai(host, &c);
    {
        let mut ch = c.borrow_mut();
        if let Some(v) = ch.recv_values.remove(&frame) {
            return Ok(Value::some(v));
        }
        if ch.closed {
            return Ok(Value::none());
        }
        // Sender chờ trước ⇒ lấy giá trị, đánh dấu đã gửi, đánh thức sender.
        if let Some(pos) = ch
            .send_waiters
            .iter()
            .position(|s| s.state == ChanSendState::Pending)
        {
            let f = ch.send_waiters[pos].frame;
            let v = std::mem::replace(&mut ch.send_waiters[pos].value, Value::Unit);
            ch.send_waiters[pos].state = ChanSendState::Taken;
            drop(ch);
            host.sched().borrow_mut().frame_wake_chan(id, true, f);
            wake_chan_timeouts(host, id, true);
            return Ok(Value::some(v));
        }
        ch.recv_waiters.push_back(frame);
    }
    Err(pending(id, false, span))
}

/// `close` trên `chan(0)`: đánh thức receiver (nhận `None`) và sender (`VS0504`).
fn close_rendezvous(host: &mut dyn Host, c: Rc<RefCell<Chan>>) -> R<Value> {
    let id = c.borrow().id;
    let mut wake_send = Vec::new();
    let mut wake_recv = Vec::new();
    {
        let mut ch = c.borrow_mut();
        if ch.closed {
            return Ok(Value::Unit);
        }
        ch.closed = true;
        for s in &mut ch.send_waiters {
            if s.state == ChanSendState::Pending {
                s.state = ChanSendState::Closed;
                wake_send.push(s.frame);
            }
        }
        wake_recv.extend(ch.recv_waiters.drain(..));
    }
    for f in wake_send {
        host.sched().borrow_mut().frame_wake_chan(id, true, f);
    }
    for f in wake_recv {
        host.sched().borrow_mut().frame_wake_chan(id, false, f);
    }
    wake_chan_timeouts(host, id, true);
    wake_chan_timeouts(host, id, false);
    Ok(Value::Unit)
}

/// **VS3-1d** — `send` trên `chan(n≥1)` đường frame: đệm nếu còn chỗ, chờ nếu đầy.
fn send_buf_frame(host: &mut dyn Host, c: Rc<RefCell<Chan>>, v: Value, span: Span) -> R<Value> {
    let (id, frame) = hien_tai(host, &c);
    {
        let mut ch = c.borrow_mut();
        if let Some(pos) = ch.send_waiters.iter().position(|s| s.frame == frame) {
            match ch.send_waiters[pos].state {
                ChanSendState::Taken => {
                    ch.send_waiters.remove(pos);
                    return Ok(Value::Unit);
                }
                ChanSendState::Closed => {
                    ch.send_waiters.remove(pos);
                    return Err(trap(
                        SEND_CLOSED,
                        "`send` vào kênh đã `close` (không nhận thêm phần tử)",
                        span,
                    ));
                }
                ChanSendState::Pending => return Err(pending(id, true, span)),
            }
        }
        if ch.closed {
            return Err(trap(
                SEND_CLOSED,
                "`send` vào kênh đã `close` (không nhận thêm phần tử)",
                span,
            ));
        }
        if let Some(rf) = ch.recv_waiters.pop_front() {
            ch.recv_values.insert(rf, v);
            drop(ch);
            host.sched().borrow_mut().frame_wake_chan(id, false, rf);
            wake_chan_timeouts(host, id, false);
            return Ok(Value::Unit);
        }
        if !ch.day() {
            ch.buf.push_back(v);
            drop(ch);
            wake_chan_timeouts(host, id, false);
            return Ok(Value::Unit);
        }
        ch.send_waiters.push_back(ChanSend {
            frame,
            value: v,
            state: ChanSendState::Pending,
        });
    }
    Err(pending(id, true, span))
}

/// **VS3-1d** — `recv` trên `chan(n≥1)` đường frame: lấy đệm/ghép sender, hoặc chờ.
fn recv_buf_frame(host: &mut dyn Host, c: Rc<RefCell<Chan>>, span: Span) -> R<Value> {
    let (id, frame) = hien_tai(host, &c);
    {
        let mut ch = c.borrow_mut();
        if let Some(v) = ch.recv_values.remove(&frame) {
            return Ok(Value::some(v));
        }
        if let Some(v) = ch.buf.pop_front() {
            // Vừa trống một chỗ: chuyển sender đang chờ vào đệm, đánh thức sender.
            let mut wake = None;
            if let Some(pos) = ch
                .send_waiters
                .iter()
                .position(|s| s.state == ChanSendState::Pending)
            {
                let f = ch.send_waiters[pos].frame;
                let nv = std::mem::replace(&mut ch.send_waiters[pos].value, Value::Unit);
                ch.send_waiters[pos].state = ChanSendState::Taken;
                ch.buf.push_back(nv);
                wake = Some(f);
            }
            drop(ch);
            if let Some(f) = wake {
                host.sched().borrow_mut().frame_wake_chan(id, true, f);
            }
            // Vừa có chỗ trống ⇒ đánh thức cả `send_timeout` đang chờ.
            wake_chan_timeouts(host, id, true);
            return Ok(Value::some(v));
        }
        if ch.closed {
            return Ok(Value::none());
        }
        if let Some(pos) = ch
            .send_waiters
            .iter()
            .position(|s| s.state == ChanSendState::Pending)
        {
            let f = ch.send_waiters[pos].frame;
            let v = std::mem::replace(&mut ch.send_waiters[pos].value, Value::Unit);
            ch.send_waiters[pos].state = ChanSendState::Taken;
            drop(ch);
            host.sched().borrow_mut().frame_wake_chan(id, true, f);
            wake_chan_timeouts(host, id, true);
            return Ok(Value::some(v));
        }
        ch.recv_waiters.push_back(frame);
    }
    Err(pending(id, false, span))
}

// ---------------------------------------------------------------------------
// **VS3-2b** — timeout kênh trên đường frame test.
//
// Không thêm timer heap mới: dùng **reactor timer** hiện có (`ReactorReg.deadline` +
// `ReactorEvent.deadline_ms`). Mỗi lần chờ timeout đăng ký một timer tổng hợp;
// channel event (send/recv/close) đánh thức timer bằng `ReactorOutcome::Ready(Unit)`,
// hết hạn bằng `ReactorOutcome::Error("chan timeout")`. Frame đang chờ ở
// `Wait::Reactor` (không phải `Wait::Chan`), nên không đụng driver `frame/**`.
// `ms == 0` chỉ thử một lần rồi trả timeout, không đăng ký timer. `ms < 0` ⇒ VS0501.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
enum TimeoutState {
    None,
    Pending { key: u64, gen: u64 },
    TimedOut,
    Woken,
}

fn timeout_handle(id: u32, send: bool) -> String {
    format!("chan-timeout:{id}:{}", u8::from(send))
}

fn probe_timeout(host: &mut dyn Host, id: u32, send: bool) -> TimeoutState {
    let frame = host.sched().borrow().frame.current;
    let mut s = host.sched().borrow_mut();
    let Some(reg) = s
        .frame
        .reactor
        .regs
        .iter()
        .find(|r| r.handle == timeout_handle(id, send) && r.frame == frame)
        .cloned()
    else {
        return TimeoutState::None;
    };
    let key = reg.key;
    let gen = reg.gen;
    let out = s.frame.reactor.results.remove(&key);
    s.frame.reactor.fired.remove(&key);
    s.frame.reactor.regs.retain(|r| r.key != key);
    s.frame.reactor.events.retain(|e| e.key != key);
    match out {
        Some(ReactorOutcome::Error(_)) => TimeoutState::TimedOut,
        Some(_) => TimeoutState::Woken,
        None => TimeoutState::Pending { key, gen },
    }
}

fn pending_timeout(key: u64, gen: u64, span: Span) -> Abort {
    Abort::Pending(Pending {
        task: None,
        span,
        wait: Some(Wait::Reactor { key, gen }),
    })
}

/// Đăng ký timer tổng hợp; trả `Pending` để driver đưa frame vào wait-set.
fn register_timeout(host: &mut dyn Host, id: u32, send: bool, ms: i64, span: Span) -> Abort {
    assert!(ms > 0);
    let frame = host.sched().borrow().frame.current;
    let now = Instant::now();
    let deadline = now
        .checked_add(Duration::from_millis(ms as u64))
        .unwrap_or(now);
    let mut s = host.sched().borrow_mut();
    let key = s.frame.reactor.next_key;
    s.frame.reactor.next_key += 1;
    let gen = 0;
    s.frame.reactor.regs.push(ReactorReg {
        key,
        handle: timeout_handle(id, send),
        op: ReactorOp::Read,
        frame,
        gen,
        token: 0,
        deadline: Some(deadline),
    });
    let ev = ReactorEvent {
        // Seq rất lớn: timer tổng hợp không được trùng trigger huỷ `cancel_on_seq` của test.
        seq: u64::MAX - key,
        key,
        gen,
        outcome: ReactorOutcome::Error("chan timeout".into()),
        deadline_ms: Some(ms as u64),
    };
    // Giữ event không-deadline (I/O) phía trước; timer sắp theo (deadline, seq) để nhiều
    // timeout đồng thời nổ đúng thứ tự thời gian.
    let d = ms as u64;
    let pos = s
        .frame
        .reactor
        .events
        .iter()
        .position(|e| match e.deadline_ms {
            Some(ed) => ed > d || (ed == d && e.key > key),
            None => false,
        })
        .unwrap_or(s.frame.reactor.events.len());
    s.frame.reactor.events.insert(pos, ev);
    s.frame
        .reactor
        .log
        .push(format!("chwtimer:{id}:{send}:{key}:{ms}"));
    drop(s);
    pending_timeout(key, gen, span)
}

/// Đánh thức mọi timer tổng hợp của `(id, send)` bằng một reactor event `Ready(Unit)`;
/// frame retry thấy channel đã sẵn sàng/đóng và hoàn tất. Không xoá timer reg — lần retry
/// `probe_timeout` xoá đúng một lần.
fn wake_chan_timeouts(host: &mut dyn Host, id: u32, send: bool) {
    let keys: Vec<(u64, u64)> = {
        let s = host.sched().borrow();
        s.frame
            .reactor
            .regs
            .iter()
            .filter(|r| r.handle == timeout_handle(id, send))
            .map(|r| (r.key, r.gen))
            .collect()
    };
    for (key, gen) in keys {
        {
            let mut s = host.sched().borrow_mut();
            s.frame.reactor.events.retain(|e| e.key != key);
        }
        let ev = ReactorEvent {
            seq: u64::MAX - key,
            key,
            gen,
            outcome: ReactorOutcome::Ready(Value::Unit),
            deadline_ms: None,
        };
        host.sched().borrow_mut().frame_reactor_fire(ev);
    }
}

fn tham_so_timeout(
    args: Vec<Value>,
    send: bool,
    name: &str,
    span: Span,
) -> Result<(i64, Option<Value>), Abort> {
    let mut it = args.into_iter();
    let Some(Value::Int(ms)) = it.next() else {
        return Err(trap(RT, format!("`{name}` cần đối số đầu `ms: int`"), span));
    };
    if ms < 0 {
        return Err(trap(
            RT,
            format!("`{name}`: `ms` không được âm (nhận {ms})"),
            span,
        ));
    }
    let v = if send {
        let Some(v) = it.next() else {
            return Err(trap(RT, format!("`{name}` cần giá trị gửi"), span));
        };
        Some(v)
    } else {
        None
    };
    if it.next().is_some() {
        return Err(trap(RT, format!("`{name}` nhận quá nhiều đối số"), span));
    }
    Ok((ms, v))
}

fn yeu_cau_frame(host: &dyn Host, name: &str, span: Span) -> Result<(), Abort> {
    if host.sched().borrow().frame.enabled {
        Ok(())
    } else {
        Err(trap(
            RT,
            format!(
                "`{name}` cần đường frame test/async (W1 sẽ flip async default); runtime đồng bộ chưa hỗ trợ"
            ),
            span,
        ))
    }
}

fn clear_send_waiter(c: &Rc<RefCell<Chan>>, frame: u32, taken: bool) -> bool {
    let mut ch = c.borrow_mut();
    if let Some(pos) = ch.send_waiters.iter().position(|s| s.frame == frame) {
        let state = ch.send_waiters[pos].state;
        ch.send_waiters.remove(pos);
        return state == ChanSendState::Taken || taken;
    }
    taken
}

fn clear_recv_waiter(c: &Rc<RefCell<Chan>>, frame: u32) -> Option<Value> {
    let mut ch = c.borrow_mut();
    ch.recv_waiters.retain(|f| *f != frame);
    ch.recv_values.remove(&frame)
}

fn recv_timeout(
    host: &mut dyn Host,
    c: Rc<RefCell<Chan>>,
    args: Vec<Value>,
    span: Span,
) -> R<Value> {
    let (ms, _) = tham_so_timeout(args, false, "recv_timeout", span)?;
    yeu_cau_frame(host, "recv_timeout", span)?;
    let (id, frame) = hien_tai(host, &c);
    match probe_timeout(host, id, false) {
        TimeoutState::TimedOut => {
            // Có thể sender đã ghép đúng lúc timer nổ; ưu tiên giá trị đã ghép.
            if let Some(v) = clear_recv_waiter(&c, frame) {
                return Ok(Value::some(v));
            }
            return Ok(Value::none());
        }
        TimeoutState::Pending { key, gen } => return Err(pending_timeout(key, gen, span)),
        TimeoutState::None | TimeoutState::Woken => {}
    }

    if c.borrow().cap == 0 {
        recv_timeout_rendezvous(host, c, ms, span)
    } else {
        recv_timeout_buffered(host, c, ms, span)
    }
}

fn recv_timeout_rendezvous(
    host: &mut dyn Host,
    c: Rc<RefCell<Chan>>,
    ms: i64,
    span: Span,
) -> R<Value> {
    let (id, frame) = hien_tai(host, &c);
    {
        let mut ch = c.borrow_mut();
        if let Some(v) = ch.recv_values.remove(&frame) {
            return Ok(Value::some(v));
        }
        if ch.closed {
            return Ok(Value::none());
        }
        if let Some(pos) = ch
            .send_waiters
            .iter()
            .position(|s| s.state == ChanSendState::Pending)
        {
            let f = ch.send_waiters[pos].frame;
            let v = std::mem::replace(&mut ch.send_waiters[pos].value, Value::Unit);
            ch.send_waiters[pos].state = ChanSendState::Taken;
            drop(ch);
            host.sched().borrow_mut().frame_wake_chan(id, true, f);
            wake_chan_timeouts(host, id, true);
            return Ok(Value::some(v));
        }
        ch.recv_waiters.push_back(frame);
    }
    if ms == 0 {
        let _ = clear_recv_waiter(&c, frame);
        return Ok(Value::none());
    }
    Err(register_timeout(host, id, false, ms, span))
}

fn recv_timeout_buffered(
    host: &mut dyn Host,
    c: Rc<RefCell<Chan>>,
    ms: i64,
    span: Span,
) -> R<Value> {
    let id = c.borrow().id;
    {
        let mut ch = c.borrow_mut();
        if let Some(v) = ch.buf.pop_front() {
            // Vừa có chỗ: chuyển sender đang chờ (nếu có) vào đệm, đánh thức sender.
            let mut wake = None;
            if let Some(pos) = ch
                .send_waiters
                .iter()
                .position(|s| s.state == ChanSendState::Pending)
            {
                let f = ch.send_waiters[pos].frame;
                let nv = std::mem::replace(&mut ch.send_waiters[pos].value, Value::Unit);
                ch.send_waiters[pos].state = ChanSendState::Taken;
                ch.buf.push_back(nv);
                wake = Some(f);
            }
            drop(ch);
            if let Some(f) = wake {
                host.sched().borrow_mut().frame_wake_chan(id, true, f);
            }
            wake_chan_timeouts(host, id, true);
            return Ok(Value::some(v));
        }
        if ch.closed {
            return Ok(Value::none());
        }
    }
    if ms == 0 {
        return Ok(Value::none());
    }
    Err(register_timeout(host, id, false, ms, span))
}

fn send_timeout(
    host: &mut dyn Host,
    c: Rc<RefCell<Chan>>,
    args: Vec<Value>,
    span: Span,
) -> R<Value> {
    let (ms, Some(v)) = tham_so_timeout(args, true, "send_timeout", span)? else {
        return Err(trap(RT, "`send_timeout` thiếu giá trị gửi", span));
    };
    yeu_cau_frame(host, "send_timeout", span)?;
    let (id, frame) = hien_tai(host, &c);
    match probe_timeout(host, id, true) {
        TimeoutState::TimedOut => {
            let taken = clear_send_waiter(&c, frame, false);
            return Ok(Value::Bool(taken));
        }
        TimeoutState::Pending { key, gen } => return Err(pending_timeout(key, gen, span)),
        TimeoutState::None | TimeoutState::Woken => {}
    }

    if c.borrow().cap == 0 {
        send_timeout_rendezvous(host, c, v, ms, span)
    } else {
        send_timeout_buffered(host, c, v, ms, span)
    }
}

fn send_timeout_rendezvous(
    host: &mut dyn Host,
    c: Rc<RefCell<Chan>>,
    v: Value,
    ms: i64,
    span: Span,
) -> R<Value> {
    let (id, frame) = hien_tai(host, &c);
    {
        let mut ch = c.borrow_mut();
        if let Some(pos) = ch.send_waiters.iter().position(|s| s.frame == frame) {
            match ch.send_waiters[pos].state {
                ChanSendState::Taken => {
                    ch.send_waiters.remove(pos);
                    return Ok(Value::Bool(true));
                }
                ChanSendState::Closed => {
                    ch.send_waiters.remove(pos);
                    return Ok(Value::Bool(false));
                }
                ChanSendState::Pending => {
                    drop(ch);
                    if ms == 0 {
                        let _ = clear_send_waiter(&c, frame, false);
                        return Ok(Value::Bool(false));
                    }
                    return Err(register_timeout(host, id, true, ms, span));
                }
            }
        }
        if ch.closed {
            return Ok(Value::Bool(false));
        }
        if let Some(rf) = ch.recv_waiters.pop_front() {
            ch.recv_values.insert(rf, v);
            drop(ch);
            host.sched().borrow_mut().frame_wake_chan(id, false, rf);
            wake_chan_timeouts(host, id, false);
            return Ok(Value::Bool(true));
        }
        ch.send_waiters.push_back(ChanSend {
            frame,
            value: v,
            state: ChanSendState::Pending,
        });
    }
    if ms == 0 {
        let _ = clear_send_waiter(&c, frame, false);
        return Ok(Value::Bool(false));
    }
    Err(register_timeout(host, id, true, ms, span))
}

fn send_timeout_buffered(
    host: &mut dyn Host,
    c: Rc<RefCell<Chan>>,
    v: Value,
    ms: i64,
    span: Span,
) -> R<Value> {
    let id = c.borrow().id;
    {
        let mut ch = c.borrow_mut();
        if ch.closed {
            return Ok(Value::Bool(false));
        }
        if let Some(rf) = ch.recv_waiters.pop_front() {
            ch.recv_values.insert(rf, v);
            drop(ch);
            host.sched().borrow_mut().frame_wake_chan(id, false, rf);
            wake_chan_timeouts(host, id, false);
            return Ok(Value::Bool(true));
        }
        if !ch.day() {
            ch.buf.push_back(v);
            drop(ch);
            wake_chan_timeouts(host, id, false);
            return Ok(Value::Bool(true));
        }
    }
    if ms == 0 {
        return Ok(Value::Bool(false));
    }
    Err(register_timeout(host, id, true, ms, span))
}

// ---------------------------------------------------------------------------
// Đường cũ: cap ≥ 1 (hoặc runtime mặc định không bật frame). Giữ nguyên ngữ nghĩa
// run-to-completion + drain task `Moi`, và `chay_mot` hết queue ⇒ `VS0505`.
// ---------------------------------------------------------------------------

fn send_buffered(host: &mut dyn Host, c: Rc<RefCell<Chan>>, v: Value, span: Span) -> R<Value> {
    loop {
        {
            let mut ch = c.borrow_mut();
            if ch.closed {
                return Err(trap(
                    SEND_CLOSED,
                    "`send` vào kênh đã `close` (không nhận thêm phần tử)",
                    span,
                ));
            }
            if !ch.day() {
                ch.buf.push_back(v);
                let id = ch.id;
                drop(ch);
                wake_chan_timeouts(host, id, false);
                return Ok(Value::Unit);
            }
        }
        if !crate::rt::chay_mot(host)? {
            return Err(trap(
                DEADLOCK,
                "deadlock: `send` vào kênh **đầy** và không còn tác vụ nào chạy được",
                span,
            ));
        }
    }
}

fn recv_buffered(host: &mut dyn Host, c: Rc<RefCell<Chan>>, span: Span) -> R<Value> {
    loop {
        {
            let mut ch = c.borrow_mut();
            if let Some(v) = ch.buf.pop_front() {
                let id = ch.id;
                drop(ch);
                wake_chan_timeouts(host, id, true);
                return Ok(Value::some(v));
            }
            if ch.closed {
                return Ok(Value::none());
            }
        }
        if !crate::rt::chay_mot(host)? {
            return Err(trap(
                DEADLOCK,
                "deadlock: `recv` trên kênh **rỗng** và không còn tác vụ nào chạy được",
                span,
            ));
        }
    }
}
