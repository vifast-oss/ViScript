// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-257 / stream B** — bằng chứng NGÂN SÁCH / FUEL bổ sung cho VD-256.
//!
//! Đường **MUST-NOT-BE-DEFAULT**: mọi charge đi qua test-hook `enable_for_test` /
//! `budget_enable_for_test`. Không thêm CLI flag, không đổi API công, không bật budget mặc định.
//! Năm quy tắc chốt ở `vd252-charging-final.md` giữ nguyên; bảng trọng số v1 vẫn **thử nghiệm**.

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
            std_mods: vec!["net".into()],
            ..Default::default()
        },
    );
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    (h.hir, h.symbols, h.types)
}

macro_rules! labels {
    ($t:expr) => {
        $t.iter()
            .map(|e| format!("{}@{}", e.kind.name(), e.site))
            .collect::<Vec<String>>()
    };
}

/// Chương trình dùng **đầy đủ các sự kiện đã instrumented** trong miền hỗ trợ:
/// Stmt/Expr/Assign/Loop/Call/Builtin (Await/Spawn nằm ở test riêng bên dưới).
const COMPOSITE: &str = r#"fn add(a: int, b: int) -> int:
    a + b
fn main():
    var i = 0
    while i < 3:
        let x = add(i, 1)
        print(x)
        i = i + 1
"#;

/// Chương trình **không vòng lặp/gọi hàm** — static fee table của compiler và dynamic runtime
/// trace trùng nhau từng sự kiện ⇒ có thể assert **dãy có thứ tự** giữa cả ba (compiler/interp/VM).
const FLAT: &str = r#"let a = 1 + 2
var b = a * 3
print(b)
let c = b - 1
print(c)
"#;

#[test]
fn ordered_event_trace_day_du_compiler_interp_vm() {
    let src = FLAT;
    let (hir, syms, tys) = tien_trinh(src);
    let (m, fee) = vis_bytecode::compile_with_fee(&hir, &syms, src).expect("compile_with_fee");

    ifee::enable_for_test(1_000_000);
    let out = vis_interp::run(src, &hir, &syms, &tys, &[]);
    let got = ifee::take_trace_for_test();
    ifee::disable_for_test();
    assert!(out.codes().is_empty(), "interp: {:?}", out.diagnostics);

    assert_eq!(
        labels!(&fee.trace()),
        labels!(&got),
        "compiler ↔ interp lệch DÃY"
    );
    let mut vm = Vm::new(&m, src);
    vm.budget_enable_for_test(fee, 1_000_000);
    let vo = vm.run(&[]);
    assert!(vo.diagnostics.is_empty(), "VM: {:?}", vo.diagnostics);
    assert_eq!(
        labels!(vm.budget_trace_for_test()),
        labels!(&got),
        "VM ↔ interp lệch DÃY"
    );
}

#[test]
fn ordered_event_trace_trong_mien_ho_tro() {
    let (hir, syms, tys) = tien_trinh(COMPOSITE);
    let (m, fee) =
        vis_bytecode::compile_with_fee(&hir, &syms, COMPOSITE).expect("compile_with_fee");

    ifee::enable_for_test(1_000_000);
    let out = vis_interp::run(COMPOSITE, &hir, &syms, &tys, &[]);
    let got = ifee::take_trace_for_test();
    ifee::disable_for_test();
    assert!(out.codes().is_empty(), "interp: {:?}", out.diagnostics);

    // Bảng phí compiler là **tĩnh** (mỗi hàm một lần, theo PC). Trace runtime là **động** (lặp lại
    // theo vòng/gọi hàm). Với vòng lặp + gọi hàm, không assert static==dynamic (vô nghĩa); thay vào
    // đó: (1) hai engine phát **cùng dãy động**, (2) mọi sự kiện động đều có trong bảng tĩnh.
    let mut vm = Vm::new(&m, COMPOSITE);
    vm.budget_enable_for_test(fee.clone(), 1_000_000);
    let vo = vm.run(&[]);
    assert!(vo.diagnostics.is_empty(), "VM: {:?}", vo.diagnostics);
    assert_eq!(
        labels!(vm.budget_trace_for_test()),
        labels!(&got),
        "VM ↔ interp lệch DÃY sự kiện động"
    );
    let static_labels = labels!(&fee.trace());
    for l in labels!(&got) {
        assert!(
            static_labels.contains(&l),
            "sự kiện động {l} không có trong bảng compiler: {:?}",
            static_labels
        );
    }
    for k in [
        "E_STMT",
        "E_EXPR",
        "E_ASSIGN",
        "E_LOOP",
        "E_CALL",
        "E_BUILTIN",
    ] {
        assert!(
            labels!(&got).iter().any(|l| l.starts_with(k)),
            "thiếu {k} trong dãy: {:?}",
            labels!(&got)
        );
    }
}

const ASYNC_SRC: &str = r#"async fn t() -> int:
    1
fn main():
    let x = spawn t()
    let y = await x
    print(y)
"#;

#[test]
fn ordered_event_trace_spawn_await() {
    let (hir, syms, tys) = tien_trinh(ASYNC_SRC);
    let (m, fee) = vis_bytecode::compile_with_fee(&hir, &syms, ASYNC_SRC).unwrap();
    ifee::enable_for_test(1_000_000);
    let out = vis_interp::run(ASYNC_SRC, &hir, &syms, &tys, &[]);
    let got = ifee::take_trace_for_test();
    ifee::disable_for_test();
    assert!(out.codes().is_empty(), "interp: {:?}", out.diagnostics);
    // Thân task chạy ở final join ⇒ static fee table (theo hàm) KHÔNG trùng thứ tự dynamic; so
    // hai engine thật + kiểm coverage. VD-257: HBody::Expr đã charge E_STMT ⇒ đủ bộ sự kiện.
    let mut vm = Vm::new(&m, ASYNC_SRC);
    vm.budget_enable_for_test(fee.clone(), 1_000_000);
    let vo = vm.run(&[]);
    assert_eq!(vo.exit, 0, "VM: {:?}", vo.diagnostics);
    assert_eq!(labels!(vm.budget_trace_for_test()), labels!(&got));
    let static_labels = labels!(&fee.trace());
    for l in labels!(&got) {
        assert!(
            static_labels.contains(&l),
            "sự kiện {l} không có trong bảng compiler"
        );
    }
    for k in ["E_SPAWN", "E_AWAIT", "E_STMT", "E_BUILTIN"] {
        assert!(labels!(&got).iter().any(|l| l.starts_with(k)), "thiếu {k}");
    }
}

/// Tính **chi phí đúng** của một chương trình bằng bảng trọng số v1 đề xuất. Dùng hook đo
/// (không lưu trace), rồi cộng `n × w`. Không phải chính sách mặc định.
fn proposed_cost(src: &str) -> u64 {
    let (hir, syms, _tys) = tien_trinh(src);
    ifee::enable_measure_for_test(u64::MAX / 4);
    let _ = vis_interp::run(src, &hir, &syms, &_tys, &[]);
    let c = ifee::take_counts_for_test();
    ifee::disable_for_test();
    c.iter()
        .zip([1u64, 1, 1, 1, 4, 2, 4, 2])
        .map(|(n, w)| n * w)
        .sum()
}

#[test]
fn ngan_sach_nho_ta_ranh_gioi_tac_dung_phu() {
    let src = "print(42)\n";
    let cost = proposed_cost(src);
    assert!(cost >= 2, "print(42) phải có ít nhất Stmt+Expr+Builtin");
    let (hir, syms, tys) = tien_trinh(src);

    // (a) budget = cost-1 ⇒ trap TRƯỚC khi print in ra ⇒ stdout rỗng + VS0501-BUDGET.
    ifee::enable_for_test(cost - 1);
    let low = vis_interp::run(src, &hir, &syms, &tys, &[]);
    ifee::disable_for_test();
    assert_eq!(low.stdout, "", "ranh giới: chưa tới ngân sách thì không in");
    assert!(
        low.codes().contains(&"VS0501-BUDGET"),
        "{:?}",
        low.diagnostics
    );

    // (b) VM cùng ngưỡng ⇒ cũng không in, cùng short code (khác engine).
    let (m, fee) = vis_bytecode::compile_with_fee(&hir, &syms, src).unwrap();
    let mut vm = Vm::new(&m, src);
    vm.budget_enable_for_test(fee, cost - 1);
    let vo = vm.run(&[]);
    assert_eq!(vo.stdout, "", "VM ranh giới thấp không in");
    assert!(
        vo.diagnostics
            .iter()
            .any(|d| d.code.short == "VS0501-BUDGET"),
        "{:?}",
        vo.diagnostics
    );

    // (c) budget = cost ⇒ vừa đủ ⇒ print in "42", sạch.
    ifee::enable_for_test(cost);
    let ok = vis_interp::run(src, &hir, &syms, &tys, &[]);
    ifee::disable_for_test();
    assert_eq!(ok.stdout, "42\n");
    assert!(ok.codes().is_empty(), "{:?}", ok.diagnostics);
}

#[test]
fn fuel_chay_truoc_khi_budget_khi_budget_lon_hon_fuel() {
    // VD-257 §: fuel 10^7 LỆNH, budget đề xuất 20M. Vòng 1.2M tốn ~10.8M lệnh nhưng chi phí
    // ngữ nghĩa ~10.8M < 20M ⇒ **fuel** phải là thứ kích hoạt, KHÔNG phải budget. Đây là bằng
    // chứng "fuel 10^7 chặn chương trình compiler sinh trước khi budget cạn" — root cause không
    // phải ở tên `MAX_STEPS`/`SAFETY_FUEL`.
    let src = "var i = 0\nwhile i < 1200000:\n    i = i + 1\n";
    let (hir, syms, _tys) = tien_trinh(src);
    let m = vis_bytecode::compile(&hir, &syms, src).unwrap();
    let fee = vis_bytecode::compile_with_fee(&hir, &syms, src).unwrap().1;
    let mut vm = Vm::new(&m, src);
    vm.budget_enable_for_test(fee, 20_000_000);
    let vo = vm.run(&[]);
    assert_eq!(vo.exit, 1);
    assert_eq!(
        vo.codes(),
        vec!["VS0501-FUEL"],
        "fuel phải kích hoạt trước budget, got {:?}",
        vo.diagnostics
    );
    assert_eq!(vo.steps, vis_vm::vm::SAFETY_FUEL + 1);
}
