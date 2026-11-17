// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-116**: nhóm khai báo cấp tệp của parser (A.1) — tách khỏi `parse/mod.rs` cho luật R1.
//! Chỉ **di chuyển** mã; logic/thứ tự/API không đổi (ST-114a `file` vẫn do lex/parse gắn).

use super::*;

impl<'a> Parser<'a> {
    pub(super) fn fn_decl(&mut self, mods: Vec<Elem>) -> PResult<Node> {
        let mut elems = mods;
        elems.push(self.expect_kw("fn")?);
        elems.push(self.expect_ident()?);
        if self.at_punct("[") {
            let g = self.generics()?;
            Self::push_node(&mut elems, g);
        }
        elems.push(self.expect_punct("(")?);
        if !self.at_punct(")") {
            let p = self.params()?;
            Self::push_node(&mut elems, p);
        }
        elems.push(self.expect_punct(")")?);
        if self.eat_punct("->").is_some() {
            let t = self.parse_type()?;
            Self::push_node(&mut elems, t);
        }
        elems.push(self.expect_punct(":")?);
        let s = self.suite()?;
        Self::push_node(&mut elems, s);
        Ok(self.node(NodeKind::FnDecl, elems))
    }

    pub(super) fn cmd_decl(&mut self, mods: Vec<Elem>) -> PResult<Node> {
        let mut elems = mods;
        elems.push(self.expect_kw("cmd")?);
        elems.push(self.expect_ident()?);
        elems.push(self.expect_punct("(")?);
        if !self.at_punct(")") {
            let p = self.params()?;
            Self::push_node(&mut elems, p);
        }
        elems.push(self.expect_punct(")")?);
        if self.eat_punct("->").is_some() {
            let t = self.parse_type()?;
            Self::push_node(&mut elems, t);
        }
        elems.push(self.expect_punct(":")?);
        let s = self.suite()?;
        Self::push_node(&mut elems, s);
        Ok(self.node(NodeKind::CmdDecl, elems))
    }

    pub(super) fn extern_decl(&mut self) -> PResult<Node> {
        let mut elems = vec![self.expect_kw("extern")?, self.expect_kw("cmd")?];
        elems.push(self.expect_ident()?);
        elems.push(self.expect_punct("(")?);
        if !self.at_punct(")") {
            let p = self.params()?;
            Self::push_node(&mut elems, p);
        }
        elems.push(self.expect_punct(")")?);
        Ok(self.node(NodeKind::ExternDecl, elems))
    }

    pub(super) fn test_decl(&mut self) -> PResult<Node> {
        let mut elems = vec![self.bump()]; // `test` là IDENT (từ khoá theo ngữ cảnh)
        elems.push(self.expect_kw("fn")?);
        elems.push(self.expect_ident()?);
        elems.push(self.expect_punct("(")?);
        elems.push(self.expect_punct(")")?);
        elems.push(self.expect_punct(":")?);
        let s = self.suite()?;
        Self::push_node(&mut elems, s);
        Ok(self.node(NodeKind::TestDecl, elems))
    }

    pub(super) fn type_decl(&mut self, mods: Vec<Elem>) -> PResult<Node> {
        let mut elems = mods;
        elems.push(self.expect_kw("type")?);
        elems.push(self.expect_ident()?);
        if self.at_punct("[") {
            let g = self.generics()?;
            Self::push_node(&mut elems, g);
        }
        if self.at_kw("derives") {
            let d = self.derives()?;
            Self::push_node(&mut elems, d);
        }
        // **R12a (VD-178)**: `type X = T` (thay `alias`) và `type Email(str)` (thay `newtype`).
        if self.eat_punct("=").is_some() {
            let t = self.parse_type()?;
            Self::push_node(&mut elems, t);
            return Ok(self.node(NodeKind::AliasDecl, elems));
        }
        if self.eat_punct("(").is_some() {
            let t = self.parse_type()?;
            Self::push_node(&mut elems, t);
            elems.push(self.expect_punct(")")?);
            return Ok(self.node(NodeKind::NewtypeDecl, elems));
        }
        elems.push(self.expect_punct(":")?);
        elems.push(self.expect_newline()?);
        elems.push(self.expect_indent()?);
        while !self.at_kind(Kind::Dedent) && !self.at_eof() {
            let f = self.type_field()?;
            Self::push_node(&mut elems, f);
            self.require_line_end(&mut elems)?;
        }
        if let Some(d) = self.take_dedent() {
            elems.push(d);
        }
        Ok(self.node(NodeKind::TypeDecl, elems))
    }

    pub(super) fn type_field(&mut self) -> PResult<Node> {
        let mut elems = vec![self.expect_ident()?, self.expect_punct(":")?];
        let t = self.parse_type()?;
        Self::push_node(&mut elems, t);
        if self.eat_punct("=").is_some() {
            let v = self.expr()?;
            Self::push_node(&mut elems, v);
        }
        Ok(self.node(NodeKind::TypeField, elems))
    }

    pub(super) fn enum_decl(&mut self, mods: Vec<Elem>) -> PResult<Node> {
        let mut elems = mods;
        elems.push(self.expect_kw("enum")?);
        elems.push(self.expect_ident()?);
        if self.at_punct("[") {
            let g = self.generics()?;
            Self::push_node(&mut elems, g);
        }
        if self.at_kw("derives") {
            let d = self.derives()?;
            Self::push_node(&mut elems, d);
        }
        // **R12a (VD-178)**: `type X = T` (thay `alias`) và `type Email(str)` (thay `newtype`).
        if self.eat_punct("=").is_some() {
            let t = self.parse_type()?;
            Self::push_node(&mut elems, t);
            return Ok(self.node(NodeKind::AliasDecl, elems));
        }
        if self.eat_punct("(").is_some() {
            let t = self.parse_type()?;
            Self::push_node(&mut elems, t);
            elems.push(self.expect_punct(")")?);
            return Ok(self.node(NodeKind::NewtypeDecl, elems));
        }
        elems.push(self.expect_punct(":")?);
        elems.push(self.expect_newline()?);
        elems.push(self.expect_indent()?);
        while !self.at_kind(Kind::Dedent) && !self.at_eof() {
            let v = self.variant()?;
            Self::push_node(&mut elems, v);
            // **R11 (VD-178)**: biến thể **cùng dòng**, ngăn bằng `,` (`Circle(float), Empty`).
            if self.eat_punct(",").is_some() {
                continue;
            }
            self.require_line_end(&mut elems)?;
        }
        if let Some(d) = self.take_dedent() {
            elems.push(d);
        }
        Ok(self.node(NodeKind::EnumDecl, elems))
    }

    pub(super) fn variant(&mut self) -> PResult<Node> {
        let mut elems = vec![self.expect_ident()?];
        if self.at_punct("(") {
            elems.push(self.bump());
            while !self.at_punct(")") {
                // **R11 (VD-178)**: `Circle(float)` — trường **vị trí** (không tên);
                // `Circle(r: float)` vẫn là trường **có tên**.
                if self.tok().kind == Kind::Ident && self.peek_punct(1, ":") {
                    let mut f = vec![self.expect_ident()?, self.expect_punct(":")?];
                    let t = self.parse_type()?;
                    Self::push_node(&mut f, t);
                    Self::push_node(&mut elems, self.node(NodeKind::VariantField, f));
                } else {
                    let t = self.parse_type()?;
                    let mut f = Vec::new();
                    Self::push_node(&mut f, t);
                    Self::push_node(&mut elems, self.node(NodeKind::VariantField, f));
                }
                if self.eat_punct(",").is_none() {
                    break;
                }
            }
            elems.push(self.expect_punct(")")?);
        }
        Ok(self.node(NodeKind::Variant, elems))
    }

    pub(super) fn derives(&mut self) -> PResult<Node> {
        let mut elems = vec![self.bump()]; // derives
        let p = self.dotted_path()?;
        Self::push_node(&mut elems, p);
        while self.eat_punct(",").is_some() {
            let p = self.dotted_path()?;
            Self::push_node(&mut elems, p);
        }
        Ok(self.node(NodeKind::Derives, elems))
    }

    pub(super) fn trait_decl(&mut self, mods: Vec<Elem>) -> PResult<Node> {
        let mut elems = mods;
        elems.push(self.expect_kw("trait")?);
        elems.push(self.expect_ident()?);
        if self.at_punct("[") {
            let g = self.generics()?;
            Self::push_node(&mut elems, g);
        }
        elems.push(self.expect_punct(":")?);
        // `trait T: A, B:` — danh sách siêu trait trước dấu `:` cuối.
        if !self.at_kind(Kind::Newline) {
            loop {
                let p = self.dotted_path()?;
                Self::push_node(&mut elems, p);
                if self.eat_punct(",").is_none() {
                    break;
                }
            }
            elems.push(self.expect_punct(":")?);
        }
        elems.push(self.expect_newline()?);
        elems.push(self.expect_indent()?);
        while !self.at_kind(Kind::Dedent) && !self.at_eof() {
            let it = self.trait_item()?;
            Self::push_node(&mut elems, it);
            self.require_line_end(&mut elems)?;
        }
        if let Some(d) = self.take_dedent() {
            elems.push(d);
        }
        Ok(self.node(NodeKind::TraitDecl, elems))
    }

    pub(super) fn trait_item(&mut self) -> PResult<Node> {
        let mut elems = Vec::new();
        if self.at_kw("pub") {
            elems.push(self.bump());
        }
        elems.push(self.expect_kw("fn")?);
        elems.push(self.expect_ident()?);
        elems.push(self.expect_punct("(")?);
        if !self.at_punct(")") {
            let p = self.params()?;
            Self::push_node(&mut elems, p);
        }
        elems.push(self.expect_punct(")")?);
        if self.eat_punct("->").is_some() {
            let t = self.parse_type()?;
            Self::push_node(&mut elems, t);
        }
        if self.eat_punct(":").is_some() {
            let s = self.suite()?;
            Self::push_node(&mut elems, s);
        }
        Ok(self.node(NodeKind::TraitItem, elems))
    }

    pub(super) fn impl_decl(&mut self) -> PResult<Node> {
        let mut elems = vec![self.expect_kw("impl")?];
        if self.at_kind(Kind::Ident) && self.peek_kw(1, "for") {
            let p = self.dotted_path()?;
            Self::push_node(&mut elems, p);
            elems.push(self.expect_kw("for")?);
        }
        let t = self.parse_type()?;
        Self::push_node(&mut elems, t);
        if self.at_punct("[") {
            let g = self.generics()?;
            Self::push_node(&mut elems, g);
        }
        elems.push(self.expect_punct(":")?);
        elems.push(self.expect_newline()?);
        elems.push(self.expect_indent()?);
        while !self.at_kind(Kind::Dedent) && !self.at_eof() {
            let m = self.fn_decl(Vec::new())?;
            Self::push_node(&mut elems, m);
            self.require_line_end(&mut elems)?;
        }
        if let Some(d) = self.take_dedent() {
            elems.push(d);
        }
        Ok(self.node(NodeKind::ImplDecl, elems))
    }

    pub(super) fn const_decl(&mut self, mods: Vec<Elem>) -> PResult<Node> {
        let mut elems = mods;
        elems.push(self.expect_kw("const")?);
        elems.push(self.expect_ident()?);
        // **R9 (VD-178)**: gợi ý cho mẫu `const A = …` (VS0204 khi RHS không literal; R7 cho
        // suy kiểu từ literal).
        if !self.at_punct(":") {
            return Err(self.err_code(
                &vis_diag::SYNTAX_CODES[3],
                "`const` cần kiểu khi vế phải không phải literal — gợi ý: `const A: int = 1` (R7 cho suy kiểu từ literal)",
            ));
        }
        elems.push(self.expect_punct(":")?);
        let t = self.parse_type()?;
        Self::push_node(&mut elems, t);
        elems.push(self.expect_punct("=")?);
        let v = self.expr()?;
        Self::push_node(&mut elems, v);
        Ok(self.node(NodeKind::ConstDecl, elems))
    }

    // ----- tham số, generic, kiểu -----

    pub(super) fn params(&mut self) -> PResult<Node> {
        let mut elems = Vec::new();
        loop {
            let p = self.param()?;
            Self::push_node(&mut elems, p);
            if self.eat_punct(",").is_none() {
                break;
            }
        }
        Ok(self.node(NodeKind::Params, elems))
    }

    pub(super) fn param(&mut self) -> PResult<Node> {
        let mut elems = Vec::new();
        // ST-27: `...rest: str` — lexer không có token `...` (`..` rồi `.`), nhận cả hai.
        if self.at_punct("..") {
            elems.push(self.bump());
            if self.at_punct(".") {
                elems.push(self.bump());
            }
        }
        if self.at_kw("var") && self.peek_kw(1, "self") {
            elems.push(self.bump());
            elems.push(self.bump());
            return Ok(self.node(NodeKind::Param, elems));
        }
        // **ST-56 (VD-92)**: `&self` / `& self` — receiver **không mượn thật**, hạ **y như** `self`.
        // `&` **chưa** là toán tử tiền tố (SPEC-lex §4) nên chỉ nhận đúng dạng này ở vị trí tham số;
        // mọi chỗ khác `&` vẫn là lỗi cú pháp như trước.
        if self.at_punct("&") && self.peek_kw(1, "self") {
            elems.push(self.bump());
            elems.push(self.bump());
            return Ok(self.node(NodeKind::Param, elems));
        }
        if self.at_kw("self") {
            elems.push(self.bump());
            return Ok(self.node(NodeKind::Param, elems));
        }
        // Tham số có tên: `--ten` (lexer tách `-` `-` vì §1 không có token `--`).
        if self.at_punct("-") && self.peek_punct(1, "-") {
            elems.push(self.bump());
            elems.push(self.bump());
        }
        elems.push(self.expect_ident()?);
        elems.push(self.expect_punct(":")?);
        let t = self.parse_type()?;
        Self::push_node(&mut elems, t);
        // `@ path_lit` (capability) và danh sách quyền `(write)`.
        if self.eat_punct("@").is_some() {
            let t = self.tok();
            if matches!(t.kind, Kind::Path | Kind::Str) {
                elems.push(self.bump());
            } else {
                return Err(self.err_here("`@` phải theo sau là literal đường dẫn"));
            }
        }
        if self.at_punct("(") {
            let g = self.grant_list()?;
            Self::push_node(&mut elems, g);
        }
        if self.eat_punct("=").is_some() {
            let v = self.expr()?;
            Self::push_node(&mut elems, v);
        }
        Ok(self.node(NodeKind::Param, elems))
    }

    pub(super) fn grant_list(&mut self) -> PResult<Node> {
        let mut elems = vec![self.expect_punct("(")?];
        loop {
            let p = self.dotted_path()?;
            Self::push_node(&mut elems, p);
            if self.eat_punct(",").is_none() {
                break;
            }
        }
        elems.push(self.expect_punct(")")?);
        Ok(self.node(NodeKind::GrantList, elems))
    }

    pub(super) fn generics(&mut self) -> PResult<Node> {
        let mut elems = vec![self.expect_punct("[")?];
        loop {
            let mut g = vec![self.expect_ident()?];
            if self.eat_punct(":").is_some() {
                if self.at_kw("has") {
                    g.push(self.bump());
                    let rt = self.record_type()?;
                    Self::push_node(&mut g, rt);
                } else {
                    let p = self.dotted_path()?;
                    Self::push_node(&mut g, p);
                }
            }
            Self::push_node(&mut elems, self.node(NodeKind::Generic, g));
            if self.eat_punct(",").is_none() {
                break;
            }
        }
        elems.push(self.expect_punct("]")?);
        Ok(self.node(NodeKind::Generics, elems))
    }

    pub(super) fn record_type(&mut self) -> PResult<Node> {
        let mut elems = vec![self.expect_punct("{")?];
        while !self.at_punct("}") {
            let mut f = vec![self.expect_ident()?, self.expect_punct(":")?];
            let t = self.parse_type()?;
            Self::push_node(&mut f, t);
            Self::push_node(&mut elems, self.node(NodeKind::TypeField, f));
            if self.eat_punct(",").is_none() {
                break;
            }
        }
        elems.push(self.expect_punct("}")?);
        Ok(self.node(NodeKind::Type, elems))
    }
}
