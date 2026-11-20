// SPDX-License-Identifier: MIT OR Apache-2.0
// **ST-276 (VD-256)**: prelude lõi của `interp` tách sang tệp riêng để giữ R1 ≤ 1500 dòng.
// Nội dung chuyển Nguyên văn; `interp.rs` `pub use` lại ⇒ API `crate::interp::*` không đổi.
use vis_diag::{Code, RUNTIME_CODES, TYPE_CODES};
use vis_syntax::{Diagnostic, Span};
use vis_types::hir::*;

use crate::interp::{pos, Interp};
use crate::value::Value;

/// `VS0501` — lỗi runtime chung.
pub const RT: &Code = &RUNTIME_CODES[0];
/// `VS0502` — chia/lấy dư cho 0.
pub const DIV0: &Code = &RUNTIME_CODES[1];
/// `VS0503` — chỉ số ngoài phạm vi.
pub const IDX: &Code = &RUNTIME_CODES[2];
/// **ST-60** — `VS0506`: lỗi trong **thân tác vụ** `async` (message kèm mã gốc).
pub const TASK_ERR: &Code = &RUNTIME_CODES[5];
/// **ST-61 (VD-96)** — `VS0504`: gửi vào kênh đã `close`.
pub const SEND_CLOSED: &Code = &RUNTIME_CODES[3];
/// **ST-61 (VD-96)** — `VS0505`: deadlock (kênh chặn, không còn tác vụ chạy được).
pub const DEADLOCK: &Code = &RUNTIME_CODES[4];
/// **R9 (VD-178)** — `VS0507`: tràn `i64` lúc chạy (tách khỏi `VS0501`).
pub const OVERFLOW: &Code = &RUNTIME_CODES[6];
/// `VS0412` — gán cho `let`/tham số (phòng thủ: tyck đã chặn trước).
pub(crate) const MISMATCH: &Code = &TYPE_CODES[3];

/// **ST-276 (VD-256)** — chẩn đoán **phân biệt** cho ngân sách ngữ nghĩa (chỉ test-hook).
/// Không đổi bảng `RUNTIME_CODES`; `short` riêng để không nhầm với `VS0501`/`VS0506`/fuel VM.
pub const BUDGET: &Code = &Code {
    id: 501,
    short: "VS0501-BUDGET",
    message_vi: "vượt ngân sách ngữ nghĩa (VD-256, test-hook)",
    message_en: "semantic budget exceeded (VD-256, test hook)",
    message_ja: "意味予算を超過（VD-256、テストフック）",
    severity: vis_diag::Severity::Error,
};

/// **ST-63 (VD-105 (6))**: phần **gọi hàm/closure** tách khỏi `interp.rs` để giữ **R1 ≤ 1.500 dòng**
/// — chuyển **nguyên văn**, không đổi ngữ nghĩa.
#[path = "../interp/goi.rs"]
pub(crate) mod goi;

/// Lỗi runtime kèm `span`.
#[derive(Debug, Clone)]
pub struct Trap {
    pub code: &'static Code,
    pub msg: String,
    pub span: Span,
}

impl Trap {
    pub fn new(code: &'static Code, msg: impl Into<String>, span: Span) -> Self {
        Self {
            code,
            msg: msg.into(),
            span,
        }
    }

    /// Chẩn đoán có **`line`/`col` thật** từ `span` + văn bản (VD-46).
    pub fn to_diag(&self, text: &str) -> Diagnostic {
        let (line, col) = pos(text, self.span);
        Diagnostic {
            code: self.code,
            message: self.msg.clone(),
            span: self.span,
            line,
            col,
        }
    }
}

/// Dừng chương trình: lỗi runtime, hoặc `return`/`?` trả sớm.
#[derive(Debug)]
pub enum Abort {
    Trap(Trap),
    Ret(Value),
    /// **ST-267/VD-251 (chỉ đường test/frame)**: frame hiện tại phải treo tại `await`.
    /// Runtime mặc định (`frame.enabled == false`) không bao giờ phát ra biến thể này.
    Pending(crate::rt::Pending),
}

pub type R<T> = Result<T, Abort>;

/// Đối số đã eval: (vị trí, có tên).
pub(crate) type Args = (Vec<Value>, Vec<(String, Value)>);

/// Nhánh `match` khớp + binding của nó.
pub(crate) type Picked<'s> = (&'s HArm, Vec<(u32, Value)>);

/// Luồng điều khiển trong thân: bình thường / `break` / `continue`.
#[derive(Debug, PartialEq)]
pub(crate) enum Ctrl {
    Normal,
    Break,
    Continue,
}

// **ST-276**: charge của Interp nằm cùng prelude này để giữ R1 ≤ 1500 dòng của `interp.rs`.
impl<'a> Interp<'a> {
    /// Ghi một sự kiện ngữ nghĩa. `None` khi hook tắt (mặc định) — không đổi hành vi.
    pub(crate) fn charge(&mut self, kind: crate::rt::fee::FeeKind, span: Span) -> Option<Abort> {
        let (w, remaining) = crate::rt::fee::charge(kind, span.start as u32)?;
        Some(Abort::Trap(Trap::new(
            BUDGET,
            format!(
                "vượt ngân sách ngữ nghĩa VD-256 ({}): cần {}, còn {}",
                kind.name(),
                w,
                remaining
            ),
            span,
        )))
    }
}
