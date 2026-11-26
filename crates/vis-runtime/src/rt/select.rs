// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-293 (VD-274/VD-280)** — `select_recv*`/`select_send*` trên **wait-set nhiều kênh**.
//!
//! Bốn hàm dựng sẵn có kiểu tĩnh (không đổi parser/EBNF):
//!
//! - `select_recv(chans: list[Chan[T]]) -> {index:int, value:T?}`
//! - `select_recv_timeout(chans, ms) -> {index:int, value:T?}?` (`ms<0` ⇒ `VS0501`; `ms=0` thăm dò)
//! - `select_send(chans, val) -> int?` (`None` nếu **tất cả** đóng)
//! - `select_send_timeout(chans, val, ms) -> int?`
//!
//! Bất biến:
//! - quét kênh theo thứ tự **xoay vòng** bởi `FrameTest.select_turn` (fairness);
//! - mỗi lần chọn chỉ **một** kênh thắng: `claim` chung `Rc<RefCell<Option<usize>>>` được kiểm
//!   bởi mọi thao tác kênh trước khi nhận/giao — nhánh thua không nhận giá trị lần hai;
//! - mọi waiter của frame (`Wait::Chan`) và timer của select được gỡ **đúng một lần** ở
//!   [`don_dep`]/[`huy_frame`]; double-wake bị chặn bởi `frame_wake_one` + log `duplicate`.
//! - `select_send` đăng ký `ChanSend { select: true }` trên các kênh chưa sẵn sàng; receiver
//!   lấy đúng một entry (claim), các entry còn lại được dọn.
//!
//! Chỉ đường frame test; runtime đồng bộ vẫn từ chối `select_*` với lỗi rõ.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use vis_syntax::Span;

use super::{
    Chan, ChanSend, ChanSendState, Pending, ReactorEvent, ReactorOp, ReactorOutcome, ReactorReg,
    Scheduler, Wait,
};
use crate::builtin::Host;
use crate::codes::RT;
use vis_core::trap::{Abort, Trap, R};
use vis_core::value::Value;

/// Trạng thái một lần `select_*` đang treo của frame.
#[derive(Debug)]
pub struct SelectState {
    pub(crate) chans: Vec<u32>,
    pub(crate) send: bool,
    /// Điểm bắt đầu xoay vòng (đã chốt cho cả lần chọn, kể cả retry).
    pub(crate) start: usize,
    /// Kênh thắng (index trong `chans`) — `None` khi chưa ai nhận.
    pub(crate) claim: Rc<RefCell<Option<usize>>>,
    /// Timer của `select_*_timeout` (key, gen) — `None` nếu không timeout/lần chọn không timeout.
    pub(crate) timer: Option<(u64, u64)>,
}

fn trap(msg: impl Into<String>, span: Span) -> Abort {
    Abort::Trap(Trap::new(RT, msg, span))
}

fn chans_tu_value(v: Value, ten: &str, span: Span) -> Result<Vec<Rc<RefCell<Chan>>>, Abort> {
    let Value::List(xs) = v else {
        return Err(trap(format!("`{ten}` cần `list[Chan[T]]`"), span));
    };
    if xs.is_empty() {
        return Err(trap(
            format!("`{ten}`: danh sách kênh **rỗng** — cần ít nhất một kênh (`VS0501`)"),
            span,
        ));
    }
    let mut out = Vec::with_capacity(xs.len());
    for x in xs.iter().cloned() {
        match x {
            Value::Chan(c) => out.push(c),
            other => {
                return Err(trap(
                    format!(
                        "`{ten}`: phần tử kênh phải `Chan[T]`, được `{}`",
                        other.show()
                    ),
                    span,
                ))
            }
        }
    }
    Ok(out)
}

fn ms_tu_value(v: Value, ten: &str, span: Span) -> Result<i64, Abort> {
    let Value::Int(ms) = v else {
        return Err(trap(format!("`{ten}`: `ms` phải `int`"), span));
    };
    if ms < 0 {
        return Err(trap(
            format!("`{ten}`: `ms` không được âm (nhận {ms})"),
            span,
        ));
    }
    Ok(ms)
}

/// Điểm vào từ `builtin::call`.
pub(crate) fn call(host: &mut dyn Host, name: &str, args: Vec<Value>, span: Span) -> R<Value> {
    if !host.sched().borrow().frame.enabled {
        return Err(trap(
            format!("`{name}` cần đường frame test/async — runtime đồng bộ chưa hỗ trợ `select`"),
            span,
        ));
    }
    let mut it = args.into_iter();
    let Some(chans_v) = it.next() else {
        return Err(trap(format!("`{name}` thiếu danh sách kênh"), span));
    };
    let chans = chans_tu_value(chans_v, name, span)?;
    match name {
        "select_recv" => {
            if it.next().is_some() {
                return Err(trap("`select_recv` nhận 1 đối số", span));
            }
            chon(host, &chans, false, None, None, span)
        }
        "select_recv_timeout" => {
            let Some(ms_v) = it.next() else {
                return Err(trap("`select_recv_timeout` thiếu `ms`", span));
            };
            if it.next().is_some() {
                return Err(trap("`select_recv_timeout` nhận 2 đối số", span));
            }
            let ms = ms_tu_value(ms_v, name, span)?;
            chon(host, &chans, false, Some(ms), None, span)
        }
        "select_send" => {
            let Some(v) = it.next() else {
                return Err(trap("`select_send` thiếu giá trị gửi", span));
            };
            if it.next().is_some() {
                return Err(trap("`select_send` nhận 2 đối số", span));
            }
            chon(host, &chans, true, None, Some(v), span)
        }
        "select_send_timeout" => {
            let Some(v) = it.next() else {
                return Err(trap("`select_send_timeout` thiếu giá trị gửi", span));
            };
            let Some(ms_v) = it.next() else {
                return Err(trap("`select_send_timeout` thiếu `ms`", span));
            };
            if it.next().is_some() {
                return Err(trap("`select_send_timeout` nhận 3 đối số", span));
            }
            let ms = ms_tu_value(ms_v, name, span)?;
            chon(host, &chans, true, Some(ms), Some(v), span)
        }
        _ => Err(trap(format!("builtin `{name}` không phải select"), span)),
    }
}

fn record(index: usize, value: Value) -> Value {
    Value::Record(
        String::new(),
        vec![
            ("index".to_string(), Value::Int(index as i64)),
            ("value".to_string(), value),
        ],
    )
}

/// Bọc kết quả recv: hàm không timeout trả **record**, hàm timeout trả **record?**.
fn recv_result(v: Value, timeout: Option<i64>) -> Value {
    if timeout.is_some() {
        Value::some(v)
    } else {
        v
    }
}

/// Chọn một kênh: quét theo `start` xoay vòng, nếu chưa sẵn sàng thì đăng ký wait-set.
fn chon(
    host: &mut dyn Host,
    chans: &[Rc<RefCell<Chan>>],
    send: bool,
    timeout: Option<i64>,
    val: Option<Value>,
    span: Span,
) -> R<Value> {
    let frame = host.sched().borrow().frame.current;
    let ids: Vec<u32> = chans.iter().map(|c| c.borrow().id).collect();
    // Lấy state cũ (retry) hoặc tạo mới. State khác call (kênh/chiều khác) ⇒ dọn rồi tạo mới.
    let cu = host.sched().borrow_mut().frame.select.remove(&frame);
    let st = match cu {
        Some(s) if s.chans == ids && s.send == send => s,
        Some(s) => {
            don_dep_kenh(host, &s.chans, frame, true);
            xoa_timer(host, &s);
            let start = {
                let mut sc = host.sched().borrow_mut();
                let t = sc.frame.select_turn;
                sc.frame.select_turn = t.wrapping_add(1);
                (t as usize) % ids.len()
            };
            SelectState {
                chans: ids.clone(),
                send,
                start,
                claim: Rc::new(RefCell::new(None)),
                timer: None,
            }
        }
        None => {
            let start = {
                let mut sc = host.sched().borrow_mut();
                let t = sc.frame.select_turn;
                sc.frame.select_turn = t.wrapping_add(1);
                (t as usize) % ids.len()
            };
            SelectState {
                chans: ids.clone(),
                send,
                start,
                claim: Rc::new(RefCell::new(None)),
                timer: None,
            }
        }
    };
    // Vào state trước khi quét (claim cần thấy nó).
    let start = st.start;
    let timer = st.timer;
    host.sched().borrow_mut().frame.select.insert(frame, st);
    // Retry: bỏ waiter cũ (nhánh thua) nhưng **giữ** recv_values của kênh thắng.
    don_waiters_cu(host, frame);
    // Timer: nếu đã nổ và chưa có kênh thắng ⇒ timeout.
    let timed_out = if let Some((key, gen)) = timer {
        if probe_timer(host, key, gen) {
            if let Some(s) = host.sched().borrow_mut().frame.select.get_mut(&frame) {
                s.timer = None;
            }
            true
        } else {
            false
        }
    } else {
        false
    };
    let claim = {
        let sc = host.sched().borrow();
        sc.frame.select.get(&frame).and_then(|s| *s.claim.borrow())
    };
    // 1) Kênh đã thắng từ lần trước (retry sau khi bị đánh thức).
    if let Some(i) = claim {
        if send {
            let st = host.sched().borrow_mut().frame.select.remove(&frame);
            if let Some(s) = st {
                xoa_timer(host, &s);
            }
            don_dep_kenh(host, &ids, frame, true);
            return Ok(Value::some(Value::Int(i as i64)));
        }
        let v = chans[i].borrow_mut().recv_values.remove(&frame);
        let closed = chans[i].borrow().closed;
        let st = host.sched().borrow_mut().frame.select.remove(&frame);
        if let Some(s) = st {
            xoa_timer(host, &s);
        }
        don_dep_kenh(host, &ids, frame, true);
        return Ok(if let Some(v) = v {
            recv_result(record(i, Value::some(v)), timeout)
        } else if closed {
            recv_result(record(i, Value::none()), timeout)
        } else {
            // Claim mà chưa có value (bị đánh thức bởi timer/kênh khác) — thử lại.
            host.sched().borrow_mut().frame.select.insert(
                frame,
                SelectState {
                    chans: ids.clone(),
                    send,
                    start,
                    claim: Rc::new(RefCell::new(Some(i))),
                    timer: None,
                },
            );
            return Err(Abort::Pending(Pending {
                task: None,
                span,
                wait: None,
            }));
        });
    }
    // 2) Quét mọi kênh theo thứ tự xoay vòng.
    for off in 0..chans.len() {
        let i = (start + off) % chans.len();
        let c = &chans[i];
        if send {
            if c.borrow().closed {
                continue;
            }
            // `send_timeout(0)` dùng đúng logic kênh hiện có (đệm/receiver chờ), không treo.
            let r = crate::rt::chan::method(
                host,
                Rc::clone(c),
                "send_timeout",
                vec![Value::Int(0), val.clone().unwrap_or(Value::Unit)],
                span,
            );
            match r {
                Ok(Value::Bool(true)) => {
                    let st = host.sched().borrow_mut().frame.select.remove(&frame);
                    if let Some(s) = st {
                        xoa_timer(host, &s);
                    }
                    don_dep_kenh(host, &ids, frame, true);
                    return Ok(Value::some(Value::Int(i as i64)));
                }
                Ok(Value::Bool(false)) => {}
                Ok(other) => {
                    return Err(trap(
                        format!("`select_send`: kênh trả giá trị lạ `{}`", other.show()),
                        span,
                    ))
                }
                Err(Abort::Pending(_)) => {}
                Err(e) => return Err(e),
            }
        } else {
            // `recv_timeout(0)` thăm dò; giá trị đã ghép cho chính frame được ưu tiên.
            if let Some(v) = c.borrow_mut().recv_values.remove(&frame) {
                let st = host.sched().borrow_mut().frame.select.remove(&frame);
                if let Some(s) = st {
                    xoa_timer(host, &s);
                }
                don_dep_kenh(host, &ids, frame, true);
                return Ok(recv_result(record(i, Value::some(v)), timeout));
            }
            let closed = c.borrow().closed;
            let r = crate::rt::chan::method(
                host,
                Rc::clone(c),
                "recv_timeout",
                vec![Value::Int(0)],
                span,
            );
            match r {
                Ok(v) => {
                    let mut some = None;
                    let mut la_none = false;
                    if let Value::Enum(_, var, mut args) = v {
                        if var.as_str() == "Some" {
                            some = args.pop();
                        } else {
                            la_none = true;
                        }
                    }
                    if let Some(vv) = some {
                        let st = host.sched().borrow_mut().frame.select.remove(&frame);
                        if let Some(s) = st {
                            xoa_timer(host, &s);
                        }
                        don_dep_kenh(host, &ids, frame, true);
                        return Ok(recv_result(record(i, Value::some(vv)), timeout));
                    }
                    if closed && la_none {
                        let st = host.sched().borrow_mut().frame.select.remove(&frame);
                        if let Some(s) = st {
                            xoa_timer(host, &s);
                        }
                        don_dep_kenh(host, &ids, frame, true);
                        return Ok(recv_result(record(i, Value::none()), timeout));
                    }
                }
                Err(Abort::Pending(_)) => {}
                Err(e) => return Err(e),
            }
        }
    }
    // 3) Không kênh nào sẵn sàng.
    let all_closed = chans.iter().all(|c| c.borrow().closed);
    if all_closed {
        let st = host.sched().borrow_mut().frame.select.remove(&frame);
        if let Some(s) = st {
            xoa_timer(host, &s);
        }
        don_dep_kenh(host, &ids, frame, true);
        return Ok(if send {
            Value::none()
        } else {
            recv_result(record(start, Value::none()), timeout)
        });
    }
    if timed_out {
        let st = host.sched().borrow_mut().frame.select.remove(&frame);
        if let Some(s) = st {
            xoa_timer(host, &s);
        }
        don_dep_kenh(host, &ids, frame, true);
        return Ok(Value::none());
    }
    // `ms == 0`: thăm dò đúng một lần.
    if timeout == Some(0) {
        let st = host.sched().borrow_mut().frame.select.remove(&frame);
        if let Some(s) = st {
            xoa_timer(host, &s);
        }
        don_dep_kenh(host, &ids, frame, true);
        return Ok(Value::none());
    }
    // 4) Đăng ký wait-set trên mọi kênh còn mở + timer (nếu có).
    let mut waits: Vec<(u32, bool)> = Vec::new();
    for (i, c) in chans.iter().enumerate() {
        if c.borrow().closed {
            continue;
        }
        {
            let mut ch = c.borrow_mut();
            if send {
                if !ch.send_waiters.iter().any(|s| s.frame == frame && s.select) {
                    ch.send_waiters.push_back(ChanSend {
                        frame,
                        value: val.clone().unwrap_or(Value::Unit),
                        state: ChanSendState::Pending,
                        select: true,
                    });
                }
            } else if !ch.recv_waiters.contains(&frame) {
                ch.recv_waiters.push_back(frame);
            }
        }
        waits.push((c.borrow().id, send));
        let _ = i;
    }
    if let Some(ms) = timeout {
        if ms > 0 {
            let (key, gen) = match timer {
                Some(t) => t,
                None => {
                    let t = dang_ky_timer(host, frame, ms);
                    if let Some(s) = host.sched().borrow_mut().frame.select.get_mut(&frame) {
                        s.timer = Some(t);
                    }
                    t
                }
            };
            // Đăng ký waiter reactor để timer đánh thức frame (retry dùng lại cùng key/gen).
            host.sched().borrow_mut().frame_register(super::Waiter {
                frame,
                wait: Wait::Reactor { key, gen },
                gen,
            });
        }
    }
    // Đăng ký các waiter kênh (driver đăng ký waiter đầu tiên trong `Pending`).
    let mut it = waits.into_iter();
    let Some((first_id, first_send)) = it.next() else {
        // Không còn kênh mở (đã xử lý all_closed ở trên) — phòng thủ.
        let st = host.sched().borrow_mut().frame.select.remove(&frame);
        if let Some(s) = st {
            xoa_timer(host, &s);
        }
        don_dep_kenh(host, &ids, frame, true);
        return Ok(Value::none());
    };
    for (id, send_dir) in it {
        host.sched().borrow_mut().frame_register(super::Waiter {
            frame,
            wait: Wait::Chan { id, send: send_dir },
            gen: 0,
        });
    }
    Err(Abort::Pending(Pending {
        task: None,
        span,
        wait: Some(Wait::Chan {
            id: first_id,
            send: first_send,
        }),
    }))
}

/// Gỡ waiter scheduler `Wait::Chan` của frame (không đụng recv_values đã thắng).
fn don_waiters_cu(host: &mut dyn Host, frame: u32) {
    host.sched()
        .borrow_mut()
        .frame
        .waiters
        .retain(|w| !(w.frame == frame && matches!(w.wait, Wait::Chan { .. })));
    let kenh = host.sched().borrow().kenh.clone();
    for c in kenh {
        let mut ch = c.borrow_mut();
        ch.recv_waiters.retain(|f| *f != frame);
        ch.send_waiters.retain(|s| !(s.frame == frame && s.select));
    }
}

/// Gỡ toàn bộ dấu vết select của frame: waiter, giá trị đã ghép, timer, state.
fn don_dep_kenh(host: &mut dyn Host, ids: &[u32], frame: u32, xoa_value: bool) {
    let kenh = host.sched().borrow().kenh.clone();
    for c in kenh {
        if !ids.contains(&c.borrow().id) {
            continue;
        }
        let mut ch = c.borrow_mut();
        ch.recv_waiters.retain(|f| *f != frame);
        ch.send_waiters.retain(|s| !(s.frame == frame && s.select));
        if xoa_value {
            ch.recv_values.remove(&frame);
        }
    }
    let _ = host.sched().borrow_mut().frame.select.remove(&frame);
}

/// `frame_cancel_one` gọi: dọn select của frame bị huỷ (state + timer + dấu vết kênh).
pub(crate) fn huy_frame(sched: &mut Scheduler, frame: u32) {
    if let Some(s) = sched.frame.select.remove(&frame) {
        let _ = s;
    }
    sched
        .frame
        .waiters
        .retain(|w| !(w.frame == frame && matches!(w.wait, Wait::Chan { .. })));
    let kenh = sched.kenh.clone();
    for c in kenh {
        let mut ch = c.borrow_mut();
        ch.recv_waiters.retain(|f| *f != frame);
        ch.send_waiters.retain(|s| !(s.frame == frame && s.select));
        ch.recv_values.remove(&frame);
    }
}

/// Claim `select_recv` cho frame trên kênh `id`; `true` nếu frame không select hoặc thắng claim.
pub(crate) fn claim_recv(host: &mut dyn Host, frame: u32, id: u32) -> bool {
    claim(host, frame, id)
}
/// Claim `select_send` cho frame trên kênh `id`.
pub(crate) fn claim_send(host: &mut dyn Host, frame: u32, id: u32) -> bool {
    claim(host, frame, id)
}

fn claim(host: &mut dyn Host, frame: u32, id: u32) -> bool {
    let mut s = host.sched().borrow_mut();
    let Some(st) = s.frame.select.get(&frame) else {
        return true;
    };
    if st.claim.borrow().is_some() {
        return false;
    }
    let Some(i) = st.chans.iter().position(|x| *x == id) else {
        return false;
    };
    *st.claim.borrow_mut() = Some(i);
    let msg = format!("selclaim:{}:{}:{}", frame, id, i);
    s.frame.log.push(msg);
    true
}

/// Tìm sender thường/select còn claim được (bỏ entry select đã thua claim).
pub(crate) fn vi_tri_sender(host: &mut dyn Host, id: u32, ch: &mut Chan) -> Option<usize> {
    let mut i = 0;
    while i < ch.send_waiters.len() {
        if ch.send_waiters[i].state == ChanSendState::Pending {
            let f = ch.send_waiters[i].frame;
            if ch.send_waiters[i].select && !claim_send(host, f, id) {
                ch.send_waiters.remove(i);
                continue;
            }
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Tìm receiver còn claim được (bỏ frame select đã thua claim).
pub(crate) fn pop_recv_waiter(host: &mut dyn Host, id: u32, ch: &mut Chan) -> Option<u32> {
    while let Some(f) = ch.recv_waiters.pop_front() {
        if claim_recv(host, f, id) {
            return Some(f);
        }
    }
    None
}

fn dang_ky_timer(host: &mut dyn Host, frame: u32, ms: i64) -> (u64, u64) {
    let now = Instant::now();
    let deadline = now
        .checked_add(Duration::from_millis(ms as u64))
        .unwrap_or(now);
    let mut s = host.sched().borrow_mut();
    let key = s.frame.reactor.next_key;
    s.frame.reactor.next_key += 1;
    let gen = 0;
    let handle = format!("select-timeout:{frame}:{key}");
    s.frame.reactor.regs.push(ReactorReg {
        key,
        handle: handle.clone(),
        op: ReactorOp::Read,
        frame,
        gen,
        token: 0,
        deadline: Some(deadline),
    });
    let ev = ReactorEvent {
        seq: u64::MAX - key,
        key,
        gen,
        outcome: ReactorOutcome::Error("select timeout".into()),
        deadline_ms: Some(ms as u64),
    };
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
        .push(format!("seltimer:{frame}:{key}:{ms}"));
    (key, gen)
}

fn probe_timer(host: &mut dyn Host, key: u64, gen: u64) -> bool {
    let mut s = host.sched().borrow_mut();
    let fired = s.frame.reactor.fired.get(&key).copied() == Some(gen);
    let out = s.frame.reactor.results.remove(&key);
    s.frame.reactor.results.remove(&key);
    s.frame.reactor.fired.remove(&key);
    s.frame.reactor.regs.retain(|r| r.key != key);
    s.frame.reactor.events.retain(|e| e.key != key);
    fired
        && matches!(
            out,
            Some(ReactorOutcome::Error(_)) | Some(ReactorOutcome::Ready(_))
        )
}

fn xoa_timer(host: &mut dyn Host, st: &SelectState) {
    if let Some((key, _)) = st.timer {
        let mut s = host.sched().borrow_mut();
        s.frame.reactor.results.remove(&key);
        s.frame.reactor.fired.remove(&key);
        s.frame.reactor.regs.retain(|r| r.key != key);
        s.frame.reactor.events.retain(|e| e.key != key);
        s.frame.waiters.retain(|w| {
            !matches!(
                w.wait,
                Wait::Reactor {
                    key: k,
                    ..
                } if k == key
            )
        });
    }
}
