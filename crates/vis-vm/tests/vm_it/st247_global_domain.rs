// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-247 — bằng chứng runtime** cho miền global suy từ lệnh ghi (VD-244 §3 / `BLOCKED.md` §14).
//!
//! Phần verify (chỉ số) nằm ở `crates/vis-bytecode/tests/st247_global_domain.rs`. Ở đây dựng tay
//! module mà verifier **chấp nhận**, rồi chạy VM và **so giá trị trả về** — không dùng `catch_unwind`
//! làm bằng chứng. Mục đích: phân biệt **hợp lệ chỉ số** ( qua verify ) với **hợp lệ khởi tạo**
//! ( ô có giá trị đúng lúc đọc ).
//!
//! Hai ca (4) dưới từng chứng minh lỗi: nếu lệnh ghi nằm ở hàm không gọi / nhánh không chạy,
//! verifier vẫn đếm nó và mở miền, nhưng runtime không bao giờ ghi ⇒ `unwrap_or(Unit)` trả `Unit`
//! "thành công". **ST-250 (VD-245) đã sửa**: giờ các ca này trap có chẩn đoán; assertion được cập
//! nhật theo hợp đồng mới (chi tiết 8 ca ở `tests/st250_global_init.rs`).

use vis_bytecode::verify::verify;
use vis_bytecode::{Const, Func, Instr, Module, Span, GLOBAL_SLOT_BASE, HASH_LEN, VERSION};

const BASE: u16 = GLOBAL_SLOT_BASE;

fn f(name: &str, code: Vec<Instr>, max_stack: u16) -> Func {
    Func {
        name: name.into(),
        arity: 0,
        nparams: 0,
        nlocals: 0,
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

// **ST-250 (VD-245)**: ca (4) giờ là trap có chẩn đoán, không còn trả `Unit` "thành công".
// Bằng chứng đầy đủ 8 ca nằm ở `tests/st250_global_init.rs`.
#[test]
fn q4_ham_khong_goi_doc_global_chua_khoi_tao_tra_trap() {
    // f0 (entry): ghi global 0 rồi trả global 1. f1 (không gọi): ghi global 1. Verifier đếm tĩnh
    // hai lệnh ghi ⇒ nglobals = 2 ⇒ đọc global 1 ở f0 qua kiểm chỉ số.
    let f0 = f(
        "main",
        vec![
            Instr::PushUnit,
            Instr::StoreLocal(BASE),
            Instr::LoadLocal(BASE + 1),
            Instr::Return,
        ],
        1,
    );
    let f1 = f(
        "khong_goi",
        vec![
            Instr::PushUnit,
            Instr::StoreLocal(BASE + 1),
            Instr::PushUnit,
            Instr::Return,
        ],
        1,
    );
    let modu = m(vec![f0, f1], vec![]);
    assert_eq!(verify(&modu), Ok(()));

    let out = vis_vm::run(&modu, "", &[]);
    assert!(
        !out.is_clean(),
        "ST-250: phải trap có chẩn đoán, không được trả Unit thành công: {out:?}"
    );
    assert_ne!(out.exit, 0, "ST-250: exit phải khác 0");
    assert!(out.value.is_none(), "ST-250: trap không trả giá trị");
    assert!(
        out.diagnostics[0]
            .message
            .contains("global chưa được khởi tạo"),
        "ST-250: thông điệp phải xác định global chưa khởi tạo: {:?}",
        out.diagnostics[0].message
    );
}

#[test]
fn q4_nhanh_khong_chay_doc_global_chua_khoi_tao_tra_trap() {
    // consts[0] = false. main: ghi global 0; nhánh false bỏ qua ghi global 1; rồi trả global 1.
    let code = vec![
        Instr::PushUnit,             // 0
        Instr::StoreLocal(BASE),     // 1  ghi global 0
        Instr::Const(0),             // 2  false
        Instr::JumpIfFalse(3),       // 3  -> pc 6
        Instr::PushUnit,             // 4
        Instr::StoreLocal(BASE + 1), // 5  ghi global 1 (bỏ qua)
        Instr::LoadLocal(BASE + 1),  // 6  đọc global 1 (chưa khởi tạo lúc chạy)
        Instr::Return,               // 7
    ];
    let modu = m(
        vec![f("main", code, 1)],
        vec![Const {
            kind: 9, // Bool
            text: "false".into(),
        }],
    );
    assert_eq!(verify(&modu), Ok(()));

    let out = vis_vm::run(&modu, "", &[]);
    assert!(
        !out.is_clean(),
        "ST-250: phải trap có chẩn đoán, không được trả Unit thành công: {out:?}"
    );
    assert_ne!(out.exit, 0, "ST-250: exit phải khác 0");
    assert!(out.value.is_none(), "ST-250: trap không trả giá trị");
    assert!(
        out.diagnostics[0]
            .message
            .contains("global chưa được khởi tạo"),
        "ST-250: thông điệp phải xác định global chưa khởi tạo: {:?}",
        out.diagnostics[0].message
    );
}

#[test]
fn q5_codegen_ghi_roi_doc_tra_gia_tri_dung() {
    // Đối chứng: module "lành" (ghi xong mới đọc, đúng miền [0,1)) ⇒ runtime trả đúng 42, không Unit.
    let modu = m(
        vec![f(
            "main",
            vec![
                Instr::Const(0),
                Instr::StoreLocal(BASE),
                Instr::LoadLocal(BASE),
                Instr::Return,
            ],
            1,
        )],
        vec![Const {
            kind: 0, // Int
            text: "42".into(),
        }],
    );
    assert_eq!(verify(&modu), Ok(()));
    let out = vis_vm::run(&modu, "", &[]);
    assert!(out.is_clean(), "diag: {:?}", out.diagnostics);
    assert_eq!(
        out.value.as_ref().map(vis_interp::show).as_deref(),
        Some("42"),
        "đọc sau ghi phải trả 42"
    );
}
