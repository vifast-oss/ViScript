// SPDX-License-Identifier: MIT OR Apache-2.0
//! Thân hàm: `HBody::Expr` ⇒ biểu thức; `HBody::Stmts` ⇒ câu lệnh (giá trị cuối giữ lại).
//! Tách khỏi `codegen.rs` để giữ **R1** ≤ 1.500 dòng. **VD-228**: `-> T?` ⇒ tự nâng `Some(…)`
//! ở biên hàm (mirror interp ở `call_hfn_trong`).

use super::*;
use crate::fee::FeeKind;
use vis_types::Type;

/// **VD-258** — phân loại sự kiện phí của một call-site (một nguồn cho codegen):
/// `Field` (host method) là `E_BUILTIN`; `Var` builtin không phải hàm tệp cũng vậy.
/// Trả thêm `callee_la_gia_tri` để codegen charge `E_EXPR` đúng một lần khi callee
/// thực sự được đánh giá như giá trị (biến/closure/builtin có `sym`), không charge
/// cho hàm tệp/method/`std.*` gọi thẳng.
pub(super) fn call_fee(g: &Gen, callee: &HExpr) -> (FeeKind, bool) {
    let la_host = match callee {
        HExpr::Field { .. } => true,
        HExpr::Var { name, .. } => !g.fmap_ten.contains_key(name.as_str()) && la_builtin(name),
        _ => false,
    };
    let callee_la_gia_tri = match callee {
        HExpr::Var { sym, .. } => match sym {
            Some(id) => !g.fmap.contains_key(id),
            None => !la_host,
        },
        _ => false,
    };
    (
        if la_host {
            FeeKind::Builtin
        } else {
            FeeKind::Call
        },
        callee_la_gia_tri,
    )
}

/// **ST-248 (VD-242 §3.3)** — gom kiểu trường bản ghi `RecId → (tên trường → kiểu)` từ HIR.
/// Dùng để biết kiểu đích của `u.f = v` mà không cần `TypeTable` (API `compile` giữ nguyên).
///
/// **VD-263**: khi có [`vis_types::TypeTable`] (`compile_voi_bang`), bảng này là **nguồn đầy đủ** — gồm cả
/// các `RecId` **instantiation** (`Box[int?]`) với trường đã thay `T`; HIR chỉ có bản khai báo.
pub(super) fn collect_rec_fields(
    hir: &Hir,
    types: Option<&vis_types::TypeTable>,
) -> HashMap<(u32, String), Type> {
    let mut m = HashMap::new();
    for it in &hir.items {
        if let HItem::Rec(r) = it {
            for f in &r.fields {
                m.insert((r.id.0, f.name.clone()), f.ty.clone());
            }
        }
    }
    if let Some(types) = types {
        for (i, r) in types.recs().iter().enumerate() {
            for f in &r.fields {
                m.insert((i as u32, f.name.clone()), f.ty.clone());
            }
        }
    }
    m
}

/// **VD-258** — gom kiểu trường biến thể enum `(EnumId, tên biến thể, vị trí) → kiểu`.
/// Dùng để nâng `T → T?` ở đối số dựng enum (interp đọc `TypeTable` trực tiếp).
///
/// **VD-263**: như [`collect_rec_fields`], có `TypeTable` ⇒ gồm cả enum **instantiation**.
pub(super) fn collect_enum_fields(
    hir: &Hir,
    types: Option<&vis_types::TypeTable>,
) -> HashMap<(u32, String, usize), Type> {
    let mut m = HashMap::new();
    for it in &hir.items {
        if let HItem::Enum(e) = it {
            for v in &e.variants {
                for (i, f) in v.fields.iter().enumerate() {
                    m.insert((e.id.0, v.name.clone(), i), f.ty.clone());
                }
            }
        }
    }
    if let Some(types) = types {
        for (i, e) in types.enums().iter().enumerate() {
            for v in &e.variants {
                for (j, f) in v.fields.iter().enumerate() {
                    m.insert((i as u32, v.name.clone(), j), f.ty.clone());
                }
            }
        }
    }
    m
}

fn la_error(t: &Type) -> bool {
    matches!(t, Type::Error)
}

/// **VD-260** — `sym` hàm thường/phương thức `impl` → kiểu trả về. `HExpr::Call` chỉ có `sym`
/// của callee nên codegen cần bảng này để suy lớp Option của lời gọi (`f() -> int?`).
pub(super) fn collect_fn_rets(hir: &Hir) -> HashMap<u32, Type> {
    let mut m = HashMap::new();
    for it in &hir.items {
        match it {
            HItem::Fn(f) => {
                m.insert(f.sym, f.ret.clone());
            }
            HItem::Impl(i) => {
                for meth in &i.methods {
                    m.insert(meth.sym, meth.ret.clone());
                }
            }
            _ => {}
        }
    }
    m
}

/// **ST-248 / VD-260** — suy kiểu tĩnh của một biểu thức HIR; `None` khi không biết ⇒ không
/// nâng (an toàn, không đoán ngữ nghĩa mới). `fn_rets` cho lời gọi hàm, `rec_fields` cho field.
fn infer_ty(
    e: &HExpr,
    sym: &HashMap<u32, Type>,
    fn_rets: &HashMap<u32, Type>,
    rec_fields: &HashMap<(u32, String), Type>,
) -> Option<Type> {
    match e {
        HExpr::Lit { ty, .. }
        | HExpr::Record { ty, .. }
        | HExpr::Opt { ty, .. }
        | HExpr::Tuple { ty, .. }
        | HExpr::EnumLit { ty, .. } => Some(ty.clone()),
        // **VD-260**: list/map literal chưa có kiểu phần tử (lowering để `Error` khi phần tử
        // là `Some(…)`/`None`/lời gọi) — suy từ **phần tử đầu tiên biết kiểu**. Nhờ vậy
        // `var xs = [Some(1)]` có `xs: list[int?]` và `xs[0] = 5` nâng đúng một lớp.
        HExpr::List { items, elem, .. } | HExpr::Set { items, elem, .. } => {
            let el = if la_error(elem) {
                items
                    .iter()
                    .find_map(|i| infer_ty(i, sym, fn_rets, rec_fields))
                    .unwrap_or(Type::Error)
            } else {
                elem.clone()
            };
            Some(Type::List(Box::new(el)))
        }
        HExpr::Map {
            entries,
            key,
            value,
            ..
        } => {
            let k = if la_error(key) {
                entries
                    .iter()
                    .find_map(|(k, _)| infer_ty(k, sym, fn_rets, rec_fields))
                    .unwrap_or(Type::Error)
            } else {
                key.clone()
            };
            let v = if la_error(value) {
                entries
                    .iter()
                    .find_map(|(_, v)| infer_ty(v, sym, fn_rets, rec_fields))
                    .unwrap_or(Type::Error)
            } else {
                value.clone()
            };
            Some(Type::Map(Box::new(k), Box::new(v)))
        }
        HExpr::Var {
            name, sym: hsym, ..
        } => {
            if name == "None" {
                Some(Type::Opt(Box::new(Type::Error)))
            } else {
                hsym.as_ref().and_then(|id| sym.get(id)).cloned()
            }
        }
        HExpr::Field {
            base,
            name,
            optional,
            ..
        } => {
            let bt = infer_ty(base, sym, fn_rets, rec_fields)?;
            let inner = match bt {
                Type::Opt(t) => *t,
                other => other,
            };
            let Type::Record(id) = inner else {
                return None;
            };
            let ft = rec_fields.get(&(id.0, name.clone()))?.clone();
            Some(if *optional {
                Type::Opt(Box::new(ft))
            } else {
                ft
            })
        }
        HExpr::Index { base, .. } => match infer_ty(base, sym, fn_rets, rec_fields)? {
            Type::Map(_, v) => Some(*v),
            Type::List(el) => Some(*el),
            _ => None,
        },
        HExpr::Call { callee, args, .. } => match (&**callee, args.first()) {
            (HExpr::Var { name, .. }, Some(a)) if name == "Some" => {
                infer_ty(a, sym, fn_rets, rec_fields).map(|t| Type::Opt(Box::new(t)))
            }
            (HExpr::Var { name, .. }, _) if name == "None" => {
                Some(Type::Opt(Box::new(Type::Error)))
            }
            // Hàm **tệp** (kể cả trùng tên builtin) — ưu tiên chữ ký khai báo.
            (HExpr::Var { sym: Some(id), .. }, _) if fn_rets.contains_key(id) => {
                fn_rets.get(id).cloned()
            }
            // **VD-260**: free builtin `first(xs) -> T?` (mirror `cmdsig`/tyck).
            (HExpr::Var { name, .. }, Some(a)) if name == "first" => {
                infer_ty(a, sym, fn_rets, rec_fields).map(|t| match t {
                    Type::List(e) => Type::Opt(e),
                    _ => Type::Opt(Box::new(Type::Error)),
                })
            }
            // **VD-260**: method dựng sẵn trả `Option` — `list.get/first/last/find`,
            // `map.get` (mirror tyck `method_ret`). Nhánh `if` gọi các method này cũng nâng.
            (HExpr::Field { base, name, .. }, _) => {
                match (infer_ty(base, sym, fn_rets, rec_fields)?, name.as_str()) {
                    (Type::List(e), "get" | "first" | "last" | "find") => Some(Type::Opt(e)),
                    (Type::Map(_, v), "get") => Some(Type::Opt(v)),
                    _ => None,
                }
            }
            _ => None,
        },
        HExpr::With { base, .. } => infer_ty(base, sym, fn_rets, rec_fields),
        // **VD-263**: `expr?` — `T?` ⇒ `T`, `Result[T, E]` ⇒ `T`. Gốc `with`/field có thể là
        // giá trị đã bóc `?` (vd `maybe()? with f: 5`); thiếu nhánh này thì `base_ty` mù.
        HExpr::Try { expr, .. } => match infer_ty(expr, sym, fn_rets, rec_fields)? {
            Type::Opt(t) | Type::Result(t, _) => Some(*t),
            _ => None,
        },
        // Nhánh `if`/`match`: chọn kiểu có **nhiều lớp Option nhất** (join theo kiểu đích).
        HExpr::If {
            then, elifs, else_, ..
        } => infer_if(then, elifs, Some(else_), sym, fn_rets, rec_fields),
        HExpr::Match { arms, .. } => infer_match(arms, sym, fn_rets, rec_fields),
        _ => None,
    }
}

/// **ST-248** — gán kiểu suy được từ pattern (`let`, `for`, arm `match`, `if let`); đệ quy theo
/// hình dạng pattern để biến bind trong arm cũng có kiểu tĩnh.
///
/// **VD-263**: trước đây chỉ `HPat::Bind` trần của `let`/`for` được gán; biến bind trong arm
/// `match` (`Some(u)`, `[u]`, `{f}`, ctor enum) không có kiểu ⇒ `base_ty` mù và `u with f: 5`
/// không nâng trên VM (interp vẫn đúng nhờ tra tên bản ghi lúc chạy).
fn bind_pat_ty(
    p: &HPat,
    ty: &Type,
    sym: &mut HashMap<u32, Type>,
    rec_fields: &HashMap<(u32, String), Type>,
    enum_fields: &HashMap<(u32, String, usize), Type>,
) {
    match p {
        HPat::Bind { sym: id, .. } => {
            sym.insert(*id, ty.clone());
        }
        HPat::Or { alts, .. } => {
            for a in alts {
                bind_pat_ty(a, ty, sym, rec_fields, enum_fields);
            }
        }
        HPat::List { items, rest, .. } => {
            let elem = match ty {
                Type::List(el) => (**el).clone(),
                _ => Type::Error,
            };
            for it in items {
                bind_pat_ty(it, &elem, sym, rec_fields, enum_fields);
            }
            if let Some(r) = rest {
                bind_pat_ty(r, &Type::List(Box::new(elem)), sym, rec_fields, enum_fields);
            }
        }
        HPat::Tuple { items, .. } => {
            let args: &[Type] = match ty {
                Type::TypeRef { name, args } if name == "tuple" => args,
                _ => &[],
            };
            for (i, it) in items.iter().enumerate() {
                let t = args.get(i).unwrap_or(&Type::Error);
                bind_pat_ty(it, t, sym, rec_fields, enum_fields);
            }
        }
        HPat::Record { fields, .. } => {
            let inner = match ty {
                Type::Opt(t) => &**t,
                other => other,
            };
            let id = match inner {
                Type::Record(id) => Some(*id),
                _ => None,
            };
            for (name, sub) in fields {
                let Some(sub) = sub else { continue };
                let ft = id
                    .and_then(|id| rec_fields.get(&(id.0, name.clone())))
                    .cloned()
                    .unwrap_or(Type::Error);
                bind_pat_ty(sub, &ft, sym, rec_fields, enum_fields);
            }
        }
        HPat::Ctor { path, args, .. } => {
            let ten = path.last().map(String::as_str).unwrap_or("");
            for (i, a) in args.iter().enumerate() {
                let ft = ctor_arg_ty(ten, i, ty, enum_fields).unwrap_or(Type::Error);
                bind_pat_ty(a, &ft, sym, rec_fields, enum_fields);
            }
        }
        HPat::Wild { .. } | HPat::Lit { .. } | HPat::Range { .. } | HPat::Rest { .. } => {}
    }
}

/// **VD-263** — kiểu đối số thứ `i` của ctor `ten` khi chủ thể có kiểu `subj` (Option/Result
/// dựng sẵn + enum đã khai). `None` ⇒ không suy được.
fn ctor_arg_ty(
    ten: &str,
    i: usize,
    subj: &Type,
    enum_fields: &HashMap<(u32, String, usize), Type>,
) -> Option<Type> {
    match (ten, subj) {
        ("Some", Type::Opt(t)) if i == 0 => Some((**t).clone()),
        ("Ok", Type::Result(t, _)) if i == 0 => Some((**t).clone()),
        ("Err", Type::Result(_, e)) if i == 0 => Some((**e).clone()),
        (_, Type::Enum(id)) => enum_fields.get(&(id.0, ten.to_string(), i)).cloned(),
        _ => None,
    }
}

/// **VD-260** — kiểu của giá trị cuối một khối (`None` = khối không cho giá trị).
fn infer_than(
    sts: &[HStmt],
    sym: &HashMap<u32, Type>,
    fn_rets: &HashMap<u32, Type>,
    rec_fields: &HashMap<(u32, String), Type>,
) -> Option<Type> {
    match sts.last()? {
        HStmt::Expr(e) => infer_ty(e, sym, fn_rets, rec_fields),
        HStmt::If {
            then, elifs, else_, ..
        } => infer_if(then, elifs, else_.as_deref(), sym, fn_rets, rec_fields),
        HStmt::Match { arms, .. } => infer_match(arms, sym, fn_rets, rec_fields),
        HStmt::Block { body, .. } | HStmt::Scope { body, .. } => {
            infer_than(body, sym, fn_rets, rec_fields)
        }
        HStmt::Return { .. } | HStmt::Break { .. } | HStmt::Continue { .. } => None,
        _ => None,
    }
}

/// **VD-260** — chọn kiểu nhánh có **nhiều lớp Option nhất** (hoà ⇒ nhánh sau; chỉ độ sâu
/// được dùng). Checker đã chấp nhận chương trình nên không cần hợp nhất đầy đủ ở đây.
fn hop_nhanh(ds: Vec<Type>) -> Option<Type> {
    ds.into_iter().max_by_key(opt_depth)
}

fn infer_if(
    then: &[HStmt],
    elifs: &[(HCond, Vec<HStmt>)],
    else_: Option<&[HStmt]>,
    sym: &HashMap<u32, Type>,
    fn_rets: &HashMap<u32, Type>,
    rec_fields: &HashMap<(u32, String), Type>,
) -> Option<Type> {
    let else_ = else_?;
    let mut ds = Vec::new();
    for body in std::iter::once(then)
        .chain(elifs.iter().map(|(_, b)| b.as_slice()))
        .chain(std::iter::once(else_))
    {
        if let Some(t) = infer_than(body, sym, fn_rets, rec_fields) {
            ds.push(t);
        }
    }
    hop_nhanh(ds)
}

fn infer_match(
    arms: &[vis_types::hir::HArm],
    sym: &HashMap<u32, Type>,
    fn_rets: &HashMap<u32, Type>,
    rec_fields: &HashMap<(u32, String), Type>,
) -> Option<Type> {
    let ds: Vec<Type> = arms
        .iter()
        .filter_map(|a| infer_than(&a.body, sym, fn_rets, rec_fields))
        .collect();
    hop_nhanh(ds)
}

/// **VD-263** — bind biến của `if let` / `elif let` theo kiểu giá trị vế phải (nếu biết).
fn bind_cond_let(
    c: &HCond,
    sym: &mut HashMap<u32, Type>,
    fn_rets: &HashMap<u32, Type>,
    rec_fields: &HashMap<(u32, String), Type>,
    enum_fields: &HashMap<(u32, String, usize), Type>,
) {
    if let HCond::Let { pat, value, .. } = c {
        if let Some(t) = infer_ty(value, sym, fn_rets, rec_fields) {
            bind_pat_ty(pat, &t, sym, rec_fields, enum_fields);
        }
    }
}

/// **ST-248 / VD-260** — đi khối câu lệnh để gom kiểu biến không có annotation (vd
/// `var u = U(…)`/`var u = pick(true)` ⇒ `u: Record(U)`) và biến bind trong pattern
/// (`match`/`if let`/`for`). Chỉ đọc cấu trúc HIR, không gọi mã người dùng (VD-242 §3.4).
fn walk_stmts(
    sts: &[HStmt],
    sym: &mut HashMap<u32, Type>,
    fn_rets: &HashMap<u32, Type>,
    rec_fields: &HashMap<(u32, String), Type>,
    enum_fields: &HashMap<(u32, String, usize), Type>,
) {
    for s in sts {
        match s {
            HStmt::Var {
                sym: id, ty, init, ..
            } => {
                // **VD-260**: annotation `Error` (không có/không phân giải) vẫn cho phép suy
                // từ initializer — trước đây chỉ `ty.is_none()` nên `var` thiếu kiểu bị bỏ.
                let t = ty
                    .clone()
                    .filter(|t| !la_error(t))
                    .or_else(|| infer_ty(init, sym, fn_rets, rec_fields));
                if let Some(t) = t {
                    sym.insert(*id, t);
                }
            }
            HStmt::Let { pat, ty, init, .. } => {
                // **VD-258**: annotation của `let` (`let u: U? = …`) trước đây không vào
                // `sym_ty` (chỉ `var`/tham số có), nên `u?.f` không biết kiểu field để lift.
                let t = ty
                    .clone()
                    .filter(|t| !la_error(t))
                    .or_else(|| infer_ty(init, sym, fn_rets, rec_fields));
                if let Some(t) = t {
                    bind_pat_ty(pat, &t, sym, rec_fields, enum_fields);
                }
            }
            HStmt::If {
                cond,
                then,
                elifs,
                else_,
                ..
            } => {
                bind_cond_let(cond, sym, fn_rets, rec_fields, enum_fields);
                walk_stmts(then, sym, fn_rets, rec_fields, enum_fields);
                for (c, b) in elifs {
                    bind_cond_let(c, sym, fn_rets, rec_fields, enum_fields);
                    walk_stmts(b, sym, fn_rets, rec_fields, enum_fields);
                }
                if let Some(b) = else_ {
                    walk_stmts(b, sym, fn_rets, rec_fields, enum_fields);
                }
            }
            HStmt::For {
                pat, iter, body, ..
            } => {
                // **VD-260**: biến vòng lặp lấy kiểu phần tử của list (nếu biết).
                if let Some(t) = infer_ty(iter, sym, fn_rets, rec_fields) {
                    bind_pat_ty(pat, &t, sym, rec_fields, enum_fields);
                }
                walk_stmts(body, sym, fn_rets, rec_fields, enum_fields);
            }
            HStmt::While { body, .. }
            | HStmt::Loop { body, .. }
            | HStmt::Scope { body, .. }
            | HStmt::Block { body, .. } => walk_stmts(body, sym, fn_rets, rec_fields, enum_fields),
            HStmt::Match { subject, arms, .. } => {
                // **VD-263**: biến bind trong arm (`Some(u)`, `[u]`, `{f}`, ctor enum) lấy
                // kiểu từ chủ thể — trước đây arm chỉ được đi thân, không bind kiểu.
                let subj = infer_ty(subject, sym, fn_rets, rec_fields);
                for a in arms {
                    if let Some(t) = &subj {
                        bind_pat_ty(&a.pat, t, sym, rec_fields, enum_fields);
                    }
                    walk_stmts(&a.body, sym, fn_rets, rec_fields, enum_fields);
                }
            }
            _ => {}
        }
    }
}

/// **ST-248 / VD-260** — `sym` → kiểu đầy đủ = annotation + kiểu suy từ initializer (mirror
/// interp). `self` trong method `impl` lấy kiểu bản ghi của `impl` (kể cả khi không annot).
pub(super) fn collect_sym_tys(
    hir: &Hir,
    symbols: &Symbols,
    types: Option<&vis_types::TypeTable>,
) -> HashMap<u32, Type> {
    let mut m: HashMap<u32, Type> = crate::util::sym_annot_map(symbols)
        .into_iter()
        .filter(|(_, t)| !la_error(t))
        .collect();
    let fn_rets = collect_fn_rets(hir);
    let rec_fields = collect_rec_fields(hir, types);
    let enum_fields = collect_enum_fields(hir, types);
    // `self` của `impl U` có kiểu `U` dù tham số không annot (lowering để `Error`).
    for it in &hir.items {
        if let HItem::Impl(i) = it {
            if la_error(&i.ty) {
                continue;
            }
            for meth in &i.methods {
                for p in &meth.params {
                    if p.is_self {
                        m.insert(p.sym, i.ty.clone());
                    }
                }
            }
        }
    }
    walk_stmts(&hir.stmts, &mut m, &fn_rets, &rec_fields, &enum_fields);
    for it in &hir.items {
        match it {
            HItem::Fn(f) => walk_stmts(&f.body, &mut m, &fn_rets, &rec_fields, &enum_fields),
            HItem::Impl(i) => {
                for meth in &i.methods {
                    walk_stmts(&meth.body, &mut m, &fn_rets, &rec_fields, &enum_fields);
                }
            }
            _ => {}
        }
    }
    m
}

/// Số lớp `Opt` của một **đích kiểu** — đếm từ ngoài vào (mirror vis-interp `opt_depth_ty`).
pub(super) fn opt_depth(t: &vis_types::Type) -> usize {
    let mut d = 0;
    let mut cur = t;
    while let vis_types::Type::Opt(inner) = cur {
        d += 1;
        cur = inner;
    }
    d
}

impl Emit<'_> {
    /// **VD-241 §3 (A0, 2026-11-19)** — nâng giá trị Opt tới **≥ `depth` lớp** (hàm. nội bộ
    /// `$lift-opt`, interp/VM dùng chung; người dùng không viết được tên `$…`).
    /// Stack trước: [v] → sau: [v'] với `v'` có tối thiểu `depth` lớp `Opt` từ ngoài vào.
    pub(super) fn lift_opt(&mut self, depth: usize, span: Span) -> Result<(), CodegenError> {
        let d = self.g.cst(vis_types::LitKind::Int, &depth.to_string());
        self.f.push(Instr::Const(d), span, 1);
        let n = self.g.cst_str("$lift-opt");
        self.f.push(Instr::CallBuiltin(n, 2), span, -1);
        Ok(())
    }

    /// **VD-228** (siết theo VD-241 §3) — tự nâng `T → …T?…` ở biên hàm khi annot return là Optional.
    pub(super) fn lift_ret_opt(&mut self, span: Span) -> Result<(), CodegenError> {
        if self.ret_depth > 0 {
            self.lift_opt(self.ret_depth, span)?;
        }
        Ok(())
    }

    /// **VD-228b** — tự nâng `T → T?` ở biên **tham số** annot Optional: giá trị plain vào
    /// ô cục bộ tham số ⇒ gói `Some(…)` (mirror interp ở `call_hfn_trong`).
    pub(super) fn lift_params_opt(
        &mut self,
        params: &[vis_types::HParam],
        span: Span,
    ) -> Result<(), CodegenError> {
        for p in params {
            let depth = opt_depth(&p.ty);
            if depth > 0 {
                let slot = self
                    .f
                    .locals
                    .get(&p.sym)
                    .copied()
                    .ok_or(CodegenError::Unsupported(
                        "tham số không có ô cục bộ",
                        p.span,
                    ))?;
                self.f.push(Instr::LoadLocal(slot), span, 1);
                self.lift_opt(depth, span)?; // VD-241 §3: nâng tới đủ lớp tham số.
                self.f.push(Instr::StoreLocal(slot), span, -1);
            }
        }
        Ok(())
    }

    pub(super) fn than(&mut self, b: &HBody, _base: u16, span: Span) -> Result<(), CodegenError> {
        match b {
            HBody::Expr(e) => {
                // **VD-257 (stream B)** — HIR coi thân `Expr` là MỘT câu nguồn; interp charge
                // `E_STMT` ở đây (`interp.rs::exec_stmt`). VD-256 đã vá đường tệp/top-level nhưng
                // sót đường thân hàm (`fn f(): expr`) ⇒ parity compiler↔interp lệch đúng 1 sự kiện.
                //charge E_STMT để dãy sự kiện khớp; không đổi opcode/ngữ nghĩa thực thi.
                self.f.charge(FeeKind::Stmt, e.span());
                self.expr(e)?;
                self.lift_ret_opt(span)?; // VD-228 (siết VD-241 §3): biên hàm `-> T?`
                Ok(())
            }
            HBody::Stmts(st) => {
                // Câu lệnh cuối là biểu thức ⇒ **giữ** giá trị làm kết quả hàm (giống interp).
                self.nhanh_gia_tri(st)?;
                self.lift_ret_opt(span)?; // VD-228 (siết VD-241 §3): biên hàm `-> T?`
                self.f.push(Instr::Return, span, 0);
                self.net_stack(span)
            }
        }
    }

    /// **VD-241 §3+ (A0, 2026-11-19)** — nâng giá trị ở biên **annot `Option`** (`let`/`var`):
    /// giá trị đã có đủ lớp giữ nguyên; thiếu thì bọc `Some(…)` tới đủ lớp. `None` trần được
    /// giữ nguyên bởi `lift_opt` (bước 1 — không bọc).
    pub(super) fn lift_to_annot(
        &mut self,
        ty: Option<&vis_types::Type>,
        span: Span,
    ) -> Result<(), CodegenError> {
        if let Some(t) = ty {
            let depth = opt_depth(t);
            if depth > 0 {
                self.lift_opt(depth, span)?;
            }
        }
        Ok(())
    }

    /// **ST-248 (VD-242 §3.3)** — kiểu tĩnh của biểu thức **gốc** (`Var`/`Field`/`Index`), dùng để
    /// suy kiểu đích của phép gán trường/chỉ số. `None` ⇒ không biết ⇒ không nâng.
    ///
    /// **VD-263**: gốc không phải `Var`/`Field`/`Index` (literal bản ghi, lời gọi hàm/phương thức,
    /// `if`/`match`, `with` lồng, `expr?`) vẫn suy được bằng [`infer_ty`] — nhờ đó `with` lấy đúng
    /// `RecId` và nâng trường `T?` như interp (interp tra tên bản ghi lúc chạy; VM **không** dựa
    /// vào tên lúc chạy).
    pub(super) fn base_ty(&self, e: &vis_types::HExpr) -> Option<Type> {
        match e {
            vis_types::HExpr::Var { sym: Some(id), .. } => self.g.sym_ty.get(id).cloned(),
            vis_types::HExpr::Field { base, name, .. } => match self.base_ty(base) {
                Some(Type::Record(id)) => self.g.rec_fields.get(&(id.0, name.clone())).cloned(),
                _ => None,
            },
            vis_types::HExpr::Index { base, .. } => match self.base_ty(base) {
                Some(Type::Map(_, v)) => Some(*v),
                Some(Type::List(el)) => Some(*el),
                _ => None,
            },
            other => infer_ty(other, &self.g.sym_ty, &self.g.fn_rets, &self.g.rec_fields),
        }
    }
}

/// **VD-258** — số lớp `Option` của biểu thức nếu biết **theo cú pháp** (`None`, `Some(…)`,
/// `if`/`match` lồng); dùng để nâng nhánh `if`/`match` lệch lớp.
fn do_bieu_thuc(
    e: &HExpr,
    sym_ty: &HashMap<u32, Type>,
    rec_fields: &HashMap<(u32, String), Type>,
    fn_rets: &HashMap<u32, Type>,
) -> usize {
    match e {
        HExpr::Opt { ty, .. } => opt_depth(ty),
        HExpr::Var { name, .. } if name == "None" => 1,
        HExpr::Var { sym: Some(id), .. } => sym_ty.get(id).map(opt_depth).unwrap_or(0),
        HExpr::Field {
            base,
            name,
            optional: true,
            ..
        } => {
            if let HExpr::Var { sym: Some(id), .. } = &**base {
                if let Some(Type::Opt(inner)) = sym_ty.get(id) {
                    if let Type::Record(rid) = &**inner {
                        if let Some(ft) = rec_fields.get(&(rid.0, name.clone())) {
                            return 1 + opt_depth(ft);
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
                    .map(|a| do_bieu_thuc(a, sym_ty, rec_fields, fn_rets))
                    .unwrap_or(0)
            }
            HExpr::Var { name, .. } if name == "None" => 1,
            // **VD-260**: lời gọi hàm tệp/phương thức — kiểu trả về quyết định lớp Option.
            HExpr::Var { sym: Some(id), .. } => fn_rets.get(id).map(opt_depth).unwrap_or(0),
            // Method dựng sẵn (`list.get/first/…`, `map.get`) — dùng chung `infer_ty`.
            _ => infer_ty(e, sym_ty, fn_rets, rec_fields)
                .map(|t| opt_depth(&t))
                .unwrap_or(0),
        },
        HExpr::If {
            then, elifs, else_, ..
        } => do_if(then, elifs, Some(else_), sym_ty, rec_fields, fn_rets).1,
        HExpr::Match { arms, .. } => do_match(arms, sym_ty, rec_fields, fn_rets).1,
        // **VD-260**: các dạng còn lại (method `get`/`first`/…, index, `with`, tuple) — dùng
        // chung suy kiểu đích để nhánh plain được nâng đúng lớp.
        _ => infer_ty(e, sym_ty, fn_rets, rec_fields)
            .map(|t| opt_depth(&t))
            .unwrap_or(0),
    }
}

/// **VD-258** — số lớp `Option` của giá trị cuối một khối (`None` = khối không cho giá trị).
fn do_than(
    stmts: &[HStmt],
    sym_ty: &HashMap<u32, Type>,
    rec_fields: &HashMap<(u32, String), Type>,
    fn_rets: &HashMap<u32, Type>,
) -> Option<usize> {
    match stmts.last()? {
        HStmt::Expr(e) => Some(do_bieu_thuc(e, sym_ty, rec_fields, fn_rets)),
        HStmt::If {
            then, elifs, else_, ..
        } => Some(do_if(then, elifs, else_.as_deref(), sym_ty, rec_fields, fn_rets).1),
        HStmt::Match { arms, .. } => Some(do_match(arms, sym_ty, rec_fields, fn_rets).1),
        HStmt::Block { body, .. } | HStmt::Scope { body, .. } => {
            do_than(body, sym_ty, rec_fields, fn_rets)
        }
        HStmt::Return { .. } | HStmt::Break { .. } | HStmt::Continue { .. } => None,
        _ => Some(0),
    }
}

/// **VD-258** — `(min, max)` lớp `Option` của các nhánh `if`; không `else` ⇒ `(0, 0)`.
pub(super) fn do_if(
    then: &[HStmt],
    elifs: &[(HCond, Vec<HStmt>)],
    else_: Option<&[HStmt]>,
    sym_ty: &HashMap<u32, Type>,
    rec_fields: &HashMap<(u32, String), Type>,
    fn_rets: &HashMap<u32, Type>,
) -> (usize, usize) {
    let Some(else_) = else_ else {
        return (0, 0);
    };
    let mut ds: Vec<usize> = Vec::new();
    for body in std::iter::once(then)
        .chain(elifs.iter().map(|(_, b)| b.as_slice()))
        .chain(std::iter::once(else_))
    {
        if let Some(d) = do_than(body, sym_ty, rec_fields, fn_rets) {
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
pub(super) fn do_match(
    arms: &[vis_types::hir::HArm],
    sym_ty: &HashMap<u32, Type>,
    rec_fields: &HashMap<(u32, String), Type>,
    fn_rets: &HashMap<u32, Type>,
) -> (usize, usize) {
    let ds: Vec<usize> = arms
        .iter()
        .filter_map(|a| do_than(&a.body, sym_ty, rec_fields, fn_rets))
        .collect();
    if ds.is_empty() {
        return (0, 0);
    }
    let min = *ds.iter().min().unwrap_or(&0);
    let max = *ds.iter().max().unwrap_or(&0);
    (min, max)
}

impl Emit<'_> {
    /// **VD-258** — nâng giá trị nhánh đã chọn tới `max` khi các nhánh lệch lớp.
    pub(super) fn lift_nhanh(
        &mut self,
        min: usize,
        max: usize,
        span: Span,
    ) -> Result<(), CodegenError> {
        if max > min {
            self.lift_opt(max, span)?;
        }
        Ok(())
    }

    /// **VD-258** — field của `?.` có kiểu `T?` không. Chỉ dựa vào kiểu tĩnh đã biết
    /// (`base_ty`); field optional thật ⇒ `MakeSome` vô điều kiện, field thường ⇒ `lift_opt` cũ.
    pub(super) fn field_la_opt(&self, base: &vis_types::HExpr, name: &str) -> bool {
        let Some(bt) = self.base_ty(base) else {
            return false;
        };
        let inner = match bt {
            Type::Opt(t) => *t,
            other => other,
        };
        let Type::Record(id) = inner else {
            return false;
        };
        self.g
            .rec_fields
            .get(&(id.0, name.to_string()))
            .map(|t| t.is_opt())
            .unwrap_or(false)
    }

    /// **VD-258** — sau `Field` của `?.`: field `T?` ⇒ `MakeSome` vô điều kiện; field thường
    /// ⇒ `lift_opt(1)` như cũ (giá trị hợp lệ không-Option).
    pub(super) fn field_optional_boc(
        &mut self,
        base: &vis_types::HExpr,
        name: &str,
        span: Span,
    ) -> Result<(), CodegenError> {
        if self.field_la_opt(base, name) {
            self.f.push(Instr::MakeSome, span, 0);
        } else {
            self.lift_opt(1, span)?;
        }
        Ok(())
    }

    /// **VD-258** — số lớp `Opt` của phần tử `list` khi gọi `push` (`0` nếu không phải list/Opt).
    pub(super) fn push_elem_depth(&self, base: &vis_types::HExpr) -> usize {
        match self.base_ty(base) {
            Some(Type::List(el)) if el.is_opt() => opt_depth(&el),
            _ => 0,
        }
    }

    /// **VD-258** — nâng đối số `i` của biến thể enum tới đủ lớp `Opt` của trường khai báo.
    pub(super) fn lift_enum_arg(
        &mut self,
        enum_id: Option<vis_types::EnumId>,
        variant: &str,
        i: usize,
        span: Span,
    ) -> Result<(), CodegenError> {
        let Some(id) = enum_id else {
            return Ok(());
        };
        if let Some(ft) = self
            .g
            .enum_fields
            .get(&(id.0, variant.to_string(), i))
            .cloned()
        {
            let depth = opt_depth(&ft);
            if depth > 0 {
                self.lift_opt(depth, span)?;
            }
        }
        Ok(())
    }

    /// **VD-258** — nâng giá trị cập nhật `with` vào trường `T?` của bản ghi đã biết.
    pub(super) fn lift_with_field(
        &mut self,
        rec_id: Option<vis_types::RecId>,
        name: &str,
        span: Span,
    ) -> Result<(), CodegenError> {
        if let Some(id) = rec_id {
            if let Some(ft) = self.g.rec_fields.get(&(id.0, name.to_string())).cloned() {
                let depth = opt_depth(&ft);
                if depth > 0 {
                    self.lift_opt(depth, span)?;
                }
            }
        }
        Ok(())
    }

    /// **ST-248 (VD-242 §3.3)** — số lớp `Opt` của **đích gán** (0 ⇒ giữ nguyên).
    /// `Var` dùng kiểu biến (không đổi theo từng lần gán); `Field`/`Index` dùng kiểu khai báo
    /// của trường/phần tử. `lift_opt` tự dừng ở đủ lớp ⇒ không nâng nhiều lớp, `None` giữ nguyên.
    pub(super) fn assign_target_depth(&self, t: &vis_types::HExpr) -> usize {
        let ty = match t {
            vis_types::HExpr::Var { sym: Some(id), .. } => self.g.sym_ty.get(id).cloned(),
            other => self.base_ty(other),
        };
        ty.as_ref().map(opt_depth).unwrap_or(0)
    }

    /// **VD-241 §3+ / ST-248** — nâng giá trị ở biên gán (đích `var`, `field`, `index`) tới đủ lớp
    /// `Opt` của kiểu đích; đích không-Opt giữ nguyên.
    pub(super) fn lift_assign_set(
        &mut self,
        target: &vis_types::HExpr,
        span: Span,
    ) -> Result<(), CodegenError> {
        let depth = self.assign_target_depth(target);
        if depth > 0 {
            self.lift_opt(depth, span)?;
        }
        Ok(())
    }
}
