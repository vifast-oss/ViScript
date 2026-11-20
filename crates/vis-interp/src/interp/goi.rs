// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-63 (VD-105 (6))**: phần **gọi hàm/closure** của interp — tách khỏi `interp.rs` cho luật
//! **R1** (≤1.500 dòng/tệp). Chuyển **nguyên văn**, không đổi một dòng ngữ nghĩa nào.
//!
//! Đây là **mô-đun con** của `interp` nên vẫn thấy `Interp`/`Closure`/`HFn` + `MAX_CALL_DEPTH`
//! như trước; `interp.rs` chỉ còn phần câu lệnh/biểu thức.

use vis_syntax::Span;
use vis_types::{hir::*, ty::Type};

use crate::interp::{builtin_name, Abort, Args, Interp, Trap, MAX_CALL_DEPTH, R, RT};
use crate::value::*;

impl<'a> Interp<'a> {
    // ---------------------------------------------------------------------
    // Gọi hàm
    // ---------------------------------------------------------------------

    pub(crate) fn eval_call(
        &mut self,
        callee: &HExpr,
        args: &[HExpr],
        named: &[(String, HExpr)],
        span: Span,
    ) -> R<Value> {
        // 1) Phương thức trên list/str/map: `xs.push(1)`, `s.upper()`.
        if let HExpr::Field {
            base,
            name,
            optional: false,
            ..
        } = callee
        {
            let recv = self.eval(base)?;
            // **ST-61 (VD-96)**: kênh cũng đi **một** nhánh `builtin::method` (như list/str/map) ⇒
            // interp ↔ VM cùng một hiện thực `send`/`recv`/`close`.
            if matches!(
                recv,
                Value::List(_) | Value::Str(_) | Value::Map(_) | Value::Chan(_)
            ) {
                // **VD-258**: `xs.push(v)` với `xs: list[T?]` — nâng `v` plain ở biên phương
                // thức (mirror codegen), tránh list chứa giá trị plain trong ô `T?`.
                let push_depth = if name == "push" {
                    match self.expr_ty(base, &recv) {
                        Some(vis_types::Type::List(el)) if el.is_opt() => opt_depth_ty(&el),
                        _ => 0,
                    }
                } else {
                    0
                };
                let mut vs = Vec::with_capacity(args.len());
                for a in args {
                    let mut vv = self.eval(a)?;
                    if push_depth > 0 {
                        vv = crate::lift::lift_opt(vv, push_depth);
                    }
                    vs.push(vv);
                }
                let kq = crate::builtin::method(self, recv, name, vs, span)?;
                // ST-15b (mục 1, §7.1 ngữ nghĩa giá trị + CoW): `push` trên **`var`** phải **ghi lại**
                // biến (bản thân list đổi), không chỉ trả list mới. `push` trên `let`/biểu thức tạm
                // vẫn "trả list mới" (không có lvalue để ghi) — ghi rõ trong JOURNAL.
                if name == "push" {
                    if let HExpr::Var { sym: Some(id), .. } = &**base {
                        if self.mutable.contains(id) {
                            self.env.borrow_mut().insert(*id, kq.clone());
                        }
                    }
                }
                return Ok(kq);
            }
        }
        // 2) Gọi thẳng hàm khai trong tệp (kể cả đệ quy) — theo `sym`.
        if let HExpr::Var { sym: Some(id), .. } = callee {
            if let Some(f) = self.fns.get(id).copied() {
                let (pos, nm) = self.eval_args(args, named)?;
                // **ST-60 (VD-90)**: gọi `async fn` **không** chạy thân — trả `Task[T]` (cùng một
                // đường với codegen: `$tac_vu` + handle, ở đây handle = `sym`).
                if f.is_async {
                    let t = self.sched.get_mut().tao(f.sym, pos, f.name.clone(), span);
                    return Ok(Value::Task(t));
                }
                return self.call_hfn(f, pos, nm, span);
            }
        }
        // 3) Hàm dựng sẵn.
        if let HExpr::Var {
            name, sym: None, ..
        } = callee
        {
            if builtin_name(name).is_some() {
                let (pos, _nm) = self.eval_args(args, named)?;
                return crate::builtin::call(self, name, pos, span);
            }
        }
        // 4) Còn lại: giá trị hàm (closure/lambda/builtin value).
        let f = self.eval(callee)?;
        let (pos, nm) = self.eval_args(args, named)?;
        self.call_value(f, pos, nm, span)
    }

    fn eval_args(&mut self, args: &[HExpr], named: &[(String, HExpr)]) -> R<Args> {
        let mut pos = Vec::with_capacity(args.len());
        for a in args {
            pos.push(self.eval(a)?);
        }
        let mut nm = Vec::with_capacity(named.len());
        for (n, a) in named {
            nm.push((n.clone(), self.eval(a)?));
        }
        Ok((pos, nm))
    }

    pub(crate) fn call_value(
        &mut self,
        f: Value,
        pos: Vec<Value>,
        named: Vec<(String, Value)>,
        span: Span,
    ) -> R<Value> {
        match f {
            Value::Fn(c) => self.call_closure(&c, pos, span),
            Value::Builtin(name) => crate::builtin::call(self, name, pos, span),
            Value::Record(name, fields) if fields.is_empty() => {
                // `User(name: "An", age: 20)` — khởi tạo bản ghi theo tên kiểu.
                let _ = pos;
                Ok(Value::Record(name, named))
            }
            other => {
                let _ = named;
                Err(Abort::Trap(Trap::new(
                    RT,
                    format!("`{}` không phải hàm", other.show()),
                    span,
                )))
            }
        }
    }

    pub(crate) fn call_closure(&mut self, c: &Closure, pos: Vec<Value>, span: Span) -> R<Value> {
        self.depth += 1;
        if self.depth > MAX_CALL_DEPTH {
            self.depth -= 1;
            return Err(Abort::Trap(Trap::new(
                RT,
                format!(
                    "gọi hàm quá sâu (>{} khung) — đệ quy không có điểm dừng?",
                    MAX_CALL_DEPTH
                ),
                span,
            )));
        }
        self.fn_depth += 1;
        let kq = self.call_closure_trong(c, pos, span);
        self.fn_depth -= 1;
        self.depth -= 1;
        kq
    }

    fn call_closure_trong(&mut self, c: &Closure, pos: Vec<Value>, span: Span) -> R<Value> {
        if pos.len() != c.params.len() {
            return Err(Abort::Trap(Trap::new(
                RT,
                format!(
                    "hàm `{}` cần {} đối số, được {}",
                    if c.name.is_empty() { "<fn>" } else { &c.name },
                    c.params.len(),
                    pos.len()
                ),
                span,
            )));
        }
        let saved_env = self.env.clone();
        self.env = c.env.clone();
        let mut old = Vec::with_capacity(c.params.len());
        for (i, p) in c.params.iter().enumerate() {
            old.push((*p, self.env.borrow().get(p).cloned()));
            self.env.borrow_mut().insert(*p, pos[i].clone());
        }
        let r = match &c.body {
            HBody::Expr(e) => self.eval(e),
            HBody::Stmts(st) => self.exec_body(st),
        };
        for (p, v) in old {
            match v {
                Some(v) => {
                    self.env.borrow_mut().insert(p, v);
                }
                None => {
                    self.env.borrow_mut().remove(&p);
                }
            }
        }
        self.env = saved_env;
        match r {
            Ok(v) => {
                if c.ret_depth > 0 {
                    Ok(opt_lift_maybe(v, c.ret_depth))
                } else {
                    Ok(v)
                }
            }
            Err(Abort::Ret(v)) => {
                if c.ret_depth > 0 {
                    Ok(opt_lift_maybe(v, c.ret_depth))
                } else {
                    Ok(v)
                }
            }
            Err(t) => Err(t),
        }
    }

    pub(crate) fn call_hfn(
        &mut self,
        f: &'a HFn,
        pos: Vec<Value>,
        named: Vec<(String, Value)>,
        span: Span,
    ) -> R<Value> {
        self.depth += 1;
        if self.depth > MAX_CALL_DEPTH {
            self.depth -= 1;
            return Err(Abort::Trap(Trap::new(
                RT,
                format!(
                    "gọi hàm quá sâu (>{} khung) — đệ quy không có điểm dừng?",
                    MAX_CALL_DEPTH
                ),
                span,
            )));
        }
        self.fn_depth += 1;
        let kq = self.call_hfn_trong(f, pos, named, span);
        self.fn_depth -= 1;
        self.depth -= 1;
        kq
    }

    fn call_hfn_trong(
        &mut self,
        f: &'a HFn,
        pos: Vec<Value>,
        named: Vec<(String, Value)>,
        span: Span,
    ) -> R<Value> {
        // ST-27: hàm có `...rest` nhận **thêm** đối số vị trí.
        let bien_di = f.params.last().map(|p| p.variadic).unwrap_or(false);
        if !bien_di && pos.len() + named.len() > f.params.len() {
            return Err(Abort::Trap(Trap::new(
                RT,
                format!(
                    "hàm `{}` nhận {} đối số, được {}",
                    f.name,
                    f.params.len(),
                    pos.len() + named.len()
                ),
                span,
            )));
        }
        let mut old = Vec::with_capacity(f.params.len());
        for (i, p) in f.params.iter().enumerate() {
            old.push((p.sym, self.env.borrow().get(&p.sym).cloned()));
            // ST-27: `...rest` gom **mọi** đối số vị trí còn lại thành `list[str]`.
            if p.variadic {
                let con: Vec<Value> = pos.iter().skip(i).cloned().collect();
                self.env.borrow_mut().insert(p.sym, Value::List(con));
                continue;
            }
            let v = if let Some(v) = pos.get(i) {
                Some(v.clone())
            } else {
                named
                    .iter()
                    .find(|(n, _)| *n == p.name)
                    .map(|(_, v)| v.clone())
            };
            let v = match v {
                Some(v) => v,
                None => match &p.default {
                    Some(d) => self.eval(d)?,
                    None => {
                        return Err(Abort::Trap(Trap::new(
                            RT,
                            format!("thiếu đối số `{}` của `{}`", p.name, f.name),
                            span,
                        )))
                    }
                },
            };
            // **VD-228b**: tham số annot `T?` — nâng giá trị plain ở biên hàm (giống `let` + `-> T?`).
            let v = match &p.ty {
                Type::Opt(_) => opt_lift_maybe(v, opt_depth_ty(&p.ty)),
                _ => v,
            };
            self.env.borrow_mut().insert(p.sym, v);
            if p.is_var_self {
                self.mutable.insert(p.sym);
            }
        }
        let r = self.exec_body(&f.body);
        for (p, v) in old {
            match v {
                Some(v) => {
                    self.env.borrow_mut().insert(p, v);
                }
                None => {
                    self.env.borrow_mut().remove(&p);
                }
            }
        }
        match r {
            Ok(v) => match &f.ret {
                Type::Opt(_) => Ok(opt_lift_maybe(v, opt_depth_ty(&f.ret))),
                _ => Ok(v),
            },
            Err(Abort::Ret(v)) => match &f.ret {
                Type::Opt(_) => Ok(opt_lift_maybe(v, opt_depth_ty(&f.ret))),
                _ => Ok(v),
            },
            Err(t) => Err(t),
        }
    }

    /// **VD-258** — đánh giá `if` biểu thức, nâng nhánh đã chọn tới `Opt` chung nếu các nhánh
    /// lệch lớp (`if c: 1 else: None` ⇒ `Some(1)`/`None`, không để `1` plain trong biến `int?`).
    pub(crate) fn eval_if(
        &mut self,
        cond: &HCond,
        then: &[HStmt],
        elifs: &[(HCond, Vec<HStmt>)],
        else_: &[HStmt],
    ) -> R<Value> {
        let (min, max) = do_if(then, elifs, Some(else_), self.symbols, self.types);
        let v = if self.eval_cond(cond)? {
            self.exec_body(then)?
        } else {
            let mut chon: Option<Value> = None;
            for (c, body) in elifs {
                if self.eval_cond(c)? {
                    chon = Some(self.exec_body(body)?);
                    break;
                }
            }
            match chon {
                Some(v) => v,
                None => self.exec_body(else_)?,
            }
        };
        Ok(boc_nhanh(v, min, max))
    }

    /// **VD-258** — đánh giá `match` biểu thức, nâng nhánh khớp tới `Opt` chung.
    pub(crate) fn eval_match(&mut self, subject: &HExpr, arms: &[HArm], span: Span) -> R<Value> {
        let (min, max) = do_match(arms, self.symbols, self.types);
        let subj = self.eval(subject)?;
        match self.pick_arm(arms, &subj)? {
            Some((arm, binds)) => {
                for (sym, v) in binds {
                    self.env.borrow_mut().insert(sym, v);
                }
                let v = self.exec_body(&arm.body)?;
                Ok(boc_nhanh(v, min, max))
            }
            None => Err(Abort::Trap(Trap::new(
                RT,
                "`match` không khớp nhánh nào (không xét hết bị lọt)",
                span,
            ))),
        }
    }

    /// **VD-258** — field của `?.` có kiểu `T?` không (theo kiểu tĩnh của base, hoặc tên bản
    /// ghi runtime khi base không annot). Quyết định `Some(None)` (field optional thật) khác
    /// `None` (field thường nhưng giá trị bất thường — giữ tương thích `lift_opt`).
    fn field_dich_la_opt(&self, base: &HExpr, name: &str, raw: &Value) -> bool {
        let root = match raw {
            Value::Enum(ea, va, ag) if ea == "Option" && va == "Some" && ag.len() == 1 => &ag[0],
            other => other,
        };
        let Some(bt) = self.expr_ty(base, root) else {
            return false;
        };
        let inner = match bt {
            Type::Opt(t) => *t,
            other => other,
        };
        let Type::Record(id) = inner else {
            return false;
        };
        self.types
            .field(id, name)
            .map(|f| f.ty.is_opt())
            .unwrap_or(false)
    }

    /// **VD-258** — đánh giá `base.name` / `base?.name`: field optional thật luôn bọc thêm một
    /// `Some` (`f: int?` + `None` ⇒ `Some(None)`); field thường dùng `lift_opt(1)` như cũ.
    pub(crate) fn eval_field(
        &mut self,
        base: &HExpr,
        name: &str,
        optional: bool,
        span: Span,
    ) -> R<Value> {
        let raw = self.eval(base)?;
        let field_la_opt = self.field_dich_la_opt(base, name, &raw);
        let (b, nang) = if optional {
            match raw {
                Value::Enum(ea, va, mut ag) if ea == "Option" => match va.as_str() {
                    "None" => return Ok(Value::none()),
                    "Some" if ag.len() == 1 => (ag.pop().expect("Some luôn có một đối số"), true),
                    _ => (Value::Enum(ea, va, ag), false),
                },
                other => (other, false),
            }
        } else {
            (raw, false)
        };
        let v = self.field_of(b, name, span)?;
        Ok(if nang {
            if field_la_opt {
                Value::some(v)
            } else {
                crate::lift::lift_opt(v, 1)
            }
        } else {
            v
        })
    }

    /// **VD-258** — nâng giá trị vào trường `T?` của bản ghi theo `RecId` đã biết.
    pub(crate) fn lift_truong_ban_ghi(
        &self,
        rec_id: Option<vis_types::ty::RecId>,
        name: &str,
        v: Value,
    ) -> Value {
        if let Some(id) = rec_id {
            if let Some(f) = self.types.field(id, name) {
                if f.ty.is_opt() {
                    return crate::lift::lift_opt(v, opt_depth_ty(&f.ty));
                }
            }
        }
        v
    }

    /// **VD-258** — nâng đối số vào trường `T?` của biến thể enum.
    pub(crate) fn lift_doi_so_bien_the(
        &self,
        enum_id: Option<vis_types::ty::EnumId>,
        variant: &str,
        i: usize,
        v: Value,
    ) -> Value {
        let ft = enum_id
            .and_then(|id| self.types.variant(id, variant))
            .and_then(|vd| vd.fields.get(i))
            .map(|f| f.ty.clone());
        match ft {
            Some(t) if t.is_opt() => crate::lift::lift_opt(v, opt_depth_ty(&t)),
            _ => v,
        }
    }
}

/// **VD-241 §3 (A0, 2026-11-19)** — tự nâng `T → T? → T??…` tại các biên hàm/annot (VD-227/228).
/// Đích `Opt(_)` bọc giá trị tới **ít nhất** đủ lớp của annot. Lõi được giữ đúng (giá trị đã là
/// `Opt`/`Result` thuộc đúng lớp đó không bị bọc thêm). (Checker từ chối từ nguồn lúc < `annot−1`
/// lớp — nên vòng lặp chỉ là kỹ thuật, không còn lớp > 1/chuyển đổi cho các chương trình hợp lệ.)
pub(in crate::interp) fn opt_lift_maybe(v: Value, depth_target: usize) -> Value {
    crate::lift::lift_opt(v, depth_target)
}

/// Số lớp `Opt` của một **đích kiểu** (đếm từ ngoài vào).
pub(in crate::interp) fn opt_depth_ty(t: &vis_types::Type) -> usize {
    let mut d = 0;
    let mut cur = t;
    while let vis_types::Type::Opt(inner) = cur {
        d += 1;
        cur = inner;
    }
    d
}

/// **VD-258** — số lớp `Option` của biểu thức nếu biết **theo cú pháp** (`None`, `Some(…)`,
/// `if`/`match` lồng); dùng để nâng nhánh `if`/`match` lệch lớp. Không đoán kiểu biến.
fn do_bieu_thuc(
    e: &HExpr,
    syms: &vis_types::lower::Symbols,
    types: &vis_types::TypeTable,
) -> usize {
    match e {
        HExpr::Opt { ty, .. } => opt_depth_ty(ty),
        HExpr::Var { name, .. } if name == "None" => 1,
        HExpr::Var { sym: Some(id), .. } => syms
            .get(*id)
            .and_then(|s| s.ty.as_ref())
            .map(opt_depth_ty)
            .unwrap_or(0),
        HExpr::Field {
            base,
            name,
            optional: true,
            ..
        } => {
            if let HExpr::Var { sym: Some(id), .. } = &**base {
                if let Some(vis_types::Type::Opt(inner)) = syms.get(*id).and_then(|s| s.ty.clone())
                {
                    if let vis_types::Type::Record(rid) = &*inner {
                        if let Some(f) = types.field(*rid, name) {
                            return 1 + opt_depth_ty(&f.ty);
                        }
                    }
                }
            }
            1
        }
        HExpr::Try { expr, .. } if matches!(&**expr, HExpr::Index { .. }) => 1,
        HExpr::Call { callee, args, .. } => match &**callee {
            HExpr::Var { name, .. } if name == "Some" => {
                1 + args
                    .first()
                    .map(|a| do_bieu_thuc(a, syms, types))
                    .unwrap_or(0)
            }
            HExpr::Var { name, .. } if name == "None" => 1,
            _ => 0,
        },
        HExpr::If {
            then, elifs, else_, ..
        } => do_if(then, elifs, Some(else_), syms, types).1,
        HExpr::Match { arms, .. } => do_match(arms, syms, types).1,
        _ => 0,
    }
}

/// **VD-258** — số lớp `Option` của giá trị cuối một khối; `None` = khối không cho giá trị
/// (kết thúc `return`/`break`/`continue`).
fn do_than(
    stmts: &[HStmt],
    syms: &vis_types::lower::Symbols,
    types: &vis_types::TypeTable,
) -> Option<usize> {
    match stmts.last()? {
        HStmt::Expr(e) => Some(do_bieu_thuc(e, syms, types)),
        HStmt::If {
            then, elifs, else_, ..
        } => Some(do_if(then, elifs, else_.as_deref(), syms, types).1),
        HStmt::Match { arms, .. } => Some(do_match(arms, syms, types).1),
        HStmt::Block { body, .. } | HStmt::Scope { body, .. } => do_than(body, syms, types),
        HStmt::Return { .. } | HStmt::Break { .. } | HStmt::Continue { .. } => None,
        _ => Some(0),
    }
}

/// **VD-258** — `(min, max)` lớp `Option` của các nhánh `if`; không `else` ⇒ `(0, 0)`.
pub(crate) fn do_if(
    then: &[HStmt],
    elifs: &[(HCond, Vec<HStmt>)],
    else_: Option<&[HStmt]>,
    syms: &vis_types::lower::Symbols,
    types: &vis_types::TypeTable,
) -> (usize, usize) {
    let Some(else_) = else_ else {
        return (0, 0);
    };
    let mut ds: Vec<usize> = Vec::new();
    for body in std::iter::once(then)
        .chain(elifs.iter().map(|(_, b)| b.as_slice()))
        .chain(std::iter::once(else_))
    {
        if let Some(d) = do_than(body, syms, types) {
            ds.push(d);
        }
    }
    if ds.is_empty() {
        return (0, 0);
    }
    let min = *ds.iter().min().unwrap_or(&0);
    let max = *ds.iter().max().unwrap_or(&0);
    (min, max)
}

/// **VD-258** — `(min, max)` lớp `Option` của các nhánh `match`.
pub(crate) fn do_match(
    arms: &[HArm],
    syms: &vis_types::lower::Symbols,
    types: &vis_types::TypeTable,
) -> (usize, usize) {
    let ds: Vec<usize> = arms
        .iter()
        .filter_map(|a| do_than(&a.body, syms, types))
        .collect();
    if ds.is_empty() {
        return (0, 0);
    }
    let min = *ds.iter().min().unwrap_or(&0);
    let max = *ds.iter().max().unwrap_or(&0);
    (min, max)
}

/// **VD-258** — nâng giá trị nhánh đã chọn tới `max` khi các nhánh lệch lớp.
pub(crate) fn boc_nhanh(v: Value, min: usize, max: usize) -> Value {
    if max > min {
        crate::lift::lift_opt(v, max)
    } else {
        v
    }
}
