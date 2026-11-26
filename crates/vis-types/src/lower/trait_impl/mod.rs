// SPDX-License-Identifier: MIT OR Apache-2.0
//! **trait / `impl` / `derives`** (ST-29) — tách khỏi `lower.rs` cho luật R1 (≤ 1.500 dòng/tệp).
//!
//! Dispatch **tĩnh**: `x.m(…)` với `x` kiểu tĩnh `Record`/`Enum` ⇒ hạ thẳng `Call(impl_fn)` (chèn `self`
//! đầu) ⇒ interp và VM dùng **cùng HIR** (không bảng `impls`, không bump `VBC1`).
//! `derives Eq/Show/Json` = **kiểm + đăng ký**, ngữ nghĩa dùng cái có sẵn.
//!
//! **ST-318c** — tách tệp theo trách nhiệm (0 đổi hành vi/API): `kiem` (thu thập + kiểm),
//! `sinh` (sinh HIR cho `Eq/Show`), `json` (sinh `to_json`/`from_json`), `sieu` (siêu trait).

use super::{ImplDangKy, Lowerer};
use vis_hir::hir::HExpr;
use vis_hir::ty::Type;
use vis_syntax::Node;

mod json;
mod kiem;
mod sieu;
mod sinh;

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
