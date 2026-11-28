// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-3b (W5, VD-271 B1–B7)** — worker pool **opt-in/test-only**: task affine + heap theo
//! luồng, `TaskId`/proxy completion, snapshot Env, print buffer merge theo **spawn order**, budget
//! atomic + chunk reservation.
//!
//! **ST-310a (VD-285)**: tách khỏi `include!("rt/worker.rs")` thành mô-đun thật
//! `vis-runtime/src/worker` (API công khai `vis_runtime::worker::*`); chia `proto`
//! (handle/chuyển đổi) và `pool` (bảng kết quả/WorkerPool) để mỗi tệp < 1.150 dòng.
//!
//! **Không bật mặc định.** `WorkerPool` chỉ được tạo tường minh bởi test/opt-in; runtime
//! `vis run` vẫn dùng `Scheduler` một luồng. `Value::Task(Rc<Task>)` **không đổi** (B5) — `TaskId`
//! là handle mới cho worker pool; W1 sẽ thay handle trong `Value` khi tích hợp.
//!
//! Task chạy dưới dạng closure `FnOnce(&mut WorkerCtx) -> Result<SendValue, String> + Send`;
//! `SendValue` là dạng Sendable của dữ liệu (không có `Rc`/`RefCell`). Đây là mô hình worker
//! thật nhưng chưa nối vào evaluator ViScript (W1 hook).

//! **ST-323a**: tách `pool` thành `board` (bảng completion/wait) + `ctx` (ngữ cảnh task),
//! giữ nguyên bề mặt API công khai `vis_runtime::worker::*` qua barrel này.

mod board;
mod ctx;
mod orchestrate;
mod pool;
mod proto;

pub use board::*;
pub use ctx::*;
pub use orchestrate::*;
pub use pool::*;
pub use proto::*;
