// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-61**: phụ trợ nhỏ của `tyck/expr.rs` — tách ra để `expr.rs` giữ **R1** ≤ 1.500 dòng.
//! Đây là các hàm **thuần** (không trạng thái), chuyển nguyên văn từ `expr.rs`.

use crate::ty::Type;

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
