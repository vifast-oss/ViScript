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

#[derive(Default)]
struct Budget {
    enabled: bool,
    remaining: i64,
    trace: Vec<FeeEntry>,
    hit: bool,
    /// Chế độ đo (không lưu trace, chỉ đếm theo kind) — tránh cấp phát khổng lồ cho workload lớn.
    measure: bool,
    counts: [u64; 8],
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

/// Tắt instrumentation — trả trạng thái về mặc định.
pub fn disable_for_test() {
    BUDGET.with(|b| *b.borrow_mut() = Budget::default());
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
        if w as i64 > b.remaining {
            b.hit = true;
            return Some((w, b.remaining));
        }
        b.remaining -= w as i64;
        None
    })
}
