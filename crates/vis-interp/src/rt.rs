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

pub(crate) mod chan;
/// **VS3-3b/W5 (P1)** — chế độ evaluator W=1 (`Off`/`Local`); P2 thêm pool.
pub(crate) mod exec;
/// **ST-276 (VD-256)**: định nghĩa sự kiện ngân sách **dùng chung** với `vis-bytecode`.
/// Khai báo ở đây (thay vì `frame.rs`) để giữ nguyên phạm vi sửa của ST-278 — file nguồn nằm
/// trong `frame/**`, nội dung là `include!` của `vis-bytecode/src/fee_shared.rs`.
#[path = "frame/fee.rs"]
pub mod fee;
mod sched;

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
    /// **VD-257/C (chỉ đường test/frame)**: bị huỷ tại một safe point — đã dọn mọi
    /// đăng ký/waiter/timer, không còn được đánh thức, thân **không** chạy tiếp.
    /// Không phải `Xong`: `kq` có thể vẫn `None` (không bịa mã lỗi huỷ mới). Runtime
    /// mặc định không bao giờ đặt trạng thái này.
    Huy,
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
    /// **VS3-2a (chỉ frame test)** — sender `cap(0)` đang chờ, FIFO. `Taken` giữ record tới
    /// khi sender resume để phân biệt "đã ghép xong" với "chưa gửi"; `Closed` để sender
    /// resume trả `VS0504`.
    pub(crate) send_waiters: VecDeque<ChanSend>,
    /// **VS3-2a (chỉ frame test)** — frame receiver `cap(0)` đang chờ, FIFO.
    pub(crate) recv_waiters: VecDeque<u32>,
    /// **VS3-2a (chỉ frame test)** — giá trị đã ghép cho một receiver cụ thể (chờ resume).
    pub(crate) recv_values: HashMap<u32, Value>,
}

/// **VS3-2a**: trạng thái một sender `cap(0)` đang chờ.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChanSendState {
    Pending,
    Taken,
    Closed,
}

/// **VS3-2a**: một sender `cap(0)` đang chờ (frame + giá trị + trạng thái).
#[doc(hidden)]
#[derive(Debug, Clone)]
pub(crate) struct ChanSend {
    pub frame: u32,
    pub value: Value,
    pub state: ChanSendState,
}

impl Chan {
    pub fn day(&self) -> bool {
        self.buf.len() >= self.cap
    }
    pub fn rong(&self) -> bool {
        self.buf.is_empty()
    }

    /// **VS3-2a** — gỡ mọi trạng thái chờ kênh của frame bị huỷ. Trả `true` nếu có thay đổi.
    /// Giá trị đã ghép cho receiver bị huỷ bị bỏ (không refund); sender đã `Taken` coi như
    /// đã gửi xong (không gửi lại). Không đóng kênh, không đụng handle.
    pub(crate) fn huy_frame(&mut self, frame: u32) -> bool {
        let mut changed = false;
        let before = self.send_waiters.len();
        self.send_waiters.retain(|s| s.frame != frame);
        changed |= self.send_waiters.len() != before;
        let before = self.recv_waiters.len();
        self.recv_waiters.retain(|f| *f != frame);
        changed |= self.recv_waiters.len() != before;
        changed |= self.recv_values.remove(&frame).is_some();
        changed
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
    /// **VS3-2a**: chờ rendezvous `chan(0)` — `id` là id kênh, `send` là chiều chờ.
    /// Chỉ đường frame test phát sinh; runtime mặc định vẫn từ chối `chan(0)`.
    Chan { id: u32, send: bool },
}

impl Wait {
    /// Thế hệ của wait (dùng để dựng `Waiter` từ `Pending::wait` ở driver).
    pub fn gen_of(&self, fallback: u64) -> u64 {
        match self {
            Wait::Event { gen, .. } | Wait::Reactor { gen, .. } => *gen,
            Wait::Task(_) | Wait::Chan { .. } => fallback,
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

/// **VD-260 (chỉ test)**: số frame sẵn sàng liên tiếp tối đa trước khi scheduler buộc giao
/// **một** event kế tiếp (fake/reactor, đúng `seq`). Bộ đếm tất định (không phải thời gian)
/// nên trace hai engine vẫn so sánh được; timer vẫn không fire sớm hơn deadline của nó.
#[doc(hidden)]
pub const FRAME_READY_BUDGET: u32 = 8;

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
    /// **VS3-W2** — chờ cả READABLE lẫn WRITABLE cho một bước HTTP/2 (session tự quyết).
    ReadWrite,
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
    /// **VS3-W2** — lái handshake h2 một bước; `Ok(true)` = sẵn sàng gửi request.
    /// Backend không hỗ trợ h2 giữ mặc định lỗi rõ (test scripted không phải sửa).
    fn h2_ready(&mut self, _handle: &str) -> Result<bool, String> {
        Err("backend không hỗ trợ HTTP/2 async".to_string())
    }
    /// **VS3-W2** — gửi request h2 trên connection `handle`; trả id stream nội bộ.
    fn h2_send(
        &mut self,
        _handle: &str,
        _method: &str,
        _target: &str,
        _headers: &[(String, String)],
    ) -> Result<u64, String> {
        Err("backend không hỗ trợ HTTP/2 async".to_string())
    }
    /// **VS3-W2** — poll một bước session + kiểm stream `id`.
    fn h2_poll(&mut self, _handle: &str, _id: u64) -> ReactorH2Outcome {
        ReactorH2Outcome::Err("backend không hỗ trợ HTTP/2 async".to_string())
    }
    /// **VS3-W2** — huỷ riêng stream `id` (timeout/cancel) mà không đóng session.
    fn h2_cancel(&mut self, _handle: &str, _id: u64) {}
    /// **ST-287 (VD-273)** — ghi nhận `handle` do **HTTP async** của frame `frame_id` đang giữ.
    /// Chỉ dùng cho tài nguyên nội bộ của `http.get/head` (không phải language handle); backend
    /// mặc định bỏ qua, backend thật ghi sổ để [`Self::cancel_task_handles`] đóng đúng tay.
    fn note_task_handle(&mut self, _frame_id: u32, _handle: &str) {}
    /// **ST-287 (VD-273)** — ghi nhận stream h2 `id` trên `handle` của frame `frame_id` để huỷ
    /// gửi `RST_STREAM` đúng stream (session có thể dùng chung với frame khác).
    fn note_task_h2(&mut self, _frame_id: u32, _handle: &str, _id: u64) {}
    /// **ST-287 (VD-273)** — **safe point** của huỷ task: đóng mọi tay HTTP async còn treo của
    /// frame `frame_id` (`H2Async::dong()` + `RST_STREAM` cho h2, đóng socket cho h1).
    /// Backend mặc định no-op để adapter scripted/test cũ không phải sửa.
    fn cancel_task_handles(&mut self, _frame_id: u32) {}
}

/// **VS3-W2** — response h2 có cấu trúc cho core (không nhồi lại thành HTTP/1.1).
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct ReactorH2Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// **VS3-W2** — kết quả một bước `h2_poll`.
#[doc(hidden)]
#[derive(Debug)]
pub enum ReactorH2Outcome {
    /// Cần readiness READABLE|WRITABLE (scheduler đăng ký rồi trả `Pending`).
    WouldBlock,
    Response(ReactorH2Response),
    Err(String),
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
    /// **VD-260**: số frame sẵn sàng liên tiếp đã chạy mà **chưa** giao event nào. Chạm
    /// [`FRAME_READY_BUDGET`] ⇒ `frame_next` giao event kế tiếp (fake/reactor) trước frame
    /// ready — chặn đói I/O/timer sau một hàng ready dài. Reset khi một event thật sự fire.
    pub ready_streak: u32,
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
    /// **ST-278 (VD-256)**: lỗi task phát hiện lúc **final join** (theo hợp đồng `VS0506`,
    /// đã bọc sẵn trong `Task.kq`). Không đổi chính sách hủy các task còn lại.
    pub final_join_error: Option<Trap>,
    /// **VD-257/C**: cancellation request đã ghi nhận nhưng CHƯA xử lý (cần safe point).
    pub cancel_req: Vec<u32>,
    /// **VD-257/C**: task id đã bị huỷ — mọi tài nguyên chờ đã gỡ đúng một lần.
    pub cancelled: std::collections::HashSet<u32>,
    /// **VD-257/C**: `(seq sự kiện, task id)` — khi event có `seq` fire, ghi nhận huỷ `id`
    /// TRƯỚC khi wake, để task đang treo không bao giờ chạy lại source.
    pub cancel_on_seq: Vec<(u64, u32)>,
    /// **ST-287**: `(số turn, task id)` — huỷ tại safe point sau `turn` lượt `frame_next`.
    pub cancel_on_turn: Vec<(u64, u32)>,
    /// **ST-287**: số turn đã qua (chỉ tăng khi `cancel_on_turn` không rỗng).
    pub turn: u64,
    /// **ST-287 (P0)**: `(tiền tố handle, task id, số byte còn lại)` — huỷ sau khi
    /// `try_read` thật của frame đã trả đủ byte (bám handle `C…`/`H…`, không đếm turn).
    pub cancel_on_bytes: Vec<(String, u32, usize)>,
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
    /// **VS3-3b/W5 (P1)**: chế độ evaluator (`Off` mặc định, `Local` = W=1 caller).
    pub(crate) exec: exec::ExecState,
}

// **VD-257/C (chỉ test)**: trigger huỷ theo `seq` event, do integration test ghi TRƯỚC
// khi gọi `run_frame_for_test`. `frame_enable_for_test` drains vào `FrameTest.cancel_on_seq`
// nên cả hai engine (interp + VM) cùng thấy một trigger — không CLI flag, không API ngôn ngữ.
thread_local! {
    static FRAME_CANCEL_TEST: RefCell<Vec<(u64, u32)>> = const { RefCell::new(Vec::new()) };
    /// **ST-287** — trigger huỷ theo **số turn scheduler** (đếm `frame_next`), drain vào
    /// `FrameTest.cancel_on_turn` ở `frame_enable_for_test` như trigger theo `seq`.
    static FRAME_CANCEL_TURNS: RefCell<Vec<(u64, u32)>> = const { RefCell::new(Vec::new()) };
    /// **ST-287 (P0)** — trigger huỷ theo **byte đã đọc** trên handle, drain như trên.
    static FRAME_CANCEL_BYTES: RefCell<Vec<(String, u32, usize)>> =
        const { RefCell::new(Vec::new()) };
}

/// **VD-257/C (chỉ test)**: đăng ký "khi event `seq` fire thì ghi nhận huỷ task `id`".
#[doc(hidden)]
pub fn frame_cancel_on_seq_for_test(seq: u64, id: u32) {
    FRAME_CANCEL_TEST.with(|x| x.borrow_mut().push((seq, id)));
}

/// **ST-287 (chỉ test)** — ghi nhận huỷ task `id` tại safe point sau `turns` lượt
/// `frame_next`. Dùng để huỷ **giữa thân HTTP chậm** (sau khi request đã đi và client đã
/// đọc phần thân đầu), không phụ thuộc thứ tự event giả.
#[doc(hidden)]
pub fn frame_cancel_after_turns_for_test(turns: u64, id: u32) {
    FRAME_CANCEL_TURNS.with(|x| x.borrow_mut().push((turns, id)));
}

/// **ST-287 (P0, chỉ test)** — huỷ task `id` sau khi `try_read` thật đã trả đủ `bytes` byte
/// trên handle có tiền tố `handle_prefix` (vd `C`/`H`). Bám **dữ liệu đã đọc**, không đếm turn.
#[doc(hidden)]
pub fn frame_cancel_after_bytes_for_test(handle_prefix: &str, bytes: usize, id: u32) {
    FRAME_CANCEL_BYTES.with(|x| {
        x.borrow_mut().push((handle_prefix.to_string(), id, bytes));
    });
}

/// **VD-257/C (chỉ test)**: xoá mọi trigger huỷ còn tồn trong thread-local (dọn giữa các ca).
#[doc(hidden)]
pub fn frame_cancel_clear_triggers_for_test() {
    FRAME_CANCEL_TEST.with(|x| x.borrow_mut().clear());
    FRAME_CANCEL_TURNS.with(|x| x.borrow_mut().clear());
    FRAME_CANCEL_BYTES.with(|x| x.borrow_mut().clear());
}

// **VS3-1d (chỉ test)** — hạ trần turn scheduler để kiểm chứng guard mà không chạy 200k turn.
// Runtime production dùng mặc định 200_000; hook này không có CLI/API ngôn ngữ.
thread_local! {
    static FRAME_MAX_TURNS_TEST: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

/// **Chỉ test**: đặt trần turn scheduler (`None` = mặc định production 200_000).
#[doc(hidden)]
pub fn frame_set_max_turns_for_test(n: Option<usize>) {
    FRAME_MAX_TURNS_TEST.with(|x| x.set(n));
}

/// **Chỉ test**: trần turn scheduler đang đặt, hoặc `None`.
#[doc(hidden)]
pub fn frame_max_turns_for_test() -> Option<usize> {
    FRAME_MAX_TURNS_TEST.with(|x| x.get())
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
// ST-267 / VD-251 — API điều khiển frame scheduler (CHỈ TEST/DEBUG) nằm trong `rt/sched.rs`.
// ===========================================================================

/// Kết quả nội bộ của một lần thử reactor thật — cho phép `net.read` giữ chuỗi lossy
/// còn HTTP async nhận **byte thô** (không cắt UTF-8/không đổi Content-Length).
enum RealPoll {
    Value(Value),
    Bytes(Vec<u8>),
    Loi(String),
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
    // **ST-94 (VD-112)**: capacity 0 (rendezvous) **không thể** trên scheduler một luồng
    // mặc định. **VS3-2a**: riêng đường frame test đã có wait-set `chan(0)` (send chờ recv và
    // ngược lại) ⇒ cho phép tạo; runtime mặc định (`frame.enabled == false`) vẫn từ chối.
    // tyck **không** kiểm `cap` (VD-154 (c)): `kiem_chan_new` chỉ đòi `n` là `int`.
    if cap == 0 && !host.sched().borrow().frame.enabled {
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
        await_task(host, &t, span)?;
        return Ok(true);
    }
}

/// Chạy một tác vụ **tới hoàn tất** (hoặc trả cache) — dùng chung cho `await` và join.
/// `Local` (W=1) chỉ dispatch sang runner caller; mặc định `Off` giữ nguyên đường cũ.
pub fn await_task(host: &mut dyn Host, t: &Rc<Task>, span: Span) -> R<Value> {
    let mode = host.sched().borrow().exec.mode;
    match mode {
        exec::WorkerMode::Local => exec::await_local(host, t, span),
        exec::WorkerMode::Parallel(_) => exec::await_parallel(host, t, span),
        exec::WorkerMode::Off => await_task_off(host, t, span),
    }
}

/// Thân `await` của đường một luồng (Off) — P1 tách tên, **không đổi ngữ nghĩa**.
pub(crate) fn await_task_off(host: &mut dyn Host, t: &Rc<Task>, span: Span) -> R<Value> {
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
    let kq = crate::interp::worker::chay_than_local(host, t.than, t.args.clone(), &t.ten, t.span);
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
pub(crate) fn wrap_task_error(ten: &str, goc: Trap, span: Span) -> Trap {
    Trap::new(
        crate::interp::TASK_ERR,
        format!("tác vụ `{ten}` lỗi: {}: {}", goc.code.short, goc.msg),
        span,
    )
}

/// **Join** mọi tác vụ theo **thứ tự tạo** (chỉ gọi khi main thành công — VD-95 (6)).
/// `Local` (W=1) dispatch sang `join_local`; mặc định `Off` giữ nguyên đường cũ.
pub fn join_all(host: &mut dyn Host) -> R<()> {
    let mode = host.sched().borrow().exec.mode;
    match mode {
        exec::WorkerMode::Local => exec::join_local(host),
        exec::WorkerMode::Parallel(_) => exec::join_parallel(host),
        exec::WorkerMode::Off => join_all_off(host),
    }
}

/// Thân join của đường một luồng (Off) — P1 tách tên, **không đổi ngữ nghĩa**.
pub(crate) fn join_all_off(host: &mut dyn Host) -> R<()> {
    let ds: Vec<Rc<Task>> = host.sched().borrow().tasks.clone();
    for t in ds {
        let span = t.span;
        await_task(host, &t, span)?;
    }
    Ok(())
}
