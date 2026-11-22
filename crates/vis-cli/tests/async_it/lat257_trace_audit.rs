// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-283 / VD-257 §4B** — giới hạn của bằng chứng "dãy sự kiện có thứ tự".
//!
//! B (`lat256_budget.rs::ordered_event_trace_interp_vs_compiler`) chứng minh parity thứ tự
//! **chỉ cho chương trình một hàm, không nhánh**. `fee.trace()` của compiler là bảng **tĩnh**
//! gồm mọi hàm và mọi nhánh (kể cả nhánh không chạy) — không phải trace thực thi. Test này
//! dùng chương trình có `match`/`with`/`?` để ghi nhận ranh giới đó thay vì giả vờ parity đầy đủ.
//! Không mở tính năng, không hạ tiêu chí của test cũ.

use vis_interp::frame::{run_frame_for_test as run_i, FrameConfig};
use vis_interp::rt::fee as ifee;
use vis_syntax::{lex, parse, Mode};
use vis_vm::vm::Vm;
use vis_vm::vm_frame::{frame_budget_enable_for_test, run_frame_for_test as run_v};

fn tien_trinh(src: &str) -> (vis_types::Hir, vis_types::Symbols, vis_types::TypeTable) {
    let p = parse(src, Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse: {:?}", p.diagnostics);
    let l = lex(src, Mode::Script);
    let h = vis_types::lower_with(&p.cst, &l, vis_types::LowerOptions::default());
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    (h.hir, h.symbols, h.types)
}

const SRC: &str = r#"type P:
    x: int
    y: int
fn pick(v: int?) -> int?:
    v?
let p: P = {x: 1, y: 2}
let q = p with x: 9
let v: int? = Some(5)
print(q.x)
print(pick(v))
match v:
    Some(n): print(n)
    None: print(0)
"#;

#[test]
fn su_kien_thuc_thi_la_tap_con_cua_bang_tinh() {
    let (hir, syms, tys) = tien_trinh(SRC);
    let (m, fee) = vis_bytecode::compile_with_fee(&hir, &syms, SRC).expect("compile_with_fee");

    // Bảng tĩnh (mọi hàm + mọi nhánh) — KHÔNG phải trace thực thi.
    let static_sites: Vec<(String, u32)> = fee
        .trace()
        .iter()
        .map(|e| (e.kind.name().to_string(), e.site))
        .collect();

    // Đường mặc định: interp + VM cùng stdout, không lỗi.
    ifee::enable_for_test(10_000_000);
    let out = vis_interp::run(SRC, &hir, &syms, &tys, &[]);
    let tr_i = ifee::take_trace_for_test();
    ifee::disable_for_test();
    assert!(out.codes().is_empty(), "interp: {:?}", out.diagnostics);
    let mut vm = Vm::new(&m, SRC);
    vm.budget_enable_for_test(fee.clone(), 10_000_000);
    let vo = vm.run(&[]);
    assert!(vo.diagnostics.is_empty(), "VM: {:?}", vo.diagnostics);
    assert_eq!(out.stdout, vo.stdout);

    // Mọi sự kiện thực thi phải có mặt trong bảng tĩnh (không có sự kiện "ma").
    for e in &tr_i {
        let key = (e.kind.name().to_string(), e.site);
        assert!(
            static_sites.contains(&key),
            "sự kiện ngoài bảng tĩnh: {key:?}"
        );
    }

    // Đường frame: hai engine cùng stdout/exit; hook riêng, không fallback đồng bộ.
    ifee::enable_for_test(10_000_000);
    ifee::frame_emit_enable_for_test();
    let fi = run_i(SRC, &hir, &syms, &tys, &[], FrameConfig::default());
    ifee::disable_for_test();
    assert_eq!(fi.exit, 0, "frame interp: {:?}", fi.diagnostics);

    frame_budget_enable_for_test(fee, 10_000_000, None);
    let fv = run_v(&m, SRC, &[], FrameConfig::default());
    ifee::disable_for_test();
    assert_eq!(fv.exit, 0, "frame VM: {:?}", fv.diagnostics);
    assert_eq!(fi.stdout, fv.stdout);
    assert_eq!(fi.stdout, out.stdout);
}

/// Ranh giới đã biết: parity **theo site và thứ tự** chỉ được test ở chương trình một hàm,
/// không nhánh (`lat256_budget.rs`). Với `match`/`with`/`?`, bảng tĩnh chứa nhánh không chạy
/// nên so sánh trực tiếp với trace thực thi là sai; không được gọi là "parity đầy đủ".
#[test]
fn ghi_nhan_gioi_han_parity() {
    let (hir, syms, _) = tien_trinh(SRC);
    let (_, fee) = vis_bytecode::compile_with_fee(&hir, &syms, SRC).unwrap();
    assert!(
        fee.trace().len() > 20,
        "bảng tĩnh phải chứa nhiều nhánh/hàm"
    );
}
