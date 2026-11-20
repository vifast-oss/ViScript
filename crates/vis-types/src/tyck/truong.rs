// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-50**: truy cập **trường**/chỉ số — tách khỏi `expr.rs` cho luật R1 (≤1.500 dòng/tệp).
//! `kiem_field` là nơi **duy nhất** phát `VS0405` ("không có trường" của bản ghi).

use crate::hir::HExpr;
use crate::ty::Type;

use super::{Checker, MISMATCH, NO_FIELD};

impl<'a> Checker<'a> {
    pub(crate) fn kiem_field(
        &mut self,
        base: &HExpr,
        name: &str,
        optional: bool,
        span: vis_syntax::Span,
    ) -> Type {
        let bt = self.kiem_expr(base);
        // **ST-239 (VD-242 Q2=A + §3.2, ca 6)**: `?.` chỉ có nghĩa trên base `Option[U]`.
        // Base thường `U` phải dùng `.`; cả hai engine từng chấp nhận chương trình sai
        // (interp trả trường thô, VM bọc `Some` rồi trap) ⇒ từ chối tĩnh `VS0412`.
        if optional && !matches!(bt, Type::Opt(_) | Type::Error) {
            self.diag(
                MISMATCH,
                format!(
                    "`?.` cần base `Option[U]`, được `{}` — dùng `.` cho base thường (VD-242 §3.2)",
                    self.show(&bt)
                ),
                span,
            );
            return Type::Error;
        }
        match &bt {
            Type::Error => Type::Error,
            Type::Record(id) => match self.types.field(*id, name) {
                Some(f) => f.ty.clone(),
                None => {
                    self.diag(
                        vis_diag::ma_truong(),
                        format!("bản ghi không có trường `{name}` (VD-43 #6)"),
                        span,
                    );
                    Type::Error
                }
            },
            Type::Enum(id) => {
                if self.types.variant(*id, name).is_some() {
                    bt.clone()
                } else {
                    self.diag(
                        vis_diag::ma_truong(),
                        format!("enum không có biến thể `{name}`"),
                        span,
                    );
                    Type::Error
                }
            }
            Type::Opt(inner) => {
                if !optional {
                    self.diag(
                        MISMATCH,
                        format!(
                            "`{}` là optional — dùng `?.` để truy cập trường (§4.4)",
                            self.show(&bt)
                        ),
                        span,
                    );
                    return Type::Error;
                }
                match &**inner {
                    Type::Record(id) => match self.types.field(*id, name) {
                        Some(f) => {
                            let t = f.ty.clone();
                            Type::Opt(Box::new(t))
                        }
                        None => {
                            self.diag(
                                vis_diag::ma_truong(),
                                format!("bản ghi không có trường `{name}`"),
                                span,
                            );
                            Type::Error
                        }
                    },
                    Type::Error => Type::Error,
                    other => {
                        // Kiểu **không phải bản ghi**: giữ mã cũ `VS0403` (không phải "trường
                        // của bản ghi"); chỉ bản ghi/enum mới dùng `VS0405` (ST-50).
                        self.diag(
                            NO_FIELD,
                            format!("`{}` không có trường `{name}`", self.show(other)),
                            span,
                        );
                        Type::Error
                    }
                }
            }
            other => {
                // Kiểu **không phải bản ghi** ⇒ `VS0403` như trước (ST-50 giữ nghĩa cũ).
                self.diag(
                    NO_FIELD,
                    format!("`{}` không có trường `{name}`", self.show(other)),
                    span,
                );
                Type::Error
            }
        }
    }

    pub(crate) fn kiem_index(
        &mut self,
        base: &HExpr,
        index: &HExpr,
        _span: vis_syntax::Span,
    ) -> Type {
        let bt = self.kiem_expr(base);
        let it = self.kiem_expr(index);
        match &bt {
            Type::List(t) => {
                let want = Type::Int;
                self.coerce(&it, &want, index.span(), "chỉ số `list`");
                (**t).clone()
            }
            Type::Map(k, v) => {
                let kk = (**k).clone();
                self.coerce(&it, &kk, index.span(), "khoá `map`");
                (**v).clone()
            }
            Type::Str => {
                self.diag(
                    MISMATCH,
                    "`str` không có `s[i]` — dùng `.chars()`, `.graphemes()`, `.byte_at(i)?` (VD-18)",
                    base.span(),
                );
                Type::Error
            }
            Type::Error => Type::Error,
            other => {
                self.diag(
                    MISMATCH,
                    format!("không chỉ số được kiểu `{}`", self.show(other)),
                    base.span(),
                );
                Type::Error
            }
        }
    }
}

/// **VD-258** — kiểm giá trị mặc định của trường bản ghi tại **khai báo** (`type U: age: int = "x"`).
/// Tách khỏi `mod.rs` để giữ R1; vẫn kiểm lại ở call-site vì lower chép default vào
/// `HExpr::Record`.
///
/// **VD-260**: trùng tên trường trong cùng khai báo ⇒ `VS0412` (trước đây `field()` lấy trường
/// đầu, runtime/interp lại lấy trường cuối).
pub(super) fn kiem_khai_rec(c: &mut Checker<'_>, r: &crate::hir::HRec) {
    let mut da_thay: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for f in &r.fields {
        if !da_thay.insert(f.name.as_str()) {
            c.diag(
                super::MISMATCH,
                format!(
                    "bản ghi `{}` có trường `{}` khai báo nhiều lần (VD-260)",
                    r.name, f.name
                ),
                f.span,
            );
        }
    }
    kiem_default_rec(c, r);
}

pub(super) fn kiem_default_rec(c: &mut Checker<'_>, r: &crate::hir::HRec) {
    let luu = std::mem::take(&mut c.generic_scope);
    c.generic_scope = r.generics.clone();
    for f in &r.fields {
        if let Some(d) = &f.default {
            let raw = c.resolve_ann(&f.ty, f.span);
            let want = c.chuan(&super::erase_gen(&raw, &r.generics));
            let got = c.kiem_expr(d);
            c.coerce_e(d, &got, &want, "giá trị mặc định của trường");
        }
    }
    c.generic_scope = luu;
}

/// **VD-260** — khai báo enum: trùng **biến thể** hoặc trùng **tên trường** trong một biến thể
/// ⇒ `VS0412`. Không chạy thêm kiểm thân (enum không có default).
pub(super) fn kiem_khai_enum(c: &mut Checker<'_>, e: &crate::hir::HEnum) {
    let mut da_thay: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for v in &e.variants {
        if !da_thay.insert(v.name.as_str()) {
            c.diag(
                super::MISMATCH,
                format!(
                    "enum `{}` có biến thể `{}` khai báo nhiều lần (VD-260)",
                    e.name, v.name
                ),
                v.span,
            );
        }
        let mut truong: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        for f in &v.fields {
            if !truong.insert(f.name.as_str()) {
                c.diag(
                    super::MISMATCH,
                    format!(
                        "biến thể `{}` của enum `{}` có trường `{}` khai báo nhiều lần (VD-260)",
                        v.name, e.name, f.name
                    ),
                    f.span,
                );
            }
        }
    }
}

/// **VD-258** — kiểm `const Name: T = value` (tách khỏi `mod.rs` để giữ R1).
pub(super) fn kiem_const(c: &mut Checker<'_>, h: &crate::hir::HConst) {
    let want = c.resolve_ann(&h.ty, h.span);
    let got = c.kiem_expr(&h.value);
    // **VD-258**: `const` chưa có biên nâng Option ở runtime (interp/codegen inline nguyên
    // giá trị). Từ chối plain → `T?`/`T??` thay vì để `const X: int? = 42` lưu `42` trần;
    // `Some(...)`/`None` khớp đúng lớp vẫn hợp lệ.
    if want.is_opt() && !c.same(&got, &want) {
        c.diag(
            super::MISMATCH,
            format!(
                "`const`: cần `{}`, được `{}` — không tự nâng `Option` tại `const`; dùng `Some(…)`/`None`",
                c.show(&want),
                c.show(&got)
            ),
            h.value.span(),
        );
        return;
    }
    c.coerce_e(&h.value, &got, &want, "hằng");
}
