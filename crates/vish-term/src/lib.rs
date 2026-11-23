// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-308 (VD-278/VD-283)** — `vish-term`: ranh giới hiển thị của dòng ViScript.
//!
//! Crate **độc lập ViFastOs** (luật R4): chỉ dùng `std` và `unicode-normalization` (NFC, đã có
//! trong workspace). [`Terminal`] là hợp đồng tối thiểu cho `Render` (§9.6) và `vish-edit` (§15):
//! ghi văn bản/có kiểu, kích thước, điều khiển con trỏ, và **hook hiển thị có cấu trúc**
//! ([`Terminal::table`], [`Terminal::record`], [`Terminal::diagnostic`]).
//!
//! Ba backend:
//! - [`AnsiPosix`] — SGR ANSI, xuống dòng `LF`; dùng trên POSIX và mọi terminal hiểu ANSI.
//! - `WinConsole` — Windows console (cfg `windows`), xuống dòng `CRLF`, mặc định **không** ANSI.
//! - [`ViTermFramed`] — khung JSON Lines `vish-term/1` cho ViTerm; dữ liệu có cấu trúc được gửi
//!   nguyên khung thay vì hạ thành văn bản.
//!
//! **Hướng API ổn định**: chỉ thêm method có default; không đổi chữ ký đã công bố. Mã nguồn crate
//! đặt `#![forbid(unsafe_code)]` (luật dòng ViScript).

#![forbid(unsafe_code)]

mod ansi;
pub mod caps;
pub mod posix;
pub mod render;
pub mod style;
pub mod viterm;
pub mod width;
#[cfg(windows)]
pub mod windows;

use std::io;

pub use caps::{Capabilities, ColorMode, Size};
pub use posix::AnsiPosix;
pub use render::{Align, Column, Diagnostic, Location, Record, Severity, Table};
pub use style::{Attrs, Color, Style};
pub use viterm::ViTermFramed;
pub use width::display_width;
#[cfg(windows)]
pub use windows::WinConsole;

/// Backend mặc định theo nền: ANSI trên mọi nền không phải Windows, Windows console trên Windows.
#[cfg(not(windows))]
pub type NativeTerminal<W> = AnsiPosix<W>;
/// Backend mặc định theo nền: ANSI trên mọi nền không phải Windows, Windows console trên Windows.
#[cfg(windows)]
pub type NativeTerminal<W> = WinConsole<W>;

/// Tạo backend mặc định gắn với `stdout`.
#[cfg(not(windows))]
pub fn native_terminal() -> NativeTerminal<io::Stdout> {
    AnsiPosix::new()
}

/// Tạo backend mặc định gắn với `stdout`.
#[cfg(windows)]
pub fn native_terminal() -> NativeTerminal<io::Stdout> {
    WinConsole::new()
}

/// Hợp đồng hiển thị tối thiểu dùng chung cho `Render` và `vish-edit`.
///
/// Backend chỉ **bắt buộc** cài [`Terminal::caps`] và [`Terminal::write`]; mọi method khác có
/// default an toàn. Backend có khả năng cấu trúc (ViTerm) ghi đè ba hook
/// [`Terminal::table`], [`Terminal::record`], [`Terminal::diagnostic`] để gửi dữ liệu nguyên khung;
/// backend văn bản dùng default của [`render`].
///
/// Trait **object-safe**: dùng được qua `&mut dyn Terminal`.
pub trait Terminal {
    /// Khả năng của backend (màu, Unicode, con trỏ, khung cấu trúc).
    fn caps(&self) -> Capabilities;

    /// Kích thước hiển thị; backend không biết thì trả `Size::default()`.
    fn size(&self) -> Size {
        Size::default()
    }

    /// Ghi một chuỗi UTF-8.
    fn write(&mut self, s: &str) -> io::Result<()>;

    /// Ghi một dòng (backend tự chọn `LF`/`CRLF`).
    fn write_line(&mut self, s: &str) -> io::Result<()> {
        self.write(s)?;
        self.write("\n")
    }

    /// Ghi chuỗi kèm kiểu; backend không hỗ trợ kiểu thì bỏ kiểu.
    fn write_styled(&mut self, style: Style, s: &str) -> io::Result<()> {
        let _ = style;
        self.write(s)
    }

    /// Đẩy bộ đệm xuống thiết bị.
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    /// Xoá dòng hiện tại và đưa con trỏ về đầu dòng; backend không hỗ trợ thì không làm gì.
    fn clear_line(&mut self) -> io::Result<()> {
        Ok(())
    }

    /// Dịch con trỏ lên `n` dòng; backend không hỗ trợ thì không làm gì.
    fn move_up(&mut self, n: u16) -> io::Result<()> {
        let _ = n;
        Ok(())
    }

    /// Hiện/ẩn con trỏ; backend không hỗ trợ thì không làm gì.
    fn show_cursor(&mut self, show: bool) -> io::Result<()> {
        let _ = show;
        Ok(())
    }

    /// Hiển thị bảng; default hạ thành văn bản qua [`render::table`].
    fn table(&mut self, table: &Table) -> io::Result<()> {
        render::table(self, table)
    }

    /// Hiển thị bản ghi; default hạ thành văn bản qua [`render::record`].
    fn record(&mut self, record: &Record) -> io::Result<()> {
        render::record(self, record)
    }

    /// Hiển thị chẩn đoán; default hạ thành văn bản qua [`render::diagnostic`].
    fn diagnostic(&mut self, diagnostic: &Diagnostic) -> io::Result<()> {
        render::diagnostic(self, diagnostic)
    }
}
