// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-3b/W5 (P2)** — evaluator W>1 non-frame trên `WorkerPool` (CPU thuần).
//!
//! W=1 dùng runner caller (`run_workers_local`); W=2/4 dựng `Interp` riêng trong worker từ
//! `Arc<HirImage>`, snapshot top-level, output merge spawn order, budget atomic, cross-write.
//! Không host/`chan(0)`/frame: task phải lỗi rõ, không treo/không chạy sai.

use std::sync::Arc;
use std::time::Instant;

use vis_core::RunOutput;
use vis_runtime::rt::fee::SharedBudget;
use vis_syntax::{lex, parse, Mode};

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

fn local(src: &str, hir: &vis_hir::Hir, s: &vis_hir::Symbols, t: &vis_hir::TypeTable) -> RunOutput {
    vis_interp::run_workers_local(src, hir, s, t, &[])
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

fn so_sanh_w(src: &str, w: usize) -> (RunOutput, RunOutput) {
    let (hir, syms, tys) = tien_trinh(src);
    let a = local(src, &hir, &syms, &tys);
    let b = parallel(src, &hir, &syms, &tys, w);
    assert_eq!(a.stdout, b.stdout, "W=1 vs W={w}: stdout");
    assert_eq!(a.exit, b.exit, "W=1 vs W={w}: exit");
    assert_eq!(a.codes(), b.codes(), "W=1 vs W={w}: codes");
    (a, b)
}

const SPAWN_PRINT: &str = r#"async fn t(n: int) -> int:
    print(n)
    n * 2

fn main():
    var s = 0
    var i = 1
    while i <= 4:
        let x = spawn t(i)
        s = s + await x
        i = i + 1
    print(s)
"#;

#[test]
fn w2_print_byte_identical() {
    let (a, _) = so_sanh_w(SPAWN_PRINT, 2);
    assert_eq!(a.stdout, "1\n2\n3\n4\n20\n");
    assert_eq!(a.exit, 0);
}

#[test]
fn w4_print_byte_identical() {
    let (a, _) = so_sanh_w(SPAWN_PRINT, 4);
    assert_eq!(a.stdout, "1\n2\n3\n4\n20\n");
    assert_eq!(a.exit, 0);
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

#[test]
fn muoi_nghin_task_w2_w4_byte_identical() {
    let (hir, syms, tys) = tien_trinh(SPAWN_10K);
    let t0 = Instant::now();
    let a = local(SPAWN_10K, &hir, &syms, &tys);
    let local_ms = t0.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(a.stdout, "50005000\n", "W=1");
    assert_eq!(a.exit, 0);
    for w in [2usize, 4] {
        let t = Instant::now();
        let b = parallel(SPAWN_10K, &hir, &syms, &tys, w);
        let par_ms = t.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(a.stdout, b.stdout, "W=1 vs W={w}");
        assert_eq!(a.exit, b.exit);
        assert!(b.steps > 0, "chống xanh-giả W={w}");
        if std::env::var("VS3_W5_MEASURE").is_ok() {
            eprintln!(
                "MEASURE_10K\tW={w}\tlocal_ms={local_ms:.1}\tparallel_ms={par_ms:.1}\tspeedup={:.2}x\tthreads={:?}\tfd={:?}\trss_kb={:?}",
                local_ms / par_ms,
                dem_thread(),
                dem_fd(),
                rss_kb()
            );
        }
    }
}

const NESTED: &str = r#"async fn con(n: int) -> int:
    n * 2

async fn cha(n: int) -> int:
    let t = spawn con(n)
    (await t) + 1

fn main():
    let t = spawn cha(20)
    print(await t)
"#;

#[test]
fn nested_spawn_await_trong_worker() {
    for w in [2usize, 4] {
        let (a, b) = so_sanh_w(NESTED, w);
        assert_eq!(a.stdout, "41\n");
        assert_eq!(b.stdout, "41\n");
        assert_eq!(b.exit, 0);
    }
}

const CROSS_WRITE: &str = r#"var counter = 0

async fn tang():
    counter = counter + 1

fn main():
    let t = spawn tang()
    await t
    print(counter)
"#;

#[test]
fn cross_write_w1_shared_w2_loi_ro() {
    let (hir, syms, tys) = tien_trinh(CROSS_WRITE);
    let a = local(CROSS_WRITE, &hir, &syms, &tys);
    assert_eq!(a.exit, 0, "W=1: {:?}", a.diagnostics);
    assert_eq!(a.stdout, "1\n");
    for w in [2usize, 4] {
        let b = parallel(CROSS_WRITE, &hir, &syms, &tys, w);
        assert_eq!(b.exit, 1, "W={w} phải lỗi cross-write");
        assert!(
            b.diagnostics
                .iter()
                .any(|d| d.message.contains("ghi chéo worker")),
            "W={w}: {:?}",
            b.diagnostics
        );
    }
}

const SNAPSHOT: &str = r#"var x = 1

async fn doc() -> int:
    x

fn main():
    let t = spawn doc()
    x = 2
    print(await t)
"#;

#[test]
fn snapshot_tai_spawn_w1_khac_w2_co_chu_dinh() {
    let (hir, syms, tys) = tien_trinh(SNAPSHOT);
    let a = local(SNAPSHOT, &hir, &syms, &tys);
    assert_eq!(a.stdout, "2\n", "W=1 shared env thấy x mới");
    for w in [2usize, 4] {
        let b = parallel(SNAPSHOT, &hir, &syms, &tys, w);
        assert_eq!(b.stdout, "1\n", "W={w} snapshot tại spawn");
        assert_eq!(b.exit, 0);
    }
}

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

#[test]
fn budget_atomic_used_cong_remaining_bang_limit() {
    let (hir, syms, tys) = tien_trinh(BUDGET);
    let budget = Arc::new(SharedBudget::new(10_000, 256));
    let out = vis_interp::run_workers_parallel_with_budget(
        BUDGET,
        &hir,
        &syms,
        &tys,
        &[],
        2,
        Arc::clone(&budget),
    );
    assert_eq!(out.exit, 1, "budget phải dừng task");
    assert!(
        out.diagnostics.iter().any(|d| d.message.contains("VS0510")),
        "phải giữ mã ngân sách: {:?}",
        out.diagnostics
    );
    let snap = budget.snapshot();
    assert_eq!(snap.used + snap.remaining, snap.limit, "{snap:?}");
    assert!(snap.used > 0, "phải charge: {snap:?}");
    assert!(snap.exhausted, "phải cạn: {snap:?}");
    if std::env::var("VS3_W5_MEASURE").is_ok() {
        eprintln!(
            "MEASURE_BUDGET\tused={}\tremaining={}\tlimit={}\texhausted={}",
            snap.used, snap.remaining, snap.limit, snap.exhausted
        );
    }
}

const CHAN0: &str = r#"async fn c() -> int:
    let ch = chan[int](0)
    let _ = ch.send(1)
    1

fn main():
    let t = spawn c()
    print(await t)
"#;

#[test]
fn chan0_trong_task_bi_tu_choi_ro() {
    let (hir, syms, tys) = tien_trinh(CHAN0);
    let b = parallel(CHAN0, &hir, &syms, &tys, 2);
    assert_eq!(b.exit, 1, "chan0 phải lỗi, không treo");
    assert!(
        b.diagnostics.iter().any(|d| d.message.contains("VS0505")),
        "{:?}",
        b.diagnostics
    );
}

const HOST_FS: &str = r#"use std.fs

async fn doc() -> int:
    let _ = fs.current_dir()
    1

fn main():
    let t = spawn doc()
    print(await t)
"#;

#[test]
fn host_fs_trong_task_bi_tu_choi_ro() {
    let (hir, syms, tys) = tien_trinh(HOST_FS);
    let b = parallel(HOST_FS, &hir, &syms, &tys, 2);
    assert_eq!(b.exit, 1, "host phải lỗi, không treo");
    assert!(
        b.diagnostics.iter().any(|d| d.message.contains("VS0604")),
        "{:?}",
        b.diagnostics
    );
}

const CANCEL: &str = r#"async fn t(n: int) -> int:
    n + 1

fn main():
    let a = spawn t(1)
    let b = spawn t(2)
    print(await a)
    print(await b)
"#;

#[test]
fn cancel_pre_run_tra_loi_ro() {
    let (hir, syms, tys) = tien_trinh(CANCEL);
    let out = vis_interp::run_workers_parallel_cancel_before(CANCEL, &hir, &syms, &tys, &[], 2);
    assert_eq!(out.exit, 1, "task bị huỷ phải lỗi");
    assert!(
        out.diagnostics.iter().any(|d| d.message.contains("huỷ")),
        "{:?}",
        out.diagnostics
    );
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

const CPU_NANG: &str = r#"async fn nang(n: int) -> int:
    var s = 0
    var i = 0
    while i < n:
        s = s + i
        i = i + 1
    s

fn main():
    let a = spawn nang(20000)
    let b = spawn nang(20000)
    let c = spawn nang(20000)
    let d = spawn nang(20000)
    var s = 0
    s = s + await a
    s = s + await b
    s = s + await c
    s = s + await d
    print(s)
"#;

#[test]
fn cpu_nang_w1_w2_w4_byte_identical() {
    let (hir, syms, tys) = tien_trinh(CPU_NANG);
    let t0 = Instant::now();
    let a = local(CPU_NANG, &hir, &syms, &tys);
    let local_ms = t0.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(a.exit, 0, "{:?}", a.diagnostics);
    for w in [2usize, 4] {
        let t = Instant::now();
        let b = parallel(CPU_NANG, &hir, &syms, &tys, w);
        let par_ms = t.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(a.stdout, b.stdout, "W=1 vs W={w}");
        assert_eq!(a.exit, b.exit);
        if std::env::var("VS3_W5_MEASURE").is_ok() {
            eprintln!(
                "MEASURE_CPU\\tW={w}\\tlocal_ms={local_ms:.1}\\tparallel_ms={par_ms:.1}\\tspeedup={:.2}x",
                local_ms / par_ms
            );
        }
    }
}

const SPAWN_10K_FINAL: &str = r#"async fn t(n: int) -> int:
    print(n)
    n + 1

fn main():
    var i = 0
    while i < 10000:
        let _x = spawn t(i)
        i = i + 1
"#;

#[test]
fn muoi_nghin_task_join_cuoi_in_spawn_order() {
    let (hir, syms, tys) = tien_trinh(SPAWN_10K_FINAL);
    let t0 = Instant::now();
    let a = local(SPAWN_10K_FINAL, &hir, &syms, &tys);
    let local_ms = t0.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(a.exit, 0, "{:?}", a.diagnostics);
    let ky_vong: String = (0..10000).map(|i| format!("{i}\n")).collect();
    assert_eq!(a.stdout, ky_vong, "W=1 output");
    for w in [2usize, 4] {
        let t = Instant::now();
        let b = parallel(SPAWN_10K_FINAL, &hir, &syms, &tys, w);
        let par_ms = t.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(a.stdout, b.stdout, "W=1 vs W={w} output");
        assert_eq!(a.exit, b.exit);
        if std::env::var("VS3_W5_MEASURE").is_ok() {
            eprintln!(
                "MEASURE_10K_FINAL\\tW={w}\\tlocal_ms={local_ms:.1}\\tparallel_ms={par_ms:.1}\\tspeedup={:.2}x",
                local_ms / par_ms
            );
        }
    }
}

const CANCEL_LONG: &str = r#"async fn nang(n: int) -> int:
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

#[test]
fn cancel_giua_chung_safe_point() {
    let (hir, syms, tys) = tien_trinh(CANCEL_LONG);
    let out =
        vis_interp::run_workers_parallel_cancel_after(CANCEL_LONG, &hir, &syms, &tys, &[], 2, 5);
    assert_eq!(out.exit, 1, "cancel phải dừng task, không trả kết quả");
    assert!(
        out.diagnostics.iter().any(|d| d.message.contains("huỷ")),
        "{:?}",
        out.diagnostics
    );
}
