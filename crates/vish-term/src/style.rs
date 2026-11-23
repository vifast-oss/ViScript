// SPDX-License-Identifier: MIT OR Apache-2.0
//! Kiểu chữ ([`Style`]), màu ([`Color`]) và thuộc tính ([`Attrs`]) — mô hình độc lập backend.

/// Màu chữ/nền. Backend tự hạ cấp theo [`crate::ColorMode`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    /// Chỉ số 0–255 của bảng màu ANSI.
    Ansi256(u8),
    /// 24-bit.
    Rgb(u8, u8, u8),
}

impl Color {
    /// Chỉ số 0–15 cho 16 màu cơ bản; `None` với [`Color::Ansi256`]/[`Color::Rgb`].
    pub fn basic_index(self) -> Option<u8> {
        Some(match self {
            Self::Black => 0,
            Self::Red => 1,
            Self::Green => 2,
            Self::Yellow => 3,
            Self::Blue => 4,
            Self::Magenta => 5,
            Self::Cyan => 6,
            Self::White => 7,
            Self::BrightBlack => 8,
            Self::BrightRed => 9,
            Self::BrightGreen => 10,
            Self::BrightYellow => 11,
            Self::BrightBlue => 12,
            Self::BrightMagenta => 13,
            Self::BrightCyan => 14,
            Self::BrightWhite => 15,
            Self::Ansi256(_) | Self::Rgb(_, _, _) => return None,
        })
    }
}

/// Thuộc tính chữ; mọi cờ mặc định `false`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Attrs {
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub reverse: bool,
}

impl Attrs {
    /// Không thuộc tính nào.
    pub const NONE: Self = Self {
        bold: false,
        dim: false,
        italic: false,
        underline: false,
        reverse: false,
    };

    /// `true` nếu không có cờ nào bật.
    pub fn is_none(self) -> bool {
        self == Self::NONE
    }
}

/// Kiểu chữ đầy đủ; `Copy` và so sánh được.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub attrs: Attrs,
}

impl Style {
    /// Kiểu trống.
    pub const PLAIN: Self = Self {
        fg: None,
        bg: None,
        attrs: Attrs::NONE,
    };

    /// In đậm.
    pub const BOLD: Self = Self {
        fg: None,
        bg: None,
        attrs: Attrs {
            bold: true,
            dim: false,
            italic: false,
            underline: false,
            reverse: false,
        },
    };

    /// Mờ (dùng cho nhãn phụ).
    pub const DIM: Self = Self {
        fg: None,
        bg: None,
        attrs: Attrs {
            bold: false,
            dim: true,
            italic: false,
            underline: false,
            reverse: false,
        },
    };

    /// Kiểu trống.
    pub const fn new() -> Self {
        Self::PLAIN
    }

    /// Đặt màu chữ.
    pub const fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    /// Đặt màu nền.
    pub const fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    /// Bật in đậm.
    pub const fn bold(mut self) -> Self {
        self.attrs.bold = true;
        self
    }

    /// Bật mờ.
    pub const fn dim(mut self) -> Self {
        self.attrs.dim = true;
        self
    }

    /// Bật nghiêng.
    pub const fn italic(mut self) -> Self {
        self.attrs.italic = true;
        self
    }

    /// Bật gạch chân.
    pub const fn underline(mut self) -> Self {
        self.attrs.underline = true;
        self
    }

    /// Bật đảo màu.
    pub const fn reverse(mut self) -> Self {
        self.attrs.reverse = true;
        self
    }

    /// `true` nếu không màu và không thuộc tính — backend bỏ qua mã SGR.
    pub fn is_plain(self) -> bool {
        self.fg.is_none() && self.bg.is_none() && self.attrs.is_none()
    }
}
