// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-299a (VD-277)** — stdout/stderr của host WASI.
//!
//! Hai chế độ: [`Dich::That`] ghi ngay ra fd 1/2 (WASI), [`Dich::Bo`] gom vào bộ đệm để test
//! tất định. Broken pipe là **lỗi trả về**, không panic; caller quyết định thoát.

use std::io::{self, Write};

/// Đích của stdio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dich {
    /// Ghi thật ra stdout/stderr của tiến trình.
    That,
    /// Gom vào bộ đệm (test).
    Bo,
}

/// Bộ đôi stdout/stderr.
#[derive(Debug)]
pub struct Stdio {
    dich: Dich,
    out: Vec<u8>,
    err: Vec<u8>,
}

impl Stdio {
    /// Ghi thật.
    pub fn that() -> Self {
        Self {
            dich: Dich::That,
            out: Vec::new(),
            err: Vec::new(),
        }
    }

    /// Gom bộ đệm.
    pub fn bo() -> Self {
        Self {
            dich: Dich::Bo,
            out: Vec::new(),
            err: Vec::new(),
        }
    }

    /// Chế độ hiện tại.
    pub fn dich(&self) -> Dich {
        self.dich
    }

    /// Ghi `s` ra stdout; `BrokenPipe` ⇒ `Err` (không panic).
    pub fn ghi_out(&mut self, s: &str) -> Result<(), String> {
        match self.dich {
            Dich::Bo => {
                self.out.extend_from_slice(s.as_bytes());
                Ok(())
            }
            Dich::That => {
                let mut w = io::stdout().lock();
                w.write_all(s.as_bytes())
                    .and_then(|()| w.flush())
                    .map_err(|e| format!("không ghi được stdout: {e}"))
            }
        }
    }

    /// Ghi `s` ra stderr; `BrokenPipe` ⇒ `Err` (không panic).
    pub fn ghi_err(&mut self, s: &str) -> Result<(), String> {
        match self.dich {
            Dich::Bo => {
                self.err.extend_from_slice(s.as_bytes());
                Ok(())
            }
            Dich::That => {
                let mut w = io::stderr().lock();
                w.write_all(s.as_bytes())
                    .and_then(|()| w.flush())
                    .map_err(|e| format!("không ghi được stderr: {e}"))
            }
        }
    }

    /// Bộ đệm stdout (chỉ có nghĩa ở [`Dich::Bo`]).
    pub fn out(&self) -> &[u8] {
        &self.out
    }

    /// Bộ đệm stderr (chỉ có nghĩa ở [`Dich::Bo`]).
    pub fn err(&self) -> &[u8] {
        &self.err
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bo_gom_dung_thu_tu() {
        let mut s = Stdio::bo();
        s.ghi_out("a").unwrap();
        s.ghi_out("b").unwrap();
        s.ghi_err("x").unwrap();
        assert_eq!(s.out(), b"ab");
        assert_eq!(s.err(), b"x");
        assert_eq!(s.dich(), Dich::Bo);
    }
}
