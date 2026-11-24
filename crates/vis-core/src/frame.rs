// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-312a (VD-289)** — kiểu cấu hình/kết quả frame test dùng chung.
//!
//! Chuyển **nguyên văn** từ `vis-interp/src/frame.rs` (`FrameConfig`/`FrameOutput`) và
//! `vis-interp/src/rt/frame_test.rs` (`FakeEvent`/`ReactorOutcome`/`ReactorEvent`/`DiagSnapshot`).
//! Toàn bộ là **harness test** (`#[doc(hidden)]`, không CLI, không ngôn ngữ); nằm ở đây để
//! `vis-vm` và các test tích hợp đọc trực tiếp `vis-core` mà không cần phụ thuộc `vis-interp`.

use std::cell::RefCell;
use std::rc::Rc;

use vis_syntax::Diagnostic;

use crate::reactor::FrameReactorBackend;
use crate::value::Value;

/// Cấu hình frame test — `fake_sources`: `(tên hàm async, khóa event)`; `events`: trace.
#[doc(hidden)]
#[derive(Debug, Default, Clone)]
pub struct FrameConfig {
    pub fake_sources: Vec<(String, u64)>,
    pub events: Vec<FakeEvent>,
    /// **Lát 2**: trace reactor giả (read/accept/timer). `seq` quyết định thứ tự fire.
    #[doc(hidden)]
    pub reactors: Vec<ReactorEvent>,
    /// **Lát 2**: `true` ⇒ khi rảnh, ngủ một lần tới deadline monotonic gần nhất (đo B1).
    #[doc(hidden)]
    pub timer_real_wait: bool,
    /// **Lát 3 (chỉ test)**: adapter selector thật. `None` ⇒ đường reactor giả Lát 2.
    #[doc(hidden)]
    pub backend: Option<Rc<RefCell<dyn FrameReactorBackend>>>,
}

/// Kết quả frame test — kèm log scheduler (tất định) + snapshot ownership.
#[doc(hidden)]
#[derive(Debug)]
pub struct FrameOutput {
    pub stdout: String,
    pub diagnostics: Vec<Diagnostic>,
    pub value: Option<Value>,
    pub exit: i32,
    pub log: Vec<String>,
    pub snapshot: DiagSnapshot,
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
