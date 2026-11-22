// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-257 / stream B** — bằng chứng dương: `await` trong thân closure chạy được trên
//! **VM frame path** (đóng lỗ hổng stream A §4 mục 1).
//!
//! Nguyên nhân: `Instr::CallValue` gọi `goi_ham_bytecode` tạo `Th` **lồng nhau**. Khi thân closure
//! `$await` trả `Abort::Pending`, driver frame lưu `Th` ngoài, `Th` lồng bị mất ⇒ resume chạy lại
//! `CallValue` trên ngăn xếp đã pop đối số ⇒ `StackUnderflow`. Sửa ở `vm.rs`: `CallValue` trên
//! `Value::Vm` đẩy khung vào **chính `Th` hiện tại** (cơ chế `Instr::Call`) ⇒ suspend/resume giữ
//! trọn trạng thái. Không fallback, không đổi opcode/ngôn ngữ/runtime mặc định, không đụng file
//! của stream A (`vm_frame.rs`, `frame/**`, `rt.rs`).

use vis_interp::frame::{run_frame_for_test as run_i, FrameConfig};
use vis_interp::rt::FakeEvent;
use vis_interp::Value;
use vis_syntax::{lex, parse, Mode};
use vis_vm::vm_frame::run_frame_for_test as run_v;

fn tien_trinh(src: &str) -> (vis_types::Hir, vis_types::Symbols, vis_types::TypeTable) {
    let p = parse(src, Mode::Script);
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    let l = lex(src, Mode::Script);
    let h = vis_types::lower_with(
        &p.cst,
        &l,
        vis_types::LowerOptions {
            std_mods: vec!["net".into()],
            ..Default::default()
        },
    );
    assert!(h.is_clean(), "{:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "{:?}", t.diags);
    (h.hir, h.symbols, h.types)
}

fn cfg() -> FrameConfig {
    FrameConfig {
        fake_sources: [("ext_src".to_string(), 1u64)].into_iter().collect(),
        events: vec![FakeEvent {
            seq: 0,
            key: 1,
            gen: 0,
            value: Value::Int(42),
        }],
        ..Default::default()
    }
}

const SRC: &str = r#"async fn ext_src() -> int:
    0
fn main():
    let f = fn() -> int:
        await ext_src()
    print(f())
"#;

#[test]
fn await_trong_than_closure_chay_duoc_tren_ca_hai_engine() {
    let (hir, syms, tys) = tien_trinh(SRC);
    let m = vis_bytecode::compile(&hir, &syms, SRC).expect("compile");
    // Bytecode hợp lệ — verify không chặn (lỗi trước đây là runtime StackUnderflow sau resume).
    assert_eq!(vis_bytecode::verify(&m), Ok(()));

    let i = run_i(SRC, &hir, &syms, &tys, &[], cfg());
    let v = run_v(&m, SRC, &[], cfg());
    for (name, o) in [("interp", &i), ("vm", &v)] {
        assert_eq!(o.exit, 0, "{name}: {:?}", o.diagnostics);
        assert!(o.diagnostics.is_empty(), "{name}: {:?}", o.diagnostics);
        assert_eq!(o.stdout, "42\n", "{name}: stdout");
    }
}

#[test]
fn goi_closure_khong_await_van_dung_va_giu_writeback() {
    // Regression: đổi đường gọi closure (CallValue) không làm hỏng closure bắt biến thường.
    let src = r#"fn main():
    var x = 10
    let f = fn() -> int:
        x = x + 1
        x
    print(f())
    print(f())
"#;
    let (hir, syms, tys) = tien_trinh(src);
    let i = vis_interp::run(src, &hir, &syms, &tys, &[]);
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    let v = vis_vm::vm::Vm::new(&m, src).run(&[]);
    assert_eq!(i.stdout, v.stdout, "interp vs VM mặc định");
    assert_eq!(
        i.stdout, "11\n12\n",
        "closure capture var writeback giữa các lần gọi"
    );
    assert!(v.diagnostics.is_empty(), "{:?}", v.diagnostics);
}
