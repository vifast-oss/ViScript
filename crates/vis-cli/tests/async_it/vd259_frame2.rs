// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-259 / frame path (lát 2)** — hồi quy các lỗi còn lại sau VD-258 (chỉ test/debug,
//! KHÔNG bật mặc định). Mỗi ca chạy **cả hai engine** khi áp dụng được.
//!
//! | Ca | Lỗi cũ |
//! |---|---|
//! | `closure_body_*` | `Box::leak(HBody)` mỗi lần gọi closure (rò rỉ theo call) |
//! | `fake_event_*` | `frame_enable_for_test` không sắp trace theo `seq` ⇒ event đảo thứ tự |
//! | `snapshot_*` | `live_frames` không bao giờ đặt; snapshot đo **sau** shutdown ⇒ luôn 0 |
//! | `frame_terminal/huy_*` | frame đã xong/bị huỷ vẫn nằm trong map driver |
//! | `timer_*` | `frame_idle` ngủ tới deadline nhỏ nhất nhưng fire event theo `seq` ⇒ timer sớm |
//! | `default_main/task_*` | `main`/task `async` bỏ qua default tham số ⇒ `Unit` |
//! | `ten_la_*` | fallback `u32::MAX` của chế độ không `main` nuốt mọi tên lạ thành argv |
//! | `reclaim_*`, `task_done_*` | `dead_keys` tính sau `retain`; `frame_task_done` nuốt `kq` |
//! | `quota_*` | `u64 → i64` quay âm với hạn lớn |

use vis_core::frame::{FakeEvent, ReactorEvent, ReactorOutcome};
use vis_core::frame::{FrameConfig, FrameOutput};
use vis_core::reactor::ReactorOp;
use vis_core::value::TrangThai;
use vis_core::Value;
use vis_interp::frame::run_frame_for_test as run_i;
use vis_interp::rt::{fee as ifee, ReactorReg, Scheduler};
use vis_syntax::{lex, parse, Mode};
use vis_vm::vm_frame::{
    frame_budget_disable_for_test, frame_budget_enable_for_test, frame_quota_yields_for_test,
    run_frame_for_test as run_v,
};

fn tien_trinh(src: &str) -> (vis_hir::Hir, vis_hir::Symbols, vis_hir::TypeTable) {
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

/// Chạy cùng nguồn + cùng config trên hai engine.
fn chay(src: &str, c: FrameConfig) -> (FrameOutput, FrameOutput) {
    let (hir, syms, tys) = tien_trinh(src);
    let i = run_i(src, &hir, &syms, &tys, &[], c.clone());
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    let v = run_v(&m, src, &[], c);
    (i, v)
}

fn assert_diag_parity(i: &FrameOutput, v: &FrameOutput) {
    assert_eq!(
        i.diagnostics.len(),
        v.diagnostics.len(),
        "số chẩn đoán lệch"
    );
    for (a, b) in i.diagnostics.iter().zip(&v.diagnostics) {
        assert_eq!(a.code.short, b.code.short, "mã chẩn đoán lệch");
        assert_eq!(a.message, b.message, "nội dung chẩn đoán lệch");
        assert_eq!((a.line, a.col), (b.line, b.col), "vị trí chẩn đoán lệch");
    }
}

fn fake_cfg(fake: &[(&str, u64)], events: Vec<FakeEvent>) -> FrameConfig {
    FrameConfig {
        fake_sources: fake.iter().map(|(n, k)| ((*n).into(), *k)).collect(),
        events,
        ..Default::default()
    }
}

fn ev(seq: u64, key: u64, gen: u64, value: Value) -> FakeEvent {
    FakeEvent {
        seq,
        key,
        gen,
        value,
    }
}

fn ready(seq: u64, key: u64, value: &str) -> ReactorEvent {
    ReactorEvent {
        seq,
        key,
        gen: 0,
        outcome: ReactorOutcome::Ready(Value::ok(Value::Str(value.into()))),
        deadline_ms: None,
    }
}

fn timer(seq: u64, key: u64, deadline_ms: u64) -> ReactorEvent {
    ReactorEvent {
        seq,
        key,
        gen: 0,
        outcome: ReactorOutcome::Ready(Value::ok(Value::Str("stale".into()))),
        deadline_ms: Some(deadline_ms),
    }
}

// ===========================================================================
// 1) Closure: không dựng lại thân mỗi lần gọi
// ===========================================================================

/// Gọi cùng một closure 2.000 lần. Bản cũ `Box::leak(HBody)` **mỗi call**; nay mỗi closure
/// dựng tối đa một bản `'static` (bộ đếm chỉ tăng 1), hành vi vẫn đúng.
#[test]
fn closure_body_chi_dung_mot_lan_cho_moi_closure() {
    let src = r#"fn main():
    let f = fn(x: int) -> int:
        x + 1
    var i = 0
    var tong = 0
    while i < 2000:
        tong = tong + f(i)
        i = i + 1
    print(tong)
"#;
    vis_core::value::frame_body_builds_reset_for_test();
    let (i, v) = chay(src, FrameConfig::default());
    let builds = vis_core::value::frame_body_builds_for_test();
    for (name, o) in [("interp", &i), ("vm", &v)] {
        assert_eq!(o.exit, 0, "{name}: {:?}", o.diagnostics);
        assert_eq!(o.stdout, "2001000\n", "{name}: hành vi closure sai");
    }
    assert_eq!(builds, 1, "thân closure bị dựng lại theo từng lần gọi");
}

// ===========================================================================
// 2) Trace `FakeEvent` phải được sắp theo `seq`
// ===========================================================================

/// Trace ghi ngược `seq` (1 rồi 0). Bản cũ fire đúng thứ tự caller ⇒ event `seq=1` tới khi
/// task tương ứng chưa tồn tại (stale), rồi deadlock. Nay sort ở `frame_enable_for_test`.
#[test]
fn fake_event_dao_thu_tu_van_chay_theo_seq() {
    let src = r#"async fn lay_a() -> int:
    100
async fn lay_b() -> int:
    200
async fn victim() -> int:
    let a = await lay_a()
    print("A:{a}")
    let b = await lay_b()
    print("B:{b}")
    a + b
fn main():
    let t = spawn victim()
    print(await t)
"#;
    let c = fake_cfg(
        &[("lay_a", 1), ("lay_b", 2)],
        vec![ev(1, 2, 0, Value::Int(200)), ev(0, 1, 0, Value::Int(100))],
    );
    let (i, v) = chay(src, c);
    for (name, o) in [("interp", &i), ("vm", &v)] {
        assert_eq!(o.exit, 0, "{name}: {:?}", o.diagnostics);
        assert_eq!(o.stdout, "A:100\nB:200\n300\n", "{name}: thứ tự seq sai");
    }
    assert_diag_parity(&i, &v);
}

// ===========================================================================
// 3) Lifecycle: frame terminal/huỷ bị bỏ; snapshot đo trước shutdown
// ===========================================================================

/// Chương trình bình thường (main + một task `await`) không được giữ frame terminal nào.
#[test]
fn frame_terminal_khong_bi_giu_trong_map() {
    let src = r#"async fn w() -> int:
    1
fn main():
    let t = spawn w()
    print(await t)
"#;
    let (i, v) = chay(src, FrameConfig::default());
    for (name, o) in [("interp", &i), ("vm", &v)] {
        assert_eq!(o.exit, 0, "{name}: {:?}", o.diagnostics);
        assert_eq!(o.stdout, "1\n");
        assert_eq!(o.snapshot.frames, 0, "{name}: frame terminal còn sống");
        assert_eq!(o.snapshot.live_tasks, 0, "{name}: task còn sống");
        assert_eq!(o.snapshot.waiters, 0, "{name}: waiter còn sống");
    }
}

/// Task bị huỷ ở safe point: driver phải bỏ `IFrame`/`Th` của nó, không giữ tới cuối.
#[test]
fn frame_bi_huy_khong_bi_giu_trong_map() {
    let src = r#"async fn src1() -> int:
    0
async fn victim() -> int:
    print("V:pre")
    let x = await src1()
    print("V:post")
    x
fn main():
    let a = spawn victim()
    print("main")
"#;
    let c = fake_cfg(&[("src1", 1)], vec![ev(0, 1, 0, Value::Int(7))]);
    let (hir, syms, tys) = tien_trinh(src);
    // Trigger huỷ là thread-local và bị drain khi bật frame ⇒ đăng ký lại cho từng engine.
    vis_interp::rt::frame_cancel_clear_triggers_for_test();
    vis_interp::rt::frame_cancel_on_seq_for_test(0, 1);
    let i = run_i(src, &hir, &syms, &tys, &[], c.clone());
    vis_interp::rt::frame_cancel_clear_triggers_for_test();
    vis_interp::rt::frame_cancel_on_seq_for_test(0, 1);
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    let v = run_v(&m, src, &[], c);
    vis_interp::rt::frame_cancel_clear_triggers_for_test();
    for (name, o) in [("interp", &i), ("vm", &v)] {
        assert_eq!(o.exit, 0, "{name}: {:?}", o.diagnostics);
        assert_eq!(o.stdout, "main\nV:pre\n", "{name}: chạy tiếp sau huỷ");
        assert_eq!(o.snapshot.frames, 0, "{name}: frame bị huỷ còn sống");
        assert_eq!(o.snapshot.live_tasks, 0, "{name}: task bị huỷ chưa thu hồi");
        assert!(
            o.log.contains(&"cancel:1".to_string()),
            "{name}: {:?}",
            o.log
        );
    }
}

/// Deadlock I/O (không event): trước khi dọn, snapshot phải thấy **frame/waiter/reg thật** —
/// bản cũ đo sau `frame_reactor_shutdown()` nên luôn 0 và không có bằng chứng vòng đời.
#[test]
fn snapshot_ro_live_frame_waiter_reg_truoc_shutdown() {
    let src = r#"use std.net
async fn victim() -> int:
    let r = net.read("C1", 8, 1000)
    print("V:post")
    1
fn main():
    let t = spawn victim()
    print("main")
"#;
    let (i, v) = chay(src, FrameConfig::default());
    for (name, o) in [("interp", &i), ("vm", &v)] {
        assert_eq!(o.exit, 1, "{name}: phải deadlock");
        assert_eq!(
            o.diagnostics[0].code.short, "VS0505",
            "{name}: {:?}",
            o.diagnostics
        );
        assert_eq!(o.stdout, "main\n", "{name}: {:?}", o.stdout);
        assert!(
            o.snapshot.frames >= 1,
            "{name}: live_frames luôn 0? {:?}",
            o.snapshot
        );
        assert!(
            o.snapshot.waiters >= 1,
            "{name}: waiter mất: {:?}",
            o.snapshot
        );
        assert!(o.snapshot.regs >= 1, "{name}: reg mất: {:?}", o.snapshot);
        assert!(
            o.snapshot.live_tasks >= 1,
            "{name}: task mất: {:?}",
            o.snapshot
        );
    }
    assert_diag_parity(&i, &v);
}

/// `diag_snapshot().timers` phải phản ánh trace **trước** shutdown, không bị xoá trắng.
#[test]
fn snapshot_timer_truoc_shutdown_khong_vacuous() {
    let mut s = Scheduler::default();
    s.frame_enable_for_test(vec![], vec![]);
    s.frame_reactor_load_for_test(vec![timer(0, 0, 5)], false);
    assert_eq!(s.diag_snapshot().timers, 1, "timer bị mất trước shutdown");
    s.frame_reactor_shutdown();
    assert_eq!(s.diag_snapshot().timers, 0, "shutdown phải dọn timer");
}

// ===========================================================================
// 4) Timer: ngủ tới deadline của chính event kế tiếp; cộng `Instant` có kiểm tràn
// ===========================================================================

/// Trace trộn I/O + timer, `seq` quyết định thứ tự: timer `seq=0` deadline 80 ms đứng trước
/// I/O `seq=1`; timer `seq=2` deadline 15 ms (đứng sau). Bản cũ ngủ theo deadline **nhỏ nhất**
/// (15 ms) rồi fire timer `seq=0` sớm ~65 ms. Nay phải đủ ~80 ms.
#[test]
fn timer_khong_fire_som_du_deadline_khac_nho_hon() {
    let src = r#"use std.net
async fn io() -> int:
    let r = net.read("C1", 8, 1000)
    1
fn main():
    let a = spawn io()
    print(await a)
"#;
    let c = FrameConfig {
        reactors: vec![timer(0, 99, 80), ready(1, 0, "A"), timer(2, 98, 15)],
        timer_real_wait: true,
        ..Default::default()
    };
    let t0 = std::time::Instant::now();
    let (i, v) = chay(src, c);
    let elapsed = t0.elapsed();
    for (name, o) in [("interp", &i), ("vm", &v)] {
        assert_eq!(o.exit, 0, "{name}: {:?}", o.diagnostics);
        assert_eq!(o.stdout, "1\n", "{name}: {:?}", o.stdout);
    }
    assert!(
        elapsed >= std::time::Duration::from_millis(55),
        "timer fire sớm: chỉ {:?}",
        elapsed
    );
}

// ===========================================================================
// 5) Default tham số của `main` + task `async`
// ===========================================================================

/// `main(a, b: int = K + 2)` và `async fn cong(a, b: int = K + 1)` — cả hai phải nhận default,
/// không phải `Unit`. VM đã có prologue default; interp dùng chung helper với `dispatch`.
#[test]
fn default_tham_so_main_va_task_async() {
    let src = r#"const K: int = 3
async fn cong(a: int, b: int = K + 1) -> int:
    a + b
fn main(a: list[str], b: int = K + 2):
    print(b)
    print(await cong(2))
"#;
    let (i, v) = chay(src, FrameConfig::default());
    for (name, o) in [("interp", &i), ("vm", &v)] {
        assert_eq!(o.exit, 0, "{name}: {:?}", o.diagnostics);
        assert_eq!(o.stdout, "5\n6\n", "{name}: default bị bỏ qua");
    }
    assert_diag_parity(&i, &v);
}

// ===========================================================================
// 6) `frame_reclaim` + `frame_task_done`
// ===========================================================================

/// `dead_keys` phải gom **trước** `retain` — bản cũ gom sau nên `results`/`fired` của frame
/// chết bị giữ lại.
#[test]
fn reclaim_xoa_results_va_fired_cua_frame_chet() {
    let mut s = Scheduler::default();
    let t = s.tao(0, Vec::new(), "t".into(), vis_syntax::Span::new(0, 0));
    let id = t.id;
    t.trang_thai.set(TrangThai::Xong);
    drop(t);
    s.frame.joined.insert(id);
    s.frame.reactor.regs.push(ReactorReg {
        key: 7,
        handle: "C1".into(),
        op: ReactorOp::Read,
        frame: id,
        gen: 0,
        token: 0,
        deadline: None,
    });
    s.frame
        .reactor
        .results
        .insert(7, ReactorOutcome::Ready(Value::ok(Value::Str("x".into()))));
    s.frame.reactor.fired.insert(7, 0);
    s.frame_reclaim();
    assert!(s.frame.reactor.regs.is_empty(), "reg chưa gỡ");
    assert!(!s.frame.reactor.results.contains_key(&7), "results rò rỉ");
    assert!(!s.frame.reactor.fired.contains_key(&7), "fired rò rỉ");
}

/// `frame_task_done` phải ghi `kq` khi engine chưa ghi (hợp đồng doc), không nuốt tham số.
#[test]
fn task_done_ghi_ket_qua_khi_chua_cache() {
    let mut s = Scheduler::default();
    let t = s.tao(0, Vec::new(), "t".into(), vis_syntax::Span::new(0, 0));
    let id = t.id;
    s.frame_task_done(id, Ok(Value::Int(9)));
    assert!(
        matches!(t.kq.borrow().as_ref(), Some(Ok(Value::Int(9)))),
        "kq chưa được ghi: {:?}",
        t.kq.borrow()
    );
}

// ===========================================================================
// 7) Quota: hạn `u64` lớn không được quay âm
// ===========================================================================

#[test]
fn quota_u64_lon_bao_hoa_khong_quay_am() {
    let src = "fn main():\n    print(\"q\")\n";
    let (hir, syms, tys) = tien_trinh(src);

    // Interp: budget + quota cùng hạn `u64::MAX`.
    ifee::enable_for_test(u64::MAX);
    ifee::frame_emit_enable_for_test();
    ifee::quota_enable_for_test(u64::MAX);
    let i = run_i(src, &hir, &syms, &tys, &[], FrameConfig::default());
    let yi = ifee::quota_yields_for_test();
    ifee::disable_for_test();
    assert_eq!(i.exit, 0, "interp: {:?}", i.diagnostics);
    assert_eq!(yi, 0, "interp: quota u64::MAX quay âm ⇒ yield oan");

    // VM: cùng hạn qua hook frame.
    let (m, fee) = vis_bytecode::compile_with_fee(&hir, &syms, src).expect("compile_with_fee");
    frame_budget_enable_for_test(fee, u64::MAX, Some(u64::MAX));
    let v = run_v(&m, src, &[], FrameConfig::default());
    let yv = frame_quota_yields_for_test();
    frame_budget_disable_for_test();
    assert_eq!(v.exit, 0, "VM: {:?}", v.diagnostics);
    assert_eq!(yv, 0, "VM: quota u64::MAX quay âm ⇒ yield oan");
}
