// SPDX-License-Identifier: MIT OR Apache-2.0
// **ST-276 / VD-256**: module nội bộ của `vis-interp` dùng **cùng một nguồn định nghĩa sự kiện**
// với `vis-bytecode`. Dùng `include!` để giữ Cargo.toml/Cargo.lock không đổi (một file, một định
// nghĩa — compiler và interpreter không tự định nghĩa riêng rồi "hy vọng trùng").
include!("../../../vis-bytecode/src/fee_shared.rs");

// ---------------------------------------------------------------------------
// Test-hook ngân sách — CHỈ internal, KHÔNG mặc định (VD-256 §5).
// Thread-local để vis-cli có thể bật/lấy trace mà không đổi `lib.rs`/API công khai khác.
// ---------------------------------------------------------------------------
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;

use vis_syntax::Span;

use crate::interp::{Abort, Interp};

#[derive(Default)]
struct Budget {
    enabled: bool,
    remaining: i64,
    trace: Vec<FeeEntry>,
    hit: bool,
    /// Chế độ đo (không lưu trace, chỉ đếm theo kind) — tránh cấp phát khổng lồ cho workload lớn.
    measure: bool,
    counts: [u64; 8],
    // -----------------------------------------------------------------------
    // VD-257 / stream D — hạn ngạch lượt (quota) THỬ NGHIỆM, CHỈ test-hook.
    // `quota_*` là bộ đếm RIÊNG, reset mỗi turn, hết ⇒ yield (KHÔNG trap). Nó
    // KHÔNG thay `remaining` (ngân sách tổng vẫn trap VS0501-BUDGET như cũ).
    // -----------------------------------------------------------------------
    /// Bật phát sự kiện trên **đường frame** (test-only). Tắt (mặc định) ⇒ frame
    /// path vẫn KHÔNG charge, nên test `f2_interp_frame_path_khong_tu_charge` của
    /// stream A vẫn đúng. Chỉ hook `*_for_test` mới bật; không CLI/API công.
    frame_emit: bool,
    /// Hạn ngạch có bật hay không. `false` ⇒ mọi thao tác quota là no-op.
    quota_enabled: bool,
    /// Giá trị reset mỗi turn (đơn vị chi phí chuẩn hoá `standard_weight`).
    quota_limit: i64,
    /// Phần còn lại **trong turn hiện tại**.
    quota_remaining: i64,
    /// Cờ "đã vượt quota trong turn này" — chỉ đọc tại continuation point.
    quota_hit: bool,
    /// Số lần yield do quota (để test/đo).
    quota_yields: u64,
}

thread_local! {
    static BUDGET: RefCell<Budget> = RefCell::new(Budget::default());
}

/// Trần `u64 → i64` an toàn: giá trị lớn hơn `i64::MAX` được **bão hoà**, không quay âm.
fn i64_sat(limit: u64) -> i64 {
    limit.min(i64::MAX as u64) as i64
}

/// Bật instrumentation + ngưỡng (internal test hook). Mặc định `false` ⇒ mọi `charge` là no-op.
pub fn enable_for_test(limit: u64) {
    BUDGET.with(|b| {
        let mut b = b.borrow_mut();
        b.enabled = true;
        b.remaining = i64_sat(limit);
        b.trace.clear();
        b.hit = false;
    });
}

/// Tắt instrumentation — trả trạng thái về mặc định (kể cả quota/frame-emit).
pub fn disable_for_test() {
    BUDGET.with(|b| *b.borrow_mut() = Budget::default());
}

// ---------------------------------------------------------------------------
// VD-257 / D — test-hook cho ĐƯỜNG FRAME + QUOTA. Mọi hàm ở đây là internal,
// mặc định `false`/`0` ⇒ runtime thường và mọi test khác không đổi hành vi.
// ---------------------------------------------------------------------------

/// Bật phát dãy sự kiện ngữ nghĩa trên **đường frame** (dùng chung `FeeKind`,
/// `standard_weight` và `charge()` — KHÔNG instrument hoá lần hai).
#[doc(hidden)]
pub fn frame_emit_enable_for_test() {
    BUDGET.with(|b| b.borrow_mut().frame_emit = true);
}

/// `true` khi đường frame được phép charge (được gọi tại mỗi điểm charge).
#[doc(hidden)]
pub fn frame_emit_enabled() -> bool {
    BUDGET.with(|b| b.borrow().frame_emit)
}

/// Charge trên đường frame **chỉ khi** hook frame bật — dùng từ `frame/expr.rs`
/// và `frame/proc.rs`. Gọi lại `Interp::charge` ⇒ cùng trap `VS0501-BUDGET` với
/// đường trực tiếp; kiểm ngân sách nằm ngay trước điểm charge (trước side-effect).
#[doc(hidden)]
pub(crate) fn frame_charge(it: &mut Interp<'_>, kind: FeeKind, span: Span) -> Option<Abort> {
    if !frame_emit_enabled() {
        return None;
    }
    it.charge(kind, span)
}

/// Bật quota thử nghiệm với hạn `limit` (đơn vị chi phí chuẩn hoá).
#[doc(hidden)]
pub fn quota_enable_for_test(limit: u64) {
    BUDGET.with(|b| {
        let mut b = b.borrow_mut();
        b.quota_enabled = true;
        b.quota_limit = i64_sat(limit);
        b.quota_remaining = i64_sat(limit);
        b.quota_hit = false;
        b.quota_yields = 0;
    });
}

/// Reset quota **đầu mỗi lượt chạy (turn)** — không reset budget tổng.
#[doc(hidden)]
pub fn quota_reset_for_test() {
    BUDGET.with(|b| {
        let mut b = b.borrow_mut();
        if b.quota_enabled {
            b.quota_remaining = b.quota_limit;
            b.quota_hit = false;
        }
    });
}

/// Lấy (và xoá) cờ hết quota tại continuation point. `false` khi không bật.
#[doc(hidden)]
pub fn quota_take_hit() -> bool {
    BUDGET.with(|b| {
        let mut b = b.borrow_mut();
        if !b.quota_enabled || !b.quota_hit {
            return false;
        }
        b.quota_hit = false;
        b.quota_yields += 1;
        true
    })
}

/// Số lần đã yield do quota (test/đo).
#[doc(hidden)]
pub fn quota_yields_for_test() -> u64 {
    BUDGET.with(|b| b.borrow().quota_yields)
}

/// Bật chế độ **đo**: không lưu trace, chỉ đếm số sự kiện theo `kind.tag()`.
pub fn enable_measure_for_test(limit: u64) {
    BUDGET.with(|b| {
        let mut b = b.borrow_mut();
        b.enabled = true;
        b.remaining = i64_sat(limit);
        b.trace.clear();
        b.hit = false;
        b.measure = true;
        b.counts = [0; 8];
    });
}

/// Số sự kiện theo `kind` (`E_STMT` … `E_AWAIT`), lấy từ chế độ đo.
pub fn take_counts_for_test() -> [u64; 8] {
    BUDGET.with(|b| b.borrow().counts)
}

/// Lấy dãy sự kiện **có thứ tự** đã charge.
pub fn take_trace_for_test() -> Vec<FeeEntry> {
    BUDGET.with(|b| b.borrow().trace.clone())
}

/// Số công đã vượt (nếu có) — dùng cho chẩn đoán.
pub fn hit_for_test() -> bool {
    BUDGET.with(|b| b.borrow().hit)
}

/// Ghi một sự kiện. Trả `Some((w, remaining))` khi `w > remaining` (nguyên tử: không trừ,
/// không thi hành). Không đụng trọng số từ artifact (D5) — dùng bảng chuẩn có phiên bản.
pub fn charge(kind: FeeKind, site: u32) -> Option<(u64, i64)> {
    BUDGET.with(|b| {
        let mut b = b.borrow_mut();
        if !b.enabled {
            return None;
        }
        let w = standard_weight(kind);
        if b.measure {
            b.counts[kind.tag() as usize - 1] += 1;
        } else {
            b.trace.push(FeeEntry::new(0, kind, site));
        }
        let budget_over = w as i64 > b.remaining;
        if budget_over {
            b.hit = true;
        } else {
            b.remaining -= w as i64;
        }
        // VD-257/D — MỘT sự kiện, hai bộ đếm: quota bị trừ bởi CÙNG `w`, nhưng
        // KHÔNG BAO GIỜ trap. Khi vượt 0 chỉ đặt cờ; driver yield ở continuation
        // point. `remaining` ở trên vẫn giữ nguyên ngữ nghĩa ngân sách tổng.
        if b.quota_enabled {
            b.quota_remaining -= w as i64;
            if b.quota_remaining <= 0 {
                b.quota_hit = true;
            }
        }
        if budget_over {
            Some((w, b.remaining))
        } else {
            None
        }
    })
}

// ===========================================================================
// **VS3-3b (VD-271)** — ngân sách **atomic dùng chung** cho worker pool opt-in.
//
// Khác với `BUDGET` thread-local ở trên (test-hook của runtime một luồng), kiểu dưới đây
// là **một pool logic** cho nhiều worker. Mỗi worker/task giữ một `BudgetLease`: xin một
// **chunk** bằng `fetch_sub`/CAS từ pool chung rồi trừ cục bộ, trả phần dư khi xong —
// giảm tranh chấp CAS so với charge từng đơn vị. Không có setter/reset sau khi tạo.
// ===========================================================================

/// Snapshot cuối của [`SharedBudget`] (đọc sau khi mọi lease đã drop).
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BudgetSnapshot {
    pub limit: i64,
    pub used: i64,
    pub remaining: i64,
    pub exhausted: bool,
}

/// Lỗi charge: pool đã cạn (kể cả chunk cục bộ).
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BudgetError {
    Exhausted { remaining: i64 },
}

/// **VS3-3b**: pool ngân sách atomic dùng chung; default **không** bật (không ai tạo nó
/// ngoài worker pool opt-in/test).
#[doc(hidden)]
#[derive(Debug)]
pub struct SharedBudget {
    limit: i64,
    chunk: i64,
    remaining: AtomicI64,
    used: AtomicI64,
    exhausted: AtomicBool,
}

impl SharedBudget {
    /// `limit` là tổng đơn vị; `chunk` là kích thước mỗi lần reserve (0 ⇒ 1).
    pub fn new(limit: u64, chunk: u64) -> Self {
        let limit = i64_sat(limit);
        let chunk = i64_sat(chunk).max(1).min(limit.max(1));
        Self {
            limit,
            chunk,
            remaining: AtomicI64::new(limit),
            used: AtomicI64::new(0),
            exhausted: AtomicBool::new(false),
        }
    }

    pub fn limit(&self) -> i64 {
        self.limit
    }

    pub fn chunk(&self) -> i64 {
        self.chunk
    }

    /// Xin một block từ pool; `None` ⇒ cạn (không reset). CAS để không âm.
    fn reserve(&self) -> Option<i64> {
        let mut cur = self.remaining.load(Ordering::Relaxed);
        loop {
            if cur <= 0 {
                self.exhausted.store(true, Ordering::Relaxed);
                return None;
            }
            let take = cur.min(self.chunk);
            match self.remaining.compare_exchange_weak(
                cur,
                cur - take,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Some(take),
                Err(actual) => cur = actual,
            }
        }
    }

    fn refund(&self, n: i64) {
        if n > 0 {
            self.remaining.fetch_add(n, Ordering::AcqRel);
        }
    }

    fn record_used(&self, n: i64) {
        if n > 0 {
            self.used.fetch_add(n, Ordering::Relaxed);
        }
    }

    pub fn snapshot(&self) -> BudgetSnapshot {
        BudgetSnapshot {
            limit: self.limit,
            used: self.used.load(Ordering::Acquire),
            remaining: self.remaining.load(Ordering::Acquire),
            exhausted: self.exhausted.load(Ordering::Acquire),
        }
    }
}

/// Lease của một worker/task: reserve chunk từ [`SharedBudget`], charge cục bộ, trả dư khi drop.
#[doc(hidden)]
pub struct BudgetLease {
    global: Arc<SharedBudget>,
    local: i64,
    used: i64,
}

impl BudgetLease {
    pub fn new(global: Arc<SharedBudget>) -> Self {
        let local = global.reserve().unwrap_or(0);
        Self {
            global,
            local,
            used: 0,
        }
    }

    /// Trừ `weight`; tự reserve thêm chunk khi cục bộ không đủ. `Err` ⇒ pool cạn.
    pub fn charge(&mut self, weight: i64) -> Result<(), BudgetError> {
        if weight <= 0 {
            return Ok(());
        }
        while self.local < weight {
            match self.global.reserve() {
                Some(n) => self.local += n,
                None => {
                    return Err(BudgetError::Exhausted {
                        remaining: self.local,
                    })
                }
            }
        }
        self.local -= weight;
        self.used += weight;
        self.global.record_used(weight);
        Ok(())
    }

    pub fn used(&self) -> i64 {
        self.used
    }
}

impl Drop for BudgetLease {
    fn drop(&mut self) {
        self.global.refund(self.local);
        self.local = 0;
    }
}
