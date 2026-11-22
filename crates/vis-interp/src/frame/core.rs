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

/// **ST-290 (VD-273)** — `VS0510`: vượt **ngân sách thực thi ngữ nghĩa**, tách hẳn khỏi
/// `VS0501` (trap nội bộ) và khỏi fuel `SAFETY_FUEL` của VM. Trước VD-273 đây là mã test-hook
/// `VS0510`; nay là mã chính thức trong sổ runtime.
pub const BUDGET: &Code = &RUNTIME_CODES[9];

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
    /// **VD-262**: thi hành **mọi** `let`/`var` cấp tệp theo **thứ tự nguồn** — dùng chung cho
    /// đường interp thường và frame test, để biến cấp tệp có giá trị trước khi gọi `fn main`.
    ///
    /// Chỉ câu **khai báo** đi qua đây; câu lệnh cấp tệp khác (`print`, gán, `if`, …) **không**
    /// chạy khi tệp có `fn main` (giữ nguyên hành vi cũ: chúng chỉ chạy ở chế độ không `main`).
    /// Gọi hàm từ initializer vẫn thấy `self.env` (nơi `const` đã được đặt trước).
    pub(crate) fn khoi_tao_bien_cap_tep(&mut self) -> R<()> {
        for s in self.stmts {
            if !matches!(s, HStmt::Let { .. } | HStmt::Var { .. }) {
                continue;
            }
            self.last = None;
            match self.exec_stmt(s)? {
                Ctrl::Normal => {}
                Ctrl::Break | Ctrl::Continue => {
                    return Err(Abort::Trap(Trap::new(
                        RT,
                        "`break`/`continue` ngoài vòng lặp",
                        s.span(),
                    )))
                }
            }
        }
        Ok(())
    }

    /// Ghi một sự kiện ngữ nghĩa. `None` khi hook tắt (mặc định) — không đổi hành vi.
    /// **P2**: task worker dùng lease atomic trong `worker_budget` thay thread-local.
    pub(crate) fn charge(&mut self, kind: crate::rt::fee::FeeKind, span: Span) -> Option<Abort> {
        if let Some(b) = &mut self.worker_budget {
            let w = crate::rt::fee::standard_weight(kind) as i64;
            if b.charge(w).is_err() {
                return Some(Abort::Trap(Trap::new(
                    BUDGET,
                    "vượt ngân sách worker".to_string(),
                    span,
                )));
            }
            return None;
        }
        let (w, remaining) = crate::rt::fee::charge(kind, span.start as u32)?;
        Some(Abort::Trap(Trap::new(
            BUDGET,
            format!(
                "vượt ngân sách ngữ nghĩa ({}): cần {}, còn {} — dùng `--max-budget <N>` hoặc `--unlimited-budget`",
                kind.name(),
                w,
                remaining
            ),
            span,
        )))
    }
}
