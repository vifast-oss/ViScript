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
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::{Duration, Instant};

use vis_syntax::Span;

use crate::interp::{Abort, Trap, R};
use crate::value::Value;

pub(crate) use vis_core::reactor::{
    FrameReactorBackend, ReactorH2Outcome, ReactorH2Response, ReactorIoOutcome, ReactorOp,
};
pub(crate) use vis_core::value::{Chan, ChanSend, ChanSendState, Pending, Task, TrangThai, Wait};

pub(crate) mod chan;
/// **VS3-3b/W5 (P1)** — chế độ evaluator W=1 (`Off`/`Local`); P2 thêm pool.
pub(crate) mod exec;
/// **ST-276 (VD-256)**: định nghĩa sự kiện ngân sách **dùng chung** với `vis-bytecode`.
/// Khai báo ở đây (thay vì `frame.rs`) để giữ nguyên phạm vi sửa của ST-278 — file nguồn nằm
/// trong `frame/**`, nội dung là `include!` của `vis-bytecode/src/fee_shared.rs`.
#[path = "frame/fee.rs"]
pub mod fee;
mod frame_test;
pub use frame_test::*;
mod ops;
mod reactor_io;
pub use ops::*;
mod sched;
/// **ST-293 (VD-274/VD-280)** — `select_recv*`/`select_send*`: wait-set nhiều kênh + fairness.
pub(crate) mod select;

/// Chặn **lồng** `await` (mỗi tầng là một khung gọi thật) — 256 như chốt VD-95 (7).
pub const MAX_TASK_NEST: usize = 256;

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
