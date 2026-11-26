// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-318c** — thu thập `trait`/`impl` và kiểm `derives`/`impl` khớp trait (nguyên văn từ
//! `lower/trait_impl.rs`; 0 đổi hành vi/chẩn đoán).

use super::super::SymKind;
use super::sinh::co_float_sau;
use super::*;
use vis_diag::TYPE_CODES;
use vis_hir::hir::HFn;
use vis_syntax::NodeKind;

impl<'a> Lowerer<'a> {
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
                    // **ST-316b (`#7`)**: đệ quy cả **bản ghi/enum lồng** qua [`TypeTable`], có tập
                    // `đã thăm` chống vòng (kiểu tự quy chiếu `type Node: next: Node?`).
                    let mut da_tham = Vec::new();
                    let co = co_float_sau(t, self.tbl, &mut da_tham);
                    if co {
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
