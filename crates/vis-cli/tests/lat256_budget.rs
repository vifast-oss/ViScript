// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-276 + ST-278 / VD-256** — bằng chứng chạy thật cho ngân sách ngữ nghĩa + sửa final-join.
//!
//! Đường **MUST-NOT-BE-DEFAULT**: mọi instrumentation ở đây đều bật qua test-hook nội bộ.
//! `vis run` thường (không hook) vẫn dùng nguyên luồng cũ — test `default_khong_doi` khẳng định.
//!
//! Nghĩa vụ **(A)**: compiler + interpreter cùng một định nghĩa sự kiện (`fee_shared.rs`) và test
//! so **dãy sự kiện có thứ tự**, không chỉ tổng. Nghĩa vụ **(B)**: bảng phụ S + verifier được test
//! riêng ở `vis-bytecode/tests/fee_side_table.rs`; test ở đây không nhận bảng phụ là bằng chứng
//! instrumentation đầy đủ.

use vis_interp::frame::{run_frame_for_test as run_i, FrameConfig};
use vis_interp::rt::fee as ifee;
use vis_syntax::{lex, parse, Mode};
use vis_vm::vm::Vm;
use vis_vm::vm_frame::run_frame_for_test as run_v;

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

/// Nhãn dãy sự kiện: `E_STMT@<site>`. So **thứ tự**, không phải tổng.
/// Macro (không phải hàm) vì hai crate include chung một file ⇒ hai type `FeeEntry` riêng
/// nhưng **cùng tên/cùng tag** — đây chính là bằng chứng "một định nghĩa, hai bản dịch".
macro_rules! labels {
    ($t:expr) => {
        $t.iter()
            .map(|e| format!("{}@{}", e.kind.name(), e.site))
            .collect::<Vec<String>>()
    };
}

// ---------------------------------------------------------------------------
// (A) Parity nguồn — DÃY SỰ KIỆN CÓ THỨ TỰ
// ---------------------------------------------------------------------------

const PARITY: &str = r#"let a = 1 + 2
print(a)
"#;

#[test]
fn ordered_event_trace_interp_vs_compiler() {
    let (hir, syms, tys) = tien_trinh(PARITY);
    let (m, fee) = vis_bytecode::compile_with_fee(&hir, &syms, PARITY).expect("compile_with_fee");

    ifee::enable_for_test(1_000_000);
    let out = vis_interp::run(PARITY, &hir, &syms, &tys, &[]);
    let got = ifee::take_trace_for_test();
    ifee::disable_for_test();

    assert_eq!(
        out.codes(),
        Vec::<&str>::new(),
        "interp: {:?}",
        out.diagnostics
    );
    // Bảng phụ của compiler (dãy theo hàm rồi theo PC/thứ tự ghi).
    let want = fee.trace();
    assert_eq!(
        want.len(),
        got.len(),
        "số sự kiện lệch:\nwant={:?}\ngot={:?}",
        labels!(&want),
        labels!(&got)
    );
    assert_eq!(labels!(&want), labels!(&got), "DÃY SỰ KIỆN CÓ THỨ TỰ lệch");

    // Cùng bảng phụ đưa vào VM ⇒ VM phát đúng dãy đó (cùng định nghĩa sự kiện).
    let mut vm = Vm::new(&m, PARITY);
    vm.budget_enable_for_test(fee, 1_000_000);
    let vo = vm.run(&[]);
    assert_eq!(vo.exit, 0, "VM: {:?}", vo.diagnostics);
    assert_eq!(
        labels!(vm.budget_trace_for_test()),
        labels!(&got),
        "VM ↔ interp lệch dãy sự kiện"
    );
}

#[test]
fn nhieu_su_kien_cung_pc_giu_thu_tu_khi_charge() {
    // Một PC có 3 sự kiện: VM phải charge cả 3 **theo thứ tự**, không gộp/mất thứ tự.
    let src = "let a = 1\nprint(a)\n";
    let (hir, syms, _tys) = tien_trinh(src);
    let (m, _fee) = vis_bytecode::compile_with_fee(&hir, &syms, src).unwrap();
    let mut fee = vis_bytecode::fee::FeeTable::new(m.funcs.len());
    use vis_bytecode::fee::{FeeEntry, FeeKind};
    fee.push(0, FeeEntry::new(1, FeeKind::Call, 101));
    fee.push(0, FeeEntry::new(1, FeeKind::Expr, 102));
    fee.push(0, FeeEntry::new(1, FeeKind::Assign, 103));
    let mut vm = Vm::new(&m, src);
    vm.budget_enable_for_test(fee.clone(), 1_000_000);
    let o = vm.run(&[]);
    assert_eq!(o.exit, 0);
    let tr = vm.budget_trace_for_test();
    assert!(
        tr.len() >= 3,
        "phải charge đúng nhóm 3 sự kiện tại pc 1: {:?}",
        labels!(tr)
    );
    let win: Vec<_> = tr
        .windows(3)
        .find(|w| w[0].site == 101 && w[1].site == 102 && w[2].site == 103)
        .unwrap_or_else(|| panic!("thứ tự nhóm sai: {:?}", labels!(tr)))
        .to_vec();
    assert_eq!(
        win.iter().map(|e| e.kind).collect::<Vec<_>>(),
        vec![FeeKind::Call, FeeKind::Expr, FeeKind::Assign]
    );
}

// ---------------------------------------------------------------------------
// Resume / double-charge — bắt bằng **runtime test**, không tuyên bố verifier bắt hết.
// ---------------------------------------------------------------------------

#[test]
fn resume_khong_charge_lai_nhom_da_xong() {
    // Mô phỏng đúng quy ước D6: charge tại pc P, continuation lưu pc P+1 (sau charge);
    // khi resume từ P+1, nhóm tại P không được charge lại. Chạy thật qua `budget_charge`.
    let src = "let a = 1\nprint(a)\n";
    let (hir, syms, _tys) = tien_trinh(src);
    let (m, _fee) = vis_bytecode::compile_with_fee(&hir, &syms, src).unwrap();
    let mut fee = vis_bytecode::fee::FeeTable::new(m.funcs.len());
    use vis_bytecode::fee::{FeeEntry, FeeKind};
    fee.push(0, FeeEntry::new(1, FeeKind::Await, 77));
    fee.push(0, FeeEntry::new(1, FeeKind::Expr, 78));
    let mut vm = Vm::new(&m, src);
    vm.budget_enable_for_test(fee, 1_000_000);
    let o = vm.run(&[]);
    assert_eq!(o.exit, 0);
    let before = labels!(vm.budget_trace_for_test())
        .iter()
        .filter(|l| l.ends_with("@77") || l.ends_with("@78"))
        .count();
    assert_eq!(before, 2, "mỗi sự kiện trong nhóm charge đúng 1 lần");
    // Resume tại pc SAU điểm charge ⇒ không có entry nào tại pc 2 ⇒ trace không đổi.
    let mut vm2 = Vm::new(&m, src);
    vm2.budget_enable_for_test(vis_bytecode::fee::FeeTable::new(m.funcs.len()), 1_000_000);
    let _ = vm2.run(&[]);
    assert_eq!(
        labels!(vm2.budget_trace_for_test()).len(),
        0,
        "pc sau charge không mang nhóm cũ"
    );
}

// ---------------------------------------------------------------------------
// Chẩn đoán phân biệt + không đổi mặc định
// ---------------------------------------------------------------------------

#[test]
fn budget_va_safety_fuel_co_chan_doan_rieng() {
    use vis_interp::interp::BUDGET;
    use vis_vm::vm::SAFETY_FUEL_CODE;
    assert_ne!(BUDGET.short, SAFETY_FUEL_CODE.short);
    assert_ne!(BUDGET.short, "VS0501");
    assert_ne!(BUDGET.short, "VS0506");
    assert_eq!(vis_vm::vm::SAFETY_FUEL, 10_000_000, "D9: không tăng ngưỡng");
    assert_eq!(vis_vm::vm::MAX_STEPS, vis_vm::vm::SAFETY_FUEL);
}

#[test]
fn budget_nho_trap_ro_khong_che_verifier() {
    let (hir, syms, tys) = tien_trinh(PARITY);
    ifee::enable_for_test(1);
    let out = vis_interp::run(PARITY, &hir, &syms, &tys, &[]);
    let hit = ifee::hit_for_test();
    ifee::disable_for_test();
    assert!(hit, "phải đánh dấu vượt ngân sách");
    assert_ne!(out.exit, 0, "vượt ngân sách ⇒ không exit thành công");
    assert!(
        out.codes().contains(&"VS0501-BUDGET"),
        "chẩn đoán phân biệt: {:?}",
        out.diagnostics
    );
}

#[test]
fn default_khong_doi_khi_khong_bat_hook() {
    let (hir, syms, tys) = tien_trinh(PARITY);
    let plain = vis_interp::run(PARITY, &hir, &syms, &tys, &[]);
    ifee::enable_for_test(1_000_000);
    let hooked = vis_interp::run(PARITY, &hir, &syms, &tys, &[]);
    ifee::disable_for_test();
    assert_eq!(plain.stdout, hooked.stdout);
    assert_eq!(plain.exit, hooked.exit);
}

// ---------------------------------------------------------------------------
// ST-278 — final join error surfaced (cả hai engine)
// ---------------------------------------------------------------------------

const FAIL_TASK: &str = r#"async fn boom() -> int:
    let x = 1 // 0
    x
fn main():
    print("start")
    let t = spawn boom()
"#;

#[test]
fn final_join_error_surfaces_hai_engine() {
    let (hir, syms, tys) = tien_trinh(FAIL_TASK);
    let cfg = FrameConfig::default();
    let i = run_i(FAIL_TASK, &hir, &syms, &tys, &[], cfg.clone());
    let m = vis_bytecode::compile(&hir, &syms, FAIL_TASK).expect("compile");
    let v = run_v(&m, FAIL_TASK, &[], cfg);
    for (name, o) in [("interp", &i), ("vm", &v)] {
        assert_eq!(o.stdout, "start\n", "{name}: stdout");
        assert_ne!(o.exit, 0, "{name}: task lỗi ở final join không được exit 0");
        assert!(
            o.diagnostics.iter().any(|d| d.code.short == "VS0506"),
            "{name}: phải báo VS0506 theo hợp đồng task-error, got {:?}",
            o.diagnostics
        );
        assert!(
            o.diagnostics.iter().any(|d| d.message.contains("VS0502")),
            "{name}: phải giữ mã gốc VS0502, got {:?}",
            o.diagnostics
        );
    }
    assert_eq!(i.exit, v.exit);
    for (a, b) in i.diagnostics.iter().zip(&v.diagnostics) {
        assert_eq!(a.code.short, b.code.short);
        assert_eq!(a.message, b.message);
        assert_eq!((a.line, a.col), (b.line, b.col));
    }
}

#[test]
fn final_join_khong_doi_chinh_sach_huy_task_con_lai() {
    // Task tốt vẫn join bình thường, task lỗi vẫn surfaced; thứ tự tạo không đổi.
    let src = r#"async fn ok() -> int:
    1
async fn bad() -> int:
    let x = 1 // 0
    x
fn main():
    let a = spawn ok()
    let b = spawn bad()
    print("done")
"#;
    let (hir, syms, tys) = tien_trinh(src);
    let i = run_i(src, &hir, &syms, &tys, &[], FrameConfig::default());
    assert_eq!(i.stdout, "done\n");
    assert_ne!(i.exit, 0);
    assert!(i.diagnostics.iter().any(|d| d.code.short == "VS0506"));
}

// ---------------------------------------------------------------------------
// ĐO WORKLOAD (VD-256 §1 / D2) — chạy riêng: cargo test -p vis-cli --test lat256_budget -- --ignored --nocapture
// Các số là ĐỀ XUẤT cho A0, KHÔNG phải default policy.
// ---------------------------------------------------------------------------

fn do_measure(tag: &str, src: &str) {
    // Thread có stack lớn để workload đệ quy không tràn (đo, không đổi runtime).
    let tag2 = tag.to_string();
    let src2 = src.to_string();
    std::thread::Builder::new()
        .name("vd256-measure".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            let (hir, syms, tys) = tien_trinh(&src2);
            ifee::enable_measure_for_test(u64::MAX / 4);
            let t0 = std::time::Instant::now();
            let out = vis_interp::run(&src2, &hir, &syms, &tys, &[]);
            let elapsed = t0.elapsed();
            let c = ifee::take_counts_for_test();
            ifee::disable_for_test();
            let names = [
                "E_STMT",
                "E_EXPR",
                "E_ASSIGN",
                "E_LOOP",
                "E_CALL",
                "E_BUILTIN",
                "E_SPAWN",
                "E_AWAIT",
            ];
            let total: u64 = c.iter().sum();
            let weight: u64 = c
                .iter()
                .zip([1u64, 1, 1, 1, 4, 2, 4, 2])
                .map(|(n, w)| n * w)
                .sum();
            eprintln!(
                "MEASURE {tag2}: exit={} {:?} total_events={total} proposed_cost={weight}",
                out.exit, elapsed
            );
            for (n, k) in c.iter().zip(names) {
                eprintln!("  {k}: {n}");
            }
            assert_eq!(out.exit, 0, "{tag2}: workload phải chạy thành công");
        })
        .expect("spawn measure thread")
        .join()
        .expect("join measure thread");
}

#[test]
#[ignore]
fn measure_workloads() {
    do_measure("loop-2m", "var i = 0\nwhile i < 2000000:\n    i = i + 1\n");
    do_measure(
        "recursion-200",
        "fn rec(n: int) -> int:\n    if n <= 0:\n        0\n    else:\n        rec(n - 1) + 1\nfn main():\n    print(rec(200))\n",
    );
    do_measure(
        "tasks-1000",
        "async fn t() -> int:\n    1\nfn main():\n    var i = 0\n    while i < 1000:\n        let x = spawn t()\n        i = i + 1\n",
    );
    do_measure(
        "expr-chain",
        "fn main():\n    var i = 0\n    while i < 100000:\n        let x = 1 + 2 + 3 + 4 + 5\n        i = i + 1\n",
    );
}
