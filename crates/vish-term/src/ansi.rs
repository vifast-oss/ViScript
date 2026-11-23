// SPDX-License-Identifier: MIT OR Apache-2.0
//! Mã hoá SGR từ [`Style`] theo [`ColorMode`] — dùng chung cho `AnsiPosix` và `WinConsole`.

use crate::{Color, ColorMode, Style};

/// Chuỗi SGR (`\x1b[…m`); rỗng nếu không có gì để phát hoặc backend không màu.
pub(crate) fn sgr(style: Style, mode: ColorMode) -> String {
    if mode == ColorMode::None || style.is_plain() {
        return String::new();
    }
    let mut parts: Vec<String> = Vec::new();
    let attrs = style.attrs;
    if attrs.bold {
        parts.push("1".to_string());
    }
    if attrs.dim {
        parts.push("2".to_string());
    }
    if attrs.italic {
        parts.push("3".to_string());
    }
    if attrs.underline {
        parts.push("4".to_string());
    }
    if attrs.reverse {
        parts.push("7".to_string());
    }
    if let Some(color) = style.fg {
        parts.push(color_code(color, mode, false));
    }
    if let Some(color) = style.bg {
        parts.push(color_code(color, mode, true));
    }
    if parts.is_empty() {
        return String::new();
    }
    format!("\x1b[{}m", parts.join(";"))
}

/// Mã màu cho chữ (`bg = false`) hoặc nền (`bg = true`), đã hạ cấp theo `mode`.
fn color_code(color: Color, mode: ColorMode, bg: bool) -> String {
    match color {
        Color::Black => basic(0, bg),
        Color::Red => basic(1, bg),
        Color::Green => basic(2, bg),
        Color::Yellow => basic(3, bg),
        Color::Blue => basic(4, bg),
        Color::Magenta => basic(5, bg),
        Color::Cyan => basic(6, bg),
        Color::White => basic(7, bg),
        Color::BrightBlack => basic(8, bg),
        Color::BrightRed => basic(9, bg),
        Color::BrightGreen => basic(10, bg),
        Color::BrightYellow => basic(11, bg),
        Color::BrightBlue => basic(12, bg),
        Color::BrightMagenta => basic(13, bg),
        Color::BrightCyan => basic(14, bg),
        Color::BrightWhite => basic(15, bg),
        Color::Ansi256(n) => match mode {
            ColorMode::Ansi16 => basic(ansi256_to_basic(n), bg),
            ColorMode::Ansi256 | ColorMode::TrueColor => {
                format!("{};5;{n}", if bg { 48 } else { 38 })
            }
            ColorMode::None => String::new(),
        },
        Color::Rgb(r, g, b) => match mode {
            ColorMode::Ansi16 => basic(rgb_to_basic(r, g, b), bg),
            ColorMode::Ansi256 => format!("{};5;{}", if bg { 48 } else { 38 }, rgb_to_256(r, g, b)),
            ColorMode::TrueColor => {
                format!("{};2;{r};{g};{b}", if bg { 48 } else { 38 })
            }
            ColorMode::None => String::new(),
        },
    }
}

/// Mã 30–37/90–97 (chữ) hoặc 40–47/100–107 (nền) cho chỉ số 0–15.
fn basic(index: u8, bg: bool) -> String {
    let (low, high) = if bg { (40u16, 100u16) } else { (30u16, 90u16) };
    let code = if index < 8 {
        low + u16::from(index)
    } else {
        high + u16::from(index - 8)
    };
    code.to_string()
}

/// Hạ `Ansi256` về 16 màu: 0–15 giữ nguyên; thang xám ⇒ đen/trắng sáng; khối 6×6×6 ⇒ RGB.
fn ansi256_to_basic(n: u8) -> u8 {
    if n < 16 {
        return n;
    }
    if n >= 232 {
        let v = u16::from(n - 232) * 10 + 8;
        return if v >= 128 { 15 } else { 8 };
    }
    let cube = n - 16;
    let r = cube / 36;
    let g = (cube % 36) / 6;
    let b = cube % 6;
    rgb_to_basic(r * 51, g * 51, b * 51)
}

/// RGB ⇒ 16 màu gần đúng (bit đỏ/xanh lá/xanh dương + độ sáng trung bình).
fn rgb_to_basic(r: u8, g: u8, b: u8) -> u8 {
    let index = u8::from(r >= 128) | (u8::from(g >= 128) << 1) | (u8::from(b >= 128) << 2);
    let avg = (u16::from(r) + u16::from(g) + u16::from(b)) / 3;
    if avg >= 170 {
        8 + index
    } else {
        index
    }
}

/// RGB ⇒ chỉ số 256 màu (khối 6×6×6).
fn rgb_to_256(r: u8, g: u8, b: u8) -> u8 {
    fn level(v: u8) -> u8 {
        (((u16::from(v) * 5) + 127) / 255) as u8
    }
    16 + 36 * level(r) + 6 * level(g) + level(b)
}
