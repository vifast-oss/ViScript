// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-313a (VD-289)** — budget/quota test-only của frame VM (tách khỏi `vm_frame.rs`).
//!
//! Chuyển **nguyên văn** khối "VD-257 / D"; không đổi hành vi hay API.

use std::cell::RefCell;

use vis_bytecode::fee::{FeeEntry, FeeTable};
use vis_core::trap::{Abort, Trap, R};
use vis_runtime::codes::BUDGET;
use vis_syntax::Span;

use super::Vm;

// ---------------------------------------------------------------------------
// VD-257 / D — cấu hình budget+quota cho `run_frame_for_test` (test-only). Dùng
// thread-local để KHÔNG đổi chữ ký `run_frame_for_test` / `FrameConfig`, không API
// công, không CLI. Mặc định rỗng ⇒ đường frame VM không charge (test f2 vẫn đúng).
// **ST-290 (VD-273)** — thêm **bảng phí** cho lần chạy kế tiếp do CLI đặt; nó là
// **một-lần** (take) để chương trình kế tiếp không dùng nhầm bảng phí của chương trình trước.
// ---------------------------------------------------------------------------
#[derive(Default)]
pub(crate) struct FrameBudgetCfg {
    pub(crate) fee: Option<FeeTable>,
    pub(crate) budget: u64,
    pub(crate) quota: Option<u64>,
}

thread_local! {
    pub(crate) static FRAME_BUDGET_CFG: RefCell<FrameBudgetCfg> = RefCell::new(FrameBudgetCfg::default());
    /// Số lần quota yield của lần `run_frame_*` gần nhất (test/đo).
    pub(crate) static LAST_QUOTA_YIELDS: RefCell<u64> = const { RefCell::new(0) };
    /// **ST-290** — bảng phí do CLI đặt cho **một** lần chạy VM kế tiếp.
    static PHI_LAN_CHAY: RefCell<Option<FeeTable>> = const { RefCell::new(None) };
    /// Đơn vị ngân sách VM đã dùng trong lần chạy gần nhất (báo cáo/đo).
    static DUNG_GAN_NHAT: std::cell::Cell<i64> = const { std::cell::Cell::new(0) };
}

/// **ST-290 (VD-273)** — CLI đặt bảng phí ngữ nghĩa cho **một** lần chạy VM trên luồng này.
/// Bảng phí đi kèm module đã verify (`FeeTable` từ `compile_with_fee` hoặc cache có trailer).
#[doc(hidden)]
pub fn dat_phi_lan_chay(fee: FeeTable) {
    PHI_LAN_CHAY.with(|p| *p.borrow_mut() = Some(fee));
}

/// Đơn vị ngân sách VM đã dùng trong lần chạy gần nhất (0 nếu không bật).
#[doc(hidden)]
pub fn da_dung_gan_nhat() -> i64 {
    DUNG_GAN_NHAT.with(|n| n.get())
}

/// Lấy (một lần) bảng phí đã đặt cho lần chạy kế tiếp, kèm giới hạn ngân sách phiên.
/// `None` khi không có bảng phí hoặc phiên chạy `--unlimited-budget`.
fn phi_va_gioi_han() -> Option<(FeeTable, u64)> {
    let limit = vis_runtime::rt::fee::gioi_han_phien()?;
    let fee = PHI_LAN_CHAY.with(|p| p.borrow_mut().take())?;
    Some((fee, limit))
}

/// Bật ngân sách VM từ bảng phí của lần chạy nếu caller chưa đặt ngân sách tường minh.
pub(crate) fn bat_dau_ngan_sach_vm(vm: &mut Vm<'_>) {
    DUNG_GAN_NHAT.with(|n| n.set(0));
    if vm.budget_fee.is_none() {
        if let Some((fee, limit)) = phi_va_gioi_han() {
            vm.budget_enable(fee, limit);
        }
    }
}

/// Số lần quota yield của lần chạy frame VM gần nhất.
#[doc(hidden)]
pub fn frame_quota_yields_for_test() -> u64 {
    LAST_QUOTA_YIELDS.with(|n| *n.borrow())
}

/// Bật instrumentation + (tuỳ chọn) quota cho đường frame VM. `quota = None` ⇒
/// chỉ charge event/budget, không yield.
#[doc(hidden)]
pub fn frame_budget_enable_for_test(fee: FeeTable, budget: u64, quota: Option<u64>) {
    FRAME_BUDGET_CFG.with(|c| {
        *c.borrow_mut() = FrameBudgetCfg {
            fee: Some(fee),
            budget,
            quota,
        }
    });
}

/// Tắt cấu hình — trả về mặc định (không charge/không quota).
#[doc(hidden)]
pub fn frame_budget_disable_for_test() {
    FRAME_BUDGET_CFG.with(|c| *c.borrow_mut() = FrameBudgetCfg::default());
}

// **ST-276 (VD-256)** — test-hook ngân sách của VM; tách sang tệp này để giữ R1 ≤ 1500 dòng
// cho `vm.rs`. Mặc định `budget_fee == None` ⇒ `budget_charge` là no-op.
impl<'a> Vm<'a> {
    /// **ST-290 (VD-273)** — bật ngân sách ngữ nghĩa ở chế độ chạy **thường** (không lưu trace).
    /// Dùng chung với CLI (`40M` mặc định hoặc `--max-budget`); giữ API test qua wrapper dưới.
    pub fn budget_enable(&mut self, fee: FeeTable, limit: u64) {
        self.budget_fee = Some(fee);
        self.budget_remaining = limit.min(i64::MAX as u64) as i64;
        self.budget_trace.clear();
        self.budget_hit = false;
        self.budget_trace_on = false;
    }

    /// Bật ngân sách ngữ nghĩa bằng bảng phụ S (internal test hook).
    pub fn budget_enable_for_test(&mut self, fee: FeeTable, limit: u64) {
        self.budget_enable(fee, limit);
        self.budget_trace_on = true;
    }

    /// **VS3-1a** — bật ngân sách ở **chế độ đo**: vẫn trừ và trap như
    /// [`Self::budget_enable_for_test`] nhưng **không** lưu trace từng sự kiện. Dùng cho workload
    /// lớn (vòng 2M ≈ 12M sự kiện) để test hợp đồng fuel/ngân sách không cấp phát trace khổng lồ.
    pub fn budget_enable_measure_for_test(&mut self, fee: FeeTable, limit: u64) {
        self.budget_enable(fee, limit);
    }

    /// Dãy sự kiện **có thứ tự** đã charge (so với trace của compiler/interp).
    pub fn budget_trace_for_test(&self) -> &[FeeEntry] {
        &self.budget_trace
    }

    // ---- VD-257/D — quota lượt (RIÊNG khỏi budget tổng, test-only) ----

    /// Bật quota với hạn `limit`. Không đụng `budget_*`.
    pub(crate) fn quota_setup_for_test(&mut self, limit: u64) {
        // **VD-259**: `u64 → i64` phải bão hoà; cast thẳng làm hạn lớn quay âm ⇒ mọi turn
        // "hết quota" ngay.
        let limit = limit.min(i64::MAX as u64) as i64;
        self.quota_active = true;
        self.quota_limit = limit;
        self.quota_remaining = limit;
        self.quota_hit = false;
        self.quota_yields = 0;
    }

    /// Reset quota **đầu mỗi turn** (không reset budget).
    pub(crate) fn quota_reset_for_test(&mut self) {
        if self.quota_active {
            self.quota_remaining = self.quota_limit;
            self.quota_hit = false;
        }
    }

    /// Trừ quota bởi CÙNG `w` với budget (một sự kiện, hai bộ đếm). Không trap.
    pub(crate) fn quota_charge(&mut self, w: i64) {
        if self.quota_active {
            self.quota_remaining -= w;
            if self.quota_remaining <= 0 {
                self.quota_hit = true;
            }
        }
    }

    /// Lấy (và xoá) cờ hết quota tại continuation point.
    pub(crate) fn quota_take_hit(&mut self) -> bool {
        if !self.quota_active || !self.quota_hit {
            return false;
        }
        self.quota_hit = false;
        self.quota_yields += 1;
        true
    }

    /// Số lần yield do quota (test/đo).
    #[doc(hidden)]
    pub fn quota_yields_for_test(&self) -> u64 {
        self.quota_yields
    }

    /// Charge **mọi** sự kiện tại `(hàm, pc)` theo thứ tự bảng phụ. D6: pc lưu sau charge ⇒
    /// resume không charge lại nhóm cũ.
    pub(crate) fn budget_charge(&mut self, fi: u32, pc: usize, span: Span) -> R<()> {
        if let Some(a) = self.worker_tick(span) {
            return Err(a);
        }
        if self.budget_fee.is_none() {
            return Ok(());
        }
        // `MAX_PER_PC` = 16 (đã verify): gom tại chỗ, không cấp phát `Vec` mỗi lệnh — đường chạy
        // mặc định (ngân sách 40M) charge hàng chục triệu lần.
        let mut ev: [Option<FeeEntry>; vis_bytecode::fee::MAX_PER_PC] =
            [None; vis_bytecode::fee::MAX_PER_PC];
        let mut n = 0usize;
        if let Some(v) = self
            .budget_fee
            .as_ref()
            .and_then(|f| f.funcs.get(fi as usize))
        {
            for e in v.iter().filter(|e| e.pc as usize == pc) {
                if n == ev.len() {
                    break;
                }
                ev[n] = Some(*e);
                n += 1;
            }
        }
        if n == 0 {
            return Ok(());
        }
        for e in ev[..n].iter().flatten() {
            let w = vis_bytecode::fee::standard_weight(e.kind) as i64;
            if self.budget_trace_on {
                self.budget_trace.push(*e);
            }
            if w > self.budget_remaining {
                self.budget_hit = true;
                return Err(Abort::Trap(Trap::new(
                    BUDGET,
                    format!(
                        "vượt ngân sách ngữ nghĩa ({}): cần {}, còn {} — dùng `--max-budget <N>` hoặc `--unlimited-budget`",
                        e.kind.name(),
                        w,
                        self.budget_remaining
                    ),
                    span,
                )));
            }
            self.budget_remaining -= w;
            DUNG_GAN_NHAT.with(|n| n.set(n.get() + w));
            // VD-257/D — MỘT sự kiện trừ CẢ HAI bộ đếm; quota KHÔNG trap.
            self.quota_charge(w);
        }
        Ok(())
    }
}
