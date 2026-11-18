// SPDX-License-Identifier: MIT OR Apache-2.0
//! Hạ **khai báo item** (fn/type/enum/trait/impl/extern/alias/const) — ST-108 tách từ `lower.rs`;
//! thuần refactor, không đổi logic/thứ tự chẩn đoán.

use super::*;

impl<'a> Lowerer<'a> {
    // ----- lượt 1: khai báo trước -----

    pub(crate) fn predeclare_item(&mut self, node: &Node) {
        match node.kind {
            NodeKind::TypeDecl => {
                let Some(name) = self.decl_name(node) else {
                    return;
                };
                let id = self.declare(&name, SymKind::Item, node.span, None);
                if self.has_token(node, "pub") {
                    if let Some(p) = self.symbols.pub_of.get_mut(id as usize) {
                        *p = true;
                    }
                }
                self.item_syms.push((name.clone(), id));
                let rid = RecId(self.tbl.recs().len() as u32);
                self.tbl.push_rec(RecDef {
                    name: Some(name.clone()),
                    fields: Vec::new(),
                    span: node.span,
                    anonymous: false,
                });
                self.rec_names.push((name, rid));
            }
            NodeKind::EnumDecl => {
                let Some(name) = self.decl_name(node) else {
                    return;
                };
                let id = self.declare(&name, SymKind::Item, node.span, None);
                if self.has_token(node, "pub") {
                    if let Some(p) = self.symbols.pub_of.get_mut(id as usize) {
                        *p = true;
                    }
                }
                self.item_syms.push((name.clone(), id));
                let eid = EnumId(self.tbl.enums().len() as u32);
                self.tbl.push_enum(EnumDef {
                    name: name.clone(),
                    variants: Vec::new(),
                    span: node.span,
                });
                self.enum_names.push((name, eid));
                let mut idx = 0usize;
                for v in node.children().filter(|n| n.kind == NodeKind::Variant) {
                    if let Some(vn) = self.decl_name(v) {
                        self.variant_index.push((vn, eid, idx));
                        idx += 1;
                    }
                }
            }
            NodeKind::FnDecl
            | NodeKind::CmdDecl
            | NodeKind::TestDecl
            | NodeKind::TraitDecl
            | NodeKind::ExternDecl
            | NodeKind::AliasDecl
            | NodeKind::NewtypeDecl
            | NodeKind::ConstDecl => {
                // **R20b-fix (VD-178)**: `test fn tên()` — `test` cũng là `Ident` ⇒ lấy ident **thứ hai**.
                let ten = if node.kind == NodeKind::TestDecl {
                    node.tokens(&self.cst.tokens)
                        .filter(|t| t.kind == Kind::Ident)
                        .nth(1)
                        .map(|t| t.text.clone())
                } else {
                    self.decl_name(node)
                };
                if let Some(name) = ten {
                    let id = self.declare(&name, SymKind::Item, node.span, None);
                    // ST-25: `pub` quyết định tầm nhìn ra module khác (`let`/`const` **chưa** hỗ trợ
                    // `pub` — nợ VS1b, ghi JOURNAL).
                    if self.has_token(node, "pub") {
                        if let Some(p) = self.symbols.pub_of.get_mut(id as usize) {
                            *p = true;
                        }
                    }
                    self.item_syms.push((name, id));
                }
            }
            _ => {}
        }
    }

    // ----- lượt 2: dựng item -----

    pub(crate) fn build_item(&mut self, node: &Node) -> Option<HItem> {
        match node.kind {
            NodeKind::FnDecl => Some(HItem::Fn(self.lower_fn(node, FnKind::Fn))),
            NodeKind::CmdDecl => Some(HItem::Fn(self.lower_fn(node, FnKind::Cmd))),
            NodeKind::TestDecl => Some(HItem::Fn(self.lower_fn(node, FnKind::Test))),
            NodeKind::ExternDecl => Some(HItem::Extern(self.lower_extern(node))),
            NodeKind::TypeDecl => self.lower_rec(node).map(HItem::Rec),
            NodeKind::EnumDecl => self.lower_enum(node).map(HItem::Enum),
            NodeKind::AliasDecl | NodeKind::NewtypeDecl => self.lower_alias(node),
            NodeKind::ConstDecl => self.lower_const(node),
            NodeKind::UseDecl => {
                // ST-25: đường dẫn **tệp tương đối** (`./util`, `./a/b`) giữ nguyên trong **một**
                // đoạn (có `/`); đường dẫn chấm (`std.fs`, `pkg.http`) tách theo `.`.
                let path = node
                    .child(NodeKind::DottedPath)
                    .map(|p| self.text_of(p.span).trim().to_string())
                    .map(|raw| {
                        if raw.contains('/') {
                            vec![raw]
                        } else {
                            raw.split('.').map(str::to_string).collect()
                        }
                    })
                    .unwrap_or_default();
                Some(HItem::Use(HUse {
                    path,
                    span: node.span,
                }))
            }
            NodeKind::TraitDecl => self.lower_trait(node),
            NodeKind::ImplDecl => self.lower_impl(node),
            _ => None,
        }
    }

    pub(crate) fn generics_of(&self, node: &Node) -> Vec<String> {
        node.child(NodeKind::Generics)
            .map(|g| {
                g.children()
                    .filter(|n| n.kind == NodeKind::Generic)
                    .filter_map(|g| self.decl_name(g))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn lower_fn(&mut self, node: &Node, kind: FnKind) -> HFn {
        // **R20b-fix (VD-178)**: `test fn tên()` — `test` cũng là `Ident` ⇒ lấy ident **thứ hai**.
        let name = if kind == FnKind::Test {
            node.tokens(&self.cst.tokens)
                .filter(|t| t.kind == Kind::Ident)
                .nth(1)
                .map(|t| t.text.clone())
                .unwrap_or_default()
        } else {
            self.decl_name(node).unwrap_or_default()
        };
        let generics = self.generics_of(node);
        let is_pub = self.has_token(node, "pub");
        let is_async = self.has_token(node, "async");
        self.push_frame(FrameKind::Fn, node.span);
        let sym = match kind {
            FnKind::Fn | FnKind::Cmd | FnKind::Test => self.item_sym(&name),
            FnKind::ImplMethod => {
                // ST-29: sym đã khai ở **lượt 1b** (để call-site trỏ tới trước khi thân hạ).
                let tim = self.cur_impl.clone().and_then(|t| {
                    self.impls
                        .iter()
                        .find(|(ty, mn, _)| {
                            self.ten_kieu(ty).as_deref() == Some(t.as_str()) && mn == &name
                        })
                        .map(|(_, _, sym)| *sym)
                });
                tim.unwrap_or_else(|| self.declare(&name, SymKind::Item, node.span, None))
            }
            FnKind::TraitMethod => self.declare(&name, SymKind::Item, node.span, None),
        };
        for g in &generics {
            self.declare(g, SymKind::Generic, node.span, None);
        }
        // ST-35: tham số kiểu của **chính hàm** phải có hiệu lực khi hạ chữ ký — nhờ vậy `Hop[T]`
        // trong chú thích giữ được **ký hiệu** `T` (không xoá thành `{error}`) để tyck hợp nhất.
        let luu_gen = std::mem::replace(&mut self.generic_scope, generics.clone());
        // ST-35: trong **chữ ký** hàm generic, `T` giữ dạng **ký hiệu** (không xoá thành `{error}`) để
        // call-site hợp nhất được; thân hàm vẫn dùng bản đã xoá (tyck `erase_gen`).
        let luu_sym = std::mem::replace(&mut self.sym_generic, !generics.is_empty());
        let params = self.lower_params(node);
        let ret = node
            .child(NodeKind::Type)
            .map(|t| self.resolve_type(t))
            .unwrap_or(Type::Unit);
        self.sym_generic = luu_sym;
        self.generic_scope = luu_gen;
        let body = node
            .child(NodeKind::Suite)
            .map(|s| self.lower_suite(s))
            .unwrap_or_default();
        self.pop_frame();
        HFn {
            name,
            span: node.span,
            sym,
            kind,
            is_pub,
            is_async,
            generics,
            params,
            ret,
            body,
        }
    }

    fn lower_params(&mut self, node: &Node) -> Vec<HParam> {
        let Some(params) = node.child(NodeKind::Params) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for p in params.children().filter(|n| n.kind == NodeKind::Param) {
            let is_self = self.has_token(p, "self");
            let is_var_self = is_self && self.has_token(p, "var");
            // `--tên` là tham số **có tên** (docs §5.1) — hợp lệ ở mọi hàm; là cờ ở chế độ lệnh.
            let named = p
                .tokens(&self.cst.tokens)
                .take(2)
                .filter(|t| t.is_punct("-"))
                .count()
                == 2;
            let name = match self.first_ident(p) {
                Some((n, _)) => n,
                None if is_self => "self".to_string(),
                None => continue,
            };
            // ST-27: `...rest: str` — lexer không có `...` ⇒ ba token `..` `.` `rest`.
            let variadic = p
                .tokens(&self.cst.tokens)
                .take(2)
                .filter(|t| t.is_punct("..") || t.is_punct("."))
                .count()
                >= 2
                && p.tokens(&self.cst.tokens).any(|t| t.is_punct(".."));
            let ty = match p.child(NodeKind::Type) {
                Some(t) => self.resolve_type(t),
                None => Type::Error,
            };
            // Giá trị mặc định: node con không phải `Type`/`GrantList` (đối số `@path` là token).
            let default = p
                .children()
                .filter(|c| c.kind != NodeKind::Type && c.kind != NodeKind::GrantList)
                .last()
                .map(|c| self.lower_expr(c));
            // **ST-72 (VD-106 (2))**: `@ p"…"`/`@ "host"` + `(write)` chỉ hợp lệ trên `cap[...]`.
            self.kiem_cap_param(p, &ty);
            // ST-27: tham số variadic nhận **list** phần tử (vd `...rest: str` ⇒ `list[str]`).
            let ty = if variadic {
                Type::List(Box::new(ty))
            } else {
                ty
            };
            let sym = self.declare(&name, SymKind::Param, p.span, Some(ty.clone()));
            // ST-29: kiểu tĩnh của tham số (từ **chú thích**) để dispatch phương thức impl.
            if !matches!(ty, Type::Error) {
                self.type_env.insert(sym, ty.clone());
            }
            out.push(HParam {
                name,
                span: p.span,
                sym,
                ty,
                named,
                is_self,
                is_var_self,
                default,
                variadic,
            });
        }
        out
    }

    fn lower_extern(&mut self, node: &Node) -> HSig {
        let name = self.decl_name(node).unwrap_or_default();
        let sym = self.item_sym(&name);
        self.push_frame(FrameKind::Fn, node.span);
        let params = self.lower_params(node);
        self.pop_frame();
        HSig {
            name,
            span: node.span,
            sym,
            params,
        }
    }

    pub(crate) fn field_of(&mut self, f: &Node) -> HField {
        let name = self.decl_name(f).unwrap_or_default();
        let ty = match f.child(NodeKind::Type) {
            Some(t) => self.resolve_type(t),
            None => Type::Error,
        };
        let default = f
            .children()
            .filter(|c| c.kind != NodeKind::Type)
            .last()
            .map(|c| self.lower_expr(c));
        let has_default = default.is_some();
        HField {
            name,
            span: f.span,
            ty,
            default,
            has_default,
        }
    }

    fn lower_rec(&mut self, node: &Node) -> Option<HRec> {
        let luu_generic = std::mem::take(&mut self.generic_scope);
        self.generic_scope = self.generics_of(node);
        let name = self.decl_name(node)?;
        let sym = self.item_sym(&name);
        let derives = self.derives_of(node);
        let fields: Vec<HField> = node
            .children()
            .filter(|n| n.kind == NodeKind::TypeField)
            .map(|f| self.field_of(f))
            .collect();
        let field_tys: Vec<Type> = fields.iter().map(|f| f.ty.clone()).collect();
        self.kiem_derives(node, &derives, &field_tys);
        // ST-35: dựng **template** (kiểu còn ký hiệu `T`) để instantiation `Hop[int]` thay được.
        if !self.generic_scope.is_empty() {
            self.dung_template_rec(node, &name);
        }
        let id = self
            .rec_names
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, id)| *id)?;
        if let Some(def) = self.tbl.rec_mut(id) {
            def.fields = fields
                .iter()
                .map(|f| FieldDef {
                    name: f.name.clone(),
                    ty: f.ty.clone(),
                    span: f.span,
                    has_default: f.has_default,
                })
                .collect();
        }
        self.generic_scope = luu_generic;
        Some(HRec {
            id,
            name,
            span: node.span,
            sym,
            generics: self.generics_of(node),
            fields,
            derives,
        })
    }

    fn lower_enum(&mut self, node: &Node) -> Option<HEnum> {
        let luu_generic = std::mem::take(&mut self.generic_scope);
        self.generic_scope = self.generics_of(node);
        let name = self.decl_name(node)?;
        let sym = self.item_sym(&name);
        let derives = self.derives_of(node);
        let mut variants: Vec<HVariant> = Vec::new();
        for v in node.children().filter(|n| n.kind == NodeKind::Variant) {
            let vname = self.decl_name(v)?;
            let fields: Vec<HField> = v
                .children()
                .filter(|n| n.kind == NodeKind::VariantField)
                .map(|f| self.field_of(f))
                .collect();
            variants.push(HVariant {
                name: vname,
                span: v.span,
                fields,
            });
        }
        let mut vt: Vec<Type> = Vec::new();
        for v in &variants {
            for f in &v.fields {
                vt.push(f.ty.clone());
            }
        }
        self.kiem_derives(node, &derives, &vt);
        // ST-35: template biến thể (kiểu còn ký hiệu `T`) cho instantiation `Or[int, str]`.
        if !self.generic_scope.is_empty() {
            self.dung_template_enum(node, &name);
        }
        let id = self
            .enum_names
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, id)| *id)?;
        if let Some(def) = self.tbl.enum_mut(id) {
            def.variants = variants
                .iter()
                .map(|v| VariantDef {
                    name: v.name.clone(),
                    fields: v
                        .fields
                        .iter()
                        .map(|f| FieldDef {
                            name: f.name.clone(),
                            ty: f.ty.clone(),
                            span: f.span,
                            has_default: f.has_default,
                        })
                        .collect(),
                    span: v.span,
                })
                .collect();
        }
        self.generic_scope = luu_generic;
        Some(HEnum {
            id,
            name,
            span: node.span,
            sym,
            generics: self.generics_of(node),
            variants,
            derives,
        })
    }

    pub(crate) fn derives_of(&self, node: &Node) -> Vec<String> {
        node.child(NodeKind::Derives)
            .map(|d| {
                d.children()
                    .filter(|n| n.kind == NodeKind::DottedPath)
                    .map(|p| self.dotted_text(p))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn lower_alias(&mut self, node: &Node) -> Option<HItem> {
        let is_newtype = node.kind == NodeKind::NewtypeDecl;
        let name = self.decl_name(node)?;
        let sym = self.item_sym(&name);
        let ty = match node.child(NodeKind::Type) {
            Some(t) => self.resolve_type(t),
            None => Type::Error,
        };
        // `alias` = tên khác của cùng kiểu; `newtype` = kiểu mới (ST-12 phân giải tiếp).
        if !is_newtype {
            self.aliases.push((name.clone(), ty.clone()));
        } else {
            // **R12a (VD-178)**: `type Email(str)` — newtype **trong suốt**.
            self.newtypes.push(name.clone());
        }
        Some(HItem::Alias(HAlias {
            name,
            span: node.span,
            sym,
            ty,
            is_newtype,
        }))
    }

    fn lower_const(&mut self, node: &Node) -> Option<HItem> {
        let name = self.decl_name(node)?;
        let sym = self.item_sym(&name);
        let ty = match node.child(NodeKind::Type) {
            Some(t) => self.resolve_type(t),
            None => Type::Error,
        };
        let value = node
            .children()
            .filter(|c| c.kind != NodeKind::Type)
            .last()
            .map(|c| self.lower_expr(c))
            .unwrap_or(HExpr::Error { span: node.span });
        Some(HItem::Const(HConst {
            name,
            span: node.span,
            sym,
            ty,
            value,
        }))
    }

    fn lower_trait(&mut self, node: &Node) -> Option<HItem> {
        let name = self.decl_name(node)?;
        let sym = self.item_sym(&name);
        let supertraits = node
            .children()
            .filter(|n| n.kind == NodeKind::DottedPath)
            .map(|p| self.dotted_text(p))
            .collect();
        let mut methods = Vec::new();
        for m in node.children().filter(|n| n.kind == NodeKind::TraitItem) {
            methods.push(self.lower_fn(m, FnKind::TraitMethod));
        }
        Some(HItem::Trait(HTrait {
            name,
            span: node.span,
            sym,
            supertraits,
            methods,
        }))
    }

    fn lower_impl(&mut self, node: &Node) -> Option<HItem> {
        let trait_name = node
            .child(NodeKind::DottedPath)
            .map(|p| self.dotted_text(p));
        let ty = node
            .child(NodeKind::Type)
            .map(|t| self.resolve_type(t))
            .unwrap_or(Type::Error);
        let mut methods = Vec::new();
        let truct = self.ten_kieu(&ty);
        self.cur_impl = truct.clone();
        for m in node.children().filter(|n| n.kind == NodeKind::FnDecl) {
            methods.push(self.lower_fn(m, FnKind::ImplMethod));
        }
        self.cur_impl = None;
        self.kiem_impl_khop(node, &trait_name, &ty, &methods);
        Some(HItem::Impl(HImpl {
            trait_name,
            ty,
            span: node.span,
            methods,
        }))
    }
}
