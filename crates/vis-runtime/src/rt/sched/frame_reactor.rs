// SPDX-License-Identifier: MIT OR Apache-2.0
//! API reactor giả (chỉ test): nạp trace, poll, fire, idle, shutdown — tách khỏi `sched.rs` ở
//! `ST-317b`.

use std::time::{Duration, Instant};

use vis_core::reactor::ReactorOp;
use vis_core::trap::{Abort, R};
use vis_core::value::{Pending, Value, Wait};
use vis_syntax::Span;

use super::scheduler::Scheduler;
use crate::rt::frame_test::{ReactorEvent, ReactorOutcome, ReactorReg, Waiter};
use crate::rt::ops::{err_str, ok_str};

impl Scheduler {
    // -----------------------------------------------------------------------
    // ST-268 / VD-252 — API reactor giả (CHỈ TEST). Mọi hàm ở đây là tất định,
    // không đọc socket, không đệ quy scheduler, không sleep-poll readiness.
    // -----------------------------------------------------------------------

    /// Nạp trace reactor giả từ config. `events` được sắp theo `seq` (tie-break bằng
    /// `key`) để thứ tự fire hoàn toàn do trace quyết định — không hash/địa chỉ.
    #[doc(hidden)]
    pub fn frame_reactor_load_for_test(&mut self, mut events: Vec<ReactorEvent>, real_wait: bool) {
        events.sort_by_key(|e| (e.seq, e.key, e.gen));
        self.frame.reactor.events = events;
        self.frame.reactor.epoch = Some(Instant::now());
        self.frame.timer_real_wait = real_wait;
        self.frame
            .reactor
            .log
            .push(format!("renable:{}", self.frame.reactor.events.len()));
    }

    /// `read`/`accept` trên đường frame test: nếu đã có kết quả từ event đã fire ⇒
    /// tiêu thụ (đúng một lần) rồi trả; nếu bị thức giả ⇒ re-arm với `gen` mới rồi
    /// `Pending`; nếu chưa sẵn sàng ⇒ đăng ký wait + `Pending`. KHÔNG gọi `NetHost`
    /// thật, KHÔNG block, KHÔNG spin.
    #[doc(hidden)]
    pub fn frame_reactor_poll(
        &mut self,
        op: ReactorOp,
        handle: &str,
        timeout_ms: i64,
        span: Span,
    ) -> R<Value> {
        let existing = self
            .frame
            .reactor
            .regs
            .iter()
            .find(|r| r.handle == handle && r.op == op)
            .cloned();
        let (key, gen) = match existing {
            Some(r) => (r.key, r.gen),
            None => {
                let key = self.frame.reactor.next_key;
                self.frame.reactor.next_key += 1;
                let frame = self.frame.current;
                self.frame.reactor.regs.push(ReactorReg {
                    key,
                    handle: handle.to_string(),
                    op,
                    frame,
                    gen: 0,
                    token: 0, // Lát 3: chỉ dùng khi có adapter thật
                    deadline: None,
                });
                self.frame.reactor.log.push(format!(
                    "reg:{}:{}:{:?}:{}:{}",
                    key, handle, op, frame, timeout_ms
                ));
                (key, 0)
            }
        };
        if let Some(out) = self.frame.reactor.results.remove(&key) {
            match out {
                ReactorOutcome::Ready(v) => {
                    self.frame.reactor.regs.retain(|r| r.key != key);
                    self.frame.reactor.log.push(format!("done:{}", key));
                    return Ok(v);
                }
                ReactorOutcome::Eof => {
                    self.frame.reactor.regs.retain(|r| r.key != key);
                    self.frame.reactor.log.push(format!("eof:{}", key));
                    return Ok(ok_str(""));
                }
                ReactorOutcome::Error(msg) => {
                    self.frame.reactor.regs.retain(|r| r.key != key);
                    self.frame.reactor.log.push(format!("err:{}", key));
                    return Ok(err_str(msg));
                }
                ReactorOutcome::Close => {
                    self.frame.reactor.regs.retain(|r| r.key != key);
                    self.frame.reactor.log.push(format!("closed:{}", key));
                    return Ok(err_str(format!("tay `{handle}` đã đóng khi đang chờ")));
                }
                // Phòng thủ: thức giả được xử lý khi fire (xem `frame_reactor_fire`).
                ReactorOutcome::Spurious => {}
            }
        }
        Err(Abort::Pending(Pending {
            task: None,
            span,
            wait: Some(Wait::Reactor { key, gen }),
        }))
    }

    /// Fire **một** event reactor theo thứ tự `seq`. Stale (gen lệch / không còn đăng
    /// ký) và duplicate (key đã fire ở gen này) chỉ ghi log — KHÔNG wake, KHÔNG rerun.
    #[doc(hidden)]
    pub fn frame_reactor_fire(&mut self, ev: ReactorEvent) {
        self.frame.ready_streak = 0;
        self.frame
            .reactor
            .log
            .push(format!("rfire:{}:{}:{}", ev.seq, ev.key, ev.gen));
        // VD-257/C: trigger huỷ theo seq (đường reactor) — xử lý trước khi wake/grant result.
        let cancels: Vec<u32> = self
            .frame
            .cancel_on_seq
            .iter()
            .filter(|(s, _)| *s == ev.seq)
            .map(|(_, id)| *id)
            .collect();
        for id in cancels {
            self.frame_cancel_request_for_test(id);
        }
        if !self.frame.cancel_req.is_empty() {
            self.frame_cancel_process_for_test();
        }
        // `key` là số thứ tự đăng ký thật (bắt đầu từ 0) — KHÔNG dùng sentinel 0 cho
        // timer, nếu không event stale của đăng ký đầu tiên sẽ đánh thức nhầm đăng ký mới.
        let key = ev.key;
        let reg = self
            .frame
            .reactor
            .regs
            .iter()
            .find(|r| r.key == key)
            .cloned();
        let Some(reg) = reg else {
            self.frame.reactor.log.push(format!("rstale:{}", key));
            return;
        };
        if reg.gen != ev.gen {
            self.frame
                .reactor
                .log
                .push(format!("rstale:{}:{}", key, ev.gen));
            return;
        }
        if self.frame.reactor.fired.get(&key).copied() == Some(ev.gen) {
            self.frame.reactor.log.push(format!("rdup:{}", key));
            return;
        }
        self.frame.reactor.fired.insert(key, ev.gen);

        match ev.outcome {
            ReactorOutcome::Spurious => {
                // Thức giả: re-arm với gen mới; wake frame để nó retry (sẽ gặp WouldBlock
                // và đăng ký lại với gen mới). Không stash kết quả ⇒ không hot spin.
                if let Some(r) = self.frame.reactor.regs.iter_mut().find(|r| r.key == key) {
                    r.gen += 1;
                }
                self.frame
                    .reactor
                    .log
                    .push(format!("rspurious:{}:{}", key, reg.gen + 1));
                self.wake_reactor(key, ev.gen);
            }
            ReactorOutcome::Close => {
                // Đóng tay: **vô hiệu event cũ bằng cách tăng gen**, giữ đăng ký lại đúng
                // một chu kỳ để lần retry tiêu thụ kết quả `Close` rồi mới gỡ (nếu gỡ ngay,
                // retry sẽ đăng ký lại ⇒ treo vô hạn / đánh thức nhầm tay mở lại).
                if let Some(r) = self.frame.reactor.regs.iter_mut().find(|r| r.key == key) {
                    r.gen += 1;
                }
                self.frame
                    .reactor
                    .results
                    .insert(key, ReactorOutcome::Close);
                self.frame.reactor.log.push(format!("rclose:{}", key));
                self.wake_reactor(key, ev.gen);
            }
            ReactorOutcome::Eof | ReactorOutcome::Error(_) | ReactorOutcome::Ready(_) => {
                // Lưu kết quả cho lần retry; wake waiter (mỗi frame một lần).
                self.frame.reactor.results.insert(key, ev.outcome);
                self.wake_reactor(key, ev.gen);
            }
        }
    }

    /// Đánh thức **mỗi** frame đang chờ `(key, gen)` đúng một lần (FIFO), dùng chung
    /// `frame_wake_one` ⇒ duplicate wake không tạo bản sao, không re-run.
    pub(crate) fn wake_reactor(&mut self, key: u64, gen: u64) {
        let ws: Vec<Waiter> = self
            .frame
            .waiters
            .iter()
            .filter(|w| w.wait == Wait::Reactor { key, gen })
            .cloned()
            .collect();
        for w in ws {
            self.frame_wake_one(w);
        }
    }

    /// Scheduler rảnh: nếu còn event reactor, **ngủ tới deadline của chính event kế tiếp**
    /// (theo `seq`) khi `timer_real_wait`, rồi fire event đó. Không vòng lặp poll.
    ///
    /// **VD-259**: bản cũ chọn deadline **nhỏ nhất trong toàn trace** rồi fire event đầu theo
    /// `seq` ⇒ timer có thể fire sớm (deadline của nó chưa tới) và I/O bị chặn bởi timer không
    /// liên quan. Nay mỗi event tự quyết định thời điểm; `Instant::checked_add` tránh tràn.
    #[doc(hidden)]
    pub fn frame_idle(&mut self) -> bool {
        if !self.frame.ready.is_empty() || !self.frame.events.is_empty() {
            return false;
        }
        if self.frame.selector_error.is_some() {
            return false; // lỗi selector đã được ghi — không chờ lại, driver sẽ trap
        }
        // Lát 3: có adapter thật ⇒ chờ selector (không cần trace reactor giả).
        if self.frame.backend.is_some() {
            return self.frame_idle_real();
        }
        if self.frame.reactor.events.is_empty() {
            return false;
        }
        // Fire theo `seq` cho tới khi có frame sẵn sàng hoặc hết trace. Event stale/
        // duplicate không wake ⇒ phải tiếp tục fire event sau, không được dừng sớm.
        while let Some(ev) = self.frame.reactor.events.first().cloned() {
            if let Some(deadline) = ev.deadline_ms {
                self.frame
                    .reactor
                    .log
                    .push(format!("timer:{}:{}", ev.key, deadline));
                if self.frame.timer_real_wait {
                    let epoch = *self.frame.reactor.epoch.get_or_insert_with(Instant::now);
                    // Deadline `u64` có thể vượt biểu diễn `Instant` — không panic, không ngủ.
                    if let Some(target) = epoch.checked_add(Duration::from_millis(deadline)) {
                        let now = Instant::now();
                        if target > now {
                            std::thread::sleep(target - now);
                        }
                    }
                }
            }
            self.frame.reactor.events.remove(0);
            self.frame_reactor_fire(ev);
            if !self.frame.ready.is_empty() {
                break;
            }
        }
        !self.frame.ready.is_empty()
    }

    /// Dọn toàn bộ trạng thái reactor sau khi task + handle test đã thả — đưa
    /// `diag_snapshot` về baseline mà KHÔNG cần đóng tay còn handle hợp lệ.
    #[doc(hidden)]
    pub fn frame_reactor_shutdown(&mut self) {
        // Lát 3: hủy mọi đăng ký còn sống ở selector thật trước khi xoá sổ sách core.
        let regs: Vec<ReactorReg> = self.frame.reactor.regs.to_vec();
        if let Some(b) = self.frame.backend.clone() {
            for r in &regs {
                let _ = b.borrow_mut().deregister(r.token, &r.handle, r.op);
            }
        }
        self.frame.token_map.clear();
        self.frame.reactor.regs.clear();
        self.frame.reactor.results.clear();
        self.frame.reactor.fired.clear();
        self.frame.reactor.events.clear();
        self.frame.timer_real_wait = false;
        self.frame.current = 0;
        self.frame.reactor.log.push("rshutdown".into());
    }
}
