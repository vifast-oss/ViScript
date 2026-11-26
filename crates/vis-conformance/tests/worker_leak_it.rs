// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-3b/W5 (P2)** — test đo rò thread/fd/RSS của worker pool trong **process riêng**
//! (tách khỏi `lib_api_it` để không đếm thread của test khác khi chạy `cargo test` song song).
//!
//! **ST-316d (`#198`)** — `stress_dai_5x10k_cancel_budget_print_cross` dời từ
//! `worker_api_it/vs3_worker.rs` về đây (cùng lý do: ca đo `Threads:` toàn tiến trình) kèm
//! vòng chờ OS thu hồi thread ~500 ms trước khi so sánh; **0 binary R8 mới**.

use std::sync::Arc;
use std::time::{Duration, Instant};

use vis_core::RunOutput;
use vis_runtime::rt::fee::SharedBudget;
use vis_runtime::worker::{EnvMap, SendValue, WorkerError, WorkerPool};
use vis_syntax::{lex, parse, Mode};

/// Hai ca trong binary đều đo tài nguyên **toàn tiến trình** (`Threads:`/fd/RSS) ⇒ chạy tuần tự
/// trong cùng binary: `cargo test` mặc định chạy song song, còn nextest thì mỗi test một tiến
/// trình (khoá vô hại). Không có khoá, ca này có thể đếm thread pool của ca kia (đúng lớp `#198`).
static KHOA_DO: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn khoa_do() -> std::sync::MutexGuard<'static, ()> {
    KHOA_DO.lock().unwrap_or_else(|e| e.into_inner())
}

fn tien_trinh(src: &str) -> (vis_hir::Hir, vis_hir::Symbols, vis_hir::TypeTable) {
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
    (h.hir, h.symbols, h.types)
}

fn parallel(
    src: &str,
    hir: &vis_hir::Hir,
    s: &vis_hir::Symbols,
    t: &vis_hir::TypeTable,
    w: usize,
) -> RunOutput {
    vis_interp::run_workers_parallel(src, hir, s, t, &[], w)
}

fn dem_thread() -> Option<usize> {
    std::fs::read_dir("/proc/self/task")
        .ok()
        .map(|it| it.filter_map(Result::ok).count())
}

fn dem_fd() -> Option<usize> {
    std::fs::read_dir("/proc/self/fd")
        .ok()
        .map(|it| it.filter_map(Result::ok).count())
}

fn rss_kb() -> Option<u64> {
    let s = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            return rest.split_whitespace().next()?.parse().ok();
        }
    }
    None
}

const NHO: &str = r#"async fn t(n: int) -> int:
    n * 2

fn main():
    var s = 0
    var i = 0
    while i < 8:
        let x = spawn t(i)
        s = s + await x
        i = i + 1
    print(s)
"#;

#[test]
fn muoi_vong_threads_fd_rss_khong_ro() {
    let _khoa = khoa_do();
    let (hir, syms, tys) = tien_trinh(NHO);
    let t0 = dem_thread();
    let f0 = dem_fd();
    let r0 = rss_kb();
    let t_bat = Instant::now();
    for vong in 0..10 {
        let out = parallel(NHO, &hir, &syms, &tys, 4);
        assert_eq!(out.exit, 0, "vòng {vong}: {:?}", out.diagnostics);
        assert_eq!(out.stdout, "56\n");
    }
    // Thread của pool có thể chưa được OS thu hồi ngay sau join (nhất là khi gate chạy tải
    // cao). Chờ tối đa 500 ms cho về mức nền trước khi kết luận rò; nếu vẫn cao hơn mới fail.
    if let Some(t0) = t0 {
        let mut t1 = dem_thread().unwrap_or(t0);
        let han = Instant::now() + std::time::Duration::from_millis(500);
        while t1 > t0 + 1 && Instant::now() < han {
            std::thread::sleep(std::time::Duration::from_millis(10));
            t1 = dem_thread().unwrap_or(t1);
        }
        assert!(t1 <= t0 + 1, "rò thread: {t0} -> {t1}");
    }
    if let (Some(f0), Some(f1)) = (f0, dem_fd()) {
        assert!(f1 <= f0 + 4, "rò fd: {f0} -> {f1}");
    }
    if let (Some(r0), Some(r1)) = (r0, rss_kb()) {
        assert!(r1 <= r0 + 65_536, "RSS tăng quá 64 MB: {r0} -> {r1}");
    }
    if std::env::var("VS3_W5_MEASURE").is_ok() {
        eprintln!(
            "MEASURE_10_VONG\tms={:.1}\tthreads={:?}\tfd={:?}\trss_kb={:?}",
            t_bat.elapsed().as_secs_f64() * 1000.0,
            dem_thread(),
            dem_fd(),
            rss_kb()
        );
    }
}

/// Env snapshot tối thiểu như `WorkerPool` cần (bản dời từ `worker_api_it/vs3_worker.rs`).
fn env() -> EnvMap {
    let mut m = EnvMap::new();
    m.insert("base".into(), SendValue::Int(10));
    m.insert(
        "data".into(),
        SendValue::List((0..8).map(SendValue::Int).collect()),
    );
    m
}

/// Đọc một khoá `key` (ví dụ `VmRSS:`) từ `/proc/self/status`.
fn status_kb(key: &str) -> u64 {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix(key) {
            return rest
                .split_whitespace()
                .next()
                .and_then(|x| x.parse().ok())
                .unwrap_or(0);
        }
    }
    0
}

/// Stress dài: **5 vòng × 10k task** (50k task/pha), print/cancel/budget/cross-write;
/// đo thời gian/RSS/thread theo vòng; join trả về, không deadlock, không rò worker thread.
#[test]
fn stress_dai_5x10k_cancel_budget_print_cross() {
    let _khoa = khoa_do();
    const ROUNDS: usize = 5;
    const N: usize = 10_000;
    let t0 = Instant::now();
    let rss0 = status_kb("VmRSS:");
    let threads0 = status_kb("Threads:");
    let mut print_s = 0.0;
    let mut cancel_s = 0.0;
    let mut budget_s = 0.0;
    let mut cross_s = 0.0;
    let mut rss_rounds = Vec::with_capacity(ROUNDS);

    for round in 0..ROUNDS {
        let t_round = Instant::now();

        // Print merge 10k: W=1 == W=4 byte-identical, outcomes đủ.
        let t_phase = Instant::now();
        let mut outs = Vec::new();
        for w in [1usize, 4] {
            let mut pool = WorkerPool::new(w, env()).expect("pool print");
            for i in 0..N {
                pool.spawn(move |ctx| {
                    ctx.print_line(&format!("t{i}"));
                    Ok(SendValue::Unit)
                })
                .unwrap();
            }
            let r = pool.join().expect("join print");
            assert_eq!(r.outcomes.len(), N);
            outs.push(r.output);
        }
        assert_eq!(outs[0], outs[1], "vòng {round}: print W=1/W=4");
        assert_eq!(outs[0].lines().count(), N);
        print_s += t_phase.elapsed().as_secs_f64();

        // Cancel nửa 10k ở W=1/W=4; task tự kiểm cancel trước/sau sleep.
        let t_phase = Instant::now();
        for w in [1usize, 4] {
            let mut pool = WorkerPool::new(w, env()).expect("pool cancel");
            let mut handles = Vec::with_capacity(N);
            for i in 0..N {
                let h = pool
                    .spawn(move |ctx| {
                        ctx.check_cancel()?;
                        std::thread::sleep(Duration::from_micros(200));
                        ctx.check_cancel()?;
                        if i % 2 == 0 {
                            ctx.print_line(&format!("c{i}"));
                        }
                        Ok(SendValue::Unit)
                    })
                    .unwrap();
                handles.push(h);
            }
            for h in handles.iter().step_by(2) {
                h.cancel();
            }
            let r = pool.join().expect("join cancel");
            assert_eq!(r.outcomes.len(), N);
            // Stress chỉ cần join đủ + không deadlock; ca cancel ngữ nghĩa nằm ở `vs3_cancel`.
            // Không bắt buộc có task bị huỷ: dưới tải, task ngắn có thể xong trước khi cancel.
            assert!(
                r.outcomes.iter().filter(|o| o.cancelled).count() <= N / 2,
                "vòng {round}: không thể huỷ quá nửa ở W={w}"
            );
        }
        cancel_s += t_phase.elapsed().as_secs_f64();

        // Budget cạn: 10k task cùng pool 5k đơn vị; không deadlock, accounting đúng.
        let t_phase = Instant::now();
        let b = Arc::new(SharedBudget::new((N / 2) as u64, 256));
        let mut pool = WorkerPool::with_budget(4, env(), Arc::clone(&b)).expect("pool budget");
        for _ in 0..N {
            pool.spawn(|ctx| {
                ctx.charge(1)?;
                Ok(SendValue::Unit)
            })
            .unwrap();
        }
        let r = pool.join().expect("join budget");
        let snap = r.budget.expect("budget");
        assert_eq!(r.outcomes.len(), N);
        assert_eq!(snap.used, (N / 2) as i64, "vòng {round}: dùng đúng pool");
        assert_eq!(snap.remaining, 0, "vòng {round}: cạn đúng 0");
        assert_eq!(snap.used + snap.remaining, snap.limit);
        assert!(snap.exhausted);
        budget_s += t_phase.elapsed().as_secs_f64();

        // Cross-write 10k task W=4: refused, không deadlock, đủ danh sách task.
        let t_phase = Instant::now();
        let mut pool = WorkerPool::new(4, env()).expect("pool cross");
        for i in 0..N {
            pool.spawn(move |ctx| {
                ctx.set("counter", SendValue::Int(i as i64));
                Ok(SendValue::Unit)
            })
            .unwrap();
        }
        match pool.join() {
            Err(WorkerError::CrossWrite { vars, tasks }) => {
                assert_eq!(vars, vec!["counter".to_string()]);
                assert_eq!(tasks.len(), N);
            }
            other => panic!("vòng {round}: stress cross-write phải refused: {other:?}"),
        }
        cross_s += t_phase.elapsed().as_secs_f64();

        let round_s = t_round.elapsed().as_secs_f64();
        let rss = status_kb("VmRSS:");
        let threads = status_kb("Threads:");
        rss_rounds.push(rss);
        println!(
            "STRESS_DAI\tround={round}\ttasks={N}\tsecs={round_s:.2}\trss_kb={rss}\tthreads={threads}"
        );
    }

    let rss1 = status_kb("VmRSS:");
    let hwm = status_kb("VmHWM:");
    // **ST-316d (`#198`)** — OS có thể chưa thu hồi thread của pool ngay sau `join` (nhất là khi
    // gate chạy tải cao). Chờ tối đa ~500 ms cho `Threads:` về mức nền; binary cách ly nên không
    // đếm thread của test khác. Chỉ fail nếu vẫn cao hơn sau khi chờ.
    let threads1 = {
        let mut t = status_kb("Threads:");
        let han = Instant::now() + Duration::from_millis(500);
        while t > threads0 + 1 && Instant::now() < han {
            std::thread::sleep(Duration::from_millis(10));
            t = status_kb("Threads:");
        }
        t
    };
    let secs = t0.elapsed().as_secs_f64();
    println!(
        "STRESS_DAI_SUMMARY\trounds={ROUNDS}\ttasks_per_round={N}\ttotal_tasks={}\tsecs={secs:.2}\tprint_s={print_s:.2}\tcancel_s={cancel_s:.2}\tbudget_s={budget_s:.2}\tcross_s={cross_s:.2}\trss0_kb={rss0}\trss1_kb={rss1}\thwm_kb={hwm}\tthreads0={threads0}\tthreads1={threads1}\trss_rounds={rss_rounds:?}",
        ROUNDS * N
    );
    assert!(
        threads1 <= threads0 + 1,
        "worker thread rò: {threads0} -> {threads1}"
    );
    assert!(
        rss1.saturating_sub(rss0) < 256 * 1024,
        "RSS tăng quá 256 MB (nghi rò): {rss0} -> {rss1}"
    );
    assert!(secs < 120.0, "stress quá chậm: {secs:.2}s");
}
