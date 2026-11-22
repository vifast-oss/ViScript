// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-1d bổ sung VS3-2a** — wait-set blocking cho `chan(n≥1)` trên đường frame.
//!
//! Trước 1d, frame path dùng `chay_mot` (hàng đợi task mặc định) nên `chan(1)` đầy/rỗng
//! trap `VS0505` dù còn task khác chạy được. Nay frame path dùng cùng wait-set như `chan(0)`
//! (đệm + sender/receiver FIFO), giữ **giá trị/ngữ nghĩa cap**, chỉ đổi cách chờ.
//! Runtime sync (`frame.enabled == false`) vẫn `chay_mot`/`VS0505` như cũ.

use std::process::Command;

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
            std_mods: vec!["net".into(), "str".into()],
            ..Default::default()
        },
    );
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    (h.hir, h.symbols, h.types)
}

fn chay_hai(
    src: &str,
    events: Vec<vis_interp::rt::FakeEvent>,
    cancels: &[(u64, u32)],
) -> (FrameOutput, FrameOutput) {
    let (hir, syms, tys) = tien_trinh(src);
    frame_cancel_clear_triggers_for_test();
    for (seq, id) in cancels {
        frame_cancel_on_seq_for_test(*seq, *id);
    }
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
    for (seq, id) in cancels {
        frame_cancel_on_seq_for_test(*seq, *id);
    }
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
    assert_eq!(i.exit, v.exit, "interp↔VM exit lệch");
    assert_eq!(i.log, v.log, "interp↔VM log lệch");
    assert_eq!(i.snapshot, v.snapshot, "interp↔VM snapshot lệch");
    (i, v)
}

fn trigger_event() -> vis_interp::rt::FakeEvent {
    vis_interp::rt::FakeEvent {
        seq: 0,
        key: 999,
        gen: 0,
        value: Value::Int(0),
    }
}

const CAP1_BLOCKING: &str = r#"async fn producer(c: Chan[int]) -> int:
    c.send(1)
    c.send(2)
    print("P:done")
    1

async fn consumer(c: Chan[int]) -> int:
    let a = c.recv()
    let b = c.recv()
    print(a)
    print(b)
    2

fn main():
    let c = chan[int](1)
    let p = spawn producer(c)
    let q = spawn consumer(c)
    print("main")
"#;

#[test]
fn cap1_blocking_send_recv_hai_engine() {
    let (i, v) = chay_hai(CAP1_BLOCKING, vec![], &[]);
    for out in [&i, &v] {
        assert_eq!(
            out.stdout, "main\nSome(1)\nSome(2)\nP:done\n",
            "{:?}",
            out.log
        );
        assert_eq!(out.exit, 0, "{:?}", out.diagnostics);
    }
}

const CAP2_BLOCKING: &str = r#"async fn producer(c: Chan[int]) -> int:
    c.send(1)
    c.send(2)
    c.send(3)
    1

async fn consumer(c: Chan[int]) -> int:
    print(c.recv())
    print(c.recv())
    print(c.recv())
    2

fn main():
    let c = chan[int](2)
    let p = spawn producer(c)
    let q = spawn consumer(c)
"#;

#[test]
fn cap2_blocking_send_recv_hai_engine() {
    let (i, v) = chay_hai(CAP2_BLOCKING, vec![], &[]);
    for out in [&i, &v] {
        assert_eq!(out.stdout, "Some(1)\nSome(2)\nSome(3)\n", "{:?}", out.log);
        assert_eq!(out.exit, 0, "{:?}", out.diagnostics);
    }
}

const CAP1_DEADLOCK_RECV: &str = r#"fn main():
    let c = chan[int](1)
    print(c.recv())
"#;

#[test]
fn cap1_recv_rong_khong_task_van_trap_vs0505() {
    let (i, v) = chay_hai(CAP1_DEADLOCK_RECV, vec![], &[]);
    for out in [&i, &v] {
        assert_ne!(out.exit, 0);
        assert_eq!(
            out.diagnostics.first().map(|d| d.code.short),
            Some("VS0505"),
            "{:?}",
            out.diagnostics
        );
        assert!(
            out.diagnostics[0].message.contains("kênh **rỗng**"),
            "{:?}",
            out.diagnostics
        );
    }
}

const CAP1_CLOSE_SENDER: &str = r#"async fn producer(c: Chan[int]) -> int:
    print("P:pre")
    c.send(1)
    c.send(2)
    print("P:post")
    1

async fn closer(c: Chan[int]) -> int:
    print("C:pre")
    c.close()
    print("C:post")
    2

fn main():
    let c = chan[int](1)
    let p = spawn producer(c)
    let q = spawn closer(c)
    print("main")
"#;

#[test]
fn cap1_close_danh_thuc_sender_dang_cho() {
    let (i, v) = chay_hai(CAP1_CLOSE_SENDER, vec![], &[]);
    for out in [&i, &v] {
        assert_eq!(out.stdout, "main\nP:pre\nC:pre\nC:post\n", "{:?}", out.log);
        assert_ne!(out.exit, 0);
        assert!(
            out.diagnostics
                .iter()
                .any(|d| d.code.short == "VS0504" || d.message.contains("VS0504")),
            "{:?}",
            out.diagnostics
        );
        assert!(!out.stdout.contains("P:post"));
    }
}

const CAP1_CANCEL_SENDER: &str = r#"async fn s(c: Chan[int]) -> int:
    print("S:pre")
    c.send(1)
    c.send(2)
    print("S:post")
    1

fn main():
    let c = chan[int](1)
    let t = spawn s(c)
    print("main")
"#;

#[test]
fn cap1_cancel_sender_dang_cho() {
    let (i, v) = chay_hai(CAP1_CANCEL_SENDER, vec![trigger_event()], &[(0, 1)]);
    for out in [&i, &v] {
        assert_eq!(out.stdout, "main\nS:pre\n");
        assert_eq!(out.exit, 0, "{:?}", out.diagnostics);
        assert!(
            out.log.contains(&"cancelchan:1".to_string()),
            "{:?}",
            out.log
        );
        assert!(!out.log.iter().any(|l| l == "wake:1"));
    }
}

// ---------------------------------------------------------------------------
// CLI: async cap1 tự chọn frame, không cần `co_chan_cap_khong_zero`.
// ---------------------------------------------------------------------------

#[test]
fn cli_async_cap1_chay_frame_hai_duong() {
    let d = std::env::temp_dir().join(format!("vis-chan-cap-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tmp");
    let p = d.join("cap1.vis");
    std::fs::write(
        &p,
        "# SPDX-License-Identifier: MIT OR Apache-2.0\n".to_string()
            + "async fn san(c: Chan[int]) -> int:\n    c.send(1)\n    0\n\nfn main():\n    let c = chan[int](1)\n    let t = spawn san(c)\n    print(c.recv())\n",
    )
    .expect("ghi");
    let mut kq = Vec::new();
    for vm in [false, true] {
        let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
        c.current_dir(&d).arg("run");
        if vm {
            c.arg("--vm").arg("--no-cache");
        }
        let o = c.arg(p.to_str().unwrap()).output().expect("vis run");
        kq.push((
            o.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&o.stdout).to_string(),
            String::from_utf8_lossy(&o.stderr).to_string(),
        ));
    }
    assert_eq!(kq[0].0, 0, "interp: {}", kq[0].2);
    assert_eq!(kq[0].1, "Some(1)\n");
    assert_eq!(kq[0], kq[1], "interp↔VM CLI lệch");
    let _ = std::fs::remove_dir_all(&d);
}
