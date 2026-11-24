// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-283/ST-301 (Pha 2a)** — lambda param/return annotation + `T??` parity interp↔VM.
//!
//! 20 ca bảng: `lambda_param := IDENT [':' type]`, `lambda_params ['->' type] '=>' expr`,
//! auto-lift `T → T?` cho tham số closure, `T??` trong lambda/generic/record.

use vis_syntax::{lex, parse, Mode};

fn chay(ten: &str, src: &str) -> (vis_core::RunOutput, vis_core::RunOutput) {
    let p = parse(src, Mode::Script);
    assert!(
        p.diagnostics.is_empty(),
        "{ten}: parse: {:?}",
        p.diagnostics
    );
    let l = lex(src, Mode::Script);
    let h = vis_types::lower_with(
        &p.cst,
        &l,
        vis_types::LowerOptions {
            std_mods: vec!["net".into(), "str".into(), "fs".into()],
            ..Default::default()
        },
    );
    assert!(h.is_clean(), "{ten}: lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "{ten}: tyck: {:?}", t.diags);
    let i = vis_interp::run(src, &h.hir, &h.symbols, &h.types, &[]);
    let m = vis_bytecode::compile(&h.hir, &h.symbols, src).expect("compile");
    let v = vis_vm::run(&m, src, &[]);
    (i, v)
}

const CASES: &[(&str, &str, &str)] = &[
    ("lam_opt_ident", "fn main():\n    let f = (x: int?) => x\n    print(f(5))\n", "Some(5)\n"),
    ("lam_opt_ret", "fn main():\n    let f = (x: int?) -> int => x ?? 0\n    print(f(5))\n", "5\n"),
    ("lam_str_opt_ret", "fn main():\n    let f = (x: int, y: str?) -> str => y ?? \"d\"\n    print(f(1, \"ok\"))\n", "ok\n"),
    ("lam_none_default", "fn main():\n    let f = (x: int?) => x ?? 0\n    print(f(None))\n", "0\n"),
    ("lam_double_opt", "fn main():\n    let f = (x: int??) => x\n    print(f(Some(Some(3))))\n", "Some(Some(3))\n"),
    ("let_double_opt", "fn main():\n    let a: int?? = Some(Some(2))\n    print(a)\n", "Some(Some(2))\n"),
    ("lam_in_hof", "fn h(f: fn(int?) -> int?) -> int?:\n    f(1)\nfn main():\n    print(h((x: int?) => x))\n", "Some(1)\n"),
    ("rec_double_opt", "type R:\n    a: int??\nfn main():\n    let r: R = {a: Some(Some(4))}\n    print(r)\n", "{a: Some(Some(4))}\n"),
    ("generic_opt", "fn id[T](x: T?) -> T?:\n    x\nfn main():\n    print(id(5))\n", "Some(5)\n"),
    ("lam_ret_double", "fn main():\n    let f = (x: int) -> int?? => Some(Some(x))\n    print(f(9))\n", "Some(Some(9))\n"),
    ("lam_nested", "fn main():\n    let f = (x: int?) => x\n    let g = (h: fn(int?) -> int?) => h\n    print(g(f)(5))\n", "Some(5)\n"),
    ("lam_trailing_comma", "fn main():\n    let f = (x: int?,) => x\n    print(f(6))\n", "Some(6)\n"),
    ("lam_fn_form", "fn main():\n    let f = fn(x: int?) -> int?:\n        x\n    print(f(7))\n", "Some(7)\n"),
    ("lam_list_opt", "fn main():\n    let f = (xs: list[int?]) -> int => xs.len()\n    print(f([Some(1), None]))\n", "2\n"),
    ("lam_map_opt", "fn main():\n    let f = (m: map[str, int?]) -> int => m.len()\n    print(f({\"a\": Some(1)}))\n", "1\n"),
    ("lambda_generic", "fn ap[T](x: T) -> T:\n    x\nfn main():\n    let f = (x: int?) => x\n    print(f(ap(5)))\n", "Some(5)\n"),
    ("lam_str_none", "fn main():\n    let f = (x: str?) -> str? => x\n    print(f(None))\n", "None\n"),
    ("opt_eq_double", "fn main():\n    let a: int?? = Some(Some(1))\n    print(a == Some(Some(1)))\n", "true\n"),
    ("lam_chain", "fn main():\n    let f = (x: int?) -> int? => x\n    let g = (x: int?) -> int? => f(x)\n    print(g(8))\n", "Some(8)\n"),
    ("lam_block", "fn main():\n    let f = (x: int?) -> int => x ?? 0\n    print(f(3))\n", "3\n"),
    ("let_triple_opt", "fn main():\n    let a: int??? = Some(Some(Some(2)))\n    print(a)\n", "Some(Some(Some(2)))\n"),
    ("lam_triple_opt", "fn main():\n    let f = (x: int???) -> int??? => x\n    print(f(Some(Some(Some(4)))))\n", "Some(Some(Some(4)))\n"),
];

#[test]
fn vd283_20_ca_lambda_annot_parity() {
    for (ten, src, muon) in CASES {
        let (i, v) = chay(ten, src);
        assert_eq!(i.exit, 0, "{ten}: interp exit {:?}", i.diagnostics);
        assert_eq!(v.exit, 0, "{ten}: VM exit {:?}", v.diagnostics);
        assert_eq!(i.stdout, *muon, "{ten}: interp stdout");
        assert_eq!(v.stdout, i.stdout, "{ten}: parity stdout");
        assert_eq!(i.codes(), v.codes(), "{ten}: parity codes");
    }
    assert_eq!(CASES.len(), 22);
}

#[test]
fn vd283_lambda_annot_fmt_bat_dong() {
    for (ten, src, _) in CASES {
        let p = parse(src, Mode::Script);
        assert!(p.diagnostics.is_empty(), "{ten}: parse");
        let a = vis_fmt::format(src).expect("fmt");
        let b = vis_fmt::format(&a).expect("fmt 2");
        assert_eq!(a, b, "{ten}: fmt không bất động");
    }
}
