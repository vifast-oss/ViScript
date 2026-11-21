// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-2a** — wait-set + `chan(0)` rendezvous trên **đường frame test** của cả hai engine.
//!
//! Không API/CLI mới, không đổi runtime mặc định (`vis run` vẫn từ chối `chan(0)`), không
//! `select`/timeout (VS3-2b), không VERSION/bytecode. `chan(n≥1)` giữ nguyên ngữ nghĩa
//! run-to-completion + `VS0505` khi hết task (test hồi quy ở đây và `lat257_a_coverage::b3`).

use vis_interp::frame::{run_frame_for_test as run_i, FrameConfig, FrameOutput};
use vis_interp::rt::{frame_cancel_clear_triggers_for_test, frame_cancel_on_seq_for_test};
use vis_interp::Value;
use vis_syntax::{lex, parse, Mode};
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

fn dat_trigger(cancels: &[(u64, u32)]) {
    frame_cancel_clear_triggers_for_test();
    for (seq, id) in cancels {
        frame_cancel_on_seq_for_test(*seq, *id);
    }
}

/// Chạy cả hai engine trên cùng nguồn; so stdout/log/snapshot/exit. `events` dùng cho
/// trigger huỷ; không có adapter/reactor.
fn chay_hai(
    src: &str,
    events: Vec<vis_interp::rt::FakeEvent>,
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

fn kiem_sach(out: &FrameOutput) {
    assert_eq!(
        out.exit, 0,
        "stdout={:?} diag={:?}",
        out.stdout, out.diagnostics
    );
    assert_eq!(out.snapshot, baseline(), "{:?}", out.log);
}

// ---------------------------------------------------------------------------
// (1) Rendezvous cơ bản — receiver chờ trước, receiver chờ sau.
// ---------------------------------------------------------------------------

const P_MAIN_RECV: &str = r#"async fn sender(c: Chan[int]) -> int:
    print("S:pre")
    c.send(7)
    print("S:post")
    1

fn main():
    let c = chan[int](0)
    let t = spawn sender(c)
    let x = c.recv()
    print(x)
"#;

#[test]
fn chan0_main_recv_cho_sender() {
    let (i, v) = chay_hai(P_MAIN_RECV, vec![], &[]);
    for out in [&i, &v] {
        assert_eq!(out.stdout, "S:pre\nS:post\nSome(7)\n", "{:?}", out.log);
        kiem_sach(out);
        assert!(
            out.log.iter().any(|l| l.starts_with("wait:0:Chan")),
            "main phải chờ kênh: {:?}",
            out.log
        );
    }
}

const P_MAIN_SEND: &str = r#"async fn receiver(c: Chan[int]) -> int:
    print("R:pre")
    let x = c.recv()
    print(x)
    print("R:post")
    1

fn main():
    let c = chan[int](0)
    let t = spawn receiver(c)
    print("main")
    c.send(9)
    print("sent")
"#;

#[test]
fn chan0_main_send_cho_receiver() {
    let (i, v) = chay_hai(P_MAIN_SEND, vec![], &[]);
    for out in [&i, &v] {
        assert_eq!(
            out.stdout, "main\nR:pre\nSome(9)\nR:post\nsent\n",
            "{:?}",
            out.log
        );
        kiem_sach(out);
        assert!(
            out.log.iter().any(|l| l.starts_with("wait:0:Chan")),
            "main phải chờ kênh: {:?}",
            out.log
        );
    }
}

// ---------------------------------------------------------------------------
// (2) FIFO nhiều sender/receiver — không mất/không nhân đôi giá trị.
// ---------------------------------------------------------------------------

const P_NHIEU: &str = r#"async fn s(c: Chan[int], v: int) -> int:
    c.send(v)
    1

async fn r(c: Chan[int]) -> int:
    let x = c.recv()
    print(x)
    1

fn main():
    let c = chan[int](0)
    let a = spawn s(c, 1)
    let b = spawn s(c, 2)
    let x = spawn r(c)
    let y = spawn r(c)
    print("main")
"#;

#[test]
fn chan0_hai_sender_hai_receiver_fifo() {
    let (i, v) = chay_hai(P_NHIEU, vec![], &[]);
    for out in [&i, &v] {
        assert_eq!(out.stdout, "main\nSome(1)\nSome(2)\n", "{:?}", out.log);
        kiem_sach(out);
    }
}

// ---------------------------------------------------------------------------
// (3) Deadlock — chỉ sender hoặc chỉ receiver, không treo, VS0505.
// ---------------------------------------------------------------------------

const P_CHI_SENDER: &str = r#"async fn s(c: Chan[int]) -> int:
    print("S:pre")
    c.send(1)
    print("S:post")
    1

fn main():
    let c = chan[int](0)
    let t = spawn s(c)
    print("main")
"#;

#[test]
fn chan0_chi_sender_deadlock_vs0505() {
    let (i, v) = chay_hai(P_CHI_SENDER, vec![], &[]);
    for out in [&i, &v] {
        assert_eq!(out.stdout, "main\nS:pre\n");
        assert_ne!(out.exit, 0, "phải deadlock, không treo/exit 0");
        assert_eq!(
            out.diagnostics.first().map(|d| d.code.short),
            Some("VS0505"),
            "{:?}",
            out.diagnostics
        );
        assert!(!out.stdout.contains("S:post"));
    }
}

const P_CHI_RECEIVER: &str = r#"async fn r(c: Chan[int]) -> int:
    print("R:pre")
    let x = c.recv()
    print(x)
    1

fn main():
    let c = chan[int](0)
    let t = spawn r(c)
    print("main")
"#;

#[test]
fn chan0_chi_receiver_deadlock_vs0505() {
    let (i, v) = chay_hai(P_CHI_RECEIVER, vec![], &[]);
    for out in [&i, &v] {
        assert_eq!(out.stdout, "main\nR:pre\n");
        assert_ne!(out.exit, 0);
        assert_eq!(
            out.diagnostics.first().map(|d| d.code.short),
            Some("VS0505"),
            "{:?}",
            out.diagnostics
        );
    }
}

// ---------------------------------------------------------------------------
// (4) close đánh thức đúng waiter.
// ---------------------------------------------------------------------------

const P_CLOSE_RECV: &str = r#"async fn r(c: Chan[int]) -> int:
    print("R:pre")
    let x = c.recv()
    print(x)
    print("R:post")
    1

async fn closer(c: Chan[int]) -> int:
    print("C:pre")
    c.close()
    print("C:post")
    2

fn main():
    let c = chan[int](0)
    let a = spawn r(c)
    let b = spawn closer(c)
    print("main")
"#;

#[test]
fn chan0_close_danh_thuc_receiver_tra_none() {
    let (i, v) = chay_hai(P_CLOSE_RECV, vec![], &[]);
    for out in [&i, &v] {
        assert_eq!(
            out.stdout, "main\nR:pre\nC:pre\nC:post\nNone\nR:post\n",
            "{:?}",
            out.log
        );
        kiem_sach(out);
    }
}

const P_CLOSE_HAI_RECV: &str = r#"async fn r(c: Chan[int], tag: str) -> int:
    print(tag)
    let x = c.recv()
    print(x)
    1

async fn closer(c: Chan[int]) -> int:
    print("C:close")
    c.close()
    2

fn main():
    let c = chan[int](0)
    let a = spawn r(c, "A")
    let b = spawn r(c, "B")
    let z = spawn closer(c)
    print("main")
"#;

#[test]
fn chan0_close_danh_thuc_tat_ca_receiver() {
    let (i, v) = chay_hai(P_CLOSE_HAI_RECV, vec![], &[]);
    for out in [&i, &v] {
        assert_eq!(
            out.stdout.matches("None\n").count(),
            2,
            "cả hai receiver phải nhận None: {:?}",
            out.stdout
        );
        assert_eq!(out.stdout.matches("A\n").count(), 1);
        assert_eq!(out.stdout.matches("B\n").count(), 1);
        assert!(out.stdout.contains("C:close\n"));
        kiem_sach(out);
    }
}

const P_CLOSE_SEND: &str = r#"async fn s(c: Chan[int]) -> int:
    print("S:pre")
    c.send(1)
    print("S:post")
    1

async fn closer(c: Chan[int]) -> int:
    print("C:pre")
    c.close()
    print("C:post")
    2

fn main():
    let c = chan[int](0)
    let a = spawn s(c)
    let b = spawn closer(c)
    print("main")
"#;

#[test]
fn chan0_close_danh_thuc_sender_tra_vs0504() {
    let (i, v) = chay_hai(P_CLOSE_SEND, vec![], &[]);
    for out in [&i, &v] {
        assert_eq!(out.stdout, "main\nS:pre\nC:pre\nC:post\n");
        assert_ne!(out.exit, 0, "send bị close phải lỗi");
        assert!(
            out.diagnostics
                .iter()
                .any(|d| d.code.short == "VS0504" || d.message.contains("VS0504")),
            "phải là VS0504 (kể cả khi bọc VS0506): {:?}",
            out.diagnostics
        );
        assert!(!out.stdout.contains("S:post"));
    }
}

// ---------------------------------------------------------------------------
// (5) close idempotent + close trước khi task chạy.
// ---------------------------------------------------------------------------

const P_CLOSE_IDEMPOTENT: &str = r#"fn main():
    let c = chan[int](0)
    c.close()
    c.close()
    print(c.recv())
"#;

#[test]
fn chan0_close_idempotent_va_recv_none() {
    let (i, v) = chay_hai(P_CLOSE_IDEMPOTENT, vec![], &[]);
    for out in [&i, &v] {
        assert_eq!(out.stdout, "None\n", "{:?}", out.log);
        kiem_sach(out);
    }
}

// ---------------------------------------------------------------------------
// (6) cancellation gỡ sender/receiver khỏi wait-set đúng một lần.
// ---------------------------------------------------------------------------

fn trigger_event() -> vis_interp::rt::FakeEvent {
    vis_interp::rt::FakeEvent {
        seq: 0,
        key: 999,
        gen: 0,
        value: Value::Int(0),
    }
}

const P_CANCEL_SENDER: &str = r#"async fn s(c: Chan[int]) -> int:
    print("S:pre")
    c.send(1)
    print("S:post")
    1

fn main():
    let c = chan[int](0)
    let t = spawn s(c)
    print("main")
"#;

#[test]
fn chan0_cancel_sender_go_wait_set() {
    let (i, v) = chay_hai(P_CANCEL_SENDER, vec![trigger_event()], &[(0, 1)]);
    for out in [&i, &v] {
        assert_eq!(out.stdout, "main\nS:pre\n");
        kiem_sach(out);
        assert!(
            out.log.contains(&"cancelchan:1".to_string()),
            "phải gỡ wait-set kênh: {:?}",
            out.log
        );
        assert!(out.log.contains(&"cancel:1".to_string()));
        assert!(!out.stdout.contains("S:post"));
    }
}

const P_CANCEL_RECEIVER: &str = r#"async fn r(c: Chan[int]) -> int:
    print("R:pre")
    let x = c.recv()
    print(x)
    1

fn main():
    let c = chan[int](0)
    let t = spawn r(c)
    print("main")
"#;

#[test]
fn chan0_cancel_receiver_go_wait_set() {
    let (i, v) = chay_hai(P_CANCEL_RECEIVER, vec![trigger_event()], &[(0, 1)]);
    for out in [&i, &v] {
        assert_eq!(out.stdout, "main\nR:pre\n");
        kiem_sach(out);
        assert!(
            out.log.contains(&"cancelchan:1".to_string()),
            "phải gỡ wait-set kênh: {:?}",
            out.log
        );
    }
}

// ---------------------------------------------------------------------------
// (7) Runtime mặc định KHÔNG đổi: `chan(0)` vẫn bị từ chối khi frame tắt.
// ---------------------------------------------------------------------------

#[test]
fn chan0_runtime_mac_dinh_van_tu_choi() {
    let src = "fn main():\n    let c = chan[int](0)\n    let _ = c.send(1)\n";
    let (hir, syms, tys) = tien_trinh(src);
    let i = vis_interp::run(src, &hir, &syms, &tys, &[]);
    assert_ne!(i.exit, 0, "interp mặc định phải từ chối chan(0)");
    assert_eq!(
        i.diagnostics.first().map(|d| d.code.short),
        Some("VS0505"),
        "{:?}",
        i.diagnostics
    );
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    let mut vm = vis_vm::vm::Vm::new(&m, src);
    let v = vm.run(&[]);
    assert_ne!(v.exit, 0, "VM mặc định phải từ chối chan(0)");
    assert_eq!(
        v.diagnostics.first().map(|d| d.code.short),
        Some("VS0505"),
        "{:?}",
        v.diagnostics
    );
}

// ---------------------------------------------------------------------------
// (8) chan(n≥1) giữ nguyên semantics (không dùng wait-set rendezvous).
// ---------------------------------------------------------------------------

const P_CAP1: &str = r#"fn main():
    let c = chan[int](1)
    c.send(1)
    c.send(2)
    print(c.recv())
    print(c.recv())
"#;

#[test]
fn chan1_giu_nguyen_semantics_khong_wait_set() {
    let (i, v) = chay_hai(P_CAP1, vec![], &[]);
    for out in [&i, &v] {
        // cap 1 + gửi 2 trước khi nhận: send thứ hai chặn và không có task khác chạy
        // ⇒ deadlock VS0505 như trước VS3-2a (không âm thầm đổi thành wait-set cap>0).
        assert_ne!(out.exit, 0, "cap 1 đầy vẫn deadlock như cũ");
        assert_eq!(
            out.diagnostics.first().map(|d| d.code.short),
            Some("VS0505"),
            "{:?}",
            out.diagnostics
        );
        assert!(!out.log.iter().any(|l| l.starts_with("wait:0:Chan")));
    }
}
