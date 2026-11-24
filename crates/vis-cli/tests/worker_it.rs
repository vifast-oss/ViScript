// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310e** — binary gộp miền: worker pool/eval/frame (W1..W5).
//!
//! Sáu tệp ca gốc nằm trong thư mục `worker_it/`; soak dài hạn nằm ở
//! `examples/vs3_worker_soak.rs` và được gọi lại bằng một ca gate bên dưới (không mất ca).
//! Tổng số ca/assertion không đổi — chỉ đổi crate test binary.

#[path = "worker_it/vs3_worker_frame.rs"]
mod vs3_worker_frame;
#[path = "../examples/vs3_worker_soak.rs"]
mod vs3_worker_soak;

/// Giữ ca soak mặc định (1 vòng) trong gate sau khi tách thân soak thành example.
#[test]
fn soak_mixed_shared_copy_move_w1_w4() {
    vs3_worker_soak::mixed_soak_shared_copy_move_w1_w4();
}
