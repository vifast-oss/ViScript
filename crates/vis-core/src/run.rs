// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-312a (VD-289)** — kết quả một lần chạy, dùng chung cho interp & VM.
//!
//! Chuyển **nguyên văn** từ `vis-interp/src/lib.rs` để `vis-vm` không phải đi qua
//! `vis-interp` chỉ vì kiểu trả về. Không đổi trường/ngữ nghĩa.

use vis_syntax::Diagnostic;

use crate::value::Value;

/// Kết quả một lần chạy.
#[derive(Debug, Clone, Default)]
pub struct RunOutput {
    /// Toàn bộ `stdout` đã in (`print`).
    pub stdout: String,
    /// Chẩn đoán runtime (`VS05xx`), tối đa một cái — lỗi đầu tiên dừng chương trình.
    pub diagnostics: Vec<Diagnostic>,
    /// Giá trị `main` trả (hoặc biểu thức cấp tệp cuối).
    pub value: Option<Value>,
    /// Mã thoát: `0` nếu sạch; `main` trả `Result::Err` ⇒ `1`; `int` của `main` **không**
    /// phải mã thoát (VD-242 §7 / quyết định A0 2026-11-19).
    pub exit: i32,
    /// Số bước đã chạy — **chống xanh-giả** trong test (`out("")` vẫn phải chạy thật).
    pub steps: usize,
}

impl RunOutput {
    /// `true` nếu không có chẩn đoán.
    pub fn is_clean(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Mã chẩn đoán theo thứ tự.
    pub fn codes(&self) -> Vec<&'static str> {
        self.diagnostics.iter().map(|d| d.code.short).collect()
    }
}
