// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-252 — capture/local ngoài miền: verifier + runtime trap** (VD-246 / `BLOCKED.md` §17 / nợ #189).
//!
//! Tám ca nghiệm thu VD-246 §3. Mỗi ca ghi rõ **verifier từ chối** hay **runtime trap** — không ép
//! mọi ca qua verifier. Nghiệm thu chứng minh **hết thành công giả**: crafted bytecode đọc capture
//! ngoài miền trước đây trả `Unit` `exit = 0`, nay trap có chẩn đoán (`VS0501`, `exit != 0`,
//! `value = None`). Capture chứa `Unit` và local đã khởi tạo `Unit` vẫn trả `Unit` bình thường.

use vis_bytecode::verify::{load_verified, verify, VerifyError};
use vis_bytecode::{compile, decode, encode, Const, Func, Instr, Module, Span, HASH_LEN, VERSION};
use vis_interp::RunOutput;

fn func(name: &str, code: Vec<Instr>, nparams: u16, nlocals: u16, max_stack: u16) -> Func {
    Func {
        name: name.into(),
        arity: 0,
        nparams,
        nlocals,
        max_stack,
        spans: vec![Span { start: 0, end: 0 }; code.len()],
        code,
    }
}

fn m(funcs: Vec<Func>, consts: Vec<Const>) -> Module {
    Module {
        version: VERSION,
        source_hash: [0u8; HASH_LEN],
        consts,
        funcs,
        entry: 0,
    }
}

fn show(out: &RunOutput) -> Option<String> {
    out.value.as_ref().map(vis_interp::show)
}

/// Khẳng định đây là **trap/chặn thật**, có chẩn đoán `VS0501`, `exit != 0`, không trả giá trị.
fn assert_trap(out: &RunOutput, msg_part: &str) {
    assert!(!out.is_clean(), "phải có chẩn đoán, got {out:?}");
    assert_ne!(out.exit, 0, "exit phải khác 0");
    assert!(out.value.is_none(), "không được trả giá trị mặc định");
    let d = &out.diagnostics[0];
    assert_eq!(
        d.code.short, "VS0501",
        "phải dùng mã runtime hiện hành: {d:?}"
    );
    assert!(
        d.message.contains(msg_part),
        "thông điệp phải chứa `{msg_part}`: {:?}",
        d.message
    );
}

/// Khẳng định module hợp lệ chạy sạch và trả đúng `()`.
fn assert_clean_unit(modu: &Module) {
    assert_eq!(verify(modu), Ok(()));
    let out = vis_vm::run(modu, "", &[]);
    assert!(out.is_clean(), "không được trap: {:?}", out.diagnostics);
    assert_eq!(out.exit, 0);
    assert_eq!(show(&out).as_deref(), Some("()"));
}

// ---------------------------------------------------------------------------
// (1) Capture rỗng đọc chỉ số 0.
//     - MakeClosure(fi, 0) với hàm đích dùng LoadCaptured(0) ⇒ **verifier từ chối** (ClosureFree).
//     - Hàm entry tự LoadCaptured(0) (không qua MakeClosure) ⇒ verifier lọt, **runtime trap**.
// ---------------------------------------------------------------------------
#[test]
fn t1_capture_rong_doc_chi_so_0() {
    // (a) verifier: MakeClosure nfree=0 < captured_used=1.
    let modu = m(
        vec![
            func(
                "main",
                vec![Instr::MakeClosure(1, 0), Instr::CallValue(0), Instr::Return],
                0,
                0,
                1,
            ),
            func("body", vec![Instr::LoadCaptured(0), Instr::Return], 0, 0, 1),
        ],
        vec![],
    );
    assert!(matches!(
        verify(&modu),
        Err(VerifyError::ClosureFree {
            func: 0,
            nfree: 0,
            used: 1,
            ..
        })
    ));
    assert_trap(
        &vis_vm::run(&modu, "", &[]),
        "bytecode hỏng", // chặn ở verify trước khi execute
    );

    // (b) runtime: entry dùng LoadCaptured(0) nhưng không có môi trường capture nào.
    let modu_rt = m(
        vec![func(
            "main",
            vec![Instr::LoadCaptured(0), Instr::Return],
            0,
            0,
            1,
        )],
        vec![],
    );
    assert_eq!(
        verify(&modu_rt),
        Ok(()),
        "verifier không có địa điểm closure để so"
    );
    assert_trap(
        &vis_vm::run(&modu_rt, "", &[]),
        "biến bắt 0 ngoài môi trường capture (0 ô)",
    );
}

// ---------------------------------------------------------------------------
// (2) Chỉ số = độ dài capture.
//     - MakeClosure(fi, 1) với hàm đích dùng LoadCaptured(1) ⇒ **verifier từ chối** (ClosureFree).
//     - Hàng rào runtime "index == len" được test nội bộ trong `vm.rs` (`doc_captured`).
// ---------------------------------------------------------------------------
#[test]
fn t2_chi_so_bang_do_dai_capture() {
    let modu = m(
        vec![
            func(
                "main",
                vec![
                    Instr::PushUnit, // cái để closure chụp (ô 0)
                    Instr::MakeClosure(1, 1),
                    Instr::CallValue(0),
                    Instr::Return,
                ],
                0,
                0,
                1,
            ),
            func("body", vec![Instr::LoadCaptured(1), Instr::Return], 0, 0, 1),
        ],
        vec![],
    );
    assert!(matches!(
        verify(&modu),
        Err(VerifyError::ClosureFree {
            nfree: 1,
            used: 2,
            ..
        })
    ));
    assert_trap(
        &vis_vm::run(&modu, "", &[]),
        "bytecode hỏng", // chặn ở verify: nfree=1 < used=2
    );
}

// ---------------------------------------------------------------------------
// (3) Chỉ số lớn nhất mã hóa được (u16::MAX = 65535).
//     - MakeClosure(fi, 0) + LoadCaptured(65535) ⇒ **verifier từ chối** (ClosureFree).
//     - Entry tự LoadCaptured(65535) (không qua MakeClosure) ⇒ **runtime trap**, không panic.
// ---------------------------------------------------------------------------
#[test]
fn t3_chi_so_u16_max() {
    let modu = m(
        vec![
            func(
                "main",
                vec![Instr::MakeClosure(1, 0), Instr::CallValue(0), Instr::Return],
                0,
                0,
                1,
            ),
            func(
                "body",
                vec![Instr::LoadCaptured(u16::MAX), Instr::Return],
                0,
                0,
                1,
            ),
        ],
        vec![],
    );
    assert!(matches!(
        verify(&modu),
        Err(VerifyError::ClosureFree { .. })
    ));
    assert_trap(&vis_vm::run(&modu, "", &[]), "bytecode hỏng");

    let modu_rt = m(
        vec![func(
            "main",
            vec![Instr::LoadCaptured(u16::MAX), Instr::Return],
            0,
            0,
            1,
        )],
        vec![],
    );
    assert_eq!(verify(&modu_rt), Ok(()));
    assert_trap(
        &vis_vm::run(&modu_rt, "", &[]),
        "biến bắt 65535 ngoài môi trường capture (0 ô)",
    );
}

// ---------------------------------------------------------------------------
// (4) Capture hợp lệ chứa `Unit` ⇒ trả `Unit` bình thường (không bị trap).
//     - **Verifier chấp nhận** (nfree=1 = used=1) và **runtime chạy sạch**.
// ---------------------------------------------------------------------------
#[test]
fn t4_capture_hop_le_chua_unit() {
    let modu = m(
        vec![
            func(
                "main",
                vec![
                    Instr::PushUnit, // capture ô 0 = Unit
                    Instr::MakeClosure(1, 1),
                    Instr::CallValue(0),
                    Instr::Return,
                ],
                0,
                0,
                1,
            ),
            func("body", vec![Instr::LoadCaptured(0), Instr::Return], 0, 0, 1),
        ],
        vec![],
    );
    assert_clean_unit(&modu);
}

// ---------------------------------------------------------------------------
// (5) Closure tạo với môi trường không khớp nhu cầu hàm đích.
//     - MakeClosure(fi, 1) nhưng hàm đích dùng LoadCaptured(3) ⇒ **verifier từ chối**.
// ---------------------------------------------------------------------------
#[test]
fn t5_closure_moi_truong_khong_khop() {
    let modu = m(
        vec![
            func(
                "main",
                vec![
                    Instr::PushUnit,
                    Instr::MakeClosure(1, 1),
                    Instr::CallValue(0),
                    Instr::Return,
                ],
                0,
                0,
                1,
            ),
            func("body", vec![Instr::LoadCaptured(3), Instr::Return], 0, 0, 1),
        ],
        vec![],
    );
    assert!(matches!(
        verify(&modu),
        Err(VerifyError::ClosureFree {
            nfree: 1,
            used: 4,
            ..
        })
    ));
    assert_trap(&vis_vm::run(&modu, "", &[]), "bytecode hỏng");
}

// ---------------------------------------------------------------------------
// (6) Closure lồng nhau + chương trình compiler sinh hợp lệ.
//     - **Verifier chấp nhận** + **runtime chạy đúng** — không có false positive trap.
//     - Khẳng định module biên dịch có cả MakeClosure lẫn LoadCaptured (đúng đường đã sửa).
// ---------------------------------------------------------------------------
#[test]
fn t6_closure_long_nhau_compiler_sinh_hop_le() {
    let src = "fn main():\n    let base = 5\n    let add = fn(x: int) -> int:\n        let bump = fn(y: int) -> int:\n            y + x + base\n        bump(1)\n    print(add(2))\n";
    let parsed = vis_syntax::parse(src, vis_syntax::Mode::Script);
    assert!(
        parsed.diagnostics.is_empty(),
        "parse: {:?}",
        parsed.diagnostics
    );
    let lexed = vis_syntax::lex(src, vis_syntax::Mode::Script);
    let h = vis_types::lower(&parsed.cst, &lexed);
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    let modu = compile(&h.hir, &h.symbols, src).expect("compile");

    let has_make = modu
        .funcs
        .iter()
        .any(|f| f.code.iter().any(|i| matches!(i, Instr::MakeClosure(..))));
    let has_load = modu
        .funcs
        .iter()
        .any(|f| f.code.iter().any(|i| matches!(i, Instr::LoadCaptured(_))));
    assert!(
        has_make && has_load,
        "phải đi qua đường MakeClosure/LoadCaptured"
    );

    assert_eq!(verify(&modu), Ok(()));
    let out = vis_vm::run(&modu, src, &[]);
    assert!(
        out.is_clean(),
        "closure lồng nhau không được trap: {:?}",
        out.diagnostics
    );
    assert_eq!(out.exit, 0);
    assert_eq!(out.stdout, "8\n", "2 + 1 + 5 = 8");
}

// ---------------------------------------------------------------------------
// (7) Local ngoài khung bị chặn; local hợp lệ chứa `Unit` không bị từ chối.
//     - LoadLocal(5) với nlocals=1 ⇒ **verifier từ chối** (LocalOverflow).
//     - StoreLocal/Assign/BindLocal(5) ⇒ **verifier từ chối** (cùng LocalOverflow).
//     - Local 0 đã khởi tạo `Unit` ⇒ **verifier chấp nhận** + **runtime trả Unit**.
//     - Hàng rào runtime `doc_local`/`ghi_local` được test nội bộ trong `vm.rs`.
// ---------------------------------------------------------------------------
#[test]
fn t7_local_ngoai_khung_bi_chan_unit_hop_le() {
    let doc_ngoai = m(
        vec![func(
            "main",
            vec![Instr::LoadLocal(5), Instr::Return],
            0,
            1,
            1,
        )],
        vec![],
    );
    assert!(matches!(
        verify(&doc_ngoai),
        Err(VerifyError::LocalOverflow {
            slot: 5,
            nlocals: 1,
            ..
        })
    ));
    assert_trap(&vis_vm::run(&doc_ngoai, "", &[]), "bytecode hỏng");

    for (ten, ins) in [
        ("StoreLocal", Instr::StoreLocal(5)),
        ("Assign", Instr::Assign(5)),
        ("BindLocal", Instr::BindLocal(5)),
    ] {
        let modu = m(
            vec![func(
                "main",
                vec![Instr::PushUnit, ins, Instr::Return],
                0,
                1,
                1,
            )],
            vec![],
        );
        assert!(
            matches!(verify(&modu), Err(VerifyError::LocalOverflow { .. })),
            "{ten}: ghi local ngoài khung phải bị từ chối"
        );
    }

    let hop_le = m(
        vec![func(
            "main",
            vec![
                Instr::PushUnit,
                Instr::StoreLocal(0),
                Instr::LoadLocal(0),
                Instr::Return,
            ],
            0,
            1,
            1,
        )],
        vec![],
    );
    assert_clean_unit(&hop_le);
}

// ---------------------------------------------------------------------------
// (8) Crafted bytecode qua encode → decode → load_verified → execute.
//     - Entry tự LoadCaptured(0) ⇒ verifier lọt (không có MakeClosure để so), encode/decode vòng
//       tròn giữ nguyên, `load_verified` chấp nhận, nhưng execute ⇒ **runtime trap** (hết thành
//       công giả), exit != 0, chẩn đoán VS0501.
// ---------------------------------------------------------------------------
#[test]
fn t8_crafted_encode_decode_load_verified_execute() {
    let modu = m(
        vec![func(
            "main",
            vec![Instr::LoadCaptured(0), Instr::Return],
            0,
            0,
            1,
        )],
        vec![],
    );
    assert_eq!(verify(&modu), Ok(()));

    let bytes = encode(&modu);
    let decoded = decode(&bytes).expect("decode sạch");
    assert_eq!(decoded, modu, "encode→decode phải giữ nguyên module");
    let loaded = load_verified(&bytes).expect("load_verified chấp nhận (đúng miền chỉ số)");
    assert_eq!(loaded, modu);

    let out = vis_vm::run(&loaded, "", &[]);
    assert_trap(&out, "biến bắt 0 ngoài môi trường capture (0 ô)");
}
