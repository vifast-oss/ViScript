// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310a (VD-285)** — máy reactor/selector của `Scheduler` (test-only), tách khỏi `rt.rs`.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use vis_syntax::Span;

use vis_core::trap::{Abort, R};
use vis_core::value::Value;

use super::ops::{err_str, ok_str};
use super::{
    FrameReactorBackend, Pending, ReactorH2Outcome, ReactorH2Response, ReactorIoOutcome, ReactorOp,
    ReactorReg, Scheduler, Wait,
};

// ===========================================================================
// ST-271 / VD-254 — LÁT 3: readiness THẬT qua adapter selector (CHỈ test).
//
// Mọi hàm ở đây chỉ chạy khi `frame.backend.is_some()`. Không backend ⇒ Lát 2
// nguyên văn. Không `unsafe`, không fd/thư viện selector trong crate lõi.
// ===========================================================================
impl Scheduler {
    /// Cài adapter selector thật (test-only). Không API/CLI/ngôn ngữ nào gọi.
    #[doc(hidden)]
    pub fn frame_install_backend_for_test(&mut self, b: Rc<RefCell<dyn FrameReactorBackend>>) {
        self.frame.backend = Some(b);
    }

    /// Điểm vào dùng bởi `stdlib` cho `net.read`/`net.accept` trên đường frame.
    /// Có adapter ⇒ readiness thật; không ⇒ reactor giả Lát 2 (byte-identical).
    #[doc(hidden)]
    pub fn frame_reactor_poll_io(
        &mut self,
        op: ReactorOp,
        handle: &str,
        max: usize,
        timeout_ms: i64,
        span: Span,
    ) -> R<Value> {
        if let Some(b) = self.frame.backend.clone() {
            match self.frame_real_poll(&b, op, handle, max, timeout_ms, span, false)? {
                RealPoll::Value(v) => Ok(v),
                RealPoll::Loi(e) => Ok(err_str(e)),
                RealPoll::Bytes(_) => unreachable!("non-raw poll không trả byte thô"),
            }
        } else {
            let _ = max;
            self.frame_reactor_poll(op, handle, timeout_ms, span)
        }
    }

    /// **VS3-1d/W1f-HTTP** — như [`frame_reactor_poll_io`] nhưng trả **byte thô** cho parser
    /// HTTP async. Không dùng cho `net.read` (hợp đồng `str` lossy không đổi).
    #[doc(hidden)]
    pub fn frame_reactor_poll_bytes(
        &mut self,
        handle: &str,
        max: usize,
        timeout_ms: i64,
        span: Span,
    ) -> R<Result<Vec<u8>, String>> {
        let Some(b) = self.frame.backend.clone() else {
            return Ok(Err(
                "async HTTP cần reactor backend thật (không có backend)".to_string(),
            ));
        };
        match self.frame_real_poll(&b, ReactorOp::Read, handle, max, timeout_ms, span, true)? {
            RealPoll::Bytes(bytes) => Ok(Ok(bytes)),
            RealPoll::Loi(e) => Ok(Err(e)),
            RealPoll::Value(_) => Ok(Err(
                "reactor trả giá trị không phải byte cho HTTP async".to_string()
            )),
        }
    }

    /// **VS3-W2** — lái handshake h2 tới khi sẵn sàng; `WouldBlock` ⇒ đăng ký READ|WRITE
    /// và trả `Pending` để frame khác chạy. Lỗi backend trả `Ok(Err)` (core đổi thành Error).
    #[doc(hidden)]
    pub fn frame_reactor_h2_ready(
        &mut self,
        handle: &str,
        timeout_ms: i64,
        span: Span,
    ) -> R<Result<(), String>> {
        let Some(b) = self.frame.backend.clone() else {
            return Ok(Err(
                "HTTP/2 async cần reactor backend thật (không có backend)".to_string(),
            ));
        };
        let kq = b.borrow_mut().h2_ready(handle);
        match kq {
            Ok(true) => {
                self.frame_real_done(handle, ReactorOp::ReadWrite);
                Ok(Ok(()))
            }
            Ok(false) => {
                if let Some(msg) =
                    self.frame_real_wait(&b, ReactorOp::ReadWrite, handle, timeout_ms, span)?
                {
                    return Ok(Err(msg));
                }
                unreachable!("frame_real_wait chỉ trả Pending hoặc lỗi")
            }
            Err(e) => {
                self.frame_real_done(handle, ReactorOp::ReadWrite);
                Ok(Err(e))
            }
        }
    }

    /// **VS3-W2** — gửi request h2 (không chờ); trả id stream nội bộ.
    #[doc(hidden)]
    pub fn frame_reactor_h2_send(
        &mut self,
        handle: &str,
        method: &str,
        target: &str,
        headers: &[(String, String)],
    ) -> Result<u64, String> {
        let Some(b) = self.frame.backend.clone() else {
            return Err("HTTP/2 async cần reactor backend thật (không có backend)".to_string());
        };
        let id = b.borrow_mut().h2_send(handle, method, target, headers)?;
        // **ST-287 (VD-273)**: ghi sổ stream để safe point huỷ gửi `RST_STREAM` đúng stream.
        b.borrow_mut().note_task_h2(self.frame.current, handle, id);
        Ok(id)
    }

    /// **ST-287 (VD-273)** — ghi sổ tay HTTP async (`C<n>`/`H<n>#frame`/`N:…`/`T:…`) của frame
    /// hiện tại để safe point huỷ đóng đúng tay. Chỉ `net/async_http.rs` gọi; language handle
    /// của `net.read`/`net.write` không đi qua đây.
    #[doc(hidden)]
    pub fn frame_reactor_note_handle(&mut self, handle: &str) {
        if let Some(b) = self.frame.backend.clone() {
            b.borrow_mut().note_task_handle(self.frame.current, handle);
        }
    }

    /// **VS3-W2** — poll một bước h2 cho stream `id`; `WouldBlock` ⇒ đăng ký READ|WRITE.
    #[doc(hidden)]
    pub fn frame_reactor_h2_poll(
        &mut self,
        handle: &str,
        id: u64,
        timeout_ms: i64,
        span: Span,
    ) -> R<Result<ReactorH2Response, String>> {
        let Some(b) = self.frame.backend.clone() else {
            return Ok(Err(
                "HTTP/2 async cần reactor backend thật (không có backend)".to_string(),
            ));
        };
        let kq = b.borrow_mut().h2_poll(handle, id);
        match kq {
            ReactorH2Outcome::WouldBlock => {
                if let Some(msg) =
                    self.frame_real_wait(&b, ReactorOp::ReadWrite, handle, timeout_ms, span)?
                {
                    return Ok(Err(msg));
                }
                unreachable!("frame_real_wait chỉ trả Pending hoặc lỗi")
            }
            ReactorH2Outcome::Response(r) => {
                self.frame_real_done(handle, ReactorOp::ReadWrite);
                Ok(Ok(r))
            }
            ReactorH2Outcome::Err(e) => {
                self.frame_real_done(handle, ReactorOp::ReadWrite);
                Ok(Err(e))
            }
        }
    }

    /// **VS3-W2** — huỷ riêng stream (timeout/cancel) nhưng giữ session cho stream khác.
    #[doc(hidden)]
    pub fn frame_reactor_h2_cancel(&mut self, handle: &str, id: u64) {
        if let Some(b) = self.frame.backend.clone() {
            b.borrow_mut().h2_cancel(handle, id);
        }
    }

    /// Lấy (và xoá) lỗi selector để driver đổi thành trap rõ — không bao giờ coi
    /// lỗi selector là "không có sự kiện".
    #[doc(hidden)]
    pub fn frame_take_selector_error(&mut self) -> Option<String> {
        self.frame.selector_error.take()
    }

    /// Huỷ mọi đăng ký liên quan đến `handle` (dùng khi `net.close` trên đường frame).
    /// Đánh thức waiter để lần retry thấy handle đã mất ⇒ `Result.Err` đúng một lần.
    #[doc(hidden)]
    pub fn frame_reactor_cancel_handle(&mut self, handle: &str) {
        let regs: Vec<ReactorReg> = self
            .frame
            .reactor
            .regs
            .iter()
            .filter(|r| r.handle == handle)
            .cloned()
            .collect();
        for r in &regs {
            if let Some(b) = self.frame.backend.clone() {
                let _ = b.borrow_mut().deregister(r.token, &r.handle, r.op);
            }
            self.frame.token_map.remove(&r.token);
            self.frame.reactor.regs.retain(|x| x.key != r.key);
            self.frame
                .reactor
                .log
                .push(format!("rcancel:{}:{}", handle, r.key));
            self.wake_reactor(r.key, r.gen);
        }
    }

    fn mint_token(&mut self) -> u64 {
        let t = self.frame.next_token;
        self.frame.next_token += 1;
        t
    }

    /// Gỡ đăng ký selector + sổ token cho mọi reg của `(handle, op)` — idempotent.
    fn frame_real_done(&mut self, handle: &str, op: ReactorOp) {
        let regs: Vec<ReactorReg> = self
            .frame
            .reactor
            .regs
            .iter()
            .filter(|r| r.handle == handle && r.op == op)
            .cloned()
            .collect();
        for r in &regs {
            if let Some(b) = self.frame.backend.clone() {
                let _ = b.borrow_mut().deregister(r.token, &r.handle, r.op);
            }
            self.frame.token_map.remove(&r.token);
        }
        self.frame
            .reactor
            .regs
            .retain(|r| !(r.handle == handle && r.op == op));
    }

    /// `read`/`accept` thật: thử I/O trước; `WouldBlock` ⇒ đăng ký/re-arm + `Pending`.
    /// Kết quả EOF/lỗi/hết giờ gỡ đăng ký đúng một lần rồi trả `Result` (không trap).
    #[allow(clippy::too_many_arguments)]
    fn frame_real_poll(
        &mut self,
        b: &Rc<RefCell<dyn FrameReactorBackend>>,
        op: ReactorOp,
        handle: &str,
        max: usize,
        timeout_ms: i64,
        span: Span,
        raw: bool,
    ) -> R<RealPoll> {
        let outcome = match op {
            ReactorOp::Read => b.borrow_mut().try_read(handle, max),
            ReactorOp::Accept => b.borrow_mut().try_accept(handle),
            ReactorOp::ReadWrite => {
                unreachable!("ReadWrite chỉ dùng cho frame_reactor_h2_*")
            }
        };
        match outcome {
            ReactorIoOutcome::Data(bytes) => {
                self.frame_real_done(handle, op);
                self.frame_dem_byte_va_yeu_cau_huy(handle, bytes.len());
                self.frame.reactor.log.push(format!("rdone:{}", handle));
                if raw {
                    return Ok(RealPoll::Bytes(bytes));
                }
                return Ok(RealPoll::Value(ok_str(String::from_utf8_lossy(&bytes))));
            }
            ReactorIoOutcome::Accepted(new_handle) => {
                self.frame_real_done(handle, op);
                self.frame.reactor.log.push(format!("raccepted:{}", handle));
                return Ok(RealPoll::Value(ok_str(new_handle)));
            }
            ReactorIoOutcome::Err(msg) => {
                self.frame_real_done(handle, op);
                self.frame
                    .reactor
                    .log
                    .push(format!("rerr:{}:{}", handle, msg));
                return Ok(RealPoll::Loi(msg));
            }
            ReactorIoOutcome::WouldBlock => {}
        }

        if let Some(msg) = self.frame_real_wait(b, op, handle, timeout_ms, span)? {
            return Ok(RealPoll::Loi(msg));
        }
        unreachable!("frame_real_wait chỉ trả Pending hoặc lỗi")
    }

    /// Đăng ký/re-arm chờ `(handle, op)` và trả `Abort::Pending`; dùng chung cho mọi op.
    fn frame_real_wait(
        &mut self,
        b: &Rc<RefCell<dyn FrameReactorBackend>>,
        op: ReactorOp,
        handle: &str,
        timeout_ms: i64,
        span: Span,
    ) -> R<Option<String>> {
        let deadline = if timeout_ms > 0 {
            Some(Instant::now() + Duration::from_millis(timeout_ms as u64))
        } else {
            None // timeout không dương: không có hạn riêng, vẫn có trần chờ selector
        };
        let existing = self
            .frame
            .reactor
            .regs
            .iter()
            .find(|r| r.handle == handle && r.op == op)
            .cloned();

        let (key, gen) = match existing {
            Some(r) => {
                // Waiter MỚI join cùng `(handle, op)` khi reg đã có: dùng chung key/gen,
                // KHÔNG re-arm (re-arm ở đây sẽ làm mất wake của waiter trước đó).
                let notified = self.frame.reactor.fired.get(&r.key).copied() == Some(r.gen);
                if !notified {
                    self.frame
                        .reactor
                        .log
                        .push(format!("rjoin:{}:{}:{}", r.key, r.gen, self.frame.current));
                    return Err(Abort::Pending(Pending {
                        task: None,
                        span,
                        wait: Some(Wait::Reactor {
                            key: r.key,
                            gen: r.gen,
                        }),
                    }));
                }
                // Hết hạn riêng ⇒ timeout, gỡ đúng một lần (wake đã đưa frame tới đây).
                if r.deadline.map(|d| Instant::now() >= d).unwrap_or(false) {
                    self.frame_real_done(handle, op);
                    self.frame
                        .reactor
                        .log
                        .push(format!("rtimeout:{}:{}", handle, r.key));
                    return Ok(Some(format!(
                        "quá thời gian chờ dữ liệu từ `{handle}` sau {timeout_ms} ms"
                    )));
                }
                // Thức giả (wake nhưng vẫn WouldBlock): token MỚI + gen MỚI ⇒ wake cũ stale.
                let _ = b.borrow_mut().deregister(r.token, &r.handle, r.op);
                self.frame.token_map.remove(&r.token);
                let gen = r.gen + 1;
                let token = self.mint_token();
                // Giữ kết quả trước khi `frame_real_done` (tránh RefMut sống qua `if let`).
                let reg = b.borrow_mut().register(token, handle, op);
                if let Err(e) = reg {
                    self.frame_real_done(handle, op);
                    return Ok(Some(format!("lỗi đăng ký readiness cho `{handle}`: {e}")));
                }
                self.frame.token_map.insert(token, (r.key, gen));
                if let Some(reg) = self.frame.reactor.regs.iter_mut().find(|x| x.key == r.key) {
                    reg.gen = gen;
                    reg.token = token;
                    // Giữ deadline ban đầu ⇒ tổng thời gian chờ không bị reset bởi wake giả.
                }
                self.frame
                    .reactor
                    .log
                    .push(format!("rrearm:{}:{}:{}", r.key, gen, token));
                (r.key, gen)
            }
            None => {
                let key = self.frame.reactor.next_key;
                self.frame.reactor.next_key += 1;
                let token = self.mint_token();
                if let Err(e) = b.borrow_mut().register(token, handle, op) {
                    return Ok(Some(format!("lỗi đăng ký readiness cho `{handle}`: {e}")));
                }
                self.frame.token_map.insert(token, (key, 0));
                self.frame.reactor.regs.push(ReactorReg {
                    key,
                    handle: handle.to_string(),
                    op,
                    frame: self.frame.current,
                    gen: 0,
                    token,
                    deadline,
                });
                self.frame.reactor.log.push(format!(
                    "rreg:{}:{}:{:?}:{}:{}",
                    key, handle, op, self.frame.current, timeout_ms
                ));
                (key, 0)
            }
        };
        Err(Abort::Pending(Pending {
            task: None,
            span,
            wait: Some(Wait::Reactor { key, gen }),
        }))
    }

    /// Scheduler rảnh + có adapter: chờ selector tới deadline gần nhất (có trần),
    /// rồi đưa event thật vào `wake_reactor`. Hết hạn ⇒ wake để retry trả timeout.
    pub(super) fn frame_idle_real(&mut self) -> bool {
        // Không có đăng ký nào ⇒ không có gì đánh thức: KHÔNG ngủ (tránh chờ vô ích).
        if self.frame.reactor.regs.is_empty() {
            return false;
        }
        const CAP: Duration = Duration::from_secs(5);
        let now = Instant::now();
        let nearest = self
            .frame
            .reactor
            .regs
            .iter()
            .filter_map(|r| r.deadline)
            .min();
        let timeout = match nearest {
            Some(d) if d > now => (d - now).min(CAP),
            Some(_) => Duration::from_millis(1), // đã quá hạn: không chờ 0, đợi 1 ms rồi xử lý
            None => CAP,
        };
        let b = match self.frame.backend.clone() {
            Some(b) => b,
            None => return false,
        };
        let events = match b.borrow_mut().wait(timeout) {
            Ok(e) => e,
            Err(e) => {
                self.frame.selector_error = Some(format!("lỗi selector khi chờ readiness: {e}"));
                self.frame.reactor.log.push(format!("rselerr:{}", e));
                return false;
            }
        };
        let mut woke = false;
        for ev in events {
            let flags = format!(
                "{}{}{}",
                if ev.readable { "r" } else { "-" },
                if ev.hup { "h" } else { "-" },
                if ev.error { "e" } else { "-" }
            );
            let Some((key, gen)) = self.frame.token_map.remove(&ev.token) else {
                self.frame
                    .reactor
                    .log
                    .push(format!("rstale:token:{}", ev.token));
                continue;
            };
            let live = self
                .frame
                .reactor
                .regs
                .iter()
                .find(|r| r.key == key)
                .map(|r| r.gen);
            if live != Some(gen) {
                self.frame
                    .reactor
                    .log
                    .push(format!("rstale:{}:{}", key, gen));
                continue;
            }
            self.frame
                .reactor
                .log
                .push(format!("rev:{}:{}:{}", key, gen, flags));
            // Đánh dấu đã giao event cho thế hệ này — lần retry tiếp theo mới được
            // coi là "wake thật" và re-arm; waiter MỚI.join cùng reg thì không.
            self.frame.reactor.fired.insert(key, gen);
            self.wake_reactor(key, gen);
            woke = true;
        }
        if !woke {
            // `now` được lấy TRƯỚC khi chờ — phải tính lại sau `poll`, nếu không
            // deadline vừa hết sẽ không được coi là hạn (timeout bị nuốt).
            let now = Instant::now();
            let expired: Vec<(u64, u64)> = self
                .frame
                .reactor
                .regs
                .iter()
                .filter(|r| r.deadline.map(|d| d <= now).unwrap_or(false))
                .map(|r| (r.key, r.gen))
                .collect();
            for (key, gen) in expired {
                self.frame
                    .reactor
                    .log
                    .push(format!("rdeadline:{}:{}", key, gen));
                self.frame.reactor.fired.insert(key, gen);
                self.wake_reactor(key, gen);
                woke = true;
            }
        }
        woke
    }
}

/// Kết quả nội bộ của một lần thử reactor thật — cho phép `net.read` giữ chuỗi lossy
/// còn HTTP async nhận **byte thô** (không cắt UTF-8/không đổi Content-Length).
pub(super) enum RealPoll {
    Value(Value),
    Bytes(Vec<u8>),
    Loi(String),
}
