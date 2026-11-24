// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310a (VD-285)** — kiểu/trạng thái frame scheduler (test-only), tách khỏi `rt.rs`.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::time::Instant;

use vis_core::trap::Trap;

use super::{
    select, FrameReactorBackend, ReactorOp, Wait, FRAME_CANCEL_BYTES, FRAME_CANCEL_TEST,
    FRAME_CANCEL_TURNS,
};

pub(crate) use vis_core::frame::{DiagSnapshot, FakeEvent, ReactorEvent, ReactorOutcome};

// ===========================================================================
// ST-267 / VD-251 — LÁT 1: frame scheduler cho đường test nội bộ.
//
// Mọi kiểu/field/hàm ở đây là **test/debug-only** (`#[doc(hidden)]`, không CLI,
// không ngôn ngữ). Runtime mặc định giữ nguyên: `frame.enabled == false` ⇒
// mọi nhánh cũ trong tệp này chạy y hệt trước. Không reactor, không socket,
// không opcode/bytecode/VERSION/cache-key mới, không `unsafe`, không dependency.
// ===========================================================================

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
    /// **ST-293 (VD-274)** — trạng thái `select_*` đang treo theo frame (claim chung nhiều kênh,
    /// điểm xoay vòng fairness, timer tuỳ chọn).
    pub select: HashMap<u32, select::SelectState>,
    /// **ST-293** — bộ đếm turn chọn để xoay vòng điểm bắt đầu giữa các lần gọi.
    pub select_turn: u64,
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
