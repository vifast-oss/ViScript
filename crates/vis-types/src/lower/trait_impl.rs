// SPDX-License-Identifier: MIT OR Apache-2.0
//! **trait / `impl` / `derives`** (ST-29) — tách khỏi `lower.rs` cho luật R1 (≤ 1.500 dòng/tệp).
//!
//! Dispatch **tĩnh**: `x.m(…)` với `x` kiểu tĩnh `Record`/`Enum` ⇒ hạ thẳng `Call(impl_fn)` (chèn `self`
//! đầu) ⇒ interp và VM dùng **cùng HIR** (không bảng `impls`, không bump `VBC1`).
//! `derives Eq/Show/Json` = **kiểm + đăng ký**, ngữ nghĩa dùng cái có sẵn.

use vis_diag::TYPE_CODES;
use vis_syntax::{Node, NodeKind};

use super::{FrameKind, ImplDangKy, Lowerer, SymKind};
use vis_hir::hir::{BinOp, HExpr, HFn, HImpl, HItem, HParam, HStmt, InterpPart, Lit, LitKind};
use vis_hir::ty::{RecId, Type};

mod sieu;

/// Chữ ký một phương thức trait: `(tên, số tham số, kiểu trả, kiểu từng tham số)` (ST-29).
pub type PhuongThucTrait = (String, usize, Type, Vec<Type>);

/// **ST-57 (VD-98 (1))** — chữ ký **công khai** của `trait`: dùng để **xuất qua module**
/// (`pub_traits`) và kiểm `impl`/siêu trait ở tệp khác. Thay tuple `ChuKyTrait` cũ (thêm
/// `supertraits`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitSig {
    pub name: String,
    /// Siêu trait khai `trait T: A, B:` — kiểm **một tầng** (VD-98 (2)).
    pub supertraits: Vec<String>,
    pub methods: Vec<PhuongThucTrait>,
}

impl<'a> Lowerer<'a> {
    // ----- ST-29: trait / impl (dispatch tĩnh) -----

    /// Tên kiểu để tra `impl` (record/enum đã khai) — `None` nếu kiểu không phải record/enum.
    pub(crate) fn ten_kieu(&self, ty: &Type) -> Option<String> {
        match ty {
            Type::Record(id) => self
                .tbl
                .rec(*id)
                .and_then(|r| r.name.clone())
                .or_else(|| Some(ty.name())),
            Type::Enum(id) => self
                .tbl
                .enum_def(*id)
                .map(|e| e.name.clone())
                .or_else(|| Some(format!("enum#{}", id.0))),
            // **VD-270**: `impl Hop[T]` giữ kiểu nhận dạng `TypeRef` (đối số còn ký hiệu) —
            // vẫn phải quy về **tên kiểu gốc** để đăng ký/khớp dispatch với `Hop[int]`.
            // Chỉ nhận tên đã khai `type`/`enum`; `TypeRef{"T"}` (tham số kiểu) vẫn `None`.
            Type::TypeRef { name, .. }
                if self.rec_names.iter().any(|(n, _)| n == name)
                    || self.enum_names.iter().any(|(n, _)| n == name) =>
            {
                Some(name.clone())
            }
            _ => None,
        }
    }

    /// **Kiểu tĩnh** của một biểu thức đã hạ — chỉ dùng chú thích tường minh (không suy diễn).
    pub(crate) fn static_ty(&self, e: &HExpr) -> Option<Type> {
        match e {
            HExpr::Var { sym: Some(id), .. } => self
                .type_env
                .get(id)
                .filter(|t| !t.is_error())
                .cloned()
                .or_else(|| self.self_ty.get(id).cloned())
                .or_else(|| {
                    self.symbols
                        .get(*id)
                        .and_then(|s| s.ty.clone())
                        .filter(|t| !t.is_error())
                }),
            HExpr::Field {
                base,
                name,
                optional: false,
                ..
            } => match self.static_ty(base)? {
                Type::Record(id) => self.tbl.field(id, name).map(|f| f.ty.clone()),
                _ => None,
            },
            // **VD-265**: đích `xs[i]` — phần tử `list`/`map` biết kiểu từ annot của `xs`.
            HExpr::Index { base, .. } => match self.static_ty(base)? {
                Type::List(e) => Some(*e),
                Type::Map(_, v) => Some(*v),
                _ => None,
            },
            // **VD-265**: kết quả gọi hàm/phương thức đã biết chữ ký (pre-pass) — nhờ vậy
            // `tao().lay(Box(v: 5))` / `tao() with f: Box(v: 5)` vẫn mang instantiation.
            HExpr::Call { callee, .. } => match &**callee {
                HExpr::Var { sym: Some(id), .. } => {
                    self.fn_ret
                        .get(id)
                        .cloned()
                        .or_else(|| match self.type_env.get(id) {
                            Some(Type::Fn(_, r)) => Some((**r).clone()),
                            _ => None,
                        })
                }
                HExpr::Field {
                    base,
                    name,
                    optional: false,
                    ..
                } => {
                    let ty = self.static_ty(base)?;
                    let sym = self.method_sym(&ty, name)?;
                    self.fn_ret.get(&sym).cloned()
                }
                _ => None,
            },
            HExpr::Lit { ty, .. }
            | HExpr::Record { ty, .. }
            | HExpr::EnumLit { ty, .. }
            | HExpr::Opt { ty, .. }
            | HExpr::Tuple { ty, .. } => Some(ty.clone()),
            _ => None,
        }
    }

    /// **VD-265** — sym phương thức `impl` khi kiểu bên nhận là `ty` và tên khớp **đúng một**
    /// impl (giống bộ lọc của [`Lowerer::dispatch_tinh`]).
    pub(crate) fn method_sym(&self, ty: &Type, name: &str) -> Option<u32> {
        let ten = self.ten_kieu(ty)?;
        let mut khop = self.khop_impl(&ten, name).into_iter();
        let first = khop.next()?;
        if khop.next().is_some() {
            None
        } else {
            Some(first)
        }
    }

    /// **VD-270** — đăng ký một phương thức `impl` vào bảng **cục bộ** của tệp và bảng **chung**
    /// của đơn vị biên dịch (để module khác dispatch được). Bản ghi chung khử trùng theo
    /// `(module, sym)` — tệp bị hạ lại không nhân đôi.
    pub(crate) fn dang_ky_impl(&mut self, ty: &Type, method: &str, sym: u32) {
        self.impls.push((ty.clone(), method.to_string(), sym));
        if self
            .symbols
            .impls
            .iter()
            .any(|r| r.sym == sym && r.mod_id == self.opts.mod_id)
        {
            return;
        }
        self.symbols.impls.push(ImplDangKy {
            ty: ty.clone(),
            method: method.to_string(),
            sym,
            mod_id: self.opts.mod_id,
        });
    }

    /// **VD-270** — các `sym` phương thức `name` cho kiểu tên `ten`: bảng **cục bộ** + bảng
    /// **chung** của module khác. Giữ **cả** trùng lặp để `dispatch_tinh` còn báo mơ hồ.
    pub(crate) fn khop_impl(&self, ten: &str, name: &str) -> Vec<u32> {
        let mut ra: Vec<u32> = self
            .impls
            .iter()
            .filter(|(t, n, _)| n == name && self.ten_kieu(t).as_deref() == Some(ten))
            .map(|(_, _, sym)| *sym)
            .collect();
        for r in &self.symbols.impls {
            if r.mod_id == self.opts.mod_id || r.method != name {
                continue;
            }
            if self.ten_kieu(&r.ty).as_deref() == Some(ten) {
                ra.push(r.sym);
            }
        }
        ra
    }

    /// Lượt 1b — thu thập `trait` (tên + **siêu trait** + chữ ký phương thức).
    pub(crate) fn collect_trait(&mut self, node: &Node) {
        let Some(name) = self.decl_name(node) else {
            return;
        };
        // **ST-57**: `trait T: A, B:` — các `DottedPath` trước dấu `:` cuối (parser đã sẵn).
        let supertraits: Vec<String> = node
            .children()
            .filter(|n| n.kind == NodeKind::DottedPath)
            .map(|p| self.dotted_text(p))
            .collect();
        let mut methods = Vec::new();
        for m in node.children().filter(|n| n.kind == NodeKind::TraitItem) {
            let Some(mn) = self.decl_name(m) else {
                continue;
            };
            let params: Vec<Type> = m
                .child(NodeKind::Params)
                .map(|p| {
                    p.children()
                        .filter(|n| n.kind == NodeKind::Param)
                        .map(|p| {
                            p.child(NodeKind::Type)
                                .map(|t| self.resolve_type(t))
                                .unwrap_or(Type::Error)
                        })
                        .collect()
                })
                .unwrap_or_default();
            let ret = m
                .child(NodeKind::Type)
                .map(|t| self.resolve_type(t))
                .unwrap_or(Type::Unit);
            methods.push((mn, params.len(), ret, params));
        }
        self.traits.push(TraitSig {
            name,
            supertraits,
            methods,
        });
    }

    /// Lượt 1b — thu thập `impl`: khai sym cho từng phương thức + ghi bảng `(kiểu, tên) → sym`.
    pub(crate) fn collect_impl(&mut self, node: &Node) {
        let ty = node
            .child(NodeKind::Type)
            .map(|t| self.resolve_type(t))
            .unwrap_or(Type::Error);
        // **ST-57 (VD-98 (2))**: ghi cặp `(kiểu, trait)` để kiểm **siêu trait** sau vòng 1b.
        if let Some(p) = node.child(NodeKind::DottedPath) {
            let tn = self.dotted_text(p);
            self.impl_pairs.push((ty.clone(), tn, node.span));
        }
        for m in node.children().filter(|n| n.kind == NodeKind::FnDecl) {
            let Some(mn) = self.decl_name(m) else {
                continue;
            };
            let sym = self.declare(&mn, SymKind::Item, m.span, None);
            self.dang_ky_impl(&ty, &mn, sym);
        }
    }

    /// Kiểm `derives` (ST-29): chỉ `Eq`/`Show`/`Json`; `Json` không dùng được với trường/biến thể
    /// kiểu **hàm** (closure không mã hoá được).
    pub(crate) fn kiem_derives(&mut self, node: &Node, derives: &[String], field_tys: &[Type]) {
        for d in derives {
            let hop_le = matches!(d.as_str(), "Eq" | "Show" | "Json");
            if !hop_le {
                self.diag(
                    &TYPE_CODES[2],
                    format!("không có `derives {d}` (chỉ có `Eq`, `Show`, `Json`)"),
                    node.span,
                );
                continue;
            }
            // **ST-57 (VD-93 (3))**: chưa **sinh mã** derive cho kiểu **generic** (cần dispatch theo
            // `T` — VS1b) ⇒ chẩn đoán **tĩnh**, không sinh.
            if !self.generics_of(node).is_empty() {
                self.diag(
                    &TYPE_CODES[2],
                    format!(
                        "derive `{d}` trên kiểu **generic** chưa hỗ trợ (không sinh mã — VD-98)"
                    ),
                    node.span,
                );
                break;
            }
            // Trường `float`/`Fn` với `Eq`: `==` trên số thực là **không toàn phần** ⇒ `VS0403` **tĩnh**.
            if d == "Eq" {
                for t in field_tys {
                    // **ST-60c (B5)**: đệ quy **sâu** (list/map/opt/result/tuple/generic).
                    if co_float_sau(t) {
                        self.diag(
                            &TYPE_CODES[2],
                            format!(
                                "derive `Eq` không dùng được: trường `{}` (số thực/hàm so sánh không toàn phần — VD-93)",
                                self.tbl.show(t)
                            ),
                            node.span,
                        );
                        break;
                    }
                }
            }
            if d == "Json" {
                // **ST-302 (#8)**: từ chối `cap[...]`/`Chan` ngay cả khi tầng kiểu chưa phân
                // giải (permissive lower giữ `{error}`); kiểm cả văn bản khai báo.
                let txt = self.text_of(node.span);
                if txt.contains("cap[") || txt.contains("Chan[") {
                    self.diag(
                        &TYPE_CODES[4],
                        "derive `Json` không dùng được: trường/biến thể chứa `cap`/`Chan` (không mã hoá được)",
                        node.span,
                    );
                    continue;
                }
                for t in field_tys {
                    if !self.json_hop_le(t, 0) {
                        self.diag(
                            &TYPE_CODES[4],
                            "derive `Json` không dùng được: trường/biến thể chứa `cap`/`Chan`/`Task`/hàm (không mã hoá được)",
                            node.span,
                        );
                        break;
                    }
                }
            }
        }
    }

    /// Kiểm `impl Trait for Type` **khớp** chữ ký trait (ST-29): thiếu/thừa ⇒ `VS0403`; sai số đối số
    /// ⇒ `VS0401`; sai kiểu ⇒ `VS0412`; trait/kiểu ngoài tệp ⇒ mồ côi `VS0403` (§5.4).
    pub(crate) fn kiem_impl_khop(
        &mut self,
        node: &Node,
        trait_name: &Option<String>,
        ty: &Type,
        methods: &[HFn],
    ) {
        let ten_kieu = self.ten_kieu(ty);
        match trait_name {
            None => {
                // `impl Type:` — inherent: không cần trait, nhưng kiểu phải **khai trong tệp**.
                if ten_kieu.is_none() {
                    self.diag(
                        &TYPE_CODES[2],
                        "impl cho kiểu không phải record/enum khai trong tệp (VS1b)",
                        node.span,
                    );
                }
            }
            Some(tn) => {
                // **ST-57 (VD-98 (1))**: trait nội tệp **hoặc** trait `pub` nhập từ module khác.
                let noi_tep = self.traits.iter().find(|t| &t.name == tn).cloned();
                let nhap = self.traits_ngoai.iter().find(|t| &t.name == tn).cloned();
                let Some(sig) = noi_tep.clone().or(nhap) else {
                    self.diag(
                        &TYPE_CODES[2],
                        format!(
                            "không có trait `{tn}` trong **chương trình** (mồ côi **thật** hoặc trait chưa khai `pub`/chưa `use`; VD-98)"
                        ),
                        node.span,
                    );
                    return;
                };
                // Mồ côi **thật** = trait **nhập** *và* kiểu **nhập** ⇒ `impl` phải ở tệp khai một trong hai.
                if noi_tep.is_none() {
                    if let Some(tk) = self.ten_kieu(ty) {
                        if self.kieu_ngoai.contains(&tk) {
                            self.diag(
                                &TYPE_CODES[2],
                                format!(
                                    "mồ côi: trait `{tn}` và kiểu `{tk}` đều ở module khác — `impl` phải ở tệp khai trait hoặc kiểu (VD-98)"
                                ),
                                node.span,
                            );
                            return;
                        }
                    }
                }
                let mong = sig.methods;
                // Mồ côi: `Type` không khai trong tệp ⇒ phải có trait khai trong tệp (đã có).
                let co: Vec<&str> = methods.iter().map(|m| m.name.as_str()).collect();
                for (mn, _, _, _) in &mong {
                    if !co.contains(&mn.as_str()) {
                        self.diag(
                            &TYPE_CODES[2],
                            format!("`impl {tn} for …` thiếu phương thức `{mn}` của trait"),
                            node.span,
                        );
                    }
                }
                for m in methods {
                    let Some((_, so_mong, ret_mong, params_mong)) =
                        mong.iter().find(|(n, _, _, _)| n == &m.name)
                    else {
                        self.diag(
                            &TYPE_CODES[2],
                            format!(
                                "`impl {tn} for …` có phương thức `{}` không thuộc trait",
                                m.name
                            ),
                            m.span,
                        );
                        continue;
                    };
                    let so = m.params.len();
                    if so != *so_mong {
                        self.diag(
                            &TYPE_CODES[0],
                            format!(
                                "`{}.{}` cần {} đối số như trait `{tn}`, được {so}",
                                ten_kieu.clone().unwrap_or_default(),
                                m.name,
                                so_mong
                            ),
                            m.span,
                        );
                        continue;
                    }
                    for (i, p) in m.params.iter().enumerate() {
                        let mong = &params_mong[i];
                        if !matches!(p.ty, Type::Error)
                            && !matches!(mong, Type::Error)
                            && p.ty != *mong
                        {
                            self.diag(
                                &TYPE_CODES[4],
                                format!(
                                    "`{}.{}` đối số {} sai kiểu: trait cần `{:?}`, impl có `{:?}`",
                                    ten_kieu.clone().unwrap_or_default(),
                                    m.name,
                                    i + 1,
                                    mong,
                                    p.ty
                                ),
                                m.span,
                            );
                        }
                    }
                    if !matches!(m.ret, Type::Error)
                        && !matches!(ret_mong, Type::Error)
                        && m.ret != ret_mong.clone()
                    {
                        self.diag(
                            &TYPE_CODES[4],
                            format!(
                                "`{}.{}` sai kiểu trả: trait cần `{:?}`, impl có `{:?}`",
                                ten_kieu.clone().unwrap_or_default(),
                                m.name,
                                ret_mong,
                                m.ret
                            ),
                            m.span,
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ST-57 (VD-93/VD-98 (3)) — `derives` **sinh HIR** ở lượt 1c
// ---------------------------------------------------------------------------

impl<'a> Lowerer<'a> {
    /// Lượt **1c** — sinh `impl` **thật** cho `derives Show`/`Eq` của mọi `type`/`enum` trong tệp.
    ///
    /// Chạy **sau** 1b và **trước** mọi lượt hạ thân ⇒ `p.show()` ở câu **trước** khai báo vẫn phân
    /// giải được (chỉ số sym + bảng `impls` đã có). Item sinh ra **nối cuối** `items` (interp/VM
    /// đăng ký hàm `impl` theo `sym` — không phân biệt sinh hay viết tay).
    pub(crate) fn sinh_derives(&mut self) {
        let mut ra: Vec<HItem> = Vec::new();
        for child in self.cst.root.children() {
            match child.kind {
                NodeKind::TypeDecl => self.sinh_derive_rec(child, &mut ra),
                NodeKind::EnumDecl => self.sinh_derive_enum(child, &mut ra),
                _ => {}
            }
        }
        self.derived.extend(ra);
        self.sinh_derives_json();
    }

    /// **ST-302 (#8)** — lượt **sau** khi `tbl` đã có bản ghi/enum: sinh `to_json`/`from_json`
    /// cho `derives Json` (descriptor lồng cần bảng kiểu đầy đủ).
    pub(crate) fn sinh_derives_json(&mut self) {
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

    fn sinh_derive_rec(&mut self, node: &Node, ra: &mut Vec<HItem>) {
        let Some(name) = self.decl_name(node) else {
            return;
        };
        let derives = self.derives_of(node);
        // Kiểu **generic** ⇒ `kiem_derives` báo `VS0403`, ở đây **không sinh** (VD-98 (3)).
        if derives.is_empty() || !self.generics_of(node).is_empty() {
            return;
        }
        let Some((_, id)) = self.rec_names.iter().find(|(n, _)| *n == name).cloned() else {
            return;
        };
        let truong: Vec<(String, Type)> = node
            .children()
            .filter(|n| n.kind == NodeKind::TypeField)
            .filter_map(|f| {
                Some((
                    self.decl_name(f)?,
                    f.child(NodeKind::Type)
                        .map(|t| self.resolve_type(t))
                        .unwrap_or(Type::Error),
                ))
            })
            .collect();
        self.sinh_derive(
            DeriveNguon {
                node,
                ty: Type::Record(id),
                ten: name,
                derives,
                truong,
                la_enum: false,
            },
            ra,
        );
    }

    fn sinh_derive_enum(&mut self, node: &Node, ra: &mut Vec<HItem>) {
        let Some(name) = self.decl_name(node) else {
            return;
        };
        let derives = self.derives_of(node);
        if derives.is_empty() || !self.generics_of(node).is_empty() {
            return;
        }
        let Some((_, id)) = self.enum_names.iter().find(|(n, _)| *n == name).cloned() else {
            return;
        };
        // `enum show` **cắt** (nợ, VD-98 (3)); `enum eq` dùng `==` sẵn có.
        // Lượt 1c chạy **trước** `lower_enum` ⇒ `TypeTable` chưa có biến thể; đọc CST.
        self.sinh_derive(
            DeriveNguon {
                node,
                ty: Type::Enum(id),
                ten: name,
                derives,
                truong: Vec::new(),
                la_enum: true,
            },
            ra,
        );
    }

    fn sinh_derive(&mut self, src: DeriveNguon, ra: &mut Vec<HItem>) {
        let mut methods: Vec<HFn> = Vec::new();
        let span = src.node.span;
        // **ST-60c (B3)**: `impl` **sinh từ `derives`** phải vào `impl_pairs` như impl viết tay —
        // nếu không `impl TraitCon for T` (với `trait TraitCon: Show`) bị `VS0403` **oan**. Chỉ ghi
        // cặp khi **thật sự** sinh được (Eq bị bỏ vì `float`/hàm ⇒ **không** ghi ⇒ thiếu thật vẫn đỏ).
        let mut sinh_show = false;
        let mut sinh_eq = false;
        if src.derives.iter().any(|d| d == "Show") {
            // **ST-60c (B4)**: `enum … derives Show` nay **sinh mã thật** (trước: im lặng ⇒
            // `e.show()` ra `VS0403` khó hiểu). Thân: **một** phần nội suy ⇒ `show` của **giá trị**
            // (`Value::Enum` in `BiénThể` hoặc `BiénThể(args…)`), không đệ quy vào phương thức.
            let f = if src.la_enum {
                self.sinh_show_enum(src.ty.clone(), &src.ten, span)
            } else {
                self.sinh_show(src.ty.clone(), &src.ten, &src.truong, span)
            };
            if let Some(f) = f {
                methods.push(f);
                sinh_show = true;
            }
        }
        if src.derives.iter().any(|d| d == "Eq") && !co_truong_khong_eq(&src.truong) {
            if let Some(f) = self.sinh_eq(src.ty.clone(), &src.ten, span) {
                methods.push(f);
                sinh_eq = true;
            }
        }
        if sinh_show {
            self.impl_pairs
                .push((src.ty.clone(), "Show".to_string(), span));
        }
        if sinh_eq {
            self.impl_pairs
                .push((src.ty.clone(), "Eq".to_string(), span));
        }
        if !methods.is_empty() {
            ra.push(HItem::Impl(HImpl {
                trait_name: None,
                ty: src.ty,
                span,
                methods,
            }));
        }
    }

    /// Khai một phương thức sinh: sym + **khung riêng** + tham số; `dung(ids)` dựng **thân** từ chỉ
    /// số sym của tham số (đúng thứ tự `tham`). `None` nếu **người dùng đã viết tay** cùng tên
    /// (VD-93 (4): người dùng **thắng**, không sinh bản trùng).
    fn sinh_pt(
        &mut self,
        ty: &Type,
        m: PtSinh,
        dung: impl FnOnce(&[u32]) -> Vec<HStmt>,
    ) -> Option<HFn> {
        let (ten, ten_pt, span) = (m.ten, m.ten_pt, m.span);
        if self
            .impls
            .iter()
            .any(|(t, n, _)| n == ten_pt && self.ten_kieu(t).as_deref() == Some(ten))
        {
            return None;
        }
        let sym = self.declare(&format!("{ten}.{ten_pt}"), SymKind::Item, span, None);
        self.dang_ky_impl(ty, ten_pt, sym);
        self.push_frame(FrameKind::Fn, span);
        let mut ids: Vec<u32> = Vec::with_capacity(m.tham.len());
        let mut params: Vec<HParam> = Vec::with_capacity(m.tham.len());
        for (n, t) in &m.tham {
            let s = self.declare(n, SymKind::Param, span, Some(t.clone()));
            ids.push(s);
            params.push(HParam {
                name: (*n).to_string(),
                span,
                sym: s,
                ty: t.clone(),
                named: false,
                is_self: false,
                is_var_self: false,
                default: None,
                variadic: false,
            });
        }
        let body = dung(&ids);
        self.pop_frame();
        Some(HFn {
            name: format!("{ten}.{ten_pt}"),
            span,
            sym,
            kind: super::FnKind::ImplMethod,
            is_pub: false,
            is_async: false,
            generics: Vec::new(),
            params,
            ret: m.ret,
            body,
        })
    }

    /// `fn show(self) -> str` — **đúng byte** `Value::show` của bản ghi (tên động rỗng):
    /// `{x: 1, ten: a}`. Dựng bằng `Interp` (không `+` nối chuỗi — tyck không có `(Add, Str, Str)`).
    /// **ST-60c (B4)**: `show` cho **enum** = **một** phần nội suy của `self` ⇒ in đúng dạng giá trị
    /// (`A`, `B(1, 2)`) như `Value::show` lúc chạy; không gọi lại phương thức (không đệ quy).
    fn sinh_show_enum(&mut self, ty: Type, ten: &str, span: vis_syntax::Span) -> Option<HFn> {
        let ty2 = ty.clone();
        self.sinh_pt(
            &ty,
            PtSinh {
                ten,
                ten_pt: "show",
                ret: Type::Str,
                tham: vec![("self", ty2)],
                span,
            },
            move |ids| {
                let me = ids[0];
                let parts: Vec<InterpPart> = vec![InterpPart::Expr(Box::new(HExpr::Var {
                    name: "self".into(),
                    sym: Some(me),
                    span,
                }))];
                vec![HStmt::Return {
                    value: Some(HExpr::Interp { parts, span }),
                    span,
                }]
            },
        )
    }

    fn sinh_show(
        &mut self,
        ty: Type,
        ten: &str,
        truong: &[(String, Type)],
        span: vis_syntax::Span,
    ) -> Option<HFn> {
        let truong = truong.to_vec();
        self.sinh_pt(
            &ty,
            PtSinh {
                ten,
                ten_pt: "show",
                ret: Type::Str,
                tham: vec![("self", ty.clone())],
                span,
            },
            move |ids| {
                let me = ids[0];
                let mut parts: Vec<InterpPart> = Vec::new();
                if truong.is_empty() {
                    parts.push(InterpPart::Str("{}".into()));
                } else {
                    for (i, (fn_, _)) in truong.iter().enumerate() {
                        if i > 0 {
                            parts.push(InterpPart::Str(", ".into()));
                        }
                        parts.push(InterpPart::Str(format!("{fn_}: ")));
                        parts.push(InterpPart::Expr(Box::new(HExpr::Field {
                            base: Box::new(HExpr::Var {
                                name: "self".into(),
                                sym: Some(me),
                                span,
                            }),
                            name: fn_.clone(),
                            optional: false,
                            span,
                        })));
                    }
                    parts.insert(0, InterpPart::Str("{".into()));
                    parts.push(InterpPart::Str("}".into()));
                }
                vec![HStmt::Return {
                    value: Some(HExpr::Interp { parts, span }),
                    span,
                }]
            },
        )
    }

    /// `fn eq(self, other) -> bool` — `self == other` (hai đường gọi cùng `eq_value`).
    fn sinh_eq(&mut self, ty: Type, ten: &str, span: vis_syntax::Span) -> Option<HFn> {
        self.sinh_pt(
            &ty,
            PtSinh {
                ten,
                ten_pt: "eq",
                ret: Type::Bool,
                tham: vec![("self", ty.clone()), ("other", ty.clone())],
                span,
            },
            |ids| {
                vec![HStmt::Return {
                    value: Some(HExpr::Bin {
                        op: BinOp::Eq,
                        lhs: Box::new(HExpr::Var {
                            name: "self".into(),
                            sym: Some(ids[0]),
                            span,
                        }),
                        rhs: Box::new(HExpr::Var {
                            name: "other".into(),
                            sym: Some(ids[1]),
                            span,
                        }),
                        span,
                    }),
                    span,
                }]
            },
        )
    }

    /// **ST-302 (#8)**: kiểu mã hoá JSON được? Từ chối `cap`/`Chan`/`Task`/`fn` ở mọi độ sâu.
    fn json_hop_le(&self, t: &Type, depth: usize) -> bool {
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

/// Nguồn một lượt sinh `derives` (gói tham số cho [`Lowerer::sinh_derive`]).
pub(super) struct DeriveNguon<'n> {
    node: &'n Node,
    ty: Type,
    ten: String,
    derives: Vec<String>,
    truong: Vec<(String, Type)>,
    la_enum: bool,
}

/// Tham số một phương thức **sinh** (gói cho [`Lowerer::sinh_pt`]).
pub(super) struct PtSinh<'a> {
    ten: &'a str,
    ten_pt: &'a str,
    ret: Type,
    tham: Vec<(&'a str, Type)>,
    span: vis_syntax::Span,
}

/// **ST-60c (B5)**: kiểu có chứa **`float`/hàm** ở **bất kỳ độ sâu** không — `Eq` đòi so sánh
/// **toàn phần** nên đệ quy qua `List`/`Map`/`Opt`/`Result`/`Tuple`/`TypeRef` (generic) (VD-93 (3)).
fn co_float_sau(t: &Type) -> bool {
    match t {
        Type::Float | Type::Fn(..) => true,
        Type::List(x) | Type::Opt(x) => co_float_sau(x),
        Type::Map(k, v) | Type::Result(k, v) => co_float_sau(k) || co_float_sau(v),
        // `TypeRef` = `tuple` nhiều phần tử, `Task[T]`, generic chưa thay ⇒ soi **mọi** đối số.
        Type::TypeRef { args, .. } => args.iter().any(co_float_sau),
        // Bản ghi lồng: cần `TypeTable` ⇒ ngoài tầm hàm thuần (nợ, xem JOURNAL).
        _ => false,
    }
}

/// Kiểu của một trường có **so sánh toàn phần** không (đệ quy **sâu** — ST-60c).
fn co_truong_khong_eq(truong: &[(String, Type)]) -> bool {
    truong.iter().any(|(_, t)| co_float_sau(t))
}
