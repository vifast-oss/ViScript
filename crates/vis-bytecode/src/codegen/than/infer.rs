// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-311d (VD-286)** — suy kiểu tĩnh của biểu thức/câu lệnh cho codegen
//! (`tuple_ty_suy`/`infer_ty`/`bind_pat_ty`/`ctor_arg_ty`).
//!
//! Tách **nguyên văn** khỏi `codegen/than.rs` để giữ mỗi tệp ≤ 1.150 dòng.

use super::*;

/// **VD-264** — kiểu `tuple` với các phần tử `Error` được suy lại từ biểu thức tương ứng.
/// `ty` của lower chỉ dựa vào `HExpr` con (lời gọi hàm/`Some`/`None` ⇒ `Error`).
pub(super) fn tuple_ty_suy(
    items: &[HExpr],
    ty: &Type,
    sym: &HashMap<u32, Type>,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
    rec_fields: &HashMap<(u32, String), Type>,
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
                .and_then(|it| infer_ty(it, sym, fn_rets, mret, rec_fields))
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

/// **ST-248 / VD-260** — suy kiểu tĩnh của một biểu thức HIR; `None` khi không biết ⇒ không
/// nâng (an toàn, không đoán ngữ nghĩa mới). `fn_rets` cho lời gọi hàm, `rec_fields` cho field.
pub(super) fn infer_ty(
    e: &HExpr,
    sym: &HashMap<u32, Type>,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
    rec_fields: &HashMap<(u32, String), Type>,
) -> Option<Type> {
    match e {
        HExpr::Lit { ty, .. }
        | HExpr::Record { ty, .. }
        | HExpr::Opt { ty, .. }
        | HExpr::EnumLit { ty, .. } => Some(ty.clone()),
        // **VD-264**: `HExpr::Tuple.ty` do lower dựng từ `type_of` từng phần tử, mà
        // `type_of(Call)` là `Error` ⇒ `(Some(1), 2)` có kiểu `tuple[Error, int]`, pattern
        // `(a, _)` mất kiểu của `a`. Suy lại phần tử `Error` từ chính biểu thức (như list/map).
        HExpr::Tuple { items, ty, .. } => {
            Some(tuple_ty_suy(items, ty, sym, fn_rets, mret, rec_fields))
        }
        // **VD-260**: list/map literal chưa có kiểu phần tử (lowering để `Error` khi phần tử
        // là `Some(…)`/`None`/lời gọi) — suy từ **phần tử đầu tiên biết kiểu**. Nhờ vậy
        // `var xs = [Some(1)]` có `xs: list[int?]` và `xs[0] = 5` nâng đúng một lớp.
        // **VD-264**: cả khi kiểu phần tử đã có nhưng còn `Error` bên trong
        // (`[(Some(1), 2)]` ⇒ `tuple[Error, int]`), dùng kiểu suy được nếu **ít lỗi hơn**.
        HExpr::List { items, elem, .. } | HExpr::Set { items, elem, .. } => {
            let mut el = elem.clone();
            if let Some(t) = items
                .iter()
                .find_map(|i| infer_ty(i, sym, fn_rets, mret, rec_fields))
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
                .find_map(|(k, _)| infer_ty(k, sym, fn_rets, mret, rec_fields))
            {
                chon_kieu(&mut k, t);
            }
            let mut v = value.clone();
            if let Some(t) = entries
                .iter()
                .find_map(|(_, v)| infer_ty(v, sym, fn_rets, mret, rec_fields))
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
            let bt = infer_ty(base, sym, fn_rets, mret, rec_fields)?;
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
        HExpr::Index { base, .. } => match infer_ty(base, sym, fn_rets, mret, rec_fields)? {
            Type::Map(_, v) => Some(*v),
            Type::List(el) => Some(*el),
            _ => None,
        },
        HExpr::Call { callee, args, .. } => match (&**callee, args.first()) {
            (HExpr::Var { name, .. }, Some(a)) if name == "Some" => {
                infer_ty(a, sym, fn_rets, mret, rec_fields).map(|t| Type::Opt(Box::new(t)))
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
                infer_ty(a, sym, fn_rets, mret, rec_fields).map(|t| match t {
                    Type::List(e) => Type::Opt(e),
                    _ => Type::Opt(Box::new(Type::Error)),
                })
            }
            // **VD-260**: method dựng sẵn trả `Option` — `list.get/first/last/find`,
            // `map.get` (mirror tyck `method_ret`). Nhánh `if` gọi các method này cũng nâng.
            //
            // **VD-265**: phương thức **người dùng** còn dạng `Field` (không dispatch tĩnh) —
            // tra [`MethodRet`] theo kiểu nhận (`Record`/`Enum`/`TypeRef`, bóc `Opt`) để biết
            // `T?`/`list[T?]`/bản ghi, khớp interp ở `goi::infer_ty`.
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
                let bt = infer_ty(base, sym, fn_rets, mret, rec_fields)?;
                let inner = match &bt {
                    Type::Opt(t) => &**t,
                    other => other,
                };
                let ret = match inner {
                    Type::List(e) if matches!(name.as_str(), "get" | "first" | "last" | "find") => {
                        Type::Opt(e.clone())
                    }
                    Type::Map(_, v) if name == "get" => Type::Opt(v.clone()),
                    // **VD-265**: bảng cần `TypeTable`; `compile` không kèm bảng vẫn chạy như cũ
                    // (`None` ⇒ không nâng), còn `compile_*_voi_bang` (đường `vis run`) tra được.
                    _ => mret.ret_voi_types(&bt, name)?,
                };
                Some(if *optional {
                    vis_types::nang_optional(ret)
                } else {
                    ret
                })
            }
            _ => None,
        },
        HExpr::With { base, .. } => infer_ty(base, sym, fn_rets, mret, rec_fields),
        // **VD-263**: `expr?` — `T?` ⇒ `T`, `Result[T, E]` ⇒ `T`. Gốc `with`/field có thể là
        // giá trị đã bóc `?` (vd `maybe()? with f: 5`); thiếu nhánh này thì `base_ty` mù.
        //
        // **VD-264**: `xs[i]?` là **chỉ số có kiểm tra** (checker `kiem_index_opt`) ⇒ `T?`,
        // không phải `T`; trước đây `infer_ty` bóc lớp `Opt` nên đoán sai độ sâu Option.
        HExpr::Try { expr, .. } => {
            if let HExpr::Index { base, .. } = &**expr {
                return match infer_ty(base, sym, fn_rets, mret, rec_fields)? {
                    Type::List(el) => Some(Type::Opt(el)),
                    Type::Map(_, v) => Some(Type::Opt(v)),
                    Type::Bytes => Some(Type::Opt(Box::new(Type::Int))),
                    _ => None,
                };
            }
            match infer_ty(expr, sym, fn_rets, mret, rec_fields)? {
                Type::Opt(t) | Type::Result(t, _) => Some(*t),
                _ => None,
            }
        }
        // Nhánh `if`/`match`: chọn kiểu có **nhiều lớp Option nhất** (join theo kiểu đích).
        HExpr::If {
            then, elifs, else_, ..
        } => infer_if(then, elifs, Some(else_), sym, fn_rets, mret, rec_fields),
        HExpr::Match { arms, .. } => infer_match(arms, sym, fn_rets, mret, rec_fields),
        _ => None,
    }
}

/// **ST-248** — gán kiểu suy được từ pattern (`let`, `for`, arm `match`, `if let`); đệ quy theo
/// hình dạng pattern để biến bind trong arm cũng có kiểu tĩnh.
///
/// **VD-263**: trước đây chỉ `HPat::Bind` trần của `let`/`for` được gán; biến bind trong arm
/// `match` (`Some(u)`, `[u]`, `{f}`, ctor enum) không có kiểu ⇒ `base_ty` mù và `u with f: 5`
/// không nâng trên VM (interp vẫn đúng nhờ tra tên bản ghi lúc chạy).
pub(super) fn bind_pat_ty(
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
pub(super) fn ctor_arg_ty(
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
