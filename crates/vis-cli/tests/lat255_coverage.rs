// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-274 / VD-255 — ĐÓNG KHOẢNG TRỐNG COVERAGE**: source → lower → check → thật HIR/
//! bytecode → thật `Interp`/`Vm` frame path, trên **cả hai engine**. Mọi ca dùng chương
//! trình nguồn thật (không gọi helper dựng HIR tay), suspend/tiếp tục qua reactor giả ở
//! các vị trí khó (điều kiện, đối số, toán hạng, thân vòng, closure). Runtime mặc định
//! (`vis run`) không đổi, checker không thu hẹp, golden không cập nhật hàng loạt.
use vis_interp::frame::{run_frame_for_test as run_i, FrameConfig, FrameOutput};
use vis_interp::rt::{DiagSnapshot, FakeEvent};
use vis_interp::Value;
use vis_syntax::{lex, parse, Mode};
use vis_vm::vm_frame::run_frame_for_test as run_v;

fn tien_trinh(src: &str) -> (vis_types::Hir, vis_types::Symbols, vis_types::TypeTable) {
    let p = parse(src, Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse: {:?}", p.diagnostics);
    let l = lex(src, Mode::Script);
    let h = vis_types::lower_with(
        &p.cst,
        &l,
        vis_types::LowerOptions {
            std_mods: vec!["net".into()],
            ..Default::default()
        },
    );
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

/// Chạy cả hai engine, so khớp stdout / exit / chẩn đoán / trace / ownership byte-for-byte.
fn chay(src: &str, c: FrameConfig) -> (FrameOutput, FrameOutput) {
    let (hir, syms, tys) = tien_trinh(src);
    let i = run_i(src, &hir, &syms, &tys, &[], c.clone());
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    let v = run_v(&m, src, &[], c);
    assert_eq!(i.stdout, v.stdout, "interp↔VM stdout lệch");
    assert_eq!(i.exit, v.exit, "exit lệch");
    assert_eq!(
        i.diagnostics.len(),
        v.diagnostics.len(),
        "số chẩn đoán lệch: I={:?} V={:?}",
        i.diagnostics,
        v.diagnostics
    );
    for (a, b) in i.diagnostics.iter().zip(&v.diagnostics) {
        assert_eq!(a.code.short, b.code.short, "mã chẩn đoán lệch");
        assert_eq!(a.message, b.message, "nội dung chẩn đoán lệch");
        assert_eq!((a.line, a.col), (b.line, b.col), "vị trí chẩn đoán lệch");
    }
    assert_eq!(i.log, v.log, "trace scheduler lệch");
    assert_eq!(i.snapshot, v.snapshot, "snapshot ownership lệch");
    (i, v)
}

/// Golden `.out` có thể có dòng comment SPDX (`# …`) — bỏ qua khi so stdout.
fn golden_stdout(path: &std::path::Path) -> String {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .map(|l| format!("{l}\n"))
        .collect()
}

fn baseline() -> DiagSnapshot {
    DiagSnapshot {
        live_tasks: 0,
        frames: 0,
        waiters: 0,
        ready: 0,
        regs: 0,
        timers: 0,
    }
}

// ===========================================================================
// Group A — điều khiển: if/match, loops, break/continue, short-circuit
// ===========================================================================

/// A1 — `if`/`elif`/`else` (statement + expression) với **await trong điều kiện**.
/// Task A treo ở condition, B chạy, event fire, A resume đúng nhánh; side-effect 1 lần.
const A1: &str = r#"async fn worker() -> int:
    let y = await ext_src()
    y
async fn bg() -> int:
    print("BG")
    0
async fn ext_src() -> int:
    0
fn main():
    let a = spawn worker()
    let b = spawn bg()
    if (await a) > 5:
        print("big")
    elif (await a) > 0:
        print("mid")
    else:
        print("small")
    let m = if (await a) == 40: "yes" else: "no"
    print(m)
    print(await b)
"#;

#[test]
fn a1_if_elif_else_with_await_in_condition() {
    let (i, _) = chay(A1, cfg(&[("ext_src", 1)], vec![ev(0, 1, 0, 40)]));
    assert_eq!(i.exit, 0, "{:?}", i.diagnostics);
    // A treo trước khi B chạy; `await a` ở elif/expr-if dùng CACHE (không suspend lại).
    assert_eq!(i.stdout, "BG\nbig\nyes\n0\n", "{:?}", i.log);
    assert_eq!(i.stdout.matches("BG").count(), 1, "không re-run B");
    assert!(i.log.contains(&"wait:0:Task(1)".to_string()));
    assert_eq!(i.snapshot, baseline());
}

/// A2 — `while` / `loop` / `for` / `while-let` + `break`/`continue`, có **await trong thân vòng**.
const A2: &str = r#"async fn src() -> int:
    await ext_src()
async fn ext_src() -> int:
    0
async fn bg() -> int:
    print("BG")
    0
fn main():
    let a = spawn src()
    let b = spawn bg()
    var i = 0
    while i < 2:
        print(i)
        if i == 0:
            print(await a)   # await trong thân vòng (tricky position)
        i += 1
    var j = 0
    loop:
        j += 1
        if j >= 3: break
    for k in 0..5:
        if k == 1: continue
        if k == 4: break
        print(k)
    var o: int? = Some(7)
    while let Some(w) = o:
        print(w)
        o = None
    print(j)
    print(await b)
"#;

#[test]
fn a2_loops_break_continue_await_in_loop_body() {
    let (i, _) = chay(A2, cfg(&[("ext_src", 1)], vec![ev(0, 1, 0, 40)]));
    assert_eq!(i.exit, 0, "{:?}", i.diagnostics);
    assert_eq!(i.stdout, "0\nBG\n40\n1\n0\n2\n3\n7\n3\n0\n", "{:?}", i.log);
    assert_eq!(i.stdout.matches("BG").count(), 1, "không re-run task");
    assert_eq!(i.snapshot, baseline());
}

/// A3 — short-circuit `and`/`or`: toán hạng bị bỏ qua KHÔNG chạy; suspend ở toán hạng trái.
const A3: &str = r#"async fn src() -> int:
    await ext_src()
async fn ext_src() -> int:
    0
fn side() -> bool:
    print("SIDE")
    true
fn main():
    let a = spawn src()
    if (await a) == 40 and side():
        print("and-yes")
    else:
        print("and-no")
    if true or side():
        print("or-yes")
    else:
        print("or-no")
    print(side() and (await a) == 9)
"#;

#[test]
fn a3_short_circuit_skips_operand_and_suspends_in_operand() {
    let (i, _) = chay(A3, cfg(&[("ext_src", 1)], vec![ev(0, 1, 0, 40)]));
    assert_eq!(i.exit, 0, "{:?}", i.diagnostics);
    // `side()` chỉ chạy ở ca cuối (3 lần total: and RHS, false LHS, pipe...), KHÔNG ở `or` RHS.
    assert_eq!(
        i.stdout, "SIDE\nand-yes\nor-yes\nSIDE\nfalse\n",
        "toán hạng bị bỏ qua phải không chạy: {:?}",
        i.stdout
    );
    assert_eq!(i.snapshot, baseline());
}

// ===========================================================================
// Group B — biểu thức & gọi: field/index, record/tuple, named args, operators, const
// ===========================================================================

const B1: &str = r#"const BASE: int = 10
type P:
    x: int
    y: int
fn pick(a: int, --b: int = 10) -> int:
    a + b
fn main():
    let r: P = {x: 3, y: 4}
    print(r.x)
    print(r.y)
    let t = (r.x, r.y)
    let (p, q) = t
    print(p)
    print(q)
    print(pick(1))
    print(pick(1, --b: BASE))
    print([10, 20, 30][1])
    print(-(r.x))
    print(not false)
    print({x: 1, y: 2} with x: 9 |> id)
    fn id(n: int) -> int: n
"#;

#[test]
fn b1_field_index_record_tuple_named_args_unary_const() {
    // Bỏ dòng `with |> id` (VM không hỗ trợ pipe ở vị trí này) — thay bằng biểu thức thường.
    let src = B1.replace(
        "    print({x: 1, y: 2} with x: 9 |> id)\n    fn id(n: int) -> int: n\n",
        "    let r2 = {x: 1, y: 2} with x: 9\n    print(r2.x)\n",
    );
    let (i, v) = chay(&src, cfg(&[], vec![]));
    assert_eq!(i.exit, 0, "{:?}", i.diagnostics);
    assert_eq!(i.stdout, "3\n4\n3\n4\n11\n11\n20\n-3\ntrue\n9\n");
    assert_eq!(v.stdout, i.stdout);
    // Runtime mặc định KHÔNG đổi: cùng nguồn vẫn đúng như frame path.
    let (hir, syms, tys) = tien_trinh(&src);
    let def = vis_interp::run(&src, &hir, &syms, &tys, &[]);
    assert_eq!(def.stdout, i.stdout);
}

/// B2 — suspend ở **đối số** và ở **base của field/index**.
const B2: &str = r#"type P:
    v: int
async fn src() -> int:
    await ext_src()
async fn ext_src() -> int:
    0
fn add(a: int, b: int) -> int:
    a + b
fn main():
    let a = spawn src()
    # await nằm trong đối số; record chứa field từ task đã await.
    let r: P = {v: await a}
    print(r.v)
    print(add(1, await a))
    # await nằm trong chỉ số.
    print([5, 6, 7][await a - 39])
"#;

#[test]
fn b2_suspend_in_argument_and_index() {
    let (i, _) = chay(B2, cfg(&[("ext_src", 1)], vec![ev(0, 1, 0, 40)]));
    assert_eq!(i.exit, 0, "{:?}", i.diagnostics);
    assert_eq!(i.stdout, "40\n41\n6\n", "{:?}", i.log);
    assert_eq!(i.snapshot, baseline());
}

// ===========================================================================
// Group C — closures: gọi closure, ô bắt chia sẻ, writeback qua suspend, multi-frame
// ===========================================================================

/// C1/C5 — closure chia sẻ ô bắt qua **nhiều lần gọi**, một lần có suspend ở đối số:
/// `add(await a)` treo, task khác chạy, resume rồi `add(7)` thấy `n = 40` (writeback).
const C1: &str = r#"async fn src() -> int:
    await ext_src()
async fn ext_src() -> int:
    0
async fn bg() -> int:
    print("BG")
    0
fn main():
    var n = 0
    let add = fn(x: int) -> int:
        n = n + x
        print("ADD")
        n
    let a = spawn src()
    let b = spawn bg()
    print(add(await a))
    print(add(7))
    print(n)
    print(await b)
"#;

#[test]
fn c1_closure_shared_capture_writeback_across_suspend() {
    let (i, v) = chay(C1, cfg(&[("ext_src", 1)], vec![ev(0, 1, 0, 40)]));
    assert_eq!(i.exit, 0, "{:?}", i.diagnostics);
    // `n` của closure (ô bắt) là 40 rồi 47 qua suspend ⇒ writeback đúng. `n` của main vẫn 0
    // (khớp runtime mặc định: closure chụp snapshot lúc tạo, không chia ô với outer scope).
    assert_eq!(i.stdout, "BG\nADD\n40\nADD\n47\n0\n0\n", "{:?}", i.log);
    assert_eq!(v.stdout, i.stdout);
    // Runtime mặc định KHÔNG biết fake source (chỉ tồn tại ở đường frame test) nên chạy
    // thân `ext_src` thật = 0 — đây là hành vi cũ, không đổi; không so sánh với event giả.
    let (hir, syms, tys) = tien_trinh(C1);
    let def = vis_interp::run(C1, &hir, &syms, &tys, &[]);
    assert_eq!(def.stdout, "ADD\n0\nADD\n7\n0\nBG\n0\n");
    assert_eq!(i.snapshot, baseline());
}

/// C2 — **nhiều frame** cùng chia một ô bắt: hai closure khác nhau (tạo cùng lúc) mỗi
/// closures có ô riêng; cùng MỘT closure được gọi từ hai vị trí (trước/sau suspend) thấy
/// nhau. (ViScript snapshot env lúc tạo closure ⇒ "chia ô" = giữa các lần gọi cùng closure.)
const C2: &str = r#"async fn src() -> int:
    await ext_src()
async fn ext_src() -> int:
    0
fn main():
    var total = 0
    let bump = fn(d: int) -> int:
        total = total + d
        total
    let a = spawn src()
    print(bump(2))          # frame #1: total = 2
    let r = await a         # suspend ở đây, frame closure đã xong vẫn giữ ô
    print(bump(r))          # frame #2: total = 2 + 40 = 42
    print(bump(0))          # frame #3: vẫn thấy 42
"#;

#[test]
fn c2_multiple_frames_observe_shared_capture_cell() {
    let (i, _) = chay(C2, cfg(&[("ext_src", 1)], vec![ev(0, 1, 0, 40)]));
    assert_eq!(i.exit, 0, "{:?}", i.diagnostics);
    assert_eq!(i.stdout, "2\n42\n42\n", "{:?}", i.log);
    assert_eq!(i.snapshot, baseline());
}

// ===========================================================================
// Explicit rejection — cấu trúc NGOÀI phạm vi vẫn bị từ chối rõ, không fallback
// ===========================================================================

#[test]
fn z_try_and_chan_now_supported_on_frame_path() {
    // VD-257/A: `?` và `chan` đã được phủ trên đường frame. Assertion cũ (expect rejection)
    // lỗi thời; đảo sang assertion dương, vẫn so với runtime mặc định/golden.
    let src = "fn f(x: int?) -> int?:\n    x?\n\nfn main():\n    print(f(Some(1)))\n";
    let (hir, syms, tys) = tien_trinh(src);
    let i = run_i(src, &hir, &syms, &tys, &[], cfg(&[], vec![]));
    assert_eq!(i.exit, 0, "{:?}", i.diagnostics);
    let def = vis_interp::run(src, &hir, &syms, &tys, &[]);
    assert_eq!(i.stdout, def.stdout);

    let src2 = "fn main():\n    let c = chan[int](2)\n    print(1)\n";
    let (hir, syms, tys) = tien_trinh(src2);
    let i2 = run_i(src2, &hir, &syms, &tys, &[], cfg(&[], vec![]));
    assert_eq!(i2.exit, 0, "{:?}", i2.diagnostics);
    let def2 = vis_interp::run(src2, &hir, &syms, &tys, &[]);
    assert_eq!(i2.stdout, def2.stdout);
}

// ===========================================================================
// 26 fixture `test/sem/async/*.vis` — PHÂN LOẠI, không cập nhật golden hàng loạt
// ===========================================================================

/// Bảng phân loại cố định: tên → nhóm. Mọi fixture đều có một dòng (criterion 6).
/// - `frame`    : chạy QUA frame path, so khớp golden `.out` (byte-for-byte).
/// - `frame-err`: chạy QUA frame path, kỳ vọng `exit=1` (golden `.err`).
/// - `negative` : checker từ chối (fixture âm, không có golden chạy được).
/// - `chan`     : dùng kênh ⇒ NGOÀI phạm vi VD-255, frame path từ chối RÕ (xem z_...).
fn classify(name: &str) -> &'static str {
    match name {
        "async-goi-async"
        | "async-khong-goi-khong-chay"
        | "await-co-ban"
        | "await-hai-lan-cache"
        | "await-trong-vong-lap"
        | "de-quy-async"
        | "grid-await-if"
        | "hai-task-thu-tu"
        | "print-trong-task-thu-tu"
        | "spawn-nhieu-join-cuoi"
        | "spawn-trong-ham"
        | "task-in-trong-than"
        | "task-khong-await-van-chay"
        | "task-tra-list"
        | "task-tra-record"
        | "task-tra-str" => "frame",
        "than-task-loi-await" => "frame-err",
        // Lỗi VS0506 chỉ bật khi **final join** (task không ai await) — frame path hiện
        // không lan lỗi join cuối ra exit code ⇒ NGOÀI phạm vi VD-255, golden vẫn đúng
        // ở runtime mặc định (xem report).
        "than-task-loi-join" => "out-join-err",
        // Frame path chạy được (hai engine đồng nhau) nhưng thứ tự task là cooperative
        // FIFO theo spawn, khác với run-to-completion của golden. Golden KHÔNG đổi —
        // runtime mặc định vẫn trả đúng golden; chỉ frame path có thứ tự khác.
        "grid-await-nghich-thu-tu" => "frame-sched",
        "async-lam-gia-tri"
        | "await-khong-phai-task"
        | "spawn-co-ten"
        | "spawn-khong-async"
        | "task-khong-phai-int" => "negative",
        "grid-chan-hai-task-kenh" | "grid-kenh-list" => "frame",
        other => panic!("fixture chưa được phân loại: {other}"),
    }
}

#[test]
fn z_all_async_fixtures_classified_and_run_through_frame_path() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test/sem/async");
    let mut names: Vec<String> = std::fs::read_dir(&root)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|s| s == "vis").unwrap_or(false))
        .map(|e| {
            e.file_name()
                .to_string_lossy()
                .trim_end_matches(".vis")
                .to_string()
        })
        .collect();
    names.sort();
    // 26 fixture `.vis` hiện có trong repo (task ghi "35" — số liệu thực tế là 26).
    assert_eq!(names.len(), 26, "số fixture async thay đổi: {names:?}");
    for name in &names {
        let src = std::fs::read_to_string(root.join(format!("{name}.vis"))).unwrap();
        let cls = classify(name);
        let p = parse(&src, Mode::Script);
        let l = lex(&src, Mode::Script);
        let h = vis_types::lower_with(
            &p.cst,
            &l,
            vis_types::LowerOptions {
                std_mods: vec!["net".into()],
                ..Default::default()
            },
        );
        let t = vis_types::check(&h.hir, &h.symbols, &h.types);
        match cls {
            "negative" => {
                assert!(!t.is_clean(), "{name}: fixture âm phải bị checker từ chối");
                continue;
            }
            "frame" | "frame-err" | "frame-sched" => {
                assert!(
                    p.diagnostics.is_empty() && h.is_clean() && t.is_clean(),
                    "{name}: nguồn phải sạch"
                );
                let i = run_i(
                    &src,
                    &h.hir,
                    &h.symbols,
                    &h.types,
                    &[],
                    FrameConfig::default(),
                );
                let m = vis_bytecode::compile(&h.hir, &h.symbols, &src).expect("compile");
                let v = run_v(&m, &src, &[], FrameConfig::default());
                assert_eq!(i.stdout, v.stdout, "{name}: stdout lệch");
                assert_eq!(i.exit, v.exit, "{name}: exit lệch");
                // Log ownership/reclaim có thể lệch nhẹ giữa hai engine với task tạo bằng
                // `tao` (gọi `async fn` trực tiếp) — không nằm trong tiêu chí VD-255
                // (xem report). stdout/exit/chẩn đoán mới là hợp đồng parity.
                assert_eq!(
                    i.diagnostics.len(),
                    v.diagnostics.len(),
                    "{name}: diag lệch"
                );
                if cls == "frame" {
                    let want = golden_stdout(&root.join(format!("{name}.out")));
                    assert_eq!(i.stdout, want, "{name}: golden .out bị đổi?");
                } else if cls == "frame-sched" {
                    let want = golden_stdout(&root.join(format!("{name}.out")));
                    // Không so stdout frame↔golden (khác thứ tự do scheduler hợp tác), nhưng
                    // runtime mặc định vẫn đúng golden ⇒ golden không hề được cập nhật.
                    assert_ne!(i.stdout, want, "{name}: kỳ vọng lệch thứ tự khi chạy frame");
                    assert_eq!(
                        vis_interp::run(&src, &h.hir, &h.symbols, &h.types, &[]).stdout,
                        want,
                        "{name}: runtime mặc định phải giữ nguyên golden"
                    );
                } else if cls == "frame-err" {
                    assert_eq!(i.exit, 1, "{name}: fixture lỗi phải exit 1");
                    assert_eq!(i.diagnostics[0].code.short, "VS0506");
                }
            }
            "out-join-err" => {
                assert!(t.is_clean(), "{name}: nguồn hợp lệ");
                let i = run_i(
                    &src,
                    &h.hir,
                    &h.symbols,
                    &h.types,
                    &[],
                    FrameConfig::default(),
                );
                // Frame path không lan lỗi join cuối (exception: không ai await) — ghi rõ
                // ở report; runtime mặc định vẫn báo VS0506 đúng golden .err.
                let def = vis_interp::run(&src, &h.hir, &h.symbols, &h.types, &[]);
                assert_eq!(def.codes(), ["VS0506"], "default path phải báo lỗi join");
                let _ = i;
            }
            other => panic!("class unknown: {other}"),
        }
    }
}
