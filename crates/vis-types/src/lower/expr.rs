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

    fn lower_literal(&mut self, node: &Node) -> HExpr {
        let span = node.span;
        let Some(t) = node.tokens(&self.cst.tokens).next() else {
            return HExpr::Error { span };
        };
        if t.kind == Kind::Kw {
            return match t.text.as_str() {
                // **R2 (VD-178)**: từ khoá `none` đã **bỏ** — dùng `None` (`VS0203`).
                // Hạ vẫn tạo Opt(None) để biên dịch tiếp (chẩn đoán là cổng chặn).
                "none" => {
                    self.diag(
                        &vis_diag::SYNTAX_CODES[2],
                        "`none` đã bỏ — dùng `None` (từ khoá đã bỏ, dùng biến thể `None` của `Option`)",
                        span,
                    );
                    HExpr::Opt {
                        value: None,
                        ty: Type::Opt(Box::new(Type::Error)),
                        span,
                    }
                }
                "true" | "false" => HExpr::Lit {
                    lit: Lit {
                        kind: LitKind::Bool,
                        text: t.text.clone(),
                    },
                    ty: Type::Bool,
                    span,
                },
                _ => HExpr::Error { span },
            };
        }
        let Some(kind) = lit_kind(t.kind) else {
            return HExpr::Error { span };
        };
        // ST-14 (VD-47a): chuỗi có `{expr}` ⇒ hạ **từng mảnh** thành `HExpr::Interp`.
        // **R6a (VD-178)**: `p"…"` cũng **có** nội suy (nhưng **không** escape).
        if (kind == LitKind::Str || kind == LitKind::Path)
            && t.kind != Kind::RawStr
            && t.kind != Kind::RawMulti
            && t.text.contains('{')
        {
            let (dau, cuoi) = match t.kind {
                Kind::MultiStr => (3, 3),
                Kind::Path => (2, 1),
                _ => (1, 1),
            };
            if t.text.len() >= dau + cuoi {
                let trong = &t.text[dau..t.text.len() - cuoi];
                return self.chuoi_noi_suy(trong, dau, t.kind == Kind::Path, span);
            }
        }
        let lit = Lit {
            kind,
            text: t.text.clone(),
        };
        let ty = lit.ty();
        HExpr::Lit { lit, ty, span }
    }

    /// Hạ chuỗi **có nội suy** (ST-14): tách mảnh chữ + `{expr}`, hạ từng `{expr}` **bằng chính
    /// `Lowerer` này** (chia sẻ `symbols`/`scope` ⇒ tên trong nội suy giải đúng ngữ cảnh), với CST
    /// của mảnh được **đệm khoảng trắng** tới đúng **offset tuyệt đối** trong tệp để `span` (và do đó
    /// `dòng:cột`) trỏ **đúng chỗ**.
    fn chuoi_noi_suy(&mut self, trong: &str, ngoac: usize, no_escape: bool, span: Span) -> HExpr {
        let mut parts: Vec<InterpPart> = Vec::new();
        let mut chu = String::new();
        let mut i = 0usize;
        let mut hong = false;
        while i < trong.len() {
            let c = trong[i..].chars().next().unwrap();
            let n = c.len_utf8();
            let sau = &trong[i + n..];
            match c {
                '{' if sau.starts_with('{') => {
                    chu.push('{');
                    i += n + 1;
                }
                '}' if sau.starts_with('}') => {
                    chu.push('}');
                    i += n + 1;
                }
                '{' => {
                    // quét tới `}` cân bằng (cho phép `{ f({a: 1}) }` ở mức ngoặc)
                    let start = i + n;
                    let mut depth = 1usize;
                    let mut j = start;
                    let mut end = None;
                    while j < trong.len() {
                        let d = trong[j..].chars().next().unwrap();
                        if d == '{' {
                            depth += 1;
                        } else if d == '}' {
                            depth -= 1;
                            if depth == 0 {
                                end = Some(j);
                                break;
                            }
                        }
                        j += d.len_utf8();
                    }
                    let Some(end) = end else {
                        self.diag(
                            &vis_diag::SYNTAX_CODES[0],
                            "nội suy `{` không có `}` đóng (§A.6)",
                            Span::new_file(
                                self.file,
                                span.start + ngoac + i,
                                span.start + ngoac + trong.len(),
                            ),
                        );
                        hong = true;
                        break;
                    };
                    if !chu.is_empty() {
                        parts.push(InterpPart::Str(std::mem::take(&mut chu)));
                    }
                    let off = span.start + ngoac + start;
                    // **R6c (VD-178)**: tách `:spec` ở độ sâu ngoặc 0 (`.N` = N chữ số thập phân).
                    let manh = &trong[start..end];
                    let (src, spec) = tach_spec(manh);
                    match self.ha_manh(src, off) {
                        Some(e) => match spec {
                            Some(sp) => parts.push(InterpPart::ExprSpec {
                                expr: Box::new(e),
                                spec: sp,
                            }),
                            None => parts.push(InterpPart::Expr(Box::new(e))),
                        },
                        None => {
                            hong = true;
                            break;
                        }
                    }
                    i = end + 1;
                }
                '\\' if !no_escape => match giai_escape(sau) {
                    Ok((c2, dung)) => {
                        chu.push(c2);
                        i += n + dung;
                    }
                    Err(_) => {
                        let file = self.file;
                        self.diag(
                                &vis_diag::CODES[1],
                                "escape chuỗi không hợp lệ (`\\u{XXXXXX}`?) (§3) — `vis explain VS0102`".to_string(),
                                Span::new_file(
                                    file,
                                    span.start + ngoac + i,
                                    span.start + ngoac + (i + n),
                                ),
                            );
                        hong = true;
                        i += n + 1;
                    }
                },
                _ => {
                    chu.push(c);
                    i += n;
                }
            }
        }
        if !chu.is_empty() {
            parts.push(InterpPart::Str(chu));
        }
        if hong && parts.is_empty() {
            return HExpr::Error { span };
        }
        HExpr::Interp { parts, span }
    }

    /// Hạ **một** mảnh `{…}` tại offset tuyệt đối `off` — đệm khoảng trắng để `span` tuyệt đối.
    fn ha_manh(&mut self, manh: &str, off: usize) -> Option<HExpr> {
        // Đệm **đúng `off` byte** nhưng chèn `\n` ở byte cuối: giữ offset tuyệt đối mà **không**
        // sinh `INDENT` giả ở đầu tệp (lexer coi dòng 1 là dòng trắng, mảnh bắt đầu ở cột 1).
        let dem = " ".repeat(off.saturating_sub(1));
        let src = if off == 0 {
            manh.to_string()
        } else {
            format!("{dem}\n{manh}")
        };
        let parsed = parse(&src, Mode::Script);
        if !parsed.diagnostics.is_empty() {
            for d in &parsed.diagnostics {
                self.diag(
                    d.code,
                    d.message.clone(),
                    Span::new_file(self.file, off, off + manh.len()),
                );
            }
            return None;
        }
        let cst: &'static Cst = Box::leak(Box::new(parsed.cst));
        let text: &'static str = Box::leak(src.into_boxed_str());
        let _lex: &'static LexOutput = Box::leak(Box::new(lex(text, Mode::Script)));
        let cu_cst = self.cst;
        let cu_text = self.text;
        self.cst = cst;
        self.text = text;
        let mut ket_qua = None;
        let con: Vec<Node> = self.cst.root.children().cloned().collect();
        // Mảnh phải là **đúng một** câu lệnh biểu thức.
        match con.as_slice() {
            [child] => match self.lower_stmt(child) {
                Some(crate::hir::HStmt::Expr(expr)) => ket_qua = Some(expr),
                _ => self.diag(
                    &vis_diag::SYNTAX_CODES[0],
                    "nội suy `{…}` chỉ nhận **một biểu thức**",
                    Span::new_file(self.file, off, off + manh.len()),
                ),
            },
            _ => self.diag(
                &vis_diag::SYNTAX_CODES[0],
                "nội suy `{…}` chỉ nhận **một biểu thức**",
                Span::new_file(self.file, off, off + manh.len()),
            ),
        }
        self.cst = cu_cst;
        self.text = cu_text;
        ket_qua
    }

    /// `nilai_kiểu` cho các vị trí cần suy ra kiểu phần tử (list/map/tuple).
    fn type_of(&self, e: &HExpr) -> Type {
        match e {
            HExpr::Lit { ty, .. }
            | HExpr::Opt { ty, .. }
            | HExpr::Record { ty, .. }
            | HExpr::Tuple { ty, .. }
            | HExpr::If { ty, .. }
            | HExpr::Match { ty, .. }
            | HExpr::EnumLit { ty, .. } => ty.clone(),
            HExpr::List { elem, .. } => Type::List(Box::new(elem.clone())),
            HExpr::Set { elem, .. } => Type::List(Box::new(elem.clone())),
            HExpr::Map { key, value, .. } => {
                Type::Map(Box::new(key.clone()), Box::new(value.clone()))
            }
            HExpr::Lambda { params, ret, .. } => Type::Fn(
                params
                    .iter()
                    .map(|p| p.ty.clone())
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                Box::new(ret.clone()),
            ),
            _ => Type::Error,
        }
    }

    /// (đối số vị trí, đối số có tên) từ node `Args`.
    fn lower_args(&mut self, args: &Node) -> (Vec<HExpr>, Vec<(String, HExpr)>) {
        let mut positional = Vec::new();
        let mut named = Vec::new();
        for e in args.children() {
            match e.kind {
                NodeKind::NamedArg => {
                    let name = self.decl_name(e).unwrap_or_default();
                    let value = e
                        .children()
                        .next()
                        .map(|c| self.lower_expr(c))
                        .unwrap_or(HExpr::Error { span: e.span });
                    named.push((name, value));
                }
                _ => positional.push(self.lower_expr(e)),
            }
        }
        (positional, named)
    }

    /// **VD-263** — như [`Lowerer::lower_args`] nhưng truyền **kiểu mong đợi** của tham số
    /// (`Some(x)` dưới annot `T?`, đối số biến thể/trường của instantiation) xuống từng biểu thức.
    fn lower_args_expect(
        &mut self,
        args: &Node,
        callee: &HExpr,
        expect: Option<&Type>,
    ) -> (Vec<HExpr>, Vec<(String, HExpr)>) {
        let (pos_expect, named_expect) = self.expect_call_args(callee, expect);
        let mut positional = Vec::new();
        let mut named = Vec::new();
        for e in args.children() {
            match e.kind {
                NodeKind::NamedArg => {
                    let name = self.decl_name(e).unwrap_or_default();
                    let want = named_expect
                        .iter()
                        .find(|(n, _)| n == &name)
                        .map(|(_, t)| t.clone());
                    let value = e
                        .children()
                        .next()
                        .map(|c| self.lower_expr_as(c, want))
                        .unwrap_or(HExpr::Error { span: e.span });
                    named.push((name, value));
                }
                _ => {
                    let want = pos_expect.get(positional.len()).cloned();
                    positional.push(self.lower_expr_as(e, want));
                }
            }
        }
        (positional, named)
    }

    /// **VD-263** — kiểu mong đợi của từng đối số theo **hình dạng lời gọi**:
    /// `Some(x)` dưới `T?`, `Ok(x)`/`Err(e)` dưới `Result`, đối số biến thể/trường của
    /// `EnumId`/`RecId` **instantiation**. Không khớp ⇒ rỗng (hạ như cũ).
    fn expect_call_args(
        &self,
        callee: &HExpr,
        expect: Option<&Type>,
    ) -> (Vec<Type>, Vec<(String, Type)>) {
        // **VD-263**: chữ ký hàm đã hạ (`fn f(b: Box[int?])`) là kiểu mong đợi của đối số.
        // **VD-265**: pre-pass nạp **mọi** chữ ký (hàm khai sau call-site), kiểu `fn(…)` của
        // biến (giá trị hàm/closure), và chữ ký phương thức `impl` khi bên nhận có kiểu tĩnh.
        let (mut pos, named) = match callee {
            HExpr::Var { sym: Some(id), .. } => match self.fn_params.get(id) {
                Some(ps) => (
                    ps.iter().map(|(_, t)| t.clone()).collect::<Vec<_>>(),
                    ps.clone(),
                ),
                None => match self.type_env.get(id) {
                    Some(Type::Fn(ps, _)) => (ps.to_vec(), Vec::new()),
                    _ => (Vec::new(), Vec::new()),
                },
            },
            HExpr::Lambda { params, .. } => (
                params.iter().map(|p| p.ty.clone()).collect(),
                params
                    .iter()
                    .map(|p| (p.name.clone(), p.ty.clone()))
                    .collect(),
            ),
            HExpr::Field {
                base,
                name,
                optional: false,
                ..
            } => match self.tham_so_phuong_thuc(base, name) {
                Some(ps) => {
                    // `fn_params` của phương thức có `self` đầu; đối số call-site **không** có.
                    let ps: &[(String, Type)] = if ps.first().is_some_and(|(n, _)| n == "self") {
                        &ps[1..]
                    } else {
                        &ps[..]
                    };
                    (ps.iter().map(|(_, t)| t.clone()).collect(), ps.to_vec())
                }
                None => (Vec::new(), Vec::new()),
            },
            _ => (Vec::new(), Vec::new()),
        };
        let Some(expect) = expect else {
            return (pos, named);
        };
        if let HExpr::Var { name, .. } = callee {
            match (name.as_str(), expect) {
                ("Some", Type::Opt(inner)) => return (vec![(**inner).clone()], Vec::new()),
                ("Ok", Type::Result(ok, _)) => return (vec![(**ok).clone()], Vec::new()),
                ("Err", Type::Result(_, err)) => return (vec![(**err).clone()], Vec::new()),
                _ => {}
            }
        }
        // Ctor trực tiếp dưới `T?`/`T??` vẫn dựng kiểu **lõi** trước khi biên annot nâng.
        match bo_opt(expect) {
            Type::Enum(inst) => {
                let Some((ten_inst, _)) = self.tbl.enum_inst_of(*inst) else {
                    return (pos, named);
                };
                let variant = match callee {
                    HExpr::Var { name, .. } => Some(name.as_str()),
                    HExpr::Field { base, name, .. } => match &**base {
                        HExpr::Var { name: en, .. } if en == ten_inst => Some(name.as_str()),
                        _ => None,
                    },
                    _ => None,
                };
                let Some(v) = variant else {
                    return (pos, named);
                };
                if let Some(vd) = self.tbl.variant(*inst, v) {
                    pos = vd.fields.iter().map(|f| f.ty.clone()).collect();
                }
            }
            Type::Record(inst) => {
                let HExpr::Var { name, .. } = callee else {
                    return (pos, named);
                };
                let Some((base, _)) = self.tbl.rec_inst_of(*inst) else {
                    return (pos, named);
                };
                if base == name {
                    if let Some(r) = self.tbl.rec(*inst) {
                        return (
                            Vec::new(),
                            r.fields
                                .iter()
                                .map(|f| (f.name.clone(), f.ty.clone()))
                                .collect(),
                        );
                    }
                }
            }
            _ => {}
        }
        (pos, named)
    }

    /// **VD-265** — chữ ký phương thức `impl` cho `base.name` khi `base` có kiểu **tĩnh** và tên
    /// phương thức khớp **đúng một** impl (giống [`Lowerer::dispatch_tinh`] nhưng không chẩn đoán).
    fn tham_so_phuong_thuc(&self, base: &HExpr, name: &str) -> Option<Vec<(String, Type)>> {
        let ty = self.static_ty(base)?;
        let sym = self.method_sym(&ty, name)?;
        self.fn_params.get(&sym).cloned()
    }

    /// **VD-265** — kiểu `fn(…) -> …` của một **giá trị hàm**: lambda, hoặc tên hàm đã có chữ ký
    /// trong pre-pass. `None` nếu không chắc.
    pub(crate) fn fn_value_ty(&self, e: &HExpr) -> Option<Type> {
        match e {
            HExpr::Lambda { .. } => match self.type_of(e) {
                t @ Type::Fn(..) => Some(t),
                _ => None,
            },
            HExpr::Var { sym: Some(id), .. } => {
                let ps = self.fn_params.get(id)?;
                let ret = self.fn_ret.get(id).cloned().unwrap_or(Type::Unit);
                Some(Type::Fn(
                    ps.iter()
                        .map(|(_, t)| t.clone())
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                    Box::new(ret),
                ))
            }
            _ => None,
        }
    }

    /// Gọi `std.<module>.<fn>(…)` khi `<module>` đã `use` (ST-26) — tên đầy đủ `std.m.f`.
    fn std_call_named(
        &mut self,
        m: String,
        f: String,
        args: Vec<HExpr>,
        named: Vec<(String, HExpr)>,
        span: Span,
    ) -> HExpr {
        let ngan = format!("{m}.{f}");
        // **ST-60b**: `std.*` **chỉ** có đối số **vị trí** — trước đây `--desc` bị **bỏ im lặng**
        // (`named: Vec::new()` dưới kia). Nay **từ chối** `VS0401`, nêu đúng tên.
        if let Some((ten, a)) = named.first() {
            let span_a = a.span();
            self.diag(
                &vis_diag::TYPE_CODES[0],
                format!(
                    "`std.{ngan}` không nhận tham số **tên** `--{ten}` — `std.*` chỉ có đối số vị trí"
                ),
                span_a,
            );
            return HExpr::Error { span };
        }
        if vis_types_stdlib::hoan(&ngan) {
            self.diag(&NAME_CODES[0], format!("`{ngan}` chưa hỗ trợ (VS1b)"), span);
            return HExpr::Error { span };
        }
        if !vis_types_stdlib::la_std(&ngan) {
            self.diag(
                &NAME_CODES[0],
                format!("module `std.{m}` không có hàm `{f}`"),
                span,
            );
            return HExpr::Error { span };
        }
        HExpr::Call {
            callee: Box::new(HExpr::Var {
                name: format!("std.{ngan}"),
                sym: None,
                span,
            }),
            args,
            named: Vec::new(),
            span,
        }
    }

    /// ST-29 — **dispatch tĩnh**: `x.m(…)` với `x` có kiểu tĩnh **record/enum** ⇒ tìm `impl` và hạ
    /// thành gọi **trực tiếp** hàm impl (chèn `self`/`dt` làm đối số đầu). Không có ⇒ `VS0403`;
    /// nhiều ⇒ `VS0403` (mơ hồ). Kiểu không tĩnh ⇒ để nguyên (nội tại/`VS0403` ở tầng sau).
    fn dispatch_tinh(&mut self, callee: &HExpr, args: &[HExpr], span: Span) -> Option<HExpr> {
        let HExpr::Field {
            base,
            name,
            optional: false,
            ..
        } = callee
        else {
            return None;
        };
        let Some(ty) = self.static_ty(base) else {
            // ST-29: kiểu bên nhận **không tĩnh** (generic/`{error}`) — nếu tên khớp một phương thức
            // `impl` nào đó thì báo rõ (VS1b), thay vì để lỗi mờ lúc chạy.
            if self.impls.iter().any(|(_, n, _)| n == name) {
                self.diag(
                    &TYPE_CODES[2],
                    format!(
                        "không phân giải được phương thức `{name}` cho kiểu không tĩnh (VS1b — thêm chú thích kiểu)"
                    ),
                    span,
                );
                return Some(HExpr::Error { span });
            }
            return None;
        };
        let ten = self.ten_kieu(&ty)?;
        let khop: Vec<u32> = self
            .impls
            .iter()
            .filter(|(t, n, _)| self.ten_kieu(t).as_deref() == Some(ten.as_str()) && n == name)
            .map(|(_, _, sym)| *sym)
            .collect();
        match khop.len() {
            1 => {
                // Đối số đầu = chính biểu thức nhận (`self`/`dt` theo giá trị).
                let mut ds: Vec<HExpr> = Vec::with_capacity(args.len() + 1);
                ds.push((**base).clone());
                ds.extend(args.iter().cloned());
                Some(HExpr::Call {
                    callee: Box::new(HExpr::Var {
                        name: format!("{ten}.{name}"),
                        sym: Some(khop[0]),
                        span,
                    }),
                    args: ds,
                    named: Vec::new(),
                    span,
                })
            }
            0 => {
                let co_trait = self
                    .traits
                    .iter()
                    .chain(self.traits_ngoai.iter())
                    .find(|t| t.methods.iter().any(|(mn, _, _, _)| mn == name))
                    .map(|t| t.name.clone());
                let msg = match co_trait {
                    Some(tn) => format!(
                        "trait `{tn}` có phương thức `{name}` nhưng `{ten}` chưa cài (`impl {tn} for {ten}`)"
                    ),
                    None => format!("không có phương thức `{name}` cho `{ten}` (VS0403)"),
                };
                self.diag(&TYPE_CODES[2], msg, span);
                Some(HExpr::Error { span })
            }
            n => {
                self.diag(
                    &TYPE_CODES[2],
                    format!("mơ hồ: {n} impl cùng cung cấp phương thức `{name}` cho `{ten}`"),
                    span,
                );
                Some(HExpr::Error { span })
            }
        }
    }

    /// **VD-263** — chọn `RecId` **instantiation** khi kiểu mong đợi là `Type::Record(id)` cùng
    /// họ tên với bản khai báo `base`; ngược lại giữ `base`.
    fn rec_inst_theo(&self, base: RecId, expect: Option<&Type>) -> RecId {
        let Some(Type::Record(id)) = expect.map(bo_opt) else {
            return base;
        };
        let Some((ten_inst, _)) = self.tbl.rec_inst_of(*id) else {
            return base;
        };
        let Some((ten_base, _)) = self.rec_names.iter().find(|(_, b)| *b == base) else {
            return base;
        };
        if ten_inst == ten_base {
            *id
        } else {
            base
        }
    }

    /// Như [`Lowerer::rec_inst_theo`] cho enum.
    fn enum_inst_theo(&self, base: EnumId, expect: Option<&Type>) -> EnumId {
        let Some(Type::Enum(id)) = expect.map(bo_opt) else {
            return base;
        };
        let Some((ten_inst, _)) = self.tbl.enum_inst_of(*id) else {
            return base;
        };
        let Some((ten_base, _)) = self.enum_names.iter().find(|(_, b)| *b == base) else {
            return base;
        };
        if ten_inst == ten_base {
            *id
        } else {
            base
        }
    }

    /// `Circle(1.0)` / `Shape.Circle(1.0)` ⇒ [`HExpr::EnumLit`] khi biến thể **đã khai**.
    fn maybe_enum_lit(
        &mut self,
        callee: HExpr,
        args: Vec<HExpr>,
        named: Vec<(String, HExpr)>,
        expect: Option<&Type>,
        span: Span,
    ) -> HExpr {
        match &callee {
            HExpr::Var { name, .. } => {
                // **R11 (VD-178)**: `U(n: 1)` — dựng bản ghi theo **tên kiểu**.
                if !named.is_empty() {
                    if name == "RowSet" && !self.sqlite_duoc_thay() {
                        // **VD-241**: `RowSet` thuộc module `std.sqlite` — dựng ngoài phạm vi ⇒
                        // lỗi biên dịch rõ ràng, không âm thầm thành tên lạ.
                        self.diag(
                            &NAME_CODES[0],
                            "không thấy tên `RowSet`: module `std.sqlite` chưa được `use`"
                                .to_string(),
                            span,
                        );
                        return HExpr::Error { span };
                    }
                    if let Some((_, base_rid)) =
                        self.rec_names.iter().find(|(n, _)| n == name).cloned()
                    {
                        // **VD-263**: annot `Box[int?]` ⇒ dựng bằng **id instantiation** (trường
                        // `T?` đã thay) để biên dựng nâng đúng lớp.
                        let rid = self.rec_inst_theo(base_rid, expect);
                        // **R5a (VD-178)**: trường **thiếu** lấy **giá trị mặc định** của khai báo.
                        let mut fields = named;
                        // **VD-241 §6/B-B (A0, 2026-11-19)**: trường trùng tên ⇒ lỗi biên dịch.
                        let mut seen = std::collections::HashSet::with_capacity(fields.len());
                        for (n, _) in &fields {
                            if !seen.insert(n.clone()) {
                                self.diag(
                                    &vis_diag::TYPE_CODES[4],
                                    format!(
                                        "trường `{}` xuất hiện nhiều lần trên bản ghi `{}`",
                                        n, name
                                    ),
                                    span,
                                );
                                return HExpr::Error { span };
                            }
                        }
                        if let Some(rec) = self.items.iter().find_map(|it| match it {
                            crate::hir::HItem::Rec(r) if r.id == base_rid => Some(r.clone()),
                            _ => None,
                        }) {
                            for f in &rec.fields {
                                if !fields.iter().any(|(n, _)| n == &f.name) {
                                    if let Some(d) = &f.default {
                                        fields.push((f.name.clone(), d.clone()));
                                    } else {
                                        // **VD-241 §6/B-B (A0, 2026-11-19)**: trường bắt buộc thiếu ⇒ lỗi tĩnh.
                                        self.diag(
                                            &vis_diag::TYPE_CODES[4],
                                            format!(
                                                "thiếu trường bắt buộc `{}` của bản ghi `{}`",
                                                f.name, name
                                            ),
                                            span,
                                        );
                                        return HExpr::Error { span };
                                    }
                                }
                            }
                        }
                        return HExpr::Record {
                            fields,
                            ty: Type::Record(rid),
                            span,
                        };
                    }
                }
                if let Some((_, eid, _)) = self
                    .variant_index
                    .iter()
                    .find(|(n, _, _)| n == name)
                    .cloned()
                {
                    // **VD-241**: ctor dựng sẵn của enum `SqlValue` (thuộc module `std.sqlite`)
                    // ngoài phạm vi `use std.sqlite` ⇒ **không** phân giải thành biến thể — rơi
                    // qua đường gọi hàm (báo lỗi tên), tránh âm thầm tạo `SqlValue` "miễn phí".
                    let la_sqlvalue = self.sqlvalue_id() == Some(eid);
                    if la_sqlvalue && !self.sqlite_duoc_thay() {
                        // rơi xuống nhánh gọi hàm thường
                    } else {
                        // **VD-241 §3/T3 (A0, 2026-11-19)**: giữ **tên enum** cho `MakeEnum` — VM dựng
                        // `Value::Enum` đúng tên (ví dụ `Arr`/`Str` ⇒ `Json`), không còn tạo biệt danh
                        // `Arr.Arr` (trước đây json.encode của VM rơi vào fallback `{"variant":…}`).
                        let e_path = self
                            .enum_names
                            .iter()
                            .find(|(_, id)| id == &eid)
                            .map(|(n, _)| n.clone())
                            .unwrap_or_else(|| name.clone());
                        let eid = self.enum_inst_theo(eid, expect);
                        return HExpr::EnumLit {
                            enum_id: Some(eid),
                            path: vec![e_path],
                            variant: name.clone(),
                            args,
                            ty: Type::Enum(eid),
                            span,
                        };
                    }
                }
            }
            HExpr::Field {
                base,
                name,
                optional: false,
                ..
            } => {
                if let HExpr::Var {
                    name: enum_name, ..
                } = &**base
                {
                    if let Some((_, eid)) = self
                        .enum_names
                        .iter()
                        .find(|(n, _)| n == enum_name)
                        .cloned()
                    {
                        // **VD-241**: ctor có tiền tố `SqlValue.` ngoài `use std.sqlite` ⇒ từ chối.
                        if enum_name == "SqlValue" && !self.sqlite_duoc_thay() {
                            self.diag(
                                &NAME_CODES[0],
                                "không thấy tên `SqlValue`: module `std.sqlite` chưa được `use`"
                                    .to_string(),
                                span,
                            );
                            return HExpr::Error { span };
                        }
                        let eid = self.enum_inst_theo(eid, expect);
                        return HExpr::EnumLit {
                            enum_id: Some(eid),
                            path: vec![enum_name.clone()],
                            variant: name.clone(),
                            args,
                            ty: Type::Enum(eid),
                            span,
                        };
                    }
                }
            }
            _ => {}
        }
        HExpr::Call {
            callee: Box::new(callee),
            args,
            named,
            span,
        }
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
