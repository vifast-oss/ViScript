// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-267 / VD-251 — LÁT 1**: source → typecheck → **đường frame thật** của cả
//! hai engine (interp + VM). Không model/harness độc lập: dùng `Interp`/`Vm` thật
//! qua API test-only (`vis_interp::frame`, `vis_vm::vm_frame`). Runtime mặc định
//! (`vis run`) không bị chạm.
use vis_interp::frame::FrameOutput;
use vis_interp::frame::{run_frame_for_test as run_i, FrameConfig};
use vis_interp::rt::{DiagSnapshot, FakeEvent};
use vis_interp::Value;
use vis_syntax::{lex, parse, Mode};
use vis_vm::vm_frame::run_frame_for_test as run_v;

fn tien_trinh(src: &str) -> (vis_types::Hir, vis_types::Symbols, vis_types::TypeTable) {
    let p = parse(src, Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse: {:?}", p.diagnostics);
    let l = lex(src, Mode::Script);
    let h = vis_types::lower(&p.cst, &l);
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    (h.hir, h.symbols, h.types)
}

fn cfg(sources: &[(&str, u64)], events: Vec<FakeEvent>) -> FrameConfig {
    FrameConfig {
        fake_sources: sources.iter().map(|(a, b)| (a.to_string(), *b)).collect(),
        events,
        ..Default::default()
    }
}

fn ev(seq: u64, key: u64, gen: u64, v: i64) -> FakeEvent {
    FakeEvent {
        seq,
        key,
        gen,
        value: Value::Int(v),
    }
}

fn chay(src: &str, c: FrameConfig) -> (FrameOutput, FrameOutput) {
    let (hir, syms, tys) = tien_trinh(src);
    let i = run_i(src, &hir, &syms, &tys, &[], c.clone());
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    let v = run_v(&m, src, &[], c);
    // Đồng nhất stdout / chẩn đoán / trace scheduler / snapshot ownership.
    assert_eq!(i.stdout, v.stdout, "interp↔VM stdout lệch");
    assert_eq!(
        i.diagnostics.len(),
        v.diagnostics.len(),
        "số chẩn đoán lệch"
    );
    for (a, b) in i.diagnostics.iter().zip(&v.diagnostics) {
        assert_eq!(a.code.short, b.code.short, "mã chẩn đoán lệch");
        assert_eq!(a.message, b.message, "nội dung chẩn đoán lệch");
        assert_eq!((a.line, a.col), (b.line, b.col), "vị trí chẩn đoán lệch");
    }
    assert_eq!(
        i.log, v.log,
        "trace scheduler lệch (tie-break không tất định?)"
    );
    assert_eq!(i.snapshot, v.snapshot, "snapshot ownership lệch");
    (i, v)
}

/// Chương trình chính: A treo tại `await ext_src()`, B chạy, A resume; có gọi hàm
/// lồng `add1(await a)` (await trong biểu thức lồng) và local `y` giữ qua resume.
const P1: &str = r#"async fn worker() -> int:
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

fn p1_cfg(extra: Vec<FakeEvent>) -> FrameConfig {
    let mut events = vec![ev(0, 1, 0, 40)];
    events.extend(extra);
    cfg(&[("ext_src", 1)], events)
}

#[test]
fn c1_a_pending_b_runs_a_resumes() {
    let (i, _) = chay(P1, p1_cfg(vec![]));
    assert_eq!(i.stdout, "A:pre\nB:run\nA:post\n48\n5\n");
    // A treo (wait Event key=1) trước khi B chạy; B xong ⇒ wake A; A xong ⇒ wake main.
    assert!(i.log.iter().any(|l| l == "wait:1:Event { key: 1, gen: 0 }"));
    assert!(i.log.contains(&"wake:1".to_string()));
    assert!(i.log.contains(&"wake:0".to_string()));
    let pre = i
        .log
        .iter()
        .position(|l| l == "wait:1:Event { key: 1, gen: 0 }")
        .unwrap();
    let wake = i.log.iter().position(|l| l == "wake:1").unwrap();
    assert!(pre < wake, "wait phải trước wake");
}

#[test]
fn c2_locals_survive_and_no_repeated_side_effect() {
    let (i, _) = chay(P1, p1_cfg(vec![]));
    // 48 = add1(7 + 40): local `y = 7` sống qua suspend.
    assert_eq!(i.stdout.matches("A:pre").count(), 1, "A:pre không được lặp");
    assert_eq!(i.stdout.matches("B:run").count(), 1);
    assert_eq!(i.stdout.matches("A:post").count(), 1);
    assert!(i.stdout.contains("48\n"));
}

#[test]
fn c3_nested_call_and_await_inside_nested_expression() {
    // `print(add1(await a))` — await nằm trong đối số của hàm lồng `add1`.
    let (i, _) = chay(P1, p1_cfg(vec![]));
    assert!(
        i.stdout.ends_with("48\n5\n"),
        "kết quả hàm lồng: {:?}",
        i.stdout
    );
}

/// Lỗi thật đi `Trap → VS0506`; hai task cùng `await` một task lỗi ⇒ lần hai trả cache.
const P2: &str = r#"async fn bad() -> int:
    print("bad:run")
    1 % 0

async fn waiter(t: Task[int]) -> int:
    let x = await t
    x + 1

fn main():
    let b = spawn bad()
    let w1 = spawn waiter(b)
    let w2 = spawn waiter(b)
    print(await w2)
"#;

#[test]
fn c4_real_error_vs0506_and_cached_on_second_await() {
    let (i, v) = chay(P2, cfg(&[], vec![]));
    assert_eq!(i.stdout, "bad:run\n", "thân task lỗi chạy đúng 1 lần");
    assert_eq!(v.stdout, i.stdout);
    assert_eq!(i.diagnostics.len(), 1);
    let d = &i.diagnostics[0];
    assert_eq!(
        d.code.short, "VS0506",
        "lỗi thân task phải là VS0506: {:?}",
        d
    );
    // w2 nhận **cache** lỗi của `bad` (VS0502 gốc nằm trong message) ⇒ hai lần await
    // cùng một task đều trả cùng error, không chạy lại thân.
    assert!(d.message.contains("bad"), "message: {}", d.message);
    assert!(
        d.message.contains("VS0502"),
        "message phải giữ mã gốc: {}",
        d.message
    );
}

/// Task không ai await vẫn được join cuối — không auto-cancel khi main kết thúc.
const P3: &str = r#"async fn lonely() -> int:
    print("lonely")
    9

fn main():
    let t = spawn lonely()
    print("main")
"#;

#[test]
fn c5_unawaited_task_still_final_joined() {
    let (i, _) = chay(P3, cfg(&[], vec![]));
    assert_eq!(
        i.stdout, "main\nlonely\n",
        "task không await vẫn chạy ở final join"
    );
    assert!(i.diagnostics.is_empty());
}

#[test]
fn c6_duplicate_and_stale_wake_do_not_rerun() {
    // Event trùng (seq1, cùng key/gen) và event stale (gen=99) phải bị bỏ.
    let extra = vec![ev(1, 1, 0, 99), ev(2, 1, 99, 7)];
    let (i, _) = chay(P1, p1_cfg(extra));
    assert_eq!(
        i.stdout, "A:pre\nB:run\nA:post\n48\n5\n",
        "giá trị vẫn là 40"
    );
    assert!(
        i.log.contains(&"duplicate:1".to_string()),
        "log: {:?}",
        i.log
    );
    assert!(i.log.contains(&"stale:1".to_string()), "log: {:?}", i.log);
    assert_eq!(
        i.log.iter().filter(|l| **l == "wake:1").count(),
        1,
        "chỉ wake 1 lần"
    );
}

#[test]
fn c7_reclaim_verified_by_diag_snapshot() {
    let (i, v) = chay(P1, p1_cfg(vec![]));
    let z = DiagSnapshot {
        live_tasks: 0,
        frames: 0,
        waiters: 0,
        ready: 0,
        ..Default::default()
    };
    assert_eq!(
        i.snapshot, z,
        "interp: mọi task đã join + hết handle ⇒ thu hồi"
    );
    assert_eq!(v.snapshot, z, "VM: cùng kỳ vọng ownership");
    assert_eq!(
        i.log.iter().filter(|l| l.starts_with("reclaim:")).count(),
        3
    );
}

/// Tiêu chí 8 — fixture cũ được **phân loại**, không đổi golden: chạy lại một fixture
/// async có sẵn qua đường **mặc định** (không frame) và so khớp `.out` nguyên văn.
#[test]
fn c8_existing_fixtures_still_pass_on_default_path() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test/sem/async");
    for name in [
        "await-co-ban",
        "await-hai-lan-cache",
        "spawn-nhieu-join-cuoi",
    ] {
        let src = std::fs::read_to_string(root.join(format!("{name}.vis"))).expect("đọc fixture");
        let want = std::fs::read_to_string(root.join(format!("{name}.out"))).expect("đọc golden");
        let (hir, syms, tys) = tien_trinh(&src);
        let o = vis_interp::run(&src, &hir, &syms, &tys, &[]);
        assert!(o.is_clean(), "{name}: {:?}", o.diagnostics);
        assert_eq!(o.stdout, want, "{name}: golden bị đổi?");
        let m = vis_bytecode::compile(&hir, &syms, &src).expect("compile");
        let vo = vis_vm::run(&m, &src, &[]);
        assert_eq!(vo.stdout, want, "VM {name}: golden bị đổi?");
    }
}
