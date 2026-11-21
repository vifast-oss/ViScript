// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — gán ghép trên **lá đường** ở **đường frame** (`async`/`spawn`).
//!
//! Trước bản sửa, `ApplyAssignPath` của frame evaluator từ chối mọi `op != Set` bằng trap
//! `VS0501` "gán ghép trên trường/phần tử chưa được đường frame test hỗ trợ" — trong khi
//! interp thường và VM đã hỗ trợ `a.b += 1` (VD-268). Bản sửa dùng **cùng** ngữ nghĩa
//! `interp::gan_long`: đọc lá cũ từ ảnh chụp gốc, áp `BinOp`, dựng lại đường.
//!
//! **RED-BEFORE**: trước sửa, 2 ca dưới đều trap ở **cả hai** engine với thông điệp trên.

use vis_interp::frame::{run_frame_for_test as run_i, FrameConfig, FrameOutput};
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

fn chay(src: &str) -> (FrameOutput, FrameOutput) {
    let (hir, syms, tys) = tien_trinh(src);
    let c = FrameConfig::default();
    let i = run_i(src, &hir, &syms, &tys, &[], c.clone());
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    let v = run_v(&m, src, &[], c);
    (i, v)
}

const DUONG: &str = r#"async fn f() -> ():
    var r = {n: 1}
    r.n += 4
    var xs = [10, 20]
    xs[1] += 5
    var s = {a: {b: 1}}
    s.a.b += 2
    print(r.n)
    print(xs[1])
    print(s.a.b)
fn main():
    let t = spawn f()
    await t
"#;

/// `+=` trên trường, phần tử, và đường lồng — cả hai engine cùng stdout, không trap.
#[test]
fn frame_gan_ghep_duong_truong_phan_tu_long() {
    let (i, v) = chay(DUONG);
    for (name, o) in [("interp", &i), ("vm", &v)] {
        assert_eq!(o.exit, 0, "{name}: {:?}", o.diagnostics);
        assert_eq!(o.stdout, "5\n25\n3\n", "{name}: {:?}", o.stdout);
        assert!(o.diagnostics.is_empty(), "{name}: {:?}", o.diagnostics);
    }
    assert_eq!(i.stdout, v.stdout, "stdout lệch giữa hai engine");
}

const NHIEU_TOAN_TU: &str = r#"async fn f() -> ():
    var r = {a: 10, b: 7}
    r.a -= 3
    r.a *= 2
    r.a //= 5
    r.b %= 4
    var xs = [1, 2, 3]
    xs[0] += 10
    xs[1] *= 3
    xs[2] //= 2
    print(r.a)
    print(r.b)
    print(xs[0])
    print(xs[1])
    print(xs[2])
fn main():
    let t = spawn f()
    await t
"#;

/// `-=`, `*=`, `//=`, `%=` trên trường; `+=`, `*=`, `/=` trên phần tử.
#[test]
fn frame_gan_ghep_nhieu_toan_tu() {
    let (i, v) = chay(NHIEU_TOAN_TU);
    for (name, o) in [("interp", &i), ("vm", &v)] {
        assert_eq!(o.exit, 0, "{name}: {:?}", o.diagnostics);
        assert_eq!(o.stdout, "2\n3\n11\n6\n1\n", "{name}: {:?}", o.stdout);
        assert!(o.diagnostics.is_empty(), "{name}: {:?}", o.diagnostics);
    }
    assert_eq!(i.stdout, v.stdout, "stdout lệch giữa hai engine");
}

const TRAP_CHIA: &str = r#"async fn f() -> ():
    var r = {n: 8}
    r.n //= 0
    print(r.n)
fn main():
    let t = spawn f()
    await t
"#;

/// Chia cho 0 trong gán ghép đường vẫn trap **cùng mã** ở hai engine (không nuốt lỗi).
#[test]
fn frame_gan_ghep_chia_0_trap_hai_engine() {
    let (i, v) = chay(TRAP_CHIA);
    assert_eq!(i.exit, v.exit, "exit lệch");
    assert_ne!(i.exit, 0, "phải trap");
    assert_eq!(
        i.diagnostics.len(),
        v.diagnostics.len(),
        "số chẩn đoán lệch"
    );
    assert!(
        i.diagnostics.iter().any(|d| {
            d.message.contains("VS0502")
                || d.message.contains("chia cho 0")
                || d.code.short.contains("VS0507")
                || d.code.short.contains("VS0501")
        }),
        "interp: {:?}",
        i.diagnostics
    );
    for (a, b) in i.diagnostics.iter().zip(&v.diagnostics) {
        assert_eq!(a.code.short, b.code.short, "mã chẩn đoán lệch");
        assert_eq!(a.message, b.message, "nội dung chẩn đoán lệch");
    }
}
