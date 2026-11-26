// SPDX-License-Identifier: MIT OR Apache-2.0
//! Chẩn đoán + render: [`Diag`], [`DiagLine`], [`render`]. Tách nguyên văn từ `lib.rs` (ST-318d).

use super::codes::{Code, Lang, Severity};
use super::explain::severity_label;

/// Chẩn đoán tối thiểu: mã **đã cấp trong sổ** + thông điệp + vị trí byte (0-based).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diag {
    pub code: &'static str,
    pub msg: String,
    pub offset: usize,
}

impl Diag {
    /// Chỉ nhận `Code` lấy từ [`lookup`]/[`CODES`] — không có đường tạo mã ngoài sổ.
    pub fn new(code: &'static Code, msg: impl Into<String>, offset: usize) -> Self {
        Self {
            code: code.short,
            msg: msg.into(),
            offset,
        }
    }
}

/// Một chẩn đoán đã có vị trí **theo Unicode scalar** (1-based) để [`render`] hiển thị.
///
/// `col`/`line` do lexer/parser tính (lexer đếm `char`, tab = 4 — SPEC-lex §4); ST-03 chỉ
/// **render** chứ không tự đoán lại vị trí.
#[derive(Debug, Clone, Copy)]
pub struct DiagLine<'a> {
    pub path: &'a str,
    pub line: u32,
    pub col: u32,
    pub severity: Severity,
    pub code: &'a str,
    pub message: &'a str,
    /// Dòng nguồn (chưa có `\n`) để in con trỏ `^`; `None` ⇒ chỉ in dòng tiêu đề.
    pub source: Option<&'a str>,
}

/// Render một chẩn đoán: `<tệp>:<dòng>:<cột>: <mức> <mã>: <thông điệp>`; có `source` thì in
/// thêm dòng nguồn và con trỏ `^` **canh theo scalar** (tab tính 4 cột như lexer).
pub fn render(d: &DiagLine<'_>, lang: Lang) -> String {
    let mut ra = format!(
        "{}:{}:{}: {} {}: {}",
        d.path,
        d.line,
        d.col,
        severity_label(d.severity, lang),
        d.code,
        d.message
    );
    if let Some(src) = d.source {
        let src = src.trim_end_matches(['\n', '\r']);
        ra.push('\n');
        ra.push_str(src);
        // Con trỏ canh theo **scalar**, tab = 4 cột (khớp cách lexer đếm).
        let want = d.col.saturating_sub(1) as usize;
        let mut width = 0usize;
        for ch in src.chars() {
            if width >= want {
                break;
            }
            width += if ch == '\t' { 4 } else { 1 };
        }
        ra.push('\n');
        ra.push_str(&" ".repeat(width));
        ra.push('^');
    }
    ra
}
