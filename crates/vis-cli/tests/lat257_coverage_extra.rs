// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-283 / VD-257** — khép các ô coverage còn ghi thiếu trong `vd255-coverage-report.md`:
//! `Set`, `Map`, `EnumLit`, interpolation, tuple — chạy **source thật** qua 4 đường, không fallback.

use vis_interp::frame::{run_frame_for_test as run_i, FrameConfig};
use vis_syntax::{lex, parse, Mode};
use vis_vm::vm_frame::run_frame_for_test as run_v;

const SRC: &str = r#"enum E:
    A
    B
let e: E = E.A
let s = set{1, 2}
let m: map[str, int] = {"a": 1}
let t = (1, "x")
let x = 5
print("gia tri {x}")
print(s)
print(m)
print(t)
match e:
    A: print(10)
    B: print(20)
"#;

fn tien_trinh(src: &str) -> (vis_types::Hir, vis_types::Symbols, vis_types::TypeTable) {
    let p = parse(src, Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse: {:?}", p.diagnostics);
    let l = lex(src, Mode::Script);
    let h = vis_types::lower_with(&p.cst, &l, vis_types::LowerOptions::default());
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    (h.hir, h.symbols, h.types)
}

#[test]
fn set_map_enum_interp_tuple_qua_bon_duong() {
    let (hir, syms, tys) = tien_trinh(SRC);
    let m = vis_bytecode::compile(&hir, &syms, SRC).expect("compile");

    let def = vis_interp::run(SRC, &hir, &syms, &tys, &[]);
    assert!(
        def.codes().is_empty(),
        "default interp: {:?}",
        def.diagnostics
    );

    let mut vm = vis_vm::vm::Vm::new(&m, SRC);
    let def_v = vm.run(&[]);
    assert!(
        def_v.diagnostics.is_empty(),
        "default VM: {:?}",
        def_v.diagnostics
    );
    assert_eq!(def.stdout, def_v.stdout, "mặc định I ↔ V lệch");

    let fi = run_i(SRC, &hir, &syms, &tys, &[], FrameConfig::default());
    assert_eq!(fi.exit, 0, "frame interp: {:?}", fi.diagnostics);
    let fv = run_v(&m, SRC, &[], FrameConfig::default());
    assert_eq!(fv.exit, 0, "frame VM: {:?}", fv.diagnostics);

    assert_eq!(fi.stdout, def.stdout, "frame interp lệch mặc định");
    assert_eq!(fv.stdout, def.stdout, "frame VM lệch mặc định");
}

const SRC_LET_ELSE: &str = r#"fn f(o: int?) -> int?:
    let x = o else:
        return None
    x
print(f(Some(5)))
"#;

#[test]
fn let_else_qua_bon_duong() {
    let (hir, syms, tys) = tien_trinh(SRC_LET_ELSE);
    let m = vis_bytecode::compile(&hir, &syms, SRC_LET_ELSE).expect("compile");

    let def = vis_interp::run(SRC_LET_ELSE, &hir, &syms, &tys, &[]);
    assert!(
        def.codes().is_empty(),
        "default interp: {:?}",
        def.diagnostics
    );
    let mut vm = vis_vm::vm::Vm::new(&m, SRC_LET_ELSE);
    let dv = vm.run(&[]);
    assert!(
        dv.diagnostics.is_empty(),
        "default VM: {:?}",
        dv.diagnostics
    );
    assert_eq!(def.stdout, dv.stdout);

    let fi = run_i(SRC_LET_ELSE, &hir, &syms, &tys, &[], FrameConfig::default());
    assert_eq!(fi.exit, 0, "frame interp: {:?}", fi.diagnostics);
    let fv = run_v(&m, SRC_LET_ELSE, &[], FrameConfig::default());
    assert_eq!(fv.exit, 0, "frame VM: {:?}", fv.diagnostics);
    assert_eq!(fi.stdout, def.stdout, "frame interp lệch mặc định");
    assert_eq!(fv.stdout, def.stdout, "frame VM lệch mặc định");
}
