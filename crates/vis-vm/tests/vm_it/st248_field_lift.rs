// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-248 / VD-242 §3.3** — Option coercion ở đích **field** và **index**.
//! Checker đã chấp nhận `u.f = 5` (`f: int?`) / `m["a"] = 7` (`map[str, int?]`), nhưng trước
//! ST-248 cả hai engine lưu giá trị thô (`5`/`7`) thay vì `Some(5)`/`Some(7)`. Test parity
//! theo VD-242 §3.1: chương trình hợp lệ phải in đúng ở **cả hai** engine; chương trình sai
//! kiểu phải bị tyck từ chối (không được sửa VM để nuốt lỗi).

use vis_bytecode::compile;
use vis_interp::RunOutput;

fn chay_interp(text: &str) -> RunOutput {
    let parsed = vis_syntax::parse(text, vis_syntax::Mode::Script);
    assert!(
        parsed.diagnostics.is_empty(),
        "parse: {:?}",
        parsed.diagnostics
    );
    let lexed = vis_syntax::lex(text, vis_syntax::Mode::Script);
    let h = vis_types::lower(&parsed.cst, &lexed);
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    vis_interp::run(text, &h.hir, &h.symbols, &h.types, &[])
}

fn chay_vm(text: &str) -> RunOutput {
    let parsed = vis_syntax::parse(text, vis_syntax::Mode::Script);
    assert!(
        parsed.diagnostics.is_empty(),
        "parse: {:?}",
        parsed.diagnostics
    );
    let lexed = vis_syntax::lex(text, vis_syntax::Mode::Script);
    let h = vis_types::lower(&parsed.cst, &lexed);
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    let m = compile(&h.hir, &h.symbols, text).expect("compile");
    vis_vm::run(&m, text, &[])
}

fn mo_ta(o: &RunOutput) -> String {
    let d: Vec<String> = o
        .diagnostics
        .iter()
        .map(|x| format!("{}:{}:{} {}", x.code_str(), x.line, x.col, x.message))
        .collect();
    format!("stdout={:?} exit={} diag={:?}", o.stdout, o.exit, d)
}

fn parity(ca: &str, src: &str, stdout: &str) {
    let a = chay_interp(src);
    let b = chay_vm(src);
    assert_eq!(mo_ta(&a), mo_ta(&b), "{ca}: interp ↔ VM lệch");
    assert_eq!(a.stdout, stdout, "{ca}: stdout sai (cả hai cùng sai?)");
    assert!(a.diagnostics.is_empty(), "{ca}: không Expected chẩn đoán");
}

fn tu_choi(ca: &str, src: &str, ma: &str) {
    let parsed = vis_syntax::parse(src, vis_syntax::Mode::Script);
    assert!(
        parsed.diagnostics.is_empty(),
        "{ca}: parse: {:?}",
        parsed.diagnostics
    );
    let lexed = vis_syntax::lex(src, vis_syntax::Mode::Script);
    let h = vis_types::lower(&parsed.cst, &lexed);
    assert!(h.is_clean(), "{ca}: lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(
        t.diags.iter().any(|d| d.code_str() == ma),
        "{ca}: thiếu {ma}: {:?}",
        t.diags.iter().map(|d| d.code_str()).collect::<Vec<_>>()
    );
}

// ---- repro 1: field ----
#[test]
fn repro_field_nang_mot_lop() {
    parity(
        "repro-field",
        "type U:\n    f: int?\nfn main():\n    var u = U(f: None)\n    u.f = 5\n    print(u.f)\n",
        "Some(5)\n",
    );
}

// ---- repro 2: index ----
#[test]
fn repro_index_nang_mot_lop() {
    parity(
        "repro-index",
        "fn main():\n    var m: map[str, int?] = {\"a\": None}\n    m[\"a\"] = 7\n    print(m[\"a\"])\n",
        "Some(7)\n",
    );
}

// ---- None lớp ngoài giữ nguyên + không nâng nhiều lớp (field int??) ----
#[test]
fn field_int2_khong_nang_nhieu_lop_va_none_giu_nguyen() {
    parity(
        "field-int2",
        "type U:\n    f: int??\nfn main():\n    var u = U(f: None)\n    u.f = Some(5)\n    print(u.f)\n    u.f = None\n    print(u.f)\n    u.f = Some(None)\n    print(u.f)\n    u.f = Some(Some(9))\n    print(u.f)\n",
        "Some(Some(5))\nNone\nSome(None)\nSome(Some(9))\n",
    );
}

// ---- index int?? ----
#[test]
fn index_int2_khong_nang_nhieu_lop() {
    parity(
        "index-int2",
        "fn main():\n    var m: map[str, int??] = {\"a\": None}\n    m[\"a\"] = Some(7)\n    print(m[\"a\"])\n    m[\"a\"] = None\n    print(m[\"a\"])\n",
        "Some(Some(7))\nNone\n",
    );
}

// ---- đích không optional ⇒ KHÔNG nâng (chống bọc oan) ----
#[test]
fn dich_khong_optional_khong_nang() {
    parity(
        "field-không-opt",
        "type U:\n    age: int\nfn main():\n    var u = U(age: 0)\n    u.age = 5\n    print(u.age)\n    var xs = [1, 2, 3]\n    xs[0] = 9\n    print(xs)\n",
        "5\n[9, 2, 3]\n",
    );
}

// ---- alias `int?` và list index đi cùng đường kiểm kiểu (VD-242 §3.3) ----
#[test]
fn alias_va_list_index() {
    parity(
        "alias-field",
        "type MyOpt = int?\ntype U:\n    f: MyOpt\nfn main():\n    var u = U(f: None)\n    u.f = 5\n    print(u.f)\n",
        "Some(5)\n",
    );
    parity(
        "list-index",
        "fn main():\n    var xs: list[int?] = [None]\n    xs[0] = 5\n    print(xs)\n",
        "[Some(5)]\n",
    );
}

// ---- ca âm: gán sai kiểu vẫn bị từ chối ----
#[test]
fn sai_kieu_bi_tu_choi() {
    tu_choi(
        "field-sai-kieu",
        "type U:\n    f: int?\nfn main():\n    var u = U(f: None)\n    u.f = \"x\"\n",
        "VS0412",
    );
    tu_choi(
        "index-sai-kieu",
        "fn main():\n    var m: map[str, int?] = {\"a\": None}\n    m[\"a\"] = \"x\"\n",
        "VS0412",
    );
    // Không tự nâng **nhiều lớp**: `5` (0 lớp) vào `int??` (2 lớp) ⇒ từ chối.
    tu_choi(
        "field-nhieu-lop-bi-tu-choi",
        "type U:\n    f: int??\nfn main():\n    var u = U(f: None)\n    u.f = 5\n",
        "VS0412",
    );
}
