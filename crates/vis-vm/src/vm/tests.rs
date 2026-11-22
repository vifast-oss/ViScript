// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-284 (VD-272)** — test nội bộ của VM (tách khỏi `vm.rs` để giữ tệp lõi ≤ 1.250 dòng).
//! Nguyên văn, cùng module path `vis_vm::vm::tests`, 0 đổi hành vi.

use super::*;

fn vm() -> Vm<'static> {
    static M: std::sync::OnceLock<Module> = std::sync::OnceLock::new();
    let m = M.get_or_init(|| Module {
        version: vis_bytecode::VERSION,
        source_hash: [0u8; vis_bytecode::HASH_LEN],
        consts: vec![],
        funcs: vec![vis_bytecode::Func {
            name: "main".into(),
            arity: 0,
            nparams: 0,
            nlocals: 0,
            max_stack: 1,
            code: vec![Instr::PushUnit, Instr::Return],
            spans: vec![vis_bytecode::Span { start: 0, end: 0 }; 2],
        }],
        entry: 0,
    });
    Vm::new(m, "")
}

fn trap(e: Abort) -> String {
    match e {
        Abort::Trap(t) => {
            assert_eq!(t.code.short, "VS0501");
            t.msg
        }
        _ => panic!("expected runtime trap"),
    }
}

#[test]
fn capture_local_barrier() {
    let vm = vm();
    let one = vec![Value::Unit];
    assert!(value::eq(
        &vm.doc_local(&one, 0, Span::new(0, 0)).unwrap(),
        &Value::Unit
    ));
    let msg = trap(vm.doc_local(&one, 1, Span::new(0, 0)).unwrap_err());
    assert!(
        msg.contains("biến cục bộ 1 ngoài khung") && msg.contains("1 ô"),
        "{msg}"
    );
    assert!(value::eq(
        &vm.doc_captured(&one, 0, Span::new(0, 0)).unwrap(),
        &Value::Unit
    ));
    for (cap, i, want) in [
        (
            &[] as &[Value],
            0,
            "biến bắt 0 ngoài môi trường capture (0 ô)",
        ),
        (&one[..], 1, "biến bắt 1 ngoài môi trường capture (1 ô)"),
        (&one[..], u16::MAX, "biến bắt 65535 ngoài"),
    ] {
        let msg = trap(vm.doc_captured(cap, i, Span::new(0, 0)).unwrap_err());
        assert!(msg.contains(want), "{msg}");
    }
    let mut slot = vec![Value::Unit];
    vm.ghi_local(&mut slot, 0, Value::Int(7), Span::new(0, 0))
        .unwrap();
    assert!(value::eq(&slot[0], &Value::Int(7)));
    let msg = trap(
        vm.ghi_local(&mut slot, 1, Value::Int(9), Span::new(0, 0))
            .unwrap_err(),
    );
    assert!(msg.contains("ghi biến cục bộ 1 ngoài khung"), "{msg}");
    assert_eq!(slot.len(), 1);
}
