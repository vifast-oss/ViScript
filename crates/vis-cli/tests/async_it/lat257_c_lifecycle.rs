// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-257 / stream C — cancellation hợp tác & lifecycle/cleanup** trên **đường frame
//! test nội bộ** của cả hai engine (interp + VM). Không API ngôn ngữ, không CLI flag,
//! không đổi runtime mặc định (`vis run`). Mọi cơ chế huỷ nằm ở `vis_interp::rt`
//! (`frame_cancel_*`, `TrangThai::Huy`), dùng chung bởi hai engine.
//!
//! Trigger huỷ được ghi vào thread-local (`frame_cancel_on_seq_for_test`) TRƯỚC khi chạy
//! engine; `frame_enable_for_test` drains vào `FrameTest.cancel_on_seq`. Khi event có `seq`
//! đó fire, scheduler ghi nhận request rồi xử lý tại đúng SAFE POINT — TRƯỚC khi wake,
//! nên task đang treo không bao giờ chạy lại source (không cắt giữa lệnh).

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::{Duration, Instant};

use vis_interp::frame::{run_frame_for_test as run_i, FrameConfig, FrameOutput};
use vis_interp::rt::{
    frame_cancel_clear_triggers_for_test, frame_cancel_on_seq_for_test, DiagSnapshot, FakeEvent,
    FrameReactorBackend, ReactorEvent, ReactorIoOutcome, ReactorOp, ReactorOutcome,
    ReactorRawEvent, Scheduler, Wait, Waiter,
};
use vis_interp::Value;
use vis_syntax::{lex, parse, Mode, Span};
use vis_vm::vm_frame::run_frame_for_test as run_v;

// ---------------------------------------------------------------------------
// Helpers dùng chung
// ---------------------------------------------------------------------------

fn tien_trinh(src: &str) -> (vis_types::Hir, vis_types::Symbols, vis_types::TypeTable) {
    let p = parse(src, Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse: {:?}", p.diagnostics);
    let l = lex(src, Mode::Script);
    // `std.net` là module opt-in — khớp đường test net thật (lat2/lat256).
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

fn fake(key: u64) -> FrameConfig {
    FrameConfig {
        fake_sources: vec![("src1".into(), key)],
        ..Default::default()
    }
}

/// Chạy cả hai engine với cùng trigger huỷ, so sánh byte stdout/log/snapshot/exit.
fn chay(src: &str, c: FrameConfig, cancels: &[(u64, u32)]) -> (FrameOutput, FrameOutput) {
    let (hir, syms, tys) = tien_trinh(src);

    frame_cancel_clear_triggers_for_test();
    for (seq, id) in cancels {
        frame_cancel_on_seq_for_test(*seq, *id);
    }
    let i = run_i(src, &hir, &syms, &tys, &[], c.clone());

    frame_cancel_clear_triggers_for_test();
    for (seq, id) in cancels {
        frame_cancel_on_seq_for_test(*seq, *id);
    }
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    let v = run_v(&m, src, &[], c);

    assert_eq!(i.stdout, v.stdout, "interp↔VM stdout lệch");
    assert_eq!(i.log, v.log, "interp↔VM trace huỷ/scheduler lệch");
    assert_eq!(i.snapshot, v.snapshot, "interp↔VM snapshot lệch");
    assert_eq!(i.exit, v.exit, "interp↔VM exit lệch");
    frame_cancel_clear_triggers_for_test();
    (i, v)
}

/// Adapter scripted (test-only) — đếm register/deregister/close để kiểm chứng "đúng một lần".
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

// ===========================================================================
// (1) Huỷ task đang treo ở fake wait ⇒ dừng tại safe point, KHÔNG re-run source.
// ===========================================================================

/// `victim` treo ở `await src1()` (fake source, key=1). Event seq=0 chính là event sẽ fire
/// nguồn đó — nhưng trigger huỷ (0, 1) được xử lý TRƯỚC khi wake ⇒ victim không chạy tiếp.
const P_CANCEL: &str = r#"async fn src1() -> int:
    0

async fn victim() -> int:
    print("V:pre")
    let x = await src1()
    print("V:post")
    99

fn main():
    let a = spawn victim()
    print("main")
"#;

fn cancel_cfg() -> FrameConfig {
    let mut c = fake(1);
    c.events = vec![FakeEvent {
        seq: 0,
        key: 1,
        gen: 0,
        value: Value::Int(7),
    }];
    c
}

#[test]
fn c1_cancel_while_waiting_stops_at_safe_point() {
    // victim = task id 1 (spawn đầu tiên, src1 = task id 2 do `tao` khi await).
    let (i, v) = chay(P_CANCEL, cancel_cfg(), &[(0, 1)]);
    for out in [&i, &v] {
        assert_eq!(
            out.stdout, "main\nV:pre\n",
            "V:post không được chạy: {:?}",
            out.stdout
        );
        assert_eq!(out.stdout.matches("V:pre").count(), 1, "không re-run");
        assert!(
            !out.stdout.contains("V:post"),
            "task đã huỷ vẫn chạy tiếp source"
        );
        assert!(
            out.log.contains(&"cancelreq:1".to_string()),
            "{:?}",
            out.log
        );
        assert!(
            out.log.contains(&"cancelwait:1".to_string()),
            "{:?}",
            out.log
        );
        assert!(out.log.contains(&"cancel:1".to_string()), "{:?}", out.log);
        assert!(
            !out.log.contains(&"wake:1".to_string()),
            "late wake của task đã huỷ"
        );
    }
}

#[test]
fn c2_cleanup_returns_to_baseline() {
    let (i, v) = chay(P_CANCEL, cancel_cfg(), &[(0, 1)]);
    assert_eq!(
        i.snapshot,
        baseline(),
        "regs/waiters/timers/ready về 0 (interp)"
    );
    assert_eq!(
        v.snapshot,
        baseline(),
        "regs/waiters/timers/ready về 0 (VM)"
    );
}

#[test]
fn c3_no_handle_task_reclaimed() {
    let (i, v) = chay(P_CANCEL, cancel_cfg(), &[(0, 1)]);
    for out in [&i, &v] {
        assert_eq!(out.snapshot.live_tasks, 0);
        assert!(
            out.log.contains(&"reclaim:1".to_string()),
            "task đã huỷ, hết handle + join xong ⇒ thu hồi: {:?}",
            out.log
        );
        assert!(
            out.log.contains(&"reclaim:2".to_string()),
            "fake source cũng về baseline: {:?}",
            out.log
        );
    }
}

// ===========================================================================
// (2) Final join giữ nguyên — task không await VẪN chạy, KHÔNG auto-cancel.
// ===========================================================================

#[test]
fn c4_unawaited_task_still_final_joined() {
    let src = "async fn lonely() -> int:\n    print(\"lonely\")\n    9\n\nfn main():\n    let t = spawn lonely()\n    print(\"main\")\n";
    let (i, v) = chay(src, FrameConfig::default(), &[]);
    for out in [&i, &v] {
        assert_eq!(
            out.stdout, "main\nlonely\n",
            "task không await vẫn được join cuối"
        );
        assert!(
            !out.log.iter().any(|l| l.starts_with("cancel")),
            "không auto-cancel"
        );
        assert_eq!(out.snapshot, baseline());
    }
}

// ===========================================================================
// (3) Handle ngôn ngữ (net "C<n>") sống qua lần task kết thúc — KHÔNG bị đóng.
// ===========================================================================

const P_SHARED_HANDLE: &str = r#"use std.net

async fn first() -> int:
    let r = net.read("C1", 8, 1000)
    print(r)
    1

async fn second() -> int:
    let r = net.read("C1", 8, 1000)
    print(r)
    2

fn main():
    let a = spawn first()
    let b = spawn second()
    print(await a)
    print(await b)
"#;

fn ok_ev(seq: u64, key: u64, s: &str) -> ReactorEvent {
    ReactorEvent {
        seq,
        key,
        gen: 0,
        outcome: ReactorOutcome::Ready(Value::ok(Value::Str(s.into()))),
        deadline_ms: None,
    }
}

#[test]
fn c5_shared_handle_survives_task_end() {
    let c = FrameConfig {
        reactors: vec![ok_ev(0, 0, "AAA"), ok_ev(1, 1, "BBB")],
        ..Default::default()
    };
    let (i, v) = chay(P_SHARED_HANDLE, c, &[]);
    for out in [&i, &v] {
        assert!(
            out.stdout.contains("Ok(AAA)"),
            "task 1 đọc C1: {:?}",
            out.stdout
        );
        assert!(
            out.stdout.contains("Ok(BBB)"),
            "C1 vẫn dùng được sau task 1 kết thúc: {:?}",
            out.stdout
        );
        assert_eq!(out.exit, 0, "{:?}", out.diagnostics);
        assert_eq!(out.snapshot, baseline());
    }
}

/// Bằng chứng trực tiếp: huỷ một task đang chờ trên "C1" ⇒ adapter `deregister` đúng
/// một lần nhưng `close` (đóng handle) KHÔNG BAO GIỜ được gọi — phân biệt tài nguyên chờ
/// do task sở hữu với handle ngôn ngữ có thể thoát task / được chia sẻ.
#[test]
fn c5b_cancel_does_not_close_language_handle() {
    let b = Rc::new(RefCell::new(Scripted::default()));
    let mut s = Scheduler::new();
    s.frame_enable_for_test(vec![], vec![]);
    s.frame_install_backend_for_test(b.clone());
    let span = Span::new(0, 0);
    // Task thật trong scheduler (cancelled task phải tồn tại, trạng thái Moi).
    let _t1 = s.tao(0, vec![], "t1".into(), span);
    let _t2 = s.tao(0, vec![], "t2".into(), span);

    s.frame.current = 1;
    let _ = s
        .frame_reactor_poll_io(ReactorOp::Read, "C1", 8, 1000, span)
        .unwrap_err();
    s.frame_register(Waiter {
        frame: 1,
        wait: Wait::Reactor { key: 0, gen: 0 },
        gen: 0,
    });
    // Một task KHÁC chờ trên handle "C2" — không bị huỷ.
    s.frame.current = 2;
    let _ = s
        .frame_reactor_poll_io(ReactorOp::Read, "C2", 8, 1000, span)
        .unwrap_err();

    // Huỷ frame 1: chỉ gỡ đăng ký của frame 1.
    s.frame_cancel_request_for_test(1);
    assert_eq!(s.frame_cancel_process_for_test(), 1);

    let bb = b.borrow();
    assert_eq!(bb.deregistered, vec![0], "deregister đúng 1 lần");
    assert!(bb.closes.is_empty(), "KHÔNG đóng language handle `C1`");
    assert!(
        s.frame
            .reactor
            .regs
            .iter()
            .any(|r| r.frame == 2 && r.handle == "C2"),
        "đăng ký chờ của task khác không bị đụng vào"
    );
    assert_eq!(s.diag_snapshot().waiters, 0, "waiter của task bị huỷ đã gỡ");
}

// ===========================================================================
// (4) Cache result/error của task còn handle sống qua cleanup (hợp đồng VS0506).
// ===========================================================================

#[test]
fn c6_cached_result_survives_cleanup() {
    let src = r#"async fn src1() -> int:
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
    // victim = task 2 (good = task 1, src1 = task 3). Huỷ victim (2) tại seq 0.
    let (i, v) = chay(src, cancel_cfg(), &[(0, 2)]);
    for out in [&i, &v] {
        assert!(
            out.stdout.contains("42\n42\n"),
            "await 2 lần trả cache: {:?}",
            out.stdout
        );
        assert!(!out.stdout.contains("V:post"));
        assert_eq!(out.snapshot, baseline());
        assert!(out.log.contains(&"cancel:2".to_string()));
    }
}

// ===========================================================================
// (5) close / timeout / error trong wait: deregister ĐÚNG MỘT LẦN, không wake nhầm.
// ===========================================================================

#[test]
fn c7_close_during_wait_deregisters_once_and_no_late_wake() {
    let b = Rc::new(RefCell::new(Scripted::default()));
    let mut s = Scheduler::new();
    s.frame_enable_for_test(vec![], vec![]);
    s.frame_install_backend_for_test(b.clone());
    let span = Span::new(0, 0);

    // frame 1 chờ "C1" (key0/token0), frame 2 chờ "C2" (key1/token1).
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
        .frame_reactor_poll_io(ReactorOp::Read, "C2", 8, 1000, span)
        .unwrap_err();
    s.frame_register(Waiter {
        frame: 2,
        wait: Wait::Reactor { key: 1, gen: 0 },
        gen: 0,
    });

    // close C1 ⇒ deregister token0 đúng một lần; C2 không bị đụng vào.
    s.frame_reactor_cancel_handle("C1");
    assert_eq!(b.borrow().deregistered, vec![0]);
    assert!(s.frame.reactor.regs.iter().any(|r| r.handle == "C2"));

    // frame 1 được wake để retry; tiêu thụ xong thì không còn reg C1.
    assert_eq!(s.frame_next(), Some(1));

    // Raw event cũ (token0) ⇒ stale, KHÔNG đánh thức task khác (kể cả frame 2).
    b.borrow_mut().events.push_back(ReactorRawEvent {
        token: 0,
        readable: true,
        hup: false,
        error: false,
    });
    assert_eq!(s.frame_next(), None, "token cũ sau close phải stale");
    assert!(
        s.frame
            .reactor
            .log
            .iter()
            .any(|l| l.starts_with("rstale:token:0")),
        "{:?}",
        s.frame.reactor.log
    );
    assert_eq!(b.borrow().deregistered, vec![0], "không deregister lại");
}

#[test]
fn c8_timeout_deregisters_once_no_late_wake() {
    let b = Rc::new(RefCell::new(Scripted::default()));
    let mut s = Scheduler::new();
    s.frame_enable_for_test(vec![], vec![]);
    s.frame_install_backend_for_test(b.clone());
    let span = Span::new(0, 0);

    s.frame.current = 1;
    let _ = s
        .frame_reactor_poll_io(ReactorOp::Read, "C1", 8, 50, span)
        .unwrap_err();
    s.frame_register(Waiter {
        frame: 1,
        wait: Wait::Reactor { key: 0, gen: 0 },
        gen: 0,
    });
    // Ép deadline đã quá hạn mà không sleep — đường timeout chạy tất định.
    if let Some(r) = s.frame.reactor.regs.iter_mut().find(|r| r.key == 0) {
        r.deadline = Some(Instant::now() - Duration::from_secs(1));
    }
    // idle: backend.wait trả rỗng ⇒ nhánh expired wake frame 1.
    assert!(s.frame_idle(), "phải wake vì deadline đã hết");
    assert_eq!(s.frame_next(), Some(1));
    // retry: thấy deadline hết ⇒ gỡ đúng một lần, trả Result.Err(timeout).
    b.borrow_mut().events.clear();
    let r = s.frame_reactor_poll_io(ReactorOp::Read, "C1", 8, 50, span);
    match r {
        Ok(v) => assert!(
            matches!(v, Value::Enum(_, ref variant, _) if variant == "Err"),
            "phải là Err(timeout): {:?}",
            v
        ),
        Err(e) => panic!("timeout phải trả Result.Err, không trap: {:?}", e),
    }
    assert_eq!(b.borrow().deregistered, vec![0], "deregister đúng 1 lần");
    assert!(s.frame.reactor.regs.is_empty(), "reg đã gỡ");
}

#[test]
fn c9_error_during_wait_deregisters_once_and_cannot_wake_wrong_task() {
    // Fake reactor: event Error cho key0 (task A) + event Ready cho key1 (task B).
    // Error ⇒ Result.Err đúng một lần; event key0 lặp/gen lệch ⇒ stale, không wake nhầm B.
    let src = r#"use std.net

async fn a() -> int:
    let r = net.read("A", 8, 1000)
    print(r)
    1

async fn b() -> int:
    let r = net.read("B", 8, 1000)
    print(r)
    2

fn main():
    let x = spawn a()
    let y = spawn b()
    print(await x)
    print(await y)
"#;
    let err = ReactorEvent {
        seq: 0,
        key: 0,
        gen: 0,
        outcome: ReactorOutcome::Error("lỗi đọc".into()),
        deadline_ms: None,
    };
    let c = FrameConfig {
        reactors: vec![err, ok_ev(1, 1, "OKB")],
        ..Default::default()
    };
    let (i, v) = chay(src, c, &[]);
    for out in [&i, &v] {
        assert_eq!(
            out.stdout.matches("Err(").count(),
            1,
            "Err đúng một lần: {:?}",
            out.stdout
        );
        assert!(
            out.stdout.contains("Ok(OKB)"),
            "task B không bị wake nhầm: {:?}",
            out.stdout
        );
        assert_eq!(out.snapshot, baseline());
    }
}

#[test]
fn c10_token_generation_respected_on_cancel() {
    let b = Rc::new(RefCell::new(Scripted::default()));
    let mut s = Scheduler::new();
    s.frame_enable_for_test(vec![], vec![]);
    s.frame_install_backend_for_test(b.clone());
    let span = Span::new(0, 0);
    let _t1 = s.tao(0, vec![], "t1".into(), span);

    s.frame.current = 1;
    let _ = s
        .frame_reactor_poll_io(ReactorOp::Read, "C1", 8, 1000, span)
        .unwrap_err();
    assert_eq!(b.borrow().registered.last().unwrap().0, 0, "token đầu = 0");
    s.frame_register(Waiter {
        frame: 1,
        wait: Wait::Reactor { key: 0, gen: 0 },
        gen: 0,
    });

    // Huỷ: token0 bị gỡ, sổ token rỗng.
    s.frame_cancel_request_for_test(1);
    assert_eq!(s.frame_cancel_process_for_test(), 1);
    assert!(s.frame.token_map.is_empty());
    assert_eq!(b.borrow().deregistered, vec![0]);

    // Handle mở lại ⇒ token MỚI (1), không tái sử dụng token0.
    s.frame.current = 3;
    let _ = s
        .frame_reactor_poll_io(ReactorOp::Read, "C1", 8, 1000, span)
        .unwrap_err();
    assert_eq!(
        b.borrow().registered.last().unwrap().0,
        1,
        "token không tái sử dụng"
    );

    // Event token0 cũ ⇒ stale (sổ token chỉ còn token1), không wake ai. Có một reg sống
    // (C1/token1) nên idle gọi adapter.wait() và consume event stale này.
    b.borrow_mut().events.push_back(ReactorRawEvent {
        token: 0,
        readable: true,
        hup: false,
        error: false,
    });
    assert_eq!(s.frame_next(), None);
    assert!(
        s.frame
            .reactor
            .log
            .iter()
            .any(|l| l.starts_with("rstale:token:0")),
        "{:?}",
        s.frame.reactor.log
    );
}
