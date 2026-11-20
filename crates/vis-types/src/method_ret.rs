// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-265** — **bảng kiểu trả về của phương thức người dùng**, dựng từ HIR `impl`.
//!
//! Lowering ST-29 **dispatch tĩnh** `recv.m(…)` thành `HExpr::Call{Var{sym}}` khi biết kiểu nhận,
//! nên phần lớn lời gọi đã có kiểu trả qua `fn_rets`. Bảng này phủ **phần còn lại**:
//!
//! * lời gọi còn dạng `HExpr::Field` (callee là `recv.m`) — HIR dựng từ API hoặc đường hạ
//!   không dispatch được; nhờ bảng, `infer_ty` vẫn biết `T?`/`list[T?]`/bản ghi để nâng `Option`.
//!   **VD-269** mở rộng cho `recv?.m(…)`: lowering vẫn không dispatch callee `Field` optional,
//!   nên checker/interp/VM tra bảng để lấy `sym` (thêm [`MethodRet::sym`]) và kiểu trả.
//! * **thay tham số kiểu** khi `impl` là instantiation (`impl Hop[int]`) hoặc kiểu nhận là
//!   instantiation (`Hop[int]`) — trả về `T` trong khai báo được thay bằng đối số thật (khi
//!   `impl`/kiểu có tham số; xem [`MethodRet::ret`]).
//!
//! Khoá theo yêu cầu VD-265: `(RecId/EnumId hoặc tên kiểu, tên phương thức)`. Bảng **bất biến**
//! sau khi dựng; chỉ mục `HashMap` cho tra nhanh, dữ liệu giữ trong `Vec` nhỏ.
//!
//! Không đổi `Value`/opcode/`VERSION`; đây là **dữ liệu suy kiểu**, không phải bảng ảo.

use std::collections::HashMap;

use crate::hir::{HEnum, HImpl, HItem, HRec};
use crate::ty::{EnumId, RecId, Type, TypeTable};

/// Khoá nhận: `RecId`/`EnumId` cụ thể, hoặc **tên kiểu** (cho `impl Hop[T]`/`TypeRef`).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
enum Khoa {
    Rec(u32),
    Enum(u32),
    Ten(String),
}

/// Một mục: phương thức của một `impl` + kiểu trả **nguyên bản** (còn ký hiệu generic).
#[derive(Clone, Debug)]
struct Muc {
    /// Tên kiểu gốc (`Hop`), dùng để khớp instantiation khi thay generic.
    ten: String,
    /// **VD-269** — `sym` của phương thức (để `recv?.m(…)` gọi thẳng hàm `impl`).
    sym: u32,
    /// Tham số kiểu của `type`/`enum` (rỗng nếu không generic).
    generics: Vec<String>,
    /// Đối số của `impl` (`impl Hop[int]` ⇒ `[int]`; `impl Hop[T]` ⇒ `[TypeRef T]`).
    impl_args: Vec<Type>,
    ret: Type,
}

/// Bảng kiểu trả về phương thức người dùng (**VD-265**).
#[derive(Clone, Debug, Default)]
pub struct MethodRet {
    muc: Vec<Muc>,
    chi_muc: HashMap<(Khoa, String), usize>,
    /// **VD-269** — `(khoá nhận, tên phương thức) → sym` cho **mọi** phương thức `impl` (kể cả
    /// phương thức có kiểu trả `{error}` chưa chốt); runtime cần `sym` để gọi `recv?.m(…)`.
    chi_muc_sym: HashMap<(Khoa, String), u32>,
    /// `RecId`/`EnumId` → tên gốc, gom từ HIR — để tra được cả khi **không** có `TypeTable`
    /// (`vis_bytecode::compile` cũ).
    rec_ten: HashMap<u32, String>,
    enum_ten: HashMap<u32, String>,
    /// **Instantiation** (ST-35) `RecId/EnumId → (tên gốc, đối số)` — chụp từ `TypeTable` khi có,
    /// để đường bytecode **không kèm bảng** vẫn thay được generic.
    rec_inst: HashMap<u32, (String, Vec<Type>)>,
    enum_inst: HashMap<u32, (String, Vec<Type>)>,
}

impl MethodRet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Dựng bảng từ HIR: mọi `HItem::Impl` + phương thức của nó.
    pub fn from_hir(hir: &crate::hir::Hir) -> Self {
        Self::dung(hir, None)
    }

    /// Như [`MethodRet::from_hir`] nhưng chụp thêm bảng **instantiation** từ [`TypeTable`] (đường
    /// `vis run`), nhờ vậy `ret_voi_types` thay được generic khi codegen không nhận bảng.
    pub fn from_hir_voi_bang(hir: &crate::hir::Hir, types: &TypeTable) -> Self {
        Self::dung(hir, Some(types))
    }

    fn dung(hir: &crate::hir::Hir, types: Option<&TypeTable>) -> Self {
        let mut b = Self::default();
        for it in &hir.items {
            match it {
                HItem::Rec(r) => {
                    b.rec_ten.insert(r.id.0, r.name.clone());
                }
                HItem::Enum(e) => {
                    b.enum_ten.insert(e.id.0, e.name.clone());
                }
                _ => {}
            }
        }
        if let Some(t) = types {
            for (i, r) in t.recs().iter().enumerate() {
                if let Some((n, a)) = t.rec_inst_of(RecId(i as u32)) {
                    b.rec_inst.insert(i as u32, (n.clone(), a.clone()));
                } else if let Some(n) = &r.name {
                    b.rec_ten.entry(i as u32).or_insert_with(|| n.clone());
                }
            }
            for (i, e) in t.enums().iter().enumerate() {
                if let Some((n, a)) = t.enum_inst_of(EnumId(i as u32)) {
                    b.enum_inst.insert(i as u32, (n.clone(), a.clone()));
                } else {
                    b.enum_ten.entry(i as u32).or_insert_with(|| e.name.clone());
                }
            }
        }
        for it in &hir.items {
            if let HItem::Impl(im) = it {
                b.them_impl(hir, im);
            }
        }
        b
    }

    /// **VD-269** — `sym` của phương thức `recv.m` (đã bóc `Opt`) nếu tìm được; dùng cho
    /// `recv?.m(…)` ở interp/VM (lowering không dispatch được callee `Field` optional).
    pub fn sym(&self, recv: &Type, method: &str, types: &TypeTable) -> Option<u32> {
        self.tim_sym(recv, method, Some(types))
    }

    /// Như [`MethodRet::sym`] nhưng **không** có [`TypeTable`] (đường `compile` cũ).
    pub fn sym_voi_types(&self, recv: &Type, method: &str) -> Option<u32> {
        self.tim_sym(recv, method, None)
    }

    /// **VD-269** — tra theo **tên kiểu runtime** (`Value::Record`/`Value::Enum`) khi interp
    /// không suy được kiểu tĩnh của biểu thức nhận.
    pub fn sym_theo_ten(&self, ten: &str, method: &str) -> Option<u32> {
        self.chi_muc_sym
            .get(&(Khoa::Ten(ten.to_string()), method.to_string()))
            .copied()
    }

    /// **VD-270** — `sym` theo **tên phương thức duy nhất** trên mọi `impl` (kể cả phương thức
    /// trả `{error}`). Dùng làm fallback khi biến nhận **suy ra** không cho ra kiểu tĩnh
    /// (lambda/generic/kênh): tên phương thức trùng giữa hai `impl` đã bị `VS0302` chặn, nên
    /// chỉ có một `sym` là an toàn; nhiều hơn ⇒ `None` (không đoán).
    pub fn sym_duy_nhat(&self, method: &str) -> Option<u32> {
        let mut sym = None;
        for ((_, ten), s) in &self.chi_muc_sym {
            if ten != method {
                continue;
            }
            match sym {
                None => sym = Some(*s),
                Some(a) if a == *s => {}
                Some(_) => return None,
            }
        }
        sym
    }

    fn them_impl(&mut self, hir: &crate::hir::Hir, im: &HImpl) {
        // Tên + tham số kiểu của kiểu nhận (nếu tìm được trong HIR/instantiation).
        //
        // **VD-270**: `impl Hop[int]` hạ thành `Type::Record(RecId của instantiation)`, không
        // phải `RecId` khai báo ⇒ `tim_rec` không thấy. Trước đây tên rỗng nên `impl` bị **bỏ
        // im lặng** khỏi bảng (`recv?.m` ⇒ `VS0405` oan). Nay lấy tên/đối số từ `rec_inst`/
        // `enum_inst` đã chụp từ `TypeTable`.
        let (ten, generics, impl_args) = match &im.ty {
            Type::Record(id) => {
                let inst = self.rec_inst.get(&id.0).cloned();
                let ten = tim_rec(hir, *id)
                    .map(|r| r.name.clone())
                    .or_else(|| inst.as_ref().map(|(n, _)| n.clone()))
                    .or_else(|| self.rec_ten.get(&id.0).cloned())
                    .unwrap_or_default();
                let g = tim_rec_ten(hir, &ten)
                    .map(|r| r.generics.clone())
                    .unwrap_or_default();
                let a = inst.map(|(_, a)| a).unwrap_or_default();
                (ten, g, a)
            }
            Type::Enum(id) => {
                let inst = self.enum_inst.get(&id.0).cloned();
                let ten = tim_enum(hir, *id)
                    .map(|e| e.name.clone())
                    .or_else(|| inst.as_ref().map(|(n, _)| n.clone()))
                    .or_else(|| self.enum_ten.get(&id.0).cloned())
                    .unwrap_or_default();
                let g = tim_enum_ten(hir, &ten)
                    .map(|e| e.generics.clone())
                    .unwrap_or_default();
                let a = inst.map(|(_, a)| a).unwrap_or_default();
                (ten, g, a)
            }
            Type::TypeRef { name, args } => {
                let g = tim_rec_ten(hir, name)
                    .map(|r| r.generics.clone())
                    .or_else(|| tim_enum_ten(hir, name).map(|e| e.generics.clone()))
                    .unwrap_or_default();
                (name.clone(), g, args.clone())
            }
            _ => (String::new(), Vec::new(), Vec::new()),
        };
        if ten.is_empty() {
            return;
        }
        // Khoá cụ thể (khi `impl` trỏ thẳng Rec/Enum) **và** khoá theo tên (cho instantiation).
        let mut khoa = vec![Khoa::Ten(ten.clone())];
        match &im.ty {
            Type::Record(id) => khoa.push(Khoa::Rec(id.0)),
            Type::Enum(id) => khoa.push(Khoa::Enum(id.0)),
            _ => {}
        }
        for m in &im.methods {
            for k in &khoa {
                self.chi_muc_sym.insert((k.clone(), m.name.clone()), m.sym);
            }
            if matches!(m.ret, Type::Error) {
                continue;
            }
            let i = self.muc.len();
            self.muc.push(Muc {
                ten: ten.clone(),
                sym: m.sym,
                generics: generics.clone(),
                impl_args: impl_args.clone(),
                ret: m.ret.clone(),
            });
            for k in &khoa {
                self.chi_muc.insert((k.clone(), m.name.clone()), i);
            }
        }
    }

    /// Kiểu trả của `recv.m()` nếu biết; `None` ⇒ không đoán (không nâng).
    ///
    /// `recv` được **bóc `Opt`** (gọi phương thức trên bản thân/`?.`), nhận
    /// `Type::Record`/`Type::Enum`/`Type::TypeRef`. Khi kiểu nhận là **instantiation** trùng tên
    /// với `impl` và `impl` có tham số kiểu, các ký hiệu `T` trong kiểu trả được **thay** bằng đối
    /// số thật của instantiation (nếu `impl` khai cụ thể thì dùng đối số của chính `impl`).
    ///
    /// `types` cần cho instantiation (tên + đối số); không có bảng vẫn tra được `RecId`/`EnumId`
    /// gốc và tên khai trong HIR.
    pub fn ret(&self, recv: &Type, method: &str, types: &TypeTable) -> Option<Type> {
        self.tra(recv, method, Some(types))
    }

    /// Như [`MethodRet::ret`] nhưng **không** có `TypeTable` (đường `vis_bytecode::compile` cũ):
    /// chỉ tra `RecId`/`EnumId` khai trong HIR + tên kiểu; không thay được đối số instantiation.
    pub fn ret_voi_types(&self, recv: &Type, method: &str) -> Option<Type> {
        self.tra(recv, method, None)
    }

    /// **VD-269** — `sym` của phương thức `rev.m`, kể cả phương thức có kiểu trả chưa chốt
    /// (`{error}`): chỉ tra **khoá** (kiểu nhận + tên), không cần kiểu trả.
    fn tim_sym(&self, recv: &Type, method: &str, types: Option<&TypeTable>) -> Option<u32> {
        let recv = bo_opt(recv);
        let mut khoa = Vec::new();
        match recv {
            Type::Record(id) => {
                khoa.push(Khoa::Rec(id.0));
                let ten = self
                    .rec_ten
                    .get(&id.0)
                    .cloned()
                    .or_else(|| types.and_then(|t| t.rec(*id).and_then(|r| r.name.clone())))
                    .or_else(|| types.and_then(|t| t.rec_inst_of(*id).map(|(n, _)| n.clone())))
                    .or_else(|| self.rec_inst.get(&id.0).map(|(n, _)| n.clone()));
                if let Some(t) = ten {
                    khoa.push(Khoa::Ten(t));
                }
            }
            Type::Enum(id) => {
                khoa.push(Khoa::Enum(id.0));
                let ten = self
                    .enum_ten
                    .get(&id.0)
                    .cloned()
                    .or_else(|| types.and_then(|t| t.enum_def(*id).map(|e| e.name.clone())))
                    .or_else(|| types.and_then(|t| t.enum_inst_of(*id).map(|(n, _)| n.clone())))
                    .or_else(|| self.enum_inst.get(&id.0).map(|(n, _)| n.clone()));
                if let Some(t) = ten {
                    khoa.push(Khoa::Ten(t));
                }
            }
            Type::TypeRef { name, .. } => khoa.push(Khoa::Ten(name.clone())),
            _ => return None,
        }
        for k in khoa {
            if let Some(&sym) = self.chi_muc_sym.get(&(k, method.to_string())) {
                return Some(sym);
            }
        }
        None
    }

    fn tra(&self, recv: &Type, method: &str, types: Option<&TypeTable>) -> Option<Type> {
        self.tra_muc(recv, method, types).map(|(_, t)| t)
    }

    /// **VD-269** — như [`Self::tra`] nhưng trả cả `sym` (phương thức `impl`) để runtime gọi thẳng.
    fn tra_muc(&self, recv: &Type, method: &str, types: Option<&TypeTable>) -> Option<(u32, Type)> {
        let recv = bo_opt(recv);
        // Khoá tra + (tên instantiation, đối số) để thay generic.
        let (khoa, inst) = match recv {
            Type::Record(id) => {
                let ten = self
                    .rec_ten
                    .get(&id.0)
                    .cloned()
                    .or_else(|| types.and_then(|t| t.rec(*id).and_then(|r| r.name.clone())))
                    .or_else(|| types.and_then(|t| t.rec_inst_of(*id).map(|(n, _)| n.clone())))
                    .or_else(|| self.rec_inst.get(&id.0).map(|(n, _)| n.clone()));
                let mut ks = vec![Khoa::Rec(id.0)];
                if let Some(t) = &ten {
                    ks.push(Khoa::Ten(t.clone()));
                }
                let inst = types
                    .and_then(|t| t.rec_inst_of(*id))
                    .map(|(n, a)| (n.clone(), a.clone()))
                    .or_else(|| self.rec_inst.get(&id.0).cloned())
                    .or_else(|| ten.map(|t| (t, Vec::new())));
                (ks, inst)
            }
            Type::Enum(id) => {
                let ten = self
                    .enum_ten
                    .get(&id.0)
                    .cloned()
                    .or_else(|| types.and_then(|t| t.enum_def(*id).map(|e| e.name.clone())))
                    .or_else(|| types.and_then(|t| t.enum_inst_of(*id).map(|(n, _)| n.clone())))
                    .or_else(|| self.enum_inst.get(&id.0).map(|(n, _)| n.clone()));
                let mut ks = vec![Khoa::Enum(id.0)];
                if let Some(t) = &ten {
                    ks.push(Khoa::Ten(t.clone()));
                }
                let inst = types
                    .and_then(|t| t.enum_inst_of(*id))
                    .map(|(n, a)| (n.clone(), a.clone()))
                    .or_else(|| self.enum_inst.get(&id.0).cloned())
                    .or_else(|| ten.map(|t| (t, Vec::new())));
                (ks, inst)
            }
            Type::TypeRef { name, args } => (
                vec![Khoa::Ten(name.clone())],
                Some((name.clone(), args.clone())),
            ),
            _ => return None,
        };
        for k in khoa {
            let Some(&i) = self.chi_muc.get(&(k, method.to_string())) else {
                continue;
            };
            let m = &self.muc[i];
            // Đối số generic: ưu tiên instantiation của kiểu nhận nếu cùng tên; nếu không, dùng
            // đối số của chính `impl` (vd `impl Hop[int]` khai cụ thể).
            let args = match &inst {
                Some((n, a)) if *n == m.ten && !a.is_empty() => a.clone(),
                _ => m.impl_args.clone(),
            };
            let bang: Vec<(String, Type)> = m.generics.iter().cloned().zip(args).collect();
            return Some((m.sym, self.chot_inst(thay(&m.ret, &bang), types)));
        }
        None
    }

    /// **VD-265** — sau khi thay generic, `TypeRef{"Hop", [int]}` (do `impl Hop[T]` giữ ký hiệu)
    /// được **chốt** về `Record/Enum` instantiation nếu bảng đã biết tổ hợp đối số; nhờ vậy
    /// `?.field`/`with` trên kết quả vẫn tra được trường. `TypeRef` không khớp giữ nguyên.
    fn chot_inst(&self, t: Type, types: Option<&TypeTable>) -> Type {
        match t {
            Type::TypeRef { name, args } => {
                let args: Vec<Type> = args.into_iter().map(|a| self.chot_inst(a, types)).collect();
                if let Some(id) = types.and_then(|t| t.find_rec_inst(&name, &args)) {
                    return Type::Record(id);
                }
                if let Some(id) = types.and_then(|t| t.find_enum_inst(&name, &args)) {
                    return Type::Enum(id);
                }
                if let Some((id, _)) = self
                    .rec_inst
                    .iter()
                    .find(|(_, (n, a))| n == &name && a == &args)
                {
                    return Type::Record(RecId(*id));
                }
                if let Some((id, _)) = self
                    .enum_inst
                    .iter()
                    .find(|(_, (n, a))| n == &name && a == &args)
                {
                    return Type::Enum(EnumId(*id));
                }
                if args.is_empty() {
                    if let Some(id) = types.and_then(|t| t.rec_by_name(&name)) {
                        return Type::Record(id);
                    }
                    if let Some(id) = types.and_then(|t| {
                        t.enums()
                            .iter()
                            .position(|e| e.name == name)
                            .map(|i| EnumId(i as u32))
                    }) {
                        return Type::Enum(id);
                    }
                    if let Some((id, _)) = self.rec_ten.iter().find(|(_, n)| *n == &name) {
                        return Type::Record(RecId(*id));
                    }
                    if let Some((id, _)) = self.enum_ten.iter().find(|(_, n)| *n == &name) {
                        return Type::Enum(EnumId(*id));
                    }
                }
                Type::TypeRef { name, args }
            }
            Type::Opt(x) => Type::Opt(Box::new(self.chot_inst(*x, types))),
            Type::List(x) => Type::List(Box::new(self.chot_inst(*x, types))),
            Type::Map(k, v) => Type::Map(
                Box::new(self.chot_inst(*k, types)),
                Box::new(self.chot_inst(*v, types)),
            ),
            Type::Result(a, b) => Type::Result(
                Box::new(self.chot_inst(*a, types)),
                Box::new(self.chot_inst(*b, types)),
            ),
            other => other,
        }
    }
}

fn tim_rec(hir: &crate::hir::Hir, id: RecId) -> Option<&HRec> {
    hir.items.iter().find_map(|it| match it {
        HItem::Rec(r) if r.id == id => Some(r),
        _ => None,
    })
}

fn tim_enum(hir: &crate::hir::Hir, id: EnumId) -> Option<&HEnum> {
    hir.items.iter().find_map(|it| match it {
        HItem::Enum(e) if e.id == id => Some(e),
        _ => None,
    })
}

fn tim_rec_ten<'a>(hir: &'a crate::hir::Hir, ten: &str) -> Option<&'a HRec> {
    hir.items.iter().find_map(|it| match it {
        HItem::Rec(r) if r.name == ten => Some(r),
        _ => None,
    })
}

fn tim_enum_ten<'a>(hir: &'a crate::hir::Hir, ten: &str) -> Option<&'a HEnum> {
    hir.items.iter().find_map(|it| match it {
        HItem::Enum(e) if e.name == ten => Some(e),
        _ => None,
    })
}

fn bo_opt(t: &Type) -> &Type {
    let mut cur = t;
    while let Type::Opt(inner) = cur {
        cur = inner;
    }
    cur
}

/// Thay ký hiệu generic (`TypeRef{"T"}`) bằng đối số thật, đệ quy trong một kiểu.
fn thay(t: &Type, bang: &[(String, Type)]) -> Type {
    match t {
        Type::TypeRef { name, args } if args.is_empty() => bang
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| t.clone()),
        Type::TypeRef { name, args } => Type::TypeRef {
            name: name.clone(),
            args: args.iter().map(|a| thay(a, bang)).collect(),
        },
        Type::Opt(x) => Type::Opt(Box::new(thay(x, bang))),
        Type::List(x) => Type::List(Box::new(thay(x, bang))),
        Type::Map(k, v) => Type::Map(Box::new(thay(k, bang)), Box::new(thay(v, bang))),
        Type::Result(a, b) => Type::Result(Box::new(thay(a, bang)), Box::new(thay(b, bang))),
        Type::Fn(ps, r) => Type::Fn(
            ps.iter().map(|p| thay(p, bang)).collect(),
            Box::new(thay(r, bang)),
        ),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ha(src: &str) -> crate::HirOutput {
        let src = format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}");
        let lex = vis_syntax::lex(&src, vis_syntax::Mode::Script);
        let out = vis_syntax::parse(&src, vis_syntax::Mode::Script);
        crate::lower(&out.cst, &lex)
    }

    /// `RecId` của tên gốc (bản khai báo, không instantiation).
    fn rec_id(h: &crate::HirOutput, ten: &str) -> RecId {
        h.hir
            .items
            .iter()
            .find_map(|it| match it {
                HItem::Rec(r) if r.name == ten => Some(r.id),
                _ => None,
            })
            .unwrap_or_else(|| panic!("không có type `{ten}`"))
    }

    /// **VD-265** — phương thức bản ghi không generic, trả `int?`/`list[int?]`/bản ghi.
    #[test]
    fn bang_tra_kieu_cho_ban_ghi() {
        let h = ha(
            "type R:\n    f: int?\n\nimpl R:\n    fn io(self) -> int?:\n        None\n    fn xs(self) -> list[int?]:\n        [None]\n    fn mk(self) -> R:\n        R(f: None)\n",
        );
        let b = MethodRet::from_hir(&h.hir);
        let r = Type::Record(rec_id(&h, "R"));
        assert_eq!(
            b.ret(&r, "io", &h.types),
            Some(Type::Opt(Box::new(Type::Int)))
        );
        assert_eq!(
            b.ret(&r, "xs", &h.types),
            Some(Type::List(Box::new(Type::Opt(Box::new(Type::Int)))))
        );
        assert_eq!(b.ret(&r, "mk", &h.types), Some(r.clone()));
        assert_eq!(b.ret(&r, "khong_co", &h.types), None);
    }

    /// **VD-265** — kiểu trả chứa tham số kiểu của `impl` được **thay** bằng đối số của
    /// instantiation nhận (`Hop[int]` ⇒ `T` = `int`).
    #[test]
    fn thay_generic_theo_instantiation() {
        let h = ha(
            "type Hop[T]:\n    v: T\n\nimpl Hop[T]:\n    fn get(self) -> T?:\n        None\n    fn wrap(self) -> Hop[T]:\n        self\n\nlet h: Hop[int] = Hop(v: 1)\n",
        );
        let b = MethodRet::from_hir(&h.hir);
        let inst = h
            .types
            .recs()
            .iter()
            .enumerate()
            .find_map(|(i, _r)| {
                h.types
                    .rec_inst_of(RecId(i as u32))
                    .filter(|(n, a)| n == "Hop" && a == &vec![Type::Int])
                    .map(|_| RecId(i as u32))
            })
            .expect("phải có instantiation Hop[int]");
        let recv = Type::Record(inst);
        assert_eq!(
            b.ret(&recv, "get", &h.types),
            Some(Type::Opt(Box::new(Type::Int)))
        );
        assert_eq!(b.ret(&recv, "wrap", &h.types), Some(Type::Record(inst)));
        // Bảng tự chụp instantiation ⇒ đường **không** TypeTable vẫn thay được (bytecode cũ).
        let b2 = MethodRet::from_hir_voi_bang(&h.hir, &h.types);
        assert_eq!(
            b2.ret_voi_types(&recv, "get"),
            Some(Type::Opt(Box::new(Type::Int)))
        );
    }

    /// **VD-265** — nhận `Type::Opt(Record)` (gọi phương thức trên bản thân `?.`) vẫn tra được.
    #[test]
    fn boc_opt_cua_kieu_nhan() {
        let h = ha("type R:\n    f: int?\n\nimpl R:\n    fn io(self) -> int?:\n        None\n");
        let b = MethodRet::from_hir(&h.hir);
        let r = Type::Record(rec_id(&h, "R"));
        let opt = Type::Opt(Box::new(r));
        assert_eq!(
            b.ret(&opt, "io", &h.types),
            Some(Type::Opt(Box::new(Type::Int)))
        );
    }

    /// **VD-270** — `impl` trên **instantiation** (`impl Hop[int]`) phải vào bảng, kể cả `sym`
    /// (đường không `TypeTable`) và kiểu trả đã thay `T`; nhận `Type::Opt(Record)`.
    #[test]
    fn impl_tren_instantiation_vao_bang() {
        let h = ha(
            "type Hop[T]:\n    v: T\n\nimpl Hop[int]:\n    fn lay(self) -> int:\n        self.v\n    fn tang(self) -> int?:\n        Some(self.v + 1)\n",
        );
        let b = MethodRet::from_hir_voi_bang(&h.hir, &h.types);
        let inst = h
            .types
            .recs()
            .iter()
            .enumerate()
            .find_map(|(i, _r)| {
                h.types
                    .rec_inst_of(RecId(i as u32))
                    .filter(|(n, a)| n == "Hop" && a == &vec![Type::Int])
                    .map(|_| RecId(i as u32))
            })
            .expect("phải có instantiation Hop[int]");
        let recv = Type::Record(inst);
        let lay = h
            .hir
            .items
            .iter()
            .find_map(|it| match it {
                HItem::Impl(i) => i.methods.iter().find(|m| m.name == "lay").map(|m| m.sym),
                _ => None,
            })
            .expect("có phương thức lay");
        assert_eq!(b.sym(&recv, "lay", &h.types), Some(lay));
        assert_eq!(b.sym_voi_types(&recv, "lay"), Some(lay));
        assert_eq!(
            b.ret(&recv, "tang", &h.types),
            Some(Type::Opt(Box::new(Type::Int)))
        );
        let opt = Type::Opt(Box::new(recv));
        assert_eq!(b.sym(&opt, "lay", &h.types), Some(lay));
        assert_eq!(
            b.ret_voi_types(&opt, "tang"),
            Some(Type::Opt(Box::new(Type::Int)))
        );
    }

    /// **VD-270** — fallback theo **tên phương thức duy nhất**: có `lay` ⇒ `sym`; tên lạ ⇒
    /// `None`.
    #[test]
    fn sym_duy_nhat_theo_ten_phuong_thuc() {
        let h = ha(
            "type A:\n    n: int\n\nimpl A:\n    fn lay(self) -> int:\n        self.n\n\ntype B:\n    m: int\n\nimpl B:\n    fn khac(self) -> int:\n        self.m\n",
        );
        let b = MethodRet::from_hir_voi_bang(&h.hir, &h.types);
        let lay = h
            .hir
            .items
            .iter()
            .find_map(|it| match it {
                HItem::Impl(i) => i.methods.iter().find(|m| m.name == "lay").map(|m| m.sym),
                _ => None,
            })
            .expect("có phương thức lay");
        assert_eq!(b.sym_duy_nhat("lay"), Some(lay));
        assert_eq!(b.sym_duy_nhat("khong_co"), None);
    }

    /// **VD-269** — `sym` của phương thức cho `recv?.m(…)`, kể cả kiểu trả `()` (không bị bỏ
    /// như bảng `ret` cũ) và nhận `Type::Opt(Record)`.
    #[test]
    fn sym_phuong_thuc_cho_goi_tuy_chon() {
        let h =
            ha("type R:\n    f: int\n\nimpl R:\n    fn io(self) -> int:\n        1\n    fn ping(self) -> ():\n        ()\n");
        let b = MethodRet::from_hir_voi_bang(&h.hir, &h.types);
        let r = Type::Record(rec_id(&h, "R"));
        let opt = Type::Opt(Box::new(r.clone()));
        let io = h
            .hir
            .items
            .iter()
            .find_map(|it| match it {
                HItem::Impl(i) => i.methods.iter().find(|m| m.name == "io").map(|m| m.sym),
                _ => None,
            })
            .expect("có phương thức io");
        assert_eq!(b.sym(&r, "io", &h.types), Some(io));
        assert_eq!(b.sym(&opt, "io", &h.types), Some(io));
        assert!(b.sym_theo_ten("R", "io").is_some());
        assert!(b.sym(&r, "ping", &h.types).is_some(), "ret `()` vẫn có sym");
        assert_eq!(b.sym(&r, "khong_co", &h.types), None);
    }
}
