// SPDX-License-Identifier: MIT OR Apache-2.0
//! VD-257: tách `impl Scheduler` (API frame) ra khỏi `rt.rs` để qua luật R1 (≤1500 dòng).
//! Module con thấy trường private của `Scheduler` qua `use super::*`; API không đổi.
use super::*;

impl Scheduler {
    /// Bật đường frame. Gọi từ integration test trên `Interp`/`Vm` thật — **không**
    /// có CLI flag hay API ngôn ngữ nào gọi tới đây.
    #[doc(hidden)]
    pub fn frame_enable_for_test(
        &mut self,
        fake_sources: Vec<(String, u64)>,
        events: Vec<FakeEvent>,
    ) {
        self.frame.enabled = true;
        self.frame.fake_sources = fake_sources;
        self.frame.events = events.into();
        self.frame.gen = 0;
        self.frame.log.push("enable".into());
        // VD-257/C: nhận trigger huỷ do test ghi trước khi chạy engine. Sắp xếp để trace
        // tất định và giống hệt giữa interp/VM. Thread-local drained ⇒ mỗi engine nhận 1 lần.
        FRAME_CANCEL_TEST.with(|x| {
            self.frame.cancel_on_seq = x.borrow_mut().drain(..).collect();
        });
        self.frame.cancel_on_seq.sort();
        for (seq, id) in &self.frame.cancel_on_seq {
            self.frame.log.push(format!("cancelon:{}:{}", seq, id));
        }
    }

    /// Tăng thế hệ khi một nguồn giả được **đăng ký lại** (re-arm) — event cũ phải
    /// trở thành stale. Chỉ test.
    #[doc(hidden)]
    pub fn frame_rearm_for_test(&mut self, key: u64) {
        self.frame.gen += 1;
        for (id, (k, g)) in self.frame.external.iter_mut() {
            if *k == key {
                *g = self.frame.gen;
                self.frame
                    .log
                    .push(format!("rearm:{}:{}", id, self.frame.gen));
            }
        }
    }

    /// **Chỉ test**: `await_task` trên đường frame — `None` nếu task đã xong (đi cache),
    /// ngược lại trả `Waiter` để driver đăng ký rồi treo frame hiện tại.
    #[doc(hidden)]
    pub fn frame_await(&self, current_frame: u32, t: &Rc<Task>) -> Option<Waiter> {
        if t.trang_thai.get() == TrangThai::Xong {
            return None;
        }
        // Đăng ký wait theo **id + thế hệ hiện hành của task được await**.
        let (wait, gen) = match self.frame.external.get(&t.id) {
            Some((key, g)) => (Wait::Event { key: *key, gen: *g }, *g),
            None => (Wait::Task(t.id), self.frame.gen),
        };
        Some(Waiter {
            frame: current_frame,
            wait,
            gen,
        })
    }

    /// **Chỉ test**: đăng ký một waiter đã được `frame_await` quyết định. Trùng waiter
    /// (cùng frame + cùng wait + cùng gen) bị bỏ ⇒ duplicate wake không tạo bản sao.
    #[doc(hidden)]
    pub fn frame_register(&mut self, w: Waiter) {
        if self.frame.waiters.contains(&w) {
            self.frame.log.push(format!("dup-wait:{}", w.frame));
            return;
        }
        self.frame.waiters.push(w);
        self.frame
            .log
            .push(format!("wait:{}:{:?}", w.frame, w.wait));
        // Frame đang chờ KHÔNG nằm trong ready. Nó đã có thể nằm đó từ spawn — bỏ ra.
        self.frame.ready.retain(|f| *f != w.frame);
    }

    /// **Chỉ test**: lấy frame tiếp theo cần chạy. Ưu tiên FIFO `ready`; nếu rỗng và
    /// còn event giả, fire event có `seq` nhỏ nhất (thứ tự recorded trong trace) và
    /// đánh thức waiter — mỗi frame được nối **vào đuôi** đúng một lần.
    #[doc(hidden)]
    pub fn frame_next(&mut self) -> Option<u32> {
        // VD-257/C: mỗi lần scheduler bước là một SAFE POINT — xử lý request đã ghi nhận,
        // trước khi chọn frame tiếp theo. Không bao giờ cắt ngang `step_frame`.
        if !self.frame.cancel_req.is_empty() {
            self.frame_cancel_process_for_test();
        }
        if let Some(f) = self.frame.ready.pop_front() {
            return Some(f);
        }
        // Không còn frame sẵn sàng ⇒ fire sự kiện giả tiếp theo (theo `seq`).
        if let Some(ev) = self.frame.events.pop_front() {
            self.frame_fire(ev);
            return self.frame.ready.pop_front();
        }
        // Lát 2: hết frame + hết event Lát 1 ⇒ chờ reactor / timer monotonic rồi fire
        // **một** event reactor theo `seq` (xem `frame_idle`). Không sleep-poll.
        if self.frame_idle() {
            return self.frame.ready.pop_front();
        }
        None
    }

    /// **Chỉ test**: fire một sự kiện giả. Bỏ event **stale** (gen lệch) và **trùng**
    /// (khóa đã fire ở thế hệ hiện hành) — không đánh thức ai, ghi log tất định.
    fn frame_fire(&mut self, ev: FakeEvent) {
        self.frame
            .log
            .push(format!("fire:{}:{}:{}", ev.seq, ev.key, ev.gen));
        // VD-257/C: event có `seq` được đăng ký huỷ ⇒ ghi nhận + xử lý NGAY tại safe point
        // này, TRƯỚC khi complete target/wake. Task bị huỷ mất waiter ⇒ không chạy lại source.
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
        // Tìm task nguồn giả có khóa này.
        let target: Option<u32> = self
            .frame
            .external
            .iter()
            .find(|(_, (k, g))| *k == ev.key && *g == ev.gen)
            .map(|(id, _)| *id);
        let Some(id) = target else {
            self.frame.log.push(format!("stale:{}", ev.key));
            return;
        };
        if self.frame.fired.get(&ev.key).copied() == Some(ev.gen) {
            self.frame.log.push(format!("duplicate:{}", ev.key));
            return;
        }
        self.frame.fired.insert(ev.key, ev.gen);
        // Hoàn tất nguồn giả (kết quả do trace cung cấp).
        if let Some(t) = self.tasks.iter().find(|t| t.id == id).cloned() {
            if t.trang_thai.get() != TrangThai::Xong {
                *t.kq.borrow_mut() = Some(Ok(ev.value));
                t.trang_thai.set(TrangThai::Xong);
                self.frame.joined.insert(id);
            }
        }
        // Đánh thức waiter theo thứ tự đăng ký (FIFO), nối vào đuối ready.
        let waits: Vec<Waiter> = self.frame.waiters.clone();
        for w in waits {
            if w.wait
                == (Wait::Event {
                    key: ev.key,
                    gen: ev.gen,
                })
                && w.gen == ev.gen
            {
                self.frame_wake_one(w);
            }
        }
    }

    /// **Chỉ test**: đánh thức đúng một frame — nếu chưa có trong ready. Đăng ký bị
    /// gỡ ngay (registry không giữ frame đã thức), nên fire trùng không thức lại lần 2.
    fn frame_wake_one(&mut self, w: Waiter) {
        self.frame.waiters.retain(|x| *x != w);
        // VD-257/C: task đã huỷ KHÔNG được xếp lại vào ready — không late wake, không re-run.
        if self.frame.cancelled.contains(&w.frame) {
            self.frame.log.push(format!("cancelwake:{}", w.frame));
            return;
        }
        if self.frame.ready.contains(&w.frame) {
            self.frame.log.push(format!("dup-ready:{}", w.frame));
            return;
        }
        self.frame.ready.push_back(w.frame);
        self.frame.log.push(format!("wake:{}", w.frame));
    }

    /// **Chỉ test**: một task vừa xong — cache kết quả, ghi nghĩa vụ join, đánh thức
    /// mọi frame đang chờ task đó (FIFO, nối đuôi).
    #[doc(hidden)]
    pub fn frame_task_done(&mut self, id: u32, kq: Result<Value, Trap>) {
        self.frame.joined.insert(id);
        let waits: Vec<Waiter> = self.frame.waiters.clone();
        for w in waits {
            if w.wait == Wait::Task(id) {
                self.frame_wake_one(w);
            }
        }
        let _ = kq; // engine đã ghi vào `t.kq`; ở đây chỉ điều phối wake.
    }

    /// **Chỉ test**: fire nốt các event còn lại trong trace —duplicate/stale cũng được
    /// ghi log ⇒ hai engine so sánh được trace **đầy đủ**, không phải trace cắt ngắn.
    #[doc(hidden)]
    pub fn frame_drain_events(&mut self) {
        while let Some(ev) = self.frame.events.pop_front() {
            self.frame_fire(ev);
        }
        // Lát 2: fire nốt event reactor còn lại (duplicate/stale vẫn được log) để trace
        // đầy đủ giữa hai engine — không cắt ngắn.
        let evs = std::mem::take(&mut self.frame.reactor.events);
        for ev in evs {
            self.frame_reactor_fire(ev);
        }
    }

    /// **Chỉ test**: main kết thúc — join mọi task chưa xong (không auto-cancel), rồi
    /// thu hồi các task đã xong + không còn handle sống + đã có nghĩa vụ join.
    ///
    /// **ST-278 (VD-256)**: trước đây hàm này **nuốt** lỗi task đã cache (`kq = Some(Err(..))`)
    /// — task lỗi mà main không `await` vẫn làm chương trình exit 0. Nay lỗi đó được **đẩy ra**
    /// theo đúng hợp đồng `VS0506` (mã gốc + span lúc tạo, do driver bọc sẵn) và trả về cho driver
    /// quyết định trap. **Không** đổi chính sách hủy/đánh thức các task còn lại.
    #[doc(hidden)]
    pub fn frame_final_join(&mut self) -> Option<Trap> {
        // Quét theo **thứ tự tạo** (cũng là thứ tự join) để kết quả tất định.
        let err = self
            .tasks
            .iter()
            .find_map(|t| match t.kq.borrow().as_ref() {
                Some(Err(tr)) => Some(tr.clone()),
                _ => None,
            });
        let ids: Vec<u32> = self.tasks.iter().map(|t| t.id).collect();
        for id in ids {
            self.frame.joined.insert(id);
        }
        self.frame_reclaim();
        if err.is_some() {
            self.frame.final_join_error = err.clone();
        }
        err
    }

    /// Lấy (và xoá) lỗi task bị nuốt ở final join — driver gán vào trap chung.
    #[doc(hidden)]
    pub fn frame_take_final_join_error_for_test(&mut self) -> Option<Trap> {
        self.frame.final_join_error.take()
    }

    /// **Chỉ test**: thu hồi task thỏa 3 điều kiện ownership: terminal (`Xong` **hoặc**
    /// `Huy` VD-257/C), có nghĩa vụ join, và không còn handle/reference sống
    /// (`Rc::strong_count == 1` chỉ còn scheduler). ready/waiter/registration cũng bị dọn
    /// theo — không giữ task chết quá ý đồ. Task `Huy` không có `kq` lỗi ⇒ final-join không
    /// bịa mã huỷ mới (xem BLOCKER trong report VD-257/C).
    #[doc(hidden)]
    pub fn frame_reclaim(&mut self) {
        let dead: Vec<u32> = self
            .tasks
            .iter()
            .filter(|t| {
                (t.trang_thai.get() == TrangThai::Xong || t.trang_thai.get() == TrangThai::Huy)
                    && self.frame.joined.contains(&t.id)
                    && Rc::strong_count(t) == 1
            })
            .map(|t| t.id)
            .collect();
        if dead.is_empty() {
            return;
        }
        for id in &dead {
            self.frame.log.push(format!("reclaim:{}", id));
        }
        self.tasks.retain(|t| !dead.contains(&t.id));
        self.frame.waiters.retain(|w| !dead.contains(&w.frame));
        self.frame.external.retain(|id, _| !dead.contains(id));
        self.frame.ready.retain(|f| !dead.contains(f));
        // Lát 2: đăng ký/kết quả của frame đã thu hồi cũng phải biến mất (lifecycle).
        self.frame.reactor.regs.retain(|r| !dead.contains(&r.frame));
        let dead_keys: Vec<u64> = self
            .frame
            .reactor
            .regs
            .iter()
            .filter(|r| dead.contains(&r.frame))
            .map(|r| r.key)
            .collect();
        for k in dead_keys {
            self.frame.reactor.results.remove(&k);
            self.frame.reactor.fired.remove(&k);
        }
    }

    /// **Chỉ test hook chẩn đoán** — đếm task/frame/waiter/ready + reactor đang sống.
    #[doc(hidden)]
    pub fn diag_snapshot(&self) -> DiagSnapshot {
        DiagSnapshot {
            live_tasks: self.tasks.len(),
            frames: self.frame.live_frames,
            waiters: self.frame.waiters.len(),
            ready: self.frame.ready.len(),
            regs: self.frame.reactor.regs.len(),
            timers: self
                .frame
                .reactor
                .events
                .iter()
                .filter(|e| e.deadline_ms.is_some())
                .count(),
        }
    }

    // -----------------------------------------------------------------------
    // VD-257 / ST-C — cancellation HỢP TÁC, NỘI BỘ (CHỈ TEST/DEBUG).
    //
    // Request chỉ được GHI NHẬN ở đây; task đang chạy KHÔNG BAO GIỜ bị cắt giữa chừng.
    // Việc dọn tài nguyên chỉ xảy ra khi scheduler đi qua một SAFE POINT mà driver đã gọi
    // sẵn (`frame_next`, `frame_fire`, `frame_reactor_fire`, `frame_wake_one`). Không API
    // ngôn ngữ, không CLI flag, không đổi runtime mặc định (`frame.enabled == false`).
    // -----------------------------------------------------------------------

    /// Ghi nhận một cancellation request cho task `id`. Idempotent — request trùng bị bỏ.
    /// Không dọn gì ngay: phải đợi safe point (`frame_cancel_process_for_test`).
    #[doc(hidden)]
    pub fn frame_cancel_request_for_test(&mut self, id: u32) {
        if self.frame.cancelled.contains(&id) || self.frame.cancel_req.contains(&id) {
            return;
        }
        self.frame.cancel_req.push(id);
        self.frame.log.push(format!("cancelreq:{}", id));
    }

    /// Xử lý mọi request đã ghi nhận tại safe point hiện tại. Trả số task vừa bị huỷ.
    #[doc(hidden)]
    pub fn frame_cancel_process_for_test(&mut self) -> usize {
        let reqs: Vec<u32> = std::mem::take(&mut self.frame.cancel_req);
        let mut n = 0;
        for id in reqs {
            if self.frame_cancel_one(id) {
                n += 1;
            }
        }
        n
    }

    /// Công việc huỷ cho MỘT task: gỡ waiter/reg/timer/external **đúng một lần**, không
    /// wake lại, không đóng language handle. `true` = task chuyển sang `Huy`.
    fn frame_cancel_one(&mut self, id: u32) -> bool {
        if self.frame.cancelled.contains(&id) {
            self.frame.log.push(format!("cancelagain:{}", id));
            return false;
        }
        let t = match self.tasks.iter().find(|t| t.id == id).cloned() {
            Some(t) => t,
            None => {
                self.frame.cancelled.insert(id);
                self.frame.log.push(format!("cancelgone:{}", id));
                return false;
            }
        };
        // Task đã xong trước khi request tới ⇒ huỷ vô hiệu: GIỮ NGUYÊN cache (`kq`).
        if t.trang_thai.get() == TrangThai::Xong {
            self.frame.cancelled.insert(id);
            self.frame.log.push(format!("cancelskip:{}:done", id));
            return false;
        }

        // Awaiter đang chờ chính task này (Wait::Task(id)) — sẽ được wake để đọc cache/error
        // (không bịa mã mới; xem BLOCKER). Thu thập TRƯỚC khi sửa waiters.
        let awaiters: Vec<Waiter> = self
            .frame
            .waiters
            .iter()
            .filter(|w| w.wait == Wait::Task(id))
            .cloned()
            .collect();

        // 1) Waiter của frame bị huỷ: gỡ đúng một lần.
        let wb = self.frame.waiters.len();
        self.frame.waiters.retain(|w| w.frame != id);
        if self.frame.waiters.len() < wb {
            self.frame.log.push(format!("cancelwait:{}", id));
        }

        // 2) Đăng ký reactor/timer của frame bị huỷ: deregister adapter đúng một lần,
        //    bỏ sổ token, results/fired (event cũ thành stale ⇒ không late wake).
        let regs: Vec<ReactorReg> = self
            .frame
            .reactor
            .regs
            .iter()
            .filter(|r| r.frame == id)
            .cloned()
            .collect();
        for r in &regs {
            if let Some(b) = self.frame.backend.clone() {
                let _ = b.borrow_mut().deregister(r.token, &r.handle, r.op);
                self.frame
                    .log
                    .push(format!("canceldereg:{}:{}", id, r.token));
            }
            self.frame.token_map.remove(&r.token);
            self.frame.reactor.results.remove(&r.key);
            self.frame.reactor.fired.remove(&r.key);
        }
        if !regs.is_empty() {
            self.frame.reactor.regs.retain(|r| r.frame != id);
            self.frame
                .log
                .push(format!("cancelregs:{}:{}", id, regs.len()));
        }

        // 3) Nguồn giả + ready: bỏ entry của task này ⇒ event nhắm vào nó thành stale.
        self.frame.external.retain(|tid, _| *tid != id);
        let rb = self.frame.ready.len();
        self.frame.ready.retain(|f| *f != id);
        if self.frame.ready.len() < rb {
            self.frame.log.push(format!("cancelready:{}", id));
        }

        // 4) Terminal. KHÔNG đóng handle mạng (language handle có thể thoát task / shared),
        //    KHÔNG ghi kq lỗi mới. Task chỉ ngừng chạy source và chờ thu hồi.
        t.trang_thai.set(TrangThai::Huy);
        self.frame.cancelled.insert(id);
        self.frame.log.push(format!("cancel:{}", id));

        // 5) Đánh thức awaiter để chúng đọc cache/error thay vì treo vô hạn.
        for w in awaiters {
            self.frame_wake_one(w);
        }
        true
    }

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
    pub(super) fn wake_reactor(&mut self, key: u64, gen: u64) {
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

    /// Scheduler rảnh: nếu còn event reactor, chọn deadline monotonic **gần nhất**
    /// (ghi log để so sánh hai engine), ngủ **một lần** tới deadline đó khi
    /// `timer_real_wait`, rồi fire event có `seq` nhỏ nhất. Không vòng lặp poll.
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
        // Deadline gần nhất được chọn MỘT lần cho cả đợt rảnh (ghi log tất định).
        if let Some((deadline, key)) = self
            .frame
            .reactor
            .events
            .iter()
            .filter_map(|e| e.deadline_ms.map(|d| (d, e.key)))
            .min_by_key(|(d, k)| (*d, *k))
        {
            self.frame
                .reactor
                .log
                .push(format!("timer:{}:{}", key, deadline));
            if self.frame.timer_real_wait {
                let epoch = self.frame.reactor.epoch.get_or_insert_with(Instant::now);
                let target = *epoch + Duration::from_millis(deadline);
                let now = Instant::now();
                if target > now {
                    std::thread::sleep(target - now);
                }
            }
        }
        // Fire theo `seq` cho tới khi có frame sẵn sàng hoặc hết trace. Event stale/
        // duplicate không wake ⇒ phải tiếp tụcfire event sau, không được dừng sớm.
        while !self.frame.reactor.events.is_empty() {
            let ev = self.frame.reactor.events.remove(0);
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
