// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-304** — raw mode: `rustix::termios` trên Unix/macOS; `crossterm` trên Windows.
//! Guard khôi phục termios khi `Drop` (kể cả panic).

use std::io;

pub struct RawMode {
    #[cfg(unix)]
    saved: Option<rustix::termios::Termios>,
    #[cfg(windows)]
    active: bool,
}

impl RawMode {
    /// Bật raw mode nếu stdin là terminal; nếu không (pipe/test) là no-op.
    pub fn enable() -> io::Result<Self> {
        #[cfg(unix)]
        {
            use std::io::IsTerminal;
            use std::os::fd::AsFd;
            let stdin = io::stdin();
            if !stdin.is_terminal() {
                return Ok(Self { saved: None });
            }
            let saved = rustix::termios::tcgetattr(stdin.as_fd())?;
            let mut raw = saved.clone();
            raw.make_raw();
            rustix::termios::tcsetattr(stdin.as_fd(), rustix::termios::OptionalActions::Now, &raw)?;
            Ok(Self { saved: Some(saved) })
        }
        #[cfg(windows)]
        {
            use std::io::IsTerminal;
            if !io::stdin().is_terminal() {
                return Ok(Self { active: false });
            }
            crossterm::terminal::enable_raw_mode()?;
            Ok(Self { active: true })
        }
        #[cfg(not(any(unix, windows)))]
        {
            Ok(Self {})
        }
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            if let Some(saved) = self.saved.take() {
                use std::os::fd::AsFd;
                let _ = rustix::termios::tcsetattr(
                    io::stdin().as_fd(),
                    rustix::termios::OptionalActions::Now,
                    &saved,
                );
            }
        }
        #[cfg(windows)]
        {
            if self.active {
                let _ = crossterm::terminal::disable_raw_mode();
                self.active = false;
            }
        }
    }
}
