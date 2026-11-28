// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-313a (VD-289)** — `Scheduler` + API điều khiển frame (chuyển nguyên văn từ `vis-interp/src/rt.rs`).
//!
//! Phần hợp nhất `rt/sched.rs` cũ (frame reactor test API) + định nghĩa `Scheduler` (trước ở `rt.rs`).
//!
//! **ST-317b**: tách `sched.rs` (1.189 dòng) thành `mod.rs` (khai báo + re-export) và các tệp
//! chuyên trách: `scheduler` (định nghĩa + tạo task + chẩn đoán), `frame_api` (bật/re-arm/
//! `await`), `frame_next` (chọn frame kế tiếp), `frame_su_kien` (fire event + wake),
//! `frame_ket_thuc` (join/thu hồi/deadlock), `frame_huy` (huỷ hợp tác), `frame_reactor`
//! (reactor giả). Hành vi và API công khai không đổi.

mod frame_api;
mod frame_huy;
mod frame_ket_thuc;
mod frame_next;
mod frame_reactor;
mod frame_su_kien;
mod scheduler;
mod trigger;

pub use scheduler::Scheduler;
pub(crate) use trigger::{FRAME_CANCEL_BYTES, FRAME_CANCEL_TEST, FRAME_CANCEL_TURNS};
