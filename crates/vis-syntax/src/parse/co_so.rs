// SPDX-License-Identifier: MIT OR Apache-2.0
//! Trợ giúp nền của [`Parser`]: truy cập token, dựng node/span, `Program` thô, lỗi cú pháp.
//!
//! **ST-318f** — tách **nguyên văn** khỏi `parse/mod.rs` (R1 mềm): chỉ đổi vị trí,
//! token/AST/CST/thứ tự chẩn đoán/API không đổi.

use super::{PErr, PResult, Parser};
use crate::cst::{Elem, Node, NodeKind};
use crate::lex::token::{Kind, Span, Token};
use crate::lex::Diagnostic;

impl<'a> Parser<'a> {
    pub(super) fn new(toks: &'a [Token], text: &'a str) -> Self {
        let sig = toks
            .iter()
            .enumerate()
            .filter(|(_, t)| !matches!(t.kind, Kind::Comment | Kind::DocComment))
            .map(|(i, _)| i as u32)
            .collect();
        Self {
            toks,
            text,
            sig,
            pos: 0,
            da_bind: crate::parse::cmd::ten_mac_dinh(),
            delim: 0,
        }
    }

    // ----- truy cập token -----

    /// Token hiện tại (đã bỏ chú thích); `EOF` khi hết.
    pub(super) fn tok(&self) -> &'a Token {
        self.tok_at(0)
    }

    pub(super) fn tok_at(&self, k: usize) -> &'a Token {
        let idx = (self.pos + k).min(self.sig.len().saturating_sub(1));
        &self.toks[self.sig[idx] as usize]
    }

    pub(super) fn peek_punct(&self, k: usize, s: &str) -> bool {
        self.tok_at(k).is_punct(s)
    }

    pub(super) fn peek_kw(&self, k: usize, s: &str) -> bool {
        let t = self.tok_at(k);
        t.kind == Kind::Kw && t.text == s
    }

    pub(super) fn at_eof(&self) -> bool {
        self.tok().kind == Kind::Eof
    }

    pub(super) fn at_punct(&self, s: &str) -> bool {
        self.tok().is_punct(s)
    }

    pub(super) fn at_kw(&self, s: &str) -> bool {
        self.tok().kind == Kind::Kw && self.tok().text == s
    }

    pub(super) fn at_ident(&self, s: &str) -> bool {
        self.tok().kind == Kind::Ident && self.tok().text == s
    }

    pub(super) fn at_kind(&self, k: Kind) -> bool {
        self.tok().kind == k
    }

    /// **VD-266**: token hiện tại đóng/dừng một biểu thức trong ngoặc: `)`, `]`, `}`, `,`.
    /// Chỉ có nghĩa khi [`Parser::delim`] `> 0` — người gọi phải kiểm điều đó.
    pub(super) fn at_delim_end(&self) -> bool {
        self.at_punct(")") || self.at_punct("]") || self.at_punct("}") || self.at_punct(",")
    }

    /// **VD-266**: token hiện tại bắt đầu một **dòng vật lý mới** so với token đã ăn trước đó.
    /// Trong ngoặc, lexer nuốt NEWLINE (§4.3) nên đây là tín hiệu duy nhất để `suite()`/`match`
    /// biết một khối một-dòng đã hết.
    pub(super) fn at_new_line(&self) -> bool {
        let Some(prev) = self.pos.checked_sub(1) else {
            return false;
        };
        self.tok().line > self.toks[self.sig[prev] as usize].line
    }

    /// Chỉ số token gốc của vị trí hiện tại (để dựng lá CST).
    pub(super) fn leaf(&self) -> u32 {
        let idx = self.pos.min(self.sig.len() - 1);
        self.sig[idx]
    }

    /// Ăn token hiện tại và trả lá CST tương ứng.
    ///
    /// **VD-266**: đồng thời cập nhật [`Parser::delim`] theo ngoặc đã ăn — phản chiếu đúng bộ đếm
    /// `depth` của lexer (§4.3) để `suite()` biết mình đang trong biểu thức có ngoặc hay không.
    pub(super) fn bump(&mut self) -> Elem {
        let i = self.leaf();
        let t = &self.toks[i as usize];
        if t.kind == Kind::Punct || t.kind == Kind::SetLbrace {
            match t.text.as_str() {
                "(" | "[" | "{" | "set{" => self.delim += 1,
                ")" | "]" | "}" => self.delim = self.delim.saturating_sub(1),
                _ => {}
            }
        }
        if self.pos + 1 < self.sig.len() {
            self.pos += 1;
        }
        Elem::Token(i)
    }

    pub(super) fn eat_punct(&mut self, s: &str) -> Option<Elem> {
        if self.at_punct(s) {
            Some(self.bump())
        } else {
            None
        }
    }

    pub(super) fn eat_kw(&mut self, s: &str) -> Option<Elem> {
        if self.at_kw(s) {
            Some(self.bump())
        } else {
            None
        }
    }

    /// Kind của token **đã ăn gần nhất** (bỏ chú thích); `None` khi chưa ăn gì.
    pub(super) fn last_kind(&self) -> Option<Kind> {
        self.pos
            .checked_sub(1)
            .map(|i| self.toks[self.sig[i] as usize].kind)
    }

    /// Bắt buộc kết thúc dòng sau một câu lệnh/item (§A.2: `expr_stmt := expr NEWLINE`,
    /// `suite := simple_stmt NEWLINE`): ăn `NEWLINE` nếu có; chấp nhận khi câu lệnh vừa kết thúc
    /// bằng khối (`DEDENT`), khi **đang đứng trước `DEDENT`/EOF** (lexer sinh DEDENT ngầm ở EOF
    /// mà **không** sinh NEWLINE trước nó — SPEC-lex §4.6), hoặc khi token trước đã là
    /// `NEWLINE`/`DEDENT`. Nhiều hơn **một** `NEWLINE` liền nhau là lỗi (dòng trống không sinh
    /// token — SPEC-lex §4.2).
    pub(super) fn require_line_end(&mut self, elems: &mut Vec<Elem>) -> PResult<()> {
        if self.at_kind(Kind::Newline) {
            elems.push(self.bump());
            return Ok(());
        }
        if self.at_eof()
            || self.at_kind(Kind::Dedent)
            || matches!(self.last_kind(), Some(Kind::Newline | Kind::Dedent))
        {
            return Ok(());
        }
        Err(self.err_here(format!(
            "mong xuống dòng sau câu lệnh, thật `{}`",
            self.tok().text
        )))
    }

    pub(super) fn expect_punct(&mut self, s: &str) -> PResult<Elem> {
        if self.at_punct(s) {
            Ok(self.bump())
        } else {
            Err(self.err_here(format!("mong `{s}`, thật `{}`", self.tok().text)))
        }
    }

    pub(super) fn expect_kw(&mut self, s: &str) -> PResult<Elem> {
        if self.at_kw(s) {
            Ok(self.bump())
        } else {
            Err(self.err_here(format!("mong từ khoá `{s}`, thật `{}`", self.tok().text)))
        }
    }

    pub(super) fn expect_ident(&mut self) -> PResult<Elem> {
        if self.tok().kind == Kind::Ident {
            Ok(self.bump())
        } else {
            Err(self.err_here(format!("mong định danh, thật `{}`", self.tok().text)))
        }
    }

    pub(super) fn expect_newline(&mut self) -> PResult<Elem> {
        if self.at_kind(Kind::Newline) || self.at_eof() {
            Ok(self.bump())
        } else {
            Err(self.err_here(format!("mong xuống dòng, thật `{}`", self.tok().text)))
        }
    }

    /// Ăn một trong các dấu câu (dùng cho nhóm toán tử cùng mức).
    pub(super) fn eat_any_punct(&mut self, ops: &[&str]) -> Option<Elem> {
        if ops.iter().any(|s| self.at_punct(s)) {
            Some(self.bump())
        } else {
            None
        }
    }

    /// Token không mong đợi ⇒ VS0201.
    pub(super) fn err_here(&mut self, msg: impl Into<String>) -> PErr {
        self.err_code(&vis_diag::SYNTAX_CODES[0], msg)
    }

    pub(crate) fn err_code(&self, code: &'static vis_diag::Code, msg: impl Into<String>) -> PErr {
        let t = self.tok();
        PErr(Diagnostic {
            code,
            message: msg.into(),
            span: t.span,
            line: t.line,
            col: t.col,
        })
    }

    // ----- dựng node -----

    pub(super) fn node(&self, kind: NodeKind, elems: Vec<Elem>) -> Node {
        let span = self.elems_span(&elems);
        Node::new(kind, span, elems)
    }

    pub(super) fn elems_span(&self, elems: &[Elem]) -> Span {
        let first = elems.iter().find_map(|e| self.elem_span(e));
        let last = elems.iter().rev().find_map(|e| self.elem_span(e));
        match (first, last) {
            (Some(a), Some(b)) => Span::new(a.start, b.end),
            _ => Span::new(self.tok().span.start, self.tok().span.start),
        }
    }

    pub(super) fn elem_span(&self, e: &Elem) -> Option<Span> {
        match e {
            Elem::Token(i) => Some(self.toks[*i as usize].span),
            Elem::Node(n) => Some(n.span),
        }
    }

    pub(super) fn push_node(elems: &mut Vec<Elem>, n: Node) {
        elems.push(Elem::Node(Box::new(n)));
    }

    /// `Program` chứa **toàn bộ** token còn lại (đường lỗi/`Mode::Command`): CST vẫn lossless.
    pub(super) fn raw_program(&mut self) -> Node {
        let mut elems = Vec::new();
        while !self.at_eof() {
            elems.push(self.bump());
        }
        let span = self.elems_span(&elems);
        Node::new(NodeKind::Program, span, elems)
    }

    pub(super) fn raw_program_from_current(&mut self) -> Node {
        self.raw_program()
    }
}
