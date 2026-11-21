// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-3b/W5 (P1)** — runner evaluator **W=1 non-frame** (`run_workers_local`).
//!
//! Hợp đồng: W=1 chạy **trên caller**, shared env, in trực tiếp; stdout/exit/steps/diagnostic/
//! `value` phải **byte-identical** với [`vis_interp::run`] (đường Off mặc định). Không tạo
//! thread, không snapshot, không đổi `Value`/`Task`/`VERSION`/cache.
//!
//! Phủ: sync + print/order, nested `spawn`/`await`, 10.000 task, lỗi thân `VS0506`, `chan(0)`
//! giữ `VS0505`, ngân sách `VS0501-BUDGET`. `chan(0)`/frame async parity là P4, không phải P1.

use vis_interp::rt::fee as ifee;
use vis_interp::RunOutput;
use vis_syntax::{lex, parse, Mode};

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

fn chan_diag(o: &RunOutput) -> Vec<(&'static str, String, u32, u32)> {
    o.diagnostics
        .iter()
        .map(|d| (d.code.short, d.message.clone(), d.line, d.col))
        .collect()
}

/// So W=1 với default trên **mọi** trường quan sát được; trả `(default, W=1)`.
fn so_khop(src: &str) -> (RunOutput, RunOutput) {
    let (hir, syms, tys) = tien_trinh(src);
    let a = vis_interp::run(src, &hir, &syms, &tys, &[]);
    let b = vis_interp::run_workers_local(src, &hir, &syms, &tys, &[]);
    assert_eq!(a.stdout, b.stdout, "stdout khác default");
    assert_eq!(a.exit, b.exit, "exit khác default");
    assert_eq!(a.steps, b.steps, "steps khác default");
    assert_eq!(a.codes(), b.codes(), "mã chẩn đoán khác default");
    assert_eq!(
        chan_diag(&a),
        chan_diag(&b),
        "chẩn đoán (mã/message/line/col) khác default"
    );
    assert_eq!(
        a.value.as_ref().map(|v| v.show()),
        b.value.as_ref().map(|v| v.show()),
        "value khác default"
    );
    (a, b)
}

struct BudgetGuard;
impl BudgetGuard {
    fn new(limit: u64) -> Self {
        ifee::enable_for_test(limit);
        Self
    }
}
impl Drop for BudgetGuard {
    fn drop(&mut self) {
        ifee::disable_for_test();
    }
}

fn so_khop_budget(src: &str, limit: u64) -> (RunOutput, RunOutput) {
    let (hir, syms, tys) = tien_trinh(src);
    let a = {
        let _g = BudgetGuard::new(limit);
        vis_interp::run(src, &hir, &syms, &tys, &[])
    };
    let b = {
        let _g = BudgetGuard::new(limit);
        vis_interp::run_workers_local(src, &hir, &syms, &tys, &[])
    };
    assert_eq!(a.stdout, b.stdout, "budget: stdout khác default");
    assert_eq!(a.exit, b.exit, "budget: exit khác default");
    assert_eq!(a.steps, b.steps, "budget: steps khác default");
    assert_eq!(a.codes(), b.codes(), "budget: mã khác default");
    (a, b)
}

const SYNC: &str = r#"fn tong(n: int) -> int:
    var s = 0
    var i = 1
    while i <= n:
        s = s + i * 2
        i = i + 1
    s

fn main():
    let x = tong(100)
    var y = x
    y = y + 1
    if y > 0:
        print("ok")
    print(y)
"#;

const THU_TU: &str = r#"async fn a():
    print("A1")
    print("A2")

async fn b():
    print("B1")
    print("B2")

fn main():
    let tb = spawn b()
    print("M1")
    let ta = spawn a()
    print("M2")
    await ta
    print("M3")
    await tb
    print("M4")
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

const LOI: &str = r#"async fn hong() -> int:
    panic("hong")

fn main():
    let t = spawn hong()
    print(await t)
"#;

const CHAN0: &str = r#"fn main():
    let c = chan[int](0)
    let _ = c.send(1)
"#;

const LOOP_BUDGET: &str = r#"fn main():
    var n = 0
    for i in 0..2_000_000:
        n = n + 1
    print(n)
"#;

const AWAIT_HAI_LAN: &str = r#"async fn t(n: int) -> int:
    print(n)
    n + 1

fn main():
    let x = spawn t(41)
    let a = await x
    let b = await x
    print(a)
    print(b)
"#;

#[test]
fn sync_byte_identical() {
    let (a, _) = so_khop(SYNC);
    assert_eq!(a.stdout, "ok\n10101\n");
    assert_eq!(a.exit, 0);
}

#[test]
fn print_va_thu_tu_byte_identical() {
    let (a, _) = so_khop(THU_TU);
    // Non-frame: `await ta` chạy đúng a; b còn lại join cuối theo thứ tự tạo.
    assert_eq!(a.stdout, "M1\nM2\nA1\nA2\nM3\nB1\nB2\nM4\n");
}

#[test]
fn nested_spawn_await_byte_identical() {
    let (a, _) = so_khop(NESTED);
    assert_eq!(a.stdout, "41\n");
    assert_eq!(a.exit, 0);
}

#[test]
fn muoi_nghin_task_byte_identical() {
    let (a, b) = so_khop(SPAWN_10K);
    assert_eq!(a.stdout, "50005000\n");
    assert_eq!(b.stdout, "50005000\n");
    assert_eq!(a.exit, 0);
    assert_eq!(a.steps, b.steps);
    assert!(a.steps > 0, "chống xanh-giả: steps = 0");
}

#[test]
fn loi_than_boc_vs0506_giong_default() {
    let (a, b) = so_khop(LOI);
    assert_eq!(a.codes(), vec!["VS0506"], "default: {:?}", a.diagnostics);
    assert_eq!(b.codes(), vec!["VS0506"], "W=1: {:?}", b.diagnostics);
    assert!(
        a.diagnostics[0].message.contains("VS0501"),
        "VS0506 phải giữ mã gốc VS0501: {:?}",
        a.diagnostics[0]
    );
    assert_eq!(a.exit, 1);
}

#[test]
fn chan0_giu_vs0505_nhu_non_frame() {
    let (a, b) = so_khop(CHAN0);
    assert_eq!(a.codes(), vec!["VS0505"], "default: {:?}", a.diagnostics);
    assert_eq!(b.codes(), vec!["VS0505"], "W=1: {:?}", b.diagnostics);
    assert_eq!(a.exit, b.exit);
}

#[test]
fn budget_can_giong_default() {
    let (a, b) = so_khop_budget(LOOP_BUDGET, 10_000);
    assert_eq!(
        a.codes(),
        vec!["VS0501-BUDGET"],
        "default: {:?}",
        a.diagnostics
    );
    assert_eq!(b.codes(), vec!["VS0501-BUDGET"], "W=1: {:?}", b.diagnostics);
    assert_eq!(a.steps, b.steps);
    assert_eq!(a.exit, 1);
}

#[test]
fn await_hai_lan_tra_cache_byte_identical() {
    let (a, _) = so_khop(AWAIT_HAI_LAN);
    assert_eq!(a.stdout, "41\n42\n42\n");
    assert_eq!(a.exit, 0);
}
