// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-2b** — timeout kênh trên **đường frame test** của cả hai engine.
//!
//! Hợp đồng A0: `recv_timeout(ms) -> T?` (None = timeout **hoặc** đóng);
//! `send_timeout(ms, v) -> bool` (false = timeout **hoặc** đóng); `ms < 0` ⇒ `VS0501`.
//! Không đổi runtime mặc định: ngoài frame, timeout trả lỗi rõ (W1 1d sẽ flip async default).
//! Timer dùng reactor timer hiện có, `timer_real_wait=false` để test tất định/không ngủ.

use vis_core::frame::{FrameConfig, FrameOutput};
use vis_interp::frame::run_frame_for_test as run_i;
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

fn chay_hai(src: &str) -> (FrameOutput, FrameOutput) {
    let (hir, syms, tys) = tien_trinh(src);
    let i = run_i(src, &hir, &syms, &tys, &[], FrameConfig::default());
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    let v = run_v(&m, src, &[], FrameConfig::default());
    assert_eq!(i.stdout, v.stdout, "interp↔VM stdout lệch: {i:?} vs {v:?}");
    assert_eq!(i.exit, v.exit, "exit lệch: {i:?} vs {v:?}");
    assert_eq!(i.snapshot, v.snapshot, "snapshot lệch: {i:?} vs {v:?}");
    (i, v)
}

fn ok(out: &FrameOutput, stdout: &str) {
    assert_eq!(
        out.exit, 0,
        "stdout={:?} diag={:?}",
        out.stdout, out.diagnostics
    );
    assert_eq!(out.stdout, stdout);
}

#[test]
fn cap1_recv_timeout_rong_roi_co_gia_tri() {
    let src = r#"fn main():
    let c = chan[int](1)
    let x = c.recv_timeout(50)
    print(x)
    let _ = c.send(7)
    let y = c.recv_timeout(50)
    print(y)
"#;
    let (i, v) = chay_hai(src);
    for out in [&i, &v] {
        ok(out, "None\nSome(7)\n");
        assert_eq!(out.snapshot, vis_core::frame::DiagSnapshot::default());
        assert!(
            out.log.iter().any(|l| l.starts_with("chwtimer:")),
            "{:?}",
            out.log
        );
    }
}

#[test]
fn cap1_send_timeout_day_va_con_cho() {
    let src = r#"fn main():
    let c = chan[int](1)
    print(c.send_timeout(50, 1))
    print(c.send_timeout(50, 2))
"#;
    let (i, v) = chay_hai(src);
    for out in [&i, &v] {
        ok(out, "true\nfalse\n");
    }
}

#[test]
fn cap0_recv_timeout_ghep_sender_dang_cho() {
    let src = r#"async fn sender(c: Chan[int]) -> int:
    c.send(7)
    1

fn main():
    let c = chan[int](0)
    let t = spawn sender(c)
    let x = c.recv_timeout(100)
    print(x)
"#;
    let (i, v) = chay_hai(src);
    for out in [&i, &v] {
        ok(out, "Some(7)\n");
    }
}

#[test]
fn cap0_send_timeout_ghep_receiver_dang_cho() {
    let src = r#"async fn receiver(c: Chan[int]) -> int:
    let x = c.recv()
    print(x)
    1

fn main():
    let c = chan[int](0)
    let t = spawn receiver(c)
    print(c.send_timeout(100, 9))
"#;
    let (i, v) = chay_hai(src);
    for out in [&i, &v] {
        ok(out, "Some(9)\ntrue\n");
    }
}

#[test]
fn cap0_timeout_khong_co_doi_tac() {
    let src = r#"fn main():
    let c = chan[int](0)
    print(c.recv_timeout(50))
    print(c.send_timeout(50, 1))
"#;
    let (i, v) = chay_hai(src);
    for out in [&i, &v] {
        ok(out, "None\nfalse\n");
    }
}

#[test]
fn close_timeout_none_va_false() {
    let src = r#"fn main():
    let c = chan[int](1)
    c.close()
    print(c.recv_timeout(50))
    print(c.send_timeout(50, 1))
"#;
    let (i, v) = chay_hai(src);
    for out in [&i, &v] {
        ok(out, "None\nfalse\n");
    }
}

#[test]
fn ms_bang_khong_chi_thu_mot_lan() {
    let src = r#"fn main():
    let c = chan[int](1)
    print(c.recv_timeout(0))
    print(c.send_timeout(0, 1))
    print(c.send_timeout(0, 2))
"#;
    let (i, v) = chay_hai(src);
    for out in [&i, &v] {
        ok(out, "None\ntrue\nfalse\n");
        assert!(
            !out.log.iter().any(|l| l.starts_with("chwtimer:")),
            "{:?}",
            out.log
        );
    }
}

#[test]
fn ms_am_trap_vs0501_hai_engine() {
    for meth in ["recv_timeout(-1)", "send_timeout(-1, 1)"] {
        let src =
            format!("fn main():\n    let c = chan[int](1)\n    let x = c.{meth}\n    print(x)\n");
        let (i, v) = chay_hai(&src);
        for out in [&i, &v] {
            assert_ne!(out.exit, 0, "phải trap: {:?}", out.stdout);
            assert_eq!(
                out.diagnostics.first().map(|d| d.code.short),
                Some("VS0501"),
                "{:?}",
                out.diagnostics
            );
            assert!(out.stdout.is_empty());
        }
    }
}

/// Runtime mặc định (không frame) chưa hỗ trợ timeout ⇒ lỗi rõ, không treo/không im lặng.
#[test]
fn runtime_mac_dinh_bao_loi_ro() {
    let src = r#"fn main():
    let c = chan[int](1)
    print(c.recv_timeout(10))
"#;
    let (hir, syms, tys) = tien_trinh(src);
    let out = vis_interp::run(src, &hir, &syms, &tys, &[]);
    assert_ne!(out.exit, 0, "runtime mặc định phải lỗi rõ");
    assert_eq!(
        out.codes(),
        vec!["VS0501"],
        "diagnostics: {:?}",
        out.diagnostics
    );
    assert!(
        out.diagnostics
            .first()
            .map(|d| d.message.contains("frame test"))
            .unwrap_or(false),
        "{:?}",
        out.diagnostics
    );
}

#[test]
fn close_danh_thuc_recv_timeout_dang_cho() {
    let src = r#"async fn r(c: Chan[int]) -> int:
    let x = c.recv_timeout(1000)
    print(x)
    1

async fn closer(c: Chan[int]) -> int:
    c.close()
    1

fn main():
    let c = chan[int](0)
    let a = spawn r(c)
    let b = spawn closer(c)
"#;
    let (i, v) = chay_hai(src);
    for out in [&i, &v] {
        ok(out, "None\n");
    }
}

#[test]
fn cap1_send_timeout_cho_duoc_cho_trong() {
    let src = r#"async fn r(c: Chan[int]) -> int:
    let x = c.recv()
    print(x)
    1

fn main():
    let c = chan[int](1)
    let _ = c.send(1)
    let t = spawn r(c)
    print(c.send_timeout(100, 2))
    print(c.recv())
"#;
    let (i, v) = chay_hai(src);
    for out in [&i, &v] {
        ok(out, "Some(1)\ntrue\nSome(2)\n");
    }
}

#[test]
fn close_danh_thuc_send_timeout_dang_cho() {
    let src = r#"async fn s(c: Chan[int]) -> int:
    print(c.send_timeout(1000, 1))
    1

async fn closer(c: Chan[int]) -> int:
    c.close()
    1

fn main():
    let c = chan[int](0)
    let a = spawn s(c)
    let b = spawn closer(c)
"#;
    let (i, v) = chay_hai(src);
    for out in [&i, &v] {
        ok(out, "false\n");
    }
}

#[test]
fn ms_bang_khong_cap0_khong_dang_ky_timer() {
    let src = r#"fn main():
    let c = chan[int](0)
    print(c.recv_timeout(0))
    print(c.send_timeout(0, 1))
"#;
    let (i, v) = chay_hai(src);
    for out in [&i, &v] {
        ok(out, "None\nfalse\n");
        assert!(
            !out.log.iter().any(|l| l.starts_with("chwtimer:")),
            "{:?}",
            out.log
        );
    }
}

#[test]
fn ms_bang_khong_cap0_ghep_ngay_khi_doi_tac_da_cho() {
    let src = r#"async fn s(c: Chan[int]) -> int:
    c.send(7)
    1

async fn r(c: Chan[int]) -> int:
    print(c.recv_timeout(0))
    1

fn main():
    let c = chan[int](0)
    let a = spawn s(c)
    let b = spawn r(c)
"#;
    let (i, v) = chay_hai(src);
    for out in [&i, &v] {
        ok(out, "Some(7)\n");
        assert!(
            !out.log.iter().any(|l| l.starts_with("chwtimer:")),
            "{:?}",
            out.log
        );
    }
}

#[test]
fn ms_bang_khong_cap0_send_ghep_ngay_khi_receiver_da_cho() {
    let src = r#"async fn r(c: Chan[int]) -> int:
    let x = c.recv()
    print(x)
    1

async fn s(c: Chan[int]) -> int:
    print(c.send_timeout(0, 9))
    1

fn main():
    let c = chan[int](0)
    let a = spawn r(c)
    let b = spawn s(c)
"#;
    let (i, v) = chay_hai(src);
    for out in [&i, &v] {
        ok(out, "true\nSome(9)\n");
        assert!(
            !out.log.iter().any(|l| l.starts_with("chwtimer:")),
            "{:?}",
            out.log
        );
    }
}

#[test]
fn cap1_recv_timeout_cho_duoc_sender() {
    let src = r#"async fn s(c: Chan[int]) -> int:
    c.send(3)
    1

fn main():
    let c = chan[int](1)
    let t = spawn s(c)
    print(c.recv_timeout(100))
"#;
    let (i, v) = chay_hai(src);
    for out in [&i, &v] {
        ok(out, "Some(3)\n");
    }
}

/// Timer thật (`timer_real_wait=true`) không treo và trả None sau ~ms; không đo chính xác
/// (máy tải), chỉ khẳng định đường deadline thật chạy được.
#[test]
fn timer_real_wait_khong_treo() {
    let src = r#"fn main():
    let c = chan[int](1)
    print(c.recv_timeout(5))
"#;
    let (hir, syms, tys) = tien_trinh(src);
    let t0 = std::time::Instant::now();
    let i = run_i(
        src,
        &hir,
        &syms,
        &tys,
        &[],
        FrameConfig {
            timer_real_wait: true,
            ..Default::default()
        },
    );
    assert_eq!(i.stdout, "None\n");
    assert!(t0.elapsed() >= std::time::Duration::from_millis(1));
    assert_eq!(i.snapshot, vis_core::frame::DiagSnapshot::default());

    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    let v = run_v(
        &m,
        src,
        &[],
        FrameConfig {
            timer_real_wait: true,
            ..Default::default()
        },
    );
    assert_eq!(v.stdout, "None\n");
    assert_eq!(v.snapshot, vis_core::frame::DiagSnapshot::default());
}
