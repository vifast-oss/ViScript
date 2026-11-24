// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `textDocument/inlayHint`: gợi ý kiểu cho `let`/`var` **không có nhãn kiểu**.
//!
//! Phạm vi (cố ý hẹp, không đoán):
//! - chỉ pattern **tên đơn** `let x = …` (`HPat::Bind`) và `var x = …` (bỏ `var self`);
//! - bỏ mọi khai báo **có** `: T` (người viết đã nói kiểu) và pattern **giã** (`let (a, b) = …`);
//! - suy kiểu **rẻ** từ HIR + `Symbols` + `TypeTable`: literal, list/set/map/record/enum,
//!   `Some(x)`/`None`, biến (kể cả `let` trước đó), lời gọi hàm **trong tệp** (kể cả generic
//!   một lớp), số học/so sánh, `field`, `index`, `with`, `expr?`, chuỗi nội suy;
//!   không suy được ⇒ **không** phát hint;
//! - bỏ kiểu `{error}`/`never`/kiểu còn `{error}` lồng (không hiển thị rác).
//!
//! Hint đặt tại **cuối tên** (byte `span.end`, đổi sang UTF-16 0-based), `label = ": " + show`,
//! `kind = 1` (Type); sắp theo nguồn, cap **200**.

use std::collections::HashMap;

use vis_core::value::Value;
use vis_hir::hir::{HBody, HCond, HExpr, HFn, HItem, HPat, HStmt, Hir};
use vis_hir::{Symbols, Type, TypeTable};

use super::jsonrpc::{m, phan_hoi, s};
use super::phan_tich::KetQua;
use super::vi_tri::vi_tri_utf16;

/// Cap số hint mỗi tệp (cùng tinh thần cap 200 của completion).
const CAP: usize = 200;

/// Một gợi ý kiểu: byte **cuối tên** + nhãn `": <kiểu>"`.
pub struct Inlay {
    pub byte: usize,
    pub nhan: String,
}

/// `true` nếu `t` còn `{error}` ở bất kỳ đâu — không hiển thị.
fn chua_loi(t: &Type) -> bool {
    match t {
        Type::Error => true,
        Type::List(a) | Type::Opt(a) => chua_loi(a),
        Type::Map(a, b) | Type::Result(a, b) => chua_loi(a) || chua_loi(b),
        Type::Fn(ps, r) => ps.iter().any(chua_loi) || chua_loi(r),
        Type::TypeRef { args, .. } => args.iter().any(chua_loi),
        _ => false,
    }
}

/// `true` nếu là kiểu số (phép toán số giữ nguyên kiểu khi hai vế cùng kiểu).
fn la_so(t: &Type) -> bool {
    matches!(
        t,
        Type::Int | Type::Float | Type::Size | Type::Duration | Type::Percent
    )
}

/// Phần tử của `list[T]`/giá trị của `map[K, V]`; kiểu khác ⇒ `None`.
fn phan_tu(t: &Type) -> Option<Type> {
    match t {
        Type::List(e) => Some((**e).clone()),
        Type::Map(_, v) => Some((**v).clone()),
        _ => None,
    }
}

/// Trạng thái suy kiểu: bảng ký hiệu/kiểu + hàm cấp tệp + env `sym → kiểu` + hints đã gom.
///
/// **VD-270**: `pub(crate)` để `typeDefinition` dùng lại **cùng** phép suy kiểu rẻ (chỉ nạp
/// `env`/`fns` bằng [`Suy::nap`], không phát hint).
pub(crate) struct Suy<'a> {
    syms: &'a Symbols,
    tbl: &'a TypeTable,
    /// Hàm cấp tệp/`impl`/`trait` theo `sym` — kiểu trả của lời gọi.
    fns: HashMap<u32, &'a HFn>,
    /// Kiểu đã biết của ký hiệu (tham số, `let`/`var` có nhãn **hoặc** suy được).
    env: HashMap<u32, Type>,
    out: Vec<Inlay>,
    /// `false` (mặc định) = chỉ nạp `env`/`fns`, **không** ghi hint (dùng cho typeDefinition).
    ghi_hint: bool,
}

impl<'a> Suy<'a> {
    pub(crate) fn moi(syms: &'a Symbols, tbl: &'a TypeTable) -> Self {
        Self {
            syms,
            tbl,
            fns: HashMap::new(),
            env: HashMap::new(),
            out: Vec::new(),
            ghi_hint: false,
        }
    }

    /// Ghi hint nếu kiểu **hiển thị được** (không `{error}`/`never`/lồng `{error}`).
    fn them(&mut self, byte: usize, kieu: &Type) {
        if !self.ghi_hint {
            return;
        }
        if kieu.is_error() || matches!(kieu, Type::Never) || chua_loi(kieu) {
            return;
        }
        self.out.push(Inlay {
            byte,
            nhan: format!(": {}", self.tbl.show(kieu)),
        });
    }

    /// **VD-270**: kiểu đã biết của ký hiệu (`env` rồi `HSym.ty`).
    pub(crate) fn kieu_sym(&self, sym: u32) -> Option<Type> {
        self.env
            .get(&sym)
            .cloned()
            .or_else(|| self.syms.get(sym).and_then(|s| s.ty.clone()))
    }

    /// **VD-270**: ghi đè kiểu của ký hiệu (vd `self` trong `impl` — lowering để `{error}`).
    pub(crate) fn dat_kieu(&mut self, sym: u32, kieu: Type) {
        self.env.insert(sym, kieu);
    }

    /// Kiểu **rẻ** của biểu thức; `None` = không chắc (không đoán).
    pub(crate) fn kieu(&self, e: &HExpr) -> Option<Type> {
        match e {
            HExpr::Lit { ty, .. } => Some(ty.clone()),
            HExpr::Var { name, sym, .. } => {
                // `None` trần: `Option[?]` (inner chưa biết) ⇒ không hiển thị.
                if name == "None" {
                    return None;
                }
                let id = (*sym)?;
                self.env
                    .get(&id)
                    .cloned()
                    .or_else(|| self.syms.get(id).and_then(|s| s.ty.clone()))
            }
            HExpr::Bin { op, lhs, rhs, .. } => {
                use vis_hir::BinOp;
                match op {
                    BinOp::Add
                    | BinOp::Sub
                    | BinOp::Mul
                    | BinOp::Div
                    | BinOp::FloorDiv
                    | BinOp::Rem
                    | BinOp::Pow => {
                        let (l, r) = (self.kieu(lhs)?, self.kieu(rhs)?);
                        if l == r && (la_so(&l) || l == Type::Str && *op == BinOp::Add) {
                            Some(l)
                        } else {
                            None
                        }
                    }
                    BinOp::Eq
                    | BinOp::Ne
                    | BinOp::Lt
                    | BinOp::Le
                    | BinOp::Gt
                    | BinOp::Ge
                    | BinOp::And
                    | BinOp::Or => Some(Type::Bool),
                    BinOp::Coalesce => match self.kieu(lhs)? {
                        Type::Opt(inner) => Some((*inner).clone()),
                        _ => None,
                    },
                    BinOp::Range | BinOp::RangeIncl | BinOp::Pipe => None,
                }
            }
            HExpr::Un { op, expr, .. } => match op {
                vis_hir::UnOp::Neg => self.kieu(expr),
                vis_hir::UnOp::Not => Some(Type::Bool),
            },
            HExpr::Call {
                callee,
                args,
                named,
                ..
            } => {
                if let HExpr::Var { name, .. } = &**callee {
                    match name.as_str() {
                        "Some" if args.len() == 1 && named.is_empty() => {
                            return self.kieu(&args[0]).map(|t| Type::Opt(Box::new(t)));
                        }
                        // `Option`/`Result` với vế còn lại chưa biết ⇒ không đoán.
                        "None" | "Ok" | "Err" if args.len() <= 1 => return None,
                        _ => {}
                    }
                }
                let HExpr::Var { sym: Some(id), .. } = &**callee else {
                    return None;
                };
                let f = self.fns.get(id)?;
                if f.generics.is_empty() {
                    return Some(f.ret.clone());
                }
                // Generic **một lớp**: hợp nhất tham số với kiểu đối số rồi thay vào kiểu trả.
                let mut bang: Vec<(String, Type)> = Vec::new();
                for (p, a) in f.params.iter().zip(args) {
                    if let Some(k) = self.kieu(a) {
                        self.hop_nhat(&p.ty, &k, &f.generics, &mut bang);
                    }
                }
                for (ten, a) in named {
                    if let Some(p) = f.params.iter().find(|p| &p.name == ten) {
                        if let Some(k) = self.kieu(a) {
                            self.hop_nhat(&p.ty, &k, &f.generics, &mut bang);
                        }
                    }
                }
                self.thay(&f.ret, &bang)
            }
            HExpr::Field { base, name, .. } => {
                let Type::Record(id) = self.kieu(base)? else {
                    return None;
                };
                self.tbl.field(id, name).map(|f| f.ty.clone())
            }
            HExpr::Index { base, .. } => self.kieu(base).and_then(|t| phan_tu(&t)),
            HExpr::List { items, elem, .. } | HExpr::Set { items, elem, .. } => {
                let e = self.kieu_danh_sach(items, elem)?;
                Some(Type::List(Box::new(e)))
            }
            HExpr::Map {
                entries,
                key,
                value,
                ..
            } => {
                let k = if key.is_error() {
                    entries.first().and_then(|(k, _)| self.kieu(k))
                } else {
                    Some(key.clone())
                }?;
                let v = if value.is_error() {
                    entries.first().and_then(|(_, v)| self.kieu(v))
                } else {
                    Some(value.clone())
                }?;
                Some(Type::Map(Box::new(k), Box::new(v)))
            }
            HExpr::Record { ty, .. } | HExpr::EnumLit { ty, .. } | HExpr::Tuple { ty, .. } => {
                Some(ty.clone())
            }
            HExpr::Opt { value, ty, .. } => {
                if ty.is_error() {
                    self.kieu(value.as_deref()?).map(|t| Type::Opt(Box::new(t)))
                } else {
                    Some(ty.clone())
                }
            }
            HExpr::Try { expr, .. } => match self.kieu(expr)? {
                Type::Opt(inner) => Some((*inner).clone()),
                Type::Result(ok, _) => Some((*ok).clone()),
                _ => None,
            },
            HExpr::With { base, .. } => self.kieu(base),
            HExpr::If { ty, .. } | HExpr::Match { ty, .. } => {
                if ty.is_error() {
                    None
                } else {
                    Some(ty.clone())
                }
            }
            HExpr::Interp { .. } => Some(Type::Str),
            // `ref`/`await`/`spawn`/`chan`/`cmd`/lambda: chưa có kiểu tĩnh chắc chắn ⇒ bỏ.
            _ => None,
        }
    }

    /// Phần tử của list literal: `elem` do lowering điền; nếu `{error}`, lấy kiểu chung của
    /// các phần tử **suy được** và đòi tất cả khớp (không đoán khi lẫn kiểu).
    fn kieu_danh_sach(&self, items: &[HExpr], elem: &Type) -> Option<Type> {
        if !elem.is_error() {
            return Some(elem.clone());
        }
        let mut chung: Option<Type> = None;
        for it in items {
            let k = self.kieu(it)?;
            match &chung {
                Some(cu) if cu != &k => return None,
                Some(_) => {}
                None => chung = Some(k),
            }
        }
        chung
    }

    /// Hợp nhất `mau` (có tham số generic) với `thuc` ⇒ ghi `T → kiểu` (lần gặp đầu).
    fn hop_nhat(&self, mau: &Type, thuc: &Type, gens: &[String], bang: &mut Vec<(String, Type)>) {
        match (mau, thuc) {
            (Type::TypeRef { name, args }, t)
                if args.is_empty() && gens.iter().any(|g| g == name) =>
            {
                if !bang.iter().any(|(n, _)| n == name) {
                    bang.push((name.clone(), t.clone()));
                }
            }
            (Type::List(a), Type::List(b)) | (Type::Opt(a), Type::Opt(b)) => {
                self.hop_nhat(a, b, gens, bang);
            }
            (Type::Map(a1, a2), Type::Map(b1, b2))
            | (Type::Result(a1, a2), Type::Result(b1, b2)) => {
                self.hop_nhat(a1, b1, gens, bang);
                self.hop_nhat(a2, b2, gens, bang);
            }
            (Type::Fn(pa, ra), Type::Fn(pb, rb)) => {
                for (a, b) in pa.iter().zip(pb.iter()) {
                    self.hop_nhat(a, b, gens, bang);
                }
                self.hop_nhat(ra, rb, gens, bang);
            }
            (Type::TypeRef { args, .. }, Type::TypeRef { args: b, .. }) => {
                for (a, b) in args.iter().zip(b.iter()) {
                    self.hop_nhat(a, b, gens, bang);
                }
            }
            _ => {}
        }
    }

    /// Thay tham số generic bằng kiểu đã hợp nhất; còn `T` chưa có ⇒ `None` (bỏ hint).
    fn thay(&self, t: &Type, bang: &[(String, Type)]) -> Option<Type> {
        Some(match t {
            Type::TypeRef { name, args } if args.is_empty() => bang
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, x)| x.clone())?,
            Type::TypeRef { name, args } => Type::TypeRef {
                name: name.clone(),
                args: args
                    .iter()
                    .map(|a| self.thay(a, bang))
                    .collect::<Option<Vec<_>>>()?,
            },
            Type::List(a) => Type::List(Box::new(self.thay(a, bang)?)),
            Type::Map(k, v) => {
                Type::Map(Box::new(self.thay(k, bang)?), Box::new(self.thay(v, bang)?))
            }
            Type::Opt(a) => Type::Opt(Box::new(self.thay(a, bang)?)),
            Type::Result(a, b) => {
                Type::Result(Box::new(self.thay(a, bang)?), Box::new(self.thay(b, bang)?))
            }
            Type::Fn(ps, r) => Type::Fn(
                ps.iter()
                    .map(|a| self.thay(a, bang))
                    .collect::<Option<Vec<_>>>()?
                    .into_boxed_slice(),
                Box::new(self.thay(r, bang)?),
            ),
            other => other.clone(),
        })
    }

    /// Gán kiểu cho pattern đơn giản (`for x in …`); pattern khác bỏ qua.
    fn bind_pat(&mut self, pat: &HPat, t: &Type) {
        if let HPat::Bind { sym, .. } = pat {
            self.env.insert(*sym, t.clone());
        }
    }

    /// Duyệt một dãy câu lệnh theo thứ tự nguồn.
    fn stmts(&mut self, ds: &[HStmt]) {
        for st in ds {
            self.stmt(st);
        }
    }

    fn stmt(&mut self, st: &HStmt) {
        match st {
            HStmt::Let {
                pat,
                ty,
                init,
                else_,
                ..
            } => {
                self.duyet_expr(init);
                if let Some(t) = ty {
                    if let HPat::Bind { sym, .. } = pat {
                        self.env.insert(*sym, t.clone());
                    }
                } else if let HPat::Bind { span, sym, .. } = pat {
                    if let Some(k) = self.kieu(init) {
                        self.them(span.end, &k);
                        self.env.insert(*sym, k);
                    }
                }
                if let Some(e) = else_ {
                    self.stmts(&e.body);
                }
            }
            HStmt::Var {
                sym,
                ty,
                init,
                is_self,
                ..
            } => {
                self.duyet_expr(init);
                if !*is_self {
                    if let Some(t) = ty {
                        self.env.insert(*sym, t.clone());
                    } else if let Some(k) = self.kieu(init) {
                        // `HStmt::Var.span` phủ cả câu; tên nằm ở `HSym.span`.
                        if let Some(byte) = self.syms.get(*sym).map(|s| s.span.end) {
                            self.them(byte, &k);
                        }
                        self.env.insert(*sym, k);
                    }
                }
            }
            HStmt::If {
                cond,
                then,
                elifs,
                else_,
                ..
            } => {
                self.duyet_cond(cond);
                self.stmts(then);
                for (c, b) in elifs {
                    self.duyet_cond(c);
                    self.stmts(b);
                }
                if let Some(b) = else_ {
                    self.stmts(b);
                }
            }
            HStmt::For {
                pat, iter, body, ..
            } => {
                if let Some(t) = self.kieu(iter) {
                    if let Some(e) = phan_tu(&t) {
                        self.bind_pat(pat, &e);
                    }
                }
                self.duyet_expr(iter);
                self.stmts(body);
            }
            HStmt::While { cond, body, .. } => {
                self.duyet_cond(cond);
                self.stmts(body);
            }
            HStmt::Loop { body, .. } | HStmt::Scope { body, .. } | HStmt::Block { body, .. } => {
                self.stmts(body)
            }
            HStmt::Match { subject, arms, .. } => {
                self.duyet_expr(subject);
                for a in arms {
                    if let Some(g) = &a.guard {
                        self.duyet_expr(g);
                    }
                    self.stmts(&a.body);
                }
            }
            HStmt::Expr(e) => self.duyet_expr(e),
            HStmt::Assign { value, .. } => self.duyet_expr(value),
            HStmt::Return { value, .. } => {
                if let Some(v) = value {
                    self.duyet_expr(v);
                }
            }
            HStmt::Break { .. } | HStmt::Continue { .. } => {}
        }
    }

    fn duyet_cond(&mut self, c: &HCond) {
        match c {
            HCond::Expr(e) => self.duyet_expr(e),
            HCond::Let { value, .. } => self.duyet_expr(value),
        }
    }

    /// Duyệt biểu thức **chỉ để tìm khối lồng** (`if`/`match`/lambda) chứa `let`.
    fn duyet_expr(&mut self, e: &HExpr) {
        match e {
            HExpr::If {
                cond,
                then,
                elifs,
                else_,
                ..
            } => {
                self.duyet_cond(cond);
                self.stmts(then);
                for (c, b) in elifs {
                    self.duyet_cond(c);
                    self.stmts(b);
                }
                self.stmts(else_);
            }
            HExpr::Match { subject, arms, .. } => {
                self.duyet_expr(subject);
                for a in arms {
                    if let Some(g) = &a.guard {
                        self.duyet_expr(g);
                    }
                    self.stmts(&a.body);
                }
            }
            HExpr::Lambda { body, .. } => match body {
                HBody::Expr(x) => self.duyet_expr(x),
                HBody::Stmts(ds) => self.stmts(ds),
            },
            HExpr::Bin { lhs, rhs, .. } => {
                self.duyet_expr(lhs);
                self.duyet_expr(rhs);
            }
            HExpr::Un { expr, .. }
            | HExpr::Try { expr, .. }
            | HExpr::Ref { expr, .. }
            | HExpr::Await { expr, .. } => self.duyet_expr(expr),
            HExpr::Call {
                callee,
                args,
                named,
                ..
            } => {
                self.duyet_expr(callee);
                for a in args {
                    self.duyet_expr(a);
                }
                for (_, a) in named {
                    self.duyet_expr(a);
                }
            }
            HExpr::Spawn {
                callee,
                args,
                named,
                ..
            } => {
                self.duyet_expr(callee);
                for a in args {
                    self.duyet_expr(a);
                }
                for (_, a) in named {
                    self.duyet_expr(a);
                }
            }
            HExpr::Field { base, .. } => self.duyet_expr(base),
            HExpr::Index { base, index, .. } => {
                self.duyet_expr(base);
                self.duyet_expr(index);
            }
            HExpr::List { items, .. } | HExpr::Set { items, .. } | HExpr::Tuple { items, .. } => {
                for it in items {
                    self.duyet_expr(it);
                }
            }
            HExpr::Map { entries, .. } => {
                for (k, v) in entries {
                    self.duyet_expr(k);
                    self.duyet_expr(v);
                }
            }
            HExpr::Record { fields, .. } => {
                for (_, v) in fields {
                    self.duyet_expr(v);
                }
            }
            HExpr::EnumLit { args, .. } => {
                for a in args {
                    self.duyet_expr(a);
                }
            }
            HExpr::Opt { value: Some(v), .. } => self.duyet_expr(v),
            HExpr::With { base, fields, .. } => {
                self.duyet_expr(base);
                for (_, v) in fields {
                    self.duyet_expr(v);
                }
            }
            HExpr::ChanNew { cap, .. } => self.duyet_expr(cap),
            HExpr::Interp { parts, .. } => {
                for p in parts {
                    match p {
                        vis_hir::InterpPart::Str(_) => {}
                        vis_hir::InterpPart::Expr(x)
                        | vis_hir::InterpPart::ExprSpec { expr: x, .. } => self.duyet_expr(x),
                    }
                }
            }
            _ => {}
        }
    }
}

impl<'a> Suy<'a> {
    /// **VD-270**: nạp `fns` + `env` cho **toàn bộ** HIR; chỉ ghi hint khi `ghi_hint = true`.
    pub(crate) fn nap(&mut self, hir: &'a Hir) {
        for it in &hir.items {
            match it {
                HItem::Fn(f) => {
                    self.fns.insert(f.sym, f);
                }
                HItem::Impl(imp) => {
                    for f in &imp.methods {
                        self.fns.insert(f.sym, f);
                    }
                }
                HItem::Trait(t) => {
                    for f in &t.methods {
                        self.fns.insert(f.sym, f);
                    }
                }
                _ => {}
            }
        }
        // Cấp tệp trước (để hàm/khối sau dùng được `let` cấp tệp), rồi thân item.
        self.stmts(&hir.stmts);
        for it in &hir.items {
            match it {
                HItem::Fn(f) => {
                    for p in &f.params {
                        if !p.ty.is_error() {
                            self.env.insert(p.sym, p.ty.clone());
                        }
                    }
                    self.stmts(&f.body);
                }
                HItem::Impl(imp) => {
                    for f in &imp.methods {
                        for p in &f.params {
                            if !p.ty.is_error() {
                                self.env.insert(p.sym, p.ty.clone());
                            }
                        }
                        self.stmts(&f.body);
                    }
                }
                HItem::Trait(t) => {
                    for f in &t.methods {
                        for p in &f.params {
                            if !p.ty.is_error() {
                                self.env.insert(p.sym, p.ty.clone());
                            }
                        }
                        self.stmts(&f.body);
                    }
                }
                HItem::Const(c) => self.duyet_expr(&c.value),
                _ => {}
            }
        }
    }
}

/// Gom inlay hint từ HIR: `let`/`var` không nhãn, theo thứ tự nguồn, cap [`CAP`].
pub fn gom(hir: &Hir, syms: &Symbols, tbl: &TypeTable) -> Vec<Inlay> {
    let mut s = Suy::moi(syms, tbl);
    s.ghi_hint = true;
    s.nap(hir);
    let mut out = s.out;
    out.sort_by_key(|h| h.byte);
    out.truncate(CAP);
    out
}

/// Phản hồi `textDocument/inlayHint` (`result` = mảng `InlayHint`, thứ tự khoá `position`,
/// `label`, `kind`).
pub fn tra_loi(kq: &KetQua, id: &Value) -> Vec<u8> {
    let items: Vec<Value> = kq
        .inlay
        .iter()
        .map(|h| {
            let (line, character) = vi_tri_utf16(&kq.text, h.byte);
            m(vec![
                (
                    "position",
                    m(vec![
                        ("line", Value::Int(line as i64)),
                        ("character", Value::Int(character as i64)),
                    ]),
                ),
                ("label", s(&h.nhan)),
                ("kind", Value::Int(1)),
            ])
        })
        .collect();
    phan_hoi(id, Value::List(items))
}
