// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-268 / VD-252 — LÁT 2**: reactor giả (read/accept/timer) nối vào **đường frame
//! test thật** của cả interp lẫn VM. Cùng một trace ⇒ stdout / chẩn đoán / log /
//! snapshot **byte-identical** giữa hai engine. Runtime mặc định (`vis run`) không bị chạm.
use vis_interp::frame::{run_frame_for_test as run_i, FrameConfig, FrameOutput};
use vis_interp::rt::{DiagSnapshot, ReactorEvent, ReactorOutcome};
use vis_interp::Value;
use vis_syntax::{lex, parse, Mode};
use vis_vm::vm_frame::{run_frame_for_test as run_v, task_requires_capture};

fn tien_trinh(src: &str) -> (vis_types::Hir, vis_types::Symbols, vis_types::TypeTable) {
    let p = parse(src, Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse: {:?}", p.diagnostics);
    let l = lex(src, Mode::Script);
    // `std.net` là module opt-in (LowerOptions::std_mods) — khớp CLI/test net thật.
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

fn cfg(reactors: Vec<ReactorEvent>, real_wait: bool) -> FrameConfig {
    FrameConfig {
        reactors,
        timer_real_wait: real_wait,
        ..Default::default()
    }
}

fn ok(s: &str) -> Value {
    Value::ok(Value::Str(s.into()))
}
fn ready(seq: u64, key: u64, gen: u64, s: &str) -> ReactorEvent {
    ReactorEvent {
        seq,
        key,
        gen,
        outcome: ReactorOutcome::Ready(ok(s)),
        deadline_ms: None,
    }
}
fn with_deadline(mut e: ReactorEvent, d: u64) -> ReactorEvent {
    e.deadline_ms = Some(d);
    e
}

/// Chạy cả hai engine trên cùng source + trace rồi so sánh byte.
fn chay(src: &str, c: FrameConfig) -> (FrameOutput, FrameOutput) {
    let (hir, syms, tys) = tien_trinh(src);
    let i = run_i(src, &hir, &syms, &tys, &[], c.clone());
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    let v = run_v(&m, src, &[], c);
    assert_eq!(i.stdout, v.stdout, "interp↔VM stdout lệch");
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
        assert_eq!((a.line, a.col), (b.line, b.col), "vị trí chẩn đoán lệch");
    }
    assert_eq!(i.log, v.log, "trace reactor/scheduler lệch");
    assert_eq!(i.snapshot, v.snapshot, "snapshot ownership lệch");
    assert_eq!(i.exit, v.exit, "exit lệch");
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

/// A treo ở `net.read` (chưa sẵn) ⇒ B chạy ⇒ event fire ⇒ A resume đúng chỗ,
/// không đọc lại từ đầu, không side-effect lặp.
const R1: &str = r#"use std.net

async fn reader() -> int:
    print("R:pre")
    let r = net.read("C1", 16, 1000)
    print(r)
    99

async fn other() -> int:
    print("O:run")
    5

fn main():
    let a = spawn reader()
    let b = spawn other()
    print(await a)
    print(await b)
"#;

#[test]
fn r1_read_pending_other_runs_then_resume() {
    let (i, _) = chay(R1, cfg(vec![ready(0, 0, 0, "DATA")], false));
    assert_eq!(i.stdout, "R:pre\nO:run\nOk(DATA)\n99\n5\n", "{:?}", i.log);
    assert!(i.log.iter().any(|l| l.starts_with("reg:0:C1:Read")));
    assert!(i.log.contains(&"rfire:0:0:0".to_string()));
    assert!(i.log.contains(&"wake:1".to_string()));
    assert_eq!(i.stdout.matches("R:pre").count(), 1, "không re-run");
    assert_eq!(i.snapshot, baseline(), "reactor về baseline sau drop");
}

/// `net.accept` cũng đi reactor giả (handshake server tối giản).
const R2: &str = r#"use std.net

async fn server() -> int:
    print("S:pre")
    let c = net.accept("L1")
    print(c)
    1

async fn bg() -> int:
    print("BG")
    2

fn main():
    let a = spawn server()
    let b = spawn bg()
    print(await a)
    print(await b)
"#;

#[test]
fn r2_accept_pending_and_resume_parity() {
    let ev = ReactorEvent {
        seq: 0,
        key: 0,
        gen: 0,
        outcome: ReactorOutcome::Ready(ok("C9")),
        deadline_ms: None,
    };
    let (i, _) = chay(R2, cfg(vec![ev], false));
    assert_eq!(i.stdout, "S:pre\nBG\nOk(C9)\n1\n2\n", "{:?}", i.log);
    assert!(i.log.iter().any(|l| l.starts_with("reg:0:L1:Accept")));
    assert_eq!(i.snapshot, baseline());
}

#[test]
fn r3_duplicate_and_stale_wake_do_not_rerun() {
    // Cùng key/gen fire lần 2 ⇒ `rdup`; gen lệch ⇒ `rstale`. Không wake lại, không re-run.
    let trace = vec![
        ready(0, 0, 0, "DATA"),
        ready(1, 0, 0, "AGAIN"),  // duplicate
        ready(2, 0, 99, "STALE"), // stale
    ];
    let (i, _) = chay(R1, cfg(trace, false));
    assert_eq!(i.stdout, "R:pre\nO:run\nOk(DATA)\n99\n5\n");
    // Sau khi kết quả được tiêu thụ, đăng ký đã gỡ ⇒ event lặp/gen lệch đều stale,
    // không đánh thức lại (xem thêm ca trực tiếp `r3b` cho nhánh `rdup`).
    assert!(
        i.log.iter().any(|l| l.starts_with("rstale:0")),
        "{:?}",
        i.log
    );
    assert_eq!(i.log.iter().filter(|l| **l == "wake:1").count(), 1);
}

/// Duplicate wake khi đăng ký vẫn còn sống (chưa tiêu thụ kết quả) ⇒ log `rdup`,
/// chỉ wake một lần — kiểm trực tiếp trên `Scheduler` (cả hai engine dùng chung).
#[test]
fn r3b_duplicate_fire_direct() {
    use vis_interp::rt::{Scheduler, Wait, Waiter};
    let mut s = Scheduler::new();
    s.frame_enable_for_test(vec![], vec![]);
    s.frame.current = 1;
    let _pending = s
        .frame_reactor_poll(
            vis_interp::rt::ReactorOp::Read,
            "C1",
            1000,
            vis_syntax::Span::new(0, 0),
        )
        .unwrap_err();
    s.frame_register(Waiter {
        frame: 1,
        wait: Wait::Reactor { key: 0, gen: 0 },
        gen: 0,
    });
    let ev = ReactorEvent {
        seq: 0,
        key: 0,
        gen: 0,
        outcome: ReactorOutcome::Ready(ok("X")),
        deadline_ms: None,
    };
    s.frame_reactor_fire(ev.clone());
    s.frame_reactor_fire(ev);
    assert!(
        s.frame.reactor.log.contains(&"rdup:0".to_string()),
        "{:?}",
        s.frame.reactor.log
    );
    assert_eq!(
        s.frame.log.iter().filter(|l| **l == "wake:1").count(),
        1,
        "{:?}",
        s.frame.log
    );
}

#[test]
fn r4_spurious_wake_reregisters_without_spin() {
    // Thức giả (gen0) ⇒ re-arm gen1, retry gặp WouldBlock ⇒ đăng ký lại gen1,
    // event thật (gen1) ⇒ thành công. gen lệch của event thật ở gen0 bị bỏ.
    let trace = vec![
        ReactorEvent {
            seq: 0,
            key: 0,
            gen: 0,
            outcome: ReactorOutcome::Spurious,
            deadline_ms: None,
        },
        ready(1, 0, 0, "WRONG"), // stale sau re-arm
        ready(2, 0, 1, "REAL"),
    ];
    let (i, _) = chay(R1, cfg(trace, false));
    assert_eq!(i.stdout, "R:pre\nO:run\nOk(REAL)\n99\n5\n", "{:?}", i.log);
    assert!(i.log.contains(&"rspurious:0:1".to_string()), "{:?}", i.log);
    assert!(
        i.log.iter().any(|l| l.starts_with("rstale:0:0")),
        "{:?}",
        i.log
    );
    assert!(
        i.log.iter().any(|l| l.starts_with("reg:0:C1:Read:1:")),
        "phải đăng ký lại với gen 1: {:?}",
        i.log
    );
    assert_eq!(i.snapshot, baseline());
}

#[test]
fn r5_close_while_waiting_invalidates_old_events() {
    let trace = vec![
        ReactorEvent {
            seq: 0,
            key: 0,
            gen: 0,
            outcome: ReactorOutcome::Close,
            deadline_ms: None,
        },
        ready(1, 0, 0, "AFTER-CLOSE"), // stale: đăng ký đã bị hủy
    ];
    let (i, _) = chay(R1, cfg(trace, false));
    assert!(
        i.stdout.contains("Err("),
        "close ⇒ Result.Err: {:?}",
        i.stdout
    );
    assert!(!i.stdout.contains("AFTER-CLOSE"));
    assert!(i.log.contains(&"rclose:0".to_string()), "{:?}", i.log);
    assert!(
        i.log.iter().any(|l| l.starts_with("rstale:0")),
        "{:?}",
        i.log
    );
    assert_eq!(i.snapshot, baseline());
}

#[test]
fn r6_eof_removes_waiter_exactly_once() {
    let ev = ReactorEvent {
        seq: 0,
        key: 0,
        gen: 0,
        outcome: ReactorOutcome::Eof,
        deadline_ms: None,
    };
    let (i, _) = chay(R1, cfg(vec![ev, ready(1, 0, 0, "X")], false));
    // EOF ⇒ `""` ⇒ `Ok()` (chuỗi rỗng) — print in ra dòng trống.
    assert_eq!(i.stdout, "R:pre\nO:run\nOk()\n99\n5\n", "{:?}", i.stdout);
    assert!(i.log.contains(&"eof:0".to_string()), "{:?}", i.log);
    assert_eq!(i.snapshot, baseline());
}

/// Tay "C1" được mở lại sau khi đóng: đăng ký mới phải có `key` mới, event cũ (key0)
/// không được đánh thức nhầm task mới.
const R7: &str = r#"use std.net

async fn reuse() -> int:
    let a = net.read("C1", 8, 1000)
    print(a)
    let b = net.read("C1", 8, 1000)
    print(b)
    0

fn main():
    let t = spawn reuse()
    print(await t)
"#;

#[test]
fn r7_reused_handle_does_not_wake_wrong_task() {
    let trace = vec![
        ReactorEvent {
            seq: 0,
            key: 0,
            gen: 0,
            outcome: ReactorOutcome::Close,
            deadline_ms: None,
        },
        ready(1, 0, 0, "OLD"), // stale cho đăng ký mới key1
        ready(2, 1, 0, "NEW"),
    ];
    let (i, _) = chay(R7, cfg(trace, false));
    assert!(!i.stdout.contains("OLD"), "{:?}", i.stdout);
    assert!(i.stdout.contains("Ok(NEW)"), "{:?}", i.stdout);
    assert!(
        i.log.iter().any(|l| l.starts_with("reg:1:C1:Read")),
        "{:?}",
        i.log
    );
    assert_eq!(i.snapshot, baseline());
}

/// Nhiều waiter cùng lúc: fire theo `seq` recorded ⇒ wake FIFO, không hash/random.
const R8: &str = r#"use std.net

async fn w1() -> int:
    let a = net.read("A", 8, 1000)
    print(a)
    1

async fn w2() -> int:
    let b = net.read("B", 8, 1000)
    print(b)
    2

fn main():
    let x = spawn w1()
    let y = spawn w2()
    print(await x)
    print(await y)
"#;

#[test]
fn r8_multiple_waiters_fifo_trace_order() {
    let trace = vec![ready(0, 1, 0, "BB"), ready(1, 0, 0, "AA")];
    let (i, _) = chay(R8, cfg(trace, false));
    assert_eq!(i.stdout, "Ok(BB)\nOk(AA)\n1\n2\n", "{:?}", i.stdout);
    // key0 = w1 (A), key1 = w2 (B) — event BB fire trước nhưng w2 xong trước w1.
    assert!(i.log.iter().any(|l| l.starts_with("reg:0:A:Read")));
    assert!(i.log.iter().any(|l| l.starts_with("reg:1:B:Read")));
    assert_eq!(i.snapshot, baseline());
}

// ---------------------------------------------------------------------------
// Explicit rejection (VD-252 §1 / ma trận coverage): cấu trúc KHÔNG được phủ phải
// từ chối rõ, không fallback sang đường đồng bộ, không re-run, checker không thu hẹp.
// ---------------------------------------------------------------------------

// **VD-255 / ST-274**: `&&`/`||` và hằng cấp tệp giờ ĐÃ được phủ ở đường frame interp
// (xem `lat255_coverage.rs`). Hai ca dưới đây chuyển từ "từ chối rõ" sang "chạy thật",
// vẫn so sánh byte giữa hai engine và với runtime mặc định.

#[test]
fn r9_and_or_short_circuit_supported() {
    // Toán hạng bị bỏ qua (`side()`) KHÔNG được chạy — side-effect xảy ra đúng một lần.
    let src = "fn side() -> bool:\n    print(\"SIDE\")\n    true\n\nfn f(a: bool) -> bool:\n    a and side()\n\nfn main():\n    print(f(false))\n";
    let (i, _) = chay(src, cfg(vec![], false));
    assert_eq!(i.exit, 0, "{:?}", i.diagnostics);
    assert_eq!(
        i.stdout, "false\n",
        "vế phải không được eval: {:?}",
        i.stdout
    );
    let (hir, syms, tys) = tien_trinh(src);
    let def = vis_interp::run(src, &hir, &syms, &tys, &[]);
    assert_eq!(def.stdout, "false\n");
}

#[test]
fn r10_file_const_supported() {
    let src = "const K: int = 7\nfn main():\n    print(K)\n";
    let (i, _) = chay(src, cfg(vec![], false));
    assert_eq!(i.exit, 0, "{:?}", i.diagnostics);
    assert_eq!(i.stdout, "7\n");
    let (hir, syms, tys) = tien_trinh(src);
    let def = vis_interp::run(src, &hir, &syms, &tys, &[]);
    assert_eq!(def.stdout, "7\n");
}

#[test]
fn r11_captured_task_rejected_clearly() {
    use vis_bytecode::{Const, Func, Instr, Module, Span, HASH_LEN, VERSION};
    // Module tối thiểu: main tạo task từ hàm 1; hàm 1 dùng LoadCaptured(0) ⇒ cần capture.
    let sp = |s: u32| Span {
        start: s,
        end: s + 1,
    };
    let m = Module {
        version: VERSION,
        source_hash: [0u8; HASH_LEN],
        consts: vec![
            Const {
                kind: 0,
                text: "1".into(),
            },
            Const {
                kind: 5,
                text: "\"$tac_vu\"".into(),
            },
        ],
        funcs: vec![
            Func {
                name: "main".into(),
                arity: 0,
                nparams: 0,
                nlocals: 1,
                max_stack: 4,
                code: vec![Instr::Const(0), Instr::CallBuiltin(1, 1), Instr::Halt],
                spans: vec![sp(0), sp(1), sp(2)],
            },
            Func {
                name: "capturing".into(),
                arity: 0,
                nparams: 0,
                nlocals: 1,
                max_stack: 4,
                code: vec![Instr::LoadCaptured(0), Instr::Return],
                spans: vec![sp(3), sp(4)],
            },
        ],
        entry: 0,
    };
    assert!(
        task_requires_capture(&m, 1),
        "guard phải nhận diện LoadCaptured"
    );
    assert!(!task_requires_capture(&m, 0));
    let out = run_v(&m, "<crafted>", &[], cfg(vec![], false));
    assert_eq!(out.exit, 1, "phải từ chối: {:?}", out.diagnostics);
    assert!(
        out.diagnostics[0].message.contains("capture"),
        "{:?}",
        out.diagnostics
    );
}

// ---------------------------------------------------------------------------
// B1 — 20×200 ms loopback, chạy QUA ĐƯỜNG TEST. Delay 200 ms của server được mô
// phỏng bằng deadline monotonic trên trace (20 timer đồng thời) — KHÔNG phải mạng
// thật, KHÔNG sleep-poll. Mục tiêu ~0.3 s (số đo, không phải ngưỡng CI).
// ---------------------------------------------------------------------------

fn b1_src() -> String {
    let mut s = String::from("use std.net\n\n");
    for i in 0..20 {
        s.push_str(&format!(
            "async fn r{i}() -> int:\n    let x = net.read(\"C{i}\", 64, 1000)\n    0\n\n"
        ));
    }
    s.push_str("fn main():\n");
    for i in 0..20 {
        s.push_str(&format!("    let a{i} = spawn r{i}()\n"));
    }
    s.push_str("    print(\"MARK\")\n");
    s
}

#[test]
fn b1_overlap_via_monotonic_timer() {
    let src = b1_src();
    let (hir, syms, tys) = tien_trinh(&src);
    let m = vis_bytecode::compile(&hir, &syms, &src).expect("compile");

    // Startup: chương trình không có reactor wait — đo chi phí dựng engine.
    let t0 = std::time::Instant::now();
    let warm = run_i(
        "fn main():\n    print(\"MARK\")\n",
        &tien_trinh("fn main():\n    print(\"MARK\")\n").0,
        &tien_trinh("fn main():\n    print(\"MARK\")\n").1,
        &tien_trinh("fn main():\n    print(\"MARK\")\n").2,
        &[],
        cfg(vec![], false),
    );
    let startup = t0.elapsed();
    assert_eq!(warm.stdout, "MARK\n");

    for run in 1..=3 {
        let trace: Vec<ReactorEvent> = (0..20)
            .map(|k| with_deadline(ready(k, k, 0, "x"), 200))
            .collect();
        let t = std::time::Instant::now();
        let oi = run_i(&src, &hir, &syms, &tys, &[], cfg(trace.clone(), true));
        let ti = t.elapsed();
        let t = std::time::Instant::now();
        let ov = run_v(&m, &src, &[], cfg(trace, true));
        let tv = t.elapsed();
        assert_eq!(oi.stdout, ov.stdout);
        assert_eq!(oi.stdout, "MARK\n");
        assert_eq!(oi.snapshot, baseline());
        eprintln!(
            "B1 run {run}: startup={:?} interp_workload={:?} vm_workload={:?}",
            startup, ti, tv
        );
        // 20 timer đồng thời ⇒ tổng ~200 ms (cộng startup), không phải 20×200 ms = 4 s.
        assert!(
            ti < std::time::Duration::from_millis(1_200),
            "interp: {:?}",
            ti
        );
        assert!(tv < std::time::Duration::from_millis(1_200), "VM: {:?}", tv);
    }
}
