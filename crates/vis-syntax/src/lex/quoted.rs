// SPDX-License-Identifier: MIT OR Apache-2.0
//! Quét chuỗi/nội suy dùng chung hai chế độ (`scan_quoted`, `skip_interpolation`).
//!
//! **ST-318f** — tách **nguyên văn** khỏi `lex/mod.rs` (R1 mềm): chỉ đổi vị trí,
//! token/span/chẩn đoán/hành vi không đổi.

use super::*;

impl<'a> Lexer<'a> {
    pub(super) fn scan_quoted(&mut self, kind: Kind) {
        let start = self.i;
        let (line, col) = (self.line, self.col);
        let (open, close, escapes, interp) = match kind {
            Kind::Str => ("\"", "\"", true, true),
            Kind::MultiStr => ("\"\"\"", "\"\"\"", true, true),
            // **R6a (VD-178)**: `p"…"` **không** escape (đường dẫn Windows), **có** nội suy.
            Kind::Path => ("p\"", "\"", false, true),
            Kind::RawStr => ("r\"", "\"", false, false),
            Kind::RawMulti => ("r\"\"\"", "\"\"\"", false, false),
            Kind::Bytes => ("b\"", "\"", true, false),
            _ => return,
        };
        let multiline = matches!(kind, Kind::MultiStr | Kind::RawMulti);
        for _ in 0..open.len() {
            self.bump_ascii();
        }
        loop {
            if self.i >= self.src.len() {
                self.error(101, Span::new(start, self.src.len()), line, col);
                return;
            }
            if self.starts_with(close) {
                for _ in 0..close.len() {
                    self.bump_ascii();
                }
                break;
            }
            let b = self.byte().unwrap_or(0);
            if b == b'\n' && !multiline {
                // Chuỗi một dòng không được chứa newline thô: coi là chưa đóng (§1).
                self.error(101, Span::new(start, self.i), line, col);
                return;
            }
            if escapes && b == b'\\' {
                self.bump_ascii();
                if self.i < self.src.len() {
                    self.bump_char();
                }
                continue;
            }
            if interp && b == b'{' {
                if self.peek(1) == Some(b'{') {
                    self.bump_ascii();
                    self.bump_ascii();
                    continue;
                }
                if !self.skip_interpolation() {
                    self.error(108, Span::new(start, self.i), line, col);
                    return;
                }
                continue;
            }
            self.bump_char();
        }

        let text = &self.src[start..self.i];
        let decoded = if kind == Kind::Bytes {
            lit::decode_bytes(text).err().map(|e| e.code)
        } else {
            lit::decode_str(kind, text).err().map(|e| e.code)
        };
        if let Some(code) = decoded {
            self.error(code, Span::new(start, self.i), line, col);
            return;
        }
        self.push_at(kind, start, line, col);
    }

    /// Quét `{expr}` của nội suy tới `}` khớp; `false` nếu hết tệp trước khi đóng (§3).
    pub(super) fn skip_interpolation(&mut self) -> bool {
        let mut depth = 0usize;
        while self.i < self.src.len() {
            match self.byte() {
                Some(b'{') => {
                    depth += 1;
                    self.bump_ascii();
                }
                Some(b'}') => {
                    depth -= 1;
                    self.bump_ascii();
                    if depth == 0 {
                        return true;
                    }
                }
                Some(b'\\') => {
                    self.bump_ascii();
                    if self.i < self.src.len() {
                        self.bump_char();
                    }
                }
                Some(_) => self.bump_char(),
                None => break,
            }
        }
        false
    }
}
