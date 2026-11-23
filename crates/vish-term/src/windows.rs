// SPDX-License-Identifier: MIT OR Apache-2.0
//! Backend Windows console ([`WinConsole`]) — chỉ biên dịch trên `cfg(windows)`.
//!
//! Mặc định ghi văn bản thuần với `CRLF` và **không** phát mã ANSI (an toàn cho `conhost` cũ).
//! Bật [`WinConsole::with_ansi`] khi chạy trong Windows Terminal/conhost đã bật VT; khi đó dùng
//! chung bộ mã SGR với [`crate::AnsiPosix`].

use std::io::{self, Write};

use crate::ansi::sgr;
use crate::caps::size_from_env;
use crate::{Capabilities, ColorMode, Size, Style, Terminal};

/// Terminal Windows console trên một `Write` bất kỳ.
pub struct WinConsole<W: Write> {
    out: W,
    caps: Capabilities,
}

impl WinConsole<io::Stdout> {
    /// Terminal gắn `stdout`; mặc định không màu, không con trỏ.
    pub fn new() -> Self {
        Self::with_writer(io::stdout())
    }
}

impl Default for WinConsole<io::Stdout> {
    fn default() -> Self {
        Self::new()
    }
}

impl<W: Write> WinConsole<W> {
    /// Terminal Windows console trên `out`; mặc định văn bản thuần.
    pub fn with_writer(out: W) -> Self {
        Self {
            out,
            caps: Capabilities {
                color: ColorMode::None,
                unicode: true,
                cursor: false,
                structured: false,
            },
        }
    }

    /// Bật ANSI (Windows Terminal/conhost đã bật VT); tắt ⇒ quay về văn bản thuần.
    pub fn with_ansi(mut self, ansi: bool) -> Self {
        self.caps.color = if ansi {
            ColorMode::Ansi16
        } else {
            ColorMode::None
        };
        self.caps.cursor = ansi;
        self
    }

    /// Bật/tắt Unicode.
    pub fn with_unicode(mut self, unicode: bool) -> Self {
        self.caps.unicode = unicode;
        self
    }

    /// Ép toàn bộ khả năng.
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
}

impl<W: Write> Terminal for WinConsole<W> {
    fn caps(&self) -> Capabilities {
        self.caps
    }

    fn size(&self) -> Size {
        size_from_env()
    }

    fn write(&mut self, s: &str) -> io::Result<()> {
        self.out.write_all(s.as_bytes())
    }

    fn write_line(&mut self, s: &str) -> io::Result<()> {
        self.out.write_all(s.as_bytes())?;
        self.out.write_all(b"\r\n")
    }

    fn write_styled(&mut self, style: Style, s: &str) -> io::Result<()> {
        if s.is_empty() {
            return Ok(());
        }
        let seq = sgr(style, self.caps.color);
        if seq.is_empty() {
            return self.out.write_all(s.as_bytes());
        }
        self.out.write_all(seq.as_bytes())?;
        self.out.write_all(s.as_bytes())?;
        self.out.write_all(b"\x1b[0m")
    }

    fn flush(&mut self) -> io::Result<()> {
        self.out.flush()
    }

    fn clear_line(&mut self) -> io::Result<()> {
        if self.caps.cursor {
            return self.out.write_all(b"\r\x1b[2K");
        }
        // Không ANSI: xoá bằng dấu cách theo bề rộng ước lượng, rồi về đầu dòng.
        let cols = usize::from(size_from_env().cols);
        self.out.write_all(b"\r")?;
        self.out.write_all(" ".repeat(cols).as_bytes())?;
        self.out.write_all(b"\r")
    }

    fn move_up(&mut self, n: u16) -> io::Result<()> {
        if !self.caps.cursor || n == 0 {
            return Ok(());
        }
        write!(self.out, "\x1b[{n}A")
    }

    fn show_cursor(&mut self, show: bool) -> io::Result<()> {
        if !self.caps.cursor {
            return Ok(());
        }
        self.out
            .write_all(if show { b"\x1b[?25h" } else { b"\x1b[?25l" })
    }
}
