// SPDX-License-Identifier: MIT OR Apache-2.0
//! Backend ANSI POSIX ([`AnsiPosix`]): SGR, `LF`, điều khiển con trỏ bằng CSI.

use std::io::{self, IsTerminal, Write};

use crate::ansi::sgr;
use crate::caps::{detect_color, size_from_env};
use crate::{Capabilities, ColorMode, Size, Style, Terminal};

/// Terminal ANSI trên một `Write` bất kỳ.
///
/// Mặc định dò màu từ môi trường (`NO_COLOR`, `TERM`, `COLORTERM`); test/caller ép bằng
/// [`AnsiPosix::with_color`]. Không phụ thuộc OS — dùng được cả trên Windows Terminal.
pub struct AnsiPosix<W: Write> {
    out: W,
    caps: Capabilities,
}

impl AnsiPosix<io::Stdout> {
    /// Terminal gắn `stdout`, dò màu từ môi trường; **tắt màu/con trỏ khi `stdout` không phải TTY**
    /// (ống/tệp không nhận mã ANSI).
    pub fn new() -> Self {
        let mut term = Self::with_writer(io::stdout());
        if !io::stdout().is_terminal() {
            term.caps.color = ColorMode::None;
            term.caps.cursor = false;
        }
        term
    }
}

impl Default for AnsiPosix<io::Stdout> {
    fn default() -> Self {
        Self::new()
    }
}

impl<W: Write> AnsiPosix<W> {
    /// Terminal ANSI trên `out`; màu dò từ môi trường, Unicode + con trỏ bật.
    pub fn with_writer(out: W) -> Self {
        Self {
            out,
            caps: Capabilities::ansi(detect_color()),
        }
    }

    /// Ép toàn bộ khả năng (dùng cho test hoặc cấu hình tường minh).
    pub fn with_caps(mut self, caps: Capabilities) -> Self {
        self.caps = caps;
        self
    }

    /// Ép mức màu.
    pub fn with_color(mut self, color: ColorMode) -> Self {
        self.caps.color = color;
        self
    }

    /// Bật/tắt Unicode (tắt ⇒ caller chịu trách nhiệm chỉ ghi ASCII).
    pub fn with_unicode(mut self, unicode: bool) -> Self {
        self.caps.unicode = unicode;
        self
    }

    /// Bật/tắt điều khiển con trỏ.
    pub fn with_cursor(mut self, cursor: bool) -> Self {
        self.caps.cursor = cursor;
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

impl<W: Write> Terminal for AnsiPosix<W> {
    fn caps(&self) -> Capabilities {
        self.caps
    }

    fn size(&self) -> Size {
        size_from_env()
    }

    fn write(&mut self, s: &str) -> io::Result<()> {
        self.out.write_all(s.as_bytes())
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
        if !self.caps.cursor {
            return Ok(());
        }
        self.out.write_all(b"\r\x1b[2K")
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
