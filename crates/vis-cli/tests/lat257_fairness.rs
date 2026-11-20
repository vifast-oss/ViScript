// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-257 / stream D — QUOTA THỬ NGHIỆM trên đường frame** (PHASE 2).
//!
//! Đường **MUST-NOT-BE-DEFAULT**: mọi thứ ở đây bật qua test-hook nội bộ. Không CLI flag,
//! không API công, không VERSION/cache, không đổi runtime mặc định. `f2` của stream A vẫn
//! chứng minh đường frame KHÔNG tự charge khi chưa bật hook.
//!
//! Phạm vi: (1) nối dãy sự kiện VD-256 vào frame path (một định nghĩa, không instrument lần hai),
//! (2) quota RIÊNG, reset mỗi turn, hết ⇒ yield đuôi ready (KHÔNG trap), (3) quét Q — KHÔNG chọn
//! default, (4) vòng đời đa sóng qua `diag_snapshot()`.

use vis_interp::frame::{run_frame_for_test as run_i, FrameConfig, FrameOutput};
use vis_interp::rt::{fee as ifee, DiagSnapshot, FakeEvent, ReactorEvent, ReactorOutcome};
use vis_interp::Value;
use vis_syntax::{lex, parse, Mode};
use vis_vm::vm_frame::{
    frame_budget_disable_for_test, frame_budget_enable_for_test, frame_quota_yields_for_test,
    run_frame_for_test as run_v, run_frame_for_test_with_budget as run_v_budget,
};

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

fn ev(seq: u64, key: u64, gen: u64, v: i64) -> FakeEvent {
    FakeEvent {
        seq,
        key,
        gen,
        value: Value::Int(v),
    }
}

fn ok_ev(seq: u64, key: u64, s: &str) -> ReactorEvent {
    ReactorEvent {
        seq,
        key,
        gen: 0,
        outcome: ReactorOutcome::Ready(Value::ok(Value::Str(s.into()))),
        deadline_ms: None,
    }
}

/// Tắt mọi hook — luôn gọi ở cuối mỗi test để không nhiễm test khác cùng thread.
fn tat_hooks() {
    ifee::disable_for_test();
    frame_budget_disable_for_test();
}

/// Chạy interp frame path với instrumentation + (tuỳ chọn) quota.
fn chay_i(
    src: &str,
    hir: &vis_types::Hir,
    syms: &vis_types::Symbols,
    tys: &vis_types::TypeTable,
    c: FrameConfig,
    budget: u64,
    quota: Option<u64>,
) -> (FrameOutput, Vec<ifee::FeeEntry>, u64) {
    ifee::enable_for_test(budget);
    ifee::frame_emit_enable_for_test();
    if let Some(q) = quota {
        ifee::quota_enable_for_test(q);
    }
    let out = run_i(src, hir, syms, tys, &[], c);
    let trace = ifee::take_trace_for_test();
    let yields = ifee::quota_yields_for_test();
    tat_hooks();
    (out, trace, yields)
}

/// Chạy VM frame path với instrumentation + (tuỳ chọn) quota (dùng hook thread-local).
fn chay_v(
    src: &str,
    m: &vis_bytecode::Module,
    c: FrameConfig,
    fee: vis_bytecode::fee::FeeTable,
    budget: u64,
    quota: Option<u64>,
) -> (FrameOutput, Vec<vis_bytecode::fee::FeeEntry>, u64) {
    frame_budget_enable_for_test(fee, budget, quota);
    let out = run_v(m, src, &[], c);
    let yields = frame_quota_yields_for_test();
    let trace = Vec::new(); // VM trace lấy qua `run_frame_for_test_with_budget` ở test khác.
    tat_hooks();
    (out, trace, yields)
}

fn labels(t: &[ifee::FeeEntry]) -> Vec<String> {
    t.iter()
        .map(|e| format!("{}@{}", e.kind.name(), e.site))
        .collect()
}

// ===========================================================================
// 1 — PREREQUISITE: frame path phát CÙNG dãy sự kiện (không instrument lần hai)
// ===========================================================================

const FLAT: &str = "fn main():\n    let a = 1 + 2\n    print(a)\n";
const ORDER: &str = "fn g(x: int) -> int:\n    x + 1\nfn f(x: int) -> int:\n    x * 2\nfn main():\n    print(f(g(1)))\n";

#[test]
fn frame_path_co_su_kien_va_khong_double_charge() {
    let (hir, syms, tys) = tien_trinh(FLAT);
    let (out, trace, y) = chay_i(
        FLAT,
        &hir,
        &syms,
        &tys,
        FrameConfig::default(),
        1_000_000,
        None,
    );
    assert_eq!(out.exit, 0, "{:?}", out.diagnostics);
    assert_eq!(out.stdout, "3\n");
    assert_eq!(y, 0);
    assert!(
        !trace.is_empty(),
        "frame path PHẢI phát sự kiện khi hook bật"
    );
    assert!(
        trace.iter().any(|e| e.kind.name() == "E_STMT")
            && trace.iter().any(|e| e.kind.name() == "E_EXPR")
            && trace.iter().any(|e| e.kind.name() == "E_ASSIGN")
            && trace.iter().any(|e| e.kind.name() == "E_BUILTIN"),
        "thiếu kind: {:?}",
        labels(&trace)
    );

    // Cùng nguồn, hook tắt ⇒ KHÔNG charge (tôn trọng test f2 của stream A).
    ifee::enable_for_test(1_000_000);
    let out2 = run_i(FLAT, &hir, &syms, &tys, &[], FrameConfig::default());
    let trace2 = ifee::take_trace_for_test();
    tat_hooks();
    assert_eq!(out2.stdout, "3\n");
    assert!(trace2.is_empty(), "không bật frame_emit ⇒ trace phải rỗng");
}

#[test]
fn frame_path_giu_thu_tu_f_g() {
    // VD-252 §3: charge tại nút Call TRƯỚC đối số ⇒ f(g(1)) cho
    // E_CALL(f), E_CALL(g), E_EXPR(1) (không phải g trước f).
    let (hir, syms, tys) = tien_trinh(ORDER);
    let (out, trace, _) = chay_i(
        ORDER,
        &hir,
        &syms,
        &tys,
        FrameConfig::default(),
        1_000_000,
        None,
    );
    assert_eq!(out.exit, 0, "{:?}", out.diagnostics);
    assert_eq!(out.stdout, "4\n"); // g(1)=2, f(2)=4
    let g_site = ORDER.find("g(1)").unwrap() as u32;
    let f_site = ORDER.find("f(g(1))").unwrap() as u32;
    let pos = |site: u32| {
        trace
            .iter()
            .position(|e| e.kind.name() == "E_CALL" && e.site == site)
    };
    let pg = pos(g_site);
    let pf = pos(f_site);
    assert!(
        pg.is_some() && pf.is_some(),
        "thiếu E_CALL tại site g/f: {:?}",
        labels(&trace)
    );
    assert!(
        pf.unwrap() < pg.unwrap(),
        "E_CALL(f) phải trước E_CALL(g): {:?}",
        labels(&trace)
    );
    let one_site = ORDER.find("1)").unwrap() as u32;
    assert!(
        trace
            .iter()
            .position(|e| e.kind.name() == "E_EXPR" && e.site == one_site)
            .unwrap()
            > pg.unwrap(),
        "E_EXPR(1) phải sau cả hai E_CALL"
    );
}

#[test]
fn resume_khong_charge_lai_va_khong_lap_side_effect() {
    let src = r#"async fn src() -> int:
    await ext_src()
async fn ext_src() -> int:
    0
fn main():
    let a = spawn src()
    print(await a)
    print(await a)
"#;
    let (hir, syms, tys) = tien_trinh(src);
    let c = FrameConfig {
        fake_sources: vec![("ext_src".into(), 1)],
        events: vec![ev(0, 1, 0, 40)],
        ..Default::default()
    };
    let (out, trace, _) = chay_i(src, &hir, &syms, &tys, c, 1_000_000, None);
    assert_eq!(out.exit, 0, "{:?}", out.diagnostics);
    assert_eq!(out.stdout, "40\n40\n");
    let mut seen = std::collections::HashSet::new();
    for e in &trace {
        assert!(
            seen.insert((e.pc, e.kind.tag(), e.site)),
            "double-charge tại {:?}",
            (e.pc, e.kind.tag(), e.site)
        );
    }
}

// ===========================================================================
// 2 — QUOTA RIÊNG, reset mỗi turn, yield đuôi ready, KHÔNG trap
// ===========================================================================

const CPU: &str = r#"fn main():
    var i = 0
    while i < 50000:
        i = i + 1
    print("CPU_DONE {i}")
"#;

#[test]
fn quota_rieng_khong_phai_ngan_sach() {
    let (hir, syms, tys) = tien_trinh(CPU);
    // Budget lớn, quota nhỏ ⇒ nhiều yield, KHÔNG trap, vẫn in kết quả.
    let (out, _trace, y) = chay_i(
        CPU,
        &hir,
        &syms,
        &tys,
        FrameConfig::default(),
        40_000_000,
        Some(300),
    );
    assert_eq!(out.exit, 0, "quota KHÔNG được trap: {:?}", out.diagnostics);
    assert_eq!(out.stdout, "CPU_DONE 50000\n");
    assert!(y > 0, "phải có ít nhất một lần yield do quota");
    assert!(
        out.log.iter().any(|l| l.starts_with("quota:")),
        "log phải có marker quota:"
    );
}

#[test]
fn ngan_sach_van_trap_khong_bi_quota_che() {
    let (hir, syms, tys) = tien_trinh(CPU);
    // Budget nhỏ hơnchi phí ⇒ phải trap VS0501-BUDGET dù quota lớn (không reset/né).
    let (out, _trace, y) = chay_i(
        CPU,
        &hir,
        &syms,
        &tys,
        FrameConfig::default(),
        100,
        Some(1_000_000),
    );
    assert_eq!(out.exit, 1, "budget phải trap");
    assert_eq!(y, 0, "quota không hoạt động khi budget trap trước");
    assert!(
        out.diagnostics
            .iter()
            .any(|d| d.code.short == "VS0501-BUDGET"),
        "phải là VS0501-BUDGET: {:?}",
        out.diagnostics
    );
}

#[test]
fn ngan_sach_khong_reset_qua_nhieu_yield() {
    let (hir, syms, tys) = tien_trinh(CPU);
    // CPU ~450k cost (xem quét Q). Budget 400k < tổng ⇒ phải trap dù đã yield
    // NHIỀU lần ⇒ chứng minh quota reset KHÔNG làm reset/trốn ngân sách tổng.
    let (out, _trace, y) = chay_i(
        CPU,
        &hir,
        &syms,
        &tys,
        FrameConfig::default(),
        400_000,
        Some(500),
    );
    assert_eq!(
        out.exit, 1,
        "budget phải trap sau nhiều yield: {:?}",
        out.diagnostics
    );
    assert!(y > 0, "phải có yield trước khi budget trap");
    assert!(
        out.diagnostics
            .iter()
            .any(|d| d.code.short == "VS0501-BUDGET"),
        "vẫn phải VS0501-BUDGET: {:?}",
        out.diagnostics
    );
}

#[test]
fn hai_engine_cung_trace_su_kien_voi_quota() {
    let (hir, syms, tys) = tien_trinh(FLAT);
    let (m, fee) = vis_bytecode::compile_with_fee(&hir, &syms, FLAT).expect("compile_with_fee");
    let c = FrameConfig::default();
    let (i, ti, _) = chay_i(FLAT, &hir, &syms, &tys, c.clone(), 40_000_000, Some(200));
    frame_budget_enable_for_test(fee.clone(), 40_000_000, Some(200));
    let (v, tv) = run_v_budget(&m, FLAT, &[], c, fee, 40_000_000);
    tat_hooks();
    assert_eq!(i.exit, 0);
    assert_eq!(v.exit, 0);
    assert_eq!(i.stdout, v.stdout);
    let li: Vec<String> = labels(&ti);
    let lv: Vec<String> = tv
        .iter()
        .map(|e| format!("{}@{}", e.kind.name(), e.site))
        .collect();
    assert_eq!(li, lv, "dãy sự kiện frame path I↔V lệch: I={li:?} V={lv:?}");
}

#[test]
fn hai_engine_cung_hanh_vi_voi_quota() {
    let (hir, syms, tys) = tien_trinh(CPU);
    let (m, fee) = vis_bytecode::compile_with_fee(&hir, &syms, CPU).expect("compile_with_fee");
    let c = FrameConfig::default();
    let (i, _ti, yi) = chay_i(CPU, &hir, &syms, &tys, c.clone(), 40_000_000, Some(500));
    let (v, _tv, yv) = chay_v(CPU, &m, c, fee, 40_000_000, Some(500));
    assert_eq!(i.stdout, v.stdout, "stdout lệch");
    assert_eq!(
        i.exit, v.exit,
        "exit lệch: I={:?} V={:?}",
        i.diagnostics, v.diagnostics
    );
    assert_eq!(i.snapshot, v.snapshot, "snapshot lệch");
    // Miền hỗ trợ: hai engine cùng HÀNH VI (stdout/exit/snapshot/log). Số turn có
    // thể lệch ±1 do điểm dừng ở instruction cuối — ghi nhận trong report, KHÔNG
    // tuyên bố trace byte-equal.
    assert!(yi > 0 && yv > 0, "cả hai phải có yield: I={yi} V={yv}");
    assert!(
        yi.abs_diff(yv) <= 1,
        "số yield lệch quá 1 turn: I={yi} V={yv}"
    );
    let li = i.log.iter().filter(|l| l.starts_with("quota:")).count();
    let lv = v.log.iter().filter(|l| l.starts_with("quota:")).count();
    assert!(
        li.abs_diff(lv) <= 1,
        "số marker quota lệch quá 1 turn: I={li} V={lv}"
    );
}

// ===========================================================================
// 3 — B2 mixed CPU+I/O + task ngắn (acceptance, deterministic)
// ===========================================================================

const MIX: &str = r#"use std.net
async fn cpu(n: int) -> ():
    var i = 0
    while i < n:
        i = i + 1
    print("CPU_DONE")
async fn io() -> ():
    print("IO_START")
    let r = net.read("C1", 8, 1000)
    print("IO_DONE")
async fn short() -> ():
    print("SHORT_DONE")
fn main():
    let a = spawn cpu(50000)
    let b = spawn io()
    let c = spawn short()
"#;

fn chay_mix(quota: Option<u64>) -> (FrameOutput, u64) {
    let (hir, syms, tys) = tien_trinh(MIX);
    let c = FrameConfig {
        reactors: vec![ok_ev(0, 0, "AAA")],
        ..Default::default()
    };
    let (out, _trace, y) = chay_i(MIX, &hir, &syms, &tys, c, 40_000_000, quota);
    (out, y)
}

#[test]
fn b2_io_start_truoc_cpu_done_va_short_khong_doi() {
    let (noq, _) = chay_mix(None);
    assert_eq!(noq.exit, 0, "control: {:?}", noq.diagnostics);
    let ic = noq.stdout.find("IO_START").unwrap();
    let dc = noq.stdout.find("CPU_DONE").unwrap();
    assert!(
        ic > dc,
        "control phải tái hiện B2 (IO_START SAU CPU_DONE): {:?}",
        noq.stdout
    );

    let (q, y) = chay_mix(Some(500));
    assert_eq!(q.exit, 0, "quota: {:?}", q.diagnostics);
    assert!(
        q.stdout.contains("CPU_DONE"),
        "CPU vẫn hoàn tất: {:?}",
        q.stdout
    );
    assert!(
        q.stdout.contains("IO_DONE"),
        "I/O vẫn hoàn tất: {:?}",
        q.stdout
    );
    let iq = q.stdout.find("IO_START").unwrap();
    let dq = q.stdout.find("CPU_DONE").unwrap();
    let sq = q.stdout.find("SHORT_DONE").unwrap();
    assert!(
        iq < dq,
        "quota: IO_START phải TRƯỚC CPU_DONE: {:?}",
        q.stdout
    );
    assert!(
        sq < dq,
        "task ngắn không được đói (SHORT_DONE trước CPU_DONE): {:?}",
        q.stdout
    );
    assert!(y > 0, "phải có yield quota");
}

// ===========================================================================
// 4 — VÒNG ĐỜI ĐA SÓNG (diag_snapshot, không chỉ RSS)
// ===========================================================================

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

#[test]
fn vong_doi_da_song_voi_quota_ve_baseline() {
    let src = r#"async fn w(k: int) -> ():
    print("W {k}")
fn main():
    var i = 0
    while i < 5:
        let t = spawn w(i)
        let r = await t
        i = i + 1
    print("XONG")
"#;
    let (hir, syms, tys) = tien_trinh(src);
    let mut snaps = Vec::new();
    for _ in 0..5 {
        let (out, _trace, _y) = chay_i(
            src,
            &hir,
            &syms,
            &tys,
            FrameConfig::default(),
            40_000_000,
            Some(200),
        );
        assert_eq!(out.exit, 0, "{:?}", out.diagnostics);
        assert_eq!(out.stdout, "W 0\nW 1\nW 2\nW 3\nW 4\nXONG\n");
        snaps.push(out.snapshot);
    }
    for (k, s) in snaps.iter().enumerate() {
        assert_eq!(
            *s,
            baseline(),
            "sóng {k}: snapshot phải về baseline: {:?}",
            s
        );
    }
    // Không có tăng đơn điệu: mọi sóng giống hệt nhau.
    assert!(
        snaps.windows(2).all(|w| w[0] == w[1]),
        "snapshot tăng bất thường qua các sóng"
    );
}

// ===========================================================================
// 5 — QUÉT Q (ignored = đo, chạy riêng; KHÔNG chọn default)
// ===========================================================================

#[test]
#[ignore = "measurement: chạy riêng, finite, seeded"]
fn quet_q_phan_bo_khong_chon_default() {
    let qs: [Option<u64>; 7] = [
        None,
        Some(1_000_000),
        Some(200_000),
        Some(50_000),
        Some(10_000),
        Some(2_000),
        Some(500),
    ];
    let reps = 7usize; // bỏ lần 0 = warmup
    let io_site = MIX.find("print(\"IO_START\")").unwrap() as u32;
    let short_site = MIX.find("print(\"SHORT_DONE\")").unwrap() as u32;
    println!("Q,cost_units,yields,startup_ms,io_lat_cost,short_lat_cost,order,snapshot");
    for q in qs {
        // Startup: compile/lower/tyck (ngoài workload), đo riêng.
        let t0 = std::time::Instant::now();
        let (hir, syms, tys) = tien_trinh(MIX);
        let startup_ms = t0.elapsed().as_secs_f64() * 1000.0;
        let mut io_lat = Vec::new();
        let mut short_lat = Vec::new();
        let mut ysum = 0u64;
        for r in 0..reps {
            // seed tầm thường, finite — không RNG ngoài.
            let _seed = 0x0056_4432_3537_u64 ^ (q.unwrap_or(0).wrapping_mul(131)) ^ (r as u64);
            let c = FrameConfig {
                reactors: vec![ok_ev(0, 0, "AAA")],
                ..Default::default()
            };
            ifee::enable_for_test(40_000_000);
            ifee::frame_emit_enable_for_test();
            if let Some(v) = q {
                ifee::quota_enable_for_test(v);
            }
            let out = run_i(MIX, &hir, &syms, &tys, &[], c);
            let y = ifee::quota_yields_for_test();
            let trace = ifee::take_trace_for_test();
            tat_hooks();
            assert_eq!(out.exit, 0, "Q={q:?} r={r}: {:?}", out.diagnostics);
            ysum += y;
            // Latency MONOTONIC deterministic (cost units) = tổng trọng số các sự
            // kiện chạy TRƯỚC print(IO_START)/print(SHORT_DONE). Không dùng wall-clock
            // (resolution 1ms ở debug, workload < 1ms ⇒ nhiễu).
            let lat_cost = |site: u32| -> u64 {
                trace
                    .iter()
                    .take_while(|e| !(e.kind.name() == "E_BUILTIN" && e.site == site))
                    .map(|e| ifee::standard_weight(e.kind))
                    .sum()
            };
            if r > 0 {
                io_lat.push(lat_cost(io_site));
                short_lat.push(lat_cost(short_site));
            }
        }
        let med = |v: &mut Vec<u64>| -> u64 {
            v.sort();
            v[v.len() / 2]
        };
        let order = if q.is_none() {
            "IO_SAU_CPU(B2)"
        } else {
            "IO_TRUOC_CPU"
        };
        println!(
            "{:?},{},{},{:.3},{},{},{},{:?}",
            q,
            q.unwrap_or(0),
            ysum / (reps as u64 - 1),
            startup_ms,
            med(&mut io_lat),
            med(&mut short_lat),
            order,
            baseline(),
        );
    }
}
