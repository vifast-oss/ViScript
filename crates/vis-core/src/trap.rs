// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310a (VD-285)** — trap/abort runtime dùng chung.
//!
//! Chuyển **nguyên văn** từ `vis-interp/src/frame/core.rs` (khai báo) và
//! `vis-interp/src/interp.rs::pos` (tính dòng/cột) để `Value` (biến thể `Task`) và các trait
//! host có thể nằm ở `vis-core` mà không phụ thuộc ngược.

use vis_diag::Code;
use vis_syntax::{Diagnostic, Span};

use crate::value::{Pending, Value};

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
    Pending(Pending),
}

pub type R<T> = Result<T, Abort>;

/// Vị trí `dòng:cột` (cột đếm theo **Unicode scalar**, bắt đầu từ 1 — §2.4) từ `span` + văn bản.
pub fn pos(text: &str, span: Span) -> (u32, u32) {
    let mut line = 1u32;
    let mut col = 1u32;
    for (i, c) in text.char_indices() {
        if i >= span.start {
            break;
        }
        match c {
            '\n' => {
                line += 1;
                col = 1;
            }
            // ST-13b (mục 7): tab = **4 cột** (SPEC/VD-12), không phải 1.
            '\t' => col += 4,
            _ => col += 1,
        }
    }
    (line, col)
}
