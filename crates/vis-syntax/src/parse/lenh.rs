// SPDX-License-Identifier: MIT OR Apache-2.0
//! Khối và **câu lệnh** (A.2): `suite`, `stmt`, `let`/`var`/`return`/gán, vòng lặp, `if`, `match`.
//!
//! **ST-318f** — tách **nguyên văn** khỏi `parse/mod.rs` (R1 mềm): chỉ đổi vị trí,
//! token/AST/CST/thứ tự chẩn đoán/API không đổi.

use super::{PResult, Parser};
use crate::cst::{Elem, Node, NodeKind};
use crate::lex::token::Kind;

impl<'a> Parser<'a> {
    // ----- A.2 khối và câu lệnh -----

    pub(super) fn expect_indent(&mut self) -> PResult<Elem> {
        if self.at_kind(Kind::Indent) {
            Ok(self.bump())
        } else {
            Err(self.err_here(format!(
                "mong khối thụt lề (INDENT), thật `{}`",
                self.tok().text
            )))
        }
    }

    pub(super) fn take_dedent(&mut self) -> Option<Elem> {
        if self.at_kind(Kind::Dedent) {
            Some(self.bump())
        } else {
            None
        }
    }

    /// `suite := simple_stmt NEWLINE | NEWLINE INDENT stmt+ DEDENT`.
    ///
    /// Khối một dòng nội tuyến (`if x: a else: b`) dừng ở `else`/`elif` cùng dòng — Phụ lục B
    /// dùng dạng này; A.2 viết `simple_stmt NEWLINE` nên đây là chỗ nới tối thiểu, ghi ở JOURNAL.
    ///
    /// **VD-266**: trong ngoặc (`delim > 0`) lexer **không** sinh NEWLINE/INDENT/DEDENT, nên khối
    /// một-dòng kết thúc ngầm ở dấu đóng `)`/`]`/`}`/`,` hoặc ở token đầu một dòng vật lý mới.
    pub(super) fn suite(&mut self) -> PResult<Node> {
        let mut elems = Vec::new();
        if self.at_kind(Kind::Newline) {
            elems.push(self.bump());
            elems.push(self.expect_indent()?);
            while !self.at_kind(Kind::Dedent) && !self.at_eof() {
                let s = self.stmt()?;
                Self::push_node(&mut elems, s);
                self.require_line_end(&mut elems)?;
            }
            if let Some(d) = self.take_dedent() {
                elems.push(d);
            }
        } else {
            let s = self.simple_stmt()?;
            Self::push_node(&mut elems, s);
            if self.at_kind(Kind::Newline) {
                elems.push(self.bump());
            } else if self.at_eof()
                || self.at_kind(Kind::Dedent)
                || self.at_kw("else")
                || self.at_kw("elif")
                || (self.delim > 0 && (self.at_delim_end() || self.at_new_line()))
            {
                // nhánh một dòng nội tuyến / khối đóng ở EOF (DEDENT ngầm) / khối trong ngoặc
            } else {
                return Err(self.err_here(format!(
                    "mong xuống dòng cuối khối, thật `{}`",
                    self.tok().text
                )));
            }
        }
        Ok(self.node(NodeKind::Suite, elems))
    }

    fn simple_stmt(&mut self) -> PResult<Node> {
        match self.tok().kind {
            Kind::Kw => match self.tok().text.as_str() {
                "return" => self.return_stmt(),
                "break" => self.break_stmt(),
                "continue" => self.continue_stmt(),
                "let" => self.let_stmt(),
                "var" => self.var_stmt(),
                _ => self.expr_or_assert(),
            },
            _ => self.expr_or_assert(),
        }
    }

    /// **ST-83 (VD-109)**: `assert` là **từ khoá mềm** ở **đầu câu lệnh**; `assert(đk)` vẫn là call.
    fn expr_or_assert(&mut self) -> PResult<Node> {
        if self.at_assert() {
            self.assert_stmt()
        } else {
            self.expr_stmt()
        }
    }

    fn at_assert(&self) -> bool {
        if self.tok().kind != Kind::Ident || self.tok().text != "assert" {
            return false;
        }
        let n = self.tok_at(1);
        if matches!(n.kind, Kind::Newline | Kind::Eof) {
            return false;
        }
        // `assert(...)`/`assert.x`/`assert[i]`/`assert = …` giữ đường **biểu thức** cũ.
        if matches!(n.text.as_str(), "(" | "." | "[" | "=") {
            return false;
        }
        // `assert += …` (gán ghép) cũng là biểu thức.
        if matches!(n.text.as_str(), "+" | "-" | "*" | "/" | "%") {
            let eq = self.tok_at(2);
            if eq.is_punct("=") && n.span.end == eq.span.start {
                return false;
            }
        }
        true
    }

    fn assert_stmt(&mut self) -> PResult<Node> {
        let mut elems = vec![self.bump()];
        let e = self.expr()?;
        Self::push_node(&mut elems, e);
        // **R20a (VD-178)**: `assert cond, "thông điệp"` — đối số thứ hai tuỳ chọn.
        if self.at_punct(",") {
            elems.push(self.bump());
            let m = self.expr()?;
            Self::push_node(&mut elems, m);
        }
        Ok(self.node(NodeKind::AssertStmt, elems))
    }

    pub(super) fn stmt(&mut self) -> PResult<Node> {
        match self.tok().kind {
            Kind::Kw => match self.tok().text.as_str() {
                "let" => self.let_stmt(),
                "var" => self.var_stmt(),
                "return" => self.return_stmt(),
                "break" => self.break_stmt(),
                "continue" => self.continue_stmt(),
                "for" => self.for_stmt(),
                "while" => self.while_stmt(),
                "loop" => self.loop_stmt(),
                "scope" => self.scope_stmt(),
                "if" => self.if_stmt(),
                "match" => self.match_stmt(),
                _ => self.expr_or_assert(),
            },
            _ => self.expr_or_assert(),
        }
    }

    fn let_stmt(&mut self) -> PResult<Node> {
        let mut elems = vec![self.expect_kw("let")?];
        let p = self.parse_pattern()?;
        Self::push_node(&mut elems, p);
        if self.eat_punct(":").is_some() {
            let t = self.parse_type()?;
            Self::push_node(&mut elems, t);
        }
        elems.push(self.expect_punct("=")?);
        let v = self.expr()?;
        Self::push_node(&mut elems, v);
        // `else [IDENT]: suite` — giá trị dự phòng khi lỗi (A.2).
        if self.at_kw("else") {
            elems.push(self.bump());
            if self.tok().kind == Kind::Ident && self.peek_punct(1, ":") {
                elems.push(self.bump());
            }
            elems.push(self.expect_punct(":")?);
            let s = self.suite()?;
            Self::push_node(&mut elems, s);
            return Ok(self.node(NodeKind::LetStmt, elems));
        }
        Ok(self.node(NodeKind::LetStmt, elems))
    }

    fn var_stmt(&mut self) -> PResult<Node> {
        let mut elems = vec![self.expect_kw("var")?];
        if self.at_kw("self") {
            elems.push(self.bump());
        } else {
            elems.push(self.expect_ident()?);
        }
        if self.eat_punct(":").is_some() {
            let t = self.parse_type()?;
            Self::push_node(&mut elems, t);
        }
        elems.push(self.expect_punct("=")?);
        let v = self.expr()?;
        Self::push_node(&mut elems, v);
        Ok(self.node(NodeKind::VarStmt, elems))
    }

    fn return_stmt(&mut self) -> PResult<Node> {
        let mut elems = vec![self.expect_kw("return")?];
        if !self.at_kind(Kind::Newline) && !self.at_eof() {
            let v = self.expr()?;
            Self::push_node(&mut elems, v);
        }
        Ok(self.node(NodeKind::ReturnStmt, elems))
    }

    /// Gán ghép `x += 1`: lexer tách `+` `=` (SPEC-lex §1 không có token `+=`) nên parser nhận
    /// cặp **dính nhau** tại vị trí hiện tại (A.2 ghi `+=`; ghi ở JOURNAL là chỗ lệch SPEC).
    ///
    /// **VD-268**: không chỉ sau `IDENT` — đích gán là **đường** (`a.b.f += 1`, `xs[0].n += 1`)
    /// theo A.2 `lvalue := IDENT postfix_op*`. `expr.rs` dừng trước cặp `+` `=` dính nhau nhờ
    /// [`Self::aug_assign_ahead`], nên `expr_stmt` nhận ra sau khi đã parse vế trái.
    pub(super) fn aug_assign_ahead(&self) -> bool {
        let op = self.tok();
        let eq = self.tok_at(1);
        // **R7 (VD-178)**: `//=` — lexer tách `//` `=`.
        matches!(op.text.as_str(), "+" | "-" | "*" | "/" | "//" | "%")
            && op.kind == Kind::Punct
            && eq.is_punct("=")
            && op.span.end == eq.span.start
    }

    fn expr_stmt(&mut self) -> PResult<Node> {
        // **VD-268**: parse **đủ** vế trái (đường `a.b[i].f`, không chỉ `IDENT`), rồi nhận
        // `=` hoặc cặp gán ghép. `expr.rs` đã dừng trước `+=`/`-=`/… dính nhau.
        let e = self.expr()?;
        if self.at_punct("=") {
            let mut elems = vec![Elem::Node(Box::new(e))];
            elems.push(self.bump());
            let v = self.expr()?;
            Self::push_node(&mut elems, v);
            return Ok(self.node(NodeKind::AssignStmt, elems));
        }
        if self.aug_assign_ahead() {
            let mut elems = vec![Elem::Node(Box::new(e))];
            elems.push(self.bump()); // `+`
            elems.push(self.bump()); // `=`
            let v = self.expr()?;
            Self::push_node(&mut elems, v);
            return Ok(self.node(NodeKind::AssignStmt, elems));
        }
        let elems = vec![Elem::Node(Box::new(e))];
        Ok(self.node(NodeKind::ExprStmt, elems))
    }

    fn break_stmt(&mut self) -> PResult<Node> {
        let elems = vec![self.bump()];
        Ok(self.node(NodeKind::BreakStmt, elems))
    }

    fn continue_stmt(&mut self) -> PResult<Node> {
        let elems = vec![self.bump()];
        Ok(self.node(NodeKind::ContinueStmt, elems))
    }

    fn for_stmt(&mut self) -> PResult<Node> {
        let mut elems = vec![self.expect_kw("for")?];
        let p = self.parse_pattern()?;
        Self::push_node(&mut elems, p);
        elems.push(self.expect_kw("in")?);
        let e = self.expr()?;
        Self::push_node(&mut elems, e);
        elems.push(self.expect_punct(":")?);
        let s = self.suite()?;
        Self::push_node(&mut elems, s);
        Ok(self.node(NodeKind::ForStmt, elems))
    }

    fn while_stmt(&mut self) -> PResult<Node> {
        let mut elems = vec![self.expect_kw("while")?];
        // **ST-226 (R18b)**: `while let PAT = EXPR:` — dùng chung `if_cond` với `if let`/`elif`.
        let e = self.if_cond()?;
        Self::push_node(&mut elems, e);
        elems.push(self.expect_punct(":")?);
        let s = self.suite()?;
        Self::push_node(&mut elems, s);
        Ok(self.node(NodeKind::WhileStmt, elems))
    }

    fn loop_stmt(&mut self) -> PResult<Node> {
        let mut elems = vec![self.expect_kw("loop")?];
        elems.push(self.expect_punct(":")?);
        let s = self.suite()?;
        Self::push_node(&mut elems, s);
        Ok(self.node(NodeKind::LoopStmt, elems))
    }

    fn scope_stmt(&mut self) -> PResult<Node> {
        let mut elems = vec![self.expect_kw("scope")?];
        if self.tok().kind == Kind::Ident && self.peek_punct(1, ":") {
            elems.push(self.bump());
        }
        elems.push(self.expect_punct(":")?);
        let s = self.suite()?;
        Self::push_node(&mut elems, s);
        Ok(self.node(NodeKind::ScopeStmt, elems))
    }

    fn if_stmt(&mut self) -> PResult<Node> {
        let mut elems = vec![self.expect_kw("if")?];
        let c = self.if_cond()?;
        Self::push_node(&mut elems, c);
        elems.push(self.expect_punct(":")?);
        let s = self.suite()?;
        Self::push_node(&mut elems, s);
        while self.at_kw("elif") {
            let mut arm = vec![self.bump()];
            let c = self.if_cond()?;
            Self::push_node(&mut arm, c);
            arm.push(self.expect_punct(":")?);
            let s = self.suite()?;
            Self::push_node(&mut arm, s);
            Self::push_node(&mut elems, self.node(NodeKind::ElifArm, arm));
        }
        if self.at_kw("else") {
            let mut arm = vec![self.bump()];
            arm.push(self.expect_punct(":")?);
            let s = self.suite()?;
            Self::push_node(&mut arm, s);
            Self::push_node(&mut elems, self.node(NodeKind::ElseArm, arm));
        }
        Ok(self.node(NodeKind::IfStmt, elems))
    }

    fn match_stmt(&mut self) -> PResult<Node> {
        let mut elems = vec![self.expect_kw("match")?];
        let e = self.expr()?;
        Self::push_node(&mut elems, e);
        elems.push(self.expect_punct(":")?);
        let body = self.match_body()?;
        Self::push_node(&mut elems, body);
        Ok(self.node(NodeKind::MatchStmt, elems))
    }

    /// `NEWLINE INDENT match_arm+ DEDENT`; arm := pattern ['if' expr] ':' suite.
    ///
    /// **VD-266**: trong ngoặc lexer không sinh NEWLINE/INDENT/DEDENT, nên dùng
    /// [`Parser::match_body_trong_ngoac`] — nhánh `match` biểu thức lồng trong đối số/ngoặc.
    pub(super) fn match_body(&mut self) -> PResult<Node> {
        if self.delim > 0 {
            return self.match_body_trong_ngoac();
        }
        let mut elems = vec![self.expect_newline()?, self.expect_indent()?];
        while !self.at_kind(Kind::Dedent) && !self.at_eof() {
            let mut arm = Vec::new();
            let p = self.parse_pattern()?;
            Self::push_node(&mut arm, p);
            if self.at_kw("if") {
                arm.push(self.bump());
                let g = self.expr()?;
                Self::push_node(&mut arm, g);
            }
            arm.push(self.expect_punct(":")?);
            let s = self.suite()?;
            Self::push_node(&mut arm, s);
            Self::push_node(&mut elems, self.node(NodeKind::MatchArm, arm));
            self.require_line_end(&mut elems)?;
        }
        if let Some(d) = self.take_dedent() {
            elems.push(d);
        }
        Ok(self.node(NodeKind::Suite, elems))
    }

    /// **VD-266**: thân `match` khi đang trong ngoặc (`delim > 0`) — không có token xuống dòng,
    /// nên ranh giới nhánh lấy từ **dòng vật lý** + **cột** của nhánh đầu:
    ///
    /// - nhánh kế tiếp phải bắt đầu một dòng mới **cùng cột** với nhánh đầu;
    /// - cột nhỏ hơn ⇒ nhánh của `match` cha; dấu đóng `)`/`]`/`}`/`,` ⇒ hết `match`.
    ///
    /// Thân mỗi nhánh vẫn là **một** `simple_stmt` (một biểu thức) như dạng một-dòng ở cấp câu
    /// lệnh; nhiều câu lệnh trong một nhánh cần cấp câu lệnh (có NEWLINE/INDENT).
    fn match_body_trong_ngoac(&mut self) -> PResult<Node> {
        let mut elems = Vec::new();
        let mut cot = None;
        while !self.at_eof() && !self.at_delim_end() {
            let t = self.tok();
            match cot {
                None => cot = Some(t.col),
                Some(c) => {
                    if !self.at_new_line() || t.col != c {
                        break;
                    }
                }
            }
            let mut arm = Vec::new();
            let p = self.parse_pattern()?;
            Self::push_node(&mut arm, p);
            if self.at_kw("if") {
                arm.push(self.bump());
                let g = self.expr()?;
                Self::push_node(&mut arm, g);
            }
            arm.push(self.expect_punct(":")?);
            let s = self.suite()?;
            Self::push_node(&mut arm, s);
            Self::push_node(&mut elems, self.node(NodeKind::MatchArm, arm));
        }
        if elems.is_empty() {
            return Err(self.err_here("`match` phải có ít nhất một nhánh"));
        }
        Ok(self.node(NodeKind::Suite, elems))
    }
}
