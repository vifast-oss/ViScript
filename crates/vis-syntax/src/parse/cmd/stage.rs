// SPDX-License-Identifier: MIT OR Apache-2.0
//! **Pipeline** và **stage** lệnh: `cmd_pipeline`, `cmd_stage`, `cmd_arg`.
//!
//! **ST-318f** — tách **nguyên văn** khỏi `parse/cmd.rs` (R1 mềm): chỉ đổi vị trí,
//! sexp/CST/thứ tự chẩn đoán/API không đổi.

use crate::cst::{Elem, NodeKind};
use crate::lex::token::Kind;
use crate::parse::{PResult, Parser};

impl<'a> Parser<'a> {
    pub(super) fn cmd_pipeline(&mut self) -> PResult<(Elem, String)> {
        let mut elems = Vec::new();
        let mut ss = Vec::new();
        loop {
            let (e, s) = self.cmd_stage()?;
            elems.push(e);
            ss.push(s);
            if self.at_punct("|") {
                elems.push(self.bump());
                continue;
            }
            break;
        }
        let pipe_node = self.node(NodeKind::CmdPipe, elems);
        let pipe_sexp = format!("(pipe {})", ss.join(" "));
        if self.at_punct("&") {
            let amp = self.bump();
            if !self.cmd_at_line_end() {
                return Err(self.err_here("`&` nền chỉ được ở **cuối dòng** (VD-40 #1)"));
            }
            let e2 = vec![Elem::Node(Box::new(pipe_node)), amp];
            let n = self.node(NodeKind::CmdBg, e2);
            Ok((Elem::Node(Box::new(n)), format!("(bg {pipe_sexp})")))
        } else {
            Ok((Elem::Node(Box::new(pipe_node)), pipe_sexp))
        }
    }

    pub(super) fn cmd_stage(&mut self) -> PResult<(Elem, String)> {
        let mut elems = Vec::new();
        let mut parts = Vec::new();
        let mut head_la_group = false;
        if self.at_punct("(") {
            let (e, s) = self.cmd_group()?;
            elems.push(e);
            parts.push(s);
            head_la_group = true;
        } else if self.at_kind(Kind::Caret) {
            let c = self.bump();
            elems.push(c);
            if !self.at_kind(Kind::Word) {
                return Err(self.err_here(format!(
                    "mong tên chương trình sau `^`, thật `{}`",
                    self.tok().text
                )));
            }
            let name = self.tok().text.clone();
            elems.push(self.bump());
            parts.push(format!("(ext {name})"));
        } else if self.at_kind(Kind::Dollar) {
            let (e, s) = self.cmd_dollar()?;
            elems.push(e);
            parts.push(s);
        } else if self.at_kind(Kind::Word) {
            let name = self.tok().text.clone();
            elems.push(self.bump());
            parts.push(format!("(cmd {name})"));
        } else {
            return Err(self.err_here(format!("mong lệnh, thật `{}`", self.tok().text)));
        }
        if head_la_group {
            // A.7 `stage := word_cmd arg* | '(' expr ')'` — nhóm không nhận tham số.
            if !(self.cmd_at_line_end()
                || self.at_punct("|")
                || self.at_punct(";")
                || self.at_punct("&")
                || self.at_punct("|>"))
            {
                return Err(
                    self.err_here("stage `( … )` không nhận tham số (A.7: nhóm là cả stage)")
                );
            }
        } else {
            while !self.cmd_at_line_end()
                && !self.at_punct("|")
                && !self.at_punct(";")
                && !self.at_punct("&")
                && !self.at_punct("|>")
            {
                let (e, s) = self.cmd_arg()?;
                elems.push(e);
                parts.push(s);
            }
        }
        let node = self.node(NodeKind::CmdStage, elems);
        Ok((
            Elem::Node(Box::new(node)),
            format!("(stage {})", parts.join(" ")),
        ))
    }

    fn cmd_arg(&mut self) -> PResult<(Elem, String)> {
        if self.at_punct("(") {
            return self.cmd_group();
        }
        if self.at_punct("[") {
            return self.cmd_word_run();
        }
        if self.at_kind(Kind::Dollar) {
            return self.cmd_dollar();
        }
        match self.tok().kind {
            Kind::Str
            | Kind::RawStr
            | Kind::MultiStr
            | Kind::RawMulti
            | Kind::Bytes
            | Kind::Path => {
                let t = self.tok().text.clone();
                Ok((self.bump(), format!("(str {t})")))
            }
            Kind::Word => {
                if self.cmd_closure_ahead() {
                    return self.cmd_closure();
                }
                self.cmd_word_run()
            }
            // `>` `<` … ra token Punct nhưng ở CmdCST **chưa định kiểu** vẫn là "từ" để
            // `cmd_lower` quyết (so sánh/redirect/…). Xem VD-40 + JOURNAL ST-06.
            Kind::Punct
                if matches!(
                    self.tok().text.as_str(),
                    ">" | "<" | ">=" | "<=" | "==" | "!="
                ) =>
            {
                let t = self.tok().text.clone();
                Ok((self.bump(), format!("(word {t})")))
            }
            _ => Err(self.err_here(format!("mong tham số, thật `{}`", self.tok().text))),
        }
    }
}
