// SPDX-License-Identifier: MIT OR Apache-2.0
//! Cú pháp **kiểu** (A.5): `parse_type` — `ref`/`weak`/`cap`/`fn`, generic, bản ghi, hậu tố `?`.
//!
//! **ST-318f** — tách **nguyên văn** khỏi `parse/mod.rs` (R1 mềm): chỉ đổi vị trí,
//! token/AST/CST/thứ tự chẩn đoán/API không đổi.

use super::{PResult, Parser};
use crate::cst::{Node, NodeKind};
use crate::lex::token::Kind;

impl<'a> Parser<'a> {
    /// `type := type_primary '?'?` (A.5); trả `NodeKind::Type` bọc toàn bộ.
    pub(crate) fn parse_type(&mut self) -> PResult<Node> {
        let mut elems = Vec::new();
        let t = self.tok();
        match t.kind {
            Kind::Ident => {
                let p = self.dotted_path()?;
                Self::push_node(&mut elems, p);
                if self.at_punct("[") {
                    let mut g = vec![self.bump()];
                    loop {
                        let ty = self.parse_type()?;
                        Self::push_node(&mut g, ty);
                        if self.eat_punct(",").is_none() {
                            break;
                        }
                    }
                    g.push(self.expect_punct("]")?);
                    Self::push_node(&mut elems, self.node(NodeKind::Generics, g));
                }
            }
            Kind::Kw if t.text == "ref" || t.text == "weak" => {
                elems.push(self.bump());
                elems.push(self.expect_punct("[")?);
                let inner = self.parse_type()?;
                Self::push_node(&mut elems, inner);
                elems.push(self.expect_punct("]")?);
            }
            Kind::Kw if t.text == "cap" => {
                elems.push(self.bump());
                elems.push(self.expect_punct("[")?);
                let p = self.dotted_path()?;
                Self::push_node(&mut elems, p);
                elems.push(self.expect_punct("]")?);
            }
            Kind::Kw if t.text == "fn" => {
                elems.push(self.bump());
                elems.push(self.expect_punct("(")?);
                if !self.at_punct(")") {
                    loop {
                        let ty = self.parse_type()?;
                        Self::push_node(&mut elems, ty);
                        if self.eat_punct(",").is_none() {
                            break;
                        }
                    }
                }
                elems.push(self.expect_punct(")")?);
                elems.push(self.expect_punct("->")?);
                let ty = self.parse_type()?;
                Self::push_node(&mut elems, ty);
            }
            Kind::Punct if t.text == "(" => {
                elems.push(self.bump());
                if !self.at_punct(")") {
                    loop {
                        let ty = self.parse_type()?;
                        Self::push_node(&mut elems, ty);
                        if self.eat_punct(",").is_none() {
                            break;
                        }
                    }
                }
                elems.push(self.expect_punct(")")?);
            }
            Kind::Punct if t.text == "{" => {
                let rt = self.record_type()?;
                elems.extend(rt.elems);
            }
            _ => {
                return Err(self.err_here(format!(
                    "mong kiểu, thật `{}` ({})",
                    t.text,
                    t.kind.name()
                )));
            }
        }
        // **R2 (VD-178)**: hậu tố optional lặp — `?` = 1 lớp, `??` (cặp token `??` của coalesce) = 2 lớp.
        // (`int??` lex thành `int` + `??`; không phải 2×`?`.)
        while self.at_punct("?") || self.at_punct("??") {
            elems.push(self.bump());
        }
        Ok(self.node(NodeKind::Type, elems))
    }
}
