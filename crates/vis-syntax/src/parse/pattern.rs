// SPDX-License-Identifier: MIT OR Apache-2.0
//! Cú pháp **mẫu** (A.4): `parse_pattern`, khoảng `..`/`..=`, mẫu nguyên tử.
//!
//! **ST-318f** — tách **nguyên văn** khỏi `parse/mod.rs` (R1 mềm): chỉ đổi vị trí,
//! token/AST/CST/thứ tự chẩn đoán/API không đổi.

use super::{PResult, Parser};
use crate::cst::{Elem, Node, NodeKind};
use crate::lex::token::Kind;

impl<'a> Parser<'a> {
    // ----- A.4 pattern -----

    pub(crate) fn parse_pattern(&mut self) -> PResult<Node> {
        let mut elems = Vec::new();
        let first = self.range_pattern()?;
        Self::push_node(&mut elems, first);
        while self.at_punct("|") {
            elems.push(self.bump());
            let r = self.range_pattern()?;
            Self::push_node(&mut elems, r);
        }
        Ok(self.node(NodeKind::Pattern, elems))
    }

    fn range_pattern(&mut self) -> PResult<Node> {
        let p = self.atom_pattern()?;
        if self.at_punct("..") || self.at_punct("..=") {
            let mut elems = vec![Elem::Node(Box::new(p))];
            elems.push(self.bump());
            if !matches!(self.tok().kind, Kind::Newline | Kind::Eof | Kind::Dedent)
                && !self.at_punct(":")
                && !self.at_punct(")")
                && !self.at_punct("]")
                && !self.at_punct("}")
                && !self.at_punct(",")
                && !self.at_punct("|")
                && !self.at_kw("if")
                && !self.at_kw("in")
            {
                let q = self.atom_pattern()?;
                Self::push_node(&mut elems, q);
            }
            return Ok(self.node(NodeKind::Pattern, elems));
        }
        Ok(p)
    }

    fn atom_pattern(&mut self) -> PResult<Node> {
        let t = self.tok();
        match t.kind {
            Kind::Ident => {
                let mut elems = vec![self.bump()];
                // biến thể có trường: `Circle(r)`
                if self.at_punct("(") {
                    elems.push(self.bump());
                    while !self.at_punct(")") {
                        let p = self.parse_pattern()?;
                        Self::push_node(&mut elems, p);
                        if self.eat_punct(",").is_none() {
                            break;
                        }
                    }
                    elems.push(self.expect_punct(")")?);
                }
                Ok(self.node(NodeKind::Pattern, elems))
            }
            Kind::Kw if t.text == "none" || t.text == "true" || t.text == "false" => {
                let e = self.bump();
                Ok(self.node(NodeKind::Pattern, vec![e]))
            }
            Kind::Int
            | Kind::Float
            | Kind::Char
            | Kind::Str
            | Kind::RawStr
            | Kind::MultiStr
            | Kind::RawMulti
            | Kind::Bytes
            | Kind::Size
            | Kind::Duration
            | Kind::Percent => {
                let e = self.bump();
                Ok(self.node(NodeKind::Pattern, vec![e]))
            }
            Kind::Punct if t.text == "-" => {
                let mut elems = vec![self.bump()];
                if self.at_kind(Kind::Int) {
                    elems.push(self.bump());
                }
                Ok(self.node(NodeKind::Pattern, elems))
            }
            Kind::Punct if t.text == "(" => {
                let mut elems = vec![self.bump()];
                if !self.at_punct(")") {
                    loop {
                        let p = self.parse_pattern()?;
                        Self::push_node(&mut elems, p);
                        if self.eat_punct(",").is_none() {
                            break;
                        }
                    }
                }
                elems.push(self.expect_punct(")")?);
                Ok(self.node(NodeKind::Pattern, elems))
            }
            Kind::Punct if t.text == "[" => {
                let mut elems = vec![self.bump()];
                while !self.at_punct("]") {
                    if self.at_punct("..") {
                        elems.push(self.bump());
                        if self.tok().kind == Kind::Ident {
                            elems.push(self.bump());
                        }
                        break;
                    }
                    let p = self.parse_pattern()?;
                    Self::push_node(&mut elems, p);
                    if self.eat_punct(",").is_none() {
                        break;
                    }
                }
                elems.push(self.expect_punct("]")?);
                Ok(self.node(NodeKind::Pattern, elems))
            }
            Kind::Punct if t.text == "{" => {
                let mut elems = vec![self.bump()];
                while !self.at_punct("}") {
                    if self.at_punct("..") {
                        elems.push(self.bump());
                        break;
                    }
                    elems.push(self.expect_ident()?);
                    if self.eat_punct(":").is_some() {
                        elems.push(self.expect_punct(":")?);
                        let p = self.parse_pattern()?;
                        Self::push_node(&mut elems, p);
                    }
                    if self.eat_punct(",").is_none() {
                        break;
                    }
                }
                elems.push(self.expect_punct("}")?);
                Ok(self.node(NodeKind::Pattern, elems))
            }
            _ => Err(self.err_here(format!(
                "mong pattern, thật `{}` ({})",
                t.text,
                t.kind.name()
            ))),
        }
    }
}
