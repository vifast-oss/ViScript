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

#[test]
fn pop_frame_writeback_ngoai_mien_trap() {
    // **ST-311a (#190)**: `pop_frame` không được bỏ qua im lặng khi ô mirror hoặc chỉ số
    // biến bắt ngoài phạm vi — phải trap `VS0501`.
    let m = Module {
        version: vis_bytecode::VERSION,
        source_hash: [0u8; vis_bytecode::HASH_LEN],
        consts: vec![],
        funcs: vec![vis_bytecode::Func {
            name: "main".into(),
            arity: 0,
            nparams: 0,
            nlocals: 1,
            max_stack: 1,
            code: vec![Instr::PushUnit, Instr::Return],
            spans: vec![vis_bytecode::Span { start: 0, end: 0 }; 2],
        }],
        entry: 0,
    };
    let cells = Rc::new(RefCell::new(vec![Value::Int(1)]));

    // Ca 1: ô cục bộ `slot = 5` ngoài khung (`locals.len() == 1`).
    let mut th = Th::new(&m, 0, vec![Value::Int(42)], vec![]);
    th.frames[0].writeback = Some((cells.clone(), vec![(5, 0)]));
    let msg = trap(th.pop_frame(Span::new(0, 0)).err().expect("phải trap"));
    assert!(msg.contains("ô cục bộ 5 ngoài khung"), "{msg}");

    // Ca 2: chỉ số biến bắt `i = 3` ngoài môi trường (`cells.len() == 1`).
    let mut th2 = Th::new(&m, 0, vec![Value::Int(42)], vec![]);
    th2.frames[0].writeback = Some((cells.clone(), vec![(0, 3)]));
    let msg2 = trap(th2.pop_frame(Span::new(0, 0)).err().expect("phải trap"));
    assert!(msg2.contains("biến bắt 3 ngoài môi trường"), "{msg2}");
}

#[test]
fn iter_init_back_edge_khong_bac_hai_va_deadline() {
    use vis_bytecode::Const;

    // **ST-311a (#195 ca 1)**: bytecode hợp lệ (verify OK) nhảy ngược liên tục về `IterInit`
    // không bao giờ chạm `end_pc` — trước ST-311a gây O(n²) (> 101 phút ở 100M fuel);
    // nay phải trap `VS0501` ngay tại `MAX_ITER_DEPTH` trong < 100 ms.
    let m_backedge = Module {
        version: vis_bytecode::VERSION,
        source_hash: [0u8; vis_bytecode::HASH_LEN],
        consts: vec![Const {
            kind: 0,
            text: "1".into(),
        }],
        funcs: vec![vis_bytecode::Func {
            name: "main".into(),
            arity: 0,
            nparams: 0,
            nlocals: 1,
            max_stack: 4,
            code: vec![
                Instr::Const(0),
                Instr::MakeList(1),
                Instr::IterInit,
                Instr::IterNext(3),
                Instr::Pop,
                Instr::Jump(-5), // nhảy ngược thẳng về Const(0) -> IterInit!
                Instr::PushUnit,
                Instr::Return,
            ],
            spans: vec![vis_bytecode::Span { start: 0, end: 0 }; 8],
        }],
        entry: 0,
    };
    vis_bytecode::verify(&m_backedge).expect("module phải verify OK");
    let t0 = std::time::Instant::now();
    let out = run(&m_backedge, "", &[]);
    let dt = t0.elapsed();
    assert_eq!(out.exit, 1, "phải trap VS0501");
    assert!(
        out.diagnostics
            .iter()
            .any(|d| d.code.short == "VS0501" && d.message.contains("lồng quá sâu")),
        "chẩn đoán: {:?}",
        out.diagnostics
    );
    assert!(
        dt < std::time::Duration::from_secs(1),
        "back-edge IterInit phải dừng tức thời, thực tế {dt:?}"
    );

    // **ST-311a (#195 ca 2)**: vòng lặp hợp lệ liên tục `IterInit` -> `IterNext` hết vòng ->
    // nhảy lại từ đầu (1M vs 5M bước) phải tăng **tuyến tính** (tỉ lệ ≤ 10×, không phải O(n²)).
    let m_repeat = Module {
        version: vis_bytecode::VERSION,
        source_hash: [0u8; vis_bytecode::HASH_LEN],
        consts: vec![],
        funcs: vec![vis_bytecode::Func {
            name: "main".into(),
            arity: 0,
            nparams: 0,
            nlocals: 1,
            max_stack: 2,
            code: vec![
                Instr::MakeList(0),
                Instr::IterInit,
                Instr::IterNext(2), // rỗng ⇒ pop iter & nhảy tới pc=4 (`dat_pc(4)`)
                Instr::Pop,
                Instr::Jump(-4), // lặp lại từ đầu
                Instr::PushUnit,
                Instr::Return,
            ],
            spans: vec![vis_bytecode::Span { start: 0, end: 0 }; 7],
        }],
        entry: 0,
    };
    vis_bytecode::verify(&m_repeat).expect("m_repeat verify OK");

    let mut vm1 = Vm::new(&m_repeat, "");
    vm1.set_safety_fuel(200_000);
    let t1 = std::time::Instant::now();
    let o1 = vm1.run(&[]);
    let d1 = t1.elapsed().as_secs_f64().max(1e-4);
    assert!(o1.diagnostics.iter().any(|d| d.code.short == "VS0501-FUEL"));

    let mut vm2 = Vm::new(&m_repeat, "");
    vm2.set_safety_fuel(1_000_000);
    let t2 = std::time::Instant::now();
    let o2 = vm2.run(&[]);
    let d2 = t2.elapsed().as_secs_f64().max(1e-4);
    assert!(o2.diagnostics.iter().any(|d| d.code.short == "VS0501-FUEL"));
    // 5× fuel phải tốn < 15× thời gian (tuyến tính, không bậc hai 25×–160×).
    assert!(
        d2 / d1 < 15.0,
        "không được bậc hai: 200k={d1:.4}s, 1M={d2:.4}s, ratio={:.2}",
        d2 / d1
    );

    // **ST-311a (#195 ca 3)**: deadline wall-clock monotonic ngắt vòng vô hạn với `VS0501-TIME`.
    let mut vm_dl = Vm::new(&m_repeat, "").with_timeout(std::time::Duration::from_millis(40));
    let t_dl = std::time::Instant::now();
    let o_dl = vm_dl.run(&[]);
    let dt_dl = t_dl.elapsed();
    assert_eq!(o_dl.exit, 1);
    assert!(
        o_dl.diagnostics
            .iter()
            .any(|d| d.code.short == "VS0501-TIME"),
        "phải trả VS0501-TIME, thật: {:?}",
        o_dl.diagnostics
    );
    assert!(
        dt_dl < std::time::Duration::from_secs(2),
        "deadline 40ms không được chờ lâu: {dt_dl:?}"
    );
}
