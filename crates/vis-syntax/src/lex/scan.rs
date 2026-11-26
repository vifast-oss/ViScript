// SPDX-License-Identifier: MIT OR Apache-2.0
//! Vị trí, phát token, vòng chạy và thụt lề của [`Lexer`].
//!
//! **ST-318f** — tách **nguyên văn** khỏi `lex/mod.rs` (R1 mềm): chỉ đổi vị trí,
//! token/span/chẩn đoán/hành vi không đổi.

use super::*;

impl<'a> Lexer<'a> {
    pub(super) fn new(src: &'a str, mode: Mode) -> Self {
        Self {
            src,
            i: 0,
            line: 1,
            col: 1,
            mode,
            tokens: Vec::new(),
            diags: Vec::new(),
            stack: vec![0],
            unit: None,
            depth: 0,
            line_has_code: false,
            at_line_start: true,
            joined: false,
            stage_start: true,
        }
    }

    // ---------------------------------------------------------------- vị trí

    pub(super) fn rest(&self) -> &'a str {
        &self.src[self.i..]
    }

    pub(super) fn byte(&self) -> Option<u8> {
        self.src.as_bytes().get(self.i).copied()
    }

    pub(super) fn peek(&self, n: usize) -> Option<u8> {
        self.src.as_bytes().get(self.i + n).copied()
    }

    pub(super) fn starts_with(&self, s: &str) -> bool {
        self.rest().starts_with(s)
    }

    pub(super) fn cur_char(&self) -> Option<char> {
        self.rest().chars().next()
    }

    pub(super) fn bump_ascii(&mut self) {
        self.i += 1;
        self.col += 1;
    }

    /// Tiến một **ký tự** bất kỳ, cập nhật dòng/cột — kể cả khi ký tự đó là `\n` (newline nằm
    /// trong nội suy hoặc trong chuỗi nhiều dòng: vẫn phải đếm dòng cho đúng, §4.4).
    pub(super) fn bump_char(&mut self) {
        if let Some(c) = self.cur_char() {
            self.i += c.len_utf8();
            match c {
                '\n' => {
                    self.line += 1;
                    self.col = 1;
                }
                '\t' => self.col += 4,
                _ => self.col += 1,
            }
        }
    }

    /// Toạ độ (dòng, cột) tại một offset bất kỳ — dùng để quy vị trí lỗi trong literal.
    pub(super) fn line_col_at(&self, offset: usize) -> (u32, u32) {
        let mut line = 1u32;
        let mut col = 1u32;
        for c in self.src[..offset].chars() {
            if c == '\n' {
                line += 1;
                col = 1;
            } else if c == '\t' {
                col += 4;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    // ---------------------------------------------------------------- phát

    pub(super) fn push_at(&mut self, kind: Kind, start: usize, line: u32, col: u32) {
        let text = self.src[start..self.i].to_string();
        self.note_stage(kind, &text);
        if !matches!(
            kind,
            Kind::Newline
                | Kind::Indent
                | Kind::Dedent
                | Kind::Eof
                | Kind::Comment
                | Kind::DocComment
        ) {
            self.line_has_code = true;
            self.joined = false;
        }
        self.tokens.push(Token {
            kind,
            text,
            span: Span::new(start, self.i),
            line,
            col,
        });
    }

    pub(super) fn push_zero(&mut self, kind: Kind) {
        self.tokens.push(Token {
            kind,
            text: String::new(),
            span: Span::new(self.i, self.i),
            line: self.line,
            col: self.col,
        });
    }

    /// Theo dõi đầu stage chế độ lệnh (SPEC-lex §6: `^` chỉ hợp lệ ở đầu stage).
    pub(super) fn note_stage(&mut self, kind: Kind, text: &str) {
        if self.mode != Mode::Command {
            return;
        }
        self.stage_start = match kind {
            Kind::Newline => true,
            Kind::Punct => matches!(text, ";" | "|" | "|>" | "&"),
            _ => false,
        };
    }

    pub(super) fn error(&mut self, id: u16, span: Span, line: u32, col: u32) {
        let code = vis_diag::lookup(id).expect("mã VS01xx phải có trong sổ vis-diag");
        self.diags.push(Diagnostic {
            code,
            message: code.message_vi.to_string(),
            span,
            line,
            col,
        });
    }

    /// Báo lỗi tại một offset byte (tự tính dòng/cột).
    pub(super) fn error_at(&mut self, id: u16, offset: usize) {
        let (line, col) = self.line_col_at(offset);
        self.error(id, Span::new(offset, self.i.max(offset)), line, col);
    }

    // ---------------------------------------------------------------- vòng chính

    pub(super) fn run(&mut self) {
        loop {
            if self.at_line_start {
                self.at_line_start = false;
                if self.mode == Mode::Script && self.depth == 0 {
                    self.begin_line();
                } else {
                    self.skip_inline_ws();
                }
            }
            self.skip_inline_ws();
            if self.i >= self.src.len() {
                break;
            }
            match self.byte() {
                Some(b'\n') => {
                    let start = self.i;
                    let (line, col) = (self.line, self.col);
                    self.finish_newline(start, line, col);
                }
                // CRLF đã thành LF ở `normalize_source`; `\r` còn lại là ký tự lạ (§1).
                Some(b'\r') => {
                    let start = self.i;
                    let (line, col) = (self.line, self.col);
                    self.bump_char();
                    self.error(103, Span::new(start, self.i), line, col);
                }
                _ => {
                    if self.mode == Mode::Script {
                        self.token_script();
                    } else {
                        self.token_cmd();
                    }
                }
            }
        }
        self.finish();
    }

    pub(super) fn skip_inline_ws(&mut self) {
        loop {
            match self.byte() {
                Some(b' ') => {
                    self.i += 1;
                    self.col += 1;
                }
                Some(b'\t') => {
                    let start = self.i;
                    let (line, col) = (self.line, self.col);
                    self.i += 1;
                    self.col += 4;
                    self.error(107, Span::new(start, self.i), line, col);
                }
                _ => return,
            }
        }
    }

    /// Đầu một dòng vật lý ở độ sâu ngoặc 0 (script): đo thụt lề và áp INDENT/DEDENT.
    pub(super) fn begin_line(&mut self) {
        let start = self.i;
        let (line, col) = (self.line, self.col);
        let indent = self.measure_indent();
        if self.i >= self.src.len() {
            return;
        }
        // Dòng trống hoặc chỉ có chú thích: không INDENT/DEDENT/NEWLINE (§4.2).
        if matches!(self.byte(), Some(b'\n') | Some(b'\r') | Some(b'#')) {
            return;
        }
        if self.joined {
            return;
        }
        self.apply_indent(indent, start, line, col);
    }

    pub(super) fn measure_indent(&mut self) -> usize {
        let mut width = 0usize;
        loop {
            match self.byte() {
                Some(b' ') => {
                    self.i += 1;
                    self.col += 1;
                    width += 1;
                }
                Some(b'\t') => {
                    let start = self.i;
                    let (line, col) = (self.line, self.col);
                    self.i += 1;
                    self.col += 4;
                    width += 4;
                    self.error(107, Span::new(start, self.i), line, col);
                }
                _ => return width,
            }
        }
    }

    pub(super) fn apply_indent(&mut self, indent: usize, start: usize, line: u32, col: u32) {
        let top = *self.stack.last().expect("ngăn xếp thụt lề luôn có 0");
        if indent > top {
            match self.unit {
                Some(u) if !indent.is_multiple_of(u) => {
                    self.error(109, Span::new(start, self.i), line, col);
                }
                None => self.unit = Some(indent),
                Some(_) => {}
            }
            self.stack.push(indent);
            let text = self.src[start..self.i].to_string();
            self.tokens.push(Token {
                kind: Kind::Indent,
                text,
                span: Span::new(start, self.i),
                line,
                col,
            });
            self.line_has_code = true;
        } else if indent < top {
            while self.stack.len() > 1 && indent < *self.stack.last().expect("có 0") {
                self.stack.pop();
                self.push_zero(Kind::Dedent);
            }
            if indent != *self.stack.last().expect("có 0") {
                self.error(109, Span::new(start, self.i), line, col);
            }
        }
    }

    /// `\n`: NEWLINE/`|>`/nuốt theo §4 (CRLF đã được chuẩn hoá thành LF trước khi lex).
    pub(super) fn finish_newline(&mut self, start: usize, line: u32, col: u32) {
        let had_code = self.line_has_code;
        let cont = self.peek_join_next();
        self.i += 1; // '\n'
        self.line += 1;
        self.col = 1;
        self.line_has_code = false;
        self.at_line_start = true;
        self.stage_start = true;
        if self.depth > 0 || !had_code {
            return;
        }
        if cont {
            self.joined = true;
            return;
        }
        self.tokens.push(Token {
            kind: Kind::Newline,
            text: self.src[start..self.i].to_string(),
            span: Span::new(start, self.i),
            line,
            col,
        });
    }

    /// Dòng kế có bắt đầu bằng `|>` không (bỏ qua khoảng trắng/dòng trống) — §4.5.
    pub(super) fn peek_join_next(&self) -> bool {
        let bytes = self.src.as_bytes();
        let mut j = self.i + 1;
        while j < bytes.len() && matches!(bytes[j], b' ' | b'\t' | b'\r' | b'\n') {
            j += 1;
        }
        self.src[j..].starts_with("|>")
    }

    pub(super) fn finish(&mut self) {
        while self.stack.len() > 1 {
            self.stack.pop();
            self.push_zero(Kind::Dedent);
        }
        let start = self.i;
        let (line, col) = (self.line, self.col);
        self.push_at(Kind::Eof, start, line, col);
    }
}
