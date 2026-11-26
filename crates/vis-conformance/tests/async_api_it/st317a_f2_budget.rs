// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-317a F2** — parity ngân sách interp ↔ VM cho fast-path `r = r with f: v`.
//!
//! Audit F1 (`cd7325b6`, P1): fast-path `gan_with_tai_cho` bỏ bookkeeping của nút `HExpr::With`
//! (`steps`, `worker_tick`, `E_EXPR`) nên mỗi vòng interp rẻ hơn VM 1 đơn vị; tại
//! `--max-budget 11500` interp exit 0 còn VM `VS0510 (E_ASSIGN)`. F2 dùng chung `vao_expr` với
//! `Interp::eval`, nên **ngưỡng ngân sách tối thiểu** hai engine phải bằng nhau từng đơn vị.
//!
//! Test tìm ngưỡng nhỏ nhất chạy được bằng tìm kiếm nhị phân (ngân sách đơn điệu), cho:
//! - `FAST`: `r = r with f: i` — đi qua fast-path;
//! - `CONTROL`: `q = r with f: i` — không fast-path (đích ≠ gốc), đường `HExpr::With` thường.

use vis_runtime::rt::fee as ifee;
use vis_syntax::{lex, parse, Mode};
use vis_vm::vm::Vm;

const FAST: &str = "type P:\n    f: int\n\nfn main():\n    var r = P(f: 0)\n    var i = 0\n    while i < 1000:\n        r = r with f: i\n        i += 1\n    print(r.f)\n";

const CONTROL: &str = "type P:\n    f: int\n\nfn main():\n    var r = P(f: 0)\n    var q = P(f: 0)\n    var i = 0\n    while i < 1000:\n        q = r with f: i\n        i += 1\n    print(q.f)\n";

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

fn chay_interp(
    src: &str,
    hir: &vis_hir::Hir,
    syms: &vis_hir::Symbols,
    tys: &vis_hir::TypeTable,
    budget: u64,
) -> vis_core::RunOutput {
    ifee::enable_measure_for_test(budget);
    let r = vis_interp::run(src, hir, syms, tys, &[]);
    ifee::disable_for_test();
    r
}

fn chay_vm(
    m: &vis_bytecode::Module,
    fee: &vis_bytecode::fee::FeeTable,
    src: &str,
    budget: u64,
) -> vis_core::RunOutput {
    let mut vm = Vm::new(m, src);
    vm.budget_enable_measure_for_test(fee.clone(), budget);
    vm.run(&[])
}

/// Ngân sách **nhỏ nhất** để chương trình chạy sạch (ngân sách đơn điệu theo hạn mức).
fn nguong_interp(
    src: &str,
    hir: &vis_hir::Hir,
    syms: &vis_hir::Symbols,
    tys: &vis_hir::TypeTable,
) -> u64 {
    let mut lo = 0u64;
    let mut hi = 1u64 << 20;
    let ok = chay_interp(src, hir, syms, tys, hi);
    assert_eq!(
        ok.exit, 0,
        "ngân sách trần phải chạy được: {:?}",
        ok.diagnostics
    );
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if chay_interp(src, hir, syms, tys, mid).exit == 0 {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    lo
}

fn nguong_vm(src: &str, m: &vis_bytecode::Module, fee: &vis_bytecode::fee::FeeTable) -> u64 {
    let mut lo = 0u64;
    let mut hi = 1u64 << 20;
    let ok = chay_vm(m, fee, src, hi);
    assert_eq!(
        ok.exit, 0,
        "ngân sách trần phải chạy được: {:?}",
        ok.diagnostics
    );
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if chay_vm(m, fee, src, mid).exit == 0 {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    lo
}

fn kiem_parity(ten: &str, src: &str) -> u64 {
    let (hir, syms, tys) = tien_trinh(src);
    let (m, fee) = vis_bytecode::compile_with_fee(&hir, &syms, src).expect("compile_with_fee");
    let i = nguong_interp(src, &hir, &syms, &tys);
    let v = nguong_vm(src, &m, &fee);
    assert_eq!(
        i, v,
        "{ten}: ngưỡng ngân sách tối thiểu lệch interp={i} vm={v}"
    );
    assert!(i > 1, "{ten}: ngưỡng phải > 1 (đo được), được {i}");

    // Biên: `ngưỡng - 1` phải **cả hai** cùng báo `VS0510` (không phải lỗi khác).
    let duoi_i = chay_interp(src, &hir, &syms, &tys, i - 1);
    let duoi_v = chay_vm(&m, &fee, src, i - 1);
    assert_ne!(duoi_i.exit, 0, "{ten}: interp phải vượt ngân sách");
    assert_ne!(duoi_v.exit, 0, "{ten}: VM phải vượt ngân sách");
    assert_eq!(
        duoi_i.codes(),
        vec!["VS0510"],
        "{ten}: interp {:?}",
        duoi_i.diagnostics
    );
    assert_eq!(
        duoi_v.codes(),
        vec!["VS0510"],
        "{ten}: VM {:?}",
        duoi_v.diagnostics
    );
    i
}

#[test]
fn fast_path_with_ngan_sach_parity_hai_engine() {
    let fast = kiem_parity("fast `r = r with`", FAST);
    let control = kiem_parity("control `q = r with`", CONTROL);
    // Đường fast-path không được rẻ hơn đường thường cùng hình dạng: `FAST` chỉ thiếu một
    // `var q = P(f: 0)` so với `CONTROL`, nên ngưỡng fast phải **nhỏ hơn hoặc bằng** control.
    assert!(
        fast <= control,
        "fast-path rẻ bất thường: fast={fast} control={control}"
    );
}
