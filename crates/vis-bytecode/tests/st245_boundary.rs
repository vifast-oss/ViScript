// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-245 — bảo vệ bản sửa ST-238 bằng test âm + test biên** (A0 bổ sung sau ST-238).
//!
//! Đối tượng là **miền ô** của `LoadLocal`/`StoreLocal`/`Assign`/`BindLocal` sau khi ST-238 dùng
//! `GLOBAL_SLOT_BASE = 0x8000` để tách ô cục bộ và ô biến cấp tệp. Các ca ở đây đều dựng `Module`
//! bằng tay (đúng cái verifier thấy) — **không** phụ thuộc codegen — và phải **đỏ** trên verifier
//! cũ (chỉ kiểm `s < 0x8000 && s >= nlocals`, còn mọi ô `>= 0x8000` được miễn trừ vô điều kiện).
//!
//! Bốn nhóm yêu cầu A0:
//! 1. slot local không va vào miền `GLOBAL_SLOT_BASE`;
//! 2. chỉ số global bị chặn bởi **số global thực tế** (suy từ lệnh ghi), không phải "cứ >= 0x8000";
//! 3. bytecode sai không lợi dụng miễn trừ `LocalOverflow` để vượt kiểm tra (cả đường `load_verified`);
//! 4. ảnh hưởng tương thích được ghi rõ trong `format.rs::GLOBAL_SLOT_BASE` (không phải chỉ "không thêm opcode").

use vis_bytecode::verify::verify;
use vis_bytecode::verify::{load_verified, LoadError, VerifyError};
use vis_bytecode::{encode, Func, Instr, Module, Span, GLOBAL_SLOT_BASE, HASH_LEN, VERSION};

const BASE: u16 = GLOBAL_SLOT_BASE;

fn f(code: Vec<Instr>, nlocals: u16, max_stack: u16) -> Func {
    Func {
        name: "main".into(),
        arity: 0,
        nparams: 0,
        nlocals,
        max_stack,
        spans: vec![Span { start: 0, end: 0 }; code.len()],
        code,
    }
}

fn m(funcs: Vec<Func>) -> Module {
    Module {
        version: VERSION,
        source_hash: [0u8; HASH_LEN],
        consts: vec![],
        funcs,
        entry: 0,
    }
}

/// Module hợp lệ: `writes` là các offset global được **định nghĩa** (phát `StoreLocal(BASE+k)`),
/// `tail` là lệnh kiểm; luôn kết thúc bằng `Return`. Không dùng const/call nên không cần const pool.
fn module_voi(writes: &[u16], tail: Vec<Instr>) -> Module {
    let mut code = Vec::new();
    for &k in writes {
        code.push(Instr::PushUnit);
        code.push(Instr::StoreLocal(BASE + k));
    }
    code.extend(tail);
    if !matches!(code.last(), Some(Instr::Return)) {
        code.push(Instr::Return);
    }
    m(vec![f(code, 0, 2)])
}

// ---------------------------------------------------------------------------
// (1) Slot local không được va vào miền GLOBAL_SLOT_BASE.
// ---------------------------------------------------------------------------

#[test]
fn nlocals_vuot_global_base_bi_tu_choi() {
    // Nếu nlocals > 0x8000 thì các ô [nparams, nlocals) tràn sang vùng global: codegen có thể
    // phát một `LoadLocal(0x8000)` mà verifier cũ hiểu nhầm là global ⇒ va miền. Phải chặn ở nguồn.
    let x = m(vec![f(vec![Instr::Return], BASE + 1, 1)]);
    assert_eq!(
        verify(&x),
        Err(VerifyError::BadArity {
            func: 0,
            arity: 0,
            nparams: 0,
            nlocals: BASE + 1,
        })
    );
}

#[test]
fn nlocals_bang_global_base_la_bien_bi() {
    // Biên: nlocals == 0x8000 ⇒ ô local cao nhất 0x7FFF vẫn nằm trọn dưới vùng global.
    let x = m(vec![f(
        vec![Instr::LoadLocal(BASE - 1), Instr::Return],
        BASE,
        1,
    )]);
    assert_eq!(verify(&x), Ok(()));
}

// ---------------------------------------------------------------------------
// (2) Chỉ số global bị chặn bởi số global thực tế.
// ---------------------------------------------------------------------------

#[test]
fn doc_global_thu_the_duoc_chap_nhan_bien() {
    // 1 global thật (offset 0) ⇒ LoadLocal(BASE+0) là biên hợp lệ.
    let x = module_voi(&[0], vec![Instr::LoadLocal(BASE)]);
    assert_eq!(verify(&x), Ok(()));
}

#[test]
fn doc_global_vuot_so_thuc_te_bi_tu_choi() {
    // Chỉ có 1 global, nhưng đọc offset 1 ⇒ phải báo GlobalOverflow, không phải "cứ >= BASE là được".
    let x = module_voi(&[0], vec![Instr::LoadLocal(BASE + 1)]);
    assert_eq!(
        verify(&x),
        Err(VerifyError::GlobalOverflow {
            func: 0,
            pc: 2,
            slot: 1,
            nglobals: 1,
        })
    );
}

#[test]
fn hai_global_chi_doc_duoc_offset_0_va_1() {
    let x = module_voi(&[0, 1], vec![Instr::LoadLocal(BASE + 1)]);
    assert_eq!(verify(&x), Ok(()));
    let mut y = module_voi(&[0, 1], vec![Instr::LoadLocal(BASE + 2)]);
    assert!(matches!(
        verify(&y),
        Err(VerifyError::GlobalOverflow {
            slot: 2,
            nglobals: 2,
            ..
        })
    ));
    // Ép biên đúng ở offset cuối cùng (1) vẫn qua.
    y.funcs[0].code[4] = Instr::LoadLocal(BASE + 1);
    assert_eq!(verify(&y), Ok(()));
}

#[test]
fn khong_co_global_ma_doc_global_bi_tu_choi() {
    let x = module_voi(&[], vec![Instr::LoadLocal(BASE)]);
    assert_eq!(
        verify(&x),
        Err(VerifyError::GlobalOverflow {
            func: 0,
            pc: 0,
            slot: 0,
            nglobals: 0,
        })
    );
}

// ---------------------------------------------------------------------------
// (3) Không lợi dụng miễn trừ LocalOverflow để vượt kiểm tra / gây panic runtime.
// ---------------------------------------------------------------------------

#[test]
fn ghi_global_ngoai_mien_cung_bi_tu_choi() {
    // Dù chính lệnh ghi nằm trong module (bị pre-pass đếm vào tập global), số global là **số ô**,
    // không phải "max offset + 1": offset 5 với chỉ 2 ô được ghi ⇒ vẫn tràn miền.
    let x = module_voi(&[0], vec![Instr::PushUnit, Instr::StoreLocal(BASE + 5)]);
    assert!(matches!(
        verify(&x),
        Err(VerifyError::GlobalOverflow { slot: 5, .. })
    ));
}

#[test]
fn slot_global_cao_nhat_bi_tu_choi_khong_panic() {
    // 0xFFFF là ô global offset 0x7FFF — verifier cũ miễn trừ hoàn toàn; VM cũ thì `.get()` trả
    // Unit (đọc) / `resize(k+1)` (ghi). Phải bị chặn ở verifier, không để lọt tới VM.
    let doc = module_voi(&[0], vec![Instr::LoadLocal(u16::MAX)]);
    assert!(matches!(
        verify(&doc),
        Err(VerifyError::GlobalOverflow {
            slot: 0x7FFF,
            nglobals: 1,
            ..
        })
    ));
    let ghi = module_voi(&[0], vec![Instr::PushUnit, Instr::StoreLocal(u16::MAX)]);
    assert!(matches!(
        verify(&ghi),
        Err(VerifyError::GlobalOverflow { slot: 0x7FFF, .. })
    ));
}

#[test]
fn load_verified_chan_global_ngoai_mien_tu_byte() {
    // Đường nạp từ cache/dĩa: encode → decode → verify. Bytecode hỏng phải bị chặn ở đây.
    let x = module_voi(&[0], vec![Instr::LoadLocal(BASE + 9)]);
    let bytes = encode(&x);
    assert!(matches!(
        load_verified(&bytes),
        Err(LoadError::Verify(VerifyError::GlobalOverflow {
            slot: 9,
            ..
        }))
    ));

    // Module sạch (1 global, đúng offset 0) vẫn qua cả hai đường.
    let sach = module_voi(&[0], vec![Instr::LoadLocal(BASE)]);
    assert_eq!(load_verified(&encode(&sach)), Ok(sach));
}
