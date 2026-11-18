// SPDX-License-Identifier: MIT OR Apache-2.0
//! Pattern: gán kiểu cho binding + **xét hết** `match` (`VS0402`, VD-43 #5).
//!
//! Luật xét hết: enum ⇒ **mọi** biến thể; `bool` ⇒ `true`/`false`; `T?` ⇒ `Some`/`None`
//! (`Some`/`None` cũng được); `Result` ⇒ `ok`/`err`; bản ghi ⇒ **đủ mọi trường, không `..`**;
//! kiểu vô hạn (`int`, `str`, `list`, `map`, …) ⇒ **bắt buộc `_`**; **guard không tính**;
//! or-pattern tính **cả hai** vế.

use std::collections::BTreeSet;

use vis_syntax::Span;

use crate::hir::{HArm, HPat, LitKind};
use crate::ty::Type;

use super::{Checker, ARG_COUNT, MISMATCH, NON_EXHAUSTIVE, NO_FIELD};

impl<'a> Checker<'a> {
    /// Gán kiểu cho các tên trong pattern; kiểm literal/ctor khớp `subj`.
    pub(crate) fn bind_pat(&mut self, pat: &HPat, subj: &Type) {
        match pat {
            HPat::Wild { .. } => {}
            HPat::Rest { .. } => {}
            HPat::Bind { sym, .. } => self.set_ty(*sym, subj.clone()),
            HPat::Lit { lit, span } => {
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
                let id = match subj {
                    Type::Record(id) => Some(*id),
                    _ => None,
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
