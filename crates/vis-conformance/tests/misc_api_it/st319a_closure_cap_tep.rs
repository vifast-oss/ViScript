// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-319a (#206)** — closure đọc/ghi **biến cấp tệp** (`var g` ở thân tệp) theo **giá trị tại
//! điểm tạo closure** (SPEC §4.7: “Closure bắt biến theo giá trị”).
//!
//! Nợ `#206`: interp snapshot `0` còn VM đọc ô global **live** `1` trên cùng chương trình
//! (`vis-diff` không sinh mẫu nên chưa bắt). ST-319a chụp global **có xuất hiện trong thân** vào
//! `MakeClosure` (ô cục bộ riêng của closure, `LoadCaptured`), **không** đổi VBC1; ca này khoá
//! **đọc**, **ghi** và lambda **lồng** trên bốn đường: interp/VM mặc định + W=1 cả hai engine.

use vis_core::RunOutput;
use vis_syntax::{lex, parse, Mode};

type Anh = (
    vis_hir::Hir,
    vis_hir::Symbols,
    vis_hir::TypeTable,
    vis_bytecode::Module,
);

fn anh(src: &str) -> Anh {
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
    let m = vis_bytecode::compile(&h.hir, &h.symbols, src).expect("compile");
    (h.hir, h.symbols, h.types, m)
}

fn so_khop(a: &RunOutput, b: &RunOutput, nhan: &str) {
    assert_eq!(a.stdout, b.stdout, "{nhan}: stdout");
    assert_eq!(a.exit, b.exit, "{nhan}: exit");
    assert_eq!(a.codes(), b.codes(), "{nhan}: codes");
    assert_eq!(
        a.value.as_ref().map(vis_core::show),
        b.value.as_ref().map(vis_core::show),
        "{nhan}: giá trị cuối"
    );
}

/// Chạy bốn đường (mặc định + W=1) và trả stdout của interp mặc định để so mong đợi.
fn bon_duong(name: &str, src: &str) -> String {
    let a = anh(src);
    let di = vis_interp::run(src, &a.0, &a.1, &a.2, &[]);
    let dv = vis_vm::run(&a.3, src, &[]);
    let wi = vis_interp::run_workers_local(src, &a.0, &a.1, &a.2, &[]);
    let wv = vis_vm::run_workers_local_vm(&a.3, src, &[]);
    for (nhan, out) in [
        ("interp", &di),
        ("VM", &dv),
        ("interp W1", &wi),
        ("VM W1", &wv),
    ] {
        assert!(out.is_clean(), "{name}/{nhan}: {:?}", out.diagnostics);
        assert_eq!(out.exit, 0, "{name}/{nhan}: exit");
    }
    so_khop(&di, &dv, &format!("{name}: I↔V"));
    so_khop(&di, &wi, &format!("{name}: I↔I-W1"));
    so_khop(&di, &wv, &format!("{name}: I↔V-W1"));
    di.stdout
}

// ---------------------------------------------------------------------------
// Ca 1 — **đọc** global qua closure: snapshot tại điểm tạo, không thấy ghi sau đó.
// ---------------------------------------------------------------------------
#[test]
fn doc_global_qua_closure_la_snapshot() {
    const SRC: &str = r#"var g = 0
fn main():
    let f = fn() -> int: g
    print(f())
    g = 1
    print(f())
"#;
    assert_eq!(bon_duong("doc-global", SRC), "0\n0\n");
}

// ---------------------------------------------------------------------------
// Ca 2 — **ghi** global qua closure: ô riêng của closure (không xuyên ra global ngoài),
// giữ giá trị qua nhiều lần gọi closure (env closure bền như interp).
// ---------------------------------------------------------------------------
#[test]
fn ghi_global_qua_closure_khong_xuyen_ra_ngoai() {
    const SRC: &str = r#"var g = 0
fn main():
    let f = fn() -> int:
        g = 5
        g
    print(f())
    print(g)
    g = 1
    print(f())
"#;
    assert_eq!(bon_duong("ghi-global", SRC), "5\n0\n5\n");
}

/// **ST-319a F2 (audit)**: `g = 5` là idempotent nên không chứng minh state của closure **bền**
/// qua nhiều lần gọi; `g = g + 1` buộc lần gọi 2 phải thấy `g` của lần 1 trong **ô riêng** của
/// closure, còn global ngoài vẫn `0`.
#[test]
fn ghi_global_qua_closure_ben_qua_nhieu_lan_goi() {
    const SRC: &str = r#"var g = 0
fn main():
    let f = fn() -> int:
        g = g + 1
        g
    print(f())
    print(f())
    print(g)
"#;
    assert_eq!(bon_duong("ghi-global-ben", SRC), "1\n2\n0\n");
}

// ---------------------------------------------------------------------------
// Ca 3 — lambda **lồng**: closure ngoài chụp global lúc tạo, lambda trong chụp lại **bản
// snapshot của closure ngoài** (không đọc global live khi nó được tạo muộn hơn).
// ---------------------------------------------------------------------------
#[test]
fn lambda_long_giu_snapshot_cua_closure_ngoai() {
    const SRC: &str = r#"var g = 0
fn main():
    let ngoai = fn() -> fn() -> int:
        fn() -> int:
            g
    g = 7
    let trong = ngoai()
    print(trong())
    print(g)
"#;
    assert_eq!(bon_duong("lambda-long", SRC), "0\n7\n");
}

/// **ST-319a F2 (audit)**: lambda lồng **ghi** `g = g + 1` — inner thấy snapshot `g=0` của
/// `ngoai` (tạo trước khi `g = 7`), state bền qua hai lần gọi, global ngoài vẫn `7`.
#[test]
fn lambda_long_ghi_global() {
    const SRC: &str = r#"var g = 0
fn main():
    let ngoai = fn() -> fn() -> int:
        fn() -> int:
            g = g + 1
            g
    g = 7
    let trong = ngoai()
    print(trong())
    print(trong())
    print(g)
"#;
    assert_eq!(bon_duong("lambda-long-ghi", SRC), "1\n2\n7\n");
}
