// SPDX-License-Identifier: MIT OR Apache-2.0
//! ST-267 / VD-251 — LÁT 1 (interp): source → tyck → đường frame thật.
use vis_interp::frame::{run_frame_for_test, FrameConfig};
use vis_syntax::{lex, parse, Mode};

fn frame_run(src: &str, cfg: FrameConfig) -> vis_interp::frame::FrameOutput {
    let p = parse(src, Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse: {:?}", p.diagnostics);
    let l = lex(src, Mode::Script);
    let h = vis_types::lower(&p.cst, &l);
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    run_frame_for_test(src, &h.hir, &h.symbols, &h.types, &[], cfg)
}

#[test]
fn smoke() {
    let src = r#"async fn worker() -> int:
    let y = 7
    print("A:pre")
    let x = await ext_src()
    print("A:post")
    y + x

async fn runner() -> int:
    print("B:run")
    5

async fn ext_src() -> int:
    0

fn add1(n: int) -> int:
    n + 1

fn main():
    let a = spawn worker()
    let b = spawn runner()
    print(add1(await a))
    print(await b)
"#;
    let cfg = FrameConfig {
        fake_sources: vec![("ext_src".into(), 1)],
        events: vec![vis_interp::rt::FakeEvent {
            seq: 0,
            key: 1,
            gen: 0,
            value: vis_interp::Value::Int(40),
        }],
        ..Default::default()
    };
    let out = frame_run(src, cfg);
    eprintln!("stdout={:?}", out.stdout);
    eprintln!("diag={:?}", out.diagnostics);
    eprintln!("log={:?}", out.log);
    eprintln!("snap={:?}", out.snapshot);
    assert_eq!(out.stdout, "A:pre\nB:run\nA:post\n48\n5\n");
}
