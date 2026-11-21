// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-3b/W5 (P3)** — parity P2 interp vs P3 VM, CPU-only non-frame W=1/2/4.
//!
//! Cùng nguồn: lower HIR cho interp, compile VBC1 cho VM. So `stdout`, `exit`, `codes`,
//! `value.show()`. Không so `steps` (đơn vị khác nhau giữa interp/VM — hợp đồng VS3-1a).

use std::sync::Arc;
use std::time::Instant;

use vis_interp::rt::fee::SharedBudget;
use vis_interp::RunOutput;
use vis_syntax::{lex, parse, Mode};

fn anh(
    src: &str,
) -> (
    vis_types::Hir,
    vis_types::Symbols,
    vis_types::TypeTable,
    vis_bytecode::Module,
) {
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
    let m = vis_bytecode::compile(&h.hir, &h.symbols, src).expect("compile");
    (h.hir, h.symbols, h.types, m)
}

fn interp_w(
    src: &str,
    a: &(
        vis_types::Hir,
        vis_types::Symbols,
        vis_types::TypeTable,
        vis_bytecode::Module,
    ),
    w: usize,
) -> RunOutput {
    if w == 1 {
        vis_interp::run_workers_local(src, &a.0, &a.1, &a.2, &[])
    } else {
        vis_interp::run_workers_parallel(src, &a.0, &a.1, &a.2, &[], w)
    }
}

fn vm_w(
    src: &str,
    a: &(
        vis_types::Hir,
        vis_types::Symbols,
        vis_types::TypeTable,
        vis_bytecode::Module,
    ),
    w: usize,
) -> RunOutput {
    if w == 1 {
        vis_vm::run_workers_local_vm(&a.3, src, &[])
    } else {
        vis_vm::run_workers_parallel_vm(&a.3, src, &[], w)
    }
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

fn parity(src: &str, w: usize) -> (RunOutput, RunOutput) {
    let a = anh(src);
    let t = Instant::now();
    let i = interp_w(src, &a, w);
    let ti = t.elapsed().as_secs_f64() * 1000.0;
    let t = Instant::now();
    let v = vm_w(src, &a, w);
    let tv = t.elapsed().as_secs_f64() * 1000.0;
    so_khop(&i, &v, &format!("W={w}"));
    if std::env::var("VS3_W5_MEASURE").is_ok() {
        eprintln!(
            "MEASURE_PARITY\tW={w}\tinterp_ms={ti:.1}\tvm_ms={tv:.1}\tvm/interp={:.2}x",
            tv / ti
        );
    }
    (i, v)
}

const CPU: &str = r#"async fn nang(n: int) -> int:
    var s = 0
    var i = 0
    while i < n:
        s = s + i
        i = i + 1
    s

fn main():
    let a = spawn nang(5000)
    let b = spawn nang(5000)
    let c = spawn nang(5000)
    let d = spawn nang(5000)
    var s = 0
    s = s + await a
    s = s + await b
    s = s + await c
    s = s + await d
    print(s)
"#;

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

const NESTED: &str = r#"async fn con(n: int) -> int:
    n * 2

async fn cha(n: int) -> int:
    let t = spawn con(n)
    (await t) + 1

fn main():
    let t = spawn cha(20)
    print(await t)
"#;

const CHAN0: &str = r#"async fn c() -> int:
    let ch = chan[int](0)
    let _ = ch.send(1)
    1

fn main():
    let t = spawn c()
    print(await t)
"#;

const HOST_FS: &str = r#"use std.fs

async fn doc() -> int:
    let _ = fs.current_dir()
    1

fn main():
    let t = spawn doc()
    print(await t)
"#;

#[test]
fn cpu_w1_w2_w4_parity() {
    for w in [1usize, 2, 4] {
        let (i, _) = parity(CPU, w);
        assert_eq!(i.exit, 0, "{:?}", i.diagnostics);
        assert_eq!(i.stdout, "49990000\n");
    }
}

#[test]
fn muoi_nghin_task_parity() {
    for w in [1usize, 2, 4] {
        let (i, _) = parity(SPAWN_10K, w);
        assert_eq!(i.exit, 0, "{:?}", i.diagnostics);
        assert_eq!(i.stdout, "50005000\n");
    }
}

#[test]
fn nested_spawn_await_parity() {
    for w in [1usize, 2, 4] {
        let (i, _) = parity(NESTED, w);
        assert_eq!(i.exit, 0, "{:?}", i.diagnostics);
        assert_eq!(i.stdout, "41\n");
    }
}

#[test]
fn chan0_bi_tu_choi_parity() {
    for w in [1usize, 2, 4] {
        let (i, v) = parity(CHAN0, w);
        assert_eq!(i.exit, 1);
        assert_eq!(i.codes(), vec!["VS0506"]);
        assert!(
            i.diagnostics[0].message.contains("VS0505"),
            "{:?}",
            i.diagnostics
        );
        assert_eq!(v.exit, 1);
        assert!(
            v.diagnostics[0].message.contains("VS0505"),
            "{:?}",
            v.diagnostics
        );
    }
}

#[test]
fn host_fs_bi_tu_choi_parity() {
    for w in [2usize, 4] {
        let (i, v) = parity(HOST_FS, w);
        assert_eq!(i.exit, 1);
        assert!(
            i.diagnostics[0].message.contains("VS0604"),
            "{:?}",
            i.diagnostics
        );
        assert_eq!(v.exit, 1);
        assert!(
            v.diagnostics[0].message.contains("VS0604"),
            "{:?}",
            v.diagnostics
        );
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
fn budget_vm_used_cong_remaining_bang_limit() {
    let a = anh(BUDGET);
    let budget = Arc::new(SharedBudget::new(10_000, 256));
    let out =
        vis_vm::run_workers_parallel_vm_with_budget(&a.3, BUDGET, &[], 2, Arc::clone(&budget));
    assert_eq!(out.exit, 1, "budget phải dừng");
    assert!(
        out.diagnostics
            .iter()
            .any(|d| d.message.contains("VS0501-BUDGET")),
        "{:?}",
        out.diagnostics
    );
    let snap = budget.snapshot();
    assert_eq!(snap.used + snap.remaining, snap.limit, "{snap:?}");
    assert!(snap.used > 0);
    assert!(snap.exhausted);
    if std::env::var("VS3_W5_MEASURE").is_ok() {
        eprintln!(
            "MEASURE_BUDGET_VM\tused={}\tremaining={}\tlimit={}\texhausted={}",
            snap.used, snap.remaining, snap.limit, snap.exhausted
        );
    }
}

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

#[test]
fn cancel_vm_giua_chung() {
    let a = anh(CANCEL);
    let out = vis_vm::run_workers_parallel_vm_cancel_after(&a.3, CANCEL, &[], 2, 5);
    assert_eq!(out.exit, 1, "cancel phải dừng");
    assert!(
        out.diagnostics.iter().any(|d| d.message.contains("huỷ")),
        "{:?}",
        out.diagnostics
    );
}

const CROSS: &str = r#"var counter = 0

async fn tang():
    counter = counter + 1

fn main():
    let t = spawn tang()
    await t
    print(counter)
"#;

#[test]
fn cross_write_vm_voi_w1_w4() {
    let a = anh(CROSS);
    let x1 = vm_w(CROSS, &a, 1);
    assert_eq!(x1.exit, 0, "{:?}", x1.diagnostics);
    assert_eq!(x1.stdout, "1\n");
    for w in [2usize, 4] {
        let x = vm_w(CROSS, &a, w);
        assert_eq!(x.exit, 1, "W={w} phải lỗi");
        assert!(
            x.diagnostics.iter().any(|d| d.message.contains("ghi chéo")),
            "W={w}: {:?}",
            x.diagnostics
        );
    }
}

#[test]
fn vm_khong_tao_them_thread_sau_khi_join() {
    let a = anh(CPU);
    let before = std::fs::read_dir("/proc/self/task")
        .map(|it| it.filter_map(Result::ok).count())
        .unwrap_or(0);
    let _ = vm_w(CPU, &a, 4);
    let after = std::fs::read_dir("/proc/self/task")
        .map(|it| it.filter_map(Result::ok).count())
        .unwrap_or(before);
    assert!(after <= before + 1, "rò thread: {before}->{after}");
}

const PRINT_ORDER: &str = r#"async fn t(n: int) -> int:
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
fn print_merge_parity() {
    for w in [1usize, 2, 4] {
        let (i, v) = parity(PRINT_ORDER, w);
        assert_eq!(i.exit, 0, "{:?}", i.diagnostics);
        assert_eq!(i.stdout, "1\n2\n3\n4\n20\n", "W={w} interp");
        assert_eq!(v.stdout, i.stdout, "W={w} VM");
    }
}
