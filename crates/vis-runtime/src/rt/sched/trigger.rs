// SPDX-License-Identifier: MIT OR Apache-2.0
//! Trigger huỷ do test ghi **trước** khi engine chạy (drain một lần ở
//! `Scheduler::frame_enable_for_test`) — tách khỏi `sched.rs` ở `ST-317b`.

use std::cell::RefCell;

// nên cả hai engine (interp + VM) cùng thấy một trigger — không CLI flag, không API ngôn ngữ.
thread_local! {
    pub(crate) static FRAME_CANCEL_TEST: RefCell<Vec<(u64, u32)>> = const { RefCell::new(Vec::new()) };
    /// **ST-287** — trigger huỷ theo **số turn scheduler** (đếm `frame_next`), drain vào
    /// `FrameTest.cancel_on_turn` ở `frame_enable_for_test` như trigger theo `seq`.
    pub(crate) static FRAME_CANCEL_TURNS: RefCell<Vec<(u64, u32)>> = const { RefCell::new(Vec::new()) };
    /// **ST-287 (P0)** — trigger huỷ theo **byte đã đọc** trên handle, drain như trên.
    pub(crate) static FRAME_CANCEL_BYTES: RefCell<Vec<(String, u32, usize)>> =
        const { RefCell::new(Vec::new()) };
}
