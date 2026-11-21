// SPDX-License-Identifier: MIT OR Apache-2.0
//! **Kiểm kiểu VS0** (ST-12) — chạy trên HIR của ST-11, báo mã `04xx` (VD-42) và `VS0301` cho
//! tên **kiểu** không tồn tại; tên **giá trị** chưa khai vẫn theo VD-45 (`Permissive`).
//!
//! # Hợp đồng
//!
//! - Đầu vào: HIR đã hạ ([`crate::hir::Hir`]) + [`crate::lower::Symbols`] + [`crate::TypeTable`].
//! - **Không** panic trên HIR hợp lệ; `Type::Error` lan truyền **im lặng** (thiết kế §14.3) và
//!   tương thích với mọi kiểu.
//! - Mã: `VS0401` sai số/thứ tự đối số, `VS0402` `match` không xét hết, `VS0403` trường/kiểu
//!   không tồn tại, `VS0412` kiểu không khớp, `VS0301` tên **kiểu** không tồn tại.
//! - `%`/`/` cho 0 là **runtime** (ST-13), **không** báo ở đây (VD-43 #2).
//!
//! # Phân tầng
//!
//! [`mod`]: khung/môi trường/câu lệnh · [`num`]: bảng toán tử số học/so sánh · [`expr`]: biểu thức · [`pat`]: pattern + **xét hết**.

mod expr;
mod flow;
mod gan;
mod num;
mod pat;
mod phuong_thuc;
mod sig;
mod truong;
mod tuy_chon;
mod vd238;

use std::collections::BTreeSet;

use sig::FnSig;

use vis_diag::{NAME_CODES, TYPE_CODES};
use vis_syntax::{Diagnostic, Span};

use crate::hir::{HExpr, HFn, HItem, HStmt, Hir};
use crate::lower::Symbols;
use crate::ty::{EnumId, RecId, Type, TypeTable};

/// Mã `VS0401` — sai số lượng/thứ tự đối số.
pub(crate) const ARG_COUNT: &vis_diag::Code = &TYPE_CODES[0];
/// Mã `VS0402` — `match` không xét hết.
pub(crate) const NON_EXHAUSTIVE: &vis_diag::Code = &TYPE_CODES[1];
/// Mã `VS0403` — trường/kiểu không tồn tại.
pub(crate) const NO_FIELD: &vis_diag::Code = &TYPE_CODES[2];
/// Mã `VS0412` — kiểu không khớp (ST-50: `04xx` có **5** mã, 0412 ở **index 4**).
pub(crate) const MISMATCH: &vis_diag::Code = &TYPE_CODES[4];
/// Mã `VS0301` — tên **kiểu** không tồn tại (ST-12; ST-11 không báo).
pub(crate) const NO_TYPE: &vis_diag::Code = &NAME_CODES[0];
/// Mã `VS0303` — **kiểu** không tồn tại (R9/VD-178: tách khỏi `VS0301` tên giá trị).
pub(crate) const NO_TYPE_NAME: &vis_diag::Code = &NAME_CODES[2];
/// Mã `VS0413` — gán vào ràng buộc bất biến (`let`/tham số) — R9 (VD-178).
pub(crate) const GAN_BAT_BIEN: &vis_diag::Code = &TYPE_CODES[5];
/// Mã `VS0414` — toán tử không hỗ trợ cho kiểu — R9 (VD-178).
pub(crate) const TOAN_TU: &vis_diag::Code = &TYPE_CODES[6];
/// Mã `VS0415` — `async` sai chỗ — R9 (VD-178).
pub(crate) const ASYNC_SAI_CHO: &vis_diag::Code = &TYPE_CODES[7];
/// Mã `VS0416` — range không `int` — R9 (VD-178).
pub(crate) const RANGE_KHONG_INT: &vis_diag::Code = &TYPE_CODES[8];
/// **VD-266**: literal `int` không biểu diễn được — dùng lại `VS0104` của tầng từ vựng cho
/// literal **dương** `i64::MAX + 1` (lexer cho qua độ lớn 2^63 để gộp `-` thành `i64::MIN`).
pub(crate) const INT_LIT: &vis_diag::Code = &vis_diag::CODES[3];
/// Mã `VS0206` — khối `else` của `let … else` phải thoát trên **mọi nhánh** (sổ cú pháp 02xx;
/// mã đã có sẵn đúng nghĩa, VD-263 dùng lại thay vì thêm mã mới).
pub(crate) const ELSE_KHONG_THOAT: &vis_diag::Code = &vis_diag::SYNTAX_CODES[5];

/// Kết quả một lượt kiểm kiểu.
#[derive(Debug, Clone, Default)]
pub struct CheckOutput {
    pub diags: Vec<Diagnostic>,
    /// Kiểu của **mọi** biểu thức, theo thứ tự duyệt (tiền tự).
    pub types_of_expr: Vec<Type>,
    /// Kiểu của các biểu thức **cấp tệp**, theo thứ tự nguồn.
    pub file_expr_types: Vec<Type>,
    /// Kiểu của từng item cấp tệp (`fn`/`const`/…).
    pub item_types: Vec<(String, Type)>,
}

impl CheckOutput {
    /// `true` nếu không có chẩn đoán.
    pub fn is_clean(&self) -> bool {
        self.diags.is_empty()
    }

    /// Mã chẩn đoán theo thứ tự.
    pub fn codes(&self) -> Vec<&'static str> {
        self.diags.iter().map(|d| d.code.short).collect()
    }

    /// Kiểu "giá trị của chương trình": biểu thức cấp tệp **cuối**, không có thì `()`.
    pub fn value_type(&self) -> Type {
        self.file_expr_types.last().cloned().unwrap_or(Type::Unit)
    }
}

/// ST-28 — **xoá kiểu**: thay mọi `TypeRef` trỏ tới tham số kiểu bằng `{error}` (không còn thông tin
/// tĩnh), đệ quy vào list/map/optional/Result/hàm. Dùng cho thân hàm generic và mọi chỗ khác.
pub(crate) fn erase_gen(ty: &Type, gens: &[String]) -> Type {
    match ty {
        Type::TypeRef { name, args } if args.is_empty() && gens.iter().any(|g| g == name) => {
            Type::Error
        }
        Type::TypeRef { name, args } => Type::TypeRef {
            name: name.clone(),
            args: args.iter().map(|a| erase_gen(a, gens)).collect(),
        },
        Type::List(t) => Type::List(Box::new(erase_gen(t, gens))),
        Type::Map(k, v) => Type::Map(Box::new(erase_gen(k, gens)), Box::new(erase_gen(v, gens))),
        Type::Opt(t) => Type::Opt(Box::new(erase_gen(t, gens))),
        Type::Result(a, b) => {
            Type::Result(Box::new(erase_gen(a, gens)), Box::new(erase_gen(b, gens)))
        }
        Type::Fn(ps, r) => Type::Fn(
            ps.iter()
                .map(|p| erase_gen(p, gens))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            Box::new(erase_gen(r, gens)),
        ),
        other => other.clone(),
    }
}

/// ST-28 — **thay** tham số kiểu bằng kiểu đã suy: `T` → `int`; chưa suy ⇒ `{error}`.
pub(crate) fn thay_gen(ty: &Type, bang: &[(String, Type)]) -> Type {
    match ty {
        Type::TypeRef { name, args } if args.is_empty() => {
            match bang.iter().find(|(n, _)| n == name) {
                Some((_, t)) => t.clone(),
                None => Type::Error,
            }
        }
        Type::TypeRef { name, args } => Type::TypeRef {
            name: name.clone(),
            args: args.iter().map(|a| thay_gen(a, bang)).collect(),
        },
        Type::List(t) => Type::List(Box::new(thay_gen(t, bang))),
        Type::Map(k, v) => Type::Map(Box::new(thay_gen(k, bang)), Box::new(thay_gen(v, bang))),
        Type::Opt(t) => Type::Opt(Box::new(thay_gen(t, bang))),
        Type::Result(a, b) => {
            Type::Result(Box::new(thay_gen(a, bang)), Box::new(thay_gen(b, bang)))
        }
        Type::Fn(ps, r) => Type::Fn(
            ps.iter()
                .map(|p| thay_gen(p, bang))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            Box::new(thay_gen(r, bang)),
        ),
        other => other.clone(),
    }
}

/// Kiểm kiểu một chương trình đã hạ.
pub fn check(hir: &Hir, symbols: &Symbols, types: &TypeTable) -> CheckOutput {
    // **VD-269**: bảng phương thức `impl` (kiểu nhận + tên → `sym`) để `recv?.m(…)` phân giải
    // được phương thức — lowering **không** dispatch callee `Field` optional.
    let mret = crate::MethodRet::from_hir_voi_bang(hir, types);
    let mut c = Checker::new(symbols, types, mret);
    c.run(hir);
    c.finish()
}

pub(crate) struct Checker<'a> {
    symbols: &'a Symbols,
    types: &'a TypeTable,
    diags: Vec<Diagnostic>,
    /// `env[sym]` = kiểu của ký hiệu `sym` (chỉ số trong [`Symbols::syms`]).
    env: Vec<Option<Type>>,
    /// `mutable[sym]` = gán được (`var`, `var self`).
    mutable: Vec<bool>,
    types_of_expr: Vec<Type>,
    file_expr_types: Vec<Type>,
    item_types: Vec<(String, Type)>,
    /// Kiểu trả về của hàm đang kiểm (`None` = đang ở cấp tệp).
    cur_ret: Option<Type>,
    /// >0 ⇒ **không** ghi chẩn đoán (lượt khai báo trước, tránh báo trùng — ST-12b).
    quiet: u32,
    /// Chữ ký đầy đủ của hàm **khai trong tệp** (tên tham số + có mặc định), theo `sym`.
    sigs: Vec<Option<FnSig>>,
    /// Tham số kiểu đang có hiệu lực (ST-28) — `resolve_ann` giữ `TypeRef{"T"}` thay vì `VS0301`.
    generic_scope: Vec<String>,
    /// **VD-259** — độ sâu vòng lặp **tĩnh** trong hàm hiện tại (`break`/`continue` ngoài vòng ⇒
    /// `VS0412`). Đặt lại **0** khi vào hàm/lambda: `break` không xuyên qua biên hàm.
    loop_depth: u32,
    /// **VD-259** — đang kiểm câu lệnh ở **vị trí cần giá trị** (câu cuối của khối cho giá trị,
    /// thân nhánh `match`, nhánh `if` biểu thức). `break`/`continue` ở đây ⇒ `VS0412`.
    in_value: bool,
    /// **VD-259** — use-set của thân hàm hiện tại (cho or-pattern trong `let`).
    fn_uses: BTreeSet<u32>,
    /// **VD-262** — đang kiểm thân phương thức: phép gán đã do `kiem_impl` (lượt khai báo) kiểm
    /// đích nên lượt này lược chẩn đoán gán để không báo trùng (vẫn kiểm biểu thức hai bên).
    bo_qua_gan_pt: bool,
    /// **VD-269** — bảng phương thức người dùng (`impl`) theo kiểu nhận + tên.
    mret: crate::MethodRet,
}

impl<'a> Checker<'a> {
    fn new(symbols: &'a Symbols, types: &'a TypeTable, mret: crate::MethodRet) -> Self {
        let n = symbols.syms.len();
        Self {
            generic_scope: Vec::new(),
            symbols,
            types,
            diags: Vec::new(),
            env: vec![None; n],
            mutable: vec![false; n],
            types_of_expr: Vec::new(),
            file_expr_types: Vec::new(),
            item_types: Vec::new(),
            cur_ret: None,
            quiet: 0,
            sigs: vec![None; n],
            loop_depth: 0,
            in_value: false,
            fn_uses: BTreeSet::new(),
            bo_qua_gan_pt: false,
            mret,
        }
    }

    fn finish(self) -> CheckOutput {
        CheckOutput {
            diags: self.diags,
            types_of_expr: self.types_of_expr,
            file_expr_types: self.file_expr_types,
            item_types: self.item_types,
        }
    }

    // ----- chẩn đoán + tiện ích -----

    /// Ghi chẩn đoán. `line`/`col` để **0**: `check` chỉ có HIR (không có văn bản nguồn) —
    /// tầng `vis` (ST-14) tính lại từ `span` khi in. Ghi rõ ở `JOURNAL` ST-12.
    pub(crate) fn diag(
        &mut self,
        code: &'static vis_diag::Code,
        msg: impl Into<String>,
        span: Span,
    ) {
        self.diags.push(Diagnostic {
            code,
            message: msg.into(),
            span,
            line: 0,
            col: 0,
        });
    }

    /// **VD-266**: literal `int` phải biểu diễn được trong `i64` **sau khi gộp dấu**. Lexer cho
    /// qua độ lớn `2^63` để `-2^63` thành `i64::MIN`; literal **dương** `2^63` (không dấu, hoặc
    /// bị `--` đảo lại) vẫn là `VS0104` **tĩnh** — cả `vis check` lẫn hai engine dừng trước runtime.
    pub(crate) fn kiem_int_lit(&mut self, lit: &crate::hir::Lit, span: Span) {
        if lit.kind == crate::hir::LitKind::Int
            && vis_syntax::lit::decode_signed_int(&lit.text).is_err()
        {
            self.diag(INT_LIT, vis_diag::CODES[3].message_vi, span);
        }
    }

    /// Chữ ký của hàm theo `sym` (nếu là hàm khai trong tệp).
    pub(crate) fn sig_of(&self, sym: u32) -> Option<&FnSig> {
        self.sigs.get(sym as usize).and_then(|s| s.as_ref())
    }

    pub(crate) fn note(&mut self, ty: Type) -> Type {
        self.types_of_expr.push(ty.clone());
        ty
    }

    // ----- thăm dò môi trường -----

    pub(crate) fn ty_of(&self, sym: u32) -> Type {
        self.env
            .get(sym as usize)
            .and_then(|t| t.clone())
            .unwrap_or(Type::Error)
    }

    pub(crate) fn set_ty(&mut self, sym: u32, ty: Type) {
        if let Some(slot) = self.env.get_mut(sym as usize) {
            *slot = Some(ty);
        }
    }

    pub(crate) fn set_mutable(&mut self, sym: u32) {
        if let Some(slot) = self.mutable.get_mut(sym as usize) {
            *slot = true;
        }
    }

    fn is_mutable(&self, sym: u32) -> bool {
        self.mutable.get(sym as usize).copied().unwrap_or(false)
    }

    // ----- kiểu: so khớp, tự nâng, tên kiểu -----

    /// ST-28 — **hợp nhất** kiểu khai báo generic với kiểu đối số thật; trả `false` (và báo
    /// `VS0412`) khi mâu thuẫn. `bang` giữ `T → int` đã suy.
    pub(crate) fn hop_nhat(
        &mut self,
        mau: &Type,
        that: &Type,
        gens: &[String],
        bang: &mut Vec<(String, Type)>,
        ten: &str,
        span: Span,
    ) -> bool {
        if matches!(that, Type::Error) {
            return true;
        }
        match mau {
            Type::TypeRef { name, args } if args.is_empty() && gens.iter().any(|g| g == name) => {
                match bang.iter().find(|(n, _)| n == name) {
                    Some((_, da)) => {
                        if self.same(da, that) {
                            true
                        } else {
                            let msg = format!(
                                "`{ten}`: tham số kiểu `{name}` đã suy là `{}`, đối số này có `{}`",
                                self.types.show(da),
                                self.types.show(that)
                            );
                            self.diag(MISMATCH, msg, span);
                            false
                        }
                    }
                    None => {
                        bang.push((name.clone(), that.clone()));
                        true
                    }
                }
            }
            // ST-35: `Hop[T]` — thay ràng buộc đã biết; khớp theo **instantiation** (id riêng) nên
            // `Hop[int]` ≠ `Hop[str]`; chưa suy được `T` ⇒ **bỏ qua** (giữ hành vi xoá kiểu ST-28).
            Type::TypeRef { name, args } if !args.is_empty() => {
                // **VD-270**: kiểu nhận của phương thức `impl Hop[T]` là `TypeRef{"Hop",[T]}`;
                // đối số thật thường là **instantiation** `Record(id)` (không phải `TypeRef`) ⇒
                // buộc `T` theo **vị trí** từ `rec_inst_of`/`enum_inst_of`. Nhờ vậy `a.get()`
                // trả `int` (không còn ký hiệu `T`) và đối số sau bị kiểm theo `T` đã suy.
                let inst = match that {
                    Type::Record(id) => self.types.rec_inst_of(*id).cloned(),
                    Type::Enum(id) => self.types.enum_inst_of(*id).cloned(),
                    _ => None,
                };
                if let Some((ten, ds)) = inst {
                    if ten == *name && ds.len() == args.len() {
                        let mut ok = true;
                        for (mau, that_arg) in args.iter().zip(ds.iter()) {
                            if !self.hop_nhat(mau, that_arg, gens, bang, &ten, span) {
                                ok = false;
                            }
                        }
                        return ok;
                    }
                }
                let cu: Vec<Type> = args.iter().map(|a| thay_gen(a, bang)).collect();
                if cu
                    .iter()
                    .any(|t| matches!(t, Type::TypeRef { args, .. } if args.is_empty()))
                {
                    return true;
                }
                if let Some(id) = self.types.find_rec_inst(name, &cu) {
                    return self.hop_nhat(&Type::Record(id), that, gens, bang, ten, span);
                }
                if let Some(id) = self.types.find_enum_inst(name, &cu) {
                    return self.hop_nhat(&Type::Enum(id), that, gens, bang, ten, span);
                }
                // Chưa có instantiation trong bảng (vd chưa ai viết `let x: Hop[int]`): khớp
                // **cấu trúc** theo vị trí tham số của trường ⇒ vẫn buộc được `T` và báo sai kiểu.
                if let (Type::Record(that_id), Some(base)) = (that, self.types.find_rec_base(name))
                {
                    let (Some(def), Some(that_def)) = (
                        self.types.rec(base).cloned(),
                        self.types.rec(*that_id).cloned(),
                    ) else {
                        return self.hop_nhat_loi(mau, that, ten, span);
                    };
                    let pos = self.types.rec_pos(base).to_vec();
                    if def.fields.len() == that_def.fields.len()
                        && def
                            .fields
                            .iter()
                            .zip(that_def.fields.iter())
                            .all(|(f, g)| f.name == g.name)
                    {
                        let mut ok = true;
                        for (i, g) in that_def.fields.iter().enumerate() {
                            match pos.get(i).copied().flatten() {
                                Some(p) => {
                                    let want = cu.get(p).cloned().unwrap_or(Type::Error);
                                    if !self.hop_nhat(&want, &g.ty, gens, bang, ten, span) {
                                        ok = false;
                                    }
                                }
                                None => {
                                    if !self.same(&def.fields[i].ty, &g.ty) {
                                        ok = false;
                                    }
                                }
                            }
                        }
                        return ok;
                    }
                }
                if self.same(mau, that) {
                    true
                } else {
                    self.hop_nhat_loi(mau, that, ten, span)
                }
            }
            Type::List(a) => match that {
                Type::List(b) => self.hop_nhat(a, b, gens, bang, ten, span),
                _ => self.hop_nhat_loi(mau, that, ten, span),
            },
            Type::Opt(a) => match that {
                Type::Opt(b) => self.hop_nhat(a, b, gens, bang, ten, span),
                _ => true, // tự nâng `T → T?` (§4.4)
            },
            Type::Map(k1, v1) => match that {
                Type::Map(k2, v2) => {
                    self.hop_nhat(k1, k2, gens, bang, ten, span)
                        && self.hop_nhat(v1, v2, gens, bang, ten, span)
                }
                _ => self.hop_nhat_loi(mau, that, ten, span),
            },
            Type::Result(a1, b1) => match that {
                Type::Result(a2, b2) => {
                    self.hop_nhat(a1, a2, gens, bang, ten, span)
                        && self.hop_nhat(b1, b2, gens, bang, ten, span)
                }
                _ => self.hop_nhat_loi(mau, that, ten, span),
            },
            _ => {
                if self.same(mau, that) {
                    true
                } else {
                    self.hop_nhat_loi(mau, that, ten, span)
                }
            }
        }
    }

    fn hop_nhat_loi(&mut self, mau: &Type, that: &Type, ten: &str, span: Span) -> bool {
        let msg = format!(
            "`{ten}`: cần `{}`, được `{}`",
            self.types.show(mau),
            self.types.show(that)
        );
        self.diag(MISMATCH, msg, span);
        false
    }

    /// So khớp **cấu trúc**; `{error}` tương thích mọi kiểu (lan truyền im lặng).
    pub(crate) fn same(&self, a: &Type, b: &Type) -> bool {
        match (a, b) {
            (Type::Error, _) | (_, Type::Error) => true,
            (Type::Record(x), Type::Record(y)) => self.same_record(*x, *y),
            (Type::Enum(x), Type::Enum(y)) => self.same_enum(*x, *y),
            (Type::List(x), Type::List(y)) => self.same(x, y),
            (Type::Map(k1, v1), Type::Map(k2, v2)) => self.same(k1, k2) && self.same(v1, v2),
            (Type::Opt(x), Type::Opt(y)) => self.same(x, y),
            (Type::Result(a1, b1), Type::Result(a2, b2)) => self.same(a1, a2) && self.same(b1, b2),
            (Type::Fn(p1, r1), Type::Fn(p2, r2)) => {
                p1.len() == p2.len()
                    && p1.iter().zip(p2.iter()).all(|(x, y)| self.same(x, y))
                    && self.same(r1, r2)
            }
            (Type::TypeRef { name: n1, args: a1 }, Type::TypeRef { name: n2, args: a2 }) => {
                n1 == n2
                    && a1.len() == a2.len()
                    && a1.iter().zip(a2.iter()).all(|(x, y)| self.same(x, y))
            }
            _ => a == b,
        }
    }

    /// ST-35: bản gốc ↔ bản instantiation **cùng tên enum** ⇒ coi là cùng họ (kiểm đối số ở
    /// `coerce_e` khi là literal); hai instantiation **khác đối số** vẫn **khác** (`Or[int,str] ≠ Or[str,int]`).
    fn same_enum(&self, x: EnumId, y: EnumId) -> bool {
        if x == y {
            return true;
        }
        let nx = self.types.enum_def(x).map(|d| d.name.clone());
        let ny = self.types.enum_def(y).map(|d| d.name.clone());
        match (self.types.enum_inst_of(x), self.types.enum_inst_of(y)) {
            (Some((b, _)), None) => ny.as_deref() == Some(b.as_str()),
            (None, Some((b, _))) => nx.as_deref() == Some(b.as_str()),
            _ => false,
        }
    }

    fn same_record(&self, x: RecId, y: RecId) -> bool {
        if x == y {
            return true;
        }
        let (Some(rx), Some(ry)) = (self.types.rec(x), self.types.rec(y)) else {
            return false;
        };
        // **VD-241 §6/B-B (A0, 2026-11-19)** — bản ghi so khớp theo **tên trường**; thứ tự khai báo
        // không quyết định (`{b: 1, a: "s"}` ≡ `{a: "s", b: 1}`).
        rx.fields.len() == ry.fields.len()
            && rx.fields.iter().all(|af| {
                ry.fields
                    .iter()
                    .find(|bf| bf.name == af.name)
                    .map(|bf| self.same(&af.ty, &bf.ty))
                    .unwrap_or(false)
            })
    }

    /// Ép `from` về `to` **có báo lỗi**: tự nâng `T → T?`, literal `int` ở vị trí `float` (§3.5).
    /// ST-35: `Hop[{error}]` (đã xoá generic) ⇒ bản ghi **gốc** `Hop`; `Or[..]` ⇒ enum gốc.
    fn chuan(&self, t: &Type) -> Type {
        if let Type::TypeRef { name, .. } = t {
            if let Some(id) = self.types.find_rec_base(name) {
                return Type::Record(id);
            }
            if let Some(id) = self.types.find_enum_base(name) {
                return Type::Enum(id);
            }
        }
        t.clone()
    }

    pub(crate) fn coerce(&mut self, from: &Type, to: &Type, span: Span, what: &str) -> bool {
        // ST-13b (mục 1): khối phân kỳ (`never`) nhận mọi kiểu — không có giá trị để sai kiểu.
        if matches!(from, Type::Never) || matches!(to, Type::Never) {
            return true;
        }
        let (from, to) = (&self.chuan(from), &self.chuan(to));
        if self.same(from, to) {
            return true;
        }
        // **VD-241 §3 (A0)**: nâng **một lớp** duy nhất ở chuyển đổi annot/đối số/return;
        // `T??` = `Option[Option[T]]`; chấp nhận khi lõi khớp, đích ≥ 1 lớp `Opt`,
        // `from_layers == to_layers - 1`.
        let mut from_peel = from;
        let mut from_layers = 0usize;
        while let Type::Opt(inner) = from_peel {
            from_layers += 1;
            from_peel = inner;
        }
        let mut to_peel = to;
        let mut to_layers = 0usize;
        while let Type::Opt(inner) = to_peel {
            to_layers += 1;
            to_peel = inner;
        }
        if to_layers >= 1 && from_layers == to_layers - 1 && self.same(from_peel, to_peel) {
            return true;
        }
        // ST-12b (BLOCKER #2): **không** nâng `int → float` đại trà — chỉ literal số nguyên
        // (§3.5). Việc nâng literal do `coerce_e` quyết định theo biểu thức nguồn.
        let msg = format!(
            "{what}: cần `{}`, được `{}`",
            self.types.show(to),
            self.types.show(from)
        );
        self.diag(MISMATCH, msg, span);
        false
    }

    /// **R17 (VD-178)**: điều kiện `if`/`while` chỉ `bool`; list/map/str ⇒ gợi ý `.is_empty()`.
    pub(crate) fn coerce_cond(&mut self, e: &HExpr, t: &Type, what: &str) -> bool {
        if matches!(t, Type::List(_) | Type::Map(..) | Type::Str) {
            let ten = match e {
                HExpr::Var { name, .. } => name.clone(),
                _ => "xs".to_string(),
            };
            let msg = format!(
                "{what}: cần `bool`, được `{}` — gợi ý: `{ten}.is_empty()`",
                self.show(t)
            );
            self.diag(MISMATCH, msg, e.span());
            return false;
        }
        self.coerce(t, &Type::Bool, e.span(), what)
    }

    /// Ép kiểu **có biết biểu thức nguồn**: chỉ nâng `int → float` khi nguồn là **literal** số
    /// nguyên (§3.5, ngoại lệ duy nhất của "không chuyển đổi ngầm").
    pub(crate) fn coerce_e(&mut self, e: &HExpr, from: &Type, to: &Type, what: &str) -> bool {
        // **VD-238** (giữ) + **VD-241 §7**: `None` hợp lệ khi kiểu đích **ngoài cùng** là `Option`.
        if vd238::la_none_bare(e) && !vd238::la_none_dich_opt(to) {
            self.diag(
                MISMATCH,
                format!(
                    "`{what}`: `None` chỉ gán được cho `T?`, được `{}` (VD-238)",
                    self.types.show(to)
                ),
                e.span(),
            );
            return false;
        }
        if matches!((from, to), (Type::Int, Type::Float)) && vd238::is_int_lit(e) {
            return true;
        }
        // ST-35: `Or.Trai(1)` gán vào `Or[int, str]` — kiểm **đối số biến thể** theo bản
        // instantiation (bản gốc đã xoá kiểu nên không tự bắt được).
        if let (HExpr::EnumLit { variant, args, .. }, Type::Enum(inst)) = (e, to) {
            if let Some(def) = self.types.enum_def(*inst).cloned() {
                if let Some(v) = def.variants.iter().find(|v| &v.name == variant) {
                    let fields = v.fields.clone();
                    for (i, a) in args.iter().enumerate() {
                        if let Some(f) = fields.get(i) {
                            let t = self.kiem_expr(a);
                            let want = f.ty.clone();
                            if !self.coerce_e(a, &t, &want, "đối số biến thể") {
                                return false;
                            }
                        }
                    }
                    return true;
                }
            }
        }
        let span = e.span();
        self.coerce(from, to, span, what)
    }

    /// Ép kiểu có **cờ literal** (tầng toán tử chỉ còn kiểu + cờ).
    pub(crate) fn coerce_lit(
        &mut self,
        from: &Type,
        to: &Type,
        span: Span,
        what: &str,
        from_lit: bool,
    ) -> bool {
        if from_lit && matches!((from, to), (Type::Int, Type::Float)) {
            return true;
        }
        self.coerce(from, to, span, what)
    }

    /// Phân giải một kiểu **chú thích**: `TypeRef` chưa biết ⇒ `VS0301` (VD-43 #3).
    pub(crate) fn resolve_ann(&mut self, ty: &Type, span: Span) -> Type {
        match ty {
            Type::Int
            | Type::Float
            | Type::Size
            | Type::Duration
            | Type::Percent
            | Type::Str
            | Type::Bool
            | Type::Bytes
            | Type::Path
            | Type::Char
            | Type::Unit
            | Type::Never
            | Type::Error
            | Type::Record(_)
            | Type::Enum(_) => ty.clone(),
            Type::Opt(inner) => {
                let t = self.resolve_ann(inner, span);
                // **R2 (VD-178)**: `T??` hợp lệ — optional lồng tuỳ ý (`Opt[Opt[T]]`).
                Type::Opt(Box::new(t))
            }
            Type::List(t) => Type::List(Box::new(self.resolve_ann(t, span))),
            Type::Map(k, v) => Type::Map(
                Box::new(self.resolve_ann(k, span)),
                Box::new(self.resolve_ann(v, span)),
            ),
            Type::Result(a, b) => Type::Result(
                Box::new(self.resolve_ann(a, span)),
                Box::new(self.resolve_ann(b, span)),
            ),
            Type::Fn(args, ret) => Type::Fn(
                args.iter()
                    .map(|a| self.resolve_ann(a, span))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                Box::new(self.resolve_ann(ret, span)),
            ),
            Type::TypeRef { name, args } => {
                // ST-28: `T` là **tham số kiểu** đang có hiệu lực ⇒ giữ nguyên làm **dấu** để
                // hợp nhất ở call-site; nơi dùng cho thân hàm sẽ được `erase_gen` xoá thành `{error}`.
                if args.is_empty() && self.generic_scope.iter().any(|g| g == name) {
                    return Type::TypeRef {
                        name: name.clone(),
                        args: Vec::new(),
                    };
                }
                self.resolve_named(name, args, span)
            }
        }
    }

    fn resolve_named(&mut self, name: &str, args: &[Type], span: Span) -> Type {
        // **ST-72 (VD-106 (1))**: `cap[Dir|File|Net|Proc|Env|Term]` — kiểu quyền **dựng sẵn**.
        // Đối số **không** resolve như kiểu thường: `Dir/File/…` **không** có nghĩa ngoài `cap`
        // (tránh biến chúng thành kiểu toàn cục).
        if name == "cap" || name == "Cap" {
            return self.resolve_cap(args, span);
        }
        let resolved: Vec<Type> = args.iter().map(|a| self.resolve_ann(a, span)).collect();
        // **VD-222** — Json=EnumId(0)·Error=RecId(0)·AnyError=RecId(0) (VD-231).
        if args.is_empty() {
            match name {
                "Json" => return Type::Enum(crate::ty::EnumId(0)),
                "Error" => return Type::Record(crate::ty::RecId(0)),
                "AnyError" => return Type::Record(crate::ty::RecId(0)), // VD-231 alias Error
                _ => {}
            }
        }
        match (name, resolved.len()) {
            ("list", 1) => return Type::List(Box::new(resolved[0].clone())),
            ("map", 2) => {
                return Type::Map(Box::new(resolved[0].clone()), Box::new(resolved[1].clone()))
            }
            // **ST-61 (VD-96)**: `Chan[T]` — kênh đệm hữu hạn (kiểu dựng sẵn, như `Task`).
            ("Chan" | "chan", 1) => {
                return Type::TypeRef {
                    name: "Chan".into(),
                    args: vec![resolved[0].clone()],
                }
            }
            // **ST-60 (VD-95 (3))**: `Task[T]` — kiểu dựng sẵn của tác vụ `async`.
            ("Task" | "task", 1) => {
                return Type::TypeRef {
                    name: "Task".into(),
                    args: vec![resolved[0].clone()],
                }
            }
            ("Result" | "result", 2) => {
                return Type::Result(Box::new(resolved[0].clone()), Box::new(resolved[1].clone()))
            }
            ("Option" | "option", 1) => return Type::Opt(Box::new(resolved[0].clone())),
            // ST-11 mã hoá kiểu tuple là `TypeRef{name:"tuple"}` — giữ nguyên (kiểu cấu trúc).
            ("tuple", _) => {
                return Type::TypeRef {
                    name: "tuple".into(),
                    args: resolved,
                }
            }
            (_, 0) => {
                if let Some(t) = Type::builtin(name) {
                    return t;
                }
            }
            _ => {}
        }
        // ST-35: tham số kiểu **đang có hiệu lực** (`T` trong `fn f[T]`) ⇒ giữ ký hiệu.
        if resolved.is_empty() && self.generic_scope.iter().any(|g| g == name) {
            return Type::TypeRef {
                name: name.to_string(),
                args: Vec::new(),
            };
        }
        // ST-35: kiểu **người dùng generic** — instantiation đã dựng trong bảng, hoặc bản gốc.
        if let Some(id) = self.types.find_rec_inst(name, &resolved) {
            return Type::Record(id);
        }
        if let Some(id) = self.types.find_enum_inst(name, &resolved) {
            return Type::Enum(id);
        }
        if resolved.is_empty() {
            if let Some(id) = self.types.find_rec_base(name) {
                return Type::Record(id);
            }
            if let Some(id) = self.types.find_enum_base(name) {
                return Type::Enum(id);
            }
        } else if resolved
            .iter()
            .any(|t| matches!(t, Type::TypeRef { args, .. } if args.is_empty()))
        {
            return Type::TypeRef {
                name: name.to_string(),
                args: resolved,
            };
        }
        // VD-48(a): kiểu **ngoài VS0** (`ref`, `set`, `stream`, …) vẫn là `VS0301` (thông điệp nói rõ "chưa hỗ trợ (VD-43)").
        let msg = if vd238::la_kieu_ngoai_vs0(name) {
            format!(
                "kiểu `{name}` chưa hỗ trợ — thuộc vis-std/vòng 2; dùng kiểu khác hoặc bỏ chú thích"
            )
        } else {
            format!(
                "tên không tồn tại: kiểu `{name}` chưa hỗ trợ (VD-43) và không được khai trong tệp"
            )
        };
        self.diag(NO_TYPE_NAME, msg, span);
        Type::Error
    }

    /// `cap[X]`: đúng 1 đối số, `X` ∈ {Dir, File, Net, Proc, Env, Term}; biểu diễn
    /// `TypeRef{"cap", [TypeRef{X}]}` (rẻ: dùng lại `TypeRef`, không thêm biến thể `Type`).
    fn resolve_cap(&mut self, args: &[Type], span: Span) -> Type {
        const KINDS: [&str; 6] = ["Dir", "File", "Net", "Proc", "Env", "Term"];
        if args.len() != 1 {
            self.diag(
                NO_TYPE,
                format!(
                    "`cap` cần đúng 1 tham số (Dir/File/Net/Proc/Env/Term), được {}",
                    args.len()
                ),
                span,
            );
            return Type::Error;
        }
        let kind = match &args[0] {
            Type::TypeRef { name, args } if args.is_empty() => name.clone(),
            _ => {
                self.diag(
                    NO_TYPE,
                    "tham số `cap[...]` phải là một trong Dir/File/Net/Proc/Env/Term".to_string(),
                    span,
                );
                return Type::Error;
            }
        };
        if !KINDS.contains(&kind.as_str()) {
            self.diag(
                NO_TYPE,
                format!(
                    "kiểu quyền `{kind}` không hợp lệ — chỉ Dir/File/Net/Proc/Env/Term (VD-106)"
                ),
                span,
            );
            return Type::Error;
        }
        Type::TypeRef {
            name: "cap".into(),
            args: vec![Type::TypeRef {
                name: kind,
                args: Vec::new(),
            }],
        }
    }

    // ----- duyệt chương trình -----

    /// **VD-265** — chữ ký phương thức `impl` vào `sigs`/`env` (cùng quy tắc hàm cấp tệp:
    /// giữ `TypeRef{"T"}` khi hợp nhất call-site, thân dùng bản đã xoá generic).
    /// Không đẩy vào `item_types` (tên phương thức không phải item cấp tệp).
    fn dang_ky_sig_phuong_thuc(&mut self, f: &HFn, recv: Option<&Type>) {
        let luu = std::mem::take(&mut self.generic_scope);
        self.generic_scope = f.generics.clone();
        let raw_params: Vec<Type> = f
            .params
            .iter()
            .map(|p| {
                // **VD-270**: `self` của `impl Hop[T]` mang kiểu nhận **còn ký hiệu**
                // (`TypeRef{"Hop", [T]}`) để `hop_nhat` buộc được `T` từ đối số thật
                // (`Hop[int]`) rồi thay vào kiểu trả. HParam.ty vẫn `{error}` (VD-262) nên
                // chỉ chèn ở đây — thân phương thức không đổi chẩn đoán.
                if p.is_self && !f.generics.is_empty() {
                    if let Some(t) = recv {
                        return t.clone();
                    }
                }
                self.resolve_ann(&p.ty, p.span)
            })
            .collect();
        let raw_ret = self.resolve_ann(&f.ret, f.span);
        self.generic_scope = luu;
        let params: Vec<Type> = raw_params
            .iter()
            .map(|t| self.chuan(&erase_gen(t, &f.generics)))
            .collect();
        let ret = self.chuan(&erase_gen(&raw_ret, &f.generics));
        let ty = Type::Fn(params.clone().into_boxed_slice(), Box::new(ret.clone()));
        self.set_ty(f.sym, ty);
        if let Some(slot) = self.sigs.get_mut(f.sym as usize) {
            *slot = Some(FnSig {
                name: f.name.clone(),
                is_async: f.is_async,
                params: f
                    .params
                    .iter()
                    .zip(params.iter())
                    .map(|(p, t)| (p.name.clone(), t.clone(), p.default.is_some(), p.named))
                    .collect(),
                ret,
                variadic: f.params.last().map(|p| p.variadic).unwrap_or(false),
                generics: f.generics.clone(),
                raw_params,
                raw_ret,
            });
        }
    }

    fn run(&mut self, hir: &Hir) {
        self.quiet += 1;
        self.khai_bao_item(hir);
        self.quiet -= 1;
        for item in &hir.items {
            self.kiem_item(item);
        }
        // **VD-259**: or-pattern trong `let` cấp tệp dùng use-set của toàn bộ câu lệnh cấp tệp.
        self.fn_uses = pat::than_uses(&hir.stmts);
        for stmt in &hir.stmts {
            let t = self.kiem_stmt(stmt);
            if let HStmt::Expr(_) = stmt {
                self.file_expr_types.push(t);
            }
        }
    }

    /// Lượt 1 — chữ ký item vào môi trường (để gọi được hàm khai sau).
    fn khai_bao_item(&mut self, hir: &Hir) {
        for item in &hir.items {
            match item {
                HItem::Fn(f) => {
                    // ST-28: chữ ký giữ `TypeRef{"T"}` để hợp nhất ở call-site…
                    let luu = std::mem::take(&mut self.generic_scope);
                    self.generic_scope = f.generics.clone();
                    let raw_params: Vec<Type> = f
                        .params
                        .iter()
                        .map(|p| self.resolve_ann(&p.ty, p.span))
                        .collect();
                    let raw_ret = self.resolve_ann(&f.ret, f.span);
                    self.generic_scope = luu;
                    // …nhưng thân hàm và mọi chỗ khác dùng bản **đã xoá kiểu** (`{error}`).
                    // ST-35: sau khi xoá generic, `Hop[?]` phải quay về **bản ghi gốc** `Hop`
                    // (không để `TypeRef{Hop}` lửng ⇒ trường không tra được trong thân hàm).
                    let params: Vec<Type> = raw_params
                        .iter()
                        .map(|t| self.chuan(&erase_gen(t, &f.generics)))
                        .collect();
                    let ret = self.chuan(&erase_gen(&raw_ret, &f.generics));
                    let ty = Type::Fn(params.clone().into_boxed_slice(), Box::new(ret.clone()));
                    self.set_ty(f.sym, ty.clone());
                    self.item_types.push((f.name.clone(), ty));
                    if let Some(slot) = self.sigs.get_mut(f.sym as usize) {
                        *slot = Some(FnSig {
                            name: f.name.clone(),
                            is_async: f.is_async,
                            params: f
                                .params
                                .iter()
                                .zip(params.iter())
                                .map(|(p, t)| {
                                    (p.name.clone(), t.clone(), p.default.is_some(), p.named)
                                })
                                .collect(),
                            ret,
                            variadic: f.params.last().map(|p| p.variadic).unwrap_or(false),
                            generics: f.generics.clone(),
                            raw_params,
                            raw_ret,
                        });
                    }
                }
                HItem::Rec(r) => {
                    let ty = Type::Record(r.id);
                    self.set_ty(r.sym, ty.clone());
                    self.item_types.push((r.name.clone(), ty));
                }
                HItem::Enum(e) => {
                    let ty = Type::Enum(e.id);
                    self.set_ty(e.sym, ty.clone());
                    self.item_types.push((e.name.clone(), ty));
                }
                HItem::Alias(a) => {
                    // **P0 (VD-170)**: `newtype` chưa hạ ⇒ báo **ngay dòng khai báo**, không im
                    // lặng (trước P0: khai một mình xanh, dùng mới `VS0301` ở dòng dùng).
                    // Chỉ ở tầng tyck ⇒ `vis check --syntax` (cổng tài liệu QD §7.4) vẫn xanh.
                    // **R12a (VD-178)**: `type Email(str)` là cú pháp **mới** — không còn `VS0203`
                    // (từ khoá `newtype` bị parser chặn). Newtype **trong suốt**: kiểu = kiểu trong.
                    let _ = a.is_newtype;
                    let ty = self.resolve_ann(&a.ty, a.span);
                    self.set_ty(a.sym, ty.clone());
                    self.item_types.push((a.name.clone(), ty));
                }
                HItem::Const(c) => {
                    let ty = self.resolve_ann(&c.ty, c.span);
                    self.set_ty(c.sym, ty.clone());
                    self.item_types.push((c.name.clone(), ty));
                }
                HItem::Extern(x) => {
                    let params: Vec<Type> = x
                        .params
                        .iter()
                        .map(|p| self.resolve_ann(&p.ty, p.span))
                        .collect();
                    let ty = Type::Fn(params.into_boxed_slice(), Box::new(Type::Error));
                    self.set_ty(x.sym, ty);
                }
                // **ST-56b (VD-97)**: thân phương thức `impl` kiểm **đích gán** (xem `kiem_impl`).
                //
                // **VD-265**: phương thức cũng có **chữ ký** như hàm cấp tệp — trước đây `sigs`
                // để trống nên `recv.m(…)` (kể cả `self.m()`) trả `{error}` trong checker; nhờ
                // vậy `let x = if …: self.m() else: 5` biết `x: T?` và `match` trên nó hợp lệ.
                HItem::Impl(im) => {
                    for m in &im.methods {
                        self.dang_ky_sig_phuong_thuc(m, Some(&im.ty));
                    }
                    self.kiem_impl(im);
                }
                HItem::Trait(_) | HItem::Use(_) => {}
            }
        }
    }

    fn kiem_item(&mut self, item: &HItem) {
        match item {
            HItem::Fn(f) => {
                // **ST-60 (VD-95 (4))**: `async` chỉ cho hàm cấp tệp — method/`cmd` ⇒ `VS0412` (nợ R10).
                if f.is_async && f.kind != crate::hir::FnKind::Fn {
                    self.diag(
                        ASYNC_SAI_CHO,
                        "`async` chỉ cho hàm cấp tệp (`async fn`) — method/`cmd` chưa hỗ trợ (nợ R10)",
                        f.span,
                    );
                }
                // ST-28: trong thân hàm, tham số kiểu `T` là **đã xoá** (`{error}`) ⇒ phép toán trên
                // `T` được phép tĩnh, sai kiểu thật bắt **lúc chạy** (`VS0501`, nợ VS1b).
                let luu = std::mem::take(&mut self.generic_scope);
                self.generic_scope = f.generics.clone();
                for p in &f.params {
                    let raw = self.resolve_ann(&p.ty, p.span);
                    let ty = self.chuan(&erase_gen(&raw, &f.generics));
                    self.set_ty(p.sym, ty.clone());
                    if p.is_var_self {
                        self.set_mutable(p.sym);
                    }
                    if let Some(d) = &p.default {
                        let dty = self.kiem_expr(d);
                        self.coerce_e(d, &dty, &ty, "giá trị mặc định");
                    }
                }
                let raw_ret = self.resolve_ann(&f.ret, f.span);
                let ret = self.chuan(&erase_gen(&raw_ret, &f.generics));
                self.generic_scope = luu;
                let saved = self.cur_ret.replace(ret.clone());
                // **VD-259**: `break`/`continue` không xuyên biên hàm — đặt lại ngữ cảnh.
                let saved_loop = std::mem::replace(&mut self.loop_depth, 0);
                let saved_value = std::mem::replace(&mut self.in_value, false);
                let saved_uses = std::mem::replace(&mut self.fn_uses, pat::than_uses(&f.body));
                self.kiem_than(&f.body, &ret, f.span);
                self.fn_uses = saved_uses;
                self.in_value = saved_value;
                self.loop_depth = saved_loop;
                self.cur_ret = saved;
            }
            HItem::Rec(r) => truong::kiem_khai_rec(self, r),
            HItem::Enum(e) => truong::kiem_khai_enum(self, e),
            HItem::Const(c) => truong::kiem_const(self, c),
            // **VD-262**: thân phương thức `impl` được kiểm như hàm (ngữ cảnh + luồng + tên).
            HItem::Impl(im) => self.kiem_impl_than(im),
            _ => {}
        }
    }

    /// Kiểu **giá trị** của một thân: kiểu biểu thức **cuối**, không có thì `()` (§5.1).
    pub(crate) fn than_value(&mut self, body: &[HStmt]) -> Type {
        self.kiem_than_ngu_canh(body, true)
    }

    /// Kiểm một câu lệnh; trả "kiểu giá trị" (chỉ `HStmt::Expr` có giá trị).
    pub(crate) fn kiem_stmt(&mut self, s: &HStmt) -> Type {
        match s {
            HStmt::Let {
                pat,
                ty,
                init,
                else_,
                ..
            } => {
                let got = self.kiem_expr(init);
                // **R3b (VD-178)**: `let x = r else:` — bóc `Result[T, E]`/`T?` cho ràng buộc;
                // khối `else` nhận lỗi (`E`) hoặc không có ràng buộc (`T?`).
                // **VD-263**: binder của `else e` xác định theo *pattern*, không theo kiểu vế phải:
                // `Ok(x)`/tên trần trên `Result` ⇒ payload `Err` (E); `Err(e)`/`Some(x)`/`None`/
                // pattern khác ⇒ `()`; `let x = o` trên Option ⇒ `()`. Interp + VM dùng cùng luật
                // (`HPat::else_binds_err`) nên không còn giá trị sai kiểu/không bind.
                let (declared, loi) = match else_ {
                    Some(_) => match &got {
                        Type::Result(t, e) => {
                            let binder = if pat.else_binds_err() {
                                (**e).clone()
                            } else {
                                Type::Unit
                            };
                            ((**t).clone(), binder)
                        }
                        Type::Opt(t) => ((**t).clone(), Type::Unit),
                        Type::Error => (Type::Error, Type::Error),
                        other => {
                            let o = other.clone();
                            self.diag(
                                MISMATCH,
                                format!(
                                    "`let … else` cần `Result[T, E]` hoặc `T?`, được `{}`",
                                    self.show(&o)
                                ),
                                init.span(),
                            );
                            (Type::Error, Type::Error)
                        }
                    },
                    None => (got.clone(), Type::Unit),
                };
                let declared = match ty {
                    Some(ann) => {
                        let ann = self.resolve_ann(ann, init.span());
                        self.coerce_e(init, &declared, &ann, "khai báo `let`");
                        ann
                    }
                    None => declared,
                };
                // **VD-260**: pattern bác được (`Some`, `Ok`, literal, …) phải khớp theo **kiểu đầy
                // đủ** của giá trị (`T?`/`Result`), không phải kiểu đã bóc của `let … else`.
                // `let x = r else:` (tên trần) vẫn nhận kiểu đã bóc theo R3b.
                let pat_subj = if matches!(
                    pat,
                    crate::hir::HPat::Bind { .. } | crate::hir::HPat::Wild { .. }
                ) {
                    declared.clone()
                } else if else_.is_some() {
                    got.clone()
                } else {
                    declared.clone()
                };
                // **VD-259**: or-pattern trong `let` — kiểm theo use-set của thân hàm.
                let uses = self.fn_uses.clone();
                self.kiem_or_bind(pat, &pat_subj, &uses);
                // **VD-260**: `let` không có `else` chỉ nhận pattern **phủ hết**.
                if else_.is_none() {
                    self.kiem_let_phu_het(pat, &pat_subj);
                }
                // **VD-263**: ảnh chụp **trước** khi bind pattern — khối `else` phải được kiểm
                // bằng môi trường trước binding; tên pattern không sống trong `else`.
                // Chỉ chụp khi thật có `else` (không làm chậm mọi `let` thường).
                let truoc = else_.as_ref().map(|_| self.luu_env());
                self.bind_pat(pat, &pat_subj);
                if let Some(e) = else_ {
                    if let Some(truoc) = truoc {
                        self.tra_env(truoc);
                    }
                    // Tên lỗi của `else` chỉ sống trong khối `else`.
                    if let Some(sym) = e.bind_sym {
                        self.set_ty(sym, loi);
                    }
                    let _ = self.kiem_than_ngu_canh(&e.body, false);
                    // **VD-263**: khối `else` phải phân kỳ trên **mọi** nhánh; nếu rơi xuống cuối
                    // thì pattern không có giá trị và hai engine từng lệch nhau (`VS0501` ↔ `()`).
                    if !flow::than_luon_thoat(&e.body) {
                        self.diag(
                            ELSE_KHONG_THOAT,
                            "`let … else`: khối `else` phải thoát trên **mọi nhánh** \
                             (`return`/`break`/`continue`, hoặc `if`/`match` mọi nhánh thoát); \
                             nếu rơi xuống cuối, biến của pattern không có giá trị (VD-263)",
                            e.span,
                        );
                    }
                    // Trả binding của nhánh khớp để các câu sau không báo `VS0301` oan khi
                    // chính khối `else` đã bị từ chối.
                    self.bind_pat(pat, &pat_subj);
                }
                Type::Unit
            }
            HStmt::Var {
                name,
                span,
                sym,
                ty,
                init,
                ..
            } => {
                let got = self.kiem_expr(init);
                let declared = match ty {
                    Some(ann) => {
                        let ann = self.resolve_ann(ann, init.span());
                        self.coerce_e(init, &got, &ann, "khai báo `var`");
                        ann
                    }
                    None => got,
                };
                let _ = name;
                self.set_ty(*sym, declared);
                self.set_mutable(*sym);
                let _ = span;
                Type::Unit
            }
            HStmt::Assign {
                target,
                op,
                value,
                span,
            } => {
                // **VD-262**: thân phương thức — đích gán đã do `kiem_impl` (lượt khai báo) kiểm;
                // ở đây chỉ kiểm biểu thức (bắt tên lạ) để không báo trùng.
                if self.bo_qua_gan_pt {
                    self.kiem_expr(target);
                    self.kiem_expr(value);
                    return Type::Unit;
                }
                let wanted = self.kiem_dich(target, *op);
                let got = self.kiem_expr(value);
                let _ = span;
                match op {
                    crate::hir::AssignOp::Set => {
                        self.coerce_e(value, &got, &wanted, "gán");
                    }
                    _ => {
                        // `+=` … ⇒ kiểm như `target + value` rồi ép về kiểu đích.
                        let op2 = match op {
                            crate::hir::AssignOp::Add => crate::hir::BinOp::Add,
                            crate::hir::AssignOp::Sub => crate::hir::BinOp::Sub,
                            crate::hir::AssignOp::Mul => crate::hir::BinOp::Mul,
                            crate::hir::AssignOp::Div => crate::hir::BinOp::Div,
                            // **R7-fix (VD-178)**: `//=` kiểm như `//` (trước đây rơi vào `%`).
                            crate::hir::AssignOp::FloorDiv => crate::hir::BinOp::FloorDiv,
                            _ => crate::hir::BinOp::Rem,
                        };
                        let v_lit = vd238::is_int_lit(value);
                        if let Some(res) = num::so_hoc(op2, &wanted, &got, false, v_lit) {
                            self.coerce(&res, &wanted, value.span(), "gán ghép");
                        } else if op2 == crate::hir::BinOp::FloorDiv {
                            // **R7-fix (VD-178)**: `//=` chỉ có cho `int` — kiểu khác ⇒ `VS0414`.
                            self.diag(
                                TOAN_TU,
                                format!(
                                    "`//=` không hỗ trợ cho `{}` và `{}` (chỉ `int`)",
                                    self.show(&wanted),
                                    self.show(&got)
                                ),
                                value.span(),
                            );
                        } else {
                            self.coerce_e(value, &got, &wanted, "gán ghép");
                        }
                    }
                }
                Type::Unit
            }
            HStmt::Expr(e) => self.kiem_expr(e),
            HStmt::If {
                cond,
                then,
                elifs,
                else_,
                span,
            } => {
                // VD-47: `if` ở vị trí câu lệnh **vẫn cho giá trị** (`if x: 1 else: 2` là `int`);
                // không có `else` ⇒ `()`. **VD-259**: ngữ cảnh giá trị do `flow.rs` quyết định.
                self.kiem_if_stmt(cond, then, elifs, else_.as_deref(), *span)
            }
            HStmt::For {
                pat, iter, body, ..
            } => {
                let it = self.kiem_expr(iter);
                let elem = self.elem_of_iter(&it, iter.span());
                // **VD-260**: binding của `for` chỉ sống trong vòng; pattern bác được ⇒ từ chối.
                let snap = self.luu_env();
                // **VD-259**: or-pattern trong `for` — use-set là thân vòng lặp.
                let uses = pat::than_uses(body);
                self.kiem_or_bind(pat, &elem, &uses);
                self.kiem_for_phu_het(pat, &elem);
                self.bind_pat(pat, &elem);
                // ST-13b (mục 3): thân vòng lặp **bỏ** giá trị cuối, không ép `unit`.
                self.loop_depth += 1;
                self.kiem_than_bo(body);
                self.loop_depth -= 1;
                self.tra_env(snap);
                Type::Unit
            }
            HStmt::While { cond, body, .. } => {
                // **ST-226 (R18b)**: `while let` dùng chung `kiem_cond` với `if let`.
                let snap = self.luu_env();
                self.loop_depth += 1;
                self.kiem_cond(cond, body);
                self.kiem_than_bo(body);
                self.loop_depth -= 1;
                self.tra_env(snap);
                Type::Unit
            }
            HStmt::Loop { body, .. } => {
                self.loop_depth += 1;
                self.kiem_than_bo(body);
                self.loop_depth -= 1;
                Type::Unit
            }
            HStmt::Scope { body, .. } => {
                // ST-13b (mục 3): `scope` **cho giá trị khối** — thống nhất với interp.
                // **VD-259**: câu cuối chỉ ở vị trí cần giá trị khi chính `scope` đang ở đó.
                let value = self.in_value;
                self.kiem_than_ngu_canh(body, value)
            }
            HStmt::Return { value, span } => {
                let want = self.cur_ret.clone().unwrap_or(Type::Error);
                match value {
                    Some(e) => {
                        let t = self.kiem_expr(e);
                        // **VD-238** (giữ) + **VD-241 §7**: `None` trả về hợp lệ khi kiểu đích **ngoài cùng** là `Option`.
                        if vd238::la_none_bare(e) && !vd238::la_none_dich_opt(&want) {
                            self.diag(
                                MISMATCH,
                                format!(
                                    "`return`: `None` chỉ hợp lệ ở `T?`, được `{}` (VD-238)",
                                    self.types.show(&want)
                                ),
                                *span,
                            );
                        } else if !want.is_error() {
                            self.coerce(&t, &want, *span, "`return`");
                        }
                    }
                    None => {
                        if !want.is_error() && want != Type::Unit {
                            self.coerce(&Type::Unit, &want, *span, "`return`");
                        }
                    }
                }
                // ST-13b (mục 1): `return` **phân kỳ** — kiểu `never`, nhận mọi kiểu trả về.
                Type::Never
            }
            HStmt::Break { span } | HStmt::Continue { span } => {
                // **VD-259**: chỉ trong vòng lặp cùng hàm + ở vị trí câu lệnh (`flow.rs`).
                self.kiem_nhay(matches!(s, HStmt::Break { .. }), *span)
            }
            HStmt::Match {
                subject,
                arms,
                span,
            } => {
                let subj = self.kiem_expr(subject);
                let value = self.in_value;
                let mut acc: Option<Type> = None;
                for arm in arms {
                    // **VD-260**: binding của mỗi nhánh `match` chỉ sống trong nhánh đó.
                    let snap = self.luu_env();
                    let uses = pat::arm_uses(arm.guard.as_ref(), &arm.body);
                    self.kiem_or_bind(&arm.pat, &subj, &uses); // ST-239 / VD-259
                    self.bind_pat(&arm.pat, &subj);
                    if let Some(g) = &arm.guard {
                        let gt = self.kiem_expr(g);
                        self.coerce(&gt, &Type::Bool, g.span(), "guard `if`");
                    }
                    // **VD-259**: thân nhánh chỉ ở vị trí cần giá trị khi `match` đang ở đó.
                    let t = self.kiem_than_ngu_canh(&arm.body, value);
                    self.tra_env(snap);
                    match &mut acc {
                        None => acc = Some(t),
                        Some(a) => self.unify_branch(a, &t, arm.span),
                    }
                }
                self.xet_het(&subj, arms, *span);
                // VD-47: `match` ở vị trí câu lệnh vẫn cho giá trị (hợp nhất các arm).
                acc.unwrap_or(Type::Unit)
            }
            HStmt::Block { body, .. } => {
                // VD-47: khối con ở đuôi **cho giá trị** của nó.
                // **VD-259**: chỉ khi chính khối đang ở vị trí cần giá trị.
                let value = self.in_value;
                self.kiem_than_ngu_canh(body, value)
            }
        }
    }

    /// Kiểm thân khối **bỏ giá trị cuối** (vòng lặp — ST-13b mục 3).
    pub(crate) fn kiem_than_bo(&mut self, body: &[HStmt]) {
        let _ = self.kiem_than_ngu_canh(body, false);
    }
}
