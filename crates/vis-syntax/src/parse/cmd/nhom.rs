// SPDX-License-Identifier: MIT OR Apache-2.0
//! Biến `$…`/`$env.…` và **nhóm** `( … )` của chế độ lệnh.
//!
//! **ST-318f** — tách **nguyên văn** khỏi `parse/cmd.rs` (R1 mềm): chỉ đổi vị trí,
//! sexp/CST/thứ tự chẩn đoán/API không đổi.

use crate::cst::{Elem, NodeKind};
use crate::lex::token::{Kind, Span};
use crate::parse::{PResult, Parser};

impl<'a> Parser<'a> {
    pub(super) fn cmd_dollar(&mut self) -> PResult<(Elem, String)> {
        let mut elems = vec![self.bump()]; // `$`
        let mut segs: Vec<String> = Vec::new();
        loop {
            if !(self.at_kind(Kind::Ident) || self.at_kind(Kind::Word)) {
                return Err(
                    self.err_here(format!("mong tên biến sau `$`, thật `{}`", self.tok().text))
                );
            }
            segs.push(self.tok().text.clone());
            elems.push(self.bump());
            if self.at_punct(".") {
                elems.push(self.bump());
                continue;
            }
            break;
        }
        let node = self.node(
            if segs.first().map(String::as_str) == Some("env") {
                NodeKind::CmdEnv
            } else {
                NodeKind::CmdVar
            },
            elems,
        );
        let sexp = if segs.len() == 2 && segs[0] == "env" {
            format!("(env {})", segs[1])
        } else {
            format!("(var {})", segs.join(" "))
        };
        Ok((Elem::Node(Box::new(node)), sexp))
    }

    pub(super) fn cmd_group(&mut self) -> PResult<(Elem, String)> {
        let open = self.bump(); // `(`
        let open_span = self.tok_span(&open);
        let mut k = 0usize;
        let mut depth = 1usize;
        loop {
            let t = self.tok_at(k);
            // **ST-49**: newline **không** kết thúc nhóm — `( 1 +\n 1 )` và
            // `( ls … |\n where … )` là **một** nhóm (tầng hạ đổi newline thành dấu cách).
            if t.kind == Kind::Eof {
                return Err(self.err_here("thiếu `)` đóng nhóm"));
            }
            if t.is_punct("(") {
                depth += 1;
            } else if t.is_punct(")") {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            k += 1;
        }
        let close = self.tok_at(k).span;
        let mut elems = vec![open];
        for _ in 0..k {
            elems.push(self.bump());
        }
        elems.push(self.bump()); // `)`
        let slice = self.text_at(Span::new(open_span.end, close.start));
        let slice = slice.trim();
        let sub = crate::parse::parse(slice, crate::lex::Mode::Script);
        if let Some(d) = sub.diagnostics.first() {
            // ST-46/ST-49: nhóm có thể là **biểu thức**, **một lệnh**, hay **pipeline lệnh**; parser
            // **không** từ chối nội dung — tầng hạ quyết định (báo `VS0201` nếu không hạ được).
            let _ = d;
            let node = self.node(NodeKind::CmdGroup, elems);
            return Ok((
                Elem::Node(Box::new(node)),
                format!("(group raw {})", slice.replace(' ', "_")),
            ));
        }
        // ST-44: `()` **rỗng** hợp lệ làm dấu **lời gọi không đối số** (`f() | where …`); tầng hạ
        // quyết định nghĩa (bỏ qua như đối số rỗng). Nhóm rỗng **đứng một mình** vẫn bị hạ báo lỗi.
        let inner = match sub.cst.sexp_expr() {
            Some(x) => x,
            None if slice.is_empty() => "()".to_string(),
            None => return Err(self.err_here("`( … )` không phải một biểu thức")),
        };
        let node = self.node(NodeKind::CmdGroup, elems);
        Ok((Elem::Node(Box::new(node)), format!("(group {inner})")))
    }
}
