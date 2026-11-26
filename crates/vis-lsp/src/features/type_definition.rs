// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `textDocument/typeDefinition` cho LSP.
//!
//! Từ tại con trỏ (biến / tham số / trường / `self`) được suy **kiểu tĩnh** bằng chính bộ suy
//! kiểu rẻ của [`super::inlay_hint`] (`Suy`: env `sym → kiểu` + hàm cấp tệp + `TypeTable`).
//! Chỉ khi kiểu là **bản ghi/enum người dùng** khai trong HIR của tệp mở mới trả `Location[]`
//! tại **đúng token tên khai báo**; kiểu dựng sẵn (`int`/`str`/`list`/`map`/`Option`/`Result`/…),
//! `{error}`, kiểu không suy được, hàm/module ⇒ `result:null` (không phải `[]`).
//!
//! Phạm vi cố ý hẹp (không đoán):
//! - từ phải là **định danh của một giá trị**: `Var`, tên trường trong `base.field`, tên kiểu
//!   trong constructor `User(…)`/`Shape.Circle(…)`, hoặc tên khai báo `let`/`var`/tham số/trường;
//! - không trả cho tên hàm (kể cả lời gọi trả bản ghi), nhãn đối số, từ khoá, type parameter;
//! - `T?`/`Option[T]`/`list[T]`/`Result[T, E]` giữ nguyên **kiểu ngoài cùng** ⇒ `null` (không bóc
//!   lớp); alias đã được lowering thay bằng kiểu RHS nên `type Alias = User` trỏ `User`.
//!
//! Khử trùng theo span, sắp theo nguồn, cap **50**. Range 0-based **UTF-16** (`vi_tri::khoang`).

use std::collections::HashSet;

use vis_core::value::Value;
use vis_hir::hir::{HBody, HCond, HExpr, HFn, HItem, HPat, HStmt, Hir};
use vis_hir::{EnumId, RecId, Symbols, Type, TypeTable};
use vis_syntax::Span;

use super::dinh_nghia::thu_hep_ten;
use super::goi_y::tu_tai;
use super::inlay_hint::Suy;
use super::jsonrpc::{m, phan_hoi, s};
use super::phan_tich::KetQua;
use super::vi_tri::khoang;

/// Số `Location` tối đa (cùng tinh thần cap 50 của implementation).
const CAP: usize = 50;

/// Một **từ** tại điểm dùng đã phân giải về khai báo kiểu người dùng: span của từ + span tên
/// khai báo (nhiều hơn một chỉ khi trùng tên khai báo — hiếm).
pub struct NutKieu {
    pub span: Span,
    pub dich: Vec<Span>,
}

fn la_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Tìm `ten` **trọn từ** trong `span` mà ký tự đầu tiên sau nó (bỏ khoảng trắng) thuộc `sau`
/// (`sau` rỗng ⇒ không ràng buộc). Trả span byte của lần gặp **đầu tiên**.
fn tim_tu(text: &str, span: Span, ten: &str, sau: &[char]) -> Option<Span> {
    if ten.is_empty() || span.start >= span.end || span.end > text.len() {
        return None;
    }
    let seg = &text[span.start..span.end];
    let mut at = 0usize;
    while let Some(i) = seg[at..].find(ten) {
        let a = at + i;
        let b = a + ten.len();
        let truoc = seg[..a].chars().next_back();
        let sau_ch = seg[b..].chars().next();
        let ria = |c: Option<char>| c.is_none_or(|c| !la_ident(c));
        if ria(truoc) && ria(sau_ch) {
            let rest = seg[b..].trim_start();
            if sau.is_empty() || sau.iter().any(|c| rest.starts_with(*c)) {
                return Some(Span::new(span.start + a, span.start + b));
            }
        }
        at = b;
    }
    None
}

/// Span của `ten` ở **cuối** `span` (tên trường của `base.ten`); fallback lần gặp cuối.
fn span_cuoi(text: &str, span: Span, ten: &str) -> Option<Span> {
    if ten.is_empty() || span.start >= span.end || span.end > text.len() {
        return None;
    }
    if span.end >= ten.len() {
        let a = span.end - ten.len();
        if a >= span.start && text.get(a..span.end) == Some(ten) {
            let truoc = text[..a].chars().next_back();
            if truoc.is_none_or(|c| !la_ident(c)) {
                return Some(Span::new(a, span.end));
            }
        }
    }
    let seg = &text[span.start..span.end];
    let mut tim = None;
    let mut at = 0usize;
    while let Some(i) = seg[at..].find(ten) {
        let a = at + i;
        let b = a + ten.len();
        let truoc = seg[..a].chars().next_back();
        let sau = seg[b..].chars().next();
        let ria = |c: Option<char>| c.is_none_or(|c| !la_ident(c));
        if ria(truoc) && ria(sau) {
            tim = Some(Span::new(span.start + a, span.start + b));
        }
        at = b;
    }
    tim
}

/// Bản ghi/enum người dùng đã gom trong HIR; `text` để thu hẹp span khai báo về đúng tên.
struct Gom<'a> {
    text: &'a str,
    suy: Suy<'a>,
    syms: &'a Symbols,
    tbl: &'a TypeTable,
    items: &'a [HItem],
    out: Vec<NutKieu>,
}

impl Gom<'_> {
    /// Ghi một từ nếu kiểu của nó trỏ tới bản ghi/enum khai trong tệp.
    fn them_tu(&mut self, span: Span, kieu: Option<&Type>) {
        let Some(k) = kieu else {
            return;
        };
        let dich = self.dich(k);
        if dich.is_empty() || span.start >= span.end {
            return;
        }
        self.out.push(NutKieu { span, dich });
    }

    /// Span tên khai báo cho **đúng** kiểu `k`; rỗng = không phải bản ghi/enum người dùng.
    fn dich(&self, k: &Type) -> Vec<Span> {
        match k {
            Type::Record(id) => self.rec_spans(*id),
            Type::Enum(id) => self.enum_spans(*id),
            // `Hop[T]` (generic còn ký hiệu) vẫn là bản ghi/enum đã khai ⇒ trỏ khai báo gốc.
            Type::TypeRef { name, .. } => self.ten_spans(name),
            _ => Vec::new(),
        }
    }

    fn rec_spans(&self, id: RecId) -> Vec<Span> {
        let base = self
            .tbl
            .rec_inst_of(id)
            .and_then(|(ten, _)| self.tbl.find_rec_base(ten))
            .unwrap_or(id);
        let ten = self.tbl.rec(id).and_then(|r| r.name.clone());
        let mut out = Vec::new();
        for it in self.items {
            let HItem::Rec(r) = it else {
                continue;
            };
            if r.id == base || ten.as_deref() == Some(r.name.as_str()) {
                out.push(thu_hep_ten(self.text, &r.name, r.span));
            }
        }
        out
    }

    fn enum_spans(&self, id: EnumId) -> Vec<Span> {
        let base = self
            .tbl
            .enum_inst_of(id)
            .and_then(|(ten, _)| self.tbl.find_enum_base(ten))
            .unwrap_or(id);
        let ten = self.tbl.enum_def(id).map(|e| e.name.clone());
        let mut out = Vec::new();
        for it in self.items {
            let HItem::Enum(e) = it else {
                continue;
            };
            if e.id == base || ten.as_deref() == Some(e.name.as_str()) {
                out.push(thu_hep_ten(self.text, &e.name, e.span));
            }
        }
        out
    }

    /// Mọi khai báo **trùng tên** (record/enum) — nhiều vị trí chỉ khi tên thật sự trùng.
    fn ten_spans(&self, name: &str) -> Vec<Span> {
        self.items
            .iter()
            .filter_map(|it| match it {
                HItem::Rec(r) if r.name == name => Some(thu_hep_ten(self.text, &r.name, r.span)),
                HItem::Enum(e) if e.name == name => Some(thu_hep_ten(self.text, &e.name, e.span)),
                _ => None,
            })
            .collect()
    }

    fn stmts(&mut self, ds: &[HStmt]) {
        for st in ds {
            self.stmt(st);
        }
    }

    fn stmt(&mut self, st: &HStmt) {
        match st {
            HStmt::Let {
                pat, init, else_, ..
            } => {
                self.expr(init);
                self.pat(pat);
                if let Some(e) = else_ {
                    self.stmts(&e.body);
                }
            }
            HStmt::Var {
                sym, init, is_self, ..
            } => {
                self.expr(init);
                if !*is_self {
                    if let Some(s) = self.syms.get(*sym) {
                        let k = self.suy.kieu_sym(*sym);
                        self.them_tu(s.span, k.as_ref());
                    }
                }
            }
            HStmt::Assign { target, value, .. } => {
                self.expr(target);
                self.expr(value);
            }
            HStmt::Expr(e) => self.expr(e),
            HStmt::If {
                cond,
                then,
                elifs,
                else_,
                ..
            } => {
                self.cond(cond);
                self.stmts(then);
                for (c, b) in elifs {
                    self.cond(c);
                    self.stmts(b);
                }
                if let Some(b) = else_ {
                    self.stmts(b);
                }
            }
            HStmt::For {
                pat, iter, body, ..
            } => {
                self.expr(iter);
                self.pat(pat);
                self.stmts(body);
            }
            HStmt::While { cond, body, .. } => {
                self.cond(cond);
                self.stmts(body);
            }
            HStmt::Loop { body, .. } | HStmt::Scope { body, .. } | HStmt::Block { body, .. } => {
                self.stmts(body)
            }
            HStmt::Return { value, .. } => {
                if let Some(v) = value {
                    self.expr(v);
                }
            }
            HStmt::Break { .. } | HStmt::Continue { .. } => {}
            HStmt::Match { subject, arms, .. } => {
                self.expr(subject);
                for a in arms {
                    self.pat(&a.pat);
                    if let Some(g) = &a.guard {
                        self.expr(g);
                    }
                    self.stmts(&a.body);
                }
            }
        }
    }

    fn cond(&mut self, c: &HCond) {
        match c {
            HCond::Expr(e) => self.expr(e),
            HCond::Let { pat, value, .. } => {
                self.expr(value);
                self.pat(pat);
            }
        }
    }

    /// Binder của pattern: chỉ tên đơn có kiểu trong `env` mới trỏ được khai báo.
    fn pat(&mut self, p: &HPat) {
        match p {
            HPat::Bind { span, sym, .. } => {
                let k = self.suy.kieu_sym(*sym);
                self.them_tu(*span, k.as_ref());
            }
            HPat::Tuple { items, .. } | HPat::List { items, .. } => {
                for x in items {
                    self.pat(x);
                }
            }
            HPat::Ctor { args, .. } => {
                for x in args {
                    self.pat(x);
                }
            }
            HPat::Record { fields, .. } => {
                for (_, x) in fields {
                    if let Some(x) = x {
                        self.pat(x);
                    }
                }
            }
            HPat::Or { alts, .. } => {
                for x in alts {
                    self.pat(x);
                }
            }
            _ => {}
        }
    }

    /// Tham số hình thức (tên khai báo → kiểu). `self` của `impl` lấy kiểu khối `impl`.
    fn tham_so(&mut self, f: &HFn, self_ty: Option<&Type>) {
        for p in &f.params {
            let k = if p.is_self || p.is_var_self || p.name == "self" {
                self_ty.cloned().or_else(|| Some(p.ty.clone()))
            } else {
                Some(p.ty.clone())
            };
            self.them_tu(p.span, k.as_ref());
        }
    }

    fn expr(&mut self, e: &HExpr) {
        match e {
            HExpr::Var { span, .. } => {
                let k = self.suy.kieu(e);
                self.them_tu(*span, k.as_ref());
            }
            HExpr::Field {
                base, name, span, ..
            } => {
                let k = self.suy.kieu(e);
                if let Some(w) = span_cuoi(self.text, *span, name) {
                    self.them_tu(w, k.as_ref());
                }
                self.expr(base);
            }
            HExpr::Record {
                fields, ty, span, ..
            } => {
                if let Some(ten) = self.ten_ban_ghi(ty) {
                    if let Some(w) = tim_tu(self.text, *span, &ten, &['(', '[']) {
                        self.them_tu(w, Some(ty));
                    }
                }
                for (_, v) in fields {
                    self.expr(v);
                }
            }
            HExpr::EnumLit {
                path,
                variant,
                args,
                ty,
                span,
                ..
            } => {
                for ten in path {
                    if let Some(w) = tim_tu(self.text, *span, ten, &['.']) {
                        self.them_tu(w, Some(ty));
                    }
                }
                if let Some(w) = tim_tu(self.text, *span, variant, &[]) {
                    self.them_tu(w, Some(ty));
                }
                for a in args {
                    self.expr(a);
                }
            }
            HExpr::Bin { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            HExpr::Un { expr, .. }
            | HExpr::Try { expr, .. }
            | HExpr::Ref { expr, .. }
            | HExpr::Await { expr, .. } => self.expr(expr),
            HExpr::Call {
                callee,
                args,
                named,
                ..
            }
            | HExpr::Spawn {
                callee,
                args,
                named,
                ..
            } => {
                self.expr(callee);
                for a in args {
                    self.expr(a);
                }
                for (_, a) in named {
                    self.expr(a);
                }
            }
            HExpr::Index { base, index, .. } => {
                self.expr(base);
                self.expr(index);
            }
            HExpr::List { items, .. } | HExpr::Set { items, .. } | HExpr::Tuple { items, .. } => {
                for it in items {
                    self.expr(it);
                }
            }
            HExpr::Map { entries, .. } => {
                for (k, v) in entries {
                    self.expr(k);
                    self.expr(v);
                }
            }
            HExpr::Opt { value: Some(v), .. } => self.expr(v),
            HExpr::Opt { value: None, .. } => {}
            HExpr::With { base, fields, .. } => {
                self.expr(base);
                for (_, v) in fields {
                    self.expr(v);
                }
            }
            HExpr::If {
                cond,
                then,
                elifs,
                else_,
                ..
            } => {
                self.cond(cond);
                self.stmts(then);
                for (c, b) in elifs {
                    self.cond(c);
                    self.stmts(b);
                }
                self.stmts(else_);
            }
            HExpr::Match { subject, arms, .. } => {
                self.expr(subject);
                for a in arms {
                    self.pat(&a.pat);
                    if let Some(g) = &a.guard {
                        self.expr(g);
                    }
                    self.stmts(&a.body);
                }
            }
            HExpr::Lambda { params, body, .. } => {
                for p in params {
                    self.them_tu(p.span, Some(&p.ty));
                }
                match body {
                    HBody::Expr(x) => self.expr(x),
                    HBody::Stmts(ds) => self.stmts(ds),
                }
            }
            HExpr::ChanNew { cap, .. } => self.expr(cap),
            HExpr::Interp { parts, .. } => {
                for p in parts {
                    match p {
                        vis_hir::InterpPart::Str(_) => {}
                        vis_hir::InterpPart::Expr(x)
                        | vis_hir::InterpPart::ExprSpec { expr: x, .. } => self.expr(x),
                    }
                }
            }
            HExpr::Lit { .. } | HExpr::CmdBlock { .. } | HExpr::Error { .. } => {}
        }
    }

    /// Tên bản ghi của literal `User(…)` (kể cả instantiation `Hop[int]`); ẩn danh ⇒ `None`.
    fn ten_ban_ghi(&self, ty: &Type) -> Option<String> {
        let Type::Record(id) = ty else {
            return None;
        };
        if let Some((ten, _)) = self.tbl.rec_inst_of(*id) {
            return Some(ten.clone());
        }
        self.tbl.rec(*id).and_then(|r| r.name.clone())
    }
}

/// Gom mọi **từ** trong HIR đã phân giải về khai báo bản ghi/enum của tệp.
pub fn gom(hir: &Hir, syms: &Symbols, tbl: &TypeTable, text: &str) -> Vec<NutKieu> {
    let mut suy = Suy::moi(syms, tbl);
    suy.nap(hir);
    // `self` trong `impl` chưa có kiểu ở `HParam` (lowering để `{error}`) — lấy kiểu khối `impl`.
    for it in &hir.items {
        if let HItem::Impl(imp) = it {
            for f in &imp.methods {
                for p in &f.params {
                    if p.is_self || p.is_var_self || p.name == "self" {
                        suy.dat_kieu(p.sym, imp.ty.clone());
                    }
                }
            }
        }
    }
    let mut g = Gom {
        text,
        suy,
        syms,
        tbl,
        items: &hir.items,
        out: Vec::new(),
    };
    g.stmts(&hir.stmts);
    for it in &hir.items {
        match it {
            HItem::Fn(f) => {
                g.tham_so(f, None);
                g.stmts(&f.body);
            }
            HItem::Impl(imp) => {
                for f in &imp.methods {
                    g.tham_so(f, Some(&imp.ty));
                    g.stmts(&f.body);
                }
            }
            HItem::Trait(t) => {
                for f in &t.methods {
                    g.tham_so(f, None);
                    g.stmts(&f.body);
                }
            }
            HItem::Extern(x) => {
                for p in &x.params {
                    g.them_tu(p.span, Some(&p.ty));
                }
            }
            HItem::Const(c) => {
                let w = thu_hep_ten(text, &c.name, c.span);
                g.them_tu(w, Some(&c.ty));
                g.expr(&c.value);
            }
            HItem::Rec(r) => {
                for f in &r.fields {
                    let w = thu_hep_ten(text, &f.name, f.span);
                    g.them_tu(w, Some(&f.ty));
                    if let Some(d) = &f.default {
                        g.expr(d);
                    }
                }
            }
            HItem::Enum(e) => {
                for v in &e.variants {
                    for f in &v.fields {
                        let w = thu_hep_ten(text, &f.name, f.span);
                        g.them_tu(w, Some(&f.ty));
                        if let Some(d) = &f.default {
                            g.expr(d);
                        }
                    }
                }
            }
            HItem::Alias(_) | HItem::Use(_) => {}
        }
    }
    g.out
}

/// Phản hồi `textDocument/typeDefinition`: `Location[]` tại token tên khai báo kiểu, hoặc
/// `result:null` khi không có từ / kiểu không phải bản ghi-enum người dùng.
pub fn tra_loi(kq: &KetQua, uri: &str, byte: usize, id: &Value) -> Vec<u8> {
    let (start, end) = tu_tai(&kq.text, byte);
    if start == end {
        return phan_hoi(id, Value::none());
    }
    let tu = Span::new(start, end);
    // Nút **nhỏ nhất** chứa từ quyết định (trường thắng biểu thức cha, biến thắng `base.field`).
    let mut chon: Option<&NutKieu> = None;
    for n in &kq.kieu {
        if tu.start < n.span.start || tu.end > n.span.end {
            continue;
        }
        let dai = n.span.end - n.span.start;
        if chon.is_none_or(|c| dai < c.span.end - c.span.start) {
            chon = Some(n);
        }
    }
    let Some(n) = chon else {
        return phan_hoi(id, Value::none());
    };
    let mut da: HashSet<(usize, usize)> = HashSet::new();
    let mut spans: Vec<Span> = n
        .dich
        .iter()
        .copied()
        .filter(|s| s.start < s.end && da.insert((s.start, s.end)))
        .collect();
    if spans.is_empty() {
        return phan_hoi(id, Value::none());
    }
    spans.sort_by_key(|s| (s.start, s.end));
    spans.truncate(CAP);
    let pos = |line: u32, character: u32| {
        m(vec![
            ("line", Value::Int(line as i64)),
            ("character", Value::Int(character as i64)),
        ])
    };
    let locs: Vec<Value> = spans
        .iter()
        .map(|span| {
            let ((l0, c0), (l1, c1)) = khoang(&kq.text, *span);
            m(vec![
                ("uri", s(uri)),
                (
                    "range",
                    m(vec![("start", pos(l0, c0)), ("end", pos(l1, c1))]),
                ),
            ])
        })
        .collect();
    phan_hoi(id, Value::list(locs))
}
