// SPDX-License-Identifier: MIT OR Apache-2.0
//! Huỷ hợp tác nội bộ (chỉ test/debug): ghi nhận request + dọn tại safe point — tách khỏi
//! `sched.rs` ở `ST-317b`.

use std::cell::RefCell;
use std::rc::Rc;

use vis_core::value::{Chan, TrangThai, Wait};

use super::scheduler::Scheduler;
use crate::rt::frame_test::{ReactorReg, Waiter};
use crate::rt::select;

impl Scheduler {
    // -----------------------------------------------------------------------
    // VD-257 / ST-C — cancellation HỢP TÁC, NỘI BỘ (CHỈ TEST/DEBUG).
    //
    // Request chỉ được GHI NHẬN ở đây; task đang chạy KHÔNG BAO GIỜ bị cắt giữa chừng.
    // Việc dọn tài nguyên chỉ xảy ra khi scheduler đi qua một SAFE POINT mà driver đã gọi
    // sẵn (`frame_next`, `frame_fire`, `frame_reactor_fire`, `frame_wake_one`). Không API
    // ngôn ngữ, không CLI flag, không đổi runtime mặc định (`frame.enabled == false`).
    // -----------------------------------------------------------------------

    /// **ST-287 (P0, chỉ test)** — cộng byte `try_read` thật của frame hiện tại vào trigger
    /// `cancel_on_bytes`; đủ ngưỡng ⇒ ghi nhận huỷ (xử lý ở safe point `frame_next` kế tiếp).
    pub(crate) fn frame_dem_byte_va_yeu_cau_huy(&mut self, handle: &str, n: usize) {
        if self.frame.cancel_on_bytes.is_empty() {
            return;
        }
        let cur = self.frame.current;
        let mut due = Vec::new();
        self.frame.cancel_on_bytes.retain_mut(|(pre, id, con)| {
            if *id == cur && handle.starts_with(pre.as_str()) {
                *con = con.saturating_sub(n);
                if *con == 0 {
                    due.push(*id);
                    return false;
                }
            }
            true
        });
        for id in due {
            self.frame_cancel_request_for_test(id);
        }
    }

    /// Ghi nhận một cancellation request cho task `id`. Idempotent — request trùng bị bỏ.
    /// Không dọn gì ngay: phải đợi safe point (`frame_cancel_process_for_test`).
    #[doc(hidden)]
    pub fn frame_cancel_request_for_test(&mut self, id: u32) {
        if self.frame.cancelled.contains(&id) || self.frame.cancel_req.contains(&id) {
            return;
        }
        self.frame.cancel_req.push(id);
        self.frame.log.push(format!("cancelreq:{}", id));
    }

    /// Xử lý mọi request đã ghi nhận tại safe point hiện tại. Trả số task vừa bị huỷ.
    #[doc(hidden)]
    pub fn frame_cancel_process_for_test(&mut self) -> usize {
        let reqs: Vec<u32> = std::mem::take(&mut self.frame.cancel_req);
        let mut n = 0;
        for id in reqs {
            if self.frame_cancel_one(id) {
                n += 1;
            }
        }
        n
    }

    /// Công việc huỷ cho MỘT task: gỡ waiter/reg/timer/external **đúng một lần**, không
    /// wake lại, không đóng language handle. `true` = task chuyển sang `Huy`.
    fn frame_cancel_one(&mut self, id: u32) -> bool {
        if self.frame.cancelled.contains(&id) {
            self.frame.log.push(format!("cancelagain:{}", id));
            return false;
        }
        let t = match self.tasks.iter().find(|t| t.id == id).cloned() {
            Some(t) => t,
            None => {
                self.frame.cancelled.insert(id);
                self.frame.log.push(format!("cancelgone:{}", id));
                return false;
            }
        };
        // Task đã xong trước khi request tới ⇒ huỷ vô hiệu: GIỮ NGUYÊN cache (`kq`).
        if t.trang_thai.get() == TrangThai::Xong {
            self.frame.cancelled.insert(id);
            self.frame.log.push(format!("cancelskip:{}:done", id));
            return false;
        }

        // Awaiter đang chờ chính task này (Wait::Task(id)) — sẽ được wake để đọc cache/error
        // (không bịa mã mới; xem BLOCKER). Thu thập TRƯỚC khi sửa waiters.
        let awaiters: Vec<Waiter> = self
            .frame
            .waiters
            .iter()
            .filter(|w| w.wait == Wait::Task(id))
            .cloned()
            .collect();

        // 1) Waiter của frame bị huỷ: gỡ đúng một lần.
        let wb = self.frame.waiters.len();
        self.frame.waiters.retain(|w| w.frame != id);
        if self.frame.waiters.len() < wb {
            self.frame.log.push(format!("cancelwait:{}", id));
        }

        // 1b) **VS3-2a**: gỡ trạng thái chờ rendezvous của frame khỏi mọi kênh. Không đóng
        //     kênh, không đụng handle; sender `Taken` coi như đã gửi, receiver đã ghép giá trị
        //     thì giá trị bị bỏ (không refund — cùng tinh thần không-refund của ngân sách).
        let kenh: Vec<Rc<RefCell<Chan>>> = self.kenh.clone();
        let mut chan_clean = false;
        for c in kenh {
            if c.borrow_mut().huy_frame(id) {
                chan_clean = true;
            }
        }
        if chan_clean {
            self.frame.log.push(format!("cancelchan:{}", id));
        }

        // 2) Đăng ký reactor/timer của frame bị huỷ: nếu **không còn waiter nào khác** dùng
        //    cùng `(key, gen)` thì deregister adapter đúng một lần, bỏ sổ token/results/fired
        //    và event timer đang chờ. Nếu còn waiter khác (handle chia sẻ) thì **giữ** đăng ký,
        //    chuyển quyền sở hữu cho waiter còn lại — huỷ một task không được làm hỏng task kia.
        let regs: Vec<ReactorReg> = self
            .frame
            .reactor
            .regs
            .iter()
            .filter(|r| r.frame == id)
            .cloned()
            .collect();
        let mut removed = 0usize;
        for r in &regs {
            let other = self
                .frame
                .waiters
                .iter()
                .find(|w| {
                    w.wait
                        == Wait::Reactor {
                            key: r.key,
                            gen: r.gen,
                        }
                        && !self.frame.cancelled.contains(&w.frame)
                })
                .map(|w| w.frame);
            if let Some(f) = other {
                if let Some(reg) = self.frame.reactor.regs.iter_mut().find(|x| x.key == r.key) {
                    reg.frame = f;
                }
                self.frame
                    .reactor
                    .log
                    .push(format!("cancelshare:{}:{}:{}", id, r.key, f));
                continue;
            }
            if let Some(b) = self.frame.backend.clone() {
                let _ = b.borrow_mut().deregister(r.token, &r.handle, r.op);
                self.frame
                    .log
                    .push(format!("canceldereg:{}:{}", id, r.token));
            }
            self.frame.token_map.remove(&r.token);
            self.frame.reactor.results.remove(&r.key);
            self.frame.reactor.fired.remove(&r.key);
            // Timer/reactor event đang chờ của chính đăng ký này cũng phải biến mất.
            self.frame.reactor.events.retain(|e| e.key != r.key);
            // **VS3-1b**: buffer `net.write` (test-only) không được mồ côi khi task bị huỷ.
            // Chỉ dọn khi đăng ký bị gỡ hẳn — handle chia sẻ còn waiter thì giữ nguyên.
            let _ = crate::stdlib::frame_net_take_write(&r.handle);
            removed += 1;
        }
        if removed > 0 {
            self.frame.reactor.regs.retain(|r| r.frame != id);
            self.frame
                .log
                .push(format!("cancelregs:{}:{}", id, removed));
        }

        // 2b) **ST-287 (VD-273)** — đóng tay HTTP async còn treo của task bị huỷ: `H2Async`
        //     nhận `RST_STREAM` đúng stream, h1 đóng socket thật; sau đó bỏ state `http.get`
        //     của frame để frame id tái dùng không đóng lần hai. Adapter không hỗ trợ HTTP
        //     (scripted/test cũ) dùng mặc định no-op.
        if let Some(b) = self.frame.backend.clone() {
            b.borrow_mut().cancel_task_handles(id);
        }
        crate::net::xoa_trang_thai_frame(id);
        // **ST-293**: dọn state/wait-set `select_*` của frame bị huỷ (nếu có).
        select::huy_frame(self, id);

        // 3) Nguồn giả + ready: bỏ entry của task này ⇒ event nhắm vào nó thành stale.
        self.frame.external.retain(|tid, _| *tid != id);
        let rb = self.frame.ready.len();
        self.frame.ready.retain(|f| *f != id);
        if self.frame.ready.len() < rb {
            self.frame.log.push(format!("cancelready:{}", id));
        }

        // 4) Terminal. KHÔNG đóng handle mạng (language handle có thể thoát task / shared),
        //    KHÔNG ghi kq lỗi mới. Task chỉ ngừng chạy source và chờ thu hồi.
        t.trang_thai.set(TrangThai::Huy);
        self.frame.cancelled.insert(id);
        self.frame.log.push(format!("cancel:{}", id));

        // 5) Đánh thức awaiter để chúng đọc cache/error thay vì treo vô hạn.
        for w in awaiters {
            self.frame_wake_one(w);
        }
        true
    }
}
