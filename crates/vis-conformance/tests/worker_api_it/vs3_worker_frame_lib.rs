// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-292 (P4a)** — frame suspend/resume + evaluator worker (API thư viện).
//!
//! Tách từ `vis-cli/tests/worker_it/vs3_worker_frame.rs` (ST-313c/VD-289); hai ca CLI
//! (`--workers 0`, `--max-budget` + `--workers 4`) giữ nguyên ở crate `vis-cli`.

// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-292 (P4a/P4b)** — frame suspend/resume + evaluator worker (main frame spawn ⇒ pool).
//!
//! W=1 frame (`run_async_with_hosts_caps`) là baseline; W=2/4 dùng
//! `run_async_workers_with_hosts_caps`. Kỳ vọng: output/exit/codes byte-identical, task chỉ
//! chạy **một lần**, final join theo spawn order, cross-write `VS0509`, budget `VS0510`.

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

fn frame_w1(src: &str) -> RunOutput {
    let (hir, syms, tys) = tien_trinh(src);
    vis_interp::run_async_with_hosts_caps(src, &hir, &syms, &tys, &[], None, None, None, None, &[])
}

fn frame_w(src: &str, w: usize) -> RunOutput {
    let (hir, syms, tys) = tien_trinh(src);
    vis_interp::run_async_workers_with_hosts_caps(
        src,
        &hir,
        &syms,
        &tys,
        &[],
        None,
        None,
        None,
        None,
        &[],
        w,
        None,
        None,
    )
}

fn so_sanh(src: &str, w: usize) -> RunOutput {
    let a = frame_w1(src);
    let b = frame_w(src, w);
    assert_eq!(
        a.stdout, b.stdout,
        "W=1 vs W={w}: stdout\nW1={:?}\nW{w}={:?}",
        a.stdout, b.stdout
    );
    assert_eq!(a.exit, b.exit, "W=1 vs W={w}: exit");
    assert_eq!(a.codes(), b.codes(), "W=1 vs W={w}: codes");
    b
}

const MOT_TASK: &str = r#"async fn cong(n: int) -> int:
    print("task {n}")
    n + 1

fn main():
    print("truoc")
    let a = spawn cong(1)
    let x = await a
    print(x)
    print("sau")
"#;

const HAI_TASK: &str = r#"async fn cong(n: int) -> int:
    print("task {n}")
    n + 1

fn main():
    let a = spawn cong(1)
    let b = spawn cong(2)
    let x = await a
    let y = await b
    print(x + y)
"#;

const KHONG_AWAIT: &str = r#"async fn cong(n: int) -> int:
    print("task {n}")
    n + 1

fn main():
    let a = spawn cong(1)
    let b = spawn cong(2)
    print("main")
"#;

#[test]
fn p4a_mot_task_await_suspend_resume() {
    let out = so_sanh(MOT_TASK, 4);
    assert_eq!(out.stdout, "truoc\ntask 1\n2\nsau\n");
    assert_eq!(out.exit, 0);
}

#[test]
fn p4a_hai_task_await_dung_thu_tu() {
    let out = so_sanh(HAI_TASK, 4);
    assert_eq!(out.stdout, "task 1\ntask 2\n5\n");
    assert_eq!(out.exit, 0);
}

#[test]
fn p4a_final_join_khong_await_spawn_order() {
    let out = so_sanh(KHONG_AWAIT, 4);
    assert_eq!(out.stdout, "main\ntask 1\ntask 2\n");
    assert_eq!(out.exit, 0);
}

#[test]
fn p4a_task_chi_chay_mot_lan() {
    let out = frame_w(
        r#"async fn dem(n: int) -> int:
    print("x")
    n

fn main():
    let a = spawn dem(1)
    let _ = await a
    let _ = await a
"#,
        4,
    );
    assert_eq!(out.stdout, "x\n", "task bị chạy lại: {:?}", out.stdout);
    assert_eq!(out.exit, 0);
}

#[test]
fn p4a_worker_cross_write_vs0509() {
    let out = frame_w(
        r#"var x = 1

async fn ghi() -> int:
    print("t")
    x = 2
    x

fn main():
    let a = spawn ghi()
    let _ = await a
    print(x)
"#,
        4,
    );
    assert!(
        out.codes().contains(&"VS0509"),
        "phải VS0509, được {:?}",
        out.codes()
    );
}

const LECH_THU_TU: &str = r#"async fn cong(n: int) -> int:
    print("task {n}")
    n + 1

fn main():
    let a = spawn cong(1)
    let b = spawn cong(2)
    let y = await b
    let x = await a
    print(x + y)
"#;

#[test]
fn p4a_await_lech_thu_tu() {
    // Lưu ý (plan §3.7): await lệch thứ tự ⇒ output flush theo **điểm tiêu thụ**, W=1 frame
    // in theo thứ tự chạy task còn W>1 có thể xong trước; chỉ khoá **giá trị/exit**, không
    // khoá thứ tự in (thứ tự spawn vẫn được khoá bởi ca final join + 20 vòng).
    let a = frame_w1(LECH_THU_TU);
    let b = frame_w(LECH_THU_TU, 4);
    let dong = |s: &str| s.lines().last().unwrap_or("").to_string();
    assert_eq!(dong(&a.stdout), "5");
    assert_eq!(dong(&b.stdout), "5");
    assert_eq!(a.exit, b.exit);
    assert_eq!(a.codes(), b.codes());
}

#[test]
fn p4a_200_task_final_join() {
    let src = r#"async fn cong(n: int) -> int:
    n + 1

fn main():
    var i = 0
    while i < 200:
        let _x = spawn cong(i)
        i = i + 1
    print("main")
"#;
    let out = so_sanh(src, 4);
    assert_eq!(out.stdout, "main\n");
    assert_eq!(out.exit, 0);
}

#[test]
fn p4a_nested_spawn_trong_frame_task_chay_local() {
    let src = r#"async fn cong(n: int) -> int:
    n + 1

async fn trong(n: int) -> int:
    let b = spawn cong(n)
    let v = await b
    v + 1

fn main():
    let a = spawn trong(1)
    print(await a)
"#;
    let out = so_sanh(src, 4);
    assert_eq!(out.stdout, "3\n");
}

#[test]
fn p4b_chan0_trong_worker_vs0505() {
    let out = frame_w(
        r#"async fn tao_kenh() -> int:
    let c = chan[int](0)
    1

fn main():
    let a = spawn tao_kenh()
    let _ = await a
    print("x")
"#,
        4,
    );
    assert!(
        out.diagnostics.iter().any(|d| d.message.contains("VS0505")),
        "phải VS0505 trong message: {:?}",
        out.codes()
    );
    assert_eq!(out.exit, 1);
}

#[test]
fn p4a_w2_w4_byte_identical_20_vong() {
    for v in 0..20 {
        for w in [2usize, 4] {
            let out = so_sanh(MOT_TASK, w);
            assert_eq!(out.exit, 0, "vòng {v} W={w}");
        }
    }
}
