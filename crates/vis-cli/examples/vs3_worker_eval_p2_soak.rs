// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-3b/W5 (P2, test-only)** — soak ~30 phút cho evaluator W>1 CPU-only.
//!
//! Chạy tay (không nằm trong gate/test suite):
//! ```text
//! cargo run --release -p vis-cli --example vs3_worker_eval_p2_soak
//! VS3_SOAK_SECS=120 cargo run --release -p vis-cli --example vs3_worker_eval_p2_soak
//! ```
//!
//! Mỗi batch:
//! - 8 task CPU × 20k vòng: chạy W=1 (local), W=2, W=4; output/exit/codes byte-identical.
//! - 10.000 task `spawn`/`await`: chạy W=1/2/4; output/exit byte-identical.
//! - budget `SharedBudget(10k, chunk 256)`: task 1M vòng phải cạn, `used+remaining==limit`.
//! - cancel-after 5 ms: task 2M vòng phải dừng với message `huỷ`.
//! - cross-write W=1 shared env / W=4 `VS0501` + message `ghi chéo`.
//! - RSS/threads/fd sau mỗi batch; dừng ngay nếu lệch/không plateau.
//!
//! Không sửa production/default/API; chỉ dùng API `#[doc(hidden)] run_workers_*`.

use std::io::Write;
use std::sync::Arc;
use std::time::{Duration, Instant};

use vis_core::RunOutput;
use vis_runtime::rt::fee::SharedBudget;
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
    Anh {
        hir: h.hir,
        syms: h.symbols,
        tys: h.types,
    }
}

fn chay(a: &Anh, src: &str, w: usize) -> RunOutput {
    if w == 1 {
        vis_interp::run_workers_local(src, &a.hir, &a.syms, &a.tys, &[])
    } else {
        vis_interp::run_workers_parallel(src, &a.hir, &a.syms, &a.tys, &[], w)
    }
}

fn so_khop(a: &RunOutput, b: &RunOutput, nhan: &str) {
    assert_eq!(a.stdout, b.stdout, "{nhan}: stdout");
    assert_eq!(a.exit, b.exit, "{nhan}: exit");
    assert_eq!(a.codes(), b.codes(), "{nhan}: codes");
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

fn main() {
    let secs = std::env::var("VS3_SOAK_SECS")
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(SOAK_MAC_DINH_SECS);
    assert!(secs >= 10, "VS3_SOAK_SECS quá nhỏ");
    let src_cpu = src_cpu(CPU_TASKS, CPU_ITERS);
    let anh_cpu = dung_anh(&src_cpu);
    let anh_10k = dung_anh(SPAWN_10K);
    let anh_budget = dung_anh(BUDGET);
    let anh_cancel = dung_anh(CANCEL);
    let anh_cross = dung_anh(CROSS);

    let threads0 = dem_thread_on_dinh();
    let fd0 = dem_fd();
    let (rss0, _) = rss_kb().unwrap_or((0, 0));
    println!(
        "SOAK_P2\tbat_dau\tsecs={secs}\tcpu_tasks={CPU_TASKS}\tcpu_iters={CPU_ITERS}\tload={}\trss_kb={rss0}\tthreads={threads0:?}\tfd={fd0:?}",
        load()
    );
    let t0 = Instant::now();
    let mut batches = 0u64;
    let mut batch_ms = Vec::new();
    let mut cpu_w1 = Vec::new();
    let mut cpu_w2 = Vec::new();
    let mut cpu_w4 = Vec::new();
    let mut k10_w1 = Vec::new();
    let mut k10_w2 = Vec::new();
    let mut k10_w4 = Vec::new();
    let mut rss_after_warm: Vec<u64> = Vec::new();
    let mut hwm = 0u64;
    let mut last_log = Instant::now();

    while t0.elapsed() < Duration::from_secs(secs) {
        let tb = Instant::now();
        // 1) 8 task CPU × 20k: W=1/2/4 byte-identical.
        let ta = Instant::now();
        let c1 = chay(&anh_cpu, &src_cpu, 1);
        let d1 = ta.elapsed().as_secs_f64() * 1000.0;
        let ta = Instant::now();
        let c2 = chay(&anh_cpu, &src_cpu, 2);
        let d2 = ta.elapsed().as_secs_f64() * 1000.0;
        let ta = Instant::now();
        let c4 = chay(&anh_cpu, &src_cpu, 4);
        let d4 = ta.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(c1.exit, 0, "CPU W=1: {:?}", c1.diagnostics);
        so_khop(&c1, &c2, "CPU W=1/W=2");
        so_khop(&c1, &c4, "CPU W=1/W=4");
        cpu_w1.push(d1);
        cpu_w2.push(d2);
        cpu_w4.push(d4);

        // 2) 10k spawn/await W=1/2/4.
        let ta = Instant::now();
        let k1 = chay(&anh_10k, SPAWN_10K, 1);
        let e1 = ta.elapsed().as_secs_f64() * 1000.0;
        let ta = Instant::now();
        let k2 = chay(&anh_10k, SPAWN_10K, 2);
        let e2 = ta.elapsed().as_secs_f64() * 1000.0;
        let ta = Instant::now();
        let k4 = chay(&anh_10k, SPAWN_10K, 4);
        let e4 = ta.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(k1.exit, 0, "10k W=1: {:?}", k1.diagnostics);
        so_khop(&k1, &k2, "10k W=1/W=2");
        so_khop(&k1, &k4, "10k W=1/W=4");
        k10_w1.push(e1);
        k10_w2.push(e2);
        k10_w4.push(e4);

        // 3) Budget: cạn chính xác, used+remaining==limit.
        let budget = Arc::new(SharedBudget::new(BUDGET_LIMIT as u64, BUDGET_CHUNK as u64));
        let bo = vis_interp::run_workers_parallel_with_budget(
            BUDGET,
            &anh_budget.hir,
            &anh_budget.syms,
            &anh_budget.tys,
            &[],
            4,
            Arc::clone(&budget),
        );
        assert_eq!(bo.exit, 1, "budget phải lỗi");
        assert!(
            bo.diagnostics.iter().any(|d| d.message.contains("VS0510")),
            "budget message: {:?}",
            bo.diagnostics
        );
        let snap = budget.snapshot();
        assert_eq!(snap.used + snap.remaining, snap.limit, "budget {snap:?}");
        assert!(snap.exhausted, "budget phải cạn: {snap:?}");

        // 4) Cancel-after 5 ms: task 2M vòng phải dừng, không treo.
        let co = vis_interp::run_workers_parallel_cancel_after(
            CANCEL,
            &anh_cancel.hir,
            &anh_cancel.syms,
            &anh_cancel.tys,
            &[],
            4,
            5,
        );
        assert_eq!(co.exit, 1, "cancel phải lỗi");
        assert!(
            co.diagnostics.iter().any(|d| d.message.contains("huỷ")),
            "cancel message: {:?}",
            co.diagnostics
        );

        // 5) Cross-write: W=1 shared env, W=4 VS0501 + message.
        let x1 = chay(&anh_cross, CROSS, 1);
        assert_eq!(x1.exit, 0, "cross W=1: {:?}", x1.diagnostics);
        assert_eq!(x1.stdout, "1\n");
        let x4 = chay(&anh_cross, CROSS, 4);
        assert_eq!(x4.exit, 1, "cross W=4 phải lỗi");
        assert!(
            x4.diagnostics
                .iter()
                .any(|d| d.message.contains("ghi chéo")),
            "cross message: {:?}",
            x4.diagnostics
        );

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
                "SOAK_BATCH\t{batches}\telapsed_s={:.1}\tbatch_ms={:.0}\tcpu_w1_ms={d1:.0}\tcpu_w2_ms={d2:.0}\tcpu_w4_ms={d4:.0}\tk10_w1_ms={e1:.0}\tk10_w2_ms={e2:.0}\tk10_w4_ms={e4:.0}\trss_kb={rss}\tthreads={th:?}\tfd={fd:?}\tload={}",
                t0.elapsed().as_secs_f64(),
                batch_ms.last().copied().unwrap_or(0.0),
                load()
            );
            std::io::stdout().flush().ok();
            last_log = Instant::now();
        }
    }

    let elapsed = t0.elapsed().as_secs_f64();
    let tasks = batches * (CPU_TASKS as u64 * 3 + 10_000 * 3 + 3);
    let (rss, _) = rss_kb().unwrap_or((0, 0));
    println!(
        "SOAK_TONG\tsecs={secs}\telapsed_s={elapsed:.1}\tbatches={batches}\tbatches_s={:.2}\ttasks={tasks}\ttasks_s={:.0}",
        batches as f64 / elapsed,
        tasks as f64 / elapsed
    );
    println!(
        "SOAK_WALL\tcpu_w1_p50={:.0}\tcpu_w1_p95={:.0}\tcpu_w2_p50={:.0}\tcpu_w2_p95={:.0}\tcpu_w4_p50={:.0}\tcpu_w4_p95={:.0}\tk10_w1_p50={:.0}\tk10_w1_p95={:.0}\tk10_w4_p50={:.0}\tk10_w4_p95={:.0}",
        pct(&mut cpu_w1, 0.50),
        pct(&mut cpu_w1, 0.95),
        pct(&mut cpu_w2, 0.50),
        pct(&mut cpu_w2, 0.95),
        pct(&mut cpu_w4, 0.50),
        pct(&mut cpu_w4, 0.95),
        pct(&mut k10_w1, 0.50),
        pct(&mut k10_w1, 0.95),
        pct(&mut k10_w4, 0.50),
        pct(&mut k10_w4, 0.95),
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
