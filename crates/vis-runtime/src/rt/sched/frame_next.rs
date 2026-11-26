// SPDX-License-Identifier: MIT OR Apache-2.0
//! `frame_next`: chọn frame kế tiếp theo FIFO ready + ngân sách nhường event — tách khỏi
//! `sched.rs` ở `ST-317b`.

use vis_core::value::Wait;

use super::scheduler::Scheduler;
use crate::rt::frame_test::FRAME_READY_BUDGET;

impl Scheduler {
    /// **Chỉ test**: lấy frame tiếp theo cần chạy. Ưu tiên FIFO `ready`; nhưng cứ
    /// [`FRAME_READY_BUDGET`] frame sẵn sàng liên tiếp thì giao **một** event kế tiếp
    /// (fake/reactor) trước (xem [`Self::frame_after_ready_budget`]) để I/O/timer không bị
    /// đói sau hàng ready dài. Nếu rỗng và còn event giả, fire event có `seq` nhỏ nhất
    /// (thứ tự recorded trong trace) và đánh thức waiter — mỗi frame được nối **vào đuôi**
    /// đúng một lần.
    #[doc(hidden)]
    pub fn frame_next(&mut self) -> Option<u32> {
        // VD-257/C: mỗi lần scheduler bước là một SAFE POINT — xử lý request đã ghi nhận,
        // trước khi chọn frame tiếp theo. Không bao giờ cắt ngang `step_frame`.
        // **ST-287**: trigger huỷ theo số turn (chỉ test) — cho phép huỷ giữa thân HTTP chậm.
        // Production: `cancel_on_turn` rỗng ⇒ chỉ một phép kiểm `is_empty`, không tăng bộ đếm.
        if !self.frame.cancel_on_turn.is_empty() {
            self.frame.turn += 1;
            let turn = self.frame.turn;
            let mut due = Vec::new();
            self.frame.cancel_on_turn.retain(|(t, id)| {
                if *t <= turn {
                    due.push(*id);
                    false
                } else {
                    true
                }
            });
            for id in due {
                self.frame_cancel_request_for_test(id);
            }
        }
        if !self.frame.cancel_req.is_empty() {
            self.frame_cancel_process_for_test();
        }
        // **VD-258**: không để event chết đói sau hàng ready dài. Nếu event kế tiếp có
        // waiter thật thì giao **ngay** (event là bất đồng bộ), rồi vẫn trả frame đầu
        // ready — thứ tự chạy CPU không đổi, chỉ timeline event tiến sớm hơn.
        if let Some(ev) = self.frame.events.front().cloned() {
            let co_nguoi_cho = self.frame.waiters.iter().any(|w| {
                w.gen == ev.gen
                    && w.wait
                        == (Wait::Event {
                            key: ev.key,
                            gen: ev.gen,
                        })
            });
            if co_nguoi_cho {
                self.frame.events.pop_front();
                self.frame_fire(ev);
            }
        }
        // **VD-258**: cùng quy tắc cho reactor giả — event kế tiếp (theo `seq`) đã có
        // đăng ký + waiter thì giao ngay, không chờ hàng ready cạn (tránh đói I/O khi
        // một frame CPU luôn sẵn sàng).
        if let Some(ev) = self.frame.reactor.events.first().cloned() {
            if ev.deadline_ms.is_none() {
                let co_nguoi_cho = self
                    .frame
                    .reactor
                    .regs
                    .iter()
                    .any(|r| r.key == ev.key && r.gen == ev.gen)
                    && self.frame.waiters.iter().any(|w| {
                        w.wait
                            == (Wait::Reactor {
                                key: ev.key,
                                gen: ev.gen,
                            })
                    });
                if co_nguoi_cho {
                    self.frame.reactor.events.remove(0);
                    self.frame_reactor_fire(ev);
                }
            }
        }
        // **VD-260**: hàng ready dài không được bỏ đói timer/reactor/fake-source. Cứ
        // `FRAME_READY_BUDGET` frame sẵn sàng liên tiếp thì giao event kế tiếp (đúng
        // `seq`) **trước** frame ready. Bộ đếm tất định — không dùng thời gian treo.
        if !self.frame.ready.is_empty() && self.frame.ready_streak >= FRAME_READY_BUDGET {
            self.frame_after_ready_budget();
        }
        if let Some(f) = self.frame.ready.pop_front() {
            self.frame.ready_streak = self.frame.ready_streak.saturating_add(1);
            return Some(f);
        }
        self.frame.ready_streak = 0;
        // Không còn frame sẵn sàng ⇒ fire sự kiện giả tiếp theo (theo `seq`).
        if let Some(ev) = self.frame.events.pop_front() {
            self.frame_fire(ev);
            if let Some(f) = self.frame.ready.pop_front() {
                self.frame.ready_streak = self.frame.ready_streak.saturating_add(1);
                return Some(f);
            }
            return None;
        }
        // Lát 2: hết frame + hết event Lát 1 ⇒ chờ reactor / timer monotonic rồi fire
        // **một** event reactor theo `seq` (xem `frame_idle`). Không sleep-poll.
        if self.frame_idle() {
            if let Some(f) = self.frame.ready.pop_front() {
                self.frame.ready_streak = self.frame.ready_streak.saturating_add(1);
                return Some(f);
            }
        }
        None
    }
}
