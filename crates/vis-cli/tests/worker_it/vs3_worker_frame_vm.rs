// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-292 (P4a/P4b)** — parity interp↔VM cho frame + evaluator worker (W=1/2/4).

use vis_core::RunOutput;
use vis_syntax::{lex, parse, Mode};

struct Anh {
    module: vis_bytecode::Module,
    src: String,
}

fn anh(src: &str) -> Anh {
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
        module,
        src: src.to_string(),
    }
}

fn vm_w1(a: &Anh) -> RunOutput {
    vis_vm::run_async_with_full_hosts_caps(&a.module, &a.src, &[], None, None, None, None, &[])
}

fn vm_w(a: &Anh, w: usize) -> RunOutput {
    vis_vm::run_async_workers_with_full_hosts_caps(
        &a.module,
        &a.src,
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
    let a = anh(src);
    let a1 = vm_w1(&a);
    let aw = vm_w(&a, w);
    assert_eq!(a1.stdout, aw.stdout, "VM W=1 vs W={w}: stdout");
    assert_eq!(a1.exit, aw.exit, "VM W=1 vs W={w}: exit");
    assert_eq!(a1.codes(), aw.codes(), "VM W=1 vs W={w}: codes");
    aw
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
fn vm_p4a_mot_task() {
    let out = so_sanh(MOT_TASK, 4);
    assert_eq!(out.stdout, "truoc\ntask 1\n2\nsau\n");
}

#[test]
fn vm_p4a_hai_task() {
    let out = so_sanh(HAI_TASK, 4);
    assert_eq!(out.stdout, "task 1\ntask 2\n5\n");
}

#[test]
fn vm_p4a_final_join_spawn_order() {
    let out = so_sanh(KHONG_AWAIT, 4);
    assert_eq!(out.stdout, "main\ntask 1\ntask 2\n");
}

#[test]
fn vm_p4a_task_chi_chay_mot_lan() {
    let out = so_sanh(
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
    assert_eq!(out.stdout, "x\n");
}

#[test]
fn vm_p4a_cross_write_vs0509() {
    let src = r#"var x = 1

async fn ghi() -> int:
    print("t")
    x = 2
    x

fn main():
    let a = spawn ghi()
    let _ = await a
    print(x)
"#;
    let a = anh(src);
    let w4 = vm_w(&a, 4);
    assert!(w4.codes().contains(&"VS0509"), "codes: {:?}", w4.codes());
    assert_eq!(w4.stdout, "t\n", "dừng trước print(x): {:?}", w4.stdout);
}

#[test]
fn vm_p4a_w2_w4_byte_identical_20_vong() {
    for v in 0..20 {
        let a = anh(MOT_TASK);
        let w1 = vm_w1(&a);
        for w in [2usize, 4] {
            let out = vm_w(&a, w);
            assert_eq!(w1.stdout, out.stdout, "vòng {v} W={w}");
            assert_eq!(w1.exit, out.exit, "vòng {v} W={w}");
            assert_eq!(w1.codes(), out.codes(), "vòng {v} W={w}");
        }
    }
}

/// **VD-283/ST-301 (#192)** — cancel worker qua trigger frame (event `seq` của scheduler),
/// VM `--workers 4`: task bị huỷ tại safe point, không chạy tiếp `V:post`, không treo.
#[test]
fn vm_p4b_cancel_worker_qua_trigger_frame() {
    use vis_core::frame::FrameConfig;
    use vis_interp::rt::{frame_cancel_clear_triggers_for_test, frame_cancel_on_seq_for_test};
    let src = r#"async fn src1() -> int:
    0

async fn victim() -> int:
    print("V:pre")
    var i = 0
    while i < 500000:
        i = i + 1
    print("V:post")
    1

fn main():
    let a = spawn victim()
    let x = await src1()
    print("main {x}")
"#;
    let a = anh(src);
    frame_cancel_clear_triggers_for_test();
    frame_cancel_on_seq_for_test(0, 1);
    let cfg = FrameConfig {
        fake_sources: vec![("src1".into(), 1)],
        events: vec![vis_core::frame::FakeEvent {
            seq: 0,
            key: 1,
            gen: 0,
            value: vis_core::Value::Int(7),
        }],
        ..Default::default()
    };
    let fo = vis_vm::vm_frame::run_frame_workers_for_test(&a.module, &a.src, &[], 4, cfg);
    frame_cancel_clear_triggers_for_test();
    assert_eq!(fo.exit, 1, "diagnostics: {:?}", fo.diagnostics);
    assert!(
        !fo.stdout.contains("V:post"),
        "task huỷ chạy tiếp: {:?}",
        fo.stdout
    );
    assert!(
        fo.log.iter().any(|l| l == "cancel:1"),
        "thiếu cancel:1: {:?}",
        fo.log
    );
}
