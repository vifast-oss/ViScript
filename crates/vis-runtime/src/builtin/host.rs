// SPDX-License-Identifier: MIT OR Apache-2.0
//! Trait [`Host`] — **chủ nhà** cho các hàm ngữ nghĩa cần tác dụng phụ (ST-27, ST-60, ST-292).

use vis_syntax::Span;

pub(crate) use vis_core::host::{DbHost, FsHost, KetQua, NetHost};
use vis_core::value::Value;

/// **Chủ nhà** cho các hàm ngữ nghĩa cần tác dụng phụ.
/// **Chương trình ngoài** (ST-27): `vis-interp` **không** biết OS — chỉ biết trait này.
///
/// `vis-cli` cài bằng `std::process::Command`; test/không host ⇒ `None` ⇒ `VS0604`.
pub trait Host {
    /// Bộ đệm `stdout` của chương trình.
    fn out(&mut self) -> &mut String;
    /// Gọi một giá trị hàm (closure/builtin) với đối số đã đánh giá.
    fn call(&mut self, f: &Value, args: Vec<Value>, span: Span) -> vis_core::trap::R<Value>;
    /// Hệ thống tệp (ST-33). Mặc định: `None` ⇒ môi trường không có host ⇒ `VS0604`.
    fn fs(&mut self) -> Option<&mut dyn FsHost> {
        None
    }

    /// **ST-63 (VD-105)**: host mạng (TCP). Mặc định: `None` ⇒ `VS0604`.
    fn net(&mut self) -> Option<&mut dyn NetHost> {
        None
    }

    /// **VD-241 (Gate 1)**: host cơ sở dữ liệu (SQLite bundled ở `vis-cli`). Mặc định: `None` ⇒
    /// `VS0604` (host không có SQLite vẫn báo lỗi rõ, không panic, không hỏng tính năng khác).
    fn db(&mut self) -> Option<&mut dyn DbHost> {
        None
    }

    /// Chương trình **ngoài** (ST-27). Mặc định: `None` ⇒ môi trường không có host ⇒ `VS0604`.
    fn ngoai(
        &mut self,
        _ten: &str,
        _args: &[String],
        _raw: bool,
        _stdin: &str,
        _span: Span,
    ) -> Option<vis_core::trap::R<Value>> {
        None
    }

    /// **ST-60 (VD-95)**: hàng đợi **tác vụ** của môi trường (`$tac_vu`/`$spawn`/`$await` dùng).
    /// Một `RefCell` để hai lời gọi lồng nhau (`await` trong thân tác vụ) **không** mượn chồng.
    fn sched(&self) -> &std::cell::RefCell<crate::rt::Scheduler>;

    /// **ST-60**: chạy **thân** tác vụ theo **handle của host** — interp: `sym` của `HFn`; VM: chỉ
    /// số hàm bytecode. Handle **không** bao giờ lộ ra chương trình.
    fn goi_than(&mut self, fi: u32, args: Vec<Value>, span: Span) -> vis_core::trap::R<Value>;

    /// **ST-60**: tên hàm theo handle (cho thông điệp `VS0506`); mặc định rỗng.
    fn ten_than(&self, _fi: u32) -> String {
        String::new()
    }

    /// **VS3-3b/W5 (P3)** — spawn task qua host (interp: scheduler cũ; VM: pool ở mode Parallel).
    fn spawn_task(
        &mut self,
        than: u32,
        args: Vec<Value>,
        ten: String,
        span: Span,
    ) -> vis_core::trap::R<Value>;

    /// **VS3-3b/W5 (P3)** — await tác vụ qua host (mặc định Off: `rt::await_task`).
    fn await_task(
        &mut self,
        t: &std::rc::Rc<crate::rt::Task>,
        span: Span,
    ) -> vis_core::trap::R<Value>;

    /// **VS3-3b/W5 (P3)** — join cuối qua host (mặc định Off: `rt::join_all`).
    fn join_tasks(&mut self) -> vis_core::trap::R<()>;

    /// Như [`Host::ngoai`] nhưng trả **đủ** `stdout/stderr/code` (ST-34, cho `| complete`).
    fn ngoai_day_du(
        &mut self,
        _ten: &str,
        _args: &[String],
        _stdin: &str,
        _span: Span,
    ) -> Option<Result<KetQua, vis_core::trap::Abort>> {
        None
    }

    // -----------------------------------------------------------------------
    // **ST-292 (VD-275/VD-280/VD-281/VD-282)** — hook evaluator worker cho frame driver.
    // Mặc định no-op ⇒ `Off`/`Local`/non-frame P1..P3 giữ nguyên hành vi. Interp và VM
    // override để pump outcome/cancel/final join mà driver không cần biết `Scheduler.exec`.
    // -----------------------------------------------------------------------

    /// `true` nếu `id` là task evaluator **worker** (đang chờ/gửi pool), không phải frame local.
    fn worker_is_task(&self, _id: u32) -> bool {
        false
    }

    /// `true` nếu còn job/task worker chưa có outcome (pool đang in-flight).
    fn worker_in_flight(&self) -> bool {
        false
    }

    /// Rút outcome đã sẵn sàng; `block` ⇒ chờ tối đa ~50 ms rồi trả. Trả số task vừa đánh thức.
    fn worker_pump(&mut self, _block: bool) -> vis_core::trap::R<usize> {
        Ok(0)
    }

    /// Final join worker: chờ pool, cache mọi task, flush output chưa flush theo spawn order.
    fn worker_final_join(&mut self) -> vis_core::trap::R<()> {
        Ok(())
    }

    /// Huỷ hợp tác task worker (idempotent); không preempt task đang chờ host/chan.
    fn worker_cancel(&mut self, _id: u32) {}
}
