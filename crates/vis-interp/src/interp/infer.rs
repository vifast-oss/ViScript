// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310a (VD-285)** — suy kiểu runtime cho lời gọi/mẫu (tách khỏi `interp/goi.rs` để
//! giữ tệp dưới 1.150 dòng). Chuyển **nguyên văn**, không đổi ngữ nghĩa.

use super::*;

pub(crate) fn la_error(t: &Type) -> bool {
    matches!(t, Type::Error)
}

/// **VD-264** — số nút `Error` trong một kiểu (mirror codegen `so_loi`).
pub(crate) fn so_loi(t: &Type) -> usize {
    match t {
        Type::Error => 1,
        Type::Opt(x) | Type::List(x) => so_loi(x),
        Type::Map(k, v) | Type::Result(k, v) => so_loi(k) + so_loi(v),
        Type::TypeRef { args, .. } => args.iter().map(so_loi).sum(),
        Type::Fn(params, ret) => params.iter().map(so_loi).sum::<usize>() + so_loi(ret),
        _ => 0,
    }
}

/// **VD-264** — thay `a` bằng kiểu suy `s` khi `a` là `Error` hoặc `s` có **ít nút `Error`
/// hơn** (mirror codegen `chon_kieu`).
pub(crate) fn chon_kieu(a: &mut Type, s: Type) {
    if la_error(a) || so_loi(&s) < so_loi(a) {
        *a = s;
    }
}

/// **VD-264** — kiểu `tuple` với các phần tử `Error` được suy lại từ biểu thức tương ứng
/// (mirror codegen `than::tuple_ty_suy`). Lower dựng `ty` từ `type_of`, mà lời gọi hàm/`Some`
/// cho `Error`, nên `(Some(1), 2)` mất kiểu `int?` của phần tử đầu.
pub(crate) fn tuple_ty_suy(
    items: &[HExpr],
    ty: &Type,
    sym: &HashMap<u32, Type>,
    types: &TypeTable,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
) -> Type {
    let Type::TypeRef { name, args } = ty else {
        return ty.clone();
    };
    if name != "tuple" {
        return ty.clone();
    }
    let args = args
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let mut a = t.clone();
            if let Some(s) = items
                .get(i)
                .and_then(|it| infer_ty(it, sym, types, fn_rets, mret))
            {
                chon_kieu(&mut a, s);
            }
            a
        })
        .collect();
    Type::TypeRef {
        name: name.clone(),
        args,
    }
}

/// **VD-260** — suy kiểu tĩnh của biểu thức HIR (mirror codegen `than::infer_ty`); `None` khi
/// không biết ⇒ không nâng. `types` cho field bản ghi, `fn_rets` cho lời gọi hàm.
pub(crate) fn infer_ty(
    e: &HExpr,
    sym: &HashMap<u32, Type>,
    types: &TypeTable,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
) -> Option<Type> {
    match e {
        HExpr::Lit { ty, .. }
        | HExpr::Record { ty, .. }
        | HExpr::Opt { ty, .. }
        | HExpr::EnumLit { ty, .. } => Some(ty.clone()),
        // **VD-264**: xem `tuple_ty_suy`.
        HExpr::Tuple { items, ty, .. } => Some(tuple_ty_suy(items, ty, sym, types, fn_rets, mret)),
        // **VD-260**: list/map literal chưa có kiểu phần tử (lowering để `Error` khi phần tử
        // là `Some(…)`/`None`/lời gọi) — suy từ **phần tử đầu tiên biết kiểu**.
        // **VD-264**: cả khi kiểu phần tử đã có nhưng còn `Error` bên trong
        // (`[(Some(1), 2)]` ⇒ `tuple[Error, int]`), dùng kiểu suy được nếu **ít lỗi hơn**.
        HExpr::List { items, elem, .. } | HExpr::Set { items, elem, .. } => {
            let mut el = elem.clone();
            if let Some(t) = items
                .iter()
                .find_map(|i| infer_ty(i, sym, types, fn_rets, mret))
            {
                chon_kieu(&mut el, t);
            }
            Some(Type::List(Box::new(el)))
        }
        HExpr::Map {
            entries,
            key,
            value,
            ..
        } => {
            let mut k = key.clone();
            if let Some(t) = entries
                .iter()
                .find_map(|(k, _)| infer_ty(k, sym, types, fn_rets, mret))
            {
                chon_kieu(&mut k, t);
            }
            let mut v = value.clone();
            if let Some(t) = entries
                .iter()
                .find_map(|(_, v)| infer_ty(v, sym, types, fn_rets, mret))
            {
                chon_kieu(&mut v, t);
            }
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
            let bt = infer_ty(base, sym, types, fn_rets, mret)?;
            let inner = match bt {
                Type::Opt(t) => *t,
                other => other,
            };
            let Type::Record(id) = inner else {
                return None;
            };
            let ft = types.field(id, name)?.ty.clone();
            Some(if *optional {
                Type::Opt(Box::new(ft))
            } else {
                ft
            })
        }
        HExpr::Index { base, .. } => match infer_ty(base, sym, types, fn_rets, mret)? {
            Type::Map(_, v) => Some(*v),
            Type::List(el) => Some(*el),
            _ => None,
        },
        HExpr::Call { callee, args, .. } => match (&**callee, args.first()) {
            (HExpr::Var { name, .. }, Some(a)) if name == "Some" => {
                infer_ty(a, sym, types, fn_rets, mret).map(|t| Type::Opt(Box::new(t)))
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
                infer_ty(a, sym, types, fn_rets, mret).map(|t| match t {
                    Type::List(e) => Type::Opt(e),
                    _ => Type::Opt(Box::new(Type::Error)),
                })
            }
            // **VD-260**: method dựng sẵn trả `Option` — `list.get/first/last/find`,
            // `map.get` (mirror tyck `method_ret`). Nhánh `if` gọi các method này cũng nâng.
            //
            // **VD-265**: phương thức **người dùng** còn dạng `Field` (không dispatch tĩnh được)
            // — tra bảng [`MethodRet`] theo kiểu nhận (`Record`/`Enum`/`TypeRef`, bóc `Opt`) để
            // biết `T?`/`list[T?]`/bản ghi; nhờ vậy `?.`/`with`/gán vẫn nâng đúng lớp Option.
            // **VD-269**: callee `optional` (gọi `recv?.m()`) ⇒ kết quả lift một lớp.
            (
                HExpr::Field {
                    base,
                    name,
                    optional,
                    ..
                },
                _,
            ) => {
                let bt = infer_ty(base, sym, types, fn_rets, mret)?;
                let inner = match &bt {
                    Type::Opt(t) => &**t,
                    other => other,
                };
                let ret = match inner {
                    Type::List(e) if matches!(name.as_str(), "get" | "first" | "last" | "find") => {
                        Type::Opt(e.clone())
                    }
                    Type::Map(_, v) if name == "get" => Type::Opt(v.clone()),
                    _ => mret.ret(&bt, name, types)?,
                };
                Some(if *optional {
                    vis_types::nang_optional(ret)
                } else {
                    ret
                })
            }
            _ => None,
        },
        HExpr::With { base, .. } => infer_ty(base, sym, types, fn_rets, mret),
        // **VD-264**: `xs[i]?` = chỉ số có kiểm tra ⇒ `T?` (mirror checker `kiem_index_opt`);
        // `expr?` khác bóc một lớp `T?`/`Result[T, E]`.
        HExpr::Try { expr, .. } => {
            if let HExpr::Index { base, .. } = &**expr {
                return match infer_ty(base, sym, types, fn_rets, mret)? {
                    Type::List(el) => Some(Type::Opt(el)),
                    Type::Map(_, v) => Some(Type::Opt(v)),
                    Type::Bytes => Some(Type::Opt(Box::new(Type::Int))),
                    _ => None,
                };
            }
            match infer_ty(expr, sym, types, fn_rets, mret)? {
                Type::Opt(t) | Type::Result(t, _) => Some(*t),
                _ => None,
            }
        }

        HExpr::If {
            then, elifs, else_, ..
        } => infer_if(then, elifs, Some(else_), sym, types, fn_rets, mret),
        HExpr::Match { arms, .. } => infer_match(arms, sym, types, fn_rets, mret),
        _ => None,
    }
}

/// **VD-264** — gán kiểu đệ quy cho biến trong pattern (mirror codegen `bind_pat_ty`):
/// `Bind`, `Or`, `List`/`Rest`, `Tuple`, `Record`, `Ctor` (Option/Result + enum). Trước đây chỉ
/// `HPat::Bind` trần của `let`/`for` được gán ⇒ biến bind trong arm `match`/`if let` mất kiểu.
pub(crate) fn bind_pat_ty(p: &HPat, ty: &Type, sym: &mut HashMap<u32, Type>, types: &TypeTable) {
    match p {
        HPat::Bind { sym: id, .. } => {
            sym.insert(*id, ty.clone());
        }
        HPat::Or { alts, .. } => {
            for a in alts {
                bind_pat_ty(a, ty, sym, types);
            }
        }
        HPat::List { items, rest, .. } => {
            let elem = match ty {
                Type::List(el) => (**el).clone(),
                _ => Type::Error,
            };
            for it in items {
                bind_pat_ty(it, &elem, sym, types);
            }
            if let Some(r) = rest {
                bind_pat_ty(r, &Type::List(Box::new(elem)), sym, types);
            }
        }
        HPat::Tuple { items, .. } => {
            let args: &[Type] = match ty {
                Type::TypeRef { name, args } if name == "tuple" => args,
                _ => &[],
            };
            for (i, it) in items.iter().enumerate() {
                let t = args.get(i).unwrap_or(&Type::Error);
                bind_pat_ty(it, t, sym, types);
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
                    .and_then(|id| types.field(id, name))
                    .map(|f| f.ty.clone())
                    .unwrap_or(Type::Error);
                bind_pat_ty(sub, &ft, sym, types);
            }
        }
        HPat::Ctor { path, args, .. } => {
            let ten = path.last().map(String::as_str).unwrap_or("");
            for (i, a) in args.iter().enumerate() {
                let ft = ctor_arg_ty(ten, i, ty, types).unwrap_or(Type::Error);
                bind_pat_ty(a, &ft, sym, types);
            }
        }
        HPat::Wild { .. } | HPat::Lit { .. } | HPat::Range { .. } | HPat::Rest { .. } => {}
    }
}

/// **VD-264** — kiểu đối số thứ `i` của ctor `ten` khi chủ thể có kiểu `subj` (mirror codegen
/// `ctor_arg_ty`; interp đọc thẳng `TypeTable` thay vì bảng enum_fields).
pub(crate) fn ctor_arg_ty(ten: &str, i: usize, subj: &Type, types: &TypeTable) -> Option<Type> {
    match (ten, subj) {
        ("Some", Type::Opt(t)) if i == 0 => Some((**t).clone()),
        ("Ok", Type::Result(t, _)) if i == 0 => Some((**t).clone()),
        ("Err", Type::Result(_, e)) if i == 0 => Some((**e).clone()),
        (_, Type::Enum(id)) => types
            .variant(*id, ten)
            .and_then(|v| v.fields.get(i))
            .map(|f| f.ty.clone()),
        _ => None,
    }
}

/// **VD-260** — kiểu của giá trị cuối một khối (`None` = khối không cho giá trị).
pub(crate) fn infer_than(
    sts: &[HStmt],
    sym: &HashMap<u32, Type>,
    types: &TypeTable,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
) -> Option<Type> {
    match sts.last()? {
        HStmt::Expr(e) => infer_ty(e, sym, types, fn_rets, mret),
        HStmt::If {
            then, elifs, else_, ..
        } => infer_if(then, elifs, else_.as_deref(), sym, types, fn_rets, mret),
        HStmt::Match { arms, .. } => infer_match(arms, sym, types, fn_rets, mret),
        HStmt::Block { body, .. } | HStmt::Scope { body, .. } => {
            infer_than(body, sym, types, fn_rets, mret)
        }
        HStmt::Return { .. } | HStmt::Break { .. } | HStmt::Continue { .. } => None,
        _ => None,
    }
}

/// **VD-260** — chọn kiểu nhánh có **nhiều lớp Option nhất** (join theo kiểu đích).
pub(crate) fn hop_nhanh(ds: Vec<Type>) -> Option<Type> {
    // **VD-269**: ưu tiên nhánh ít `{error}` hơn (`Some(U(…))`/`U(…)` vs `None`), rồi nâng
    // lên đủ độ sâu `Option` của nhánh sâu nhất (biên nâng nhánh VD-258).
    let max = ds.iter().map(opt_depth_ty).max()?;
    let mut best: Option<Type> = None;
    let mut it_loi = usize::MAX;
    for t in ds {
        let loi = so_loi(&t);
        if loi <= it_loi {
            it_loi = loi;
            best = Some(t);
        }
    }
    let mut t = best?;
    while opt_depth_ty(&t) < max {
        t = Type::Opt(Box::new(t));
    }
    Some(t)
}

pub(crate) fn infer_if(
    then: &[HStmt],
    elifs: &[(HCond, Vec<HStmt>)],
    else_: Option<&[HStmt]>,
    sym: &HashMap<u32, Type>,
    types: &TypeTable,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
) -> Option<Type> {
    let else_ = else_?;
    let mut ds = Vec::new();
    for body in std::iter::once(then)
        .chain(elifs.iter().map(|(_, b)| b.as_slice()))
        .chain(std::iter::once(else_))
    {
        if let Some(t) = infer_than(body, sym, types, fn_rets, mret) {
            ds.push(t);
        }
    }
    hop_nhanh(ds)
}

pub(crate) fn infer_match(
    arms: &[HArm],
    sym: &HashMap<u32, Type>,
    types: &TypeTable,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
) -> Option<Type> {
    let ds: Vec<Type> = arms
        .iter()
        .filter_map(|a| infer_than(&a.body, sym, types, fn_rets, mret))
        .collect();
    hop_nhanh(ds)
}

/// **VD-264** — kiểu phần tử của biểu thức `for` (mirror checker `elem_of_iter`).
pub(crate) fn elem_of_iter_ty(t: &Type) -> Option<Type> {
    match t {
        Type::List(x) => Some((**x).clone()),
        Type::Map(k, v) => Some(Type::TypeRef {
            name: "tuple".into(),
            args: vec![(**k).clone(), (**v).clone()],
        }),
        Type::Str => Some(Type::Char),
        Type::TypeRef { name, args } if name == "range" => {
            Some(args.first().cloned().unwrap_or(Type::Int))
        }
        _ => None,
    }
}
