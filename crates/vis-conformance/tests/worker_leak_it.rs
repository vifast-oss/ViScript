// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-3b/W5 (P2)** — test đo rò thread/fd/RSS của worker pool trong **process riêng**
//! (tách khỏi `lib_api_it` để không đếm thread của test khác khi chạy `cargo test` song song).

use std::time::Instant;

use vis_core::RunOutput;
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
