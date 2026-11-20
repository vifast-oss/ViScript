// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-60 (VD-90/VD-95)** — runtime **tác vụ async**: `Task` + bộ lập lịch **một luồng, tất định**.
//!
//! Mô hình (chốt VD-90): gọi `async fn` **không** chạy thân — trả `Task[T]`; `await` chạy tác vụ
//! **tới hoàn tất** (run-to-completion, **không** nhường giữa thân) rồi trả cache; cuối chương
//! trình **join** mọi tác vụ theo **thứ tự tạo**. Một hiện thực cho **cả** interp lẫn VM (VD-95):
//! `VBC1` **không** thêm tag — `$tac_vu`/`$spawn`/`$await` là builtin nội bộ, `Int` của chúng là
//! **handle của host** (interp: `sym`; VM: chỉ số hàm bytecode), không bao giờ lộ ra chương trình.
//!
//! Nợ ghi rõ: **không** nhường giữa thân ⇒ chưa có song song thật; **không** GC (chu trình `Rc`);
//! `await` lồng quá [`MAX_TASK_NEST`] ⇒ `VS0501`.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use vis_syntax::Span;

use crate::builtin::Host;
use crate::interp::{Abort, Trap, R, RT};
use crate::value::Value;

/// Chặn **lồng** `await` (mỗi tầng là một khung gọi thật) — 256 như chốt VD-95 (7).
pub const MAX_TASK_NEST: usize = 256;

/// Trạng thái một tác vụ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrangThai {
    /// Tạo rồi, thân **chưa** chạy.
    Moi,
    /// Đang chạy (bắt `await` chính nó ⇒ `VS0501`).
    DangChay,
    /// Xong (kết quả **cache**, `await` lần hai trả lại).
    Xong,
    /// **Chỉ đường test/frame**: đang treo trên một event giả (`ST-267/VD-251`).
    /// Runtime mặc định không bao giờ đặt trạng thái này.
    Pending,
}

/// Một tác vụ `async fn`.
#[derive(Debug)]
pub struct Task {
    /// Số thứ tự **tạo** (1, 2, …) — cũng là thứ tự join cuối.
    pub id: u32,
    /// **Handle của host**: interp = `sym` của `HFn`; VM = chỉ số hàm bytecode.
    pub than: u32,
    pub args: Vec<Value>,
    /// Tên hàm — chỉ dùng cho thông điệp `VS0506`.
    pub ten: String,
    /// **Span lúc tạo** — lỗi của thân báo ở đây (một quy tắc cho `await` lẫn join).
    pub span: Span,
    pub trang_thai: Cell<TrangThai>,
    /// `Ok`/`Err` **đã bọc** (`VS0506` nếu thân lỗi) — cache cho `await` lần hai.
    pub kq: RefCell<Option<Result<Value, Trap>>>,
}

/// **ST-61 (VD-96)**: một kênh đệm **hữu hạn** (FIFO). `cap = 0` ⇒ không đệm (gửi phải có
/// người nhận **đã chạy** — mô hình run-to-completion, nợ như async).
#[derive(Debug)]
pub struct Chan {
    /// Số thứ tự tạo (1, 2, …) — chỉ để chẩn đoán/tất định, chương trình **không** thấy.
    pub id: u32,
    pub cap: usize,
    pub buf: VecDeque<Value>,
    pub closed: bool,
}

impl Chan {
    pub fn day(&self) -> bool {
        self.buf.len() >= self.cap
    }
    pub fn rong(&self) -> bool {
        self.buf.is_empty()
    }
}

// ===========================================================================
// ST-267 / VD-251 — LÁT 1: frame scheduler cho đường test nội bộ.
//
// Mọi kiểu/field/hàm ở đây là **test/debug-only** (`#[doc(hidden)]`, không CLI,
// không ngôn ngữ). Runtime mặc định giữ nguyên: `frame.enabled == false` ⇒
// mọi nhánh cũ trong tệp này chạy y hệt trước. Không reactor, không socket,
// không opcode/bytecode/VERSION/cache-key mới, không `unsafe`, không dependency.
// ===========================================================================

/// **Chỉ test**: tín hiệu "frame hiện tại phải treo ở `await` này".
/// Lan qua `Abort::Pending` — engine (driver) là nơi lưu frame + đăng ký wait.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct Pending {
    /// Tác vụ đang được `await` (kết quả/event của nó sẽ đánh thức frame hiện tại).
    pub task: Rc<Task>,
    pub span: Span,
}

/// **Chỉ test**: cái một frame đang chờ.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wait {
    /// Chờ một task khác xong (khi task đó xong ⇒ đánh thức, không cần event giả).
    Task(u32),
    /// Chờ event giả có khóa `key`, thế hệ `gen`.
    Event { key: u64, gen: u64 },
}

/// **Chỉ test**: một sự kiện giả trong trace. `seq` là khóa thứ tự **được ghi trong
/// trace** — tie-break tất định, không dùng hash-map/memory address. `gen` là thế hệ
/// đăng ký; event có `gen` lệch thế hệ hiện hành bị bỏ (stale wake).
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct FakeEvent {
    pub seq: u64,
    pub key: u64,
    pub gen: u64,
    pub value: Value,
}

/// **Chỉ test**: một đăng ký chờ (waiter). Thứ tự trong `Vec` là thứ tự đăng ký ⇒
/// đánh thức theo FIFO, không hash-map.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Waiter {
    pub frame: u32, // 0 = main, 1.. = id task
    pub wait: Wait,
    pub gen: u64,
}

/// **Chỉ test**: snapshot chẩn đoán (không fd/timer thật).
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DiagSnapshot {
    pub live_tasks: usize,
    pub frames: usize,
    pub waiters: usize,
    pub ready: usize,
}

/// **Chỉ test**: toàn bộ trạng thái frame scheduler. Nằm trong `Scheduler.frame`;
/// khi `enabled == false` (mặc định) không nhánh nào trong runtime đọc vào đây.
#[doc(hidden)]
#[derive(Debug, Default)]
pub struct FrameTest {
    pub enabled: bool,
    /// Thế hệ hiện hành — tăng khi một đăng ký được thay/re-arm.
    pub gen: u64,
    /// `(tên hàm async, khóa event)` — task spawn từ hàm này là **nguồn giả**:
    /// thân Không chạy, kết quả do `events` cung cấp.
    pub fake_sources: Vec<(String, u64)>,
    /// Trace sự kiện giả, đã sắp theo `seq` (driver fire theo thứ tự `seq`).
    pub events: std::collections::VecDeque<FakeEvent>,
    /// Nhật ký hành động scheduler, **tất định** (dùng để so sánh hai engine).
    pub log: Vec<String>,
    /// task id → `(khóa event, thế hệ đăng ký hiện hành)`.
    pub external: HashMap<u32, (u64, u64)>,
    /// Các wait đang hoạt động, theo thứ tự đăng ký.
    pub waiters: Vec<Waiter>,
    /// Hàng đợi sẵn sàng **của đường frame** (FIFO): frame được đánh thức nối vào đuôi.
    pub ready: VecDeque<u32>,
    /// Frame đã được join/await xong nghĩa vụ (để thu hồi).
    pub joined: std::collections::HashSet<u32>,
    /// Số frame đang giữ trạng thái treo (do engine cập nhật).
    pub live_frames: usize,
    /// Sự kiện đã fire (khóa → thế hệ cuối) — chặn fire trùng.
    pub fired: HashMap<u64, u64>,
}

/// Bộ lập lịch **một luồng, tất định**: danh sách tác vụ **theo thứ tự tạo** + hàng đợi **FIFO**
/// cho `spawn` (hàng phẳng `Vec` ⇒ 10.000 tác vụ **không** đệ quy, không tràn).
#[derive(Debug, Default)]
pub struct Scheduler {
    pub tasks: Vec<Rc<Task>>,
    pub ready: VecDeque<usize>,
    /// Số tác vụ đã tạo (đếm để đặt `id`).
    pub tiep: u32,
    /// Độ sâu `await` đang mở (chặn lồng).
    pub sau: usize,
    /// **ST-61**: các kênh đã tạo — đếm `id` (và giữ sống để chẩn đoán; chương trình cầm
    /// `Rc<RefCell<Chan>>` riêng).
    pub kenh: Vec<Rc<RefCell<Chan>>>,
    /// **ST-267/VD-251 (chỉ test)**: trạng thái frame scheduler. Mặc định rỗng/`false`.
    #[doc(hidden)]
    pub frame: FrameTest,
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Tạo tác vụ (chưa chạy) — trả `Task` cho chương trình.
    pub fn tao(&mut self, than: u32, args: Vec<Value>, ten: String, span: Span) -> Rc<Task> {
        self.tiep += 1;
        let id = self.tiep;
        let t = Rc::new(Task {
            id,
            than,
            args,
            ten,
            span,
            trang_thai: Cell::new(TrangThai::Moi),
            kq: RefCell::new(None),
        });
        self.tasks.push(Rc::clone(&t));
        if self.frame.enabled {
            // `await f()` cũng tạo task qua `tao` — nhận diện nguồn giả như `spawn`.
            let src = self
                .frame
                .fake_sources
                .iter()
                .find(|(n, _)| *n == t.ten)
                .map(|(_, k)| *k);
            if let Some(key) = src {
                let gen = self.frame.gen;
                self.frame.external.insert(id, (key, gen));
                self.frame
                    .log
                    .push(format!("external:{}:{}:{}", id, key, gen));
            }
        }
        t
    }

    /// Tạo + đưa vào **hàng đợi sẵn sàng** (`spawn`: chạy ở `await`/join đầu tiên gặp).
    pub fn spawn(&mut self, than: u32, args: Vec<Value>, ten: String, span: Span) -> Rc<Task> {
        let t = self.tao(than, args, ten, span);
        let idx = self.tasks.len() - 1;
        if self.frame.enabled {
            // Đường frame: nhận diện nguồn giả theo **tên hàm** (không dùng địa chỉ).
            let src = self
                .frame
                .fake_sources
                .iter()
                .find(|(n, _)| *n == t.ten)
                .map(|(_, k)| *k);
            match src {
                Some(key) => {
                    let gen = self.frame.gen;
                    self.frame.external.insert(t.id, (key, gen));
                    self.frame
                        .log
                        .push(format!("external:{}:{}:{}", t.id, key, gen));
                }
                None => self.frame.ready.push_back(t.id),
            }
        } else {
            self.ready.push_back(idx);
        }
        t
    }
}

// ===========================================================================
// ST-267 / VD-251 — API điều khiển frame scheduler (CHỈ TEST/DEBUG).
// ===========================================================================
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
        if let Some(f) = self.frame.ready.pop_front() {
            return Some(f);
        }
        // Không còn frame sẵn sàng ⇒ fire sự kiện giả tiếp theo (theo `seq`).
        if let Some(ev) = self.frame.events.pop_front() {
            self.frame_fire(ev);
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
    }

    /// **Chỉ test**: main kết thúc — join mọi task chưa xong (không auto-cancel), rồi
    /// thu hồi các task đã xong + không còn handle sống + đã có nghĩa vụ join.
    #[doc(hidden)]
    pub fn frame_final_join(&mut self) {
        let ids: Vec<u32> = self.tasks.iter().map(|t| t.id).collect();
        for id in ids {
            self.frame.joined.insert(id);
        }
        self.frame_reclaim();
    }

    /// **Chỉ test**: thu hồi task thỏa 3 điều kiện ownership: xong, có nghĩa vụ join,
    /// và không còn handle/reference sống (`Rc::strong_count == 1` chỉ còn scheduler).
    /// ready/waiter/registration cũng bị dọn theo — không giữ task chết quá ý đồ.
    #[doc(hidden)]
    pub fn frame_reclaim(&mut self) {
        let dead: Vec<u32> = self
            .tasks
            .iter()
            .filter(|t| {
                t.trang_thai.get() == TrangThai::Xong
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
    }

    /// **Chỉ test hook chẩn đoán** — đếm task/frame/waiter/ready đang sống.
    #[doc(hidden)]
    pub fn diag_snapshot(&self) -> DiagSnapshot {
        DiagSnapshot {
            live_tasks: self.tasks.len(),
            frames: self.frame.live_frames,
            waiters: self.frame.waiters.len(),
            ready: self.frame.ready.len(),
        }
    }
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
    // **ST-94 (VD-112)**: capacity 0 (rendezvous) **không thể** trên scheduler một luồng.
    // tyck **không** kiểm `cap` (VD-154 (c)): `kiem_chan_new` chỉ đòi `n` là `int`; `VS0505` là
    // **runtime** trap (exit 1) kể cả với literal `chan[T](0)` — đường **động** (`let n = 0`) cũng
    // **từ chối** tại đây, không treo. Bảng mã: `VS0505` UNCLASSIFIED ở `interp.rs`/VM (vd. test/kenh_cap0.rs).
    if cap == 0 {
        return Err(Abort::Trap(Trap::new(
            crate::interp::DEADLOCK,
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
        await_task(host, &t, span)?;
        return Ok(true);
    }
}

/// Chạy một tác vụ **tới hoàn tất** (hoặc trả cache) — dùng chung cho `await` và join.
pub fn await_task(host: &mut dyn Host, t: &Rc<Task>, span: Span) -> R<Value> {
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
    }
    // **ST-267/VD-251 (chỉ test)**: không chạy thân tới hoàn tất. Báo driver treo frame,
    // driver lưu frame thật (Th/IFrame) rồi đăng ký wait + chạy frame khác. Đường cache
    // (`Xong`) ở trên vẫn đi nguyên văn ⇒ `await` lần hai trả cùng result/error.
    if host.sched().borrow().frame.enabled {
        return Err(Abort::Pending(Pending {
            task: Rc::clone(t),
            span,
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
    let kq = host.goi_than(t.than, t.args.clone(), t.span);
    t.trang_thai.set(TrangThai::Xong);
    let kq: Result<Value, Trap> = match kq {
        Ok(v) => Ok(v),
        // `return` trong thân tác vụ: `Abort::Ret` đã được host bắt; phòng thủ thì coi là kết quả.
        Err(Abort::Ret(v)) => Ok(v),
        Err(Abort::Trap(goc)) => Err(wrap_task_error(&t.ten, goc, t.span)),
        // Đường frame không đi qua đây (đã trả Pending ở trên); phòng thủ.
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
fn wrap_task_error(ten: &str, goc: Trap, span: Span) -> Trap {
    Trap::new(
        crate::interp::TASK_ERR,
        format!("tác vụ `{ten}` lỗi: {}: {}", goc.code.short, goc.msg),
        span,
    )
}

/// **Join** mọi tác vụ theo **thứ tự tạo** (chỉ gọi khi main thành công — VD-95 (6)).
pub fn join_all(host: &mut dyn Host) -> R<()> {
    let ds: Vec<Rc<Task>> = host.sched().borrow().tasks.clone();
    for t in ds {
        let span = t.span;
        await_task(host, &t, span)?;
    }
    Ok(())
}
