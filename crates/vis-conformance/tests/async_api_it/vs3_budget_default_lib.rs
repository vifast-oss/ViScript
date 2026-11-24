// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-290 (VD-273)** — ngân sách mặc định: cache/bảng phí/crafted + parity 2M (API).
//!
//! Tách từ `vis-cli/tests/async_it/vs3_budget_default.rs` (ST-313c/VD-289);
//! 5 ca CLI/hybrid (`--max-budget`, `--unlimited-budget`, mac_dinh+cache) giữ ở `vis-cli`.

use vis_runtime::rt::fee as ifee;
use vis_syntax::{lex, parse, Mode};

fn tien_trinh(src: &str) -> (vis_hir::Hir, vis_hir::Symbols, vis_hir::TypeTable) {
    let p = parse(src, Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse: {:?}", p.diagnostics);
    let l = lex(src, Mode::Script);
    let h = vis_types::lower_with(
        &p.cst,
        &l,
        vis_types::LowerOptions {
            std_mods: vec!["net".into(), "str".into()],
            ..Default::default()
        },
    );
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    (h.hir, h.symbols, h.types)
}

fn tren_stack_lon<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .name("vs3-budget-default".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(f)
        .expect("spawn test thread")
        .join()
        .expect("join test thread")
}

const LOOP_100K: &str =
    "fn main():\n    var n = 0\n    for i in 0..100_000:\n        n = n + 1\n    print(n)\n";
const LOOP_2M: &str =
    "fn main():\n    var n = 0\n    for i in 0..2_000_000:\n        n = n + 1\n    print(n)\n";

#[test]
fn cache_co_phi_charge_va_khong_ne_bang_cache() {
    let (hir, syms, _tys) = tien_trinh(LOOP_100K);
    let (m, fee) = vis_bytecode::compile_with_fee_voi_bang(&hir, &syms, None, LOOP_100K)
        .expect("compile_with_fee");
    let bytes = vis_bytecode::encode_with_fee(&m, &fee);
    // Artifact instrumented: cửa thường **từ chối** (hàng rào VD-257 giữ nguyên)…
    assert!(
        vis_bytecode::load_verified(&bytes).is_err(),
        "load_verified thường không được nhận artifact version 3"
    );
    // …cửa có bảng phí nhận và cho chạy với ngân sách. Hạ dấu `INSTRUMENTED_VERSION` về
    // `VERSION` sau khi tách bảng phí (bytecode không đổi) — đúng như `cache::nap_co_phi`.
    let (mut m2, fee2) = vis_bytecode::load_verified_with_fee(&bytes).expect("nạp có phí");
    m2.version = vis_bytecode::VERSION;
    let mut vm = vis_vm::vm::Vm::new(&m2, LOOP_100K);
    vm.budget_enable(fee2.clone(), 1000);
    let out = vm.run(&[]);
    assert_eq!(out.codes(), vec!["VS0510"], "{:?}", out.diagnostics);
    assert!(out.stdout.is_empty());

    // Ngân sách đủ ⇒ chạy sạch, cùng stdout.
    let mut vm2 = vis_vm::vm::Vm::new(&m2, LOOP_100K);
    vm2.budget_enable(fee2, 1_000_000);
    let ok = vm2.run(&[]);
    assert_eq!(ok.exit, 0, "{:?}", ok.diagnostics);
    assert_eq!(ok.stdout, "100000\n");
}

#[test]
fn crafted_bytecode_khong_ne_phi() {
    let (hir, syms, _tys) = tien_trinh(LOOP_100K);
    let (m, fee) = vis_bytecode::compile_with_fee_voi_bang(&hir, &syms, None, LOOP_100K)
        .expect("compile_with_fee");
    assert!(
        vis_bytecode::fee::verify_fee(&m, &fee).is_ok(),
        "bảng phí gốc phải hợp lệ"
    );
    // Bảng rỗng (mọi hàm) — tương đương artifact crafted xoá phí.
    let rong = vis_bytecode::fee::FeeTable::new(m.funcs.len());
    assert!(
        vis_bytecode::fee::verify_fee(&m, &rong).is_err(),
        "bảng phí rỗng phải bị từ chối"
    );
    // Xoá phí của đúng hàm có vòng lặp ⇒ CFG không còn điểm phí ⇒ từ chối.
    let mut cat = fee.clone();
    let fi = m
        .funcs
        .iter()
        .position(|f| f.name == "main")
        .expect("có main");
    cat.funcs[fi].clear();
    assert!(
        vis_bytecode::fee::verify_fee(&m, &cat).is_err(),
        "hàm có vòng lặp mà không còn điểm phí phải bị từ chối"
    );
}

#[test]
fn parity_2m_de_quy_da_task_duoi_mac_dinh() {
    tren_stack_lon(|| {
        let de_quy = "fn rec(n: int) -> int:\n    if n <= 0:\n        0\n    else:\n        rec(n - 1) + 1\nfn main():\n    print(rec(200))\n";
        let da_task = "async fn t() -> int:\n    1\nfn main():\n    var i = 0\n    while i < 1000:\n        let x = spawn t()\n        i = i + 1\n    print(i)\n";
        for src in [LOOP_2M, de_quy, da_task] {
            let (hir, syms, tys) = tien_trinh(src);
            ifee::disable_for_test();
            let i = vis_interp::run(src, &hir, &syms, &tys, &[]);
            assert_eq!(i.exit, 0, "interp: {:?}", i.diagnostics);

            let (m, fee) = vis_bytecode::compile_with_fee_voi_bang(&hir, &syms, Some(&tys), src)
                .expect("compile");
            let mut vm = vis_vm::vm::Vm::new(&m, src);
            vm.budget_enable(fee, 40_000_000);
            let v = vm.run(&[]);
            assert_eq!(v.exit, 0, "VM: {:?}", v.diagnostics);
            assert_eq!(v.stdout, i.stdout, "stdout phải khớp");
            assert!(v.codes().is_empty(), "VM không trap: {:?}", v.codes());
        }
    });
}
