// SPDX-License-Identifier: MIT OR Apache-2.0
//! Quét token chế độ **lệnh** (`.vish`/REPL).
//!
//! **ST-318f** — tách **nguyên văn** khỏi `lex/mod.rs` (R1 mềm): chỉ đổi vị trí,
//! token/span/chẩn đoán/hành vi không đổi.

use super::*;

impl<'a> Lexer<'a> {
    pub(super) fn token_cmd(&mut self) {
        let c = match self.byte() {
            Some(c) => c,
            None => return,
        };
        match c {
            b'$' => self.scan_dollar(),
            b'^' => self.scan_caret(),
            b'\'' => self.scan_cmd_raw_string(),
            b'"' => {
                if self.starts_with("\"\"\"") {
                    self.scan_quoted(Kind::MultiStr);
                } else {
                    self.scan_quoted(Kind::Str);
                }
            }
            b'#' => self.scan_comment(),
            b'(' | b')' | b'[' | b']' | b'{' | b'}' | b';' | b'&' | b'>' | b'<' | b'|' => {
                self.scan_punct();
            }
            _ => self.scan_word(),
        }
    }

    /// Metachar chế độ lệnh (SPEC-lex §6) — `#` chỉ ở đầu từ nên không nằm đây.
    pub(super) fn is_cmd_metachar(c: char) -> bool {
        matches!(
            c,
            ' ' | '\t'
                | '('
                | ')'
                | '['
                | ']'
                | '{'
                | '}'
                | ';'
                | '&'
                | '$'
                | '\''
                | '"'
                | '|'
                | '>'
                | '<'
                | '^'
                | '\n'
                | '\r'
        )
    }

    pub(super) fn scan_word(&mut self) {
        let start = self.i;
        let (line, col) = (self.line, self.col);
        while let Some(c) = self.cur_char() {
            if Self::is_cmd_metachar(c) {
                break;
            }
            self.bump_char();
        }
        if self.i == start {
            self.bump_char();
            self.error(103, Span::new(start, self.i), line, col);
            return;
        }
        self.push_at(Kind::Word, start, line, col);
    }

    pub(super) fn scan_dollar(&mut self) {
        let start = self.i;
        let (line, col) = (self.line, self.col);
        self.bump_ascii();
        self.push_at(Kind::Dollar, start, line, col);
        // `$env.NAME` (§6): biến, rồi chuỗi `.TÊN` — mỗi đoạn một token như §8 #50.
        loop {
            if !self.is_ident_start() {
                return;
            }
            let seg = self.i;
            let (l, c) = (self.line, self.col);
            self.bump_char();
            while let Some(ch) = self.cur_char() {
                if ch == '_' || unicode_ident::is_xid_continue(ch) {
                    self.bump_char();
                } else {
                    break;
                }
            }
            self.push_at(Kind::Ident, seg, l, c);
            if self.byte() == Some(b'.')
                && self
                    .peek(1)
                    .is_some_and(|b| b == b'_' || (b as char).is_ascii_alphabetic())
            {
                let dot = self.i;
                let (dl, dc) = (self.line, self.col);
                self.bump_ascii();
                self.push_at(Kind::Punct, dot, dl, dc);
                continue;
            }
            return;
        }
    }

    pub(super) fn scan_caret(&mut self) {
        let start = self.i;
        let (line, col) = (self.line, self.col);
        self.bump_ascii();
        if self.stage_start {
            self.push_at(Kind::Caret, start, line, col);
        } else {
            // §7 VS0110: `^` ngoài đầu stage.
            self.error(110, Span::new(start, self.i), line, col);
            self.push_at(Kind::Punct, start, line, col);
        }
    }

    /// `'…'` chế độ lệnh = RAW_STRING thô, không escape, không nội suy (§6).
    pub(super) fn scan_cmd_raw_string(&mut self) {
        let start = self.i;
        let (line, col) = (self.line, self.col);
        self.bump_ascii();
        loop {
            if self.i >= self.src.len() {
                self.error(101, Span::new(start, self.src.len()), line, col);
                return;
            }
            match self.byte() {
                Some(b'\'') => {
                    self.bump_ascii();
                    break;
                }
                Some(b'\n' | b'\r') => {
                    self.error(101, Span::new(start, self.i), line, col);
                    return;
                }
                _ => self.bump_char(),
            }
        }
        self.push_at(Kind::RawStr, start, line, col);
    }
}
