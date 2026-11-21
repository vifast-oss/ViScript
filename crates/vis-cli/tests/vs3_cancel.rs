// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-1b** — cancellation hợp tác + cleanup trên **đường frame test nội bộ** của cả hai
//! engine (interp + VM). Không API ngôn ngữ/CLI, không đổi runtime mặc định.
//!
//! Hợp đồng của lát này:
//! 1. huỷ tại **safe point** cho `read`/`connect`/`accept`/`write`, timer và wait task;
//! 2. gỡ đăng ký I/O/timer/waiter **đúng một lần**, kể cả khi nhiều task cùng chờ một handle;
//! 3. **không** đóng language handle đã thoát/chia sẻ; handle còn sống giữ cache kết quả/lỗi;
//! 4. final join vẫn surfaced `VS0506` (giữ mã gốc) — không đổi policy task lỗi.
//!
//! Đường `chan` chặn thật vẫn là `VS0505` (wait-set thuộc VS3-2a) — test ở đây chỉ khẳng định
//! **không fallback đồng bộ** và không để lại đăng ký giả.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

use vis_interp::frame::{run_frame_for_test as run_i, FrameConfig, FrameOutput};
use vis_interp::rt::{
    frame_cancel_clear_triggers_for_test, frame_cancel_on_seq_for_test, FrameReactorBackend,
    ReactorEvent, ReactorIoOutcome, ReactorOp, ReactorOutcome, ReactorRawEvent, Scheduler, Wait,
    Waiter,
};
use vis_interp::stdlib::{frame_net_set_write, frame_net_take_write};
use vis_interp::Value;
use vis_syntax::{lex, parse, Mode, Span};
use vis_vm::vm_frame::run_frame_for_test as run_v;

// ---------------------------------------------------------------------------
// Harness chung
// ---------------------------------------------------------------------------

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

fn baseline() -> vis_interp::rt::DiagSnapshot {
    vis_interp::rt::DiagSnapshot {
        live_tasks: 0,
        frames: 0,
        waiters: 0,
        ready: 0,
        regs: 0,
        timers: 0,
    }
}

/// Adapter scripted: mọi I/O trả `WouldBlock`; đếm register/deregister/close.
#[derive(Default)]
struct Scripted {
    events: VecDeque<ReactorRawEvent>,
    registered: Vec<(u64, String, ReactorOp)>,
    deregistered: Vec<u64>,
    closes: Vec<String>,
}

impl FrameReactorBackend for Scripted {
    fn try_read(&mut self, _h: &str, _max: usize) -> ReactorIoOutcome {
        ReactorIoOutcome::WouldBlock
    }
    fn try_accept(&mut self, _h: &str) -> ReactorIoOutcome {
        ReactorIoOutcome::WouldBlock
    }
    fn register(&mut self, token: u64, h: &str, op: ReactorOp) -> Result<(), String> {
        self.registered.push((token, h.to_string(), op));
        Ok(())
    }
    fn deregister(&mut self, token: u64, _h: &str, _op: ReactorOp) -> Result<(), String> {
        self.deregistered.push(token);
        Ok(())
    }
    fn wait(&mut self, _t: Duration) -> Result<Vec<ReactorRawEvent>, String> {
        Ok(self.events.drain(..).collect())
    }
    fn close(&mut self, h: &str) -> Result<(), String> {
        self.closes.push(h.to_string());
        Ok(())
    }
}

/// Kết quả một engine + trạng thái adapter (để so parity cả cleanup).
struct KetQua {
    out: FrameOutput,
    registered: Vec<(u64, String, ReactorOp)>,
    deregistered: Vec<u64>,
    closes: Vec<String>,
}

fn cfg_scripted(sb: &Rc<RefCell<Scripted>>, events: Vec<vis_interp::rt::FakeEvent>) -> FrameConfig {
    FrameConfig {
        events,
        backend: Some(sb.clone()),
        ..Default::default()
    }
}

fn dat_trigger(cancels: &[(u64, u32)]) {
    frame_cancel_clear_triggers_for_test();
    for (seq, id) in cancels {
        frame_cancel_on_seq_for_test(*seq, *id);
    }
}

fn chay_i_scripted(
    src: &str,
    events: Vec<vis_interp::rt::FakeEvent>,
    cancels: &[(u64, u32)],
) -> KetQua {
    let (hir, syms, tys) = tien_trinh(src);
    let sb = Rc::new(RefCell::new(Scripted::default()));
    dat_trigger(cancels);
    let out = run_i(src, &hir, &syms, &tys, &[], cfg_scripted(&sb, events));
    frame_cancel_clear_triggers_for_test();
    let s = sb.borrow();
    KetQua {
        out,
        registered: s.registered.clone(),
        deregistered: s.deregistered.clone(),
        closes: s.closes.clone(),
    }
}

fn chay_v_scripted(
    src: &str,
    events: Vec<vis_interp::rt::FakeEvent>,
    cancels: &[(u64, u32)],
) -> KetQua {
    let (hir, syms, _tys) = tien_trinh(src);
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    let sb = Rc::new(RefCell::new(Scripted::default()));
    dat_trigger(cancels);
    let out = run_v(&m, src, &[], cfg_scripted(&sb, events));
    frame_cancel_clear_triggers_for_test();
    let s = sb.borrow();
    KetQua {
        out,
        registered: s.registered.clone(),
        deregistered: s.deregistered.clone(),
        closes: s.closes.clone(),
    }
}

/// Chạy cả hai engine với **adapter riêng** cho mỗi engine, so stdout/log/snapshot/exit và
/// trạng thái cleanup. Đây là assert chéo engine, không chỉ một engine.
fn chay_hai_scripted(
    src: &str,
    events: Vec<vis_interp::rt::FakeEvent>,
    cancels: &[(u64, u32)],
) -> (KetQua, KetQua) {
    let i = chay_i_scripted(src, events.clone(), cancels);
    let v = chay_v_scripted(src, events, cancels);
    assert_eq!(i.out.stdout, v.out.stdout, "interp↔VM stdout lệch");
    assert_eq!(i.out.log, v.out.log, "interp↔VM log huỷ/scheduler lệch");
    assert_eq!(i.out.snapshot, v.out.snapshot, "interp↔VM snapshot lệch");
    assert_eq!(i.out.exit, v.out.exit, "interp↔VM exit lệch");
    assert_eq!(i.registered, v.registered, "interp↔VM register lệch");
    assert_eq!(i.deregistered, v.deregistered, "interp↔VM deregister lệch");
    assert_eq!(i.closes, v.closes, "interp↔VM close lệch");
    (i, v)
}

const TRIGGER: u64 = 999;

fn trigger_event() -> vis_interp::rt::FakeEvent {
    vis_interp::rt::FakeEvent {
        seq: 0,
        key: TRIGGER,
        gen: 0,
        value: Value::Int(0),
    }
}

fn kiem_ca_huy(out: &FrameOutput, dereg: &[u64], closes: &[String], co_dereg: bool) {
    assert_eq!(
        out.stdout, "main\nV:pre\n",
        "task bị huỷ không được chạy tiếp: {:?}",
        out.stdout
    );
    assert_eq!(out.exit, 0, "huỷ không tự tạo lỗi: {:?}", out.diagnostics);
    assert!(!out.stdout.contains("V:post"));
    assert!(out.log.contains(&"cancel:1".to_string()), "{:?}", out.log);
    assert!(
        !out.log.contains(&"wake:1".to_string()),
        "late wake task đã huỷ: {:?}",
        out.log
    );
    if co_dereg {
        assert_eq!(dereg, &[0], "deregister đúng một lần");
    }
    assert!(closes.is_empty(), "KHÔNG đóng language handle");
    assert_eq!(out.snapshot, baseline(), "{:?}", out.log);
}

// ---------------------------------------------------------------------------
// Nguồn cho từng thao tác — thân task treo ở đúng builtin cần kiểm.
// ---------------------------------------------------------------------------

fn nguon_op(op: &str) -> String {
    format!(
        "use std.net\n\nasync fn victim() -> int:\n    print(\"V:pre\")\n    let r = {op}\n    print(\"V:post\")\n    print(r)\n    1\n\nfn main():\n    let t = spawn victim()\n    print(\"main\")\n"
    )
}

#[test]
fn cancel_read_hai_engine() {
    let src = nguon_op("net.read(\"C1\", 8, 10000)");
    let (i, v) = chay_hai_scripted(&src, vec![trigger_event()], &[(0, 1)]);
    for k in [&i, &v] {
        kiem_ca_huy(&k.out, &k.deregistered, &k.closes, true);
        assert_eq!(
            k.registered.len(),
            1,
            "read phải đăng ký 1 lần: {:?}",
            k.registered
        );
    }
}

#[test]
fn cancel_connect_hai_engine() {
    let src = nguon_op("net.connect_timeout(\"127.0.0.1:1\", 10000)");
    let (i, v) = chay_hai_scripted(&src, vec![trigger_event()], &[(0, 1)]);
    for k in [&i, &v] {
        kiem_ca_huy(&k.out, &k.deregistered, &k.closes, true);
        assert!(
            k.registered.iter().any(|(_, h, _)| h.starts_with("N:")),
            "connect phải đăng ký khoá N:<addr>: {:?}",
            k.registered
        );
    }
}

#[test]
fn cancel_accept_hai_engine() {
    let src = nguon_op("net.accept_timeout(\"L1\", 10000)");
    let (i, v) = chay_hai_scripted(&src, vec![trigger_event()], &[(0, 1)]);
    for k in [&i, &v] {
        kiem_ca_huy(&k.out, &k.deregistered, &k.closes, true);
        assert!(
            k.registered
                .iter()
                .any(|(_, h, op)| h == "L1" && *op == ReactorOp::Accept),
            "accept phải đăng ký op Accept: {:?}",
            k.registered
        );
    }
}

#[test]
fn cancel_write_hai_engine() {
    let src = nguon_op("net.write(\"C1\", \"payload\")");
    let (i, v) = chay_hai_scripted(&src, vec![trigger_event()], &[(0, 1)]);
    for k in [&i, &v] {
        kiem_ca_huy(&k.out, &k.deregistered, &k.closes, true);
        assert!(
            k.registered.iter().any(|(_, h, _)| h.starts_with("W:")),
            "write phải đăng ký khoá W:<tay>: {:?}",
            k.registered
        );
    }
    // Không còn buffer write mồ côi sau khi task bị huỷ (thread-local test-only).
    assert!(
        frame_net_take_write("W:C1").is_none(),
        "buffer write phải được dọn khi huỷ"
    );
}

#[test]
fn cancel_timer_hai_engine() {
    // Không adapter: timer/reactor giả fire tại deadline ⇒ safe point xử lý huỷ TRƯỚC wake.
    let src = nguon_op("net.read(\"C1\", 8, 1000)");
    let (i, v) = chay_hai_fake(
        &src,
        vec![],
        vec![ReactorEvent {
            seq: 0,
            key: 0,
            gen: 0,
            outcome: ReactorOutcome::Ready(Value::ok(Value::Str("X".into()))),
            deadline_ms: Some(0),
        }],
        &[(0, 1)],
    );
    for out in [&i, &v] {
        kiem_ca_huy(out, &[], &[], false);
        assert!(
            out.log.contains(&"cancelregs:1:1".to_string()),
            "timer/reg phải bị gỡ: {:?}",
            out.log
        );
        assert_eq!(out.snapshot.timers, 0, "timer event phải bị gỡ");
    }
}

/// Chạy cả hai engine với reactor giả (không adapter), giống `chay` của lat257_c.
fn chay_hai_fake(
    src: &str,
    events: Vec<vis_interp::rt::FakeEvent>,
    reactors: Vec<ReactorEvent>,
    cancels: &[(u64, u32)],
) -> (FrameOutput, FrameOutput) {
    let (hir, syms, tys) = tien_trinh(src);
    dat_trigger(cancels);
    let i = run_i(
        src,
        &hir,
        &syms,
        &tys,
        &[],
        FrameConfig {
            events: events.clone(),
            reactors: reactors.clone(),
            ..Default::default()
        },
    );
    frame_cancel_clear_triggers_for_test();
    dat_trigger(cancels);
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    let v = run_v(
        &m,
        src,
        &[],
        FrameConfig {
            events,
            reactors,
            ..Default::default()
        },
    );
    frame_cancel_clear_triggers_for_test();
    assert_eq!(i.stdout, v.stdout, "interp↔VM stdout lệch");
    assert_eq!(i.log, v.log, "interp↔VM log lệch");
    assert_eq!(i.snapshot, v.snapshot, "interp↔VM snapshot lệch");
    assert_eq!(i.exit, v.exit, "interp↔VM exit lệch");
    (i, v)
}

// ---------------------------------------------------------------------------
// Cleanup dùng chung một đăng ký (nhiều waiter cùng handle) — đúng một lần.
// ---------------------------------------------------------------------------

fn sch() -> (Scheduler, Rc<RefCell<Scripted>>) {
    let b = Rc::new(RefCell::new(Scripted::default()));
    let mut s = Scheduler::new();
    s.frame_enable_for_test(vec![], vec![]);
    s.frame_install_backend_for_test(b.clone());
    (s, b)
}

fn tao_task(s: &mut Scheduler, ten: &str) -> Rc<vis_interp::rt::Task> {
    s.tao(0, vec![], ten.into(), Span::new(0, 0))
}

#[test]
fn cancel_mot_trong_hai_waiter_cung_handle_khong_deregister() {
    let (mut s, b) = sch();
    let _t1 = tao_task(&mut s, "t1");
    let _t2 = tao_task(&mut s, "t2");
    let span = Span::new(0, 0);

    s.frame.current = 1;
    let _ = s
        .frame_reactor_poll_io(ReactorOp::Read, "C1", 8, 1000, span)
        .unwrap_err();
    s.frame_register(Waiter {
        frame: 1,
        wait: Wait::Reactor { key: 0, gen: 0 },
        gen: 0,
    });
    s.frame.current = 2;
    let _ = s
        .frame_reactor_poll_io(ReactorOp::Read, "C1", 8, 1000, span)
        .unwrap_err();
    s.frame_register(Waiter {
        frame: 2,
        wait: Wait::Reactor { key: 0, gen: 0 },
        gen: 0,
    });

    s.frame_cancel_request_for_test(1);
    assert_eq!(s.frame_cancel_process_for_test(), 1);
    assert!(
        b.borrow().deregistered.is_empty(),
        "còn waiter frame 2: KHÔNG được deregister"
    );
    assert!(
        b.borrow().closes.is_empty(),
        "huỷ không được đóng language handle"
    );
    assert!(
        s.frame
            .reactor
            .regs
            .iter()
            .any(|r| r.key == 0 && r.frame == 2),
        "đăng ký phải chuyển cho waiter còn lại: {:?}",
        s.frame.reactor.regs
    );
    assert_eq!(s.diag_snapshot().waiters, 1);

    s.frame_cancel_request_for_test(2);
    assert_eq!(s.frame_cancel_process_for_test(), 1);
    assert_eq!(b.borrow().deregistered, vec![0], "deregister đúng một lần");
    assert!(s.frame.reactor.regs.is_empty());
    assert_eq!(s.diag_snapshot().waiters, 0);
}

const P_SHARED_WAIT: &str = r#"use std.net

async fn a() -> int:
    print("A:pre")
    let r = net.read("C1", 8, 1000)
    print(r)
    print("A:post")
    1

async fn b() -> int:
    print("B:pre")
    let r = net.read("C1", 8, 1000)
    print(r)
    print("B:post")
    2

fn main():
    let x = spawn a()
    let y = spawn b()
    print("main")
"#;

#[test]
fn cancel_mot_trong_hai_task_cung_handle_nguon_that() {
    // a = task 1, b = task 2; cả hai cùng chờ "C1" (dùng chung key 0). Huỷ a tại safe point:
    // b phải được đánh thức bởi event và chạy xong — huỷ một task không được làm mất
    // đăng ký của task còn lại.
    let (i, v) = chay_hai_fake(
        P_SHARED_WAIT,
        vec![],
        vec![ReactorEvent {
            seq: 0,
            key: 0,
            gen: 0,
            outcome: ReactorOutcome::Ready(Value::ok(Value::Str("S".into()))),
            // Deadline buộc event nằm lại tới khi cả hai task đã đăng ký; `frame_idle` mới fire.
            deadline_ms: Some(0),
        }],
        &[(0, 1)],
    );
    for out in [&i, &v] {
        assert!(out.stdout.contains("A:pre"), "{:?}", out.stdout);
        assert!(out.stdout.contains("B:pre"), "{:?}", out.stdout);
        assert!(!out.stdout.contains("A:post"), "task a bị huỷ");
        assert!(
            out.stdout.contains("B:post"),
            "task b vẫn chạy: {:?} log={:?}",
            out.stdout,
            out.log
        );
        assert_eq!(out.stdout.matches("Ok(S)").count(), 1, "{:?}", out.stdout);
        assert_eq!(out.exit, 0, "{:?}", out.diagnostics);
        assert_eq!(out.snapshot, baseline(), "{:?}", out.log);
        assert!(
            out.log.contains(&"cancelshare:1:0:2".to_string()),
            "phải chuyển quyền đăng ký cho task b: {:?}",
            out.log
        );
        assert!(!out.log.contains(&"canceldereg:1:0".to_string()));
        assert!(!out.log.contains(&"rstale:0".to_string()));
    }
}

#[test]
fn cancel_nhieu_dang_ky_cung_task_deregister_dung_mot_lan() {
    let (mut s, b) = sch();
    let _t = tao_task(&mut s, "t");
    let span = Span::new(0, 0);
    s.frame.current = 1;
    let _ = s
        .frame_reactor_poll_io(ReactorOp::Read, "C1", 8, 1000, span)
        .unwrap_err();
    let _ = s
        .frame_reactor_poll_io(ReactorOp::Accept, "L1", 0, 1000, span)
        .unwrap_err();
    s.frame_register(Waiter {
        frame: 1,
        wait: Wait::Reactor { key: 0, gen: 0 },
        gen: 0,
    });
    assert_eq!(s.diag_snapshot().regs, 2);

    s.frame_cancel_request_for_test(1);
    assert_eq!(s.frame_cancel_process_for_test(), 1);
    let mut dereg = b.borrow().deregistered.clone();
    dereg.sort_unstable();
    assert_eq!(dereg, vec![0, 1], "mỗi đăng ký deregister đúng một lần");
    assert!(s.frame.reactor.regs.is_empty());
    assert!(s.frame.token_map.is_empty());
}

#[test]
fn cancel_write_don_buffer_test_only_dung_mot_lan() {
    let (mut s, b) = sch();
    let _t = tao_task(&mut s, "t");
    let span = Span::new(0, 0);
    let khoa = "W:C1";
    frame_net_set_write(khoa, b"payload");
    s.frame.current = 1;
    let _ = s
        .frame_reactor_poll_io(ReactorOp::Read, khoa, 7, 1000, span)
        .unwrap_err();
    s.frame_register(Waiter {
        frame: 1,
        wait: Wait::Reactor { key: 0, gen: 0 },
        gen: 0,
    });

    s.frame_cancel_request_for_test(1);
    assert_eq!(s.frame_cancel_process_for_test(), 1);
    assert_eq!(b.borrow().deregistered, vec![0], "deregister đúng một lần");
    assert!(
        frame_net_take_write(khoa).is_none(),
        "buffer write của task bị huỷ phải được dọn"
    );
}

#[test]
fn cancel_go_timer_event_dang_cho() {
    let mut s = Scheduler::new();
    s.frame_enable_for_test(vec![], vec![]);
    let _t = tao_task(&mut s, "t");
    let span = Span::new(0, 0);
    s.frame.current = 1;
    let _ = s
        .frame_reactor_poll_io(ReactorOp::Read, "C1", 8, 1000, span)
        .unwrap_err();
    s.frame_register(Waiter {
        frame: 1,
        wait: Wait::Reactor { key: 0, gen: 0 },
        gen: 0,
    });
    s.frame.reactor.events.push(ReactorEvent {
        seq: 0,
        key: 0,
        gen: 0,
        outcome: ReactorOutcome::Ready(Value::ok(Value::Str("X".into()))),
        deadline_ms: Some(500),
    });
    assert_eq!(s.diag_snapshot().timers, 1);

    s.frame_cancel_request_for_test(1);
    assert_eq!(s.frame_cancel_process_for_test(), 1);
    assert!(
        s.frame.reactor.events.is_empty(),
        "event timer của task bị huỷ phải bị gỡ: {:?}",
        s.frame.reactor.events
    );
    assert_eq!(s.diag_snapshot().timers, 0);
}

// ---------------------------------------------------------------------------
// Final join + channel wait (chưa có wait-set: từ chối rõ, không fallback).
// ---------------------------------------------------------------------------

const P_FINAL_JOIN: &str = r#"async fn src1() -> int:
    0

async fn victim() -> int:
    print("V:pre")
    let x = await src1()
    print("V:post")
    1

async fn boom() -> int:
    let x = 1 // 0
    x

fn main():
    let t = spawn victim()
    let b = spawn boom()
    print("main")
"#;

#[test]
fn final_join_surfaced_sau_khi_huy_task_khac() {
    let (i, v) = chay_hai_fake(P_FINAL_JOIN, vec![trigger_event()], vec![], &[(0, 1)]);
    for out in [&i, &v] {
        assert_eq!(out.stdout, "main\nV:pre\n");
        assert_ne!(out.exit, 0, "task boom lỗi phải surfaced");
        assert!(
            out.diagnostics.iter().any(|d| d.code.short == "VS0506"),
            "{:?}",
            out.diagnostics
        );
        assert!(
            out.diagnostics.iter().any(|d| d.message.contains("VS0502")),
            "phải giữ mã gốc VS0502: {:?}",
            out.diagnostics
        );
        assert!(out.log.contains(&"cancel:1".to_string()));
    }
    assert_eq!(i.exit, v.exit);
    assert_eq!(i.diagnostics.len(), v.diagnostics.len());
    for (a, b) in i.diagnostics.iter().zip(&v.diagnostics) {
        assert_eq!(a.code.short, b.code.short);
        assert_eq!(a.message, b.message);
        assert_eq!((a.line, a.col), (b.line, b.col));
    }
}

const P_CACHE: &str = r#"async fn src1() -> int:
    0

async fn good() -> int:
    42

async fn victim() -> int:
    print("V:pre")
    let x = await src1()
    print("V:post")
    0

fn main():
    let g = spawn good()
    let a = spawn victim()
    print(await g)
    print(await g)
"#;

#[test]
fn handle_con_song_giu_cache_sau_khi_huy_task_khac() {
    // good = task 1, victim = task 2, src1 = task 3 (tạo khi victim await).
    let (i, v) = chay_hai_fake(P_CACHE, vec![trigger_event()], vec![], &[(0, 2)]);
    for out in [&i, &v] {
        assert!(
            out.stdout.contains("42\n42\n"),
            "await 2 lần phải trả cache: {:?}",
            out.stdout
        );
        assert!(!out.stdout.contains("V:post"));
        assert_eq!(out.exit, 0, "{:?}", out.diagnostics);
        assert_eq!(out.snapshot, baseline(), "{:?}", out.log);
        assert!(out.log.contains(&"cancel:2".to_string()));
        assert!(!out.log.contains(&"wake:2".to_string()));
    }
}

const P_AWAIT_CANCELLED: &str = r#"async fn src1() -> int:
    0

async fn victim() -> int:
    print("V:pre")
    let x = await src1()
    print("V:post")
    1

fn main():
    let t = spawn victim()
    print(await t)
"#;

#[test]
fn await_task_bi_huy_tra_vs0506_khong_bia_ma_moi() {
    // A0 chưa chốt mã/result khi await task bị huỷ ⇒ pin hành vi hiện tại: VS0506 (mã cũ),
    // không thêm mã cancellation mới, không treo.
    let (i, v) = chay_hai_fake(P_AWAIT_CANCELLED, vec![trigger_event()], vec![], &[(0, 1)]);
    for out in [&i, &v] {
        assert_eq!(out.stdout, "V:pre\n");
        assert_ne!(out.exit, 0, "await task bị huỷ phải lỗi");
        assert!(
            out.diagnostics.iter().any(|d| d.code.short == "VS0506"),
            "phải tái dùng VS0506: {:?}",
            out.diagnostics
        );
        assert!(
            out.diagnostics
                .iter()
                .any(|d| d.message.contains("bị huỷ") || d.message.contains("hủy")),
            "thông điệp phải nói task bị huỷ: {:?}",
            out.diagnostics
        );
    }
    for (a, b) in i.diagnostics.iter().zip(&v.diagnostics) {
        assert_eq!(a.code.short, b.code.short);
        assert_eq!(a.message, b.message);
        assert_eq!((a.line, a.col), (b.line, b.col));
    }
}

const P_CHAN: &str = r#"async fn victim() -> int:
    let c = chan[int](1)
    c.send(1)
    c.send(2)
    print("V:post")
    1

fn main():
    let t = spawn victim()
    print("main")
"#;

#[test]
fn channel_wait_van_tu_choi_ro_khong_fallback_dong_bo() {
    let (i, v) = chay_hai_fake(P_CHAN, vec![], vec![], &[]);
    for out in [&i, &v] {
        assert_ne!(out.exit, 0, "send trên kênh đầy phải trap");
        // Lỗi trong thân task được bọc `VS0506` nhưng phải giữ mã gốc `VS0505`: không fallback
        // đồng bộ, không biến thành trap chung `VS0501`.
        assert!(
            out.diagnostics
                .iter()
                .any(|d| d.code.short == "VS0505" || d.message.contains("VS0505")),
            "phải là deadlock VS0505 (kể cả khi bọc VS0506): {:?}",
            out.diagnostics
        );
        assert!(!out.stdout.contains("V:post"));
        assert!(
            !out.log.iter().any(|l| l.starts_with("cancel")),
            "không được bịa huỷ: {:?}",
            out.log
        );
    }
    assert_eq!(i.exit, v.exit);
    assert_eq!(
        i.diagnostics
            .iter()
            .map(|d| d.code.short)
            .collect::<Vec<_>>(),
        v.diagnostics
            .iter()
            .map(|d| d.code.short)
            .collect::<Vec<_>>()
    );
    for (a, b) in i.diagnostics.iter().zip(&v.diagnostics) {
        assert_eq!(a.message, b.message);
        assert_eq!((a.line, a.col), (b.line, b.col));
    }
}

// ---------------------------------------------------------------------------
