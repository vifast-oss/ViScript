// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-313a (VD-289)** — sổ mã runtime dùng chung interp + VM.
//!
//! Chuyển **nguyên văn** từ `vis-interp/src/frame/core.rs`; không đổi mã/ý nghĩa.

use vis_diag::{Code, RUNTIME_CODES};

/// `VS0501` — lỗi runtime chung.
pub const RT: &Code = &RUNTIME_CODES[0];
/// `VS0502` — chia/lấy dư cho 0.
pub const DIV0: &Code = &RUNTIME_CODES[1];
/// `VS0503` — chỉ số ngoài phạm vi.
pub const IDX: &Code = &RUNTIME_CODES[2];
/// **ST-60** — `VS0506`: lỗi trong **thân tác vụ** `async` (message kèm mã gốc).
pub const TASK_ERR: &Code = &RUNTIME_CODES[5];
/// **ST-61 (VD-96)** — `VS0504`: gửi vào kênh đã `close`.
pub const SEND_CLOSED: &Code = &RUNTIME_CODES[3];
/// **ST-61 (VD-96)** — `VS0505`: deadlock (kênh chặn, không còn tác vụ chạy được).
pub const DEADLOCK: &Code = &RUNTIME_CODES[4];
/// **R9 (VD-178)** — `VS0507`: tràn `i64` lúc chạy (tách khỏi `VS0501`).
pub const OVERFLOW: &Code = &RUNTIME_CODES[6];

/// **ST-290 (VD-273)** — `VS0510`: vượt **ngân sách thực thi ngữ nghĩa**, tách hẳn khỏi
/// `VS0501` (trap nội bộ) và khỏi fuel `SAFETY_FUEL` của VM.
pub const BUDGET: &Code = &RUNTIME_CODES[9];
