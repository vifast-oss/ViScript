// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-1a/VS3-1b — đo thủ công**, không phải test tự động (để không thêm `#[ignore]`/skipped
//! vào gate). Chạy:
//!
//! ```text
//! cargo run -p vis-cli --example vs3_budget_measure
//! ```
//!
//! In cost ngữ nghĩa, số lệnh VM và tỷ lệ bước/lệnh cho các workload đại diện. Không bật ngân
//! sách làm mặc định; không đổi runtime.

use vis_interp::rt::fee as ifee;
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
            std_mods: vec!["net".into(), "str".into()],
            ..Default::default()
        },
    );
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    (h.hir, h.symbols, h.types)
}

fn proposed_cost(src: &str) -> u64 {
    let (hir, syms, tys) = tien_trinh(src);
    ifee::enable_measure_for_test(u64::MAX / 4);
    let _ = vis_interp::run(src, &hir, &syms, &tys, &[]);
    let c = ifee::take_counts_for_test();
    ifee::disable_for_test();
    c.iter()
        .zip([1u64, 1, 1, 1, 4, 2, 4, 2])
        .map(|(n, w)| n * w)
        .sum()
}

fn do_do(tag: &str, src: &str) {
    let cost = proposed_cost(src);
    let (hir, syms, _tys) = tien_trinh(src);
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    let mut vm = Vm::new(&m, src);
    let vo = vm.run(&[]);
    println!(
        "VS3-1a {tag}: cost={cost} vm_steps={} ty_le={:.3} fuel={} exit={}",
        vo.steps,
        vo.steps as f64 / cost as f64,
        vis_vm::vm::SAFETY_FUEL,
        vo.exit
    );
}

fn main() {
    let loop_2m =
        "fn main():\n    var n = 0\n    for i in 0..2_000_000:\n        n = n + 1\n    print(n)\n";
    let tasks = "async fn t() -> int:\n    1\nfn main():\n    var i = 0\n    while i < 1000:\n        let x = spawn t()\n        i = i + 1\n    print(i)\n";
    let task_await = "async fn t() -> int:\n    1\nfn main():\n    var i = 0\n    while i < 100:\n        let x = spawn t()\n        let y = await x\n        i = i + 1\n    print(i)\n";
    let workloads: [(&str, &str); 6] = [
        ("loop-2m", loop_2m),
        (
            "loop-2m-literal",
            "fn main():\n    for i in 0..2_000_000:\n        0\n",
        ),
        (
            "loop-2m-ba-gan",
            "fn main():\n    var a = 0\n    var b = 0\n    var c = 0\n    for i in 0..2_000_000:\n        a = 1\n        b = 2\n        c = 3\n",
        ),
        (
            "recursion-200",
            "fn rec(n: int) -> int:\n    if n <= 0:\n        0\n    else:\n        rec(n - 1) + 1\nfn main():\n    print(rec(200))\n",
        ),
        ("tasks-1000", tasks),
        ("task-await-100", task_await),
    ];
    // Đệ quy 200 khung cần stack lớn như test harness.
    std::thread::Builder::new()
        .name("vs3-budget-measure".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            for (tag, src) in workloads {
                do_do(tag, src);
            }
        })
        .expect("spawn measure thread")
        .join()
        .expect("join measure thread");
}
