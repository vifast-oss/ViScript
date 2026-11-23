// SPDX-License-Identifier: MIT OR Apache-2.0
//! Backend ViTerm ([`ViTermFramed`]): giao thức **JSON Lines** `vish-term/1`.
//!
//! Mỗi khung là **một dòng JSON** kết thúc `\n`, khoá theo thứ tự cố định để snapshot ổn định.
//! Dữ liệu có cấu trúc (`table`/`record`/`diag`) đi **nguyên khung** — ViTerm tự quyết định cách
//! hiển thị, không bị hạ thành văn bản như backend ANSI.
//!
//! Khung:
//! - `hello` — `proto`, `caps`
//! - `text` — `s`
//! - `styled` — `style`, `s`
//! - `table` — `cols`, `rows`
//! - `record` — `fields`
//! - `diag` — `severity`, `code`, `message`, `location`, `notes`
//! - `clear` — `what`
//! - `cursor` — `op`, `n` (tuỳ op)
//! - `flush`, `bye`

use std::io::{self, Write};

use crate::{Capabilities, Color, ColorMode, Diagnostic, Record, Style, Table, Terminal};

/// Phiên bản giao thức ViTerm do crate này phát.
pub const PROTOCOL: &str = "vish-term/1";

/// Terminal phát khung JSON Lines cho ViTerm.
pub struct ViTermFramed<W: Write> {
    out: W,
    caps: Capabilities,
}

impl<W: Write> ViTermFramed<W> {
    /// Terminal ViTerm trên `out`; khả năng mặc định là [`Capabilities::structured`].
    pub fn new(out: W) -> Self {
        Self {
            out,
            caps: Capabilities::structured(),
        }
    }

    /// Ghi đè khả năng (ví dụ ViTerm chạy trên thiết bị ít màu); `structured` nên giữ `true`.
    pub fn with_caps(mut self, caps: Capabilities) -> Self {
        self.caps = caps;
        self
    }

    /// Writer bên trong.
    pub fn get_ref(&self) -> &W {
        &self.out
    }

    /// Writer bên trong (mutable).
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.out
    }

    /// Lấy lại writer.
    pub fn into_inner(self) -> W {
        self.out
    }

    /// Khung `hello` — nên gửi **đầu tiên** trên một kết nối ViTerm.
    pub fn send_hello(&mut self) -> io::Result<()> {
        let caps = self.caps;
        let body = format!(
            "{{\"v\":1,\"t\":\"hello\",\"proto\":\"{}\",\"caps\":{{\"color\":\"{}\",\"unicode\":{},\"cursor\":{},\"structured\":{}}}}}",
            PROTOCOL,
            color_mode_name(caps.color),
            caps.unicode,
            caps.cursor,
            caps.structured
        );
        self.emit(&body)
    }

    /// Khung `text`.
    pub fn send_text(&mut self, s: &str) -> io::Result<()> {
        self.emit(&format!("{{\"v\":1,\"t\":\"text\",\"s\":{}}}", json_str(s)))
    }

    /// Khung `styled`.
    pub fn send_styled(&mut self, style: Style, s: &str) -> io::Result<()> {
        self.emit(&format!(
            "{{\"v\":1,\"t\":\"styled\",\"style\":{},\"s\":{}}}",
            style_json(style),
            json_str(s)
        ))
    }

    /// Khung `table` — gửi **nguyên** cột/hàng.
    pub fn send_table(&mut self, table: &Table) -> io::Result<()> {
        let mut body = String::from("{\"v\":1,\"t\":\"table\",\"cols\":[");
        for (i, col) in table.columns.iter().enumerate() {
            if i > 0 {
                body.push(',');
            }
            body.push_str(&format!(
                "{{\"title\":{},\"align\":\"{}\"}}",
                json_str(&col.title),
                align_name(col.align)
            ));
        }
        body.push_str("],\"rows\":[");
        for (i, row) in table.rows.iter().enumerate() {
            if i > 0 {
                body.push(',');
            }
            body.push('[');
            for (j, cell) in row.iter().enumerate() {
                if j > 0 {
                    body.push(',');
                }
                body.push_str(&json_str(cell));
            }
            body.push(']');
        }
        body.push_str("]}");
        self.emit(&body)
    }

    /// Khung `record` — gửi **nguyên** các cặp khoá–giá trị.
    pub fn send_record(&mut self, record: &Record) -> io::Result<()> {
        let mut body = String::from("{\"v\":1,\"t\":\"record\",\"fields\":[");
        for (i, (key, value)) in record.fields.iter().enumerate() {
            if i > 0 {
                body.push(',');
            }
            body.push('[');
            body.push_str(&json_str(key));
            body.push(',');
            body.push_str(&json_str(value));
            body.push(']');
        }
        body.push_str("]}");
        self.emit(&body)
    }

    /// Khung `diag` — gửi **nguyên** chẩn đoán.
    pub fn send_diagnostic(&mut self, diagnostic: &Diagnostic) -> io::Result<()> {
        let code = diagnostic
            .code
            .as_deref()
            .map_or_else(|| "null".to_string(), json_str);
        let location = diagnostic.location.as_ref().map_or_else(
            || "null".to_string(),
            |loc| {
                format!(
                    "{{\"path\":{},\"line\":{},\"col\":{}}}",
                    json_str(&loc.path),
                    loc.line,
                    loc.col
                )
            },
        );
        let notes = diagnostic
            .notes
            .iter()
            .map(|note| json_str(note))
            .collect::<Vec<_>>()
            .join(",");
        let body = format!(
            "{{\"v\":1,\"t\":\"diag\",\"severity\":\"{}\",\"code\":{},\"message\":{},\"location\":{},\"notes\":[{}]}}",
            diagnostic.severity.label(),
            code,
            json_str(&diagnostic.message),
            location,
            notes
        );
        self.emit(&body)
    }

    /// Khung `clear` cho dòng hiện tại.
    pub fn send_clear_line(&mut self) -> io::Result<()> {
        self.emit("{\"v\":1,\"t\":\"clear\",\"what\":\"line\"}")
    }

    /// Khung `cursor` dịch lên `n` dòng.
    pub fn send_move_up(&mut self, n: u16) -> io::Result<()> {
        self.emit(&format!(
            "{{\"v\":1,\"t\":\"cursor\",\"op\":\"up\",\"n\":{n}}}"
        ))
    }

    /// Khung `cursor` hiện/ẩn con trỏ.
    pub fn send_show_cursor(&mut self, show: bool) -> io::Result<()> {
        let op = if show { "show" } else { "hide" };
        self.emit(&format!("{{\"v\":1,\"t\":\"cursor\",\"op\":\"{op}\"}}"))
    }

    /// Khung `flush`.
    pub fn send_flush(&mut self) -> io::Result<()> {
        self.emit("{\"v\":1,\"t\":\"flush\"}")
    }

    /// Khung `bye` + đẩy bộ đệm.
    pub fn send_bye(&mut self) -> io::Result<()> {
        self.emit("{\"v\":1,\"t\":\"bye\"}")?;
        self.out.flush()
    }

    /// Ghi một khung đã dựng sẵn kèm `\n`.
    fn emit(&mut self, body: &str) -> io::Result<()> {
        self.out.write_all(body.as_bytes())?;
        self.out.write_all(b"\n")
    }
}

impl<W: Write> Terminal for ViTermFramed<W> {
    fn caps(&self) -> Capabilities {
        self.caps
    }

    fn write(&mut self, s: &str) -> io::Result<()> {
        self.send_text(s)
    }

    fn write_line(&mut self, s: &str) -> io::Result<()> {
        let mut line = String::with_capacity(s.len() + 1);
        line.push_str(s);
        line.push('\n');
        self.send_text(&line)
    }

    fn write_styled(&mut self, style: Style, s: &str) -> io::Result<()> {
        self.send_styled(style, s)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.send_flush()?;
        self.out.flush()
    }

    fn clear_line(&mut self) -> io::Result<()> {
        self.send_clear_line()
    }

    fn move_up(&mut self, n: u16) -> io::Result<()> {
        self.send_move_up(n)
    }

    fn show_cursor(&mut self, show: bool) -> io::Result<()> {
        self.send_show_cursor(show)
    }

    fn table(&mut self, table: &Table) -> io::Result<()> {
        self.send_table(table)
    }

    fn record(&mut self, record: &Record) -> io::Result<()> {
        self.send_record(record)
    }

    fn diagnostic(&mut self, diagnostic: &Diagnostic) -> io::Result<()> {
        self.send_diagnostic(diagnostic)
    }
}

/// Tên mức màu trong khung `hello`.
fn color_mode_name(mode: ColorMode) -> &'static str {
    match mode {
        ColorMode::None => "none",
        ColorMode::Ansi16 => "ansi16",
        ColorMode::Ansi256 => "ansi256",
        ColorMode::TrueColor => "truecolor",
    }
}

/// Tên căn cột trong khung `table`.
fn align_name(align: crate::Align) -> &'static str {
    match align {
        crate::Align::Left => "left",
        crate::Align::Right => "right",
    }
}

/// Mã hoá một chuỗi thành literal JSON (có nháy kép).
fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Mã hoá kiểu thành object JSON.
fn style_json(style: Style) -> String {
    let fg = style.fg.map_or_else(|| "null".to_string(), color_json);
    let bg = style.bg.map_or_else(|| "null".to_string(), color_json);
    let mut attrs: Vec<&str> = Vec::new();
    if style.attrs.bold {
        attrs.push("\"bold\"");
    }
    if style.attrs.dim {
        attrs.push("\"dim\"");
    }
    if style.attrs.italic {
        attrs.push("\"italic\"");
    }
    if style.attrs.underline {
        attrs.push("\"underline\"");
    }
    if style.attrs.reverse {
        attrs.push("\"reverse\"");
    }
    format!(
        "{{\"fg\":{fg},\"bg\":{bg},\"attrs\":[{}]}}",
        attrs.join(",")
    )
}

/// Mã hoá màu thành chuỗi/object JSON.
fn color_json(color: Color) -> String {
    match color {
        Color::Black => "\"black\"".to_string(),
        Color::Red => "\"red\"".to_string(),
        Color::Green => "\"green\"".to_string(),
        Color::Yellow => "\"yellow\"".to_string(),
        Color::Blue => "\"blue\"".to_string(),
        Color::Magenta => "\"magenta\"".to_string(),
        Color::Cyan => "\"cyan\"".to_string(),
        Color::White => "\"white\"".to_string(),
        Color::BrightBlack => "\"bright_black\"".to_string(),
        Color::BrightRed => "\"bright_red\"".to_string(),
        Color::BrightGreen => "\"bright_green\"".to_string(),
        Color::BrightYellow => "\"bright_yellow\"".to_string(),
        Color::BrightBlue => "\"bright_blue\"".to_string(),
        Color::BrightMagenta => "\"bright_magenta\"".to_string(),
        Color::BrightCyan => "\"bright_cyan\"".to_string(),
        Color::BrightWhite => "\"bright_white\"".to_string(),
        Color::Ansi256(n) => format!("{{\"ansi256\":{n}}}"),
        Color::Rgb(r, g, b) => format!("{{\"rgb\":[{r},{g},{b}]}}"),
    }
}
