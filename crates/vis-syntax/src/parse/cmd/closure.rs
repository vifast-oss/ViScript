// SPDX-License-Identifier: MIT OR Apache-2.0
//! **Closure** chế độ lệnh: `x => thân`, `x= > thân` (A.7).
//!
//! **ST-318f** — tách **nguyên văn** khỏi `parse/cmd.rs` (R1 mềm): chỉ đổi vị trí,
//! sexp/CST/thứ tự chẩn đoán/API không đổi.

use crate::cst::{Elem, NodeKind};
use crate::lex::token::{Kind, Span};
use crate::parse::{PResult, Parser};

impl<'a> Parser<'a> {
    pub(super) fn cmd_closure_ahead(&self) -> bool {
        if !self.at_kind(Kind::Word) {
            return false;
        }
        if self.tok_at(1).kind == Kind::Word
            && self.tok_at(1).text == "="
            && self.tok_at(2).is_punct(">")
        {
            return true;
        }
        let t = &self.tok().text;
        t.ends_with('=')
            && t.len() > 1
            && self.tok_at(1).is_punct(">")
            && t[..t.len() - 1]
                .chars()
                .all(|c| c == '_' || c.is_alphanumeric())
    }

    pub(super) fn cmd_closure(&mut self) -> PResult<(Elem, String)> {
        let mut elems = Vec::new();
        let first = self.bump();
        let raw = self.text_at(self.tok_span(&first));
        let name = raw.trim_end_matches('=').to_string();
        elems.push(first);
        if !raw.ends_with('=') {
            if self.at_kind(Kind::Word) && self.tok().text == "=" {
                elems.push(self.bump());
            } else {
                return Err(self.err_here("mong `=>` của closure"));
            }
        }
        let gt = self.expect_punct(">")?;
        let gt_span = self.tok_span(&gt);
        elems.push(gt);
        let mut depth = 0usize;
        let mut last_end = gt_span.end;
        while !self.at_eof() {
            let t = self.tok();
            if t.kind == Kind::Newline {
                break;
            }
            if depth == 0 && (t.is_punct("|") || t.is_punct(";") || t.is_punct("&")) {
                break;
            }
            // ST-39: cờ `--x` **không** thuộc thân closure: `sort_by x => x --desc` ⇒ thân `x`,
            // cờ `--desc` ở stage (nếu nuốt vào thân, bộ hạ đọc `x --desc` thành `-(-desc)`).
            if depth == 0 && t.text.starts_with("--") {
                break;
            }
            if t.is_punct("(") || t.is_punct("[") || t.is_punct("{") {
                depth += 1;
            } else if t.is_punct(")") || t.is_punct("]") || t.is_punct("}") {
                depth = depth.saturating_sub(1);
            }
            last_end = t.span.end;
            elems.push(self.bump());
        }
        if elems.len() <= 3 {
            return Err(self.err_here("closure thiếu thân"));
        }
        let slice = self.text_at(Span::new(gt_span.end, last_end));
        let slice = slice.trim();
        let sub = crate::parse::parse(slice, crate::lex::Mode::Script);
        if let Some(d) = sub.diagnostics.first() {
            return Err(self.err_here(format!("trong thân closure: {}", d.message)));
        }
        let body = sub
            .cst
            .sexp_expr()
            .ok_or_else(|| self.err_here("thân closure không phải biểu thức"))?;
        let node = self.node(NodeKind::CmdClosure, elems);
        Ok((
            Elem::Node(Box::new(node)),
            format!("(closure {name} {body})"),
        ))
    }
}
