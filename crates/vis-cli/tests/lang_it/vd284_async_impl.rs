// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-284/ST-302 (#13)** — `async fn` trong `impl` + `async cmd`, parity interp↔VM.

use vis_syntax::{lex, parse, Mode};

fn chay(ten: &str, src: &str) -> (vis_interp::RunOutput, vis_interp::RunOutput) {
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

fn khop(ten: &str, src: &str, muon: &str) {
    let (i, v) = chay(ten, src);
    assert_eq!(i.exit, 0, "{ten}: interp {:?}", i.diagnostics);
    assert_eq!(v.exit, 0, "{ten}: VM {:?}", v.diagnostics);
    assert_eq!(i.stdout, muon, "{ten}: interp stdout");
    assert_eq!(v.stdout, i.stdout, "{ten}: parity");
    assert_eq!(i.codes(), v.codes(), "{ten}: parity codes");
}

#[test]
fn async_method_self() {
    khop(
        "self",
        "type C:\n    x: int\nimpl C:\n    async fn lay(self) -> int:\n        self.x\nfn main():\n    let c: C = {x: 7}\n    print(await c.lay())\n",
        "7\n",
    );
}

#[test]
fn async_method_var_self() {
    khop(
        "var self",
        "type C:\n    x: int\nimpl C:\n    async fn bump(var self) -> int:\n        self.x = self.x + 1\n        self.x\nfn main():\n    var c: C = {x: 1}\n    print(await c.bump())\n",
        "2\n",
    );
}

#[test]
fn async_cmd_await() {
    khop(
        "async cmd",
        "async cmd chao(ten: str):\n    print(\"chao {ten}\")\nfn main():\n    await chao(\"an\")\n",
        "chao an\n",
    );
}
