// SPDX-License-Identifier: MIT OR Apache-2.0
//! Luật or-pattern (`VS0412`, ST-239/VD-259): bind ở mọi vế + kiểu thống nhất — tách khỏi
//! `pat.rs` ở `ST-317b`.

use std::collections::{BTreeMap, BTreeSet};

use vis_hir::hir::HPat;
use vis_hir::ty::{Type, TypeTable};

use super::phu::cap_dau;
use crate::tyck::{Checker, MISMATCH};

/// `sym → kiểu` mà pattern bind, tính **thuần** từ kiểu chủ thể (không ghi `env`, không chẩn đoán).
/// Dùng để so kiểu giữa các alternative mà không phụ thuộc thứ tự `bind_pat`.
type BindTypes = BTreeMap<u32, Type>;

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
}
