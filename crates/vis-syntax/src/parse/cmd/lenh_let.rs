// SPDX-License-Identifier: MIT OR Apache-2.0
//! **Shell-let** chế độ lệnh: `let tên [: kiểu] = pipeline` (A.7).
//!
//! **ST-318f** — tách **nguyên văn** khỏi `parse/cmd.rs` (R1 mềm): chỉ đổi vị trí,
//! sexp/CST/thứ tự chẩn đoán/API không đổi.

use crate::cst::{Elem, NodeKind};
use crate::lex::token::{Kind, Span};
use crate::parse::{PResult, Parser};

impl<'a> Parser<'a> {
    pub(super) fn cmd_let(&mut self) -> PResult<(Elem, String)> {
        let mut elems = vec![self.bump()]; // `let`
        if !self.at_kind(Kind::Word) {
            return Err(self.err_here("mong tên sau `let`"));
        }
        // Chế độ lệnh: `:` và `=` **không** phải metachar nên có thể dính trong WORD
        // (`x:` / `x` + `:` / `=`), hoặc đứng rời — A.7 `shell_let := 'let' IDENT [':' type] '=' pipeline`.
        let raw = self.tok().text.clone();
        let mut name = raw.clone();
        let mut co_type = false;
        if name.ends_with(':') {
            name = name.trim_end_matches(':').to_string();
            co_type = true;
        }
        elems.push(self.bump());
        if !co_type && self.at_kind(Kind::Word) && self.tok().text == ":" {
            elems.push(self.bump());
            co_type = true;
        }
        let mut ty: Option<String> = None;
        if co_type {
            let tstart = self.tok().span.start;
            let mut tend = tstart;
            let mut depth = 0usize;
            loop {
                if depth == 0 && self.at_kind(Kind::Word) && self.tok().text == "=" {
                    break;
                }
                if self.cmd_at_line_end() || self.at_eof() {
                    return Err(self.err_here("thiếu `=` trong `let` (A.7)"));
                }
                let t = self.tok();
                if t.is_punct("(") || t.is_punct("[") || t.is_punct("{") {
                    depth += 1;
                } else if t.is_punct(")") || t.is_punct("]") || t.is_punct("}") {
                    depth = depth.saturating_sub(1);
                }
                tend = t.span.end;
                elems.push(self.bump());
            }
            let text = self.text_at(Span::new(tstart, tend));
            ty = Some(text.trim().to_string());
        }
        if !(self.at_kind(Kind::Word) && self.tok().text == "=") {
            return Err(self.err_here(format!(
                "mong `=` sau tên trong `let`, thật `{}`",
                self.tok().text
            )));
        }
        elems.push(self.bump());
        let (pipe, pipe_sexp) = self.cmd_pipeline()?;
        elems.push(pipe);
        let node = self.node(NodeKind::CmdLet, elems);
        let sexp = match ty {
            None => format!("(let {name} {pipe_sexp})"),
            Some(t) => format!("(let {name} {t} {pipe_sexp})"),
        };
        Ok((Elem::Node(Box::new(node)), sexp))
    }
}
