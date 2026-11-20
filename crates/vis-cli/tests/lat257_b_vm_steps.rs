// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-257 / stream B** — đo SỐ LỆNH VM (`RunOutput.steps`) trên workload đại diện.
//!
//! Chạy riêng: `cargo test -p vis-cli --test lat257_b_vm_steps -- --ignored --nocapture`.
//! Mọi số là ĐỀ XUẤT / bằng chứng, **KHÔNG** bật budget làm mặc định, không đổi fuel trong code.
//! Workload chạy finite (debug, vài giây); ghi rõ seed/duration trong report.

use vis_syntax::{lex, parse, Mode};
use vis_vm::vm::Vm;

fn tien_trinh(src: &str) -> (vis_types::Hir, vis_types::Symbols, vis_types::TypeTable) {
    let p = parse(src, Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse: {:?}", p.diagnostics);
    let l = lex(src, Mode::Script);
    let h = vis_types::lower_with(
        &p.cst,
        &l,
        vis_types::LowerOptions {
            std_mods: vec!["net".into()],
            ..Default::default()
        },
    );
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    (h.hir, h.symbols, h.types)
}

/// Chạy một workload qua **VM thật**, đọc số lệnh đã thực thi (`steps`). Không bật budget;
/// fuel an toàn (10^7) vẫn là hàng rào cuối — test ghi nhận nếu workload chạm trần đó.
fn do_steps(tag: &str, src: &str) {
    let tag2 = tag.to_string();
    let src2 = src.to_string();
    std::thread::Builder::new()
        .name("vd257-vm-steps".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            let (hir, syms, _tys) = tien_trinh(&src2);
            let m = vis_bytecode::compile(&hir, &syms, &src2).expect("compile");
            let t0 = std::time::Instant::now();
            let mut vm = Vm::new(&m, &src2);
            let vo = vm.run(&[]);
            let elapsed = t0.elapsed();
            let fuel = vis_vm::vm::SAFETY_FUEL;
            let hit_fuel = vo.codes().contains(&"VS0501-FUEL");
            eprintln!(
                "VMSTEPS {tag2}: exit={} steps={} fuel={fuel} hit_fuel={hit_fuel} wall={elapsed:?}",
                vo.exit, vo.steps
            );
            if hit_fuel {
                eprintln!("  diag={:?}", vo.codes());
            }
            assert!(
                hit_fuel || vo.exit == 0,
                "{tag2}: workload phải chạy sạch hoặc chạm fuel, got {:?}",
                vo.diagnostics
            );
        })
        .expect("spawn")
        .join()
        .expect("join");
}

#[test]
#[ignore]
fn measure_vm_instruction_counts() {
    // Scale xuống để nằm dưới fuel 10^7 (đo đặc tính, không chạm trần).
    do_steps("loop-100k", "var i = 0\nwhile i < 100000:\n    i = i + 1\n");
    do_steps(
        "recursion-200",
        "fn rec(n: int) -> int:\n    if n <= 0:\n        0\n    else:\n        rec(n - 1) + 1\nfn main():\n    print(rec(200))\n",
    );
    do_steps(
        "tasks-1000",
        "async fn t() -> int:\n    1\nfn main():\n    var i = 0\n    while i < 1000:\n        let x = spawn t()\n        i = i + 1\n",
    );
    do_steps(
        "expr-chain",
        "fn main():\n    var i = 0\n    while i < 100000:\n        let x = 1 + 2 + 3 + 4 + 5\n        i = i + 1\n",
    );
    // Ca 2M: semantic cost ~18M (dưới budget đề xuất 20M) NHƯNG VM chạm fuel 10^7 ⇒ bị chặn.
    do_steps("loop-2m", "var i = 0\nwhile i < 2000000:\n    i = i + 1\n");
}
