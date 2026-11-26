// SPDX-License-Identifier: MIT OR Apache-2.0
//! Phân phối sự kiện fake/reactor: ngân sách ready, fire event, đánh thức waiter — tách khỏi
//! `sched.rs` ở `ST-317b`.

use std::time::{Duration, Instant};

use vis_core::value::{TrangThai, Wait};

use super::scheduler::Scheduler;
use crate::rt::frame_test::{FakeEvent, ReactorEvent, Waiter};

impl Scheduler {
    /// **VD-260**: giao **một** event kế tiếp sau khi hàng ready đã chiếm trọn
    /// [`FRAME_READY_BUDGET`] lượt liên tiếp. Hai trace (fake + reactor) giữ nguyên thứ tự
    /// `seq` của từng trace; giữa hai trace chọn head có `seq` nhỏ hơn (hòa ⇒ fake trước)
    /// — khóa tie-break tất định, không phụ thuộc hash/thời gian. Chỉ event **giao được**
    /// mới bị lấy khỏi trace: event của một khóa chưa từng đăng ký (có thể là consumer
    /// tương lai) phải nằm lại — nếu fire sớm nó thành stale và consumer sau đó treo.
    /// Event stale thật (khóa đã cấp rồi bị gỡ, hoặc `gen` cũ hơn) vẫn được fire để event
    /// hợp lệ phía sau không bị kẹt.
    pub(super) fn frame_after_ready_budget(&mut self) -> bool {
        let fake = self.frame.events.front().cloned();
        let reactor = self.frame.reactor.events.first().cloned();
        let fake_ok = fake
            .as_ref()
            .map(|e| self.fake_event_deliverable(e))
            .unwrap_or(false);
        let reactor_ok = reactor
            .as_ref()
            .map(|e| self.reactor_event_deliverable(e))
            .unwrap_or(false);
        match (fake, reactor) {
            (Some(f), Some(r)) if fake_ok && reactor_ok => {
                if f.seq <= r.seq {
                    self.frame_after_budget_fire_fake()
                } else {
                    self.frame_after_budget_fire_reactor()
                }
            }
            (Some(_), _) if fake_ok => self.frame_after_budget_fire_fake(),
            (_, Some(_)) if reactor_ok => self.frame_after_budget_fire_reactor(),
            _ => false,
        }
    }

    /// Event fake có giao được **bây giờ** không: có waiter đúng `(key, gen)`, hoặc
    /// `external` đã có khóa với `gen` hiện hành ≥ `gen` event (khớp/stale), hoặc khóa
    /// không hề được cấu hình làm nguồn giả (stale chắc chắn). Khóa đã cấu hình nhưng
    /// chưa có task / `gen` còn nhỏ hơn ⇒ để dành cho tương lai, không fire sớm.
    fn fake_event_deliverable(&self, ev: &FakeEvent) -> bool {
        if self.frame.waiters.iter().any(|w| {
            w.wait
                == (Wait::Event {
                    key: ev.key,
                    gen: ev.gen,
                })
                && w.gen == ev.gen
        }) {
            return true;
        }
        let mut co_nguon = false;
        for (k, g) in self.frame.external.values() {
            if *k == ev.key {
                co_nguon = true;
                if *g >= ev.gen {
                    return true;
                }
            }
        }
        if co_nguon {
            return false;
        }
        !self.frame.fake_sources.iter().any(|(_, k)| *k == ev.key)
    }

    /// Event reactor có giao được **bây giờ** không: có waiter đúng `(key, gen)`, hoặc
    /// đăng ký hiện hành của `key` có `gen` ≥ `gen` event (khớp/stale). Không còn đăng ký:
    /// `key < next_key` ⇒ khóa đã từng được cấp rồi bị gỡ (stale chắc chắn), ngược lại là
    /// khóa chưa cấp — để dành cho consumer tương lai.
    fn reactor_event_deliverable(&self, ev: &ReactorEvent) -> bool {
        if self.frame.waiters.iter().any(|w| {
            w.wait
                == (Wait::Reactor {
                    key: ev.key,
                    gen: ev.gen,
                })
        }) {
            return true;
        }
        match self.frame.reactor.regs.iter().find(|r| r.key == ev.key) {
            Some(r) => r.gen >= ev.gen,
            None => ev.key < self.frame.reactor.next_key,
        }
    }

    /// Fire head của trace fake sau ngân sách ready (stale cũng fire để mở đường).
    fn frame_after_budget_fire_fake(&mut self) -> bool {
        let Some(ev) = self.frame.events.pop_front() else {
            return false;
        };
        self.frame_fire(ev);
        true
    }

    /// Fire head của trace reactor sau ngân sách ready. Timer có đăng ký sống +
    /// `timer_real_wait` thì ngủ tới **đúng deadline của chính nó** (không fire sớm);
    /// timer đã thành stale (không còn đăng ký) fire ngay — không chặn CPU vì một mốc
    /// thời gian vô chủ.
    fn frame_after_budget_fire_reactor(&mut self) -> bool {
        let Some(ev) = self.frame.reactor.events.first().cloned() else {
            return false;
        };
        if let Some(deadline) = ev.deadline_ms {
            self.frame
                .reactor
                .log
                .push(format!("timer:{}:{}", ev.key, deadline));
            let con_dang_ky = self
                .frame
                .reactor
                .regs
                .iter()
                .any(|r| r.key == ev.key && r.gen == ev.gen);
            if self.frame.timer_real_wait && con_dang_ky {
                let epoch = *self.frame.reactor.epoch.get_or_insert_with(Instant::now);
                if let Some(target) = epoch.checked_add(Duration::from_millis(deadline)) {
                    let now = Instant::now();
                    if target > now {
                        std::thread::sleep(target - now);
                    }
                }
            }
        }
        self.frame.reactor.events.remove(0);
        self.frame_reactor_fire(ev);
        true
    }

    /// **Chỉ test**: fire một sự kiện giả. Bỏ event **stale** (gen lệch) và **trùng**
    /// (khóa đã fire ở thế hệ hiện hành) — không đánh thức ai, ghi log tất định.
    pub(super) fn frame_fire(&mut self, ev: FakeEvent) {
        self.frame.ready_streak = 0;
        self.frame
            .log
            .push(format!("fire:{}:{}:{}", ev.seq, ev.key, ev.gen));
        // VD-257/C: event có `seq` được đăng ký huỷ ⇒ ghi nhận + xử lý NGAY tại safe point
        // này, TRƯỚC khi complete target/wake. Task bị huỷ mất waiter ⇒ không chạy lại source.
        let cancels: Vec<u32> = self
            .frame
            .cancel_on_seq
            .iter()
            .filter(|(s, _)| *s == ev.seq)
            .map(|(_, id)| *id)
            .collect();
        for id in cancels {
            self.frame_cancel_request_for_test(id);
        }
        if !self.frame.cancel_req.is_empty() {
            self.frame_cancel_process_for_test();
        }
        // Tìm task nguồn giả có khóa này.
        let target: Option<u32> = self
            .frame
            .external
            .iter()
            .find(|(_, (k, g))| *k == ev.key && *g == ev.gen)
            .map(|(id, _)| *id);
        let Some(id) = target else {
            self.frame.log.push(format!("stale:{}", ev.key));
            return;
        };
        if self.frame.fired.get(&ev.key).copied() == Some(ev.gen) {
            self.frame.log.push(format!("duplicate:{}", ev.key));
            return;
        }
        self.frame.fired.insert(ev.key, ev.gen);
        // Hoàn tất nguồn giả (kết quả do trace cung cấp).
        if let Some(t) = self.tasks.iter().find(|t| t.id == id).cloned() {
            if t.trang_thai.get() != TrangThai::Xong {
                *t.kq.borrow_mut() = Some(Ok(ev.value));
                t.trang_thai.set(TrangThai::Xong);
                self.frame.joined.insert(id);
            }
        }
        // Đánh thức waiter theo thứ tự đăng ký (FIFO), nối vào đuối ready.
        let waits: Vec<Waiter> = self.frame.waiters.clone();
        for w in waits {
            if w.wait
                == (Wait::Event {
                    key: ev.key,
                    gen: ev.gen,
                })
                && w.gen == ev.gen
            {
                self.frame_wake_one(w);
            }
        }
    }

    /// **Chỉ test**: đánh thức đúng một frame — nếu chưa có trong ready. Đăng ký bị
    /// gỡ ngay (registry không giữ frame đã thức), nên fire trùng không thức lại lần 2.
    pub(super) fn frame_wake_one(&mut self, w: Waiter) {
        self.frame.waiters.retain(|x| *x != w);
        // VD-257/C: task đã huỷ KHÔNG được xếp lại vào ready — không late wake, không re-run.
        if self.frame.cancelled.contains(&w.frame) {
            self.frame.log.push(format!("cancelwake:{}", w.frame));
            return;
        }
        if self.frame.ready.contains(&w.frame) {
            self.frame.log.push(format!("dup-ready:{}", w.frame));
            return;
        }
        self.frame.ready.push_back(w.frame);
        self.frame.log.push(format!("wake:{}", w.frame));
    }
}
