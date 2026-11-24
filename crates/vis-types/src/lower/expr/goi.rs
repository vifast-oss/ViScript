// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — gọi hàm/phương thức, đối số, dispatch kiểu của `lower::expr`.

use super::*;

impl<'a> Lowerer<'a> {
    /// `nilai_kiểu` cho các vị trí cần suy ra kiểu phần tử (list/map/tuple).
    pub(super) fn type_of(&self, e: &HExpr) -> Type {
        match e {
            HExpr::Lit { ty, .. }
            | HExpr::Opt { ty, .. }
            | HExpr::Record { ty, .. }
            | HExpr::Tuple { ty, .. }
            | HExpr::If { ty, .. }
            | HExpr::Match { ty, .. }
            | HExpr::EnumLit { ty, .. } => ty.clone(),
            HExpr::List { elem, .. } => Type::List(Box::new(elem.clone())),
            HExpr::Set { elem, .. } => Type::List(Box::new(elem.clone())),
            HExpr::Map { key, value, .. } => {
                Type::Map(Box::new(key.clone()), Box::new(value.clone()))
            }
            HExpr::Lambda { params, ret, .. } => Type::Fn(
                params
                    .iter()
                    .map(|p| p.ty.clone())
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                Box::new(ret.clone()),
            ),
            _ => Type::Error,
        }
    }

    /// (đối số vị trí, đối số có tên) từ node `Args`.
    pub(super) fn lower_args(&mut self, args: &Node) -> (Vec<HExpr>, Vec<(String, HExpr)>) {
        let mut positional = Vec::new();
        let mut named = Vec::new();
        for e in args.children() {
            match e.kind {
                NodeKind::NamedArg => {
                    let name = self.decl_name(e).unwrap_or_default();
                    let value = e
                        .children()
                        .next()
                        .map(|c| self.lower_expr(c))
                        .unwrap_or(HExpr::Error { span: e.span });
                    named.push((name, value));
                }
                _ => positional.push(self.lower_expr(e)),
            }
        }
        (positional, named)
    }

    /// **VD-263** — như [`Lowerer::lower_args`] nhưng truyền **kiểu mong đợi** của tham số
    /// (`Some(x)` dưới annot `T?`, đối số biến thể/trường của instantiation) xuống từng biểu thức.
    pub(super) fn lower_args_expect(
        &mut self,
        args: &Node,
        callee: &HExpr,
        expect: Option<&Type>,
    ) -> (Vec<HExpr>, Vec<(String, HExpr)>) {
        let (pos_expect, named_expect) = self.expect_call_args(callee, expect);
        let mut positional = Vec::new();
        let mut named = Vec::new();
        for e in args.children() {
            match e.kind {
                NodeKind::NamedArg => {
                    let name = self.decl_name(e).unwrap_or_default();
                    let want = named_expect
                        .iter()
                        .find(|(n, _)| n == &name)
                        .map(|(_, t)| t.clone());
                    let value = e
                        .children()
                        .next()
                        .map(|c| self.lower_expr_as(c, want))
                        .unwrap_or(HExpr::Error { span: e.span });
                    named.push((name, value));
                }
                _ => {
                    let want = pos_expect.get(positional.len()).cloned();
                    positional.push(self.lower_expr_as(e, want));
                }
            }
        }
        (positional, named)
    }

    /// **VD-263** — kiểu mong đợi của từng đối số theo **hình dạng lời gọi**:
    /// `Some(x)` dưới `T?`, `Ok(x)`/`Err(e)` dưới `Result`, đối số biến thể/trường của
    /// `EnumId`/`RecId` **instantiation**. Không khớp ⇒ rỗng (hạ như cũ).
    pub(super) fn expect_call_args(
        &self,
        callee: &HExpr,
        expect: Option<&Type>,
    ) -> (Vec<Type>, Vec<(String, Type)>) {
        // **VD-263**: chữ ký hàm đã hạ (`fn f(b: Box[int?])`) là kiểu mong đợi của đối số.
        // **VD-265**: pre-pass nạp **mọi** chữ ký (hàm khai sau call-site), kiểu `fn(…)` của
        // biến (giá trị hàm/closure), và chữ ký phương thức `impl` khi bên nhận có kiểu tĩnh.
        let (mut pos, named) = match callee {
            HExpr::Var { sym: Some(id), .. } => match self.fn_params.get(id) {
                Some(ps) => (
                    ps.iter().map(|(_, t)| t.clone()).collect::<Vec<_>>(),
                    ps.clone(),
                ),
                None => match self.type_env.get(id) {
                    Some(Type::Fn(ps, _)) => (ps.to_vec(), Vec::new()),
                    _ => (Vec::new(), Vec::new()),
                },
            },
            HExpr::Lambda { params, .. } => (
                params.iter().map(|p| p.ty.clone()).collect(),
                params
                    .iter()
                    .map(|p| (p.name.clone(), p.ty.clone()))
                    .collect(),
            ),
            HExpr::Field {
                base,
                name,
                optional: false,
                ..
            } => match self.tham_so_phuong_thuc(base, name) {
                Some(ps) => {
                    // `fn_params` của phương thức có `self` đầu; đối số call-site **không** có.
                    let ps: &[(String, Type)] = if ps.first().is_some_and(|(n, _)| n == "self") {
                        &ps[1..]
                    } else {
                        &ps[..]
                    };
                    (ps.iter().map(|(_, t)| t.clone()).collect(), ps.to_vec())
                }
                None => (Vec::new(), Vec::new()),
            },
            _ => (Vec::new(), Vec::new()),
        };
        let Some(expect) = expect else {
            return (pos, named);
        };
        if let HExpr::Var { name, .. } = callee {
            match (name.as_str(), expect) {
                ("Some", Type::Opt(inner)) => return (vec![(**inner).clone()], Vec::new()),
                ("Ok", Type::Result(ok, _)) => return (vec![(**ok).clone()], Vec::new()),
                ("Err", Type::Result(_, err)) => return (vec![(**err).clone()], Vec::new()),
                _ => {}
            }
        }
        // Ctor trực tiếp dưới `T?`/`T??` vẫn dựng kiểu **lõi** trước khi biên annot nâng.
        match bo_opt(expect) {
            Type::Enum(inst) => {
                let Some((ten_inst, _)) = self.tbl.enum_inst_of(*inst) else {
                    return (pos, named);
                };
                let variant = match callee {
                    HExpr::Var { name, .. } => Some(name.as_str()),
                    HExpr::Field { base, name, .. } => match &**base {
                        HExpr::Var { name: en, .. } if en == ten_inst => Some(name.as_str()),
                        _ => None,
                    },
                    _ => None,
                };
                let Some(v) = variant else {
                    return (pos, named);
                };
                if let Some(vd) = self.tbl.variant(*inst, v) {
                    pos = vd.fields.iter().map(|f| f.ty.clone()).collect();
                }
            }
            Type::Record(inst) => {
                let HExpr::Var { name, .. } = callee else {
                    return (pos, named);
                };
                let Some((base, _)) = self.tbl.rec_inst_of(*inst) else {
                    return (pos, named);
                };
                if base == name {
                    if let Some(r) = self.tbl.rec(*inst) {
                        return (
                            Vec::new(),
                            r.fields
                                .iter()
                                .map(|f| (f.name.clone(), f.ty.clone()))
                                .collect(),
                        );
                    }
                }
            }
            _ => {}
        }
        (pos, named)
    }

    /// **VD-265** — chữ ký phương thức `impl` cho `base.name` khi `base` có kiểu **tĩnh** và tên
    /// phương thức khớp **đúng một** impl (giống [`Lowerer::dispatch_tinh`] nhưng không chẩn đoán).
    pub(super) fn tham_so_phuong_thuc(
        &self,
        base: &HExpr,
        name: &str,
    ) -> Option<Vec<(String, Type)>> {
        let ty = self.static_ty(base)?;
        let sym = self.method_sym(&ty, name)?;
        self.fn_params.get(&sym).cloned()
    }

    /// **VD-265** — kiểu `fn(…) -> …` của một **giá trị hàm**: lambda, hoặc tên hàm đã có chữ ký
    /// trong pre-pass. `None` nếu không chắc.
    pub(crate) fn fn_value_ty(&self, e: &HExpr) -> Option<Type> {
        match e {
            HExpr::Lambda { .. } => match self.type_of(e) {
                t @ Type::Fn(..) => Some(t),
                _ => None,
            },
            HExpr::Var { sym: Some(id), .. } => {
                let ps = self.fn_params.get(id)?;
                let ret = self.fn_ret.get(id).cloned().unwrap_or(Type::Unit);
                Some(Type::Fn(
                    ps.iter()
                        .map(|(_, t)| t.clone())
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                    Box::new(ret),
                ))
            }
            _ => None,
        }
    }

    /// Gọi `std.<module>.<fn>(…)` khi `<module>` đã `use` (ST-26) — tên đầy đủ `std.m.f`.
    pub(super) fn std_call_named(
        &mut self,
        m: String,
        f: String,
        args: Vec<HExpr>,
        named: Vec<(String, HExpr)>,
        span: Span,
    ) -> HExpr {
        let ngan = format!("{m}.{f}");
        // **ST-60b**: `std.*` **chỉ** có đối số **vị trí** — trước đây `--desc` bị **bỏ im lặng**
        // (`named: Vec::new()` dưới kia). Nay **từ chối** `VS0401`, nêu đúng tên.
        if let Some((ten, a)) = named.first() {
            let span_a = a.span();
            self.diag(
                &vis_diag::TYPE_CODES[0],
                format!(
                    "`std.{ngan}` không nhận tham số **tên** `--{ten}` — `std.*` chỉ có đối số vị trí"
                ),
                span_a,
            );
            return HExpr::Error { span };
        }
        if vis_types_stdlib::hoan(&ngan) {
            self.diag(&NAME_CODES[0], format!("`{ngan}` chưa hỗ trợ (VS1b)"), span);
            return HExpr::Error { span };
        }
        if !vis_types_stdlib::la_std(&ngan) {
            self.diag(
                &NAME_CODES[0],
                format!("module `std.{m}` không có hàm `{f}`"),
                span,
            );
            return HExpr::Error { span };
        }
        HExpr::Call {
            callee: Box::new(HExpr::Var {
                name: format!("std.{ngan}"),
                sym: None,
                span,
            }),
            args,
            named: Vec::new(),
            span,
        }
    }

    /// ST-29 — **dispatch tĩnh**: `x.m(…)` với `x` có kiểu tĩnh **record/enum** ⇒ tìm `impl` và hạ
    /// thành gọi **trực tiếp** hàm impl (chèn `self`/`dt` làm đối số đầu). Không có ⇒ `VS0403`;
    /// nhiều ⇒ `VS0403` (mơ hồ). Kiểu không tĩnh ⇒ để nguyên (nội tại/`VS0403` ở tầng sau).
    pub(super) fn dispatch_tinh(
        &mut self,
        callee: &HExpr,
        args: &[HExpr],
        span: Span,
    ) -> Option<HExpr> {
        let HExpr::Field {
            base,
            name,
            optional: false,
            ..
        } = callee
        else {
            return None;
        };
        let Some(ty) = self.static_ty(base) else {
            // ST-29: kiểu bên nhận **không tĩnh** (generic/`{error}`) — nếu tên khớp một phương thức
            // `impl` nào đó thì báo rõ (VS1b), thay vì để lỗi mờ lúc chạy.
            if self.impls.iter().any(|(_, n, _)| n == name) {
                self.diag(
                    &TYPE_CODES[2],
                    format!(
                        "không phân giải được phương thức `{name}` cho kiểu không tĩnh (VS1b — thêm chú thích kiểu)"
                    ),
                    span,
                );
                return Some(HExpr::Error { span });
            }
            return None;
        };
        let ten = self.ten_kieu(&ty)?;
        let khop = self.khop_impl(&ten, name);
        match khop.len() {
            1 => {
                // Đối số đầu = chính biểu thức nhận (`self`/`dt` theo giá trị).
                let mut ds: Vec<HExpr> = Vec::with_capacity(args.len() + 1);
                ds.push((**base).clone());
                ds.extend(args.iter().cloned());
                Some(HExpr::Call {
                    callee: Box::new(HExpr::Var {
                        name: format!("{ten}.{name}"),
                        sym: Some(khop[0]),
                        span,
                    }),
                    args: ds,
                    named: Vec::new(),
                    span,
                })
            }
            0 => {
                let co_trait = self
                    .traits
                    .iter()
                    .chain(self.traits_ngoai.iter())
                    .find(|t| t.methods.iter().any(|(mn, _, _, _)| mn == name))
                    .map(|t| t.name.clone());
                let msg = match co_trait {
                    Some(tn) => format!(
                        "trait `{tn}` có phương thức `{name}` nhưng `{ten}` chưa cài (`impl {tn} for {ten}`)"
                    ),
                    None => format!("không có phương thức `{name}` cho `{ten}` (VS0403)"),
                };
                self.diag(&TYPE_CODES[2], msg, span);
                Some(HExpr::Error { span })
            }
            n => {
                self.diag(
                    &TYPE_CODES[2],
                    format!("mơ hồ: {n} impl cùng cung cấp phương thức `{name}` cho `{ten}`"),
                    span,
                );
                Some(HExpr::Error { span })
            }
        }
    }

    /// **VD-263** — chọn `RecId` **instantiation** khi kiểu mong đợi là `Type::Record(id)` cùng
    /// họ tên với bản khai báo `base`; ngược lại giữ `base`.
    pub(super) fn rec_inst_theo(&self, base: RecId, expect: Option<&Type>) -> RecId {
        let Some(Type::Record(id)) = expect.map(bo_opt) else {
            return base;
        };
        let Some((ten_inst, _)) = self.tbl.rec_inst_of(*id) else {
            return base;
        };
        let Some((ten_base, _)) = self.rec_names.iter().find(|(_, b)| *b == base) else {
            return base;
        };
        if ten_inst == ten_base {
            *id
        } else {
            base
        }
    }

    /// Như [`Lowerer::rec_inst_theo`] cho enum.
    pub(super) fn enum_inst_theo(&self, base: EnumId, expect: Option<&Type>) -> EnumId {
        let Some(Type::Enum(id)) = expect.map(bo_opt) else {
            return base;
        };
        let Some((ten_inst, _)) = self.tbl.enum_inst_of(*id) else {
            return base;
        };
        let Some((ten_base, _)) = self.enum_names.iter().find(|(_, b)| *b == base) else {
            return base;
        };
        if ten_inst == ten_base {
            *id
        } else {
            base
        }
    }

    /// `Circle(1.0)` / `Shape.Circle(1.0)` ⇒ [`HExpr::EnumLit`] khi biến thể **đã khai**.
    pub(super) fn maybe_enum_lit(
        &mut self,
        callee: HExpr,
        args: Vec<HExpr>,
        named: Vec<(String, HExpr)>,
        expect: Option<&Type>,
        span: Span,
    ) -> HExpr {
        match &callee {
            HExpr::Var { name, .. } => {
                // **R11 (VD-178)**: `U(n: 1)` — dựng bản ghi theo **tên kiểu**.
                if !named.is_empty() {
                    if name == "RowSet" && !self.sqlite_duoc_thay() {
                        // **VD-241**: `RowSet` thuộc module `std.sqlite` — dựng ngoài phạm vi ⇒
                        // lỗi biên dịch rõ ràng, không âm thầm thành tên lạ.
                        self.diag(
                            &NAME_CODES[0],
                            "không thấy tên `RowSet`: module `std.sqlite` chưa được `use`"
                                .to_string(),
                            span,
                        );
                        return HExpr::Error { span };
                    }
                    if let Some((_, base_rid)) =
                        self.rec_names.iter().find(|(n, _)| n == name).cloned()
                    {
                        // **VD-263**: annot `Box[int?]` ⇒ dựng bằng **id instantiation** (trường
                        // `T?` đã thay) để biên dựng nâng đúng lớp.
                        let rid = self.rec_inst_theo(base_rid, expect);
                        // **R5a (VD-178)**: trường **thiếu** lấy **giá trị mặc định** của khai báo.
                        let mut fields = named;
                        // **VD-241 §6/B-B (A0, 2026-11-19)**: trường trùng tên ⇒ lỗi biên dịch.
                        let mut seen = std::collections::HashSet::with_capacity(fields.len());
                        for (n, _) in &fields {
                            if !seen.insert(n.clone()) {
                                self.diag(
                                    &vis_diag::TYPE_CODES[4],
                                    format!(
                                        "trường `{}` xuất hiện nhiều lần trên bản ghi `{}`",
                                        n, name
                                    ),
                                    span,
                                );
                                return HExpr::Error { span };
                            }
                        }
                        if let Some(rec) = self.items.iter().find_map(|it| match it {
                            vis_hir::hir::HItem::Rec(r) if r.id == base_rid => Some(r.clone()),
                            _ => None,
                        }) {
                            for f in &rec.fields {
                                if !fields.iter().any(|(n, _)| n == &f.name) {
                                    if let Some(d) = &f.default {
                                        fields.push((f.name.clone(), d.clone()));
                                    } else {
                                        // **VD-241 §6/B-B (A0, 2026-11-19)**: trường bắt buộc thiếu ⇒ lỗi tĩnh.
                                        self.diag(
                                            &vis_diag::TYPE_CODES[4],
                                            format!(
                                                "thiếu trường bắt buộc `{}` của bản ghi `{}`",
                                                f.name, name
                                            ),
                                            span,
                                        );
                                        return HExpr::Error { span };
                                    }
                                }
                            }
                        }
                        return HExpr::Record {
                            fields,
                            ty: Type::Record(rid),
                            span,
                        };
                    }
                }
                if let Some((_, eid, _)) = self
                    .variant_index
                    .iter()
                    .find(|(n, _, _)| n == name)
                    .cloned()
                {
                    // **VD-241**: ctor dựng sẵn của enum `SqlValue` (thuộc module `std.sqlite`)
                    // ngoài phạm vi `use std.sqlite` ⇒ **không** phân giải thành biến thể — rơi
                    // qua đường gọi hàm (báo lỗi tên), tránh âm thầm tạo `SqlValue` "miễn phí".
                    let la_sqlvalue = self.sqlvalue_id() == Some(eid);
                    if la_sqlvalue && !self.sqlite_duoc_thay() {
                        // rơi xuống nhánh gọi hàm thường
                    } else {
                        // **VD-241 §3/T3 (A0, 2026-11-19)**: giữ **tên enum** cho `MakeEnum` — VM dựng
                        // `Value::Enum` đúng tên (ví dụ `Arr`/`Str` ⇒ `Json`), không còn tạo biệt danh
                        // `Arr.Arr` (trước đây json.encode của VM rơi vào fallback `{"variant":…}`).
                        let e_path = self
                            .enum_names
                            .iter()
                            .find(|(_, id)| id == &eid)
                            .map(|(n, _)| n.clone())
                            .unwrap_or_else(|| name.clone());
                        let eid = self.enum_inst_theo(eid, expect);
                        return HExpr::EnumLit {
                            enum_id: Some(eid),
                            path: vec![e_path],
                            variant: name.clone(),
                            args,
                            ty: Type::Enum(eid),
                            span,
                        };
                    }
                }
            }
            HExpr::Field {
                base,
                name,
                optional: false,
                ..
            } => {
                if let HExpr::Var {
                    name: enum_name, ..
                } = &**base
                {
                    if let Some((_, eid)) = self
                        .enum_names
                        .iter()
                        .find(|(n, _)| n == enum_name)
                        .cloned()
                    {
                        // **VD-241**: ctor có tiền tố `SqlValue.` ngoài `use std.sqlite` ⇒ từ chối.
                        if enum_name == "SqlValue" && !self.sqlite_duoc_thay() {
                            self.diag(
                                &NAME_CODES[0],
                                "không thấy tên `SqlValue`: module `std.sqlite` chưa được `use`"
                                    .to_string(),
                                span,
                            );
                            return HExpr::Error { span };
                        }
                        let eid = self.enum_inst_theo(eid, expect);
                        return HExpr::EnumLit {
                            enum_id: Some(eid),
                            path: vec![enum_name.clone()],
                            variant: name.clone(),
                            args,
                            ty: Type::Enum(eid),
                            span,
                        };
                    }
                }
            }
            _ => {}
        }
        HExpr::Call {
            callee: Box::new(callee),
            args,
            named,
            span,
        }
    }
}
