// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-260 / frame path (lát 3)** — hồi quy các lỗi còn lại sau VD-259 (chỉ test/debug,
//! KHÔNG bật mặc định). Mỗi ca chạy **cả hai engine** khi áp dụng được.
//!
//! | Ca | Lỗi cũ |
//! |---|---|
//! | `timer_khong_bi_doi_*` | timer/reactor chỉ fire khi hàng `ready` cạn ⇒ đói sau CPU yield |
//! | `timer_that_*` | timer có thể fire sớm hơn deadline khi đường ngân sách kích hoạt |
//! | `event_sau_budget_*` | event stale ở đầu trace chặn event hợp lệ phía sau |
//! | `fake_event_stale_*` | cùng lỗi cho trace fake-source (không chỉ reactor) |
//! | `snapshot_frames_*` | `live_frames` đo sau khi đã dọn ⇒ luôn 0 (metric rỗng) |
//! | `frame_huy_bi_bo_*` | frame bị huỷ vẫn nằm trong map driver tới cuối chương trình |
//! | `quota_*` | yield quota không được làm mất ngân sách event (timer vẫn phải tới) |

use vis_core::frame::{FakeEvent, ReactorEvent, ReactorOutcome};
use vis_core::frame::{FrameConfig, FrameOutput};
use vis_core::reactor::ReactorOp;
use vis_core::value::{TrangThai, Wait};
use vis_core::Value;
use vis_interp::frame::run_frame_for_test as run_i;
use vis_interp::rt::{fee as ifee, Scheduler, Waiter, FRAME_READY_BUDGET};
use vis_syntax::{lex, parse, Mode, Span};
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

fn chay(src: &str, c: FrameConfig) -> (FrameOutput, FrameOutput) {
    let (hir, syms, tys) = tien_trinh(src);
    let i = run_i(src, &hir, &syms, &tys, &[], c.clone());
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    let v = run_v(&m, src, &[], c);
    (i, v)
}

/// Chạy cả hai engine với quota bật (yield CPU) và trả số yield của từng engine.
fn chay_co_quota(src: &str, c: FrameConfig, q: u64) -> (FrameOutput, u64, FrameOutput, u64) {
    let (hir, syms, tys) = tien_trinh(src);
    ifee::enable_for_test(40_000_000);
    ifee::frame_emit_enable_for_test();
    ifee::quota_enable_for_test(q);
    let i = run_i(src, &hir, &syms, &tys, &[], c.clone());
    let yi = ifee::quota_yields_for_test();
    ifee::disable_for_test();

    let (m, fee) = vis_bytecode::compile_with_fee(&hir, &syms, src).expect("compile_with_fee");
    frame_budget_enable_for_test(fee, 40_000_000, Some(q));
    let v = run_v(&m, src, &[], c);
    let yv = frame_quota_yields_for_test();
    frame_budget_disable_for_test();
    (i, yi, v, yv)
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

fn fake(seq: u64, key: u64, gen: u64, value: Value) -> FakeEvent {
    FakeEvent {
        seq,
        key,
        gen,
        value,
    }
}

fn timer(seq: u64, key: u64, deadline_ms: u64) -> ReactorEvent {
    ReactorEvent {
        seq,
        key,
        gen: 0,
        outcome: ReactorOutcome::Ready(Value::ok(Value::Str("A".into()))),
        deadline_ms: Some(deadline_ms),
    }
}

fn dang_ky_cho_reactor(s: &mut Scheduler, frame: u32, key: u64) {
    s.frame.current = frame;
    let _ = s.frame_reactor_poll(ReactorOp::Read, "C1", 1000, Span::new(0, 0));
    s.frame_register(Waiter {
        frame,
        wait: Wait::Reactor { key, gen: 0 },
        gen: 0,
    });
}

// ===========================================================================
// 1) Timer không bị đói sau hàng ready dài (CPU yield quota)
// ===========================================================================

fn nguon_cpu_io(n: u32) -> String {
    format!(
        r#"use std.net
async fn cpu(n: int) -> ():
    var i = 0
    while i < n:
        i = i + 1
    print("CPU_DONE")
async fn io() -> ():
    print("IO_START")
    let r = net.read("C1", 8, 1000)
    print("IO_DONE")
fn main():
    let a = spawn io()
    let b = spawn cpu({n})
    print(await a)
"#
    )
}

/// Task CPU luôn sẵn sàng (yield quota liên tục), task I/O chờ một timer. Bản cũ chỉ fire
/// timer khi `ready` cạn ⇒ `CPU_DONE` trước `IO_DONE`. Nay sau `FRAME_READY_BUDGET` lượt
/// ready, timer được giao ⇒ I/O hoàn tất trước khi CPU xong.
#[test]
fn timer_khong_bi_doi_sau_hang_ready() {
    let c = FrameConfig {
        reactors: vec![timer(0, 0, 5)],
        ..Default::default()
    };
    let (i, yi, v, yv) = chay_co_quota(&nguon_cpu_io(50_000), c, 500);
    for (name, o) in [("interp", &i), ("vm", &v)] {
        assert_eq!(o.exit, 0, "{name}: {:?}", o.diagnostics);
        let io = o.stdout.find("IO_DONE").expect("IO_DONE");
        let cpu = o.stdout.find("CPU_DONE").expect("CPU_DONE");
        assert!(
            io < cpu,
            "{name}: timer bị đói sau hàng ready: {:?}",
            o.stdout
        );
        assert_eq!(
            o.stdout.matches("IO_START").count(),
            1,
            "{name}: re-run I/O"
        );
        assert!(
            o.log.contains(&"timer:0:5".to_string()),
            "{name}: không thấy timer: {:?}",
            o.log
        );
        assert!(
            o.log.contains(&"rfire:0:0:0".to_string()),
            "{name}: không fire timer: {:?}",
            o.log
        );
    }
    assert!(yi > 0, "interp phải có yield quota để tái hiện đói timer");
    assert!(yv > 0, "VM phải có yield quota để tái hiện đói timer");
    assert_eq!(i.stdout, v.stdout, "stdout lệch giữa hai engine");
    assert_diag_parity(&i, &v);
}

/// **Quota guard**: Q=1 ⇒ turn nào cũng yield (turn CPU không bao giờ "dày"). Ngân sách
/// event phải tiến theo **turn**, không bị yield quota reset/che ⇒ timer vẫn tới trước
/// khi CPU xong. Ca Q=500 ở trên không phủ được trường hợp mỗi turn đều yield.
#[test]
fn quota_moi_turn_van_khong_lam_doi_timer() {
    let c = FrameConfig {
        reactors: vec![timer(0, 0, 5)],
        ..Default::default()
    };
    let (i, yi, v, yv) = chay_co_quota(&nguon_cpu_io(5_000), c, 1);
    for (name, o) in [("interp", &i), ("vm", &v)] {
        assert_eq!(o.exit, 0, "{name}: {:?}", o.diagnostics);
        let io = o.stdout.find("IO_DONE").expect("IO_DONE");
        let cpu = o.stdout.find("CPU_DONE").expect("CPU_DONE");
        assert!(
            io < cpu,
            "{name}: yield mỗi turn làm đói timer: {:?}",
            o.stdout
        );
        assert_eq!(
            o.stdout.matches("IO_START").count(),
            1,
            "{name}: re-run I/O"
        );
    }
    assert!(yi > 0, "interp: Q=1 phải yield");
    assert!(yv > 0, "VM: Q=1 phải yield");
    assert_diag_parity(&i, &v);
}

// ===========================================================================
// 2) Timer thật không fire sớm hơn deadline của chính nó
// ===========================================================================

/// Đường ngân sách ready phải ngủ tới deadline của timer **có đăng ký sống** trước khi
/// fire. Không có bước này, timer fire ngay sau 8 lượt ready dù deadline chưa tới.
#[test]
fn timer_that_khong_fire_som_truoc_deadline() {
    let mut s = Scheduler::default();
    s.frame_enable_for_test(vec![], vec![]);
    // **VD-270 (flake timing, 2026-11-21)**: mốc đo phải lấy **trước** khi nạp timer, vì
    // `epoch` của reactor có thể được ghim ngay khi nạp/8 lượt ready đầu; dưới tải, 8 lượt đó
    // có thể tốn ~10 ms nên đo từ sau chúng sẽ thấy "chỉ 29,3 ms" dù timer không hề fire sớm.
    // Tổng thời gian từ trước khi nạp tới lúc fire vẫn phải ≥ 40 ms − dung sai 10 ms.
    let t_bat_dau = std::time::Instant::now();
    s.frame_reactor_load_for_test(vec![timer(0, 0, 40)], true);
    dang_ky_cho_reactor(&mut s, 5, 0);
    // 8 lượt ready đầu chưa chạm ngân sách: timer chưa được fire.
    for _ in 0..FRAME_READY_BUDGET {
        s.frame.ready.push_back(99);
        assert_eq!(s.frame_next(), Some(99), "phải trả frame ready");
    }
    assert!(
        !s.frame.reactor.fired.contains_key(&0),
        "timer fire trước ngân sách"
    );
    s.frame.ready.push_back(99);
    assert_eq!(s.frame_next(), Some(99));
    let elapsed = t_bat_dau.elapsed();
    assert!(
        elapsed >= std::time::Duration::from_millis(30),
        "timer fire sớm: chỉ {:?}",
        elapsed
    );
    assert!(
        s.frame.reactor.fired.contains_key(&0),
        "timer không fire sau deadline"
    );
    assert!(
        s.frame.ready.contains(&5),
        "waiter của timer không được đánh thức"
    );
}

// ===========================================================================
// 3) Event stale ở đầu trace không chặn event hợp lệ phía sau
// ===========================================================================

/// Trace reactor: event stale (khóa đã cấp rồi bị gỡ) đứng trước timer hợp lệ. Nếu chỉ
/// fire head khi `ready` cạn, timer hợp lệ bị đói sau hàng CPU. Nay mỗi cửa sổ ngân sách
/// fire đúng head kế tiếp ⇒ stale mở đường ở cửa sổ 1, timer tới ở cửa sổ 2, **đúng `seq`**.
#[test]
fn event_sau_budget_giao_dung_seq_stale_roi_hop_le() {
    fn chay_mot_lan() -> (Vec<String>, bool, u32, u32) {
        let mut s = Scheduler::default();
        s.frame_enable_for_test(vec![], vec![]);
        s.frame_reactor_load_for_test(vec![timer(0, 0, 0), timer(1, 1, 0)], false);
        // Cấp `key=0` rồi gỡ (đăng ký cũ đã chết ⇒ event `key=0` là stale chắc chắn).
        s.frame.current = 1;
        let _ = s.frame_reactor_poll(ReactorOp::Read, "OLD", 1000, Span::new(0, 0));
        s.frame.reactor.regs.clear();
        // `key=1` là consumer của timer hợp lệ.
        dang_ky_cho_reactor(&mut s, 5, 1);
        let mut moc_stale = 0;
        let mut moc_hop_le = 0;
        for turn in 1..=64u32 {
            if s.frame.ready.is_empty() {
                s.frame.ready.push_back(99);
            }
            let _ = s.frame_next();
            if moc_stale == 0 && s.frame.reactor.log.iter().any(|l| l == "rfire:0:0:0") {
                moc_stale = turn;
            }
            if moc_hop_le == 0 && s.frame.reactor.log.iter().any(|l| l == "rfire:1:1:0") {
                moc_hop_le = turn;
                break;
            }
        }
        let woke = s.frame.ready.contains(&5);
        (s.frame.reactor.log, woke, moc_stale, moc_hop_le)
    }

    let (log1, woke, moc_stale, moc_hop_le) = chay_mot_lan();
    assert_eq!(
        moc_stale,
        FRAME_READY_BUDGET + 1,
        "event stale phải được fire đúng cửa sổ ngân sách đầu: {log1:?}"
    );
    assert_eq!(
        moc_hop_le,
        2 * FRAME_READY_BUDGET + 1,
        "timer hợp lệ phải tới ở cửa sổ kế tiếp: {log1:?}"
    );
    assert!(woke, "timer hợp lệ không đánh thức waiter: {log1:?}");
    let pos_stale = log1.iter().position(|l| l == "rfire:0:0:0").unwrap();
    let pos_hop_le = log1.iter().position(|l| l == "rfire:1:1:0").unwrap();
    assert!(pos_stale < pos_hop_le, "sai thứ tự seq: {log1:?}");
    assert!(
        log1.iter().any(|l| l == "rstale:0"),
        "stale phải được ghi log: {log1:?}"
    );

    // Tất định tuyệt đối: chạy lại cho cùng chuỗi log (không hash/thời gian).
    let (log2, woke2, _, _) = chay_mot_lan();
    assert_eq!(log1, log2, "trace reactor không tất định");
    assert!(woke2);
}

/// Cùng quy tắc cho trace fake-source: event stale (`gen` cũ hơn sau re-arm) ở đầu không
/// được chặn event hợp lệ phía sau khi một frame CPU luôn sẵn sàng.
#[test]
fn fake_event_stale_roi_hop_le_sau_hang_ready() {
    let mut s = Scheduler::default();
    s.frame_enable_for_test(
        vec![("src".into(), 1)],
        vec![fake(0, 1, 0, Value::Int(0)), fake(1, 1, 1, Value::Int(7))],
    );
    let t = s.tao(0, Vec::new(), "src".into(), Span::new(0, 0));
    // Re-arm: `external` của task lên gen 1 ⇒ event gen 0 thành stale chắc chắn.
    s.frame_rearm_for_test(1);
    let mut moc_stale = 0;
    let mut moc_hop_le = 0;
    for turn in 1..=64u32 {
        if s.frame.ready.is_empty() {
            s.frame.ready.push_back(99);
        }
        let _ = s.frame_next();
        if moc_stale == 0 && s.frame.log.iter().any(|l| l == "stale:1") {
            moc_stale = turn;
        }
        if moc_hop_le == 0 && t.trang_thai.get() == TrangThai::Xong {
            moc_hop_le = turn;
            break;
        }
    }
    assert_eq!(
        moc_stale,
        FRAME_READY_BUDGET + 1,
        "fake event stale phải fire ở cửa sổ đầu: {:?}",
        s.frame.log
    );
    assert_eq!(
        moc_hop_le,
        2 * FRAME_READY_BUDGET + 1,
        "fake event hợp lệ bị chặn sau stale: {:?}",
        s.frame.log
    );
    assert!(
        matches!(t.kq.borrow().as_ref(), Some(Ok(Value::Int(7)))),
        "kết quả event hợp lệ không được cache: {:?}",
        t.kq.borrow()
    );
    assert!(
        s.frame.log.iter().any(|l| l == "fire:1:1:1"),
        "{:?}",
        s.frame.log
    );
}

// ===========================================================================
// 4) Snapshot: `live_frames` phải đếm frame đang treo thật, không rỗng
// ===========================================================================

/// Deadlock I/O với `main` cũng treo ở `await`: hai frame (main + task) còn sống tại
/// thời điểm chụp ⇒ `frames == 2` là bằng chứng thật, không phải 0 mặc định.
#[test]
fn snapshot_frames_dem_dung_frame_dang_treo() {
    let src = r#"use std.net
async fn victim() -> int:
    let r = net.read("C1", 8, 1000)
    1
fn main():
    let t = spawn victim()
    print("main")
    print(await t)
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
        assert_eq!(
            o.snapshot.frames, 2,
            "{name}: live_frames sai: {:?}",
            o.snapshot
        );
        assert_eq!(
            o.snapshot.waiters, 2,
            "{name}: waiter sai: {:?}",
            o.snapshot
        );
        assert_eq!(o.snapshot.regs, 1, "{name}: reg sai: {:?}", o.snapshot);
        assert_eq!(
            o.snapshot.live_tasks, 1,
            "{name}: task sai: {:?}",
            o.snapshot
        );
        assert_eq!(o.snapshot.ready, 0, "{name}: ready sai: {:?}", o.snapshot);
    }
    assert_diag_parity(&i, &v);
}

// ===========================================================================
// 5) Frame bị huỷ phải được bỏ ngay tại safe point, không giữ tới cuối
// ===========================================================================

/// Task `victim` bị huỷ khi event `seq=0` fire; một task `witness` khác vẫn chạy tiếp.
/// Frame của victim phải bị dọn **giữa** chương trình (`purge:1`), không phải chỉ ở cuối
/// (`purge-end:1`). Bản cũ giữ `IFrame` + env tới hết chương trình.
#[test]
fn frame_huy_bi_bo_ngay_tai_safe_point() {
    let src = r#"async fn src1() -> int:
    0
async fn victim() -> int:
    print("V:pre")
    let x = await src1()
    print("V:post")
    x
async fn witness() -> ():
    print("W:run")
fn main():
    let a = spawn victim()
    let b = spawn witness()
    print("M:run")
"#;
    let c = FrameConfig {
        fake_sources: vec![("src1".into(), 1)],
        events: vec![fake(0, 1, 0, Value::Int(7))],
        ..Default::default()
    };
    let (hir, syms, tys) = tien_trinh(src);
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
        assert_eq!(o.stdout, "M:run\nV:pre\nW:run\n", "{name}: {:?}", o.stdout);
        assert!(!o.stdout.contains("V:post"), "{name}: chạy tiếp sau huỷ");
        assert!(
            o.log.contains(&"cancel:1".to_string()),
            "{name}: không thấy cancel:1: {:?}",
            o.log
        );
        assert!(
            o.log.contains(&"purge:1".to_string()),
            "{name}: frame huỷ không được bỏ giữa chương trình: {:?}",
            o.log
        );
        assert!(
            !o.log.contains(&"purge-end:1".to_string()),
            "{name}: frame huỷ chỉ được bỏ ở cuối: {:?}",
            o.log
        );
        assert_eq!(o.snapshot.frames, 0, "{name}: frame huỷ còn sống");
        assert_eq!(o.snapshot.live_tasks, 0, "{name}: task chưa thu hồi");
    }
    assert_diag_parity(&i, &v);
}
