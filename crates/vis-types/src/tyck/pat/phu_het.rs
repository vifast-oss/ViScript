// SPDX-License-Identifier: MIT OR Apache-2.0
//! Pattern phủ hết cho `let`/`for` (`VS0401`-ngữ nghĩa, VD-260) — tách khỏi `pat.rs` ở `ST-317b`.

use std::collections::BTreeSet;

use vis_hir::hir::{HPat, LitKind};
use vis_hir::ty::Type;

use crate::tyck::{Checker, MISMATCH};

impl<'a> Checker<'a> {
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
