// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-293 (VD-274/VD-280)** — `select_recv*`/`select_send*` trên frame scheduler: cả hai
//! engine, byte-identical; fairness xoay vòng; timeout/closed; wait-set nhiều kênh.

use vis_core::frame::{FrameConfig, FrameOutput};
use vis_interp::frame::run_frame_for_test as run_i;
use vis_interp::rt::{frame_cancel_after_turns_for_test, frame_cancel_clear_triggers_for_test};
use vis_syntax::{lex, parse, Mode};
use vis_vm::vm_frame::run_frame_for_test as run_v;

fn tien_trinh(src: &str) -> (vis_hir::Hir, vis_hir::Symbols, vis_hir::TypeTable) {
    let p = parse(src, Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse: {:?}", p.diagnostics);
    let l = lex(src, Mode::Script);
    let h = vis_types::lower_with(&p.cst, &l, vis_types::LowerOptions::default());
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
    assert_eq!(i.stdout, v.stdout, "interp↔VM stdout lệch");
    assert_eq!(i.exit, v.exit, "interp↔VM exit lệch");
    assert_eq!(i.diagnostics, v.diagnostics, "interp↔VM diagnostics lệch");
    assert_eq!(i.snapshot, v.snapshot, "interp↔VM snapshot lệch");
    assert_eq!(i.log, v.log, "interp↔VM log lệch");
    (i, v)
}

fn ok(out: &FrameOutput, stdout: &str) {
    assert_eq!(
        out.exit, 0,
        "stdout={:?} diag={:?}",
        out.stdout, out.diagnostics
    );
    assert_eq!(out.stdout, stdout, "log={:?}", out.log);
    assert_eq!(
        out.snapshot,
        vis_core::frame::DiagSnapshot::default(),
        "{:?}",
        out.log
    );
}

// ---------------------------------------------------------------------------
// (1) recv — chọn kênh có giá trị, ghi index + value.
// ---------------------------------------------------------------------------

#[test]
fn recv_chon_kênh_dau_co_gia_tri() {
    let (i, v) = chay_hai(
        r#"fn main():
    let c1 = chan[int](1)
    let c2 = chan[int](1)
    let _ = c1.send(10)
    let r = select_recv([c1, c2])
    print(r.index)
    print(r.value)
"#,
    );
    for out in [&i, &v] {
        ok(out, "0\nSome(10)\n");
    }
}

#[test]
fn recv_kênh_thu_hai() {
    let (i, v) = chay_hai(
        r#"fn main():
    let c1 = chan[int](1)
    let c2 = chan[int](1)
    let _ = c2.send(20)
    let r = select_recv([c1, c2])
    print(r.index)
    print(r.value)
"#,
    );
    for out in [&i, &v] {
        ok(out, "1\nSome(20)\n");
    }
}

#[test]
fn recv_chon_dong_none() {
    let (i, v) = chay_hai(
        r#"fn main():
    let c = chan[int](1)
    c.close()
    let r = select_recv([c])
    print(r.index)
    print(r.value)
"#,
    );
    for out in [&i, &v] {
        ok(out, "0\nNone\n");
    }
}

#[test]
fn recv_giu_gia_tri_da_dem_truoc_close() {
    let (i, v) = chay_hai(
        r#"fn main():
    let c = chan[int](1)
    let _ = c.send(7)
    c.close()
    let r = select_recv([c])
    print(r.value)
"#,
    );
    for out in [&i, &v] {
        ok(out, "Some(7)\n");
    }
}

#[test]
fn recv_rỗng_la_vs0501() {
    let src = r#"fn main():
    let cs = []
    let r = select_recv(cs)
"#;
    let (i, v) = chay_hai(src);
    for out in [&i, &v] {
        assert_eq!(out.exit, 1);
        assert_eq!(
            out.diagnostics.first().map(|d| d.code.short),
            Some("VS0501")
        );
    }
}

// ---------------------------------------------------------------------------
// (2) timeout.
// ---------------------------------------------------------------------------

#[test]
fn recv_timeout_rong_tra_none() {
    let (i, v) = chay_hai(
        r#"fn main():
    let c = chan[int](1)
    print(select_recv_timeout([c], 20))
"#,
    );
    for out in [&i, &v] {
        ok(out, "None\n");
    }
}

#[test]
fn recv_timeout_ms0_tham_do() {
    let (i, v) = chay_hai(
        r#"fn main():
    let c = chan[int](1)
    let _ = c.send(3)
    match select_recv_timeout([c], 0):
        Some(r): print(r.index)
        None: print("none")
"#,
    );
    for out in [&i, &v] {
        ok(out, "0\n");
    }
}

#[test]
fn recv_timeout_ms_am_la_vs0501() {
    let src = r#"fn main():
    let c = chan[int](1)
    let r = select_recv_timeout([c], -1)
"#;
    let (i, v) = chay_hai(src);
    for out in [&i, &v] {
        assert_eq!(out.exit, 1);
        assert_eq!(
            out.diagnostics.first().map(|d| d.code.short),
            Some("VS0501")
        );
    }
}

#[test]
fn recv_timeout_nhan_sau_khi_cho() {
    let (i, v) = chay_hai(
        r#"async fn gui(c: Chan[int]) -> int:
    c.send(42)
    1

fn main():
    let c = chan[int](0)
    let t = spawn gui(c)
    match select_recv_timeout([c], 100):
        Some(r):
            print(r.index)
            print(r.value)
        None: print("none")
"#,
    );
    for out in [&i, &v] {
        ok(out, "0\nSome(42)\n");
    }
}

// ---------------------------------------------------------------------------
// (3) send.
// ---------------------------------------------------------------------------

#[test]
fn send_vao_kênh_co_receiver() {
    let (i, v) = chay_hai(
        r#"async fn nhan(c: Chan[int]) -> int:
    print(c.recv())
    1

fn main():
    let c = chan[int](0)
    let t = spawn nhan(c)
    let i = select_send([c], 9)
    print(i)
"#,
    );
    for out in [&i, &v] {
        ok(out, "Some(9)\nSome(0)\n");
    }
}

#[test]
fn send_vao_kênh_co_dem() {
    let (i, v) = chay_hai(
        r#"fn main():
    let c = chan[int](1)
    print(select_send([c], 5))
    print(c.recv())
"#,
    );
    for out in [&i, &v] {
        ok(out, "Some(0)\nSome(5)\n");
    }
}

#[test]
fn send_tat_ca_dong_none() {
    let (i, v) = chay_hai(
        r#"fn main():
    let c1 = chan[int](1)
    let c2 = chan[int](1)
    c1.close()
    c2.close()
    print(select_send([c1, c2], 1))
"#,
    );
    for out in [&i, &v] {
        ok(out, "None\n");
    }
}

#[test]
fn send_timeout_khong_receiver_none() {
    let (i, v) = chay_hai(
        r#"fn main():
    let c = chan[int](0)
    print(select_send_timeout([c], 1, 20))
"#,
    );
    for out in [&i, &v] {
        ok(out, "None\n");
    }
}

#[test]
fn send_timeout_ms_am_la_vs0501() {
    let src = r#"fn main():
    let c = chan[int](1)
    let r = select_send_timeout([c], 1, -1)
"#;
    let (i, v) = chay_hai(src);
    for out in [&i, &v] {
        assert_eq!(out.exit, 1);
        assert_eq!(
            out.diagnostics.first().map(|d| d.code.short),
            Some("VS0501")
        );
    }
}

#[test]
fn send_chon_kênh_thu_hai_co_receiver() {
    let (i, v) = chay_hai(
        r#"async fn nhan(c: Chan[int]) -> int:
    print(c.recv())
    1

fn main():
    let c1 = chan[int](0)
    let c2 = chan[int](0)
    let t = spawn nhan(c2)
    let i = select_send([c1, c2], 7)
    print(i)
"#,
    );
    for out in [&i, &v] {
        ok(out, "Some(7)\nSome(1)\n");
        assert!(
            !out.log.iter().any(|l| l.contains("dup-ready")),
            "{:?}",
            out.log
        );
    }
}

#[test]
fn recv_timeout_dong_tra_record_none() {
    let (i, v) = chay_hai(
        r#"fn main():
    let c = chan[int](1)
    c.close()
    match select_recv_timeout([c], 10):
        Some(r): print(r.value)
        None: print("none")
"#,
    );
    for out in [&i, &v] {
        ok(out, "None\n");
    }
}

#[test]
fn select_don_waiters_nhanh_thua() {
    let (i, v) = chay_hai(
        r#"async fn gui(c: Chan[int]) -> int:
    c.send(11)
    1

fn main():
    let c1 = chan[int](0)
    let c2 = chan[int](0)
    let t = spawn gui(c1)
    let r = select_recv([c1, c2])
    print(r.value)
    let c3 = chan[int](1)
    let _ = c3.send(22)
    let s = select_recv([c2, c3])
    print(s.value)
"#,
    );
    for out in [&i, &v] {
        ok(out, "Some(11)\nSome(22)\n");
        assert!(
            out.log.iter().any(|l| l.starts_with("selclaim:")),
            "{:?}",
            out.log
        );
        assert!(
            !out.log.iter().any(|l| l.contains("dup-ready")),
            "{:?}",
            out.log
        );
        assert!(
            !out.log.iter().any(|l| l.starts_with("chanmiss:")),
            "{:?}",
            out.log
        );
    }
}

// ---------------------------------------------------------------------------
// (4) fairness — xoay vòng giữa các lần gọi.
// ---------------------------------------------------------------------------

#[test]
fn fairness_xoay_vong_hai_lan() {
    let (i, v) = chay_hai(
        r#"fn main():
    let c1 = chan[int](1)
    let c2 = chan[int](1)
    let _ = c1.send(10)
    let _ = c2.send(20)
    let a = select_recv([c1, c2])
    let b = select_recv([c1, c2])
    print(a.index)
    print(a.value)
    print(b.index)
    print(b.value)
"#,
    );
    for out in [&i, &v] {
        ok(out, "0\nSome(10)\n1\nSome(20)\n");
    }
}

#[test]
fn fairness_ba_kenh_xoay_vong() {
    let (i, v) = chay_hai(
        r#"fn main():
    let c1 = chan[int](1)
    let c2 = chan[int](1)
    let c3 = chan[int](1)
    let _ = c1.send(1)
    let _ = c2.send(2)
    let _ = c3.send(3)
    let a = select_recv([c1, c2, c3])
    let b = select_recv([c1, c2, c3])
    let c = select_recv([c1, c2, c3])
    print(a.index + b.index + c.index)
"#,
    );
    for out in [&i, &v] {
        ok(out, "3\n");
    }
}

// ---------------------------------------------------------------------------
// (5) blocking/đóng/cancel.
// ---------------------------------------------------------------------------

#[test]
fn recv_tat_ca_treo_vs0505() {
    let src = r#"fn main():
    let c = chan[int](0)
    let r = select_recv([c])
"#;
    let (i, v) = chay_hai(src);
    for out in [&i, &v] {
        assert_eq!(out.exit, 1);
        assert_eq!(
            out.diagnostics.first().map(|d| d.code.short),
            Some("VS0505")
        );
    }
}

#[test]
fn send_tat_ca_treo_vs0505() {
    let src = r#"fn main():
    let c = chan[int](0)
    let r = select_send([c], 1)
"#;
    let (i, v) = chay_hai(src);
    for out in [&i, &v] {
        assert_eq!(out.exit, 1);
        assert_eq!(
            out.diagnostics.first().map(|d| d.code.short),
            Some("VS0505")
        );
    }
}

#[test]
fn recv_cancel_khong_in_post() {
    let src = r#"async fn cho(c: Chan[int]) -> int:
    print("pre")
    let r = select_recv([c])
    print("post")
    1

fn main():
    let c = chan[int](0)
    let t = spawn cho(c)
    print("main")
"#;
    let (hir, syms, tys) = tien_trinh(src);
    frame_cancel_clear_triggers_for_test();
    frame_cancel_after_turns_for_test(3, 1);
    let i = run_i(src, &hir, &syms, &tys, &[], FrameConfig::default());
    frame_cancel_clear_triggers_for_test();
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    frame_cancel_after_turns_for_test(3, 1);
    let v = run_v(&m, src, &[], FrameConfig::default());
    frame_cancel_clear_triggers_for_test();
    for out in [&i, &v] {
        assert_eq!(out.exit, 0, "{:?}", out.diagnostics);
        assert!(!out.stdout.contains("post"), "{:?}", out.stdout);
        assert!(out.log.iter().any(|l| l == "cancel:1"), "{:?}", out.log);
        assert_eq!(
            out.snapshot,
            vis_core::frame::DiagSnapshot::default(),
            "{:?}",
            out.log
        );
    }
    assert_eq!(i.stdout, v.stdout);
}

#[test]
fn send_sau_close_none_khong_treo() {
    let (i, v) = chay_hai(
        r#"fn main():
    let c = chan[int](1)
    c.close()
    print(select_send_timeout([c], 1, 10))
"#,
    );
    for out in [&i, &v] {
        ok(out, "None\n");
    }
}

#[test]
fn send_timeout_cho_receiver_roi_thanh_cong() {
    let (i, v) = chay_hai(
        r#"async fn nhan(c: Chan[int]) -> int:
    print(c.recv())
    1

fn main():
    let c = chan[int](0)
    let t = spawn nhan(c)
    print(select_send_timeout([c], 9, 100))
"#,
    );
    for out in [&i, &v] {
        ok(out, "Some(9)\nSome(0)\n");
    }
}

#[test]
fn select_sai_kieu_bi_tyck_chan() {
    let src = r#"fn main():
    let r = select_recv([1])
"#;
    let p = parse(src, Mode::Script);
    let l = lex(src, Mode::Script);
    let h = vis_types::lower_with(&p.cst, &l, vis_types::LowerOptions::default());
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(
        t.diags.iter().any(|d| d.code.short == "VS0412"),
        "phải báo VS0412: {:?}",
        t.diags
    );
}
