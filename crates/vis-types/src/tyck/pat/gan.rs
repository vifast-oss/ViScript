// SPDX-License-Identifier: MIT OR Apache-2.0
//! Gán kiểu cho binding trong pattern + kiểm literal/ctor khớp chủ thể — tách khỏi `pat.rs` ở
//! `ST-317b`.

use std::collections::BTreeSet;

use vis_syntax::Span;

use vis_hir::hir::HPat;
use vis_hir::ty::Type;

use super::phu::cap_dau;
use crate::tyck::{Checker, ARG_COUNT, MISMATCH, NO_FIELD};

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
        fields: &[vis_hir::ty::FieldDef],
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
}
