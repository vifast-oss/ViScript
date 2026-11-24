// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-312c (VD-289)** — test `MethodRet` (chuyển từ `vis-hir/src/method_ret.rs`).

use vis_hir::*;
use vis_types::{lower, HirOutput};

fn ha(src: &str) -> HirOutput {
    let src = format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}");
    let lex = vis_syntax::lex(&src, vis_syntax::Mode::Script);
    let out = vis_syntax::parse(&src, vis_syntax::Mode::Script);
    lower(&out.cst, &lex)
}

/// `RecId` của tên gốc (bản khai báo, không instantiation).
fn rec_id(h: &HirOutput, ten: &str) -> RecId {
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
