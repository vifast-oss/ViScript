// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-245 — hồi quy tự động** cho ca 10k task VM từng trả `()` (A0, ST-238 review §2.2).
//!
//! Nguyên nhân đã được sửa trong lúc làm ST-238: khai `var` cấp tệp dùng `f.slot` (ô cục bộ 0)
//! thay vì `slot_cua` (ô global) — trong khi mọi lần **đọc** đều lấy ô global ⇒ đọc `()`.
//! Test dưới ép **đọc lại một `var` cấp tệp từ trong async task** (đúng dạng "task trả `()`"),
//! so sánh cả với `vis_interp` để không nhận parity nếu hai đường cùng sai (VD-242 §3.1).
//!
//! **Đỏ trên bản cũ**: chỉ cần đảo dòng `codegen.rs` `self.slot_cua(*sym)?` → `self.f.slot(*sym, *span)?`
//! ở nhánh `HStmt::Var` là task trả `()` / trap ⇒ `stdout` lệch kỳ vọng. Ở đây dùng bản nhỏ (1 task)
//! nhưng cùng cơ chế; phép đo 10k nguyên vẫn nằm ở `vis-cli/tests/async_10k.rs`.

use vis_bytecode::{compile, Func, Instr, Module, Span, GLOBAL_SLOT_BASE, HASH_LEN, VERSION};
use vis_interp::RunOutput;

fn tien_trinh(text: &str) -> (vis_types::Hir, vis_types::Symbols, vis_types::TypeTable) {
    let parsed = vis_syntax::parse(text, vis_syntax::Mode::Script);
    assert!(
        parsed.diagnostics.is_empty(),
        "parse: {:?}",
        parsed.diagnostics
    );
    let lexed = vis_syntax::lex(text, vis_syntax::Mode::Script);
    let h = vis_types::lower(&parsed.cst, &lexed);
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    (h.hir, h.symbols, h.types)
}

fn chay_interp(text: &str) -> RunOutput {
    let (hir, syms, tys) = tien_trinh(text);
    vis_interp::run(text, &hir, &syms, &tys, &[])
}

fn chay_vm(text: &str) -> RunOutput {
    let (hir, syms, _tys) = tien_trinh(text);
    let m = compile(&hir, &syms, text).expect("compile");
    vis_vm::run(&m, text, &[])
}

/// Async task đọc `var` cấp tệp rồi trả về — đúng dạng từng trả `()` trước bản sửa.
const NGUON_ASYNC: &str = "\
var dem = 42\n\
\n\
async fn doc_dem() -> int:\n\
\x20   dem\n\
\n\
print(await doc_dem())\n";

#[test]
fn async_task_doc_var_cap_tep_khong_tra_unit() {
    let a = chay_interp(NGUON_ASYNC);
    let b = chay_vm(NGUON_ASYNC);
    assert_eq!(
        a.stdout, "42\n",
        "interp: task phải trả 42, không phải (): {:?}",
        a
    );
    assert_eq!(
        b.stdout, "42\n",
        "VM: hồi quy — task trả ()/sai ô global: {:?}",
        b
    );
    assert_eq!(a.stdout, b.stdout, "interp ↔ VM lệch");
    assert!(a.diagnostics.is_empty(), "interp diag: {:?}", a.diagnostics);
    assert!(b.diagnostics.is_empty(), "VM diag: {:?}", b.diagnostics);
}

/// Bản tối giản không async: `var` phải đọc lại đúng giá trị khởi tạo.
#[test]
fn var_cap_tep_readback() {
    let src = "var x = 7\nprint(x)\n";
    assert_eq!(chay_interp(src).stdout, "7\n");
    assert_eq!(chay_vm(src).stdout, "7\n");
}

/// Bản nhỏ của `spawn` loop 10k: biến điều khiển là `var` cấp tệp (cùng cơ chế sửa).
#[test]
fn spawn_loop_ban_nho() {
    let src = "\
async fn viec(n: int) -> int:\n\
\x20   print(n)\n\
\x20   n\n\
\n\
var i = 0\n\
while i < 3:\n\
\x20   let t = spawn viec(i)\n\
\x20   i = i + 1\n\
print(\"main\")\n";
    let want = "main\n0\n1\n2\n";
    let a = chay_interp(src);
    let b = chay_vm(src);
    assert_eq!(a.stdout, want, "interp: {:?}", a);
    assert_eq!(b.stdout, want, "VM: {:?}", b);
}

/// **ST-245 (3)** — bytecode sai ô global không được làm VM panic: `Vm::run` xác minh trước,
/// trả chẩn đoán `bytecode hỏng` sạch. Không sửa `vm.rs`; chỉ kiểm hành vi hiện có.
#[test]
fn bytecode_global_ngoai_mien_khong_panic_va_bi_chan() {
    let code = vec![
        Instr::LoadLocal(GLOBAL_SLOT_BASE + 5), // 5 global nhưng module định nghĩa 0
        Instr::Return,
    ];
    let m = Module {
        version: VERSION,
        source_hash: [0u8; HASH_LEN],
        consts: vec![],
        funcs: vec![Func {
            name: "main".into(),
            arity: 0,
            nparams: 0,
            nlocals: 0,
            max_stack: 1,
            spans: vec![Span { start: 0, end: 0 }; code.len()],
            code,
        }],
        entry: 0,
    };
    let ketqua = std::panic::catch_unwind(|| vis_vm::run(&m, "", &[]));
    let out = ketqua.expect("VM không được panic trên bytecode hỏng");
    assert!(
        out.diagnostics
            .iter()
            .any(|d| d.message.contains("bytecode hỏng")),
        "phải bị chặn bởi verifier: {:?}",
        out
    );
}
