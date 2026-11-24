// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-265** — pre-pass **chữ ký** trước khi hạ thân: template `type`/`enum` generic + kiểu
//! tham số/trả về của **mọi** hàm, phương thức `impl`, `cmd`, `test`, `extern`.
//!
//! Trước bản sửa, `fn_params` chỉ có hàm **đã hạ** ⇒ call-site trước khai báo không biết kiểu
//! tham số (`nhan(Box(v: 5))` với `fn nhan(b: Box[int?])` mất instantiation); kiểu trả về của
//! hàm khai sau cũng chưa có cho biểu thức `?`/giá trị hàm.
//!
//! Pre-pass **không** phát chẩn đoán (chụp/khôi phục `diags`) và **không** hạ thân/biểu thức
//! mặc định. Nó chỉ:
//! 1. dựng **template** + **khung** trường/biến thể rỗng kiểu (`{error}`) cho `type`/`enum`
//!    generic — lượt hạ item sau đó ghi đè khung bằng kiểu đã xoá ký hiệu như cũ;
//! 2. ghi `fn_params`/`fn_ret` để call-site (kể cả **trước** khai báo) mang instantiation.
//!
//! Ghi `alias`/`newtype` sớm để template thấy alias khai trước nó; `lower_alias` thay bản ghi
//! cùng tên thay vì nhân đôi.

use vis_syntax::{Node, NodeKind};

use super::Lowerer;
use vis_hir::hir::FnKind;
use vis_hir::ty::{FieldDef, Type, VariantDef};

impl<'a> Lowerer<'a> {
    /// Chạy **một** pre-pass trên item cấp tệp; gọi từ [`Lowerer::run`] sau lượt khai báo trước.
    pub(crate) fn chuan_bi_chu_ky(&mut self) {
        let luu_diag = self.diags.len();
        let con: Vec<Node> = self.cst.root.children().cloned().collect();
        // Pha A — `alias`/`newtype`/`type`/`enum` theo **đúng thứ tự nguồn** (alias trước dùng).
        for node in &con {
            match node.kind {
                NodeKind::AliasDecl | NodeKind::NewtypeDecl => self.chuan_bi_alias(node),
                NodeKind::TypeDecl => self.chuan_bi_rec(node),
                NodeKind::EnumDecl => self.chuan_bi_enum(node),
                _ => {}
            }
        }
        // Pha B — chữ ký hàm/phương thức (template đã sẵn ⇒ instantiation dựng được).
        for node in &con {
            match node.kind {
                NodeKind::FnDecl => self.chuan_bi_ham(node, FnKind::Fn),
                NodeKind::CmdDecl => self.chuan_bi_ham(node, FnKind::Cmd),
                NodeKind::TestDecl => self.chuan_bi_ham(node, FnKind::Test),
                NodeKind::ExternDecl => self.chuan_bi_ngoai(node),
                NodeKind::ImplDecl => self.chuan_bi_impl(node),
                _ => {}
            }
        }
        // Pre-pass chỉ **dò** kiểu; chẩn đoán thật do lượt hạ item phát (đúng thứ tự nguồn).
        self.diags.truncate(luu_diag);
    }

    fn chuan_bi_alias(&mut self, node: &Node) {
        let Some(name) = self.decl_name(node) else {
            return;
        };
        let ty = node
            .child(NodeKind::Type)
            .map(|t| self.resolve_type(t))
            .unwrap_or(Type::Error);
        if node.kind == NodeKind::NewtypeDecl {
            if !self.newtypes.iter().any(|n| n == &name) {
                self.newtypes.push(name);
            }
        } else if let Some(e) = self.aliases.iter_mut().find(|(n, _)| n == &name) {
            e.1 = ty;
        } else {
            self.aliases.push((name, ty));
        }
    }

    fn chuan_bi_rec(&mut self, node: &Node) {
        let Some(name) = self.decl_name(node) else {
            return;
        };
        let generics = self.generics_of(node);
        if generics.is_empty() {
            return;
        }
        let luu_gen = std::mem::replace(&mut self.generic_scope, generics.clone());
        let luu_sym = std::mem::replace(&mut self.sym_generic, true);
        let mut defs: Vec<FieldDef> = Vec::new();
        let mut tys: Vec<Type> = Vec::new();
        for f in node.children().filter(|n| n.kind == NodeKind::TypeField) {
            let ten = self.decl_name(f).unwrap_or_default();
            let ty = f
                .child(NodeKind::Type)
                .map(|t| self.resolve_type(t))
                .unwrap_or(Type::Error);
            // Khung **rỗng kiểu** (như `sym_generic = false` sẽ cho `T` ⇒ `{error}`); lượt hạ
            // `lower_rec` ghi đè bằng kiểu thật. `inst_rec` chỉ cần **tên/số trường** từ khung.
            defs.push(FieldDef {
                name: ten,
                ty: Type::Error,
                span: f.span,
                has_default: f
                    .children()
                    .filter(|c| c.kind != NodeKind::Type)
                    .last()
                    .is_some(),
            });
            tys.push(ty);
        }
        self.sym_generic = luu_sym;
        self.generic_scope = luu_gen;
        if let Some(id) = self
            .rec_names
            .iter()
            .find(|(n, _)| n == &name)
            .map(|(_, id)| *id)
        {
            if let Some(def) = self.tbl.rec_mut(id) {
                def.fields = defs;
            }
        }
        self.dung_template_rec_tu(&name, generics, tys);
    }

    fn chuan_bi_enum(&mut self, node: &Node) {
        let Some(name) = self.decl_name(node) else {
            return;
        };
        let generics = self.generics_of(node);
        if generics.is_empty() {
            return;
        }
        let luu_gen = std::mem::replace(&mut self.generic_scope, generics.clone());
        let luu_sym = std::mem::replace(&mut self.sym_generic, true);
        let mut variants: Vec<VariantDef> = Vec::new();
        let mut vs: Vec<(String, Vec<Type>)> = Vec::new();
        for v in node.children().filter(|n| n.kind == NodeKind::Variant) {
            let Some(vn) = self.decl_name(v) else {
                continue;
            };
            let mut fdefs: Vec<FieldDef> = Vec::new();
            let mut ftys: Vec<Type> = Vec::new();
            for f in v.children().filter(|n| n.kind == NodeKind::VariantField) {
                fdefs.push(FieldDef {
                    name: self.decl_name(f).unwrap_or_default(),
                    ty: Type::Error,
                    span: f.span,
                    has_default: false,
                });
                ftys.push(
                    f.child(NodeKind::Type)
                        .map(|t| self.resolve_type(t))
                        .unwrap_or(Type::Error),
                );
            }
            variants.push(VariantDef {
                name: vn.clone(),
                fields: fdefs,
                span: v.span,
            });
            vs.push((vn, ftys));
        }
        self.sym_generic = luu_sym;
        self.generic_scope = luu_gen;
        if let Some(id) = self
            .enum_names
            .iter()
            .find(|(n, _)| n == &name)
            .map(|(_, id)| *id)
        {
            if let Some(def) = self.tbl.enum_mut(id) {
                def.variants = variants;
            }
        }
        self.dung_template_enum_tu(&name, generics, vs);
    }

    fn chuan_bi_ham(&mut self, node: &Node, kind: FnKind) {
        // **R20b-fix (VD-178)**: `test fn tên()` — ident thứ hai.
        let name = if kind == FnKind::Test {
            node.tokens(&self.cst.tokens)
                .filter(|t| t.kind == vis_syntax::Kind::Ident)
                .nth(1)
                .map(|t| t.text.clone())
                .unwrap_or_default()
        } else {
            self.decl_name(node).unwrap_or_default()
        };
        let Some(sym) = self.item_sym_cua(&name) else {
            return;
        };
        let generics = self.generics_of(node);
        let (params, ret) = self.chu_ky_ham(node, &generics);
        self.fn_params.insert(sym, params);
        self.fn_ret.insert(sym, ret);
    }

    fn chuan_bi_ngoai(&mut self, node: &Node) {
        let Some(sym) = self.item_sym_cua(&self.decl_name(node).unwrap_or_default()) else {
            return;
        };
        let generics = self.generics_of(node);
        let (params, _ret) = self.chu_ky_ham(node, &generics);
        self.fn_params.insert(sym, params);
    }

    fn chuan_bi_impl(&mut self, node: &Node) {
        let ty = node
            .child(NodeKind::Type)
            .map(|t| self.resolve_type(t))
            .unwrap_or(Type::Error);
        let ten = self.ten_kieu(&ty);
        // **VD-270**: `impl Hop[T]` — `T` thuộc chữ ký mọi phương thức (giữ ký hiệu để hợp nhất).
        let impl_generics = self.impl_generics_of(node);
        for m in node.children().filter(|n| n.kind == NodeKind::FnDecl) {
            let Some(mn) = self.decl_name(m) else {
                continue;
            };
            let Some(sym) = self
                .impls
                .iter()
                .find(|(t, n, _)| n == &mn && self.ten_kieu(t).as_deref() == ten.as_deref())
                .map(|(_, _, sym)| *sym)
            else {
                continue;
            };
            let mut generics = self.generics_of(m);
            for g in &impl_generics {
                if !generics.contains(g) {
                    generics.push(g.clone());
                }
            }
            let (params, ret) = self.chu_ky_ham(m, &generics);
            self.fn_params.insert(sym, params);
            self.fn_ret.insert(sym, ret);
        }
    }

    /// `(name, type)` tham số + kiểu trả về — **không** khai ký hiệu/khung, **không** hạ giá trị
    /// mặc định (khác [`Lowerer::lower_params`]).
    fn chu_ky_ham(&mut self, node: &Node, generics: &[String]) -> (Vec<(String, Type)>, Type) {
        let luu_gen = std::mem::replace(&mut self.generic_scope, generics.to_vec());
        let luu_sym = std::mem::replace(&mut self.sym_generic, !generics.is_empty());
        let mut out = Vec::new();
        if let Some(ps) = node.child(NodeKind::Params) {
            for p in ps.children().filter(|n| n.kind == NodeKind::Param) {
                let is_self = self.has_token(p, "self");
                let name = match self.first_ident(p) {
                    Some((n, _)) => n,
                    None if is_self => "self".to_string(),
                    None => continue,
                };
                let ty = p
                    .child(NodeKind::Type)
                    .map(|t| self.resolve_type(t))
                    .unwrap_or(Type::Error);
                let variadic = p
                    .tokens(&self.cst.tokens)
                    .take(2)
                    .filter(|t| t.is_punct("..") || t.is_punct("."))
                    .count()
                    >= 2
                    && p.tokens(&self.cst.tokens).any(|t| t.is_punct(".."));
                let ty = if variadic {
                    Type::List(Box::new(ty))
                } else {
                    ty
                };
                out.push((name, ty));
            }
        }
        let ret = node
            .child(NodeKind::Type)
            .map(|t| self.resolve_type(t))
            .unwrap_or(Type::Unit);
        self.sym_generic = luu_sym;
        self.generic_scope = luu_gen;
        (out, ret)
    }

    /// Sym **item cấp tệp** theo tên; `None` nếu chưa khai (không dùng fallback `0`).
    fn item_sym_cua(&self, name: &str) -> Option<u32> {
        self.item_syms
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .map(|(_, id)| *id)
    }
}
