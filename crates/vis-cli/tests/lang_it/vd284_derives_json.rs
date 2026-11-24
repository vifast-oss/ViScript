// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-284/ST-302 (#8)** — `derives Json`: `.to_json()` + `Type.from_json` interp↔VM.

use vis_syntax::{lex, parse, Mode};

fn chay(ten: &str, src: &str) -> (vis_core::RunOutput, vis_core::RunOutput) {
    let p = parse(src, Mode::Script);
    assert!(p.diagnostics.is_empty(), "{ten}: parse {:?}", p.diagnostics);
    let l = lex(src, Mode::Script);
    let h = vis_types::lower_with(
        &p.cst,
        &l,
        vis_types::LowerOptions {
            std_mods: vec!["net".into(), "str".into(), "fs".into()],
            ..Default::default()
        },
    );
    assert!(h.is_clean(), "{ten}: lower {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "{ten}: tyck {:?}", t.diags);
    let i = vis_interp::run(src, &h.hir, &h.symbols, &h.types, &[]);
    let m = vis_bytecode::compile(&h.hir, &h.symbols, src).expect("compile");
    let v = vis_vm::run(&m, src, &[]);
    (i, v)
}

const CASES: &[(&str, &str, &str)] = &[
    ("rec_co_ban", "type U derives Json:\n    name: str\n    age: int\nfn main():\n    let u: U = {name: \"An\", age: 1}\n    print(U.from_json(u.to_json()))\n", "Ok({name: An, age: 1})\n"),
    ("rec_opt_none", "type U derives Json:\n    a: int\n    b: str?\nfn main():\n    let u: U = {a: 1, b: None}\n    print(U.from_json(u.to_json()))\n", "Ok({a: 1, b: None})\n"),
    ("rec_opt_some", "type U derives Json:\n    a: int?\nfn main():\n    let u: U = {a: Some(2)}\n    print(U.from_json(u.to_json()))\n", "Ok({a: Some(2)})\n"),
    ("rec_list", "type U derives Json:\n    xs: list[int]\nfn main():\n    let u: U = {xs: [1, 2, 3]}\n    print(U.from_json(u.to_json()))\n", "Ok({xs: [1, 2, 3]})\n"),
    ("rec_map", "type U derives Json:\n    m: map[str, int]\nfn main():\n    let u: U = {m: {\"a\": 1, \"b\": 2}}\n    print(U.from_json(u.to_json()))\n", "Ok({m: {\"a\": 1, \"b\": 2}})\n"),
    ("rec_long", "type I derives Json:\n    x: int\ntype O derives Json:\n    i: I\n    tag: str\nfn main():\n    let o: O = {i: {x: 3}, tag: \"t\"}\n    print(O.from_json(o.to_json()))\n", "Ok({i: {x: 3}, tag: t})\n"),
    ("enum_unit", "enum E derives Json:\n    A\n    B\nfn main():\n    let a: E = E.A\n    print(a.to_json())\n", "Str(A)\n"),
    ("enum_payload", "enum E derives Json:\n    A(x: int)\nfn main():\n    let a: E = E.A(5)\n    print(E.from_json(a.to_json()))\n", "Ok(A(5))\n"),
    ("enum_multi", "enum E derives Json:\n    P(a: int, b: str)\nfn main():\n    let p: E = E.P(1, \"x\")\n    print(E.from_json(p.to_json()))\n", "Ok(P(1, x))\n"),
    ("float", "type F derives Json:\n    x: float\nfn main():\n    let f: F = {x: 1.5}\n    print(F.from_json(f.to_json()))\n", "Ok({x: 1.5})\n"),
    ("bool_char_path", "type B derives Json:\n    b: bool\n    c: char\n    p: path\nfn main():\n    let x: B = {b: true, c: 'z', p: p\"./a\"}\n    print(B.from_json(x.to_json()))\n", "Ok({b: true, c: 'z', p: ./a})\n"),
    ("sai_kieu", "type U derives Json:\n    a: int\nenum E derives Json:\n    A\nfn main():\n    let e: E = E.A\n    print(U.from_json(e.to_json()))\n", "Err("),
    ("thieu_truong", "type A derives Json:\n    a: int\ntype B derives Json:\n    a: int\n    b: int\nfn main():\n    let a: A = {a: 1}\n    print(B.from_json(a.to_json()))\n", "Err("),
    ("list_record", "type P derives Json:\n    x: int\ntype L derives Json:\n    ps: list[P]\nfn main():\n    let l: L = {ps: [{x: 1}, {x: 2}]}\n    print(L.from_json(l.to_json()))\n", "Ok({ps: [{x: 1}, {x: 2}]})\n"),
    ("map_enum", "enum E derives Json:\n    A\n    B\nfn main():\n    let m: map[str, E] = {\"a\": E.A, \"b\": E.B}\n    print(m)\n", "{\"a\": A, \"b\": B}\n"),
];

#[test]
fn vd284_15_ca_derives_json_parity() {
    for (ten, src, muon) in CASES {
        let (i, v) = chay(ten, src);
        assert_eq!(i.exit, 0, "{ten}: interp {:?}", i.diagnostics);
        assert_eq!(v.exit, 0, "{ten}: VM {:?}", v.diagnostics);
        if *muon == "Err(" {
            assert!(i.stdout.starts_with("Err("), "{ten}: interp {}", i.stdout);
        } else {
            assert_eq!(i.stdout, *muon, "{ten}: interp stdout");
        }
        assert_eq!(v.stdout, i.stdout, "{ten}: parity");
        assert_eq!(i.codes(), v.codes(), "{ten}: parity codes");
    }
    assert_eq!(CASES.len(), 15);
}

#[test]
fn vd284_derives_json_tu_choi_tinh() {
    for (ten, src) in [
        (
            "cap",
            "type C derives Json:\n    c: cap[Dir]\nfn main():\n    print(1)\n",
        ),
        (
            "fn",
            "type F derives Json:\n    f: fn(int) -> int\nfn main():\n    print(1)\n",
        ),
        (
            "chan",
            "type C derives Json:\n    c: Chan[int]\nfn main():\n    print(1)\n",
        ),
    ] {
        let p = parse(src, Mode::Script);
        assert!(p.diagnostics.is_empty(), "{ten}: parse {:?}", p.diagnostics);
        let l = lex(src, Mode::Script);
        let h = vis_types::lower_with(&p.cst, &l, vis_types::LowerOptions::default());
        let t = vis_types::check(&h.hir, &h.symbols, &h.types);
        let mut ma: Vec<&str> = h.diagnostics.iter().map(|d| d.code.short).collect();
        ma.extend(t.diags.iter().map(|d| d.code.short));
        assert!(ma.contains(&"VS0412"), "{ten}: phải VS0412, được {ma:?}");
    }
}
