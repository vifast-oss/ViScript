// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-313a (VD-289)** — charge ngân sách trên đường frame (interp-only).
//!
//! Chuyển nguyên văn từ `vis-interp/src/frame/fee.rs` (nay `vis-runtime::rt::fee`), vì hàm cần
//! `Interp::charge` — không thể nằm ở crate runtime.

use vis_core::trap::Abort;
use vis_syntax::Span;

use crate::interp::Interp;
use vis_runtime::rt::fee::{frame_emit_enabled, FeeKind};

/// Charge trên đường frame **chỉ khi** hook frame bật — dùng từ `frame/expr.rs`
/// và `frame/proc.rs`. Gọi lại `Interp::charge` ⇒ cùng trap `VS0510` với
/// đường trực tiếp; kiểm ngân sách nằm ngay trước điểm charge (trước side-effect).
#[doc(hidden)]
pub(crate) fn frame_charge(it: &mut Interp<'_>, kind: FeeKind, span: Span) -> Option<Abort> {
    if !frame_emit_enabled() {
        return None;
    }
    it.charge(kind, span)
}
