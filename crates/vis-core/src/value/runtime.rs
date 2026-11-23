// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310a (VD-285)** — dữ liệu runtime dùng chung của `Value`: tác vụ `async`, kênh, và
//! tín hiệu treo frame. Chuyển **nguyên văn** từ `vis-interp/src/rt.rs` để `Value` nằm ở
//! `vis-core` mà không kéo cả bộ lập lịch.
//!
//! Mọi field từng `pub(crate)` nay là `pub` + `#[doc(hidden)]` — `vis-interp` vẫn là nơi duy
//! nhất *hành xử* các kiểu này; `vis-core` chỉ giữ **hình dạng dữ liệu**.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use vis_syntax::Span;

use crate::trap::Trap;
use crate::value::Value;

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

/// Một kênh đệm **hữu hạn** (FIFO). `cap = 0` ⇒ không đệm (gửi phải có người nhận **đã chạy**
/// — mô hình run-to-completion, nợ như async).
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
    #[doc(hidden)]
    pub send_waiters: VecDeque<ChanSend>,
    /// **VS3-2a (chỉ frame test)** — frame receiver `cap(0)` đang chờ, FIFO.
    #[doc(hidden)]
    pub recv_waiters: VecDeque<u32>,
    /// **VS3-2a (chỉ frame test)** — giá trị đã ghép cho một receiver cụ thể (chờ resume).
    #[doc(hidden)]
    pub recv_values: HashMap<u32, Value>,
}

/// **VS3-2a**: trạng thái một sender `cap(0)` đang chờ.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChanSendState {
    Pending,
    Taken,
    Closed,
}

/// **VS3-2a**: một sender `cap(0)` đang chờ (frame + giá trị + trạng thái).
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct ChanSend {
    pub frame: u32,
    pub value: Value,
    pub state: ChanSendState,
    /// **ST-293** — entry do `select_send` đăng ký (value dùng chung; claim ở scheduler).
    pub select: bool,
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
    pub fn huy_frame(&mut self, frame: u32) -> bool {
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
