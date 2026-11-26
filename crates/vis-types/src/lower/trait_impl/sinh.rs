// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-318c** — sinh HIR cho `derives Eq/Show` (nguyên văn từ `lower/trait_impl.rs`; 0 đổi
//! hành vi). `derives Json` nằm ở `json.rs`.

use super::super::{FnKind, FrameKind, SymKind};
use super::*;
use vis_hir::hir::{BinOp, HFn, HImpl, HItem, HParam, HStmt, InterpPart};
use vis_syntax::NodeKind;

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
        if src.derives.iter().any(|d| d == "Eq") && !co_truong_khong_eq(&src.truong, self.tbl) {
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
    pub(super) fn sinh_pt(
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
            kind: FnKind::ImplMethod,
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
}

/// **ST-60c (B5)** + **ST-316b (`#7`)** — kiểu có chứa **`float`/hàm** ở **bất kỳ độ sâu** không
/// (`Eq` đòi so sánh **toàn phần**). Đệ quy qua `List`/`Map`/`Opt`/`Result`/`Tuple`/`TypeRef`
/// (generic) **và** bản ghi/enum lồng qua [`TypeTable`]; `da_tham` giữ các `RecId`/`EnumId` đã
/// soi để kiểu **tự quy chiếu** (`type Node: next: Node?`) không đệ quy vô hạn.
pub(super) fn co_float_sau(
    t: &Type,
    tbl: &vis_hir::ty::TypeTable,
    da_tham: &mut Vec<(u8, u32)>,
) -> bool {
    match t {
        Type::Float | Type::Fn(..) => true,
        Type::List(x) | Type::Opt(x) => co_float_sau(x, tbl, da_tham),
        Type::Map(k, v) | Type::Result(k, v) => {
            co_float_sau(k, tbl, da_tham) || co_float_sau(v, tbl, da_tham)
        }
        // `TypeRef` = `tuple` nhiều phần tử, `Task[T]`, generic chưa thay ⇒ soi **mọi** đối số.
        Type::TypeRef { args, .. } => args.iter().any(|a| co_float_sau(a, tbl, da_tham)),
        Type::Record(id) => {
            if da_tham.contains(&(0, id.0)) {
                return false;
            }
            da_tham.push((0, id.0));
            tbl.rec(*id)
                .is_some_and(|d| d.fields.iter().any(|f| co_float_sau(&f.ty, tbl, da_tham)))
        }
        Type::Enum(id) => {
            if da_tham.contains(&(1, id.0)) {
                return false;
            }
            da_tham.push((1, id.0));
            tbl.enum_def(*id).is_some_and(|d| {
                d.variants
                    .iter()
                    .any(|v| v.fields.iter().any(|f| co_float_sau(&f.ty, tbl, da_tham)))
            })
        }
        _ => false,
    }
}

/// Kiểu của một trường có **so sánh toàn phần** không (đệ quy **sâu** — ST-60c; `#7`: cả
/// bản ghi/enum lồng).
fn co_truong_khong_eq(truong: &[(String, Type)], tbl: &vis_hir::ty::TypeTable) -> bool {
    truong.iter().any(|(_, t)| {
        let mut da_tham = Vec::new();
        co_float_sau(t, tbl, &mut da_tham)
    })
}
