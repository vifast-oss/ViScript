// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-2a** — phương thức kênh + wait-set rendezvous `chan(0)` **chỉ đường frame test**.
//!
//! Tách khỏi `builtin.rs` để giữ R1. Runtime mặc định (`frame.enabled == false`) giữ nguyên
//! hoàn toàn: `chan(0)` vẫn bị `tao_kenh` từ chối, blocking `send`/`recv` vẫn drain task rồi
//! `VS0505`. Chỉ khi frame test bật và `cap == 0` mới dùng wait-set bên dưới.
//!
//! Bất biến:
//! - sender/receiver `cap(0)` chờ theo FIFO; ghép đúng một lần;
//! - `close` đánh thức receiver (nhận `None` sau khi hết giá trị đã ghép) và sender
//!   (**`VS0504`** — không gửi vào kênh đã đóng);
//! - không còn task chạy được ⇒ `frame_stuck_trap` trả `VS0505`, không treo;
//! - cancellation gỡ sender/receiver khỏi kênh (xem `Chan::huy_frame`).

use std::cell::RefCell;
use std::rc::Rc;

use vis_syntax::Span;

use super::{Chan, ChanSend, ChanSendState, Pending, Wait};
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
    let rendezvous = host.sched().borrow().frame.enabled && c.borrow().cap == 0;
    match name {
        "send" => {
            let Some(v) = args.into_iter().next() else {
                return Err(trap(RT, "`send` cần 1 đối số", span));
            };
            if rendezvous {
                send_rendezvous(host, c, v, span)
            } else {
                send_buffered(host, c, v, span)
            }
        }
        "recv" => {
            if rendezvous {
                recv_rendezvous(host, c, span)
            } else {
                recv_buffered(host, c, span)
            }
        }
        // `close` **idempotent** (VD-96 (3)): lần hai **không** lỗi.
        "close" => {
            if rendezvous {
                close_rendezvous(host, c)
            } else {
                c.borrow_mut().closed = true;
                Ok(Value::Unit)
            }
        }
        _ => Err(trap(
            RT,
            format!("kênh không có phương thức `{name}` (chỉ `send`, `recv`, `close`)"),
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
    Ok(Value::Unit)
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
