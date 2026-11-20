// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-61**: phụ trợ nhỏ của `tyck/expr.rs` — tách ra để `expr.rs` giữ **R1** ≤ 1.500 dòng.
//! Đây là các hàm **thuần** (không trạng thái), chuyển nguyên văn từ `expr.rs`.

use crate::ty::Type;

/// **ST-239 (VD-242 Q4a + §3.4)**: dạng chuẩn của **khoá map literal xác định được tĩnh**.
/// Chỉ nhận literal — mọi biểu thức khác trả `None` và để runtime last-wins xử lý. Tuyệt đối
/// **không gọi mã người dùng** ở checker. Số `int`/`float` cùng giá trị (VD-17: `1 == 1.0`)
/// dùng chung một dạng để bắt cả cặp trùng `1` / `1.0`.
pub(super) fn static_key(e: &crate::hir::HExpr) -> Option<String> {
    let crate::hir::HExpr::Lit { lit, .. } = e else {
        return None;
    };
    use crate::hir::LitKind;
    let key = match lit.kind {
        LitKind::Int => match lit.text.parse::<i64>() {
            Ok(n) => format!("n:{n}"),
            Err(_) => format!("int:{}", lit.text),
        },
        LitKind::Float => match lit.text.parse::<f64>() {
            Ok(f)
                if f.is_finite()
                    && f.fract() == 0.0
                    && f >= i64::MIN as f64
                    && f <= i64::MAX as f64 =>
            {
                format!("n:{}", f as i64)
            }
            Ok(f) => format!("n:{f:?}"),
            Err(_) => format!("float:{}", lit.text),
        },
        kind => format!("{kind:?}:{}", lit.text),
    };
    Some(key)
}

/// `percent` và `float` so sánh được **theo tỉ lệ** (§3.1/§3.6) — đặc cách duy nhất.
pub(super) fn is_percent_float(a: &Type, b: &Type) -> bool {
    matches!(
        (a, b),
        (Type::Percent, Type::Float) | (Type::Float, Type::Percent)
    )
}

/// 8 tên dựng sẵn (VD-44 #3) + lệnh chế độ shell — dùng khi `sym = None`.
pub(super) fn builtin_is_value(name: &str) -> bool {
    matches!(
        name,
        "print"
            | "len"
            | "str"
            | "Ok"
            | "Err"
            | "Some"
            | "None"
            | "ref"
            | "panic"
            | "sort_by"
            | "get"
            | "select"
    )
}

/// Kiểu của 8 tên giá trị dựng sẵn (không phải hàm) — `None`/`Some`… không có kiểu riêng ở đây.
pub(super) fn builtin_value_ty(name: &str) -> Type {
    match name {
        "None" => Type::Opt(Box::new(Type::Error)),
        _ => Type::Error,
    }
}

/// **R6c (VD-178)**: kiểm `{expr:spec}` — kiểu biểu thức + spec (`.N`, N ≤ 17).
pub(super) fn kiem_expr_spec(c: &mut super::super::Checker, expr: &crate::hir::HExpr, spec: &str) {
    let t = c.kiem_expr(expr);
    if !matches!(t, Type::Int | Type::Float | Type::Str | Type::Error) {
        c.diag(
            super::super::MISMATCH,
            format!(
                "`{{…:{spec}}}` chỉ định dạng được `int`/`float`/`str`, được `{}`",
                c.show(&t)
            ),
            expr.span(),
        );
    }
    if !spec_hop_le(spec) {
        c.diag(
            super::super::MISMATCH,
            format!("format spec `{spec}` chưa hỗ trợ — dùng `.N` (N chữ số thập phân)"),
            expr.span(),
        );
    }
}

/// Spec hợp lệ — `.N` với `N` 0..=17 (chữ số thập phân).
fn spec_hop_le(spec: &str) -> bool {
    let Some(n) = spec.strip_prefix('.') else {
        return false;
    };
    !n.is_empty()
        && n.len() <= 2
        && n.chars().all(|c| c.is_ascii_digit())
        && n.parse::<u8>().map(|v| v <= 17).unwrap_or(false)
}

/// **R6c (VD-178)**: kiểm **từng** mảnh của chuỗi nội suy (ST-14 + `{expr:spec}`).
pub(super) fn kiem_interp(c: &mut super::super::Checker, parts: &[crate::hir::InterpPart]) {
    for p in parts {
        match p {
            crate::hir::InterpPart::Expr(e) => {
                c.kiem_expr(e);
            }
            crate::hir::InterpPart::ExprSpec { expr, spec } => kiem_expr_spec(c, expr, spec),
            crate::hir::InterpPart::Str(_) => {}
        }
    }
}

/// **R17 (VD-178)**: `is_empty()` — 0 đối số, trả `bool` (list/map/str).
pub(super) fn is_empty(
    c: &mut super::super::Checker,
    args: &[crate::hir::HExpr],
    name: &str,
    span: vis_syntax::Span,
) -> Type {
    c.arity(args, 0, name, span);
    Type::Bool
}

/// **R8 (VD-178)**: `xs[i]?` — chỉ số **có kiểm tra** ⇒ `T?` (không cần hàm trả `T?`).
pub(super) fn kiem_index_opt(
    c: &mut super::super::Checker,
    base: &crate::hir::HExpr,
    index: &crate::hir::HExpr,
    span: vis_syntax::Span,
) -> Type {
    let bt = c.kiem_expr(base);
    // **R8-fix (VD-178)**: chỉ số phải đúng kiểu — `int` cho `list`/`bytes`, `K` cho `map[K, V]`.
    let it = c.kiem_expr(index);
    let khoa = match &bt {
        Type::Map(k, _) => (**k).clone(),
        _ => Type::Int,
    };
    c.coerce_e(index, &it, &khoa, "chỉ số");
    let elem = match &bt {
        Type::List(e) => (**e).clone(),
        Type::Bytes => Type::Int,
        Type::Map(_, v) => (**v).clone(),
        Type::Error => Type::Error,
        other => {
            let o = other.clone();
            c.diag(
                super::super::MISMATCH,
                format!("`xs[i]?` cần `list`/`map`/`bytes`, được `{}`", c.show(&o)),
                span,
            );
            Type::Error
        }
    };
    Type::Opt(Box::new(elem))
}

/// **R8 (VD-178) + §4.4**: `?` — `xs[i]?` là chỉ số có kiểm tra; còn lại đòi hàm trả `T?`/`Result`.
pub(super) fn kiem_try(
    c: &mut super::super::Checker,
    expr: &crate::hir::HExpr,
    span: vis_syntax::Span,
) -> Type {
    if let crate::hir::HExpr::Index { base, index, .. } = expr {
        return kiem_index_opt(c, base, index, span);
    }
    let t = c.kiem_expr(expr);
    match t {
        Type::Opt(inner) => {
            if c.try_hop_le(true) {
                (*inner).clone()
            } else {
                c.diag(
                    super::super::MISMATCH,
                    format!(
                        "`?` trên `{}` chỉ dùng được khi hàm trả `{}?`, hàm này trả `{}` (§4.4)",
                        c.show(&Type::Opt(inner.clone())),
                        c.show(&inner),
                        c.show(&c.ham_tra())
                    ),
                    span,
                );
                Type::Error
            }
        }
        Type::Result(inner, _) => {
            if c.try_hop_le(false) {
                (*inner).clone()
            } else {
                c.diag(
                    super::super::MISMATCH,
                    format!(
                        "`?` trên `Result` chỉ dùng được khi hàm trả `Result[…, …]`, hàm này trả `{}` (§4.4)",
                        c.show(&c.ham_tra())
                    ),
                    span,
                );
                Type::Error
            }
        }
        Type::Error => Type::Error,
        other => {
            c.diag(
                super::super::MISMATCH,
                format!("`?` chỉ dùng cho `T?`/`Result`, được `{}`", c.show(&other)),
                span,
            );
            Type::Error
        }
    }
}

/// **R5c (VD-178)**: `x.f(a) ≡ std.<mod>.f(x, a)` khi **tham số đầu** của `f` có kiểu của `x`.
pub(super) fn kiem_std_method(
    c: &mut super::super::Checker,
    recv: &Type,
    name: &str,
    args: &[crate::hir::HExpr],
    span: vis_syntax::Span,
) -> Option<Type> {
    let m = match recv {
        Type::Str => "str",
        Type::List(_) => "list",
        Type::Map(..) => "map",
        _ => return None,
    };
    let full = format!("{m}.{name}");
    let f = vis_types_stdlib_sig(&full)?;
    let want = f.params.first()?.sang_type();
    if !c.same(&want, recv) {
        return None;
    }
    for (i, a) in args.iter().enumerate() {
        let at = c.kiem_expr(a);
        if let Some(p) = f.params.get(i + 1) {
            let pt = p.sang_type();
            c.coerce_e(a, &at, &pt, "đối số");
        }
    }
    let _ = span;
    Some(f.ret.sang_type())
}

/// **VD-258** — hợp nhất một phần tử literal vào `acc` (có xử lý `Option`).
/// Literal `list`/`set`/`map` **không có biên nâng** plain → `T?`; trộn hai dạng ⇒ `VS0412`
/// thay vì để runtime giữ giá trị plain trong ô `T?`. `None` (`Option[{error}]`) nhường cho
/// vế có lõi cụ thể (cùng là `Option`).
pub(super) fn hop_ve_literal(
    c: &mut super::super::Checker,
    acc: &mut Type,
    e: &crate::hir::HExpr,
    t: &Type,
    what: &str,
) {
    if acc.is_error() {
        *acc = t.clone();
        return;
    }
    let a_opt = acc.is_opt();
    let t_opt = t.is_opt();
    if a_opt != t_opt {
        c.diag(
            super::super::MISMATCH,
            format!(
                "{what}: không trộn `Option` và giá trị thường (`{}` và `{}`) — bọc `Some(…)` cho đồng nhất",
                c.show(acc),
                c.show(t)
            ),
            e.span(),
        );
        return;
    }
    if let Type::Opt(inner) = acc {
        if inner.is_error() {
            *acc = t.clone();
            return;
        }
    }
    if let Type::Opt(inner) = t {
        if inner.is_error() {
            return;
        }
    }
    c.coerce_e(e, t, acc, what);
}

/// **VD-258** — kiểu chung của dãy phần tử `list`/`set` (gọi từ `tyck/expr.rs`).
pub(super) fn unify_all(
    c: &mut super::super::Checker,
    items: &[crate::hir::HExpr],
    what: &str,
    _span: vis_syntax::Span,
) -> Type {
    let mut acc = Type::Error;
    for (i, it) in items.iter().enumerate() {
        let t = c.kiem_expr(it);
        if i == 0 {
            acc = t;
            continue;
        }
        hop_ve_literal(c, &mut acc, it, &t, what);
    }
    acc
}

/// **VD-258** — hợp nhất hai nhánh `if`/`match`; `None` trần (`Opt[{error}]`) với nhánh thường
/// cho `T?` để `fn f() -> int: if c: 1 else: None` bị từ chối ở biên return.
pub(super) fn unify_branch(
    c: &mut super::super::Checker,
    acc: &mut Type,
    t: &Type,
    span: vis_syntax::Span,
) {
    if matches!(acc, Type::Error) {
        *acc = t.clone();
        return;
    }
    // **VD-241 §6/B-A (A0, 2026-11-19)** — `Never` khớp mọi kiểu; một nhánh toàn `return`/di
    // không bao giờ ép bên kia.
    if matches!(t, Type::Error) || matches!(t, Type::Never) || c.same(acc, t) {
        return;
    }
    if matches!(acc, Type::Never) {
        *acc = t.clone();
        return;
    }
    if let Type::Opt(inner) = t {
        if !matches!(acc, Type::Opt(_)) {
            // `None` (`Opt[{error}]`) hoặc nhánh `Some(…)` gặp nhánh thường: kiểu chung là `T?`;
            // runtime nâng nhánh thường (`than::do_if`/`do_match` + `lift_opt`).
            if inner.is_error() || c.same(acc, inner) {
                *acc = Type::Opt(Box::new(acc.clone()));
                return;
            }
        }
    }
    if let Type::Opt(inner) = acc {
        if c.same(inner, t) {
            return;
        }
    }
    let msg = format!("hai nhánh khác kiểu: `{}` và `{}`", c.show(acc), c.show(t));
    c.diag(super::super::MISMATCH, msg, span);
}

/// **VD-267** — chữ ký **builtin trần** (`stdlib::BUILTIN_FUNCS`): `panic(str|Error) -> never`.
/// Trả `None` nếu `name` không có trong bảng (để `kiem_builtin` xử lý các builtin cũ).
pub(super) fn kiem_builtin_sig(
    c: &mut super::super::Checker<'_>,
    name: &str,
    args: &[crate::hir::HExpr],
    span: vis_syntax::Span,
) -> Option<Type> {
    let sig = crate::stdlib::sig_builtin(name)?;
    let mut tys = Vec::with_capacity(args.len());
    for a in args {
        tys.push(c.kiem_expr(a));
    }
    if tys.len() != sig.so_doi() {
        c.diag(
            super::super::ARG_COUNT,
            format!("`{name}` cần {} đối số, được {}", sig.so_doi(), tys.len()),
            span,
        );
        return Some(sig.ret.sang_type());
    }
    if let Some(t) = tys.first() {
        // Thiết kế §6.4: `panic(err)` nhận `E: Error`; `Ty` không có union nên nhận diện
        // bản ghi dựng sẵn `Error` ở đây (cùng `RecId(0)` ở mọi bảng).
        let la_error = match t {
            Type::Record(id) => c.types.rec(*id).and_then(|r| r.name.as_deref()) == Some("Error"),
            _ => false,
        };
        if !matches!(t, Type::Str | Type::Error) && !la_error {
            c.diag(
                super::super::MISMATCH,
                format!("`{name}` cần `str` hoặc `Error`, được `{}`", c.show(t)),
                args[0].span(),
            );
        }
    }
    Some(sig.ret.sang_type())
}

/// **VD-258** — kiểm trường của literal/dựng bản ghi theo `Type::Record(id)`:
/// sai kiểu ⇒ `VS0412` (kèm VD-238 cho `None`), trường lạ ⇒ `VS0403`, thiếu trường bắt buộc
/// (HIR dựng tay) ⇒ `VS0412`.
pub(super) fn kiem_truong_ban_ghi(
    c: &mut super::super::Checker,
    fields: &[(String, crate::hir::HExpr)],
    ty: &Type,
) {
    let Type::Record(id) = ty else {
        for (_, v) in fields {
            let _ = c.kiem_expr(v);
        }
        return;
    };
    for (n, v) in fields {
        let vt = c.kiem_expr(v);
        match c.types.field(*id, n) {
            Some(f) => {
                let want = f.ty.clone();
                c.coerce_e(v, &vt, &want, "trường bản ghi");
            }
            None => c.diag(
                super::super::NO_FIELD,
                format!("bản ghi `{}` không có trường `{n}` (VD-43 #6)", c.show(ty)),
                v.span(),
            ),
        }
    }
    if let Some(def) = c.types.rec(*id).cloned() {
        for f in &def.fields {
            if !f.has_default && !fields.iter().any(|(n, _)| n == &f.name) {
                c.diag(
                    super::super::MISMATCH,
                    format!(
                        "thiếu trường bắt buộc `{}` của bản ghi `{}`",
                        f.name,
                        c.show(ty)
                    ),
                    f.span,
                );
            }
        }
    }
}

/// **VD-260** — tên `HExpr::Var{sym:None}` được miễn `VS0301`: tên dựng sẵn, module `std`,
/// tiền tố module của đơn vị biên dịch, hoặc tên kiểu dùng làm tiền tố ctor (`Json.Arr(…)`).
pub(super) fn ten_duoc_mien(c: &super::super::Checker<'_>, name: &str) -> bool {
    if name == "std" || name.starts_with("std.") || name.starts_with("exec.") {
        return true;
    }
    if crate::lower::BUILTIN_VALUE_NAMES.contains(&name) {
        return true;
    }
    if crate::stdlib::MODULES.contains(&name) {
        return true;
    }
    if matches!(
        name,
        "assert"
            | "sort_by"
            | "get"
            | "select"
            | "where"
            | "each"
            | "chan"
            | "render"
            | "ls"
            | "open"
            | "count"
            | "first"
            | "pass"
            | "collect"
            | "Option"
            | "Result"
    ) {
        return true;
    }
    if Type::builtin(name).is_some()
        || c.types.find_rec_base(name).is_some()
        || c.types.find_enum_base(name).is_some()
    {
        return true;
    }
    c.symbols.mods.iter().any(|m| m.name == name)
}

/// **VD-260** — gọi/dựng bản ghi: chỉ nhận **trường có tên**; `U(1)`/`U()` bị từ chối tĩnh.
pub(super) fn kiem_goi_ban_ghi(
    c: &mut super::super::Checker<'_>,
    id: crate::ty::RecId,
    args: &[crate::hir::HExpr],
    named: &[(String, crate::hir::HExpr)],
    span: vis_syntax::Span,
) {
    if !args.is_empty() {
        c.diag(
            super::super::MISMATCH,
            format!(
                "dựng bản ghi `{}` cần trường có tên — không nhận đối số vị trí (VD-260)",
                c.show(&Type::Record(id))
            ),
            span,
        );
        for a in args {
            let _ = c.kiem_expr(a);
        }
        for (n, a) in named {
            let at = c.kiem_expr(a);
            if let Some(f) = c.types.field(id, n) {
                let want = f.ty.clone();
                c.coerce(&at, &want, a.span(), "trường bản ghi");
            } else {
                c.diag(
                    super::super::NO_FIELD,
                    format!("bản ghi không có trường `{n}`"),
                    a.span(),
                );
            }
        }
    } else if named.is_empty() {
        let ten = c.show(&Type::Record(id));
        c.diag(
            super::super::MISMATCH,
            format!("dựng bản ghi `{ten}` thiếu trường — cần `{ten}(trường: giá trị, …)` (VD-260)"),
            span,
        );
    } else {
        kiem_truong_ban_ghi(c, named, &Type::Record(id));
    }
}

/// **VD-260** — `with` trùng trường không được lặng lẽ last-wins.
pub(super) fn kiem_with_trung(
    c: &mut super::super::Checker<'_>,
    fields: &[(String, crate::hir::HExpr)],
    span: vis_syntax::Span,
) {
    let mut da_thay: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for (n, _) in fields {
        if !da_thay.insert(n.as_str()) {
            c.diag(
                super::super::MISMATCH,
                format!("`with`: trường `{n}` xuất hiện nhiều lần (VD-260)"),
                span,
            );
        }
    }
}

/// Tách riêng để `phu.rs` không phải `use` cả crate `stdlib` ở đầu tệp.
fn vis_types_stdlib_sig(name: &str) -> Option<&'static crate::stdlib::StdFn> {
    crate::stdlib::sig_of(name)
}

/// **ST-72 (VD-106)**: kiểu quyền `cap[...]` — chuyển từ `expr.rs` để giữ R1.
pub(super) fn la_cap(t: &Type) -> bool {
    matches!(t, Type::TypeRef { name, .. } if name == "cap")
}
