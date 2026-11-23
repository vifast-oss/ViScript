// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-302b** — `spawn obj.m(args)` (method `async` trong `impl`) + `await`, parity
//! interp↔VM. Giữ nguyên `spawn f(args)` cấp tệp (VD-90/VD-95).

use vis_syntax::{lex, parse, Mode};

struct TienTrinh {
    hir: vis_types::Hir,
    syms: vis_types::Symbols,
    tys: vis_types::TypeTable,
    module: vis_bytecode::Module,
}

fn chuan_bi(ten: &str, src: &str) -> TienTrinh {
    let p = parse(src, Mode::Script);
    assert!(p.diagnostics.is_empty(), "{ten}: parse {:?}", p.diagnostics);
    let l = lex(src, Mode::Script);
    let h = vis_types::lower_with(&p.cst, &l, vis_types::LowerOptions::default());
    assert!(h.is_clean(), "{ten}: lower {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "{ten}: tyck {:?}", t.diags);
    let module = vis_bytecode::compile(&h.hir, &h.symbols, src).expect("compile");
    TienTrinh {
        hir: h.hir,
        syms: h.symbols,
        tys: h.types,
        module,
    }
}

fn chay_khop(ten: &str, src: &str, muon: &str) {
    let d = chuan_bi(ten, src);
    let i = vis_interp::run(src, &d.hir, &d.syms, &d.tys, &[]);
    let v = vis_vm::run(&d.module, src, &[]);
    assert_eq!(i.exit, 0, "{ten}: interp {:?}", i.diagnostics);
    assert_eq!(v.exit, 0, "{ten}: VM {:?}", v.diagnostics);
    assert_eq!(i.stdout, muon, "{ten}: interp stdout");
    assert_eq!(v.stdout, i.stdout, "{ten}: parity stdout");
    assert_eq!(i.codes(), v.codes(), "{ten}: parity codes");
    assert!(i.steps > 0, "{ten}: interp phải chạy thật");
    assert!(v.steps > 0, "{ten}: VM phải chạy thật");
}

const C_LAY: &str =
    "type C:\n    x: int\n\nimpl C:\n    async fn lay(self) -> int:\n        self.x\n";

#[test]
fn spawn_method_self() {
    let src = format!("{C_LAY}\nfn main():\n    let c: C = {{x: 7}}\n    let t = spawn c.lay()\n    print(await t)\n");
    chay_khop("self", &src, "7\n");
}

#[test]
fn spawn_ham_cap_tep_van_chay() {
    let src = "async fn f(n: int) -> int:\n    n + 1\n\nfn main():\n    let t = spawn f(1)\n    print(await t)\n";
    chay_khop("ham cap tep", src, "2\n");
}

#[test]
fn spawn_method_tra_str() {
    let src = "type C:\n    ten: str\n\nimpl C:\n    async fn chao(self, nguoi: str) -> str:\n        nguoi\n\nfn main():\n    let c: C = {ten: \"chao\"}\n    let t = spawn c.chao(\"an\")\n    print(await t)\n";
    chay_khop("tra str", src, "an\n");
}

#[test]
fn spawn_method_var_self_khong_ghi_nguoc() {
    let src = "type C:\n    x: int\n\nimpl C:\n    async fn bump(var self) -> int:\n        self.x = self.x + 1\n        self.x\n\nfn main():\n    var c: C = {x: 1}\n    let t = spawn c.bump()\n    print(await t)\n    print(c.x)\n";
    // Task nhận **bản sao** của `self`: tăng trong tác vụ không ghi ngược `c` của main.
    chay_khop("var self", src, "2\n1\n");
}

#[test]
fn spawn_method_co_tham_so() {
    let src = "type C:\n    x: int\n\nimpl C:\n    async fn cong(self, n: int) -> int:\n        self.x + n\n\nfn main():\n    let c: C = {x: 7}\n    let t = spawn c.cong(5)\n    print(await t)\n";
    chay_khop("tham so", src, "12\n");
}

#[test]
fn spawn_method_tu_trong_method_khac() {
    let src = "type C:\n    x: int\n\nimpl C:\n    async fn lay(self) -> int:\n        self.x\n\n    async fn go(self) -> int:\n        let t = spawn self.lay()\n        await t\n\nfn main():\n    let c: C = {x: 9}\n    print(await c.go())\n";
    chay_khop("self trong method", src, "9\n");
}

#[test]
fn spawn_hai_method_await_lai_tra_cache() {
    let src = "type C:\n    x: int\n\nimpl C:\n    async fn tang(self, n: int) -> int:\n        self.x + n\n\nfn main():\n    let c: C = {x: 10}\n    let a = spawn c.tang(1)\n    let b = spawn c.tang(2)\n    print(await a)\n    print(await b)\n    print(await a)\n";
    chay_khop("cache", src, "11\n12\n11\n");
}

#[test]
fn spawn_method_outcome_loi_la_vs0506() {
    let src = "type C:\n    x: int\n\nimpl C:\n    async fn hong(self) -> int:\n        let z = 0\n        self.x // z\n\nfn main():\n    let c: C = {x: 1}\n    let t = spawn c.hong()\n    print(await t)\n";
    let d = chuan_bi("loi outcome", src);
    let i = vis_interp::run(src, &d.hir, &d.syms, &d.tys, &[]);
    let v = vis_vm::run(&d.module, src, &[]);
    assert_eq!(i.exit, 1, "interp: {:?}", i.diagnostics);
    assert_eq!(v.exit, 1, "VM: {:?}", v.diagnostics);
    assert_eq!(i.codes(), vec!["VS0506"], "interp codes");
    assert_eq!(i.codes(), v.codes(), "parity codes");
    assert!(
        i.diagnostics[0].message.contains("tác vụ"),
        "{:?}",
        i.diagnostics
    );
}

#[test]
fn spawn_method_dong_bo_bi_tu_choi_ro() {
    let src = "type C:\n    x: int\n\nimpl C:\n    fn lay(self) -> int:\n        self.x\n\nfn main():\n    let c: C = {x: 1}\n    let t = spawn c.lay()\n    print(await t)\n";
    let p = parse(src, Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse {:?}", p.diagnostics);
    let l = lex(src, Mode::Script);
    let h = vis_types::lower_with(&p.cst, &l, vis_types::LowerOptions::default());
    assert!(h.is_clean(), "lower {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(!t.is_clean(), "method đồng bộ qua `spawn` phải bị từ chối");
    assert!(
        t.diags
            .iter()
            .any(|d| d.code.short == "VS0412" && d.message.contains("phương thức")),
        "{:?}",
        t.diags
    );
}

#[test]
fn spawn_method_khong_ton_tai_bao_vs0403() {
    let src = "type C:\n    x: int\n\nimpl C:\n    async fn lay(self) -> int:\n        self.x\n\nfn main():\n    let c: C = {x: 1}\n    let t = spawn c.khong_co()\n    print(await t)\n";
    let p = parse(src, Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse {:?}", p.diagnostics);
    let l = lex(src, Mode::Script);
    let h = vis_types::lower_with(&p.cst, &l, vis_types::LowerOptions::default());
    assert!(!h.is_clean(), "phương thức lạ phải bị chặn khi hạ");
    assert!(
        h.diagnostics
            .iter()
            .any(|d| d.code.short == "VS0403" && d.message.contains("không có phương thức")),
        "{:?}",
        h.diagnostics
    );
}

#[test]
fn spawn_method_worker_local_va_parallel_khop() {
    let src = "type C:\n    x: int\n\nimpl C:\n    async fn lay(self) -> int:\n        self.x\n\nfn main():\n    let c: C = {x: 5}\n    let t = spawn c.lay()\n    print(await t)\n";
    let d = chuan_bi("worker", src);
    let il = vis_interp::run_workers_local(src, &d.hir, &d.syms, &d.tys, &[]);
    let vl = vis_vm::run_workers_local_vm(&d.module, src, &[]);
    let ip = vis_interp::run_workers_parallel(src, &d.hir, &d.syms, &d.tys, &[], 2);
    let vp = vis_vm::run_workers_parallel_vm(&d.module, src, &[], 2);
    for (ten, o) in [
        ("interp local", &il),
        ("VM local", &vl),
        ("interp parallel", &ip),
        ("VM parallel", &vp),
    ] {
        assert_eq!(o.exit, 0, "{ten}: {:?}", o.diagnostics);
        assert_eq!(o.stdout, "5\n", "{ten}: stdout");
    }
}

#[test]
fn spawn_method_cancel_khop_hai_engine() {
    let src = "type C:\n    x: int\n\nimpl C:\n    async fn nang(self, n: int) -> int:\n        var s = self.x\n        var i = 0\n        while i < n:\n            s = s + i\n            i = i + 1\n        s\n\nfn main():\n    let c: C = {x: 1}\n    let t = spawn c.nang(2000000)\n    print(await t)\n";
    let d = chuan_bi("cancel", src);
    let i = vis_interp::run_workers_parallel_cancel_after(src, &d.hir, &d.syms, &d.tys, &[], 2, 5);
    let v = vis_vm::run_workers_parallel_vm_cancel_after(&d.module, src, &[], 2, 5);
    assert_eq!(i.exit, 1, "interp cancel: {:?}", i.diagnostics);
    assert_eq!(v.exit, 1, "VM cancel: {:?}", v.diagnostics);
    assert_eq!(i.codes(), v.codes(), "parity mã huỷ");
    assert_eq!(i.codes(), vec!["VS0506"], "mã huỷ");
    assert!(
        i.diagnostics[0].message.contains("huỷ"),
        "{:?}",
        i.diagnostics
    );
    assert!(
        v.diagnostics[0].message.contains("huỷ"),
        "{:?}",
        v.diagnostics
    );
}
