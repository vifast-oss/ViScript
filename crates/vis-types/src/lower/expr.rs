// SPDX-License-Identifier: MIT OR Apache-2.0
//! Hạ **biểu thức** CST → [`HExpr`] (ST-11) — phần đuôi của [`crate::lower`].
//!
//! Không kiểm kiểu: chỉ *điền* kiểu cho literal (VD-43), dựng mặt nạ bản ghi ẩn danh cho record
//! literal và nhận ra enum literal khi tên biến thể đã khai trong tệp. Mọi thứ khác giữ
//! `sym = None` / `Type::Error` và **không** sinh chẩn đoán (VD-44 #1).

use vis_syntax::{lex, parse, Cst, Elem, Kind, LexOutput, Mode, Node, NodeKind, Span};

use super::{bin_op, lit_kind, un_op, Lowerer};
use crate::hir::{BinOp, HBody, HCond, HExpr, HParam, InterpPart, Lit, LitKind};
use crate::stdlib as vis_types_stdlib;
use crate::ty::{EnumId, FieldDef, RecDef, RecId, Type};
use vis_diag::{NAME_CODES, TYPE_CODES};

mod chuoi;
mod goi;

impl<'a> Lowerer<'a> {
    /// Hạ một biểu thức; không bao giờ panic — node lạ ⇒ [`HExpr::Error`].
    ///
    /// **VD-263**: `expect` (do [`Lowerer::lower_expr_as`] đặt) chỉ tiêu thụ ở **node hiện tại**
    /// — lấy ra ngay để các lời gọi đệ quy không thừa hưởng kiểu mong đợi của cha.
    pub(crate) fn lower_expr(&mut self, node: &Node) -> HExpr {
        let expect = self.expect.take();
        self.lower_expr_voi(node, expect)
    }

    /// **VD-263** — hạ biểu thức với kiểu **mong đợi** (annot `let`/`var`, `Some(…)`, biến thể
    /// enum). Chỉ constructor/record literal dùng; dạng khác hạ y như không có gợi ý.
    pub(crate) fn lower_expr_as(&mut self, node: &Node, expect: Option<Type>) -> HExpr {
        let cu = std::mem::replace(&mut self.expect, expect);
        let e = self.lower_expr(node);
        self.expect = cu;
        e
    }

    fn lower_expr_voi(&mut self, node: &Node, expect: Option<Type>) -> HExpr {
        let span = node.span;
        match node.kind {
            NodeKind::Literal => self.lower_literal(node),
            NodeKind::Ident => {
                let name = self.text_of(span).to_string();
                self.lower_var_use(&name, span)
            }
            NodeKind::BinOp => {
                let mut lhs: Option<HExpr> = None;
                let mut rhs: Option<HExpr> = None;
                let mut op_text = String::new();
                for e in &node.elems {
                    match e {
                        Elem::Node(n) => {
                            if lhs.is_none() {
                                lhs = Some(self.lower_expr(n));
                            } else {
                                rhs = Some(self.lower_expr(n));
                            }
                        }
                        Elem::Token(i) => {
                            if let Some(t) = self.tok(*i) {
                                if !op_text.is_empty() {
                                    op_text.clear();
                                }
                                op_text.push_str(&t.text);
                            }
                        }
                    }
                }
                let op = bin_op(&op_text).unwrap_or(BinOp::Add);
                HExpr::Bin {
                    op,
                    lhs: Box::new(lhs.unwrap_or(HExpr::Error { span })),
                    rhs: Box::new(rhs.unwrap_or(HExpr::Error { span })),
                    span,
                }
            }
            NodeKind::UnOp => {
                let op_text = node
                    .tokens(&self.cst.tokens)
                    .next()
                    .map(|t| t.text.clone())
                    .unwrap_or_default();
                let op = un_op(&op_text).unwrap_or(crate::hir::UnOp::Neg);
                let expr = node
                    .children()
                    .next()
                    .map(|c| self.lower_expr(c))
                    .unwrap_or(HExpr::Error { span });
                // **VD-266**: gộp `-` quanh literal `int` ở **biên** 2^63. Lexer cho qua độ lớn
                // `i64::MAX + 1` (để `-9223372036854775808` thành `i64::MIN`), nên nếu không gộp
                // thì literal dương 2^63 sẽ bị checker từ chối oan. Chỉ chạm đúng biên: literal
                // thường vẫn giữ `HExpr::Un` như trước.
                if op == crate::hir::UnOp::Neg {
                    if let HExpr::Lit { lit, .. } = &expr {
                        if lit.kind == LitKind::Int {
                            if let Some(text) = vis_syntax::lit::fold_neg_min_int(&lit.text) {
                                return HExpr::Lit {
                                    lit: Lit {
                                        kind: LitKind::Int,
                                        text,
                                    },
                                    ty: Type::Int,
                                    span,
                                };
                            }
                        }
                    }
                }
                HExpr::Un {
                    op,
                    expr: Box::new(expr),
                    span,
                }
            }
            // `(expr)` — HIR không có nút `paren` (xem quy ước sexp ở `hir.rs`).
            NodeKind::Paren => node
                .children()
                .next()
                .map(|c| self.lower_expr(c))
                .unwrap_or(HExpr::Error { span }),
            NodeKind::Tuple => {
                let items: Vec<HExpr> = node.children().map(|c| self.lower_expr(c)).collect();
                let ty = if items.is_empty() {
                    Type::Unit
                } else {
                    Type::TypeRef {
                        name: "tuple".into(),
                        args: items.iter().map(|i| self.type_of(i)).collect(),
                    }
                };
                HExpr::Tuple { items, ty, span }
            }
            NodeKind::ListLit => {
                let items: Vec<HExpr> = node.children().map(|c| self.lower_expr(c)).collect();
                let elem = first_known(items.iter().map(|i| self.type_of(i)));
                HExpr::List { items, elem, span }
            }
            NodeKind::SetLit => {
                let items: Vec<HExpr> = node.children().map(|c| self.lower_expr(c)).collect();
                let elem = first_known(items.iter().map(|i| self.type_of(i)));
                HExpr::Set { items, elem, span }
            }
            NodeKind::MapLit => {
                let mut entries = Vec::new();
                // **VD-215 (2026-11-18)**: `{[k]: …}` (khoá là danh sách) ⇒ **cảnh báo** `VS0208`;
                // `{([k]): …}` (lồng trong ngoặc) là cách tắt cảnh báo. Khoá tính toán là `(expr)`.
                for e in node.children().filter(|n| n.kind == NodeKind::MapEntry) {
                    let mut it = e.children();
                    let knode = it.next();
                    if let Some(kn) = &knode {
                        if kn.kind == NodeKind::ListLit {
                            self.diag(
                                &vis_diag::SYNTAX_CODES[7],
                                "`{[k]: …}` khoá là danh sách — dùng `{(k): v}`; viết `{([k]): …}` nếu muốn giữ danh sách làm khoá",
                                kn.span,
                            );
                        }
                    }
                    let k = knode
                        .map(|n| self.lower_expr(n))
                        .unwrap_or(HExpr::Error { span: e.span });
                    let v = it
                        .next()
                        .map(|n| self.lower_expr(n))
                        .unwrap_or(HExpr::Error { span: e.span });
                    entries.push((k, v));
                }
                let key = first_known(entries.iter().map(|(k, _)| self.type_of(k)));
                let value = first_known(entries.iter().map(|(_, v)| self.type_of(v)));
                HExpr::Map {
                    entries,
                    key,
                    value,
                    span,
                }
            }
            NodeKind::RecordLit => {
                // **VD-265**: gom tên/span trước (chưa hạ giá trị) để biết **instantiation** từ
                // `expect`, rồi mới hạ từng trường với kiểu mong đợi của nó — constructor lồng
                // trong literal ẩn danh nhận đúng lớp `T?`.
                let mut tho: Vec<(String, Option<&Node>, Span)> = Vec::new();
                for f in node.children().filter(|n| n.kind == NodeKind::RecordField) {
                    let name = self.decl_name(f).unwrap_or_default();
                    tho.push((name, f.children().next(), f.span));
                }
                let inst = match expect.as_ref().map(bo_opt) {
                    Some(Type::Record(exp))
                        if self.tbl.rec_inst_of(*exp).is_some()
                            && tho.iter().all(|(n, _, _)| {
                                self.tbl
                                    .rec(*exp)
                                    .is_some_and(|d| d.fields.iter().any(|f| &f.name == n))
                            }) =>
                    {
                        Some(*exp)
                    }
                    _ => None,
                };
                let mut fields: Vec<(String, HExpr)> = Vec::new();
                let mut defs: Vec<FieldDef> = Vec::new();
                let mut seen = std::collections::HashSet::with_capacity(8);
                for (name, value_node, fspan) in tho {
                    let want = inst
                        .and_then(|id| self.tbl.rec(id))
                        .and_then(|d| d.fields.iter().find(|f| f.name == name))
                        .map(|f| f.ty.clone());
                    let value = match value_node {
                        Some(v) => self.lower_expr_as(v, want),
                        // `{ ten }` — viết tắt `ten: ten`.
                        None => {
                            let n2 = name.clone();
                            self.lower_var_use(&n2, fspan)
                        }
                    };
                    if !seen.insert(name.clone()) {
                        self.diag(
                            &vis_diag::TYPE_CODES[4],
                            format!("trường `{name}` xuất hiện nhiều lần trên bản ghi ẩn danh"),
                            fspan,
                        );
                    }
                    defs.push(FieldDef {
                        name: name.clone(),
                        ty: self.type_of(&value),
                        span: fspan,
                        has_default: false,
                    });
                    fields.push((name, value));
                }
                // **VD-263**: annot generic (`let a: Hop[int?] = {gia_tri: 5}`) ⇒ literal mang
                // **id instantiation** để biên dựng nâng trường `T?` theo kiểu đã thay.
                let ty = match inst {
                    Some(id) => Type::Record(id),
                    None => {
                        let id = RecId(self.tbl.recs().len() as u32);
                        self.tbl.push_rec(RecDef {
                            name: None,
                            fields: defs,
                            span,
                            anonymous: true,
                        });
                        Type::Record(id)
                    }
                };
                HExpr::Record { fields, ty, span }
            }
            NodeKind::Field | NodeKind::OptField => {
                let base = match node.children().next() {
                    // **VD-258**: `Json.Arr(…)` / `SqlValue.Integer(…)` dưới `Strict` — tên kiểu
                    // dựng sẵn không có ký hiệu giá trị; đừng báo `VS0301` trước `maybe_enum_lit`.
                    Some(c) if c.kind == NodeKind::Ident => {
                        let ten = self.text_of(c.span).to_string();
                        if self.resolve(&ten).is_none() && self.la_ten_kieu_dung_san(&ten) {
                            HExpr::Var {
                                name: ten,
                                sym: None,
                                span: c.span,
                            }
                        } else {
                            self.lower_expr(c)
                        }
                    }
                    Some(c) => self.lower_expr(c),
                    None => HExpr::Error { span },
                };
                let name = node
                    .elems
                    .iter()
                    .rev()
                    .find_map(|e| match e {
                        Elem::Token(i) => self.tok(*i).map(|t| t.text.clone()),
                        Elem::Node(_) => None,
                    })
                    .unwrap_or_default();
                HExpr::Field {
                    base: Box::new(base),
                    name,
                    optional: node.kind == NodeKind::OptField,
                    span,
                }
            }
            NodeKind::Call | NodeKind::GenericCall => {
                // **ST-61 (VD-96)**: `chan` là **soft keyword** — trong `GenericCall` với callee
                // `Var{name:"chan", sym:None}` + **đúng 1** đối số kiểu ⇒ `HExpr::ChanNew`. Người dùng
                // khai `fn chan` (sym `Some`) ⇒ gọi hàm bình thường.
                if node.kind == NodeKind::GenericCall {
                    let callee = node
                        .children()
                        .next()
                        .map(|c| self.lower_expr(c))
                        .unwrap_or(HExpr::Error { span });
                    let la_chan = matches!(
                        &callee,
                        HExpr::Var { name, sym: None, .. } if name == "chan"
                    );
                    if la_chan {
                        let ty = node
                            .child(NodeKind::Generics)
                            .and_then(|g| g.children().next())
                            .map(|t| self.resolve_type(t))
                            .unwrap_or(Type::Error);
                        let ds_node = node.child(NodeKind::Args);
                        let mut ds = ds_node.map(|a| a.children()).into_iter().flatten();
                        let cap = ds
                            .next()
                            .map(|c| self.lower_expr(c))
                            .unwrap_or(HExpr::Error { span });
                        if ds.next().is_some() {
                            self.diag(
                                &vis_diag::TYPE_CODES[0],
                                "`chan[T](n)` cần **đúng một** đối số (dung lượng)",
                                span,
                            );
                        }
                        return HExpr::ChanNew {
                            elem: ty,
                            cap: Box::new(cap),
                            span,
                        };
                    }
                }
                let callee = node
                    .children()
                    .next()
                    .map(|c| self.lower_callee(c))
                    .unwrap_or(HExpr::Error { span });
                let (args, named) = node
                    .child(NodeKind::Args)
                    .map(|a| self.lower_args_expect(a, &callee, expect.as_ref()))
                    .unwrap_or_default();
                // ST-26: `str.split(…)`, `list.sort(…)` — **tên đầy đủ** `std.<module>.<fn>` khi tệp đã
                // `use std.<module>`; gọi vào tên đã biết nhưng hoãn ⇒ `VS0301` "chưa hỗ trợ (VS1b)".
                // ST-29: **dispatch tĩnh** — `x.m(…)` với `x` kiểu tĩnh record/enum và `m` không
                // phải phương thức nội tại ⇒ hạ thẳng `Call(impl_fn)` (chèn `self/dt` đầu).
                if let Some(e) = self.dispatch_tinh(&callee, &args, span) {
                    return e;
                }
                // ST-26: chuyển `str.split(…)` sang tên đầy đủ `std.str.split` (nếu tệp đã `use`).
                if let HExpr::Field {
                    base,
                    name: f,
                    optional: false,
                    ..
                } = &callee
                {
                    if let HExpr::Var { name: m, .. } = &**base {
                        // **VD-223 (điểm 2, amendment VD-224)**: gọi `str.join(…)` luôn rơi về
                        // module std khi tài liệu đã `use std.str` — vị trí module là ý định;
                        // user bind chỉ ảnh hưởng tới **dùng trần** tên (`str(…)`) — gọi trùng
                        // `str(…)` sẽ là `VS0412` nếu `str` là biến (đã có thông điệp riêng).
                        // Tên bị che ở vị trí module được báo **lúc khai báo** bằng `VS0211`.
                        if self.opts.std_mods.iter().any(|x| x == m) {
                            return self.std_call_named(m.clone(), f.clone(), args, named, span);
                        }
                    }
                }
                // **R12a (VD-178)**: `Email("a@b")` — newtype **trong suốt** ⇒ hạ thẳng đối số.
                if let HExpr::Var { name, .. } = &callee {
                    if self.newtypes.iter().any(|n| n == name)
                        && args.len() == 1
                        && named.is_empty()
                    {
                        return args.into_iter().next().unwrap();
                    }
                }
                self.maybe_enum_lit(callee, args, named, expect.as_ref(), span)
            }
            NodeKind::Index => {
                let mut it = node.children();
                let base = it
                    .next()
                    .map(|c| self.lower_expr(c))
                    .unwrap_or(HExpr::Error { span });
                let index = it
                    .next()
                    .map(|c| self.lower_expr(c))
                    .unwrap_or(HExpr::Error { span });
                HExpr::Index {
                    base: Box::new(base),
                    index: Box::new(index),
                    span,
                }
            }
            NodeKind::TryOp => {
                let inner = node
                    .children()
                    .next()
                    .map(|c| self.lower_expr(c))
                    .unwrap_or(HExpr::Error { span });
                HExpr::Try {
                    expr: Box::new(inner),
                    span,
                }
            }
            NodeKind::WithOp => {
                let mut it = node.children();
                let base = it
                    .next()
                    .map(|c| self.lower_expr(c))
                    .unwrap_or(HExpr::Error { span });
                // **VD-265**: kiểu trường của **bản nhận** là kiểu mong đợi của giá trị `with`.
                let base_ty = self.static_ty(&base);
                let mut fields = Vec::new();
                for f in node.children().filter(|n| n.kind == NodeKind::WithField) {
                    let name = self.decl_name(f).unwrap_or_default();
                    let want = match &base_ty {
                        Some(Type::Record(id)) => {
                            self.tbl.field(*id, &name).map(|fd| fd.ty.clone())
                        }
                        _ => None,
                    };
                    let value = f
                        .children()
                        .next()
                        .map(|c| self.lower_expr_as(c, want))
                        .unwrap_or(HExpr::Error { span: f.span });
                    fields.push((name, value));
                }
                HExpr::With {
                    base: Box::new(base),
                    fields,
                    span,
                }
            }
            NodeKind::IfExpr => self.lower_if_expr(node, expect),
            NodeKind::MatchExpr => {
                let subject = node
                    .children()
                    .find(|c| c.kind != NodeKind::Suite)
                    .map(|c| self.lower_expr(c))
                    .unwrap_or(HExpr::Error { span });
                // **VD-265**: nhánh cuối của `match` ở vị trí giá trị mang kiểu mong đợi.
                let arms = node
                    .child(NodeKind::Suite)
                    .map(|s| self.lower_arms_ret(s, expect))
                    .unwrap_or_default();
                HExpr::Match {
                    subject: Box::new(subject),
                    arms,
                    ty: Type::Error,
                    span,
                }
            }
            NodeKind::Lambda => self.lower_lambda(node),
            NodeKind::RefExpr => {
                let inner = node
                    .children()
                    .next()
                    .map(|c| self.lower_expr(c))
                    .unwrap_or(HExpr::Error { span });
                HExpr::Ref {
                    expr: Box::new(inner),
                    span,
                }
            }
            NodeKind::AwaitExpr => {
                let inner = node
                    .children()
                    .next()
                    .map(|c| self.lower_expr(c))
                    .unwrap_or(HExpr::Error { span });
                HExpr::Await {
                    expr: Box::new(inner),
                    span,
                }
            }
            // **ST-60**: `spawn f(args)` — callee là **tên hàm** (tyck đòi `async` cấp tệp).
            NodeKind::SpawnExpr => {
                let callee = node
                    .children()
                    .next()
                    .map(|c| self.lower_expr(c))
                    .unwrap_or(HExpr::Error { span });
                let (args, named) = node
                    .child(NodeKind::Args)
                    .map(|a| self.lower_args(a))
                    .unwrap_or_default();
                HExpr::Spawn {
                    callee: Box::new(callee),
                    args,
                    named,
                    span,
                }
            }
            NodeKind::CmdBlock => HExpr::CmdBlock {
                raw: self.text_of(span).to_string(),
                span,
            },
            _ => HExpr::Error { span },
        }
    }

    /// **VD-258** — hạ **callee** của lời gọi: tên biến thể enum dựng sẵn (`Arr(…)`, `Integer(…)`)
    /// không có ký hiệu giá trị, nên đừng báo `VS0301` trước `maybe_enum_lit`. Chỉ áp cho tên
    /// **chưa phân giải** và **có** trong `variant_index` của lượt hạ.
    fn lower_callee(&mut self, node: &Node) -> HExpr {
        if node.kind == NodeKind::Ident {
            let ten = self.text_of(node.span).to_string();
            if self.resolve(&ten).is_none() && self.variant_index.iter().any(|(n, _, _)| n == &ten)
            {
                let cu = self.suppress_name_diag;
                self.suppress_name_diag = true;
                let e = self.lower_expr(node);
                self.suppress_name_diag = cu;
                return e;
            }
        }
        self.lower_expr(node)
    }

    fn lower_if_expr(&mut self, node: &Node, expect: Option<Type>) -> HExpr {
        let span = node.span;
        // **VD-265**: nhánh ở **vị trí giá trị** — biểu thức cuối mang kiểu mong đợi (như
        // [`Lowerer::lower_suite_ret`]) để constructor generic lồng trong `if` giữ instantiation.
        let then_ret = expect.clone();
        let cond_node = node.children().find(|n| {
            !matches!(
                n.kind,
                NodeKind::ElifArm | NodeKind::ElseArm | NodeKind::Suite
            )
        });
        let (cond, then) = match cond_node {
            Some(c) => {
                let is_let = self.is_let_cond(c);
                if is_let {
                    self.push_frame(super::FrameKind::Block, c.span);
                }
                let cond = self.lower_cond(c);
                let then = node
                    .child(NodeKind::Suite)
                    .map(|s| self.lower_suite_ret(s, then_ret))
                    .unwrap_or_default();
                if is_let {
                    self.pop_frame();
                }
                (cond, then)
            }
            None => (
                HCond::Expr(HExpr::Error { span }),
                node.child(NodeKind::Suite)
                    .map(|s| self.lower_suite_ret(s, expect.clone()))
                    .unwrap_or_default(),
            ),
        };
        let mut elifs = Vec::new();
        for arm in node.children().filter(|n| n.kind == NodeKind::ElifArm) {
            let body = arm
                .child(NodeKind::Suite)
                .map(|s| self.lower_suite_ret(s, expect.clone()))
                .unwrap_or_default();
            let c = arm
                .children()
                .find(|n| n.kind != NodeKind::Suite)
                .map(|n| self.lower_cond(n))
                .unwrap_or(HCond::Expr(HExpr::Error { span: arm.span }));
            elifs.push((c, body));
        }
        let else_ = node
            .children()
            .find(|n| n.kind == NodeKind::ElseArm)
            .and_then(|a| a.child(NodeKind::Suite))
            .map(|s| self.lower_suite_ret(s, expect))
            .unwrap_or_default();
        HExpr::If {
            cond: Box::new(cond),
            then,
            elifs,
            else_,
            ty: Type::Error,
            span,
        }
    }

    fn lower_lambda(&mut self, node: &Node) -> HExpr {
        let span = node.span;
        self.push_frame(super::FrameKind::Fn, span);
        let mut params: Vec<HParam> = Vec::new();
        if let Some(lp) = node.child(NodeKind::LambdaParams) {
            for p in lp.children().filter(|n| n.kind == NodeKind::Param) {
                let Some((name, psp)) = self.first_ident(p) else {
                    continue;
                };
                let ty = match p.child(NodeKind::Type) {
                    Some(t) => self.resolve_type(t),
                    None => Type::Error,
                };
                let sym = self.declare(&name, super::SymKind::Param, psp, Some(ty.clone()));
                params.push(HParam {
                    name,
                    span: psp,
                    sym,
                    ty,
                    named: false,
                    is_self: false,
                    is_var_self: false,
                    default: None,
                    variadic: false,
                });
            }
        }
        let ret = node
            .child(NodeKind::Type)
            .map(|t| self.resolve_type(t))
            .unwrap_or(Type::Error);
        // **VD-263**: `return`/biểu thức cuối của **lambda** dùng kiểu trả về của chính nó.
        let luu_ret = self.cur_ret.replace(ret.clone());
        let body = match node.child(NodeKind::Suite) {
            Some(s) => HBody::Stmts(self.lower_suite_ret(s, Some(ret.clone()))),
            None => {
                let e = node
                    .children()
                    .filter(|n| n.kind != NodeKind::LambdaParams && n.kind != NodeKind::Type)
                    .last()
                    .map(|c| self.lower_expr_as(c, Some(ret.clone())))
                    .unwrap_or(HExpr::Error { span });
                HBody::Expr(Box::new(e))
            }
        };
        self.cur_ret = luu_ret;
        self.pop_frame();
        HExpr::Lambda {
            params,
            ret,
            body,
            span,
        }
    }
}

/// Kiểu "đã biết" đầu tiên trong danh sách; `Type::Error` nếu không có.
fn first_known(it: impl Iterator<Item = Type>) -> Type {
    for t in it {
        if !t.is_error() {
            return t;
        }
    }
    Type::Error
}

/// Giải một escape trong chuỗi (`\n`, `\t`, `\"`, `\\`, `\{`, …) — trả (ký tự, số byte đã dùng
/// sau dấu `\`).
/// **VD-241 §3/T4 (A0, 2026-11-19)** — giải mã escape sa `\`; hợp lệ thì trả `(ký tự, số byte)`,
/// sai hoặc không-known-escape thì `Err(VS0102)`. Hỗ trợ: `\n \t \r \0 \\ \" \' \{ \} \u{…}`.
fn giai_escape(sau: &str) -> Result<(char, usize), u16> {
    let mut it = sau.chars();
    match it.next() {
        Some('n') => Ok(('\n', 1)),
        Some('t') => Ok(('\t', 1)),
        Some('r') => Ok(('\r', 1)),
        Some('0') => Ok(('\0', 1)),
        Some('\\') => Ok(('\\', 1)),
        Some('"') => Ok(('"', 1)),
        Some('\'') => Ok(('\'', 1)),
        Some('{') => Ok(('{', 1)),
        Some('}') => Ok(('}', 1)),
        Some('u') => {
            // `\u{XXXXXX}` — 1..=6 chữ hex; giá trị phải là Unicode scalar hợp lệ.
            let rest = &sau[1..];
            if !rest.starts_with('{') {
                return Err(102);
            }
            let Some(close) = rest.find('}') else {
                return Err(102);
            };
            let hex = &rest[1..close];
            if hex.is_empty() || hex.len() > 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(102);
            }
            let v = u32::from_str_radix(hex, 16).map_err(|_| 102u16)?;
            let ch = char::from_u32(v).ok_or(102u16)?;
            Ok((ch, 1 + 1 + hex.len() + 1)) // 'u' + '{' + hex + '}'
        }
        Some(_) => Err(102), // escape không hợp lệ — trùng luật ở lexer (VS0102).
        None => Err(102),
    }
}

/// **R6c (VD-178)**: tách `expr` và `:spec` ở **độ sâu ngoặc 0** của một chỗ nội suy.
fn tach_spec(inner: &str) -> (&str, Option<String>) {
    let mut depth = 0usize;
    for (idx, ch) in inner.char_indices() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ':' if depth == 0 => {
                return (&inner[..idx], Some(inner[idx + 1..].to_string()));
            }
            _ => {}
        }
    }
    (inner, None)
}

/// **VD-263** — bỏ mọi lớp `Option` ngoài cùng của **kiểu mong đợi**: constructor dựng kiểu lõi
/// (`G[int?]`) rồi biên annot mới nâng cả giá trị lên `G[int?]?`.
fn bo_opt(t: &Type) -> &Type {
    let mut cur = t;
    while let Type::Opt(inner) = cur {
        cur = inner;
    }
    cur
}
