// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-1c** — ma trận HIR → đường frame test, bổ sung ô coverage + chuẩn bị 1d.
//!
//! # Ma trận tóm tắt (source thật, cả 2 engine)
//!
//! | Nhóm HIR | Trạng thái | Bằng chứng |
//! |---|---|---|
//! | `Lit`/`Var`/`Bin` (`+ - * / // % ** .. ..= == != < <= > >= and or ??`)/`Un` | **CÓ** | `lat255_coverage` A3/B1 + file này |
//! | `Pipe` (toán tử ống) | **KHÔNG parity mặc định** — interp default chạy, VM default `VS0501` ("Pipe phải xử lý ở tầng biểu thức"); frame không được tuyên bố tương thích | ghi nhận, xem §1d blocker |
//! | `Call`/`CallValue`/`Lambda`/capture | **CÓ** | `lat255_coverage` C1/C2, `lat257_b_closure_await` |
//! | `Field` (gồm `?.`)/`Index`/`Record`/`Tuple`/`List`/`Set`/`Map`/`EnumLit` | **CÓ** | `lat257_coverage_extra` + file này (đã sửa `?.` giữ đúng lớp `Option`) |
//! | `Opt`/`Try` (`?`)/`With`/`If`/`Match`/`Interp` (kể cả `{expr:spec}`) | **CÓ** | `lat257_a_coverage` + file này |
//! | `Await`/`Spawn`/`ChanNew`/`Chan(0)`/`CmdBlock` | **CÓ / từ chối rõ** | `lat257_a_coverage`, `vs3_chan0`, file này |
//! | nested async (3 tầng `await` + closure chứa `await`) | **CÓ (cơ bản)** | `nested_async_co_ban_qua_bon_duong`; biến thể lồng sâu hơn chưa tái hiện |
//! | `HStmt` Let/Var/Assign/Expr/If/For/While/Loop/Scope/Return/Break/Continue/Match/Block | **CÓ** | `lat255_coverage` A1/A2/B1 + file này |
//! | `HPat` Wild/Bind/Lit/Ctor/Tuple/List/Rest/Record/Range/Or | **CÓ** | file này + `vd260_rest_pattern` + `lat255_coverage` |
//! | `chan(n≥1)` blocking đầy/rỗng | **TỪ CHỐI RÕ** (`VS0505`) | `lat257_a_coverage::b3`, file này |
//! | `cmd { … }` | **TỪ CHỐI RÕ** (`VS1`; VM compile `Unsupported("cmd-block")`) | `lat257_a_coverage::c1`, file này |
//! | `Ref` (VS1) | **KHÔNG áp dụng / guard test** — parser/lower/checker có thể chặn trước | `ref_node_khop_runtime_mac_dinh` |
//!
//! **1d chưa flip**: file này chỉ chứng minh đường frame chạy được cùng source với runtime mặc
//! định cho các cấu trúc sync/async đã phủ. Điều kiện flip nằm ở ghi chú cuối file.

use vis_interp::frame::{run_frame_for_test as run_i, FrameConfig};
use vis_interp::rt::DiagSnapshot;
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
            std_mods: vec!["net".into(), "str".into()],
            ..Default::default()
        },
    );
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    (h.hir, h.symbols, h.types)
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

/// Chạy 4 đường: default interp/VM + frame interp/VM. Trả stdout chuẩn để assert thêm.
fn bon_duong(name: &str, src: &str) -> String {
    let (hir, syms, tys) = tien_trinh(src);
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");

    let di = vis_interp::run(src, &hir, &syms, &tys, &[]);
    assert!(
        di.codes().is_empty(),
        "{name}: default interp {:?}",
        di.diagnostics
    );
    let mut vm = vis_vm::vm::Vm::new(&m, src);
    let dv = vm.run(&[]);
    assert!(
        dv.codes().is_empty(),
        "{name}: default VM {:?}",
        dv.diagnostics
    );
    assert_eq!(di.stdout, dv.stdout, "{name}: default I↔V stdout lệch");

    let fi = run_i(src, &hir, &syms, &tys, &[], FrameConfig::default());
    assert_eq!(fi.exit, 0, "{name}: frame interp {:?}", fi.diagnostics);
    let fv = run_v(&m, src, &[], FrameConfig::default());
    assert_eq!(fv.exit, 0, "{name}: frame VM {:?}", fv.diagnostics);

    assert_eq!(fi.stdout, di.stdout, "{name}: frame interp lệch default");
    assert_eq!(fv.stdout, di.stdout, "{name}: frame VM lệch default");
    assert_eq!(
        fi.snapshot,
        baseline(),
        "{name}: frame interp rò tài nguyên"
    );
    assert_eq!(fv.snapshot, baseline(), "{name}: frame VM rò tài nguyên");
    di.stdout
}

/// Ca dương: source phải chạy **cùng stdout** ở 4 đường, không fallback.
fn duong(name: &str, src: &str, want: &str) {
    let got = bon_duong(name, src);
    assert_eq!(got, want, "{name}: stdout");
}

/// Ca âm: frame path phải từ chối **rõ**, cùng thông điệp trên 2 engine.
fn tu_choi(name: &str, src: &str, want_code: &str, want_msg: &str) {
    let (hir, syms, tys) = tien_trinh(src);
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    let fi = run_i(src, &hir, &syms, &tys, &[], FrameConfig::default());
    let fv = run_v(&m, src, &[], FrameConfig::default());
    for (engine, out) in [("interp", &fi), ("VM", &fv)] {
        assert_ne!(out.exit, 0, "{name}/{engine}: phải bị từ chối");
        let d = out
            .diagnostics
            .first()
            .unwrap_or_else(|| panic!("{name}/{engine}: thiếu diagnostic"));
        assert_eq!(d.code.short, want_code, "{name}/{engine}: mã");
        assert!(
            d.message.contains(want_msg),
            "{name}/{engine}: msg {:?}",
            d.message
        );
    }
    assert_eq!(
        fi.diagnostics.len(),
        fv.diagnostics.len(),
        "{name}: số diag"
    );
    for (a, b) in fi.diagnostics.iter().zip(&fv.diagnostics) {
        assert_eq!(a.code.short, b.code.short);
        assert_eq!(a.message, b.message);
        assert_eq!((a.line, a.col), (b.line, b.col));
    }
}

// ---------------------------------------------------------------------------
// Set / Map / EnumLit có payload / Match pattern
// ---------------------------------------------------------------------------

#[test]
fn set_map_enum_payload_va_match_pattern() {
    let src = r#"enum E:
    A
    B(int)

fn main():
    let s = set{1, 2, 2, 3}
    print(s.len())
    print(s.contains(2))
    let m: map[str, int] = {"a": 1, "b": 2}
    print(m.len())
    print(m.get("b"))
    print(m.has("z"))
    let e: E = E.B(7)
    match e:
        A: print("A")
        B(x): print(x)
"#;
    duong("set-map-enum", src, "3\ntrue\n2\nSome(2)\nfalse\n7\n");
}

#[test]
fn match_or_range_list_rest_record_pattern() {
    let src = r#"type P:
    x: int
    y: int

fn k(n: int) -> str:
    match n:
        1 | 2: "small"
        3..=5: "mid"
        _: "big"

fn main():
    print(k(2))
    print(k(4))
    print(k(9))
    let xs = [1, 2, 3, 4]
    match xs:
        [a, ..rest]:
            print(a)
            print(rest.len())
        _: print("empty")
    let r: P = {x: 1, y: 2}
    match r:
        {x, y}: print(x + y)
"#;
    duong("match-pattern", src, "small\nmid\nbig\n1\n3\n3\n");
}

// ---------------------------------------------------------------------------
// Interp (chuỗi nội suy) + format spec; With non-Var; Coalesce
// ---------------------------------------------------------------------------

#[test]
fn interp_with_spec_with_nonvar_coalesce() {
    let src = r#"fn main():
    let x = 3.14159
    print("x={x:.2} n={1 + 2}")
    let r = {x: 1, y: 2} with x: 9
    print(r.x)
    print(r.y)
    let o: int? = None
    print(o ?? 42)
"#;
    duong("interp-with-coalesce", src, "x=3.14 n=3\n9\n2\n42\n");
}

// ---------------------------------------------------------------------------
// Let-else (Some / None / Result.Err) + `?` Result
// ---------------------------------------------------------------------------

#[test]
fn let_else_va_try_result() {
    let src = r#"fn f(o: int?) -> int:
    let x = o else:
        return 0
    x + 1

fn g(r: Result[int, str]) -> Result[int, str]:
    let x = r?
    Ok(x + 1)

fn main():
    print(f(Some(5)))
    print(f(None))
    print(g(Ok(7)))
    print(g(Err("e")))
"#;
    duong("let-else-try", src, "6\n0\nOk(8)\nErr(e)\n");
}

// ---------------------------------------------------------------------------
// `?.` optional field + if-let / while-let
// ---------------------------------------------------------------------------

#[test]
fn optional_field_va_if_while_let() {
    let src = r#"type P:
    v: int?

fn main():
    let a: P? = Some({v: Some(3)})
    let b: P? = Some({v: None})
    let c: P? = None
    print(a?.v)
    print(b?.v)
    print(c?.v)
    let o: int? = Some(4)
    if let Some(x) = o:
        print(x)
    var w: int? = Some(2)
    while let Some(x) = w:
        print(x)
        w = None
"#;
    duong(
        "opt-field-if-while-let",
        src,
        "Some(Some(3))\nSome(None)\nNone\n4\n2\n",
    );
}

// ---------------------------------------------------------------------------
// Ref (VS1 node) — hành vi hiện tại phải khớp runtime mặc định, không crash.
// ---------------------------------------------------------------------------

#[test]
fn ref_node_khop_runtime_mac_dinh() {
    // `ref` là VS1; nếu checker/lower cho qua thì frame path phải không crash và khớp default.
    let src = "fn main():\n    let x = ref(5)\n    print(x)\n";
    let p = parse(src, Mode::Script);
    if !p.diagnostics.is_empty() {
        return; // parser từ chối — ô không áp dụng
    }
    let l = lex(src, Mode::Script);
    let h = vis_types::lower_with(&p.cst, &l, vis_types::LowerOptions::default());
    if !h.is_clean() {
        return;
    }
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    if !t.is_clean() {
        return;
    }
    let _ = bon_duong("ref-node", src);
}

// ---------------------------------------------------------------------------
// Từ chối rõ: chan(n≥1) chặn, cmd
// ---------------------------------------------------------------------------

#[test]
fn chan_cap1_day_va_rong_tu_choi_ro() {
    let full = "fn main():\n    let c = chan[int](1)\n    c.send(1)\n    c.send(2)\n";
    tu_choi("chan-full", full, "VS0505", "`send` vào kênh **đầy**");
    let empty = "fn main():\n    let c = chan[int](1)\n    let x = c.recv()\n    print(x)\n";
    tu_choi("chan-empty", empty, "VS0505", "`recv` trên kênh **rỗng**");
}

#[test]
fn cmd_block_tu_choi_ro() {
    let src = "fn main():\n    print(\"truoc\")\n    let x = cmd { echo hi }\n    print(\"sau\")\n";
    let (hir, syms, tys) = tien_trinh(src);
    // Interp frame: trap VS1 rõ, trước khi in "sau".
    let fi = run_i(src, &hir, &syms, &tys, &[], FrameConfig::default());
    assert_eq!(fi.stdout, "truoc\n", "cmd: không chạy sau điểm từ chối");
    let d = fi.diagnostics.first().expect("cmd: thiếu diagnostic");
    assert_eq!(d.code.short, "VS0501");
    assert!(d.message.contains("khối `cmd` là VS1"), "{:?}", d.message);
    // VM compile từ chối cùng lý do — không fallback đồng bộ.
    match vis_bytecode::compile(&hir, &syms, src) {
        Err(vis_bytecode::CodegenError::Unsupported(what, _)) => {
            assert_eq!(what, "cmd-block");
        }
        other => panic!("cmd: VM phải Unsupported(cmd-block), got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Chuẩn bị 1d: quy tắc phân loại (chưa dùng để flip)
// ---------------------------------------------------------------------------

/// **Điều kiện 1d (đề xuất)**: chỉ chọn frame runtime khi HIR có ít nhất một `async fn`.
/// Chương trình sync không có `async fn` ⇒ giữ `Interp::run`/`Vm::run` nguyên vẹn.
fn co_async(hir: &vis_types::Hir) -> bool {
    hir.items
        .iter()
        .any(|it| matches!(it, vis_types::hir::HItem::Fn(f) if f.is_async))
}

#[test]
fn phan_loai_async_sync_cho_1d() {
    let sync = "fn main():\n    print(1)\n";
    let (hs, _, _) = tien_trinh(sync);
    assert!(!co_async(&hs), "sync không được chọn frame runtime");

    let async_src = "async fn t() -> int:\n    1\nfn main():\n    print(await spawn t())\n";
    let (ha, _, _) = tien_trinh(async_src);
    assert!(co_async(&ha), "có async fn phải chọn frame runtime");
}

// Ghi chú 1d (CHƯA FLIP — chỉ điều kiện/checklist):
//
// 1) Phân loại: chỉ chương trình có `HItem::Fn { is_async: true }` hoặc có `HExpr::Await`/
//    `HExpr::Spawn` trong HIR mới chọn frame runtime; **chương trình sync tuyệt đối giữ**
//    `Interp::run`/`Vm::run` hiện tại. Cần helper phân loại tĩnh (đề xuất đặt ở vis-types hoặc
//    CLI) + test sync-không-đổi trước khi flip.
// 2) Test bắt buộc trước flip: `vs3_budget`, `vs3_cancel`, `vs3_chan0`, `vs3_coverage_matrix`,
//    `lat255_coverage`, `lat257_a_coverage`, `lat257_c_lifecycle`, `lat256_net`, `lat3_real`,
//    toàn bộ fixture `test/sem/async`, golden sync của `vis run`/`vis run --vm`, diagnostics
//    parity, cache warm/cold.
// 3) VERSION/cache: 1d chỉ đổi **chọn runtime**, không đổi HIR/bytecode/opcode ⇒ không cần bump
//    `VERSION`. Nếu thêm nhãn chọn-runtime vào cache key thì bump `PHIEN_BAN` + test
//    `st246_cache_compat` trong cùng commit; nếu không, cache giữ nguyên.
// 4) Hạ tầng trước flip: `run_frame_for_test` hiện là entry **test-only**; cần một entry runtime
//    nội bộ (không API công khai) nối host/fs/net/args/caps như `Interp::run`/`Vm::run`, và chính
//    sách guard `200_000` turn cho chương trình thật. Chưa làm trong 1c.
// 5) Blocker trước flip: biến thể nested async lồng sâu (báo cáo cũ) chưa được xác minh; `Pipe`
//    default VM chưa parity; `chan(n≥1)` blocking vẫn `VS0505`; hợp đồng mã/result khi huỷ chờ A0.

#[test]
fn nested_async_co_ban_qua_bon_duong() {
    // Ba tầng async + closure chứa await. Biến thể lồng sâu hơn từng báo guard >200k chưa
    // tái hiện ở probe này; không tuyên bố phủ mọi hình dạng lồng.
    let src = r#"async fn leaf() -> int:
    1
async fn src() -> int:
    await leaf()
async fn outer() -> int:
    await src()
fn main():
    print(await outer())
    let f = fn() -> int:
        await src()
    print(f())
"#;
    duong("nested-async", src, "1\n1\n");
}
