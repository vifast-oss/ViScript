// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-1a** — parity step-cap / ngân sách giữa interp và VM.
//!
//! Bằng chứng gốc (VD-256 D9, VD-257-B §3.3): `for i in 0..2_000_000` chạy xong trên interp
//! nhưng VM trap `VS0501-FUEL` ở 10^7 **lệnh** — fuel chặn trước ngân sách ngữ nghĩa. Test ở đây
//! là hợp đồng tối thiểu của VS3-1a:
//!
//! 1. cùng chương trình ⇒ cùng `stdout`/`exit`/chẩn đoán trên hai engine;
//! 2. fuel là **hàng rào an toàn**, không được chặn chương trình còn trong ngân sách;
//! 3. vượt ngân sách ⇒ `VS0501-BUDGET` (không phải `VS0501-FUEL`) trên **cả hai** engine;
//! 4. ngân sách dùng chung cho cả lần chạy: không reset khi `spawn`/`await`/final join.
//!
//! Ngân sách **không bật mặc định**; mọi test ngân sách đi qua hook nội bộ như VD-256/VD-257.

use vis_interp::rt::fee as ifee;
use vis_syntax::{lex, parse, Mode};
use vis_vm::vm::Vm;

/// Chạy thân test trên luồng có stack lớn: đệ quy 200 khung của interp vượt stack test mặc định
/// (2 MB), còn runtime thật chạy trên `main` của tiến trình. Chỉ là chi tiết harness, không đổi
/// ngữ nghĩa.
fn tren_stack_lon<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .name("vs3-budget".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(f)
        .expect("spawn test thread")
        .join()
        .expect("join test thread")
}

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

fn chay_interp(src: &str) -> vis_interp::RunOutput {
    let (hir, syms, tys) = tien_trinh(src);
    vis_interp::run(src, &hir, &syms, &tys, &[])
}

fn chay_vm(src: &str) -> vis_interp::RunOutput {
    let (hir, syms, _tys) = tien_trinh(src);
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    let mut vm = Vm::new(&m, src);
    vm.run(&[])
}

/// **Ca gốc VS3-1a**: 2 triệu vòng `for` range. Interp chạy sạch; VM từng trap fuel ở 10^7 lệnh.
const LOOP_2M: &str =
    "fn main():\n    var n = 0\n    for i in 0..2_000_000:\n        n = n + 1\n    print(n)\n";

#[test]
fn loop_2m_hai_engine_cung_ket_qua_khong_trap_fuel() {
    tren_stack_lon(|| {
        let i = chay_interp(LOOP_2M);
        assert_eq!(i.exit, 0, "interp: {:?}", i.diagnostics);
        assert_eq!(i.stdout, "2000000\n", "interp stdout");
        assert!(i.codes().is_empty(), "interp: {:?}", i.codes());

        let v = chay_vm(LOOP_2M);
        assert_eq!(
            v.codes(),
            Vec::<&str>::new(),
            "VM không được trap fuel cho chương trình còn trong ngân sách; steps={}",
            v.steps
        );
        assert_eq!(v.exit, i.exit, "exit phải khớp interp");
        assert_eq!(v.stdout, i.stdout, "stdout phải khớp interp");
    });
}

/// Đệ quy sâu 200 khung: cap **độ sâu** là giới hạn riêng, không phải ngân sách/fuel.
#[test]
fn de_quy_200_parity_hai_engine() {
    tren_stack_lon(|| {
        let src = "fn rec(n: int) -> int:\n    if n <= 0:\n        0\n    else:\n        rec(n - 1) + 1\nfn main():\n    print(rec(200))\n";
        let i = chay_interp(src);
        let v = chay_vm(src);
        assert_eq!(i.exit, 0, "interp: {:?}", i.diagnostics);
        assert_eq!(i.stdout, "200\n");
        assert_eq!(v.exit, i.exit);
        assert_eq!(v.stdout, i.stdout);
        assert!(v.codes().is_empty(), "VM: {:?}", v.codes());
    });
}

/// Nhiều task: 1000 `spawn` + final join. Ngân sách là **một pool** cho cả lần chạy.
const NHIEU_TASK: &str = "async fn t() -> int:\n    1\nfn main():\n    var i = 0\n    while i < 1000:\n        let x = spawn t()\n        i = i + 1\n    print(i)\n";

#[test]
fn nhieu_task_parity_hai_engine() {
    tren_stack_lon(|| {
        let i = chay_interp(NHIEU_TASK);
        let v = chay_vm(NHIEU_TASK);
        assert_eq!(i.exit, 0, "interp: {:?}", i.diagnostics);
        assert_eq!(i.stdout, "1000\n");
        assert_eq!(v.exit, i.exit);
        assert_eq!(v.stdout, i.stdout);
        assert!(v.codes().is_empty(), "VM: {:?}", v.codes());
    });
}

/// Vượt ngân sách phải báo `VS0501-BUDGET`, **không** phải `VS0501-FUEL` — ở cả hai engine.
/// Dùng vòng nhỏ để test nhanh; ca 2M ở trên mới là ca biên fuel.
#[test]
fn vuot_ngan_sach_bao_budget_khong_bao_fuel_hai_engine() {
    tren_stack_lon(|| {
        let src = "fn main():\n    var n = 0\n    for i in 0..100_000:\n        n = n + 1\n    print(n)\n";
        let (hir, syms, tys) = tien_trinh(src);
        let cost = proposed_cost(src);
        assert!(cost > 1, "cost phải đo được");

        ifee::enable_measure_for_test(cost - 1);
        let i = vis_interp::run(src, &hir, &syms, &tys, &[]);
        ifee::disable_for_test();
        assert_ne!(i.exit, 0, "interp: vượt ngân sách phải lỗi");
        assert_eq!(
            i.codes(),
            vec!["VS0501-BUDGET"],
            "interp: {:?}",
            i.diagnostics
        );

        let (m, fee) = vis_bytecode::compile_with_fee(&hir, &syms, src).unwrap();
        let mut vm = Vm::new(&m, src);
        vm.budget_enable_for_test(fee, cost - 1);
        let v = vm.run(&[]);
        assert_ne!(v.exit, 0, "VM: vượt ngân sách phải lỗi");
        assert_eq!(v.codes(), vec!["VS0501-BUDGET"], "VM: {:?}", v.diagnostics);
    });
}

/// Chi phí ngữ nghĩa theo bảng trọng số v1 (đề xuất VD-256, chưa chốt) — đo bằng hook, không
/// lưu trace. Cùng định nghĩa sự kiện cho interp và compiler/VM.
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

/// **Một nguồn hằng số** cho interp/compiler/VM + bất biến định cỡ fuel của VS3-1a.
#[test]
fn hang_so_ngan_sach_fuel_mot_nguon_va_bat_bien() {
    use vis_bytecode::fee as bfee;
    assert_eq!(
        ifee::VM_SAFETY_FUEL,
        bfee::VM_SAFETY_FUEL,
        "interp và compiler/VM phải thấy cùng một hằng số"
    );
    assert_eq!(
        vis_vm::vm::SAFETY_FUEL as u64,
        bfee::VM_SAFETY_FUEL,
        "VM phải dùng fuel từ hợp đồng chung, không hardcode"
    );
    assert_eq!(vis_vm::vm::MAX_STEPS, vis_vm::vm::SAFETY_FUEL);
    const {
        assert!(
            bfee::VM_SAFETY_FUEL >= bfee::BUDGET_SIZING_CEILING * bfee::VM_STEPS_PER_COST_CEILING,
            "bất biến: fuel ≥ trần ngân sách định cỡ × tỷ lệ bước/lệnh trần"
        );
        assert!(
            bfee::BUDGET_SIZING_CEILING >= bfee::MAX_LOOP_ITERATIONS,
            "trần ngân sách định cỡ phải phủ ít nhất một cấu trúc lặp tối đa"
        );
    }
}

/// **Biên ngân sách trên ca 2M**: còn ngân sách (đúng trần định cỡ) ⇒ không trap fuel; vượt
/// ngân sách 1 đơn vị ⇒ `VS0501-BUDGET` trên **cả hai** engine, không phải `VS0501-FUEL`.
#[test]
fn loop_2m_bien_ngan_sach_khong_trap_fuel_hai_engine() {
    tren_stack_lon(|| {
        use vis_bytecode::fee as bfee;
        let (hir, syms, tys) = tien_trinh(LOOP_2M);
        let cost = proposed_cost(LOOP_2M);
        assert!(
            cost < bfee::BUDGET_SIZING_CEILING,
            "2M vòng phải nằm dưới trần định cỡ: cost={cost}"
        );

        // (a) interp: ngân sách = trần định cỡ ⇒ chạy xong, không trap.
        ifee::enable_measure_for_test(bfee::BUDGET_SIZING_CEILING);
        let ok_i = vis_interp::run(LOOP_2M, &hir, &syms, &tys, &[]);
        ifee::disable_for_test();
        assert_eq!(ok_i.exit, 0, "interp: {:?}", ok_i.diagnostics);
        assert_eq!(ok_i.stdout, "2000000\n");

        // (b) interp: ngân sách = cost-1 ⇒ BUDGET, không phải FUEL (interp không có fuel).
        ifee::enable_measure_for_test(cost - 1);
        let bad_i = vis_interp::run(LOOP_2M, &hir, &syms, &tys, &[]);
        ifee::disable_for_test();
        assert_eq!(
            bad_i.codes(),
            vec!["VS0501-BUDGET"],
            "interp: {:?}",
            bad_i.diagnostics
        );

        let (m, fee) = vis_bytecode::compile_with_fee(&hir, &syms, LOOP_2M).unwrap();

        // (c) VM: cùng trần định cỡ ⇒ chạy xong; chứng minh fuel không chặn trước ngân sách.
        let mut vm = Vm::new(&m, LOOP_2M);
        vm.budget_enable_measure_for_test(fee.clone(), bfee::BUDGET_SIZING_CEILING);
        let ok_v = vm.run(&[]);
        assert_eq!(ok_v.exit, 0, "VM: {:?}", ok_v.diagnostics);
        assert_eq!(ok_v.stdout, "2000000\n");
        assert!(ok_v.codes().is_empty(), "VM: {:?}", ok_v.codes());
        assert!(
            ok_v.steps > 10_000_000,
            "ca này phải vượt ngưỡng fuel cũ, steps={}",
            ok_v.steps
        );
        assert!(ok_v.steps <= vis_vm::vm::SAFETY_FUEL);

        // (d) VM: ngân sách = cost-1 ⇒ BUDGET thắng fuel (không phải VS0501-FUEL).
        let mut vm2 = Vm::new(&m, LOOP_2M);
        vm2.budget_enable_measure_for_test(fee, cost - 1);
        let bad_v = vm2.run(&[]);
        assert_eq!(
            bad_v.codes(),
            vec!["VS0501-BUDGET"],
            "VM: {:?} (steps={})",
            bad_v.diagnostics,
            bad_v.steps
        );
        assert!(bad_v.steps > 10_000_000, "vẫn phải vượt fuel cũ");
    });
}

/// Nhiều task + `await`: ngân sách là **một pool** cho cả lần chạy, không reset khi spawn/await/
/// final join. Ngân sách `cost-1` phải trap dù thân task chạy ở nhiều "lượt" khác nhau.
const TASK_AWAIT: &str = "async fn t() -> int:\n    1\nfn main():\n    var i = 0\n    while i < 100:\n        let x = spawn t()\n        let y = await x\n        i = i + 1\n    print(i)\n";

#[test]
fn nhieu_task_await_dung_chung_mot_ngan_sach() {
    tren_stack_lon(|| {
        let (hir, syms, tys) = tien_trinh(TASK_AWAIT);
        let cost = proposed_cost(TASK_AWAIT);
        assert!(cost > 1);

        ifee::enable_measure_for_test(cost);
        let ok_i = vis_interp::run(TASK_AWAIT, &hir, &syms, &tys, &[]);
        ifee::disable_for_test();
        assert_eq!(ok_i.exit, 0, "interp: {:?}", ok_i.diagnostics);
        assert_eq!(ok_i.stdout, "100\n");

        ifee::enable_measure_for_test(cost - 1);
        let bad_i = vis_interp::run(TASK_AWAIT, &hir, &syms, &tys, &[]);
        ifee::disable_for_test();
        assert_eq!(
            bad_i.codes(),
            vec!["VS0501-BUDGET"],
            "interp task: {:?}",
            bad_i.diagnostics
        );

        let (m, fee) = vis_bytecode::compile_with_fee(&hir, &syms, TASK_AWAIT).unwrap();
        let mut vm = Vm::new(&m, TASK_AWAIT);
        vm.budget_enable_for_test(fee.clone(), cost);
        let ok_v = vm.run(&[]);
        assert_eq!(ok_v.exit, 0, "VM: {:?}", ok_v.diagnostics);
        assert_eq!(ok_v.stdout, "100\n");

        let mut vm2 = Vm::new(&m, TASK_AWAIT);
        vm2.budget_enable_for_test(fee, cost - 1);
        let bad_v = vm2.run(&[]);
        assert_eq!(
            bad_v.codes(),
            vec!["VS0501-BUDGET"],
            "VM task: {:?}",
            bad_v.diagnostics
        );
    });
}

/// **Final join**: thân task chạy ở cuối chương trình (spawn nhưng **không** await) vẫn tiêu
/// **cùng** ngân sách. Nếu engine reset ngân sách sau `main`, `cost-1` sẽ không trap.
/// Chẩn đoán có phải **ngân sách** không: lỗi trong thân task được bọc `VS0506` nhưng phải giữ
/// mã gốc `VS0501-BUDGET` trong thông điệp (ST-278) — không được biến thành `VS0501-FUEL`.
fn la_budget(diags: &[vis_syntax::Diagnostic]) -> bool {
    diags
        .iter()
        .any(|d| d.code.short == "VS0501-BUDGET" || d.message.contains("VS0501-BUDGET"))
}

#[test]
fn final_join_dung_chung_ngan_sach_khong_reset() {
    tren_stack_lon(|| {
        let (hir, syms, tys) = tien_trinh(NHIEU_TASK);
        let cost = proposed_cost(NHIEU_TASK);
        assert!(cost > 1);

        ifee::enable_measure_for_test(cost - 1);
        let bad_i = vis_interp::run(NHIEU_TASK, &hir, &syms, &tys, &[]);
        ifee::disable_for_test();
        assert_ne!(bad_i.exit, 0, "interp: phải lỗi khi hết ngân sách");
        assert!(
            la_budget(&bad_i.diagnostics),
            "interp final join: {:?}",
            bad_i.diagnostics
        );

        let (m, fee) = vis_bytecode::compile_with_fee(&hir, &syms, NHIEU_TASK).unwrap();
        let mut vm = Vm::new(&m, NHIEU_TASK);
        vm.budget_enable_for_test(fee, cost - 1);
        let bad_v = vm.run(&[]);
        assert_ne!(bad_v.exit, 0, "VM: phải lỗi khi hết ngân sách");
        assert!(
            la_budget(&bad_v.diagnostics),
            "VM final join: {:?}",
            bad_v.diagnostics
        );
    });
}

/// **Đo** fuel/lệnh thực tế cho hợp đồng VS3-1a. Chạy riêng:
/// `cargo test -p vis-cli --test vs3_budget -- --ignored --nocapture`.
#[test]
#[ignore]
fn do_buoc_fuel_tren_workload_dai_dien() {
    tren_stack_lon(|| {
        for (tag, src) in [
            ("loop-2m", LOOP_2M),
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
            ("tasks-1000", NHIEU_TASK),
            ("task-await-100", TASK_AWAIT),
        ] {
            let cost = proposed_cost(src);
            let (hir, syms, _tys) = tien_trinh(src);
            let m = vis_bytecode::compile(&hir, &syms, src).unwrap();
            let mut vm = Vm::new(&m, src);
            let vo = vm.run(&[]);
            eprintln!(
                "VS3-1a {tag}: cost={cost} vm_steps={} ty_le={:.3} fuel={} exit={}",
                vo.steps,
                vo.steps as f64 / cost as f64,
                vis_vm::vm::SAFETY_FUEL,
                vo.exit
            );
        }
    });
}
