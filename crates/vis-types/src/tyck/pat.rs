// SPDX-License-Identifier: MIT OR Apache-2.0
//! Pattern: gán kiểu cho binding + **xét hết** `match` (`VS0402`, VD-43 #5).
//!
//! Luật xét hết: enum ⇒ **mọi** biến thể; `bool` ⇒ `true`/`false`; `T?` ⇒ `Some`/`None`
//! (`Some`/`None` cũng được); `Result` ⇒ `ok`/`err`; bản ghi ⇒ **đủ mọi trường, không `..`**;
//! kiểu vô hạn (`int`, `str`, `list`, `map`, …) ⇒ **bắt buộc `_`**; **guard không tính**;
//! or-pattern tính **cả hai** vế.

use std::collections::BTreeSet;

use vis_syntax::Span;

use crate::hir::{HArm, HBody, HCond, HExpr, HPat, HStmt, InterpPart, LitKind};
use crate::ty::{Type, TypeTable};

use super::{Checker, ARG_COUNT, MISMATCH, NON_EXHAUSTIVE, NO_FIELD};

impl<'a> Checker<'a> {
    /// Gán kiểu cho các tên trong pattern; kiểm literal/ctor khớp `subj`.
    pub(crate) fn bind_pat(&mut self, pat: &HPat, subj: &Type) {
        match pat {
            HPat::Wild { .. } => {}
            HPat::Rest { .. } => {}
            HPat::Bind { sym, .. } => self.set_ty(*sym, subj.clone()),
            HPat::Lit { lit, span } => {
                // **VD-266**: pattern `int` cũng phải biểu diễn được trong `i64`; lexer chỉ
                // kiểm **độ lớn** nên literal dương 2^63 bị chặn ở đây (pattern không đi qua
                // phép gộp `-` của tầng hạ biểu thức).
                self.kiem_int_lit(lit, *span);
                let t = lit.ty();
                if !self.same(&t, subj) {
                    let msg = format!(
                        "pattern literal: cần `{}`, được `{}`",
                        self.show(subj),
                        self.show(&t)
                    );
                    self.diag(MISMATCH, msg, *span);
                }
            }
            HPat::Or { alts, .. } => {
                for a in alts {
                    self.bind_pat(a, subj);
                }
            }
            HPat::Range { lo, hi, .. } => {
                for p in [lo, hi].into_iter().flatten() {
                    self.bind_pat(p, subj);
                }
            }
            HPat::Tuple { items, .. } => {
                let elem: Vec<Type> = match subj {
                    Type::TypeRef { name, args } if name == "tuple" => args.clone(),
                    _ => Vec::new(),
                };
                for (i, it) in items.iter().enumerate() {
                    let t = elem.get(i).cloned().unwrap_or(Type::Error);
                    self.bind_pat(it, &t);
                }
            }
            HPat::List { items, rest, .. } => {
                let elem = match subj {
                    Type::List(t) => (**t).clone(),
                    _ => Type::Error,
                };
                for it in items {
                    self.bind_pat(it, &elem);
                }
                if let Some(r) = rest {
                    let rt = Type::List(Box::new(elem));
                    self.bind_pat(r, &rt);
                }
            }
            HPat::Record { fields, .. } => {
                // **VD-260**: `{a, a, b}` — cùng một trường xuất hiện nhiều lần trong pattern.
                let mut da_thay: BTreeSet<String> = BTreeSet::new();
                for (name, _) in fields {
                    if !name.is_empty() && !da_thay.insert(name.clone()) {
                        self.diag(
                            MISMATCH,
                            format!(
                                "pattern bản ghi có trường `{name}` xuất hiện nhiều lần (VD-260)"
                            ),
                            pat.span(),
                        );
                    }
                }
                let id = match subj {
                    Type::Record(id) => Some(*id),
                    Type::Error => None,
                    other => {
                        self.diag(
                            MISMATCH,
                            format!(
                                "pattern bản ghi cần bản ghi, được `{}` (VD-260)",
                                self.show(other)
                            ),
                            pat.span(),
                        );
                        None
                    }
                };
                for (name, sub) in fields {
                    let fty = id
                        .and_then(|id| self.types.field(id, name).map(|f| f.ty.clone()))
                        .unwrap_or_else(|| {
                            if let (Some(id), "") = (id, name.as_str()) {
                                let _ = id;
                            }
                            Type::Error
                        });
                    if id.is_some() && fty.is_error() && !name.is_empty() {
                        self.diag(
                            NO_FIELD,
                            format!("bản ghi không có trường `{name}`"),
                            pat.span(),
                        );
                    }
                    if let Some(s) = sub {
                        self.bind_pat(s, &fty);
                    }
                }
            }
            HPat::Ctor { path, args, span } => {
                let name = path.last().cloned().unwrap_or_default();
                match subj {
                    Type::Error => {
                        for a in args {
                            self.bind_pat(a, &Type::Error);
                        }
                    }
                    Type::Enum(id) => {
                        let def = self.types.enum_def(*id).cloned();
                        let variant = def
                            .as_ref()
                            .and_then(|d| d.variants.iter().find(|v| v.name == name).cloned());
                        match variant {
                            Some(v) => {
                                self.kiem_ctor_args(&v.fields, args, *span, &name);
                            }
                            None => {
                                // **VD-212 (A0)**: biến thể viết sai hoa thường ⇒ gợi ý dạng đúng
                                // (`circle` ⇒ `Circle`); enum người dùng cũng vậy (hai biến thể
                                // `A`/`a` phải khớp đúng từng biến thể).
                                let sai_case = def.as_ref().and_then(|d| {
                                    d.variants
                                        .iter()
                                        .find(|v| v.name.eq_ignore_ascii_case(&name))
                                        .cloned()
                                });
                                let ename = def.map(|d| d.name).unwrap_or_default();
                                match sai_case {
                                    Some(v) => {
                                        self.diag(
                                            &vis_diag::SYNTAX_CODES[2],
                                            format!(
                                                "`{name}` viết sai hoa thường — biến thể là `{}`",
                                                v.name
                                            ),
                                            *span,
                                        );
                                        self.kiem_ctor_args(&v.fields, args, *span, &v.name);
                                    }
                                    None => {
                                        self.diag(
                                            NO_FIELD,
                                            format!("enum `{ename}` không có biến thể `{name}`"),
                                            *span,
                                        );
                                        for a in args {
                                            self.bind_pat(a, &Type::Error);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Type::Opt(inner) => match name.as_str() {
                        "Some" => {
                            if args.len() != 1 {
                                self.diag(
                                    ARG_COUNT,
                                    format!("`Some` cần 1 đối số, được {}", args.len()),
                                    *span,
                                );
                            }
                            let t = (**inner).clone();
                            for a in args {
                                self.bind_pat(a, &t);
                            }
                        }
                        "None" => {
                            if !args.is_empty() {
                                self.diag(
                                    ARG_COUNT,
                                    format!("`None` không có đối số, được {}", args.len()),
                                    *span,
                                );
                            }
                        }
                        other => {
                            // **VD-212 (A0)**: sai hoa thường (`some`/`none`) — hạ đã báo `VS0203`
                            // (lower/pat.rs) nên ở đây chỉ bind đúng kiểu, không báo trùng.
                            let cap = cap_dau(other);
                            if cap == "Some" || cap == "None" {
                                let t = (**inner).clone();
                                if cap == "Some" {
                                    if args.len() != 1 {
                                        self.diag(
                                            ARG_COUNT,
                                            format!("`Some` cần 1 đối số, được {}", args.len()),
                                            *span,
                                        );
                                    }
                                    for a in args {
                                        self.bind_pat(a, &t);
                                    }
                                } else if !args.is_empty() {
                                    self.diag(
                                        ARG_COUNT,
                                        format!("`None` không có đối số, được {}", args.len()),
                                        *span,
                                    );
                                }
                            } else {
                                self.diag(
                                    NO_FIELD,
                                    format!("`{}` chỉ có `Some`/`None`", self.show(subj)),
                                    *span,
                                );
                                for a in args {
                                    self.bind_pat(a, &Type::Error);
                                }
                            }
                        }
                    },
                    Type::Result(a, b) => match name.as_str() {
                        "Ok" => {
                            let t = (**a).clone();
                            if args.len() != 1 {
                                self.diag(ARG_COUNT, "`Ok` cần 1 đối số", *span);
                            }
                            for x in args {
                                self.bind_pat(x, &t);
                            }
                        }
                        "Err" => {
                            let t = (**b).clone();
                            if args.len() != 1 {
                                self.diag(ARG_COUNT, "`Err` cần 1 đối số", *span);
                            }
                            for x in args {
                                self.bind_pat(x, &t);
                            }
                        }
                        other => {
                            // **VD-212 (A0)**: sai hoa thường — hạ đã báo `VS0203` (lower/pat.rs);
                            // chỉ bind đúng kiểu, không báo trùng.
                            let cap = cap_dau(other);
                            if cap == "Ok" || cap == "Err" {
                                let t = if cap == "Ok" {
                                    (**a).clone()
                                } else {
                                    (**b).clone()
                                };
                                if args.len() != 1 {
                                    self.diag(ARG_COUNT, format!("`{cap}` cần 1 đối số"), *span);
                                }
                                for x in args {
                                    self.bind_pat(x, &t);
                                }
                            } else {
                                self.diag(
                                    NO_FIELD,
                                    format!("`{}` chỉ có `Ok`/`Err`", self.show(subj)),
                                    *span,
                                );
                                for x in args {
                                    self.bind_pat(x, &Type::Error);
                                }
                            }
                        }
                    },
                    other => {
                        self.diag(
                            MISMATCH,
                            format!(
                                "pattern ctor `{name}` không khớp kiểu `{}` — gợi ý: `let`/gán bóc giá trị cần khớp **luôn**; dùng `match` hoặc `if let` khi pattern có thể không khớp",
                                self.show(other)
                            ),
                            *span,
                        );
                        for a in args {
                            self.bind_pat(a, &Type::Error);
                        }
                    }
                }
            }
        }
    }

    fn kiem_ctor_args(
        &mut self,
        fields: &[crate::ty::FieldDef],
        args: &[HPat],
        span: Span,
        name: &str,
    ) {
        if args.len() != fields.len() {
            self.diag(
                ARG_COUNT,
                format!(
                    "biến thể `{name}` cần {} đối số, pattern có {}",
                    fields.len(),
                    args.len()
                ),
                span,
            );
        }
        for (i, a) in args.iter().enumerate() {
            let t = fields.get(i).map(|f| f.ty.clone()).unwrap_or(Type::Error);
            self.bind_pat(a, &t);
        }
    }

    /// `match` phải **xét hết** — `guard` không tính (thiết kế §4.3).
    pub(crate) fn xet_het(&mut self, subj: &Type, arms: &[HArm], span: Span) {
        if arms.is_empty() {
            self.diag(NON_EXHAUSTIVE, "`match` không có nhánh nào", span);
            return;
        }
        // Chỉ nhánh **không guard** mới chứng minh phủ.
        let plain: Vec<&HPat> = arms
            .iter()
            .filter(|a| a.guard.is_none())
            .map(|a| &a.pat)
            .collect();
        if plain.iter().any(|p| phu_tat_ca(p)) {
            return;
        }
        match subj {
            Type::Error => {}
            Type::Enum(id) => {
                let Some(def) = self.types.enum_def(*id) else {
                    return;
                };
                let mut cov = BTreeSet::new();
                for p in &plain {
                    ctor_names(p, &mut cov);
                }
                let missing: Vec<String> = def
                    .variants
                    .iter()
                    .filter(|v| !cov.contains(&v.name.to_ascii_lowercase()))
                    .map(|v| v.name.clone())
                    .collect();
                if !missing.is_empty() {
                    self.diag(
                        NON_EXHAUSTIVE,
                        format!("`match` thiếu nhánh: {}", missing.join(", ")),
                        span,
                    );
                }
            }
            Type::Opt(_) | Type::Result(..) => {
                let mut cov = BTreeSet::new();
                for p in &plain {
                    ctor_names(p, &mut cov);
                }
                let need: &[&str] = match subj {
                    // `ctor_names` so khớp **thường**; thông điệp in **hoa** chữ đầu (UpperCamel).
                    Type::Opt(_) => &["some", "none"],
                    _ => &["ok", "err"],
                };
                let hoa = |n: &str| -> String {
                    let mut c = n.chars();
                    match c.next() {
                        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                        None => n.to_string(),
                    }
                };
                let missing: Vec<String> = need
                    .iter()
                    .filter(|n| !cov.contains(**n))
                    .map(|n| hoa(n))
                    .collect();
                if !missing.is_empty() {
                    self.diag(
                        NON_EXHAUSTIVE,
                        format!("`match` thiếu nhánh: {}", missing.join(", ")),
                        span,
                    );
                }
            }
            Type::Bool => {
                let mut cov = BTreeSet::new();
                for p in &plain {
                    bool_names(p, &mut cov);
                }
                let missing: Vec<&str> = ["true", "false"]
                    .into_iter()
                    .filter(|n| !cov.contains(*n))
                    .collect();
                if !missing.is_empty() {
                    self.diag(
                        NON_EXHAUSTIVE,
                        format!("`match` thiếu nhánh: {}", missing.join(", ")),
                        span,
                    );
                }
            }
            Type::Record(id) => {
                let Some(def) = self.types.rec(*id) else {
                    return;
                };
                let all: BTreeSet<String> = def.fields.iter().map(|f| f.name.clone()).collect();
                // Một nhánh **không `..`** phủ đủ mọi trường ⇒ xét hết.
                for p in &plain {
                    let mut cov = BTreeSet::new();
                    let mut open = false;
                    record_cov(p, &mut cov, &mut open);
                    // ST-15b (mục 3): nhánh c�� `..` (open) **phủ mọi trường còn lại** (§4.3).
                    if open || all.is_subset(&cov) {
                        return;
                    }
                }
                let mut cov = BTreeSet::new();
                let mut open = false;
                for p in &plain {
                    record_cov(p, &mut cov, &mut open);
                }
                let missing: Vec<String> = all.difference(&cov).cloned().collect();
                self.diag(
                    NON_EXHAUSTIVE,
                    format!(
                        "`match` trên bản ghi phải có đủ mọi trường (không `..`), thiếu: {}",
                        missing.join(", ")
                    ),
                    span,
                );
            }
            // ST-12b MAJOR #7: `unit` và tuple là kiểu **hữu hạn** — không đòi `_` oan.
            Type::Unit => {
                let covered = plain
                    .iter()
                    .any(|p| matches!(p, HPat::Tuple { items, .. } if items.is_empty()));
                if !covered {
                    self.diag(
                        NON_EXHAUSTIVE,
                        "`match` trên `()` cần nhánh `()` hoặc `_` (kiểu `unit` chỉ có một giá trị)",
                        span,
                    );
                }
            }
            Type::TypeRef { name, args } if name == "tuple" => {
                let n = args.len();
                let covered = plain.iter().any(|p| tuple_phu_het(p, n));
                if !covered {
                    self.diag(
                        NON_EXHAUSTIVE,
                        format!(
                            "`match` trên tuple {n} thành phần phải phủ **mọi** thành phần bằng `_`/tên trần (hoặc dùng `_` cho cả tuple)"
                        ),
                        span,
                    );
                }
            }
            other => {
                self.diag(
                    NON_EXHAUSTIVE,
                    format!(
                        "`{}` là kiểu vô hạn — `match` bắt buộc có nhánh `_` (thiết kế §4.3)",
                        self.show(other)
                    ),
                    span,
                );
            }
        }
    }
}

/// Pattern phủ **mọi** giá trị (`_`, tên trần, hoặc or-pattern có một vế như vậy).
fn phu_tat_ca(p: &HPat) -> bool {
    match p {
        HPat::Wild { .. } | HPat::Bind { .. } => true,
        HPat::Or { alts, .. } => alts.iter().any(phu_tat_ca),
        _ => false,
    }
}

/// **VD-212 (A0)**: viết thường chữ cái đầu (`SOME` ⇒ `Some`) — dùng để nhận ra biến thể viết sai
/// hoa thường và gợi ý dạng đúng. Giữ nguyên phần còn lại (không `title case`).
fn cap_dau(s: &str) -> String {
    let low = s.to_ascii_lowercase();
    let mut it = low.chars();
    match it.next() {
        Some(f) => f.to_uppercase().collect::<String>() + it.as_str(),
        None => low,
    }
}

fn ctor_names(p: &HPat, out: &mut BTreeSet<String>) {
    match p {
        HPat::Ctor { path, .. } => {
            if let Some(n) = path.last() {
                out.insert(n.to_ascii_lowercase());
            }
        }
        HPat::Or { alts, .. } => {
            for a in alts {
                ctor_names(a, out);
            }
        }
        _ => {}
    }
}

fn bool_names(p: &HPat, out: &mut BTreeSet<String>) {
    match p {
        HPat::Lit { lit, .. } if lit.kind == LitKind::Bool => {
            out.insert(lit.text.clone());
        }
        HPat::Or { alts, .. } => {
            for a in alts {
                bool_names(a, out);
            }
        }
        _ => {}
    }
}

fn record_cov(p: &HPat, out: &mut BTreeSet<String>, open: &mut bool) {
    match p {
        HPat::Record {
            fields, open: o, ..
        } => {
            for (n, _) in fields {
                if !n.is_empty() {
                    out.insert(n.clone());
                }
            }
            if *o {
                *open = true;
            }
        }
        HPat::Or { alts, .. } => {
            for a in alts {
                record_cov(a, out, open);
            }
        }
        _ => {}
    }
}

/// Tuple pattern phủ hết tuple `n` thành phần: đúng số thành phần và **mọi** thành phần bất khả bác
/// (`_`, tên trần, hoặc tuple con như vậy).
fn tuple_phu_het(p: &HPat, n: usize) -> bool {
    match p {
        HPat::Tuple { items, .. } => items.len() == n && items.iter().all(bat_kha_bac),
        _ => false,
    }
}

fn bat_kha_bac(p: &HPat) -> bool {
    match p {
        HPat::Wild { .. } | HPat::Bind { .. } => true,
        HPat::Tuple { items, .. } => items.iter().all(bat_kha_bac),
        HPat::Or { alts, .. } => alts.iter().any(bat_kha_bac),
        _ => false,
    }
}

// ---- ST-239 / VD-259 (VD-242 §3.1): or-pattern ----
//
// Hai luật, áp cho **mọi** `HPat::Or` kể cả lồng trong ctor/tuple/list/record:
//   1. Tên bind ở một số alternative nhưng không phải **mọi** alternative, mà được dùng ở
//      guard/thân (hoặc thân `if let`/`while let`/`for`/các câu sau `let`) ⇒ `VS0412`.
//   2. Tên bind ở **mọi** alternative nhưng kiểu khác nhau ⇒ `VS0412` (binding không có một
//      kiểu duy nhất; checker không được lấy kiểu của alternative cuối).

/// `sym → kiểu` mà pattern bind, tính **thuần** từ kiểu chủ thể (không ghi `env`, không chẩn đoán).
/// Dùng để so kiểu giữa các alternative mà không phụ thuộc thứ tự `bind_pat`.
type BindTypes = std::collections::BTreeMap<u32, Type>;

/// Các pattern con của `p` kèm kiểu chủ thể tương ứng (cùng luật với [`Checker::bind_pat`]).
fn pat_children<'p>(p: &'p HPat, subj: &Type, types: &TypeTable) -> Vec<(&'p HPat, Type)> {
    match p {
        HPat::Ctor { path, args, .. } => {
            let name = path.last().cloned().unwrap_or_default();
            let field_tys = ctor_field_types(subj, &name, types);
            args.iter()
                .enumerate()
                .map(|(i, a)| (a, field_tys.get(i).cloned().unwrap_or(Type::Error)))
                .collect()
        }
        HPat::Tuple { items, .. } => {
            let elem = match subj {
                Type::TypeRef { name, args } if name == "tuple" => args.clone(),
                _ => Vec::new(),
            };
            items
                .iter()
                .enumerate()
                .map(|(i, it)| (it, elem.get(i).cloned().unwrap_or(Type::Error)))
                .collect()
        }
        HPat::List { items, rest, .. } => {
            let elem = match subj {
                Type::List(t) => (**t).clone(),
                _ => Type::Error,
            };
            let mut out: Vec<(&HPat, Type)> = items.iter().map(|it| (it, elem.clone())).collect();
            if let Some(r) = rest {
                out.push((r, Type::List(Box::new(elem))));
            }
            out
        }
        HPat::Record { fields, .. } => {
            let id = match subj {
                Type::Record(id) => Some(*id),
                _ => None,
            };
            fields
                .iter()
                .filter_map(|(name, sub)| {
                    sub.as_ref().map(|s| {
                        let fty = id
                            .and_then(|id| types.field(id, name).map(|f| f.ty.clone()))
                            .unwrap_or(Type::Error);
                        (s, fty)
                    })
                })
                .collect()
        }
        HPat::Range { lo, hi, .. } => [lo, hi]
            .into_iter()
            .flatten()
            .map(|p| (p.as_ref(), subj.clone()))
            .collect(),
        HPat::Or { alts, .. } => alts.iter().map(|a| (a, subj.clone())).collect(),
        HPat::Wild { .. } | HPat::Bind { .. } | HPat::Lit { .. } | HPat::Rest { .. } => Vec::new(),
    }
}

/// Kiểu các trường của ctor `name` khi chủ thể là `subj` (`{error}` nếu không suy được).
fn ctor_field_types(subj: &Type, name: &str, types: &TypeTable) -> Vec<Type> {
    match subj {
        Type::Enum(id) => types
            .enum_def(*id)
            .and_then(|d| d.variants.iter().find(|v| v.name == name))
            .map(|v| v.fields.iter().map(|f| f.ty.clone()).collect())
            .unwrap_or_default(),
        Type::Opt(inner) => match name {
            "Some" => vec![(**inner).clone()],
            "None" => Vec::new(),
            _ if cap_dau(name) == "Some" => vec![(**inner).clone()],
            _ => Vec::new(),
        },
        Type::Result(a, b) => match name {
            "Ok" => vec![(**a).clone()],
            "Err" => vec![(**b).clone()],
            _ if cap_dau(name) == "Ok" => vec![(**a).clone()],
            _ if cap_dau(name) == "Err" => vec![(**b).clone()],
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

/// Mọi binding (kể cả lồng) của một pattern kèm kiểu suy từ `subj`.
fn pat_binds_typed(p: &HPat, subj: &Type, types: &TypeTable, out: &mut BindTypes) {
    match p {
        HPat::Bind { sym, .. } => {
            out.insert(*sym, subj.clone());
        }
        HPat::Wild { .. } | HPat::Lit { .. } | HPat::Rest { .. } => {}
        _ => {
            for (child, ty) in pat_children(p, subj, types) {
                pat_binds_typed(child, &ty, types, out);
            }
        }
    }
}

/// Mọi `sym` được **tham chiếu** (đọc/ghi) trong một biểu thức — để biết binding or-pattern
/// nào thực sự được dùng ở thân nhánh. Không đếm binding của pattern lồng (khác `sym`).
fn expr_syms(e: &HExpr, out: &mut BTreeSet<u32>) {
    match e {
        HExpr::Lit { .. } | HExpr::CmdBlock { .. } | HExpr::Error { .. } => {}
        HExpr::Var { sym, .. } => {
            if let Some(id) = sym {
                out.insert(*id);
            }
        }
        HExpr::Bin { lhs, rhs, .. } => {
            expr_syms(lhs, out);
            expr_syms(rhs, out);
        }
        HExpr::Index { base, index, .. } => {
            expr_syms(base, out);
            expr_syms(index, out);
        }
        HExpr::Un { expr, .. }
        | HExpr::Try { expr, .. }
        | HExpr::Ref { expr, .. }
        | HExpr::Await { expr, .. } => expr_syms(expr, out),
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
            expr_syms(callee, out);
            for a in args {
                expr_syms(a, out);
            }
            for (_, a) in named {
                expr_syms(a, out);
            }
        }
        HExpr::Field { base, .. } => expr_syms(base, out),
        HExpr::List { items, .. } | HExpr::Set { items, .. } | HExpr::Tuple { items, .. } => {
            for it in items {
                expr_syms(it, out);
            }
        }
        HExpr::EnumLit { args, .. } => {
            for it in args {
                expr_syms(it, out);
            }
        }
        HExpr::Map { entries, .. } => {
            for (k, v) in entries {
                expr_syms(k, out);
                expr_syms(v, out);
            }
        }
        HExpr::Record { fields, .. } | HExpr::With { fields, .. } => {
            for (_, v) in fields {
                expr_syms(v, out);
            }
            if let HExpr::With { base, .. } = e {
                expr_syms(base, out);
            }
        }
        HExpr::Opt { value, .. } => {
            if let Some(v) = value {
                expr_syms(v, out);
            }
        }
        HExpr::If {
            cond,
            then,
            elifs,
            else_,
            ..
        } => {
            cond_syms(cond, out);
            stmts_syms(then, out);
            for (c, b) in elifs {
                cond_syms(c, out);
                stmts_syms(b, out);
            }
            stmts_syms(else_, out);
        }
        HExpr::Match { subject, arms, .. } => {
            expr_syms(subject, out);
            for arm in arms {
                if let Some(g) = &arm.guard {
                    expr_syms(g, out);
                }
                stmts_syms(&arm.body, out);
            }
        }
        HExpr::Lambda { body, .. } => body_syms(body, out),
        HExpr::ChanNew { cap, .. } => expr_syms(cap, out),
        HExpr::Interp { parts, .. } => {
            for part in parts {
                match part {
                    InterpPart::Str(_) => {}
                    InterpPart::Expr(e) => expr_syms(e, out),
                    InterpPart::ExprSpec { expr, .. } => expr_syms(expr, out),
                }
            }
        }
    }
}

fn cond_syms(c: &HCond, out: &mut BTreeSet<u32>) {
    match c {
        HCond::Expr(e) => expr_syms(e, out),
        HCond::Let { value, .. } => expr_syms(value, out),
    }
}

fn body_syms(b: &HBody, out: &mut BTreeSet<u32>) {
    match b {
        HBody::Expr(e) => expr_syms(e, out),
        HBody::Stmts(ss) => stmts_syms(ss, out),
    }
}

fn stmts_syms(ss: &[HStmt], out: &mut BTreeSet<u32>) {
    for s in ss {
        match s {
            HStmt::Let { init, else_, .. } => {
                expr_syms(init, out);
                if let Some(e) = else_ {
                    stmts_syms(&e.body, out);
                }
            }
            HStmt::Var { init, .. } => expr_syms(init, out),
            HStmt::Assign { target, value, .. } => {
                expr_syms(target, out);
                expr_syms(value, out);
            }
            HStmt::Expr(e) => expr_syms(e, out),
            HStmt::If {
                cond,
                then,
                elifs,
                else_,
                ..
            } => {
                cond_syms(cond, out);
                stmts_syms(then, out);
                for (c, b) in elifs {
                    cond_syms(c, out);
                    stmts_syms(b, out);
                }
                if let Some(b) = else_ {
                    stmts_syms(b, out);
                }
            }
            HStmt::For { iter, body, .. } => {
                expr_syms(iter, out);
                stmts_syms(body, out);
            }
            HStmt::While { cond, body, .. } => {
                cond_syms(cond, out);
                stmts_syms(body, out);
            }
            HStmt::Loop { body, .. } | HStmt::Scope { body, .. } | HStmt::Block { body, .. } => {
                stmts_syms(body, out)
            }
            HStmt::Return { value, .. } => {
                if let Some(v) = value {
                    expr_syms(v, out);
                }
            }
            HStmt::Match { subject, arms, .. } => {
                expr_syms(subject, out);
                for arm in arms {
                    if let Some(g) = &arm.guard {
                        expr_syms(g, out);
                    }
                    stmts_syms(&arm.body, out);
                }
            }
            HStmt::Break { .. } | HStmt::Continue { .. } => {}
        }
    }
}

/// Mọi `sym` được dùng trong một **thân** (đọc/ghi) — use-set cho luật or-pattern.
pub(crate) fn than_uses(body: &[HStmt]) -> BTreeSet<u32> {
    let mut out = BTreeSet::new();
    stmts_syms(body, &mut out);
    out
}

/// Mọi `sym` được dùng trong guard + thân một nhánh `match`.
pub(crate) fn arm_uses(guard: Option<&HExpr>, body: &[HStmt]) -> BTreeSet<u32> {
    let mut out = BTreeSet::new();
    if let Some(g) = guard {
        expr_syms(g, &mut out);
    }
    stmts_syms(body, &mut out);
    out
}

impl<'a> Checker<'a> {
    /// **ST-239 / VD-259 (VD-242 §3.1, ca 9)** — kiểm **mọi** `HPat::Or`, kể cả lồng:
    ///
    /// * một tên bind ở một số alternative nhưng không phải **mọi** alternative mà được dùng
    ///   trong `uses` ⇒ từ chối (`VS0412`);
    /// * một tên bind ở **mọi** alternative nhưng kiểu khác nhau ⇒ từ chối (`VS0412`).
    ///
    /// `subj` là kiểu chủ thể (đầy đủ, chưa bóc) để suy kiểu binding. Không sửa VM để in `2`
    /// (bị VD-242 §3.1 bác trực tiếp).
    pub(crate) fn kiem_or_bind(&mut self, pat: &HPat, subj: &Type, uses: &BTreeSet<u32>) {
        let mut da_bao: BTreeSet<u32> = BTreeSet::new();
        self.kiem_or_bind_sau(pat, subj, uses, &mut da_bao);
    }

    fn kiem_or_bind_sau(
        &mut self,
        pat: &HPat,
        subj: &Type,
        uses: &BTreeSet<u32>,
        da_bao: &mut BTreeSet<u32>,
    ) {
        if let HPat::Or { alts, span } = pat {
            let mut maps: Vec<BindTypes> = Vec::with_capacity(alts.len());
            for alt in alts {
                let mut m = BindTypes::new();
                pat_binds_typed(alt, subj, self.types, &mut m);
                maps.push(m);
            }
            let mut tat_ca: BTreeSet<u32> = BTreeSet::new();
            for m in &maps {
                tat_ca.extend(m.keys().copied());
            }
            // Tên bind ở **mọi** alternative: kiểu phải thống nhất.
            for id in &tat_ca {
                if !maps.iter().all(|m| m.contains_key(id)) {
                    continue;
                }
                let Some(t0) = maps[0].get(id) else {
                    continue;
                };
                let lech = maps[1..]
                    .iter()
                    .filter_map(|m| m.get(id))
                    .find(|t| !self.same(t0, t))
                    .cloned();
                if let Some(t1) = lech {
                    let ten = self.ten_cua(*id);
                    self.diag(
                        MISMATCH,
                        format!(
                            "or-pattern: `{ten}` có kiểu khác nhau giữa các alternative (`{}` ↔ `{}`) — binding phải cùng kiểu (VD-259)",
                            self.types.show(t0),
                            self.types.show(&t1)
                        ),
                        *span,
                    );
                }
            }
            // Tên chỉ bind ở một số alternative: nếu được dùng ⇒ không có giá trị ở nhánh khác.
            for id in &tat_ca {
                if maps.iter().all(|m| m.contains_key(id)) {
                    continue;
                }
                if !uses.contains(id) || !da_bao.insert(*id) {
                    continue;
                }
                let ten = self.ten_cua(*id);
                self.diag(
                    MISMATCH,
                    format!(
                        "or-pattern: `{ten}` được dùng trong thân nhưng không bind ở mọi alternative (VD-242 §3.1)"
                    ),
                    *span,
                );
            }
        }
        for (child, ty) in pat_children(pat, subj, self.types) {
            self.kiem_or_bind_sau(child, &ty, uses, da_bao);
        }
    }

    fn ten_cua(&self, sym: u32) -> String {
        self.symbols
            .name_of(sym)
            .map(str::to_string)
            .unwrap_or_else(|| "?".into())
    }

    // ---- VD-260: pattern phủ hết cho `let`/`for` ----

    /// `let` **không** `else` chỉ nhận pattern phủ hết (`_`, tên trần, tuple/bản ghi toàn
    /// pattern phủ hết, hoặc or-pattern phủ đủ biến thể của kiểu hữu hạn).
    pub(crate) fn kiem_let_phu_het(&mut self, pat: &HPat, subj: &Type) {
        if !self.pat_phu_het(pat, subj) {
            self.diag(
                MISMATCH,
                "`let`: pattern có thể **không khớp** — dùng `let … else`, `if let`, hoặc pattern \
                 phủ hết (`_`/tên trần) (VD-260)",
                pat.span(),
            );
        }
    }

    /// `for` chỉ nhận pattern phủ hết theo **kiểu phần tử** — phần tử không khớp không được
    /// lặng lẽ bỏ qua (interp/VM từng lệch nhau).
    pub(crate) fn kiem_for_phu_het(&mut self, pat: &HPat, subj: &Type) {
        if !self.pat_phu_het(pat, subj) {
            self.diag(
                MISMATCH,
                "`for`: pattern có thể **không khớp** với một số phần tử — dùng `_`/tên trần \
                 (VD-260)",
                pat.span(),
            );
        }
    }

    /// `true` nếu pattern khớp **mọi** giá trị của `subj` (không cần runtime bác).
    fn pat_phu_het(&self, p: &HPat, subj: &Type) -> bool {
        if subj.is_error() || matches!(subj, Type::Never) {
            return true;
        }
        match p {
            HPat::Wild { .. } | HPat::Bind { .. } | HPat::Rest { .. } => true,
            HPat::Or { alts, .. } => self.or_phu_het(alts, subj),
            HPat::Tuple { items, .. } => match subj {
                Type::TypeRef { name, args } if name == "tuple" => {
                    items.len() == args.len()
                        && items
                            .iter()
                            .zip(args.iter())
                            .all(|(p, t)| self.pat_phu_het(p, t))
                }
                Type::Unit => items.is_empty(),
                _ => false,
            },
            HPat::Record { fields, .. } => match subj {
                Type::Record(id) => fields.iter().all(|(n, sp)| match sp {
                    None => true,
                    Some(sp) => match self.types.field(*id, n) {
                        Some(f) => self.pat_phu_het(sp, &f.ty),
                        // Trường lạ đã có `VS0403` ở `bind_pat` — đừng báo thêm.
                        None => true,
                    },
                }),
                _ => true,
            },
            // `[..]`/`[..rest]` khớp mọi list; list cố định hoặc `[a, ..]` là bác được.
            HPat::List { items, rest, .. } => items.is_empty() && rest.is_some(),
            HPat::Lit { .. } | HPat::Range { .. } => false,
            HPat::Ctor { path, args, .. } => {
                let name = path.last().map(String::as_str).unwrap_or("");
                match subj {
                    Type::Enum(id) => {
                        let Some(def) = self.types.enum_def(*id) else {
                            return true;
                        };
                        if def.variants.len() != 1 {
                            return false;
                        }
                        let v = &def.variants[0];
                        // Variant/arity sai đã có `VS0403`/`VS0401` — đừng báo chồng.
                        if v.name != name || args.len() != v.fields.len() {
                            return true;
                        }
                        args.iter()
                            .zip(v.fields.iter())
                            .all(|(a, f)| self.pat_phu_het(a, &f.ty))
                    }
                    // `Some`/`None`/`Ok`/`Err` luôn còn nhánh kia ⇒ bác được khi đứng một mình.
                    Type::Opt(_) | Type::Result(..) => false,
                    _ => true,
                }
            }
        }
    }

    fn or_phu_het(&self, alts: &[HPat], subj: &Type) -> bool {
        if alts.iter().any(|a| self.pat_phu_het(a, subj)) {
            return true;
        }
        let mut cov: BTreeSet<String> = BTreeSet::new();
        for a in alts {
            self.phu_bien_the(a, subj, &mut cov);
        }
        match subj {
            Type::Bool => cov.contains("true") && cov.contains("false"),
            Type::Opt(_) => cov.contains("Some") && cov.contains("None"),
            Type::Result(..) => cov.contains("Ok") && cov.contains("Err"),
            Type::Enum(id) => self
                .types
                .enum_def(*id)
                .map(|d| d.variants.iter().all(|v| cov.contains(&v.name)))
                .unwrap_or(true),
            _ => false,
        }
    }

    /// Gom các biến thể (hoặc `true`/`false`) mà **một vế** của or-pattern phủ hết.
    fn phu_bien_the(&self, p: &HPat, subj: &Type, out: &mut BTreeSet<String>) {
        match p {
            HPat::Ctor { path, args, .. } => {
                let name = path.last().map(String::as_str).unwrap_or("");
                match subj {
                    Type::Opt(inner)
                        if name == "Some"
                            && args.len() == 1
                            && self.pat_phu_het(&args[0], inner) =>
                    {
                        out.insert("Some".into());
                    }
                    Type::Opt(_) if name == "None" && args.is_empty() => {
                        out.insert("None".into());
                    }
                    Type::Result(a, _)
                        if name == "Ok" && args.len() == 1 && self.pat_phu_het(&args[0], a) =>
                    {
                        out.insert("Ok".into());
                    }
                    Type::Result(_, b)
                        if name == "Err" && args.len() == 1 && self.pat_phu_het(&args[0], b) =>
                    {
                        out.insert("Err".into());
                    }
                    Type::Enum(id) => {
                        if let Some(v) = self
                            .types
                            .enum_def(*id)
                            .and_then(|d| d.variants.iter().find(|v| v.name == name))
                        {
                            if args.len() == v.fields.len()
                                && args
                                    .iter()
                                    .zip(v.fields.iter())
                                    .all(|(a, f)| self.pat_phu_het(a, &f.ty))
                            {
                                out.insert(v.name.clone());
                            }
                        }
                    }
                    _ => {}
                }
            }
            HPat::Or { alts, .. } => {
                for a in alts {
                    self.phu_bien_the(a, subj, out);
                }
            }
            HPat::Lit { lit, .. } if lit.kind == LitKind::Bool => {
                out.insert(lit.text.clone());
            }
            _ => {}
        }
    }
}
