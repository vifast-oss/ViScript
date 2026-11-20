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

/// Bật instrumentation + ngưỡng (internal test hook). Mặc định `false` ⇒ mọi `charge` là no-op.
pub fn enable_for_test(limit: u64) {
    BUDGET.with(|b| {
        let mut b = b.borrow_mut();
        b.enabled = true;
        b.remaining = limit as i64;
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
        b.quota_limit = limit as i64;
        b.quota_remaining = limit as i64;
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
        b.remaining = limit as i64;
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
