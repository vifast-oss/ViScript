// SPDX-License-Identifier: MIT OR Apache-2.0
//! Kết thúc tác vụ: deadlock/stuck, wake theo kênh, task done, drain event, final join, thu hồi
//! — tách khỏi `sched.rs` ở `ST-317b`.

use std::rc::Rc;

use vis_core::trap::Trap;
use vis_core::value::{TrangThai, Value, Wait};
use vis_syntax::Span;

use super::scheduler::Scheduler;
use crate::rt::frame_test::Waiter;

impl Scheduler {
    /// **VD-258**: sau khi scheduler hết frame sẵn sàng, còn waiter/task chưa terminal hoặc
    /// đăng ký reactor sống ⇒ chương trình **treo thật** (event không bao giờ tới). Trả trap
    /// `VS0505` thay vì để driver coi là "xong" rồi exit 0. Hai engine dùng chung hàm này để
    /// chẩn đoán giống hệt nhau.
    #[doc(hidden)]
    pub fn frame_stuck_trap(&self) -> Option<Trap> {
        let live = self
            .tasks
            .iter()
            .any(|t| !matches!(t.trang_thai.get(), TrangThai::Xong | TrangThai::Huy));
        if self.frame.waiters.is_empty() && !live && self.frame.reactor.regs.is_empty() {
            return None;
        }
        // **VS3-1d**: giữ thông điệp cụ thể cho kênh đầy/rỗng (test/golden cũ dùng), nhưng
        // vẫn là cùng mã `VS0505` — không thêm mã mới.
        let msg = self
            .frame
            .waiters
            .iter()
            .find_map(|w| match w.wait {
                Wait::Chan { id, send } => Some((id, send)),
                _ => None,
            })
            .and_then(|(id, send)| {
                let ch = self.kenh.iter().find(|c| c.borrow().id == id)?;
                let ch = ch.borrow();
                if send && ch.cap > 0 && ch.day() {
                    Some(
                        "deadlock: `send` vào kênh **đầy** và không còn tác vụ nào chạy được"
                            .to_string(),
                    )
                } else if !send && ch.rong() && !ch.closed {
                    Some(
                        "deadlock: `recv` trên kênh **rỗng** và không còn tác vụ nào chạy được"
                            .to_string(),
                    )
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                "deadlock: scheduler hết frame sẵn sàng nhưng còn tác vụ/frame chờ sự kiện không bao giờ tới"
                    .to_string()
            });
        Some(Trap::new(crate::codes::DEADLOCK, msg, Span::new(0, 0)))
    }

    /// **VS3-2a**: đánh thức một frame đang chờ kênh `id` theo chiều `send`. Tìm waiter theo
    /// `(frame, Wait::Chan)` (không phụ thuộc `gen`), rồi dùng chung `frame_wake_one` ⇒ không
    /// wake trùng, không đưa frame đã huỷ vào ready.
    pub(crate) fn frame_wake_chan(&mut self, id: u32, send: bool, frame: u32) {
        let w = self
            .frame
            .waiters
            .iter()
            .find(|w| w.frame == frame && w.wait == Wait::Chan { id, send })
            .copied();
        match w {
            Some(w) => self.frame_wake_one(w),
            None => self
                .frame
                .log
                .push(format!("chanmiss:{}:{}:{}", id, send, frame)),
        }
    }

    /// **Chỉ test**: một task vừa xong — cache kết quả, ghi nghĩa vụ join, đánh thức
    /// mọi frame đang chờ task đó (FIFO, nối đuôi).
    #[doc(hidden)]
    pub fn frame_task_done(&mut self, id: u32, kq: Result<Value, Trap>) {
        self.frame.joined.insert(id);
        // **VD-259**: ghi cache nếu engine chưa ghi (hợp đồng của doc). Khi engine đã ghi
        // `t.kq` trước đó thì giữ nguyên giá trị thật (không bị `Unit` ghi đè).
        if let Some(t) = self.tasks.iter().find(|t| t.id == id) {
            let mut slot = t.kq.borrow_mut();
            if slot.is_none() {
                *slot = Some(kq);
            }
        }
        let waits: Vec<Waiter> = self.frame.waiters.clone();
        for w in waits {
            if w.wait == Wait::Task(id) {
                self.frame_wake_one(w);
            }
        }
    }

    /// **Chỉ test**: fire nốt các event còn lại trong trace —duplicate/stale cũng được
    /// ghi log ⇒ hai engine so sánh được trace **đầy đủ**, không phải trace cắt ngắn.
    #[doc(hidden)]
    pub fn frame_drain_events(&mut self) {
        while let Some(ev) = self.frame.events.pop_front() {
            self.frame_fire(ev);
        }
        // Lát 2: fire nốt event reactor còn lại (duplicate/stale vẫn được log) để trace
        // đầy đủ giữa hai engine — không cắt ngắn.
        let evs = std::mem::take(&mut self.frame.reactor.events);
        for ev in evs {
            self.frame_reactor_fire(ev);
        }
    }

    /// **Chỉ test**: main kết thúc — join mọi task chưa xong (không auto-cancel), rồi
    /// thu hồi các task đã xong + không còn handle sống + đã có nghĩa vụ join.
    ///
    /// **ST-278 (VD-256)**: trước đây hàm này **nuốt** lỗi task đã cache (`kq = Some(Err(..))`)
    /// — task lỗi mà main không `await` vẫn làm chương trình exit 0. Nay lỗi đó được **đẩy ra**
    /// theo đúng hợp đồng `VS0506` (mã gốc + span lúc tạo, do driver bọc sẵn) và trả về cho driver
    /// quyết định trap. **Không** đổi chính sách hủy/đánh thức các task còn lại.
    #[doc(hidden)]
    pub fn frame_final_join(&mut self) -> Option<Trap> {
        // Quét theo **thứ tự tạo** (cũng là thứ tự join) để kết quả tất định.
        let err = self
            .tasks
            .iter()
            .find_map(|t| match t.kq.borrow().as_ref() {
                Some(Err(tr)) => Some(tr.clone()),
                _ => None,
            });
        let ids: Vec<u32> = self.tasks.iter().map(|t| t.id).collect();
        for id in ids {
            self.frame.joined.insert(id);
        }
        self.frame_reclaim();
        if err.is_some() {
            self.frame.final_join_error = err.clone();
        }
        err
    }

    /// Lấy (và xoá) lỗi task bị nuốt ở final join — driver gán vào trap chung.
    #[doc(hidden)]
    pub fn frame_take_final_join_error_for_test(&mut self) -> Option<Trap> {
        self.frame.final_join_error.take()
    }

    /// **Chỉ test**: thu hồi task thỏa 3 điều kiện ownership: terminal (`Xong` **hoặc**
    /// `Huy` VD-257/C), có nghĩa vụ join, và không còn handle/reference sống
    /// (`Rc::strong_count == 1` chỉ còn scheduler). ready/waiter/registration cũng bị dọn
    /// theo — không giữ task chết quá ý đồ. Task `Huy` không có `kq` lỗi ⇒ final-join không
    /// bịa mã huỷ mới (xem BLOCKER trong report VD-257/C).
    #[doc(hidden)]
    pub fn frame_reclaim(&mut self) {
        let dead: Vec<u32> = self
            .tasks
            .iter()
            .filter(|t| {
                (t.trang_thai.get() == TrangThai::Xong || t.trang_thai.get() == TrangThai::Huy)
                    && self.frame.joined.contains(&t.id)
                    && Rc::strong_count(t) == 1
            })
            .map(|t| t.id)
            .collect();
        if dead.is_empty() {
            return;
        }
        for id in &dead {
            self.frame.log.push(format!("reclaim:{}", id));
        }
        self.tasks.retain(|t| !dead.contains(&t.id));
        self.frame.waiters.retain(|w| !dead.contains(&w.frame));
        self.frame.external.retain(|id, _| !dead.contains(id));
        self.frame.ready.retain(|f| !dead.contains(f));
        // Lát 2: đăng ký/kết quả của frame đã thu hồi cũng phải biến mất (lifecycle).
        // **VD-259**: phải gom `key` TRƯỚC khi `retain` — bản cũ gom sau nên danh sách luôn
        // rỗng và `results`/`fired` của frame chết bị giữ lại.
        let dead_keys: Vec<u64> = self
            .frame
            .reactor
            .regs
            .iter()
            .filter(|r| dead.contains(&r.frame))
            .map(|r| r.key)
            .collect();
        self.frame.reactor.regs.retain(|r| !dead.contains(&r.frame));
        for k in dead_keys {
            self.frame.reactor.results.remove(&k);
            self.frame.reactor.fired.remove(&k);
        }
    }
}
