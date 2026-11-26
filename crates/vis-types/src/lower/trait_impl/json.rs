// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-318c** — sinh HIR cho `derives Json` (`to_json`/`from_json`) + kiểm kiểu mã hoá được
//! (nguyên văn từ `lower/trait_impl.rs`; 0 đổi hành vi/chẩn đoán).

use super::*;
use vis_diag::TYPE_CODES;
use vis_hir::hir::{HFn, HImpl, HItem, HStmt, Lit, LitKind};
use vis_hir::ty::RecId;
use vis_syntax::NodeKind;

impl<'a> Lowerer<'a> {
    /// **ST-302 (#8)** — lượt **sau** khi `tbl` đã có bản ghi/enum: sinh `to_json`/`from_json`
    /// cho `derives Json` (descriptor lồng cần bảng kiểu đầy đủ).
    pub(super) fn sinh_derives_json(&mut self) {
        use std::collections::HashMap;
        #[derive(Clone)]
        enum Def {
            Rec(Vec<(String, Type)>),
            Enum(Vec<(String, Vec<Type>)>),
        }
        let mut defs: HashMap<String, Def> = HashMap::new();
        for child in self.cst.root.children() {
            if self.generics_of(child).is_empty() {
                match child.kind {
                    NodeKind::TypeDecl => {
                        if let Some(name) = self.decl_name(child) {
                            let fs = child
                                .children()
                                .filter(|n| n.kind == NodeKind::TypeField)
                                .filter_map(|f| {
                                    Some((
                                        self.decl_name(f)?,
                                        f.child(NodeKind::Type).map(|t| self.resolve_type(t))?,
                                    ))
                                })
                                .collect();
                            defs.insert(name, Def::Rec(fs));
                        }
                    }
                    NodeKind::EnumDecl => {
                        if let Some(name) = self.decl_name(child) {
                            let vs = child
                                .children()
                                .filter(|n| n.kind == NodeKind::Variant)
                                .filter_map(|v| {
                                    let vn = self.decl_name(v)?;
                                    let fs = v
                                        .children()
                                        .filter(|n| n.kind == NodeKind::VariantField)
                                        .filter_map(|f| {
                                            f.child(NodeKind::Type).map(|t| self.resolve_type(t))
                                        })
                                        .collect();
                                    Some((vn, fs))
                                })
                                .collect();
                            defs.insert(name, Def::Enum(vs));
                        }
                    }
                    _ => {}
                }
            }
        }
        fn desc(
            ty: &Type,
            defs: &HashMap<String, Def>,
            recs: &[(String, RecId)],
            enums: &[(String, vis_hir::ty::EnumId)],
            depth: usize,
        ) -> String {
            if depth > 16 {
                return "_".into();
            }
            match ty {
                Type::Int => "i".into(),
                Type::Float => "f".into(),
                Type::Str => "s".into(),
                Type::Bool => "b".into(),
                Type::Bytes => "y".into(),
                Type::Path => "h".into(),
                Type::Char => "c".into(),
                Type::Unit => "u".into(),
                Type::Size => "z".into(),
                Type::Duration => "d".into(),
                Type::Percent => "p".into(),
                Type::Opt(t) => format!("O{}", desc(t, defs, recs, enums, depth + 1)),
                Type::List(t) => format!("L{}", desc(t, defs, recs, enums, depth + 1)),
                Type::Map(a, b) => format!(
                    "M{}{}",
                    desc(a, defs, recs, enums, depth + 1),
                    desc(b, defs, recs, enums, depth + 1)
                ),
                Type::Result(a, b) => format!(
                    "R{}{}",
                    desc(a, defs, recs, enums, depth + 1),
                    desc(b, defs, recs, enums, depth + 1)
                ),
                Type::Record(id) => {
                    let name = recs
                        .iter()
                        .find(|(_, i)| *i == *id)
                        .map(|(n, _)| n.clone())
                        .unwrap_or_default();
                    let fs = match defs.get(&name) {
                        Some(Def::Rec(fs)) => fs.clone(),
                        _ => Vec::new(),
                    };
                    let mut out = format!("T{name}{{");
                    for (i, (n, t)) in fs.iter().enumerate() {
                        if i > 0 {
                            out.push(';');
                        }
                        out.push_str(n);
                        out.push(':');
                        out.push_str(&desc(t, defs, recs, enums, depth + 1));
                    }
                    out.push('}');
                    out
                }
                Type::Enum(id) => {
                    let name = enums
                        .iter()
                        .find(|(_, i)| *i == *id)
                        .map(|(n, _)| n.clone())
                        .unwrap_or_default();
                    let vs = match defs.get(&name) {
                        Some(Def::Enum(vs)) => vs.clone(),
                        _ => Vec::new(),
                    };
                    let mut out = format!("E{name}{{");
                    for (i, (vn, fs)) in vs.iter().enumerate() {
                        if i > 0 {
                            out.push(';');
                        }
                        out.push_str(vn);
                        out.push(':');
                        if fs.is_empty() {
                            out.push('_');
                        } else {
                            for (j, t) in fs.iter().enumerate() {
                                if j > 0 {
                                    out.push('|');
                                }
                                out.push_str(&desc(t, defs, recs, enums, depth + 1));
                            }
                        }
                    }
                    out.push('}');
                    out
                }
                Type::TypeRef { name, .. } if name == "Json" => "j".into(),
                _ => "_".into(),
            }
        }
        let mut ra: Vec<HItem> = Vec::new();
        for child in self.cst.root.children() {
            let (name, ty, span) = match child.kind {
                NodeKind::TypeDecl => {
                    let Some(name) = self.decl_name(child) else {
                        continue;
                    };
                    let Some((_, id)) = self.rec_names.iter().find(|(n, _)| *n == name).cloned()
                    else {
                        continue;
                    };
                    (name, Type::Record(id), child.span)
                }
                NodeKind::EnumDecl => {
                    let Some(name) = self.decl_name(child) else {
                        continue;
                    };
                    let Some((_, id)) = self.enum_names.iter().find(|(n, _)| *n == name).cloned()
                    else {
                        continue;
                    };
                    (name, Type::Enum(id), child.span)
                }
                _ => continue,
            };
            let derives = self.derives_of(child);
            if !derives.iter().any(|d| d == "Json") || !self.generics_of(child).is_empty() {
                continue;
            }
            let bad = match defs.get(&name) {
                Some(Def::Rec(fs)) => fs.iter().any(|(_, t)| !self.json_hop_le(t, 0)),
                Some(Def::Enum(vs)) => vs
                    .iter()
                    .any(|(_, fs)| fs.iter().any(|t| !self.json_hop_le(t, 0))),
                None => false,
            };
            if bad {
                self.diag(
                    &TYPE_CODES[4],
                    "derive `Json` không dùng được: trường/biến thể chứa `cap`/`Chan`/`Task`/hàm (không mã hoá được)",
                    child.span,
                );
                continue;
            }
            let d = desc(&ty, &defs, &self.rec_names, &self.enum_names, 0);
            let mut methods = Vec::new();
            if let Some(f) = self.sinh_json_to(&ty, &name, &d, span) {
                methods.push(f);
            }
            if let Some(f) = self.sinh_json_from(&ty, &name, &d, span) {
                methods.push(f);
            }
            if !methods.is_empty() {
                ra.push(HItem::Impl(HImpl {
                    trait_name: None,
                    ty,
                    span,
                    methods,
                }));
            }
        }
        self.derived.extend(ra);
    }

    /// **ST-302 (#8)**: kiểu mã hoá JSON được? Từ chối `cap`/`Chan`/`Task`/`fn` ở mọi độ sâu.
    pub(super) fn json_hop_le(&self, t: &Type, depth: usize) -> bool {
        if depth > 16 {
            return false;
        }
        match t {
            Type::Fn(..) => false,
            Type::TypeRef { name, args } => {
                if matches!(name.as_str(), "cap" | "Chan" | "Task") {
                    return false;
                }
                args.iter().all(|x| self.json_hop_le(x, depth + 1))
            }
            Type::List(x) | Type::Opt(x) => self.json_hop_le(x, depth + 1),
            Type::Map(a, b) | Type::Result(a, b) => {
                self.json_hop_le(a, depth + 1) && self.json_hop_le(b, depth + 1)
            }
            Type::Record(id) => self
                .tbl
                .recs()
                .get(id.0 as usize)
                .map(|r| r.fields.iter().all(|f| self.json_hop_le(&f.ty, depth + 1)))
                .unwrap_or(true),
            Type::Enum(id) => self
                .tbl
                .enums()
                .get(id.0 as usize)
                .map(|e| {
                    e.variants
                        .iter()
                        .all(|v| v.fields.iter().all(|f| self.json_hop_le(&f.ty, depth + 1)))
                })
                .unwrap_or(true),
            _ => true,
        }
    }

    fn lit_str(s: &str, span: vis_syntax::Span) -> HExpr {
        HExpr::Lit {
            lit: Lit {
                kind: LitKind::Str,
                text: format!("{s:?}"),
            },
            ty: Type::Str,
            span,
        }
    }

    fn call_json(name: &str, args: Vec<HExpr>, span: vis_syntax::Span) -> HExpr {
        HExpr::Call {
            callee: Box::new(HExpr::Var {
                name: name.into(),
                sym: None,
                span,
            }),
            args,
            named: Vec::new(),
            span,
        }
    }

    fn sinh_json_to(
        &mut self,
        ty: &Type,
        ten: &str,
        desc: &str,
        span: vis_syntax::Span,
    ) -> Option<HFn> {
        let ty = ty.clone();
        let desc = desc.to_string();
        self.sinh_pt(
            &ty,
            PtSinh {
                ten,
                ten_pt: "to_json",
                ret: Type::TypeRef {
                    name: "Json".into(),
                    args: Vec::new(),
                },
                tham: vec![("self", ty.clone())],
                span,
            },
            move |ids| {
                let me = HExpr::Var {
                    name: "self".into(),
                    sym: Some(ids[0]),
                    span,
                };
                vec![HStmt::Return {
                    value: Some(Self::call_json(
                        "$json:to",
                        vec![me, Self::lit_str(&desc, span)],
                        span,
                    )),
                    span,
                }]
            },
        )
    }

    fn sinh_json_from(
        &mut self,
        ty: &Type,
        ten: &str,
        desc: &str,
        span: vis_syntax::Span,
    ) -> Option<HFn> {
        let ty = ty.clone();
        let desc = desc.to_string();
        let ret = Type::Result(Box::new(ty.clone()), Box::new(Type::Record(RecId(0))));
        let json = Type::TypeRef {
            name: "Json".into(),
            args: Vec::new(),
        };
        self.sinh_pt(
            &ty,
            PtSinh {
                ten,
                ten_pt: "from_json",
                ret,
                tham: vec![("j", json)],
                span,
            },
            move |ids| {
                let j = HExpr::Var {
                    name: "j".into(),
                    sym: Some(ids[0]),
                    span,
                };
                vec![HStmt::Return {
                    value: Some(Self::call_json(
                        "$json:from",
                        vec![j, Self::lit_str(&desc, span)],
                        span,
                    )),
                    span,
                }]
            },
        )
    }
}
