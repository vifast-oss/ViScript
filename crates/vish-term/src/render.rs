// SPDX-License-Identifier: MIT OR Apache-2.0
//! Mô hình hiển thị **có cấu trúc** và bản hạ văn bản mặc định.
//!
//! [`Table`] (bảng), [`Record`] (khối khoá–giá trị), [`Diagnostic`] (lỗi/cảnh báo) là dữ liệu
//! thuần, không phụ thuộc backend. [`Terminal::table`] và hai hook còn lại
//! dùng các hàm trong module này làm default; backend ViTerm ghi đè để gửi nguyên khung.
//!
//! Bản hạ văn bản giữ **đúng quy tắc `Render` ST-41**: cột cách nhau 2 dấu cách, cột số căn phải,
//! độ rộng theo NFC, không để khoảng trắng thừa cuối dòng.

use std::io;

use crate::width::display_width;
use crate::{Color, Style, Terminal};

/// Căn cột.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    /// Căn trái (mặc định).
    #[default]
    Left,
    /// Căn phải (số/kích thước/thời lượng).
    Right,
}

/// Một cột bảng.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Column {
    pub title: String,
    pub align: Align,
}

impl Column {
    /// Cột căn trái.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            align: Align::Left,
        }
    }

    /// Cột căn phải.
    pub fn right(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            align: Align::Right,
        }
    }
}

/// Bảng có cấu trúc: tiêu đề cột + hàng ô.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Table {
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<String>>,
    /// Vẽ dòng kẻ `----` dưới tiêu đề.
    pub rule: bool,
}

impl Table {
    /// Bảng với các cột cho trước, chưa có hàng, không kẻ.
    pub fn new(columns: Vec<Column>) -> Self {
        Self {
            columns,
            rows: Vec::new(),
            rule: false,
        }
    }

    /// Bật/tắt dòng kẻ dưới tiêu đề.
    pub fn with_rule(mut self, rule: bool) -> Self {
        self.rule = rule;
        self
    }

    /// Thêm một hàng; ô thiếu được hiển thị rỗng, ô thừa bị bỏ khi hạ văn bản.
    pub fn push_row(&mut self, row: Vec<String>) -> &mut Self {
        self.rows.push(row);
        self
    }

    /// Không có cột nào ⇒ bảng rỗng, hạ văn bản không ghi gì.
    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }

    /// Số hàng.
    pub fn len(&self) -> usize {
        self.rows.len()
    }
}

/// Khối khoá–giá trị của **một** bản ghi.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Record {
    pub fields: Vec<(String, String)>,
}

impl Record {
    /// Bản ghi rỗng.
    pub fn new() -> Self {
        Self::default()
    }

    /// Thêm trường, giữ thứ tự chèn.
    pub fn push(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.fields.push((key.into(), value.into()));
        self
    }

    /// Không có trường nào.
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}

/// Mức chẩn đoán.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Severity {
    /// Lỗi.
    #[default]
    Error,
    /// Cảnh báo.
    Warning,
    /// Ghi chú.
    Note,
}

impl Severity {
    /// Nhãn ổn định dùng trong văn bản và khung ViTerm.
    pub fn label(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Note => "note",
        }
    }
}

/// Vị trí nguồn của chẩn đoán (1-based).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
    pub path: String,
    pub line: u32,
    pub col: u32,
}

impl Location {
    /// Vị trí mới.
    pub fn new(path: impl Into<String>, line: u32, col: u32) -> Self {
        Self {
            path: path.into(),
            line,
            col,
        }
    }
}

/// Chẩn đoán có cấu trúc: mức, mã, thông điệp, vị trí, ghi chú.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: Option<String>,
    pub message: String,
    pub location: Option<Location>,
    pub notes: Vec<String>,
}

impl Diagnostic {
    /// Chẩn đoán tối thiểu (không mã/vị trí/ghi chú).
    pub fn new(severity: Severity, message: impl Into<String>) -> Self {
        Self {
            severity,
            code: None,
            message: message.into(),
            location: None,
            notes: Vec::new(),
        }
    }

    /// Gắn mã lỗi (`VS####`).
    pub fn code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Gắn vị trí nguồn.
    pub fn at(mut self, path: impl Into<String>, line: u32, col: u32) -> Self {
        self.location = Some(Location::new(path, line, col));
        self
    }

    /// Thêm ghi chú.
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
}

/// Hạ bảng thành văn bản có kiểu; ghi rỗng nếu không có cột.
pub fn table<T: Terminal + ?Sized>(term: &mut T, table: &Table) -> io::Result<()> {
    if table.columns.is_empty() {
        return Ok(());
    }
    let widths = column_widths(table);
    let header = table.columns.iter().map(|c| (c.title.as_str(), c.align));
    write_row(term, header, &widths, Some(Style::BOLD))?;
    if table.rule {
        let rule = widths
            .iter()
            .map(|w| "-".repeat(*w))
            .collect::<Vec<_>>()
            .join("  ");
        term.write_line(rule.trim_end())?;
    }
    for row in &table.rows {
        let cells = (0..table.columns.len()).map(|j| {
            (
                row.get(j).map(String::as_str).unwrap_or(""),
                table.columns[j].align,
            )
        });
        write_row(term, cells, &widths, None)?;
    }
    Ok(())
}

/// Hạ bản ghi thành khối khoá–giá trị; ghi rỗng nếu không có trường.
pub fn record<T: Terminal + ?Sized>(term: &mut T, record: &Record) -> io::Result<()> {
    if record.fields.is_empty() {
        return Ok(());
    }
    let width = record
        .fields
        .iter()
        .map(|(k, _)| display_width(k))
        .max()
        .unwrap_or(0);
    for (key, value) in &record.fields {
        term.write_styled(Style::DIM, key)?;
        let pad = width.saturating_sub(display_width(key));
        if pad > 0 {
            term.write(&" ".repeat(pad))?;
        }
        term.write("  ")?;
        term.write(value)?;
        term.write_line("")?;
    }
    Ok(())
}

/// Hạ chẩn đoán thành văn bản: dòng tiêu đề + vị trí (`-->`) + ghi chú (`=`).
pub fn diagnostic<T: Terminal + ?Sized>(term: &mut T, diagnostic: &Diagnostic) -> io::Result<()> {
    let color = match diagnostic.severity {
        Severity::Error => Color::Red,
        Severity::Warning => Color::Yellow,
        Severity::Note => Color::Cyan,
    };
    term.write_styled(Style::new().fg(color), diagnostic.severity.label())?;
    if let Some(code) = &diagnostic.code {
        term.write("[")?;
        term.write_styled(Style::BOLD, code)?;
        term.write("]")?;
    }
    term.write(": ")?;
    term.write(&diagnostic.message)?;
    term.write_line("")?;
    if let Some(loc) = &diagnostic.location {
        term.write("  ")?;
        term.write_styled(Style::DIM, "-->")?;
        term.write(" ")?;
        term.write(&format!("{}:{}:{}", loc.path, loc.line, loc.col))?;
        term.write_line("")?;
    }
    for note in &diagnostic.notes {
        term.write("  = ")?;
        term.write(note)?;
        term.write_line("")?;
    }
    Ok(())
}

/// Độ rộng từng cột: max(tiêu đề, mọi ô cùng cột).
fn column_widths(table: &Table) -> Vec<usize> {
    let mut widths: Vec<usize> = table
        .columns
        .iter()
        .map(|c| display_width(&c.title))
        .collect();
    for row in &table.rows {
        for (j, width) in widths.iter_mut().enumerate() {
            let cell = row.get(j).map(String::as_str).unwrap_or("");
            *width = (*width).max(display_width(cell));
        }
    }
    widths
}

/// Ghi một hàng: bỏ ô rỗng ở cuối, không để khoảng trắng thừa cuối dòng.
fn write_row<'a, T, I>(
    term: &mut T,
    cells: I,
    widths: &[usize],
    style: Option<Style>,
) -> io::Result<()>
where
    T: Terminal + ?Sized,
    I: IntoIterator<Item = (&'a str, Align)>,
{
    let cells: Vec<(&str, Align)> = cells.into_iter().collect();
    let last = cells.iter().rposition(|(text, _)| !text.is_empty());
    if let Some(last) = last {
        for (j, (text, align)) in cells.iter().enumerate().take(last + 1) {
            if j > 0 {
                term.write("  ")?;
            }
            let width = widths.get(j).copied().unwrap_or(0);
            let pad = width.saturating_sub(display_width(text));
            let is_last = j == last;
            match align {
                Align::Right => {
                    if pad > 0 {
                        term.write(&" ".repeat(pad))?;
                    }
                    write_cell(term, text, style)?;
                }
                Align::Left => {
                    write_cell(term, text, style)?;
                    if pad > 0 && !is_last {
                        term.write(&" ".repeat(pad))?;
                    }
                }
            }
        }
    }
    term.write_line("")
}

/// Ghi ô với kiểu tuỳ chọn.
fn write_cell<T: Terminal + ?Sized>(
    term: &mut T,
    text: &str,
    style: Option<Style>,
) -> io::Result<()> {
    match style {
        Some(style) => term.write_styled(style, text),
        None => term.write(text),
    }
}
