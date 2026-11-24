// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-3b/W5 (P3, test-only)** — soak ~30 phút parity **interp P2 vs VM P3** (CPU-only W=4).
//!
//! ```text
//! cargo run --release -p vis-cli --example vs3_worker_eval_p3_soak
//! VS3_SOAK_SECS=120 cargo run --release -p vis-cli --example vs3_worker_eval_p3_soak
//! ```
//!
//! Mỗi batch:
//! - 8 task CPU × 20k vòng: interp W=4 vs VM W=4; stdout/exit/codes/value byte-identical.
//! - 10k `spawn`/`await`: interp W=4 vs VM W=4; byte-identical.
//! - budget 10k/chunk 256: cả interp/VM cạn, `used+remaining==limit`, message `VS0510`.
//! - cancel-after 5 ms: task 2M vòng dừng, message `huỷ`.
//! - cross-write W=4: `VS0501` + `ghi chéo`; RSS/threads/fd sau mỗi batch; dừng nếu fail.
//!
//! Không sửa production/default/API; chỉ dùng API `#[doc(hidden)] run_workers_*`.

use std::io::Write;
use std::sync::Arc;
use std::time::{Duration, Instant};

use vis_core::RunOutput;
use vis_runtime::rt::fee::{BudgetSnapshot, SharedBudget};
use vis_syntax::{lex, parse, Mode};

const SOAK_MAC_DINH_SECS: u64 = 1800;
const CPU_TASKS: usize = 8;
const CPU_ITERS: usize = 20_000;
const BUDGET_LIMIT: i64 = 10_000;
const BUDGET_CHUNK: i64 = 256;
const RSS_DAO_DONG_KB: u64 = 65_536;

struct Anh {
    hir: vis_hir::Hir,
    syms: vis_hir::Symbols,
    tys: vis_hir::TypeTable,
    module: vis_bytecode::Module,
}

fn dung_anh(src: &str) -> Anh {
    let p = parse(src, Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse: {:?}", p.diagnostics);
    let l = lex(src, Mode::Script);
    let h = vis_types::lower_with(
        &p.cst,
        &l,
        vis_types::LowerOptions {
            std_mods: vec!["net".into(), "str".into(), "fs".into()],
            ..Default::default()
        },
    );
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    let module = vis_bytecode::compile(&h.hir, &h.symbols, src).expect("compile");
    Anh {
        hir: h.hir,
        syms: h.symbols,
        tys: h.types,
        module,
    }
}

fn interp(a: &Anh, src: &str) -> RunOutput {
    vis_interp::run_workers_parallel(src, &a.hir, &a.syms, &a.tys, &[], 4)
}

fn vm(a: &Anh, src: &str) -> RunOutput {
    vis_vm::run_workers_parallel_vm(&a.module, src, &[], 4)
}

fn so_khop(a: &RunOutput, b: &RunOutput, nhan: &str) {
    assert_eq!(a.stdout, b.stdout, "{nhan}: stdout");
    assert_eq!(a.exit, b.exit, "{nhan}: exit");
    assert_eq!(a.codes(), b.codes(), "{nhan}: codes");
    assert_eq!(
        a.value.as_ref().map(|v| v.show()),
        b.value.as_ref().map(|v| v.show()),
        "{nhan}: value"
    );
}

fn dem_thread() -> Option<usize> {
    std::fs::read_dir("/proc/self/task")
        .ok()
        .map(|it| it.filter_map(Result::ok).count())
}

fn dem_thread_on_dinh() -> Option<usize> {
    let mut min = None;
    for _ in 0..10 {
        if let Some(n) = dem_thread() {
            min = Some(min.map_or(n, |m: usize| m.min(n)));
            if n <= 1 {
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    min
}

fn dem_fd() -> Option<usize> {
    std::fs::read_dir("/proc/self/fd")
        .ok()
        .map(|it| it.filter_map(Result::ok).count())
}

fn rss_kb() -> Option<(u64, u64)> {
    let s = std::fs::read_to_string("/proc/self/status").ok()?;
    let mut rss = None;
    let mut hwm = None;
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            rss = rest.split_whitespace().next()?.parse().ok();
        } else if let Some(rest) = line.strip_prefix("VmHWM:") {
            hwm = rest.split_whitespace().next()?.parse().ok();
        }
    }
    Some((rss?, hwm?))
}

fn load() -> String {
    std::fs::read_to_string("/proc/loadavg")
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn src_cpu(n: usize, iters: usize) -> String {
    let mut than = String::new();
    let mut cong = String::new();
    for i in 0..n {
        than.push_str(&format!("    let t{i} = spawn nang({iters})\n"));
        cong.push_str(&format!("    s = s + await t{i}\n"));
    }
    format!(
        r#"async fn nang(n: int) -> int:
    var s = 0
    var i = 0
    while i < n:
        s = s + i
        i = i + 1
    s

fn main():
{than}    var s = 0
{cong}    print(s)
"#
    )
}

const SPAWN_10K: &str = r#"async fn t(n: int) -> int:
    n + 1

fn main():
    var s = 0
    var i = 0
    while i < 10000:
        let x = spawn t(i)
        s = s + await x
        i = i + 1
    print(s)
"#;

const BUDGET: &str = r#"async fn nang(n: int) -> int:
    var s = 0
    var i = 0
    while i < n:
        s = s + i
        i = i + 1
    s

fn main():
    let t = spawn nang(1000000)
    print(await t)
"#;

const CANCEL: &str = r#"async fn nang(n: int) -> int:
    var s = 0
    var i = 0
    while i < n:
        s = s + i
        i = i + 1
    s

fn main():
    let t = spawn nang(2000000)
    print(await t)
"#;

const CROSS: &str = r#"var counter = 0

async fn tang():
    counter = counter + 1

fn main():
    let t = spawn tang()
    await t
    print(counter)
"#;

fn pct(v: &mut [f64], p: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let i = ((v.len() - 1) as f64 * p).round() as usize;
    v[i]
}

fn budget_i(a: &Anh) -> (RunOutput, BudgetSnapshot) {
    let b = Arc::new(SharedBudget::new(BUDGET_LIMIT as u64, BUDGET_CHUNK as u64));
    let out = vis_interp::run_workers_parallel_with_budget(
        BUDGET,
        &a.hir,
        &a.syms,
        &a.tys,
        &[],
        4,
        Arc::clone(&b),
    );
    (out, b.snapshot())
}

fn budget_v(a: &Anh) -> (RunOutput, BudgetSnapshot) {
    let b = Arc::new(SharedBudget::new(BUDGET_LIMIT as u64, BUDGET_CHUNK as u64));
    let out =
        vis_vm::run_workers_parallel_vm_with_budget(&a.module, BUDGET, &[], 4, Arc::clone(&b));
    (out, b.snapshot())
}

fn main() {
    let secs = std::env::var("VS3_SOAK_SECS")
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(SOAK_MAC_DINH_SECS);
    assert!(secs >= 10, "VS3_SOAK_SECS quá nhỏ");
    let src_cpu = src_cpu(CPU_TASKS, CPU_ITERS);
    let a_cpu = dung_anh(&src_cpu);
    let a_10k = dung_anh(SPAWN_10K);
    let a_budget = dung_anh(BUDGET);
    let a_cancel = dung_anh(CANCEL);
    let a_cross = dung_anh(CROSS);

    let threads0 = dem_thread_on_dinh();
    let fd0 = dem_fd();
    let (rss0, _) = rss_kb().unwrap_or((0, 0));
    println!(
        "SOAK_P3\tbat_dau\tsecs={secs}\tcpu_tasks={CPU_TASKS}\tcpu_iters={CPU_ITERS}\tload={}\trss_kb={rss0}\tthreads={threads0:?}\tfd={fd0:?}",
        load()
    );
    let t0 = Instant::now();
    let mut batches = 0u64;
    let mut batch_ms = Vec::new();
    let mut cpu_i = Vec::new();
    let mut cpu_v = Vec::new();
    let mut k10_i = Vec::new();
    let mut k10_v = Vec::new();
    let mut rss_after_warm: Vec<u64> = Vec::new();
    let mut hwm = 0u64;
    let mut last_log = Instant::now();

    while t0.elapsed() < Duration::from_secs(secs) {
        let tb = Instant::now();

        // 8 task CPU × 20k: interp P2 vs VM P3, cùng output.
        let t = Instant::now();
        let ci = interp(&a_cpu, &src_cpu);
        let di = t.elapsed().as_secs_f64() * 1000.0;
        let t = Instant::now();
        let cv = vm(&a_cpu, &src_cpu);
        let dv = t.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(ci.exit, 0, "CPU interp: {:?}", ci.diagnostics);
        so_khop(&ci, &cv, "CPU interp/VM");
        cpu_i.push(di);
        cpu_v.push(dv);

        // 10k spawn/await: interp vs VM.
        let t = Instant::now();
        let ki = interp(&a_10k, SPAWN_10K);
        let ei = t.elapsed().as_secs_f64() * 1000.0;
        let t = Instant::now();
        let kv = vm(&a_10k, SPAWN_10K);
        let ev = t.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(ki.exit, 0, "10k interp: {:?}", ki.diagnostics);
        so_khop(&ki, &kv, "10k interp/VM");
        k10_i.push(ei);
        k10_v.push(ev);

        // Budget: cả hai phải cạn, tổng đúng.
        let (bi, si) = budget_i(&a_budget);
        let (bv, sv) = budget_v(&a_budget);
        for (ten, o, s) in [("interp", &bi, &si), ("vm", &bv, &sv)] {
            assert_eq!(o.exit, 1, "budget {ten} phải lỗi");
            assert!(
                o.diagnostics.iter().any(|d| d.message.contains("VS0510")),
                "budget {ten}: {:?}",
                o.diagnostics
            );
            assert_eq!(s.used + s.remaining, s.limit, "budget {ten}: {s:?}");
            assert!(s.used > 0 && s.exhausted, "budget {ten}: {s:?}");
        }

        // Cancel-after 5ms: cả hai dừng.
        let co_i = vis_interp::run_workers_parallel_cancel_after(
            CANCEL,
            &a_cancel.hir,
            &a_cancel.syms,
            &a_cancel.tys,
            &[],
            4,
            5,
        );
        let co_v =
            vis_vm::run_workers_parallel_vm_cancel_after(&a_cancel.module, CANCEL, &[], 4, 5);
        for (ten, o) in [("interp", &co_i), ("vm", &co_v)] {
            assert_eq!(o.exit, 1, "cancel {ten} phải lỗi");
            assert!(
                o.diagnostics.iter().any(|d| d.message.contains("huỷ")),
                "cancel {ten}: {:?}",
                o.diagnostics
            );
        }

        // Cross-write W=4: cả hai VS0501 + ghi chéo.
        let xi = interp(&a_cross, CROSS);
        let xv = vm(&a_cross, CROSS);
        for (ten, o) in [("interp", &xi), ("vm", &xv)] {
            assert_eq!(o.exit, 1, "cross {ten} phải lỗi");
            assert!(
                o.diagnostics.iter().any(|d| d.message.contains("ghi chéo")),
                "cross {ten}: {:?}",
                o.diagnostics
            );
        }

        batches += 1;
        batch_ms.push(tb.elapsed().as_secs_f64() * 1000.0);
        let (rss, h) = rss_kb().unwrap_or((0, 0));
        hwm = hwm.max(h);
        if t0.elapsed() >= Duration::from_secs(60) {
            rss_after_warm.push(rss);
            let mn = *rss_after_warm.iter().min().unwrap();
            let mx = *rss_after_warm.iter().max().unwrap();
            assert!(
                mx.saturating_sub(mn) <= RSS_DAO_DONG_KB,
                "RSS không plateau: min={mn} max={mx} kB"
            );
            assert!(
                rss <= rss0 + RSS_DAO_DONG_KB,
                "RSS tăng quá 64 MB: {rss0}->{rss}"
            );
        }
        let th = dem_thread_on_dinh();
        let fd = dem_fd();
        if let (Some(t0n), Some(tn)) = (threads0, th) {
            assert!(tn <= t0n + 1, "rò thread: {t0n} -> {tn}");
        }
        if let (Some(f0), Some(fn_)) = (fd0, fd) {
            assert!(fn_ <= f0 + 4, "rò fd: {f0} -> {fn_}");
        }

        if last_log.elapsed() >= Duration::from_secs(5) {
            println!(
                "SOAK_BATCH\t{batches}\telapsed_s={:.1}\tbatch_ms={:.0}\tcpu_i_ms={di:.0}\tcpu_v_ms={dv:.0}\tk10_i_ms={ei:.0}\tk10_v_ms={ev:.0}\trss_kb={rss}\tthreads={th:?}\tfd={fd:?}\tload={}",
                t0.elapsed().as_secs_f64(),
                batch_ms.last().copied().unwrap_or(0.0),
                load()
            );
            std::io::stdout().flush().ok();
            last_log = Instant::now();
        }
    }

    let elapsed = t0.elapsed().as_secs_f64();
    let tasks = batches * (CPU_TASKS as u64 * 2 + 10_000 * 2 + 2 + 2 + 2);
    let (rss, _) = rss_kb().unwrap_or((0, 0));
    println!(
        "SOAK_TONG\tsecs={secs}\telapsed_s={elapsed:.1}\tbatches={batches}\tbatches_s={:.2}\ttasks={tasks}\ttasks_s={:.0}",
        batches as f64 / elapsed,
        tasks as f64 / elapsed
    );
    println!(
        "SOAK_WALL\tcpu_i_p50={:.0}\tcpu_i_p95={:.0}\tcpu_v_p50={:.0}\tcpu_v_p95={:.0}\tk10_i_p50={:.0}\tk10_i_p95={:.0}\tk10_v_p50={:.0}\tk10_v_p95={:.0}",
        pct(&mut cpu_i, 0.50),
        pct(&mut cpu_i, 0.95),
        pct(&mut cpu_v, 0.50),
        pct(&mut cpu_v, 0.95),
        pct(&mut k10_i, 0.50),
        pct(&mut k10_i, 0.95),
        pct(&mut k10_v, 0.50),
        pct(&mut k10_v, 0.95),
    );
    let rmin = rss_after_warm.iter().min().copied().unwrap_or(rss);
    let rmax = rss_after_warm.iter().max().copied().unwrap_or(rss);
    println!(
        "SOAK_TAI_NGUYEN\trss0={rss0}\trss_min_sau_warm={rmin}\trss_max_sau_warm={rmax}\trss_cuoi={rss}\thwm={hwm}\tthreads={:?}\tfd={:?}",
        dem_thread_on_dinh(),
        dem_fd()
    );
    println!("KET_LUAN\tPASS\t{batches} batch\t{elapsed:.1}s\tkhông lỗi");
}
