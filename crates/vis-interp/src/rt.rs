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
use std::time::{Duration, Instant};

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
    /// `None` cho wait reactor (`read`/`accept`) — khi đó dùng [`Pending::wait`].
    pub task: Option<Rc<Task>>,
    pub span: Span,
    /// **ST-268/VD-252 (Lát 2)**: wait trực tiếp do reactor đăng ký (`read`/`accept`/
    /// timer), không cần một `Task` trung gian. `None` cho `await` thường (Lát 1).
    pub wait: Option<Wait>,
}

/// **Chỉ test**: cái một frame đang chờ.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wait {
    /// Chờ một task khác xong (khi task đó xong ⇒ đánh thức, không cần event giả).
    Task(u32),
    /// Chờ event giả có khóa `key`, thế hệ `gen`.
    Event { key: u64, gen: u64 },
    /// **ST-268/VD-252 (Lát 2)**: chờ reactor giả — `read`/`accept` có `key` đăng ký
    /// duy nhất và thế hệ `gen`. `key` là số thứ tự đăng ký tăng dần (không phải fd/tay),
    /// nên tay được mở lại không bao giờ bị event cũ đánh thức nhầm.
    Reactor { key: u64, gen: u64 },
}

impl Wait {
    /// Thế hệ của wait (dùng để dựng `Waiter` từ `Pending::wait` ở driver).
    pub fn gen_of(&self, fallback: u64) -> u64 {
        match self {
            Wait::Event { gen, .. } | Wait::Reactor { gen, .. } => *gen,
            Wait::Task(_) => fallback,
        }
    }
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
    /// **Lát 2**: số đăng ký reactor còn sống (`read`/`accept` đang treo).
    pub regs: usize,
    /// **Lát 2**: số timer monotonic còn sống (sự kiện có deadline).
    pub timers: usize,
}

// ===========================================================================
// ST-268 / VD-252 — LÁT 2: reactor giả **tất định** + timer monotonic (chỉ test).
//
// Reactor này KHÔNG phải sleep-poll và KHÔNG đọc socket thật: nó là nguồn sự kiện
// do trace cung cấp (`seq`, `key`, `gen`), được driver fire theo thứ tự `seq` ghi
// trong trace. `read`/`accept` khi chưa sẵn sàng ⇒ đăng ký wait + `Abort::Pending`;
// scheduler chạy frame khác; hết frame sẵn sàng ⇒ fire event tiếp theo (hoặc chờ
// tới deadline monotonic gần nhất). Mọi thao tác ghi log tất định để hai engine
// so sánh byte-with-byte. Không `unsafe`, không dependency, không CLI/API mới.
// ===========================================================================

/// **Chỉ test (Lát 2)**: phép toán mạng được reactor giả phủ (mở rộng dần).
/// `connect`/`write` vẫn đồng bộ ở đường test — xem report (có thể chặn scheduler).
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReactorOp {
    Read,
    Accept,
}

/// **Chỉ test (Lát 2)**: kết quả một sự kiện reactor giả.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub enum ReactorOutcome {
    /// Dữ liệu sẵn sàng — `Value::ok(...)` cho `read`, handle cho `accept`.
    Ready(Value),
    /// Thức giả (spurious): retry gặp `WouldBlock` ⇒ re-arm với `gen` mới (không spin).
    Spurious,
    /// EOF — `read` trả `""`, bỏ đăng ký đúng một lần.
    Eof,
    /// Lỗi dữ liệu — `read`/`accept` trả `Result.Err`.
    Error(String),
    /// Tay bị đóng khi đang chờ — hủy đăng ký + vô hiệu event cũ.
    Close,
}

/// **Chỉ test (Lát 2)**: một sự kiện trong trace reactor. `seq` khóa thứ tự fire;
/// `key` là khóa đăng ký (0 = khớp mọi đăng ký, dùng cho timer gần nhất); `gen` là
/// thế hệ đăng ký; `deadline_ms` (nếu có) là deadline monotonic tương đối kể từ
/// lúc bật frame — dùng để chọn timer gần nhất khi scheduler rảnh.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct ReactorEvent {
    pub seq: u64,
    pub key: u64,
    pub gen: u64,
    pub outcome: ReactorOutcome,
    pub deadline_ms: Option<u64>,
}

/// **Chỉ test (Lát 2)**: một đăng ký chờ đang sống trong reactor giả.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct ReactorReg {
    pub key: u64,
    pub handle: String,
    pub op: ReactorOp,
    pub frame: u32,
    pub gen: u64,
    /// **Lát 3**: token mờ do core cấp cho adapter — MỖI lần đăng ký là một token mới
    /// (kể cả re-arm) ⇒ phân biệt được generation, fd/tay dùng lại không wake nhầm.
    pub token: u64,
    /// **Lát 3**: deadline monotonic của riêng lần chờ này (`None` = không có timeout
    /// riêng; vẫn có trần chờ selector để không block vô hạn / không hot-spin).
    pub deadline: Option<Instant>,
}

/// **Chỉ test (Lát 2)**: toàn bộ trạng thái reactor giả.
#[doc(hidden)]
#[derive(Debug, Default)]
pub struct ReactorState {
    pub regs: Vec<ReactorReg>,
    pub next_key: u64,
    pub events: Vec<ReactorEvent>,
    /// `key → gen` đã fire — chặn duplicate wake ở thế hệ hiện hành.
    pub fired: HashMap<u64, u64>,
    /// Kết quả đã fire, chờ frame retry tiêu thụ (mỗi key đúng một lần).
    pub results: HashMap<u64, ReactorOutcome>,
    pub log: Vec<String>,
    /// Mốc monotonic dùng để tính deadline của timer trong test B1.
    pub epoch: Option<Instant>,
}

// ===========================================================================
// ST-271 / VD-254 — LÁT 3: adapter selector THẬT (chỉ đường frame test).
//
/// Trait adapter nằm ở crate lõi (zero-dep, cross-platform) để core/wasm32 không
/// bao giờ phụ thuộc crate selector. Crate host (Linux) cài đặt bằng `mio` — mọi
/// API ta gọi đều safe; `unsafe` FFI epoll nằm hoàn toàn trong dependency.
/// Không có implementation mặc định ⇒ không có fallback im lặng sang đồng bộ.
#[doc(hidden)]
pub trait FrameReactorBackend {
    /// Thử `read` không chặn. `max` đã bị chặn bởi `NET_READ_CAP` ở `stdlib`.
    fn try_read(&mut self, handle: &str, max: usize) -> ReactorIoOutcome;
    /// Thử `accept` không chặn — khi có kết nối, adapter cấp handle mới.
    fn try_accept(&mut self, handle: &str) -> ReactorIoOutcome;
    /// Đăng ký/kích hoạt lại interest READABLE cho `handle` với `token` MỚI.
    fn register(&mut self, token: u64, handle: &str, op: ReactorOp) -> Result<(), String>;
    /// Hủy đăng ký `token` (idempotent).
    fn deregister(&mut self, token: u64, handle: &str, op: ReactorOp) -> Result<(), String>;
    /// Chờ readiness tối đa `timeout`; trả sự kiện thật đã dịch sang token/core.
    fn wait(&mut self, timeout: Duration) -> Result<Vec<ReactorRawEvent>, String>;
    /// Đóng tay (`net.close`) — adapter đóng cả socket thật; stale event vô hiệu ở core.
    fn close(&mut self, handle: &str) -> Result<(), String>;
}

impl std::fmt::Debug for dyn FrameReactorBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FrameReactorBackend")
    }
}

/// Kết quả một lần thử I/O thật. `WouldBlock` ⇒ đăng ký readiness + `Pending`;
/// không phải "không có sự kiện" (select error là `Err`, lan rõ).
#[doc(hidden)]
#[derive(Debug)]
pub enum ReactorIoOutcome {
    /// `read`: byte nhận được (rỗng = EOF). Caller giải mã lossy/đóng gói `Result`.
    Data(Vec<u8>),
    /// `accept`: handle kết nối mới do adapter cấp.
    Accepted(String),
    /// Kernel chưa sẵn sàng — re-arm, không hot-spin, không mất wake.
    WouldBlock,
    /// Lỗi I/O/selector — lan thành `Result.Err` rõ, không nuốt.
    Err(String),
}

/// Một sự kiện readiness thật đã dịch sang token/core (không lộ fd hay `mio`).
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReactorRawEvent {
    pub token: u64,
    pub readable: bool,
    pub hup: bool,
    pub error: bool,
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
    /// **Lát 2**: reactor giả + timer monotonic (trace-driven).
    pub reactor: ReactorState,
    /// **Lát 2**: frame hiện hành — `stdlib` đọc để gán đăng ký cho đúng frame.
    pub current: u32,
    /// **Lát 2**: nếu `true`, khi rảnh scheduler ngủ **một lần** tới deadline monotonic
    /// gần nhất (dùng cho đo B1); `false` (mặc định) ⇒ fire ngay, hoàn toàn tất định.
    pub timer_real_wait: bool,
    /// **Lát 3 (chỉ test)**: adapter selector thật. `None` (mặc định) ⇒ đường reactor giả
    /// Lát 2 chạy nguyên văn; `Some` ⇒ `read`/`accept` lấy readiness từ kernel.
    pub backend: Option<Rc<RefCell<dyn FrameReactorBackend>>>,
    /// **Lát 3**: token → `(key, gen)` hiện hành; token cũ (fd/tay reuse, re-arm) bị bỏ ⇒ stale.
    pub token_map: HashMap<u64, (u64, u64)>,
    /// **Lát 3**: bộ đếm token tăng dần — không bao giờ tái sử dụng.
    pub next_token: u64,
    /// **Lát 3**: lỗi selector được lưu lại rồi driver biến thành trap rõ (không nuốt).
    pub selector_error: Option<String>,
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
        // Lát 2: fire nốt event reactor còn lại (duplicate/stale vẫn được log) để trace
        // đầy đủ giữa hai engine — không cắt ngắn.
        let evs = std::mem::take(&mut self.frame.reactor.events);
        for ev in evs {
            self.frame_reactor_fire(ev);
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
    fn wake_reactor(&mut self, key: u64, gen: u64) {
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
            self.frame_real_poll(&b, op, handle, max, timeout_ms, span)
        } else {
            let _ = max;
            self.frame_reactor_poll(op, handle, timeout_ms, span)
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
    fn frame_real_poll(
        &mut self,
        b: &Rc<RefCell<dyn FrameReactorBackend>>,
        op: ReactorOp,
        handle: &str,
        max: usize,
        timeout_ms: i64,
        span: Span,
    ) -> R<Value> {
        let outcome = match op {
            ReactorOp::Read => b.borrow_mut().try_read(handle, max),
            ReactorOp::Accept => b.borrow_mut().try_accept(handle),
        };
        match outcome {
            ReactorIoOutcome::Data(bytes) => {
                self.frame_real_done(handle, op);
                self.frame.reactor.log.push(format!("rdone:{}", handle));
                return Ok(ok_str(String::from_utf8_lossy(&bytes)));
            }
            ReactorIoOutcome::Accepted(new_handle) => {
                self.frame_real_done(handle, op);
                self.frame.reactor.log.push(format!("raccepted:{}", handle));
                return Ok(ok_str(new_handle));
            }
            ReactorIoOutcome::Err(msg) => {
                self.frame_real_done(handle, op);
                self.frame
                    .reactor
                    .log
                    .push(format!("rerr:{}:{}", handle, msg));
                return Ok(err_str(msg));
            }
            ReactorIoOutcome::WouldBlock => {}
        }

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
                    return Ok(err_str(format!(
                        "quá thời gian chờ dữ liệu từ `{handle}` sau {timeout_ms} ms"
                    )));
                }
                // Thức giả (wake nhưng vẫn WouldBlock): token MỚI + gen MỚI ⇒ wake cũ stale.
                let _ = b.borrow_mut().deregister(r.token, &r.handle, r.op);
                self.frame.token_map.remove(&r.token);
                let gen = r.gen + 1;
                let token = self.mint_token();
                if let Err(e) = b.borrow_mut().register(token, handle, op) {
                    self.frame_real_done(handle, op);
                    return Ok(err_str(format!(
                        "lỗi đăng ký readiness cho `{handle}`: {e}"
                    )));
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
                    return Ok(err_str(format!(
                        "lỗi đăng ký readiness cho `{handle}`: {e}"
                    )));
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
    fn frame_idle_real(&mut self) -> bool {
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

/// **Chỉ test (Lát 2)**: dựng `Result[str,str]` khớp đường `std.net` mặc định
/// (`hop_nhat_fs`: `Ok` → `Value::ok`, `Err` → `Value::err(Error)` dữ liệu).
fn ok_str(s: impl Into<String>) -> Value {
    Value::ok(Value::Str(s.into()))
}
fn err_str(s: impl Into<String>) -> Value {
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
