// SPDX-License-Identifier: MIT OR Apache-2.0
//! API frame chỉ-test: bật/tắt, re-arm thế hệ, `await` + đăng ký waiter — tách khỏi `sched.rs`
//! ở `ST-317b`.

use std::rc::Rc;

use vis_core::value::{Task, TrangThai, Wait};

use super::scheduler::Scheduler;
use super::trigger::{FRAME_CANCEL_BYTES, FRAME_CANCEL_TEST, FRAME_CANCEL_TURNS};
use crate::rt::frame_test::{FakeEvent, Waiter};

impl Scheduler {
    /// Bật đường frame. Gọi từ integration test trên `Interp`/`Vm` thật — **không**
    /// có CLI flag hay API ngôn ngữ nào gọi tới đây.
    #[doc(hidden)]
    pub fn frame_enable_for_test(
        &mut self,
        fake_sources: Vec<(String, u64)>,
        events: Vec<FakeEvent>,
    ) {
        self.frame.enabled = true;
        self.frame.fake_sources = fake_sources;
        // **VD-259**: trace phải được sắp theo `seq` (tie-break `key`, `gen`) ngay tại nguồn —
        // trước đây giữ thứ tự caller, nên trace đảo thứ tự làm event chạy sai/ bị stale.
        let mut events = events;
        events.sort_by_key(|e| (e.seq, e.key, e.gen));
        self.frame.events = events.into();
        self.frame.gen = 0;
        self.frame.ready_streak = 0;
        self.frame.log.push("enable".into());
        // VD-257/C: nhận trigger huỷ do test ghi trước khi chạy engine. Sắp xếp để trace
        // tất định và giống hệt giữa interp/VM. Thread-local drained ⇒ mỗi engine nhận 1 lần.
        FRAME_CANCEL_TEST.with(|x| {
            self.frame.cancel_on_seq = x.borrow_mut().drain(..).collect();
        });
        self.frame.cancel_on_seq.sort();
        for (seq, id) in &self.frame.cancel_on_seq {
            self.frame.log.push(format!("cancelon:{}:{}", seq, id));
        }
        // **ST-287**: trigger huỷ theo số turn (drain một lần như trigger theo seq).
        FRAME_CANCEL_TURNS.with(|x| {
            self.frame.cancel_on_turn = x.borrow_mut().drain(..).collect();
        });
        self.frame.cancel_on_turn.sort();
        self.frame.turn = 0;
        for (turn, id) in &self.frame.cancel_on_turn {
            self.frame.log.push(format!("cancelonturn:{}:{}", turn, id));
        }
        FRAME_CANCEL_BYTES.with(|x| {
            self.frame.cancel_on_bytes = x.borrow_mut().drain(..).collect();
        });
        for (pre, id, n) in &self.frame.cancel_on_bytes {
            self.frame
                .log
                .push(format!("cancelonbytes:{}:{}:{}", pre, n, id));
        }
    }

    /// Tăng thế hệ khi một nguồn giả được **đăng ký lại** (re-arm) — event cũ phải
    /// trở thành stale. Chỉ test.
    #[doc(hidden)]
    pub fn frame_rearm_for_test(&mut self, key: u64) {
        self.frame.gen += 1;
        for (id, (k, g)) in self.frame.external.iter_mut() {
            if *k == key {
                *g = self.frame.gen;
                self.frame
                    .log
                    .push(format!("rearm:{}:{}", id, self.frame.gen));
            }
        }
    }

    /// **Chỉ test**: `await_task` trên đường frame — `None` nếu task đã xong (đi cache),
    /// ngược lại trả `Waiter` để driver đăng ký rồi treo frame hiện tại.
    #[doc(hidden)]
    pub fn frame_await(&self, current_frame: u32, t: &Rc<Task>) -> Option<Waiter> {
        if t.trang_thai.get() == TrangThai::Xong {
            return None;
        }
        // Đăng ký wait theo **id + thế hệ hiện hành của task được await**.
        let (wait, gen) = match self.frame.external.get(&t.id) {
            Some((key, g)) => (Wait::Event { key: *key, gen: *g }, *g),
            None => (Wait::Task(t.id), self.frame.gen),
        };
        Some(Waiter {
            frame: current_frame,
            wait,
            gen,
        })
    }

    /// **Chỉ test**: đăng ký một waiter đã được `frame_await` quyết định. Trùng waiter
    /// (cùng frame + cùng wait + cùng gen) bị bỏ ⇒ duplicate wake không tạo bản sao.
    #[doc(hidden)]
    pub fn frame_register(&mut self, w: Waiter) {
        if self.frame.waiters.contains(&w) {
            self.frame.log.push(format!("dup-wait:{}", w.frame));
            return;
        }
        self.frame.waiters.push(w);
        self.frame
            .log
            .push(format!("wait:{}:{:?}", w.frame, w.wait));
        // Frame đang chờ KHÔNG nằm trong ready. Nó đã có thể nằm đó từ spawn — bỏ ra.
        self.frame.ready.retain(|f| *f != w.frame);
    }
}
