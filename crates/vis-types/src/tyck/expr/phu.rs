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

/// 8 tên dựng sẵn (VD-44 #3) — dùng khi `sym = None`.
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

/// Tách riêng để `phu.rs` không phải `use` cả crate `stdlib` ở đầu tệp.
fn vis_types_stdlib_sig(name: &str) -> Option<&'static crate::stdlib::StdFn> {
    crate::stdlib::sig_of(name)
}
