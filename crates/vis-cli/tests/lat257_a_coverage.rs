// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-257 / stream A — FRAME COVERAGE còn lại**: `?`, kênh (`chan`), `cmd`, gán
//! trường/chỉ số, phân biệt method host/value, closure có `await`, và resume không
//! double-charge. Mọi ca dùng **nguồn thật** → lower → check → HIR/bytecode thật →
//! `Interp`/`Vm` frame path (cả hai engine khi engine đó hỗ trợ). Không gọi helper dựng
//! HIR tay, không fallback đồng bộ, không đổi checker / runtime mặc định / golden.
//!
//! Ô không phủ được thì **từ chối rõ** (diagnostic có `Span`) và được ghi là "explicitly
//! rejected" ở `agent/JOURNAL/vd257-a-coverage-report.md` — KHÔNG tuyên bố coverage complete.

use std::collections::HashSet;

use vis_interp::frame::{run_frame_for_test as run_i, FrameConfig, FrameOutput};
use vis_interp::rt::{fee as ifee, DiagSnapshot, FakeEvent};
use vis_interp::Value;
use vis_syntax::{lex, parse, Mode};
use vis_vm::vm_frame::{
    run_frame_for_test as run_v, run_frame_for_test_with_budget as run_v_budget,
};

/// Nguồn thật → lower → check. `check().is_clean()` ở đây là bằng chứng checker KHÔNG thu hẹp.
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

fn cfg(sources: &[(&str, u64)], events: Vec<FakeEvent>) -> FrameConfig {
    FrameConfig {
        fake_sources: sources.iter().map(|(a, b)| (a.to_string(), *b)).collect(),
        events,
        ..Default::default()
    }
}

fn ev(seq: u64, key: u64, gen: u64, v: i64) -> FakeEvent {
    FakeEvent {
        seq,
        key,
        gen,
        value: Value::Int(v),
    }
}

/// Chạy cả hai engine, so stdout / exit / chẩn đoán / trace / ownership byte-for-byte.
fn chay(src: &str, c: FrameConfig) -> (FrameOutput, FrameOutput) {
    let (hir, syms, tys) = tien_trinh(src);
    let i = run_i(src, &hir, &syms, &tys, &[], c.clone());
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    let v = run_v(&m, src, &[], c);
    assert_eq!(i.stdout, v.stdout, "interp↔VM stdout lệch");
    assert_eq!(i.exit, v.exit, "exit lệch");
    assert_eq!(
        i.diagnostics.len(),
        v.diagnostics.len(),
        "số chẩn đoán lệch: I={:?} V={:?}",
        i.diagnostics,
        v.diagnostics
    );
    for (a, b) in i.diagnostics.iter().zip(&v.diagnostics) {
        assert_eq!(a.code.short, b.code.short, "mã chẩn đoán lệch");
        assert_eq!(a.message, b.message, "nội dung chẩn đoán lệch");
        assert_eq!((a.line, a.col), (b.line, b.col), "vị trí lệch");
    }
    assert_eq!(i.log, v.log, "trace scheduler lệch");
    assert_eq!(i.snapshot, v.snapshot, "snapshot ownership lệch");
    (i, v)
}

fn baseline() -> DiagSnapshot {
    DiagSnapshot {
        live_tasks: 0,
        frames: 0,
        waiters: 0,
        ready: 0,
        regs: 0,
        timers: 0,
    }
}

// ===========================================================================
// A — `?` (try): unwrap `Option`/`Result` + trả sớm / `xs[i]?` (VD-178 R8)
// ===========================================================================

/// A1 — `?` trên `Option`: `Some` bóc giá trị; `None` trả sớm ⇒ câu `print("SAU")`
/// SAU `?` bị bỏ (short-circuit). Side-effect đúng số lần, cả hai engine đồng nhau.
const A1: &str = r#"fn f(x: int?) -> int?:
    let y = x?
    print("SAU")
    y
fn main():
    print(f(Some(5)))
    print(f(None))
"#;

#[test]
fn a1_try_option_unwrap_and_early_return_short_circuit() {
    let (i, _) = chay(A1, cfg(&[], vec![]));
    assert_eq!(i.exit, 0, "{:?}", i.diagnostics);
    assert_eq!(i.stdout, "SAU\nSome(5)\nNone\n");
    assert_eq!(
        i.stdout.matches("SAU").count(),
        1,
        "`?` phải bỏ vế sau None"
    );
    assert_eq!(i.snapshot, baseline());
    // Runtime mặc định không đổi: cùng stdout.
    let (hir, syms, tys) = tien_trinh(A1);
    let def = vis_interp::run(A1, &hir, &syms, &tys, &[]);
    assert_eq!(def.stdout, i.stdout);
}

/// A2 — `xs[i]?`: chỉ số có kiểm tra ⇒ `T?`, không trap dù ngoài phạm vi / âm.
#[test]
fn a2_try_index_checked_returns_option() {
    let src = r#"fn main():
    let xs = [10, 20]
    print(xs[1]?)
    print(xs[9]?)
    print(xs[-1]?)
"#;
    let (i, _) = chay(src, cfg(&[], vec![]));
    assert_eq!(i.exit, 0, "{:?}", i.diagnostics);
    assert_eq!(i.stdout, "Some(20)\nNone\nNone\n");
    assert_eq!(i.snapshot, baseline());
}

/// A3 — `?` trên `Result`: `Ok` bóc; `Err` trả sớm, bỏ `print("SAU")`.
/// Dùng cả fixture thật `hoi-cham-ok.vis` (nhánh Ok).
const A3: &str = r#"fn f(n: int) -> Result[int, str]:
    if n > 0: Ok(n) else: Err("am")
fn g(n: int) -> Result[int, str]:
    let v = f(n)?
    print("SAU")
    Ok(v * 2)
fn main():
    print(g(3))
    print(g(-1))
"#;

#[test]
fn a3_try_result_early_return_and_fixture_ok() {
    let (i, _) = chay(A3, cfg(&[], vec![]));
    assert_eq!(i.exit, 0, "{:?}", i.diagnostics);
    assert_eq!(i.stdout, "SAU\nOk(6)\nErr(am)\n");
    assert_eq!(
        i.stdout.matches("SAU").count(),
        1,
        "`Err` phải short-circuit"
    );

    // Fixture `?` trên Result thật: nhánh Ok, stdout = golden, cả hai engine.
    let ok_fixture = include_str!("../../../test/sem/result/hoi-cham-ok.vis");
    let golden = include_str!("../../../test/sem/result/hoi-cham-ok.out");
    let (i2, v2) = chay(ok_fixture, cfg(&[], vec![]));
    assert_eq!(i2.exit, 0, "{:?}", i2.diagnostics);
    let want = golden
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .map(|l| format!("{l}\n"))
        .collect::<String>();
    assert_eq!(i2.stdout, want);
    assert_eq!(v2.stdout, want);
}

// ===========================================================================
// B — `chan`: tạo / send / recv (đầy / rỗng / đóng) trên đường frame test
// ===========================================================================

/// B1 — `chan[T](n)` tạo đúng kênh qua `$kenh`; send/recv không chặn; `close`
/// idempotent; `recv` sau close trả nốt hàng rồi `None`. Cả hai engine.
#[test]
fn b1_chan_create_send_recv_close() {
    let src = r#"fn main():
    let c = chan[int](1)
    let _ = c.send(7)
    c.close()
    match c.recv():
        Some(v): print(v)
        None: print("het")
    match c.recv():
        Some(v): print(v)
        None: print("het")
    c.close()
    print("done")
"#;
    let (i, v) = chay(src, cfg(&[], vec![]));
    assert_eq!(i.exit, 0, "{:?}", i.diagnostics);
    assert_eq!(i.stdout, "7\nhet\ndone\n");
    assert_eq!(i.stdout, v.stdout);
    assert_eq!(i.snapshot, baseline());
}

/// B2 — hai fixture chan thật (task gửi qua kênh) chạy QUA frame path, stdout = golden,
/// hai engine byte-identical. Đây là bằng chứng send/recv qua task không fallback.
#[test]
fn b2_chan_async_fixtures_frame_path() {
    for (name, src, out) in [
        (
            "grid-chan-hai-task-kenh",
            include_str!("../../../test/sem/async/grid-chan-hai-task-kenh.vis"),
            include_str!("../../../test/sem/async/grid-chan-hai-task-kenh.out"),
        ),
        (
            "grid-kenh-list",
            include_str!("../../../test/sem/async/grid-kenh-list.vis"),
            include_str!("../../../test/sem/async/grid-kenh-list.out"),
        ),
    ] {
        let (i, v) = chay(src, cfg(&[], vec![]));
        assert_eq!(i.exit, 0, "{name}: {:?}", i.diagnostics);
        let golden: String = out
            .lines()
            .filter(|l| !l.trim_start().starts_with('#'))
            .map(|l| format!("{l}\n"))
            .collect();
        assert_eq!(i.stdout, golden, "{name}: stdout lệch golden");
        assert_eq!(v.stdout, golden, "{name}: VM lệch golden");
        assert_eq!(i.snapshot, baseline(), "{name}");
    }
}

/// B3 — trạng thái ĐẦY / RỖNG / ĐÓNG: frame path KHÔNG mô phỏng chờ kênh (s.ready
/// rỗng trên đường frame), nên từ chối RÕ bằng `VS0505`/`VS0504` — không fallback âm
/// thầm, không chạy câu lệnh sau điểm chặn. Checker vẫn chấp nhận các nguồn này.
#[test]
fn b3_chan_full_empty_closed_explicit_rejection() {
    let cases = [
        (
            "full-send",
            "fn main():\n    let c = chan[int](1)\n    let _ = c.send(1)\n    let _ = c.send(2)\n    print(\"SAU\")\n",
            "deadlock: `send` vào kênh **đầy**",
        ),
        (
            "empty-recv",
            "fn main():\n    let c = chan[int](1)\n    match c.recv():\n        Some(v): print(v)\n        None: print(\"het\")\n    print(\"SAU\")\n",
            "deadlock: `recv` trên kênh **rỗng**",
        ),
        (
            "send-after-close",
            "fn main():\n    let c = chan[int](1)\n    c.close()\n    let _ = c.send(1)\n    print(\"SAU\")\n",
            "`send` vào kênh đã `close`",
        ),
    ];
    for (name, src, want) in cases {
        let (i, v) = chay(src, cfg(&[], vec![]));
        assert_eq!(i.exit, 1, "{name}: phải từ chối");
        assert!(
            i.diagnostics[0].message.contains(want),
            "{name}: diag sai: {:?}",
            i.diagnostics
        );
        assert!(!i.stdout.contains("SAU"), "{name}: chạy tiếp sau điểm chặn");
        assert_eq!(i.stdout, v.stdout, "{name}");
    }
}

// ===========================================================================
// C — `cmd { … }`: VS1, từ chối rõ (không phải lỗi "frame test unsupported")
// ===========================================================================

#[test]
fn c1_cmdblock_rejected_clearly_matches_default_runtime() {
    let src = r#"fn main():
    print("TRUOC")
    let x = cmd { echo hi }
    print("SAU")
"#;
    let (hir, syms, tys) = tien_trinh(src); // checker chấp nhận — không thu hẹp
    let i = run_i(src, &hir, &syms, &tys, &[], cfg(&[], vec![]));
    assert_eq!(i.exit, 1);
    assert_eq!(i.stdout, "TRUOC\n", "không chạy câu sau cmd-block");
    assert!(
        i.diagnostics[0]
            .message
            .contains("khối `cmd` là VS1 — chưa hỗ trợ ở VS0"),
        "{:?}",
        i.diagnostics
    );
    // Runtime mặc định từ chối cùng thông điệp; compiler VM từ chối cùng structure.
    let def = vis_interp::run(src, &hir, &syms, &tys, &[]);
    assert_eq!(def.exit, 1);
    assert_eq!(def.diagnostics[0].message, i.diagnostics[0].message);
    let err = vis_bytecode::compile(&hir, &syms, src).unwrap_err();
    assert!(format!("{err:?}").contains("cmd-block"), "{err:?}");
}

// ===========================================================================
// D — gán trường/chỉ số + phân biệt method host vs value (closure)
// ===========================================================================

#[test]
fn d1_field_and_index_assignment_on_frame_path() {
    let src = r#"type P:
    x: int
fn main():
    var p: P = {x: 1}
    p.x = 5
    var xs = [1, 2, 3]
    xs[1] = 9
    print(p.x)
    print(xs[1])
"#;
    let (i, v) = chay(src, cfg(&[], vec![]));
    assert_eq!(i.exit, 0, "{:?}", i.diagnostics);
    assert_eq!(i.stdout, "5\n9\n");
    assert_eq!(i.stdout, v.stdout);
    assert_eq!(i.snapshot, baseline());
}

#[test]
fn d2_host_method_vs_closure_value_dispatch() {
    // Host method: receiver list/str ⇒ `builtin::method` (ApplyMethod).
    // Value call: tên hàm/closure ⇒ `call_value` (ApplyValueCall) — KHÔNG đi qua host.
    let src = r#"fn main():
    let inc = fn(x: int) -> int: x + 1
    print(inc(4))
    let xs = [3, 1, 2]
    print(xs.len())
    let s = "abc"
    print(s.len())
"#;
    let (i, v) = chay(src, cfg(&[], vec![]));
    assert_eq!(i.exit, 0, "{:?}", i.diagnostics);
    assert_eq!(i.stdout, "5\n3\n3\n");
    assert_eq!(i.stdout, v.stdout);
    assert_eq!(i.snapshot, baseline());
}

/// Gán ghép trên trường/phần tử (`p.x += 1`) bị **parser** từ chối (VS0201) — không
/// bao giờ tới frame path; ghi rõ để không tưởng là ô bị bỏ sót ở evaluator.
#[test]
fn d2b_compound_field_assign_rejected_by_parser() {
    let src = "type P:\n    x: int\nfn main():\n    var p: P = {x: 1}\n    p.x += 2\n";
    let p = parse(src, Mode::Script);
    assert!(
        p.diagnostics.iter().any(|d| d.code.short == "VS0201"),
        "parser phải từ chối `p.x +=`: {:?}",
        p.diagnostics
    );
}

// ===========================================================================
// E — `await` trong THÂN closure: interp có, VM codegen vẫn hỏng (ngoài phạm vi A)
// ===========================================================================

#[test]
fn e1_closure_body_await_supported_on_both_engines() {
    let src = r#"async fn ext_src() -> int:
    0
fn main():
    let f = fn() -> int:
        await ext_src()
    print(f())
"#;
    let (hir, syms, tys) = tien_trinh(src);
    let i = run_i(
        src,
        &hir,
        &syms,
        &tys,
        &[],
        cfg(&[("ext_src", 1)], vec![ev(0, 1, 0, 42)]),
    );
    assert_eq!(i.exit, 0, "interp phải chạy được: {:?}", i.diagnostics);
    assert_eq!(i.stdout, "42\n");

    // VD-257/B đã sửa đường VM (goi_ham_bytecode đẩy frame vào Th hiện tại): giờ cả hai
    // engine chạy được. Assertion "VM từ chối" của stream A lỗi thời sau khi B merge.
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile trả module");
    let v = run_v(&m, src, &[], cfg(&[("ext_src", 1)], vec![ev(0, 1, 0, 42)]));
    assert_eq!(v.exit, 0, "VM phải chạy được: {:?}", v.diagnostics);
    assert_eq!(v.stdout, "42\n");
}

// ===========================================================================
// F — resume KHÔNG double-charge (dùng đúng hook VD-256, KHÔNG implement budget)
// ===========================================================================

#[test]
fn f1_resume_khong_charge_lai_nhom_cu_vm_budget() {
    let src = r#"async fn src() -> int:
    await ext_src()
async fn ext_src() -> int:
    0
fn main():
    let a = spawn src()
    print(await a)
    print(await a)
"#;
    let (hir, syms, _tys) = tien_trinh(src);
    let (m, fee) = vis_bytecode::compile_with_fee(&hir, &syms, src).expect("compile_with_fee");
    let (o, trace) = run_v_budget(
        &m,
        src,
        &[],
        cfg(&[("ext_src", 1)], vec![ev(0, 1, 0, 40)]),
        fee.clone(),
        1_000_000,
    );
    assert_eq!(o.exit, 0, "{:?}", o.diagnostics);
    assert_eq!(
        o.stdout, "40\n40\n",
        "resume phải lấy cache, không re-run task"
    );

    // Mỗi sự kiện (pc, kind, site) charge ĐÚNG 1 lần dù main đã treo/rồi resume.
    let mut seen = HashSet::new();
    for e in &trace {
        let key = (e.pc, e.kind.tag(), e.site);
        assert!(seen.insert(key), "double-charge tại {:?}", key);
    }
    // Có suspend thật (task chạy giữa 2 lần await của main) → trace có event của task.
    assert!(
        trace
            .iter()
            .any(|e| e.kind.name() == "E_AWAIT" && e.pc == 0),
        "phải thấy E_AWAIT của thân task (chạy giữa suspend/resume)"
    );
    // Fee table tĩnh có đủ sự kiện; không tự dựng/thêm budget ở stream A.
    assert!(!fee.trace().is_empty());
}

/// Interp frame path intentionally KHÔNG gọi `charge` (theo lệnh "đừng implement
/// budget"). Bật hook thì trace vẫn rỗng — chứng minh không có tính năng budget ngầm.
#[test]
fn f2_interp_frame_path_khong_tu_charge() {
    let src = "fn main():\n    print(1)\n";
    let (hir, syms, tys) = tien_trinh(src);
    ifee::enable_for_test(1_000_000);
    let o = run_i(src, &hir, &syms, &tys, &[], cfg(&[], vec![]));
    let trace = ifee::take_trace_for_test();
    ifee::disable_for_test();
    assert_eq!(o.exit, 0);
    assert_eq!(o.stdout, "1\n");
    assert!(trace.is_empty(), "frame path interp không được tự charge");
}
