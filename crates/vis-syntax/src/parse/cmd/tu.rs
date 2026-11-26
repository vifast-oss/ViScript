// SPDX-License-Identifier: MIT OR Apache-2.0
//! **Từ** của chế độ lệnh: ghép token liền kề, cờ `-l`/`--x=v`, glob class, biểu thức con.
//!
//! **ST-318f** — tách **nguyên văn** khỏi `parse/cmd.rs` (R1 mềm): chỉ đổi vị trí,
//! sexp/CST/thứ tự chẩn đoán/API không đổi.

use crate::cst::{Elem, NodeKind};
use crate::lex::token::{Kind, Span};
use crate::parse::{PResult, Parser};

impl<'a> Parser<'a> {
    /// Một **từ** của chế độ lệnh: mọi token **liền kề span** trong vùng từ (`WORD`, `[`…`]`, …)
    /// gộp thành **một** `(word …)`; có khoảng trắng ⇒ tách (VD-40 #3, review vòng 1).
    pub(super) fn cmd_word_run(&mut self) -> PResult<(Elem, String)> {
        let start = self.tok().span.start;
        let mut elems = Vec::new();
        let mut end = start;
        loop {
            if self.at_kind(Kind::Word) {
                let sp = self.tok().span;
                elems.push(self.bump());
                end = sp.end;
            } else if self.at_punct("[") {
                self.cmd_glob_group(&mut elems, &mut end)?;
            } else {
                break;
            }
            let t = self.tok();
            if t.kind == Kind::Eof || t.span.start != end {
                break;
            }
            if !(t.kind == Kind::Word || t.is_punct("[")) {
                break;
            }
        }
        let text = self.text_at(Span::new(start, end));
        let sexp = self.cmd_word_sexp(&text)?;
        let node = self.node(NodeKind::CmdWord, elems);
        Ok((Elem::Node(Box::new(node)), sexp))
    }

    /// `-l` / `--oneline` / `--` là cờ; `-5` là **từ trần**; `--x=expr` tách tại `=` **đầu tiên**.
    fn cmd_word_sexp(&mut self, text: &str) -> PResult<String> {
        let co_gach = text == "--"
            || (text.starts_with("--") && text.len() > 2)
            || (text.starts_with('-')
                && text.len() > 1
                && !text[1..].starts_with(|c: char| c.is_ascii_digit()));
        if co_gach {
            if let Some((name, val)) = text.split_once('=') {
                let v = self.sub_expr(val.trim())?;
                return Ok(format!("(flag {name} {v})"));
            }
            return Ok(format!("(flag {text})"));
        }
        Ok(format!("(word {text})"))
    }

    /// Cụm `[` … `]` (glob char-class / list?) — lexer cắt token; ghép khi span **liền kề**.
    fn cmd_glob_group(&mut self, elems: &mut Vec<Elem>, end: &mut usize) -> PResult<()> {
        let open = self.bump();
        let open_span = self.tok_span(&open);
        elems.push(open);
        *end = open_span.end;
        loop {
            let t = self.tok();
            if t.kind == Kind::Eof || t.kind == Kind::Newline {
                return Err(self.err_here("thiếu `]` đóng glob class"));
            }
            if t.span.start != *end {
                return Err(self.err_here("glob `[ … ]` không được có khoảng trắng bên trong"));
            }
            if t.is_punct("]") {
                elems.push(self.bump());
                *end = t.span.end;
                return Ok(());
            }
            if t.is_punct("(") || t.is_punct("{") || t.is_punct(";") || t.is_punct("|") {
                return Err(self.err_here(format!("glob class không hợp lệ: `{}`", t.text)));
            }
            elems.push(self.bump());
            *end = t.span.end;
        }
    }

    fn sub_expr(&mut self, slice: &str) -> PResult<String> {
        let sub = crate::parse::parse(slice, crate::lex::Mode::Script);
        if let Some(d) = sub.diagnostics.first() {
            return Err(self.err_here(format!("trong biểu thức: {}", d.message)));
        }
        sub.cst
            .sexp_expr()
            .ok_or_else(|| self.err_here("không phải biểu thức"))
    }
}
