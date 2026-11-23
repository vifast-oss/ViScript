// SPDX-License-Identifier: MIT OR Apache-2.0
//! Khả năng backend hiển thị ([`Capabilities`]) và kích thước ([`Size`]).

/// Mức màu backend hỗ trợ.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorMode {
    /// Không màu; mọi mã SGR bị bỏ.
    #[default]
    None,
    /// 16 màu ANSI (30–37, 90–97).
    Ansi16,
    /// 256 màu ANSI (`38;5;n`).
    Ansi256,
    /// 24-bit (`38;2;r;g;b`).
    TrueColor,
}

impl ColorMode {
    /// `true` nếu backend có phát mã màu.
    pub fn supports_color(self) -> bool {
        !matches!(self, Self::None)
    }
}

/// Khả năng của một backend; `Copy` để truyền qua trait mà không cấp phát.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capabilities {
    /// Mức màu.
    pub color: ColorMode,
    /// Backend ghi được Unicode (không hạ về ASCII).
    pub unicode: bool,
    /// Backend hỗ trợ điều khiển con trỏ (`clear_line`/`move_up`/`show_cursor`).
    pub cursor: bool,
    /// Backend nhận **khung cấu trúc** (ViTerm) thay vì chỉ văn bản.
    pub structured: bool,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            color: ColorMode::None,
            unicode: true,
            cursor: false,
            structured: false,
        }
    }
}

impl Capabilities {
    /// Backend văn bản thuần: không màu, không con trỏ, không khung.
    pub const fn plain() -> Self {
        Self {
            color: ColorMode::None,
            unicode: true,
            cursor: false,
            structured: false,
        }
    }

    /// Backend ANSI đầy đủ (màu + con trỏ), không khung.
    pub const fn ansi(color: ColorMode) -> Self {
        Self {
            color,
            unicode: true,
            cursor: true,
            structured: false,
        }
    }

    /// Backend ViTerm: khung cấu trúc + con trỏ + truecolor.
    pub const fn structured() -> Self {
        Self {
            color: ColorMode::TrueColor,
            unicode: true,
            cursor: true,
            structured: true,
        }
    }
}

/// Kích thước cửa sổ theo ô ký tự.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    pub cols: u16,
    pub rows: u16,
}

impl Size {
    /// Kích thước mới; `0` được giữ nguyên (caller tự quyết định fallback).
    pub const fn new(cols: u16, rows: u16) -> Self {
        Self { cols, rows }
    }
}

impl Default for Size {
    fn default() -> Self {
        Self { cols: 80, rows: 24 }
    }
}

/// Đọc `COLUMNS`/`LINES`; thiếu hoặc sai ⇒ 80×24.
pub(crate) fn size_from_env() -> Size {
    fn lay(ten: &str) -> Option<u16> {
        std::env::var(ten)
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .filter(|n| *n > 0)
    }
    Size {
        cols: lay("COLUMNS").unwrap_or(80),
        rows: lay("LINES").unwrap_or(24),
    }
}

/// Dò mức màu từ môi trường: `NO_COLOR`/`CLICOLOR=0`/`TERM=dumb` ⇒ không màu.
pub(crate) fn detect_color() -> ColorMode {
    if std::env::var_os("NO_COLOR").is_some() {
        return ColorMode::None;
    }
    if std::env::var("CLICOLOR").ok().as_deref() == Some("0") {
        return ColorMode::None;
    }
    let term = std::env::var("TERM").unwrap_or_default();
    if term == "dumb" {
        return ColorMode::None;
    }
    if let Ok(ct) = std::env::var("COLORTERM") {
        if ct == "truecolor" || ct == "24bit" {
            return ColorMode::TrueColor;
        }
    }
    if term.contains("256color") {
        return ColorMode::Ansi256;
    }
    ColorMode::Ansi16
}
