// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-276 / VD-256** — bảng phụ S (kind/site theo PC, nhiều sự kiện có thứ tự) + verifier
//! cấu trúc/CFG. Kiểm **nghĩa vụ B**: bytecode không đáng tin không né phí được.
//! Không tái dựng HIR; không tin trọng số trong artifact (D5).

use vis_bytecode::fee::*;
use vis_bytecode::{encode, encode_with_fee, Const, Func, Instr, Module, Span, HASH_LEN, VERSION};

fn f(code: Vec<Instr>, spans: Vec<Span>, max_stack: u16) -> Func {
    Func {
        name: "f".into(),
        arity: 0,
        nparams: 0,
        nlocals: 2,
        max_stack,
        code,
        spans,
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

fn pr() -> Const {
    Const {
        kind: 5,
        text: "print".to_string(),
    }
}

/// `Const(0); CallBuiltin(1,1); Return` + spans rộng足够的 site.
fn hop_le() -> Module {
    m(
        vec![f(
            vec![Instr::Const(0), Instr::CallBuiltin(1, 1), Instr::Return],
            vec![
                Span { start: 10, end: 20 },
                Span { start: 20, end: 30 },
                Span { start: 30, end: 40 },
            ],
            2,
        )],
        vec![
            Const {
                kind: 0,
                text: "1".to_string(),
            },
            pr(),
        ],
    )
}

fn feehop() -> FeeTable {
    let mut t = FeeTable::new(1);
    t.push(0, FeeEntry::new(0, FeeKind::Expr, 12));
    t.push(0, FeeEntry::new(1, FeeKind::Builtin, 22));
    t
}

#[test]
fn trailer_chi_chua_kind_site_khong_chua_trong_so() {
    let (m, fee) = (hop_le(), feehop());
    let bytes = encode_with_fee(&m, &fee);
    // VD-257: phần VBC1 trước trailer **giống encode thường** trừ đúng 2 byte version (2 → 3).
    // MAGIC/opcode/bố cục không đổi; chỉ giá trị version lõi mang hàng rào tương thích.
    let plain = encode(&m);
    assert_eq!(
        bytes.len(),
        plain.len() + 4 + 2 + 2 + 4 + 4 * fee.funcs.len() + fee.total() * 9
    );
    assert_eq!(&bytes[..4], &plain[..4], "MAGIC không đổi");
    assert_eq!(
        u16::from_le_bytes([bytes[4], bytes[5]]),
        vis_bytecode::INSTRUMENTED_VERSION,
        "artifact có trailer phải đóng dấu module version 3"
    );
    assert_eq!(
        &bytes[6..plain.len()],
        &plain[6..],
        "phần lõi sau version giữ nguyên"
    );
    let got = decode_fee(&bytes).expect("decode fee");
    assert_eq!(got, fee);
    assert_eq!(got.trace()[0].label(), "E_EXPR@12");
    assert_eq!(got.trace()[1].label(), "E_BUILTIN@22");
    // Không có trailer ⇒ Absent (artifact cũ / đường mặc định).
    assert_eq!(decode_fee(&encode(&m)), Err(FeeDecodeError::Absent));
    // VERSION phí = 4, không phải 3 (D7).
    assert_eq!(vis_bytecode::fee::VERSION, 4);
}

#[test]
fn verifier_structure_va_cfg_hop_le() {
    let m = hop_le();
    assert_eq!(verify_fee(&m, &feehop()), Ok(()));
    assert!(vis_bytecode::load_verified_with_fee(&encode_with_fee(&m, &feehop())).is_ok());
}

#[test]
fn nhieu_su_kien_cung_pc_giu_thu_tu_trong_artifact() {
    let mut fee = FeeTable::new(1);
    fee.push(0, FeeEntry::new(1, FeeKind::Call, 21));
    fee.push(0, FeeEntry::new(1, FeeKind::Expr, 22));
    fee.push(0, FeeEntry::new(1, FeeKind::Assign, 23));
    let bytes = encode_with_fee(&hop_le(), &fee);
    let got = decode_fee(&bytes).unwrap();
    assert_eq!(got.funcs[0].len(), 3);
    assert_eq!(
        got.trace().iter().map(|e| e.kind).collect::<Vec<_>>(),
        vec![FeeKind::Call, FeeKind::Expr, FeeKind::Assign]
    );
    assert_eq!(verify_fee(&hop_le(), &got), Ok(()));
}

#[test]
fn bang_rong_khong_duoc_chap_nhan_la_khong_ton_cong() {
    let m = hop_le();
    assert_eq!(
        verify_fee(&m, &FeeTable::new(1)),
        Err(FeeVerifyError::EmptyTable)
    );
}

#[test]
fn pc_va_site_sai_bi_tu_choi() {
    let m = hop_le();
    let mut fee = feehop();
    fee.funcs[0][0].pc = 99;
    assert!(matches!(
        verify_fee(&m, &fee),
        Err(FeeVerifyError::BadPc { .. })
    ));

    let mut fee = feehop();
    fee.funcs[0][0].site = 999; // nằm ngoài span [10,20) của lệnh pc 0
    assert!(matches!(
        verify_fee(&m, &fee),
        Err(FeeVerifyError::BadSite { .. })
    ));

    let mut fee = feehop();
    fee.funcs.push(Vec::new()); // số hàm khác module
    assert!(matches!(
        verify_fee(&m, &fee),
        Err(FeeVerifyError::FuncCount { .. })
    ));
}

#[test]
fn chu_trinh_cfg_khong_co_phi_bi_tu_choi() {
    // `Jump(0)` tự nhảy ⇒ chu trình, không có điểm phí ⇒ từ chối.
    let m = m(
        vec![f(vec![Instr::Jump(-1)], vec![Span { start: 0, end: 1 }], 1)],
        vec![],
    );
    let fee = FeeTable::new(1);
    assert!(matches!(
        verify_fee(&m, &fee),
        Err(FeeVerifyError::CycleUncharged { .. }) | Err(FeeVerifyError::LoopGuardUncharged { .. })
    ));

    // Cùng chu trình nhưng có E_LOOP tại header ⇒ qua.
    let mut fee = FeeTable::new(1);
    fee.push(0, FeeEntry::new(0, FeeKind::Loop, 0));
    assert_eq!(verify_fee(&m, &fee), Ok(()));
}

#[test]
fn canh_nhay_lui_vao_dau_vong_bo_guard_bi_tu_choi() {
    // `JumpIfFalse(+2); Jump(-2); Return` — cạnh lùi vào pc 0; pc 0 có phí ⇒ guard OK.
    let m = m(
        vec![f(
            vec![Instr::JumpIfFalse(2), Instr::Jump(-2), Instr::Return],
            vec![Span { start: 0, end: 3 }; 3],
            1,
        )],
        vec![],
    );
    let mut fee = FeeTable::new(1);
    fee.push(0, FeeEntry::new(1, FeeKind::Loop, 1));
    assert!(matches!(
        verify_fee(&m, &fee),
        Err(FeeVerifyError::LoopGuardUncharged { .. })
    ));
    fee.push(0, FeeEntry::new(0, FeeKind::Loop, 0));
    assert_eq!(verify_fee(&m, &fee), Ok(()));
}

#[test]
fn de_quy_qua_call_khong_co_phi_bi_tu_choi() {
    // f: `Call(0,0); Halt` — tự gọi ⇒ chu trình đệ quy không điểm phí.
    let m = m(
        vec![f(
            vec![Instr::Call(0, 0), Instr::Halt],
            vec![Span { start: 0, end: 2 }; 2],
            1,
        )],
        vec![],
    );
    assert!(matches!(
        verify_fee(&m, &FeeTable::new(1)),
        Err(FeeVerifyError::CallCycleUncharged { .. })
    ));
    let mut fee = FeeTable::new(1);
    fee.push(0, FeeEntry::new(0, FeeKind::Call, 0));
    assert_eq!(verify_fee(&m, &fee), Ok(()));
}

#[test]
fn tag_su_kien_la_bi_tu_choi() {
    let mut bytes = encode_with_fee(&hop_le(), &feehop());
    let at = bytes.windows(4).rposition(|w| w == FEE_MAGIC).unwrap();
    // pc(4) + kind(1) ⇒ trailer bắt đầu tại at+4+2+2+4+4 = at+16; kind của entry đầu ở at+16+4.
    let kind_at = at + 16 + 4;
    bytes[kind_at] = 99;
    assert_eq!(decode_fee(&bytes), Err(FeeDecodeError::BadKind(99)));
}
