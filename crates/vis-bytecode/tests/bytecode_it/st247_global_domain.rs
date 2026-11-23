// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-247 — làm rõ miền global suy từ lệnh ghi** (VD-244 §3 / `BLOCKED.md` §14).
//!
//! Năm câu A0, trả lời bằng test trên **chính cái verifier thấy** (dựng `Module` tay, không phụ
//! thuộc codegen):
//! 1. `StoreLocal` ô global rất cao có tự nới miền chấp nhận không?
//! 2. Chỉ số global có vượt miền mã hóa `u16` không?
//! 3. Slot có lỗ hổng (chưa khởi tạo) có bị đọc thành `Unit` "thành công" không?
//! 4. Ghi trong nhánh không chạy / hàm không gọi ⇒ đọc-trước-ghi thế nào?
//! 5. Miền verifier có khớp miền runtime cấp phát/truy cập không?
//!
//! **Phân biệt**: hợp lệ **chỉ số** (ô nằm trong `[0, nglobals)`) ≠ hợp lệ **khởi tạo** (ô có giá trị
//! đúng lúc đọc). Bằng chứng runtime (giá trị `Unit`, không chỉ "không panic") nằm ở
//! `crates/vis-vm/tests/st247_global_domain.rs`.
//!
//! Cơ chế đang được kiểm chứng (ST-245): VBC1 không serialize `nglobals`; verifier **suy từ lệnh
//! ghi** — `nglobals = số ô vùng GLOBAL_SLOT_BASE có ít nhất một lệnh StoreLocal/Assign/BindLocal`
//! trong toàn module — rồi chặn mọi truy cập `s - BASE >= nglobals` bằng `GlobalOverflow`.

use vis_bytecode::verify::{load_verified, verify, LoadError, VerifyError};
use vis_bytecode::{
    decode, encode, Const, Func, Instr, Module, Span, GLOBAL_SLOT_BASE, HASH_LEN, VERSION,
};

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

fn named(name: &str, code: Vec<Instr>, max_stack: u16) -> Func {
    let mut fun = f(code, 0, max_stack);
    fun.name = name.into();
    fun
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

/// Module một hàm: ghi các offset global `writes` (liên tiếp hay không tùy ca), rồi `tail`, rồi `Return`.
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
    m(vec![f(code, 0, 2)], vec![])
}

// ---------------------------------------------------------------------------
// (1) Ghi ô global rất cao KHÔNG tự nới miền chấp nhận.
// ---------------------------------------------------------------------------

#[test]
fn q1_ghi_global_rat_cao_khong_tu_nor_mien() {
    // Chỉ 1 global thật (offset 0) ⇒ đọc offset 1 bị chặn.
    let a = module_voi(&[0], vec![Instr::LoadLocal(BASE + 1)]);
    assert!(matches!(
        verify(&a),
        Err(VerifyError::GlobalOverflow {
            slot: 1,
            nglobals: 1,
            ..
        })
    ));

    // Thêm một lệnh GHI ở offset 1000. Pre-pass đếm **số ô được ghi** (2: offset 0 và 1000),
    // KHÔNG phải max offset + 1 (1001). Chính lệnh ghi offset 1000 tự bị từ chối ⇒ module hỏng;
    // đọc offset 1 vẫn không được mở. Nghĩa là một `StoreLocal` chỉ số cao không "tự làm tăng miền".
    let b = module_voi(
        &[0],
        vec![
            Instr::PushUnit,
            Instr::StoreLocal(BASE + 1000),
            Instr::LoadLocal(BASE + 1),
        ],
    );
    assert!(
        matches!(
            verify(&b),
            Err(VerifyError::GlobalOverflow {
                slot: 1000,
                nglobals: 2,
                ..
            })
        ),
        "ghi ô cao phải tự bị chặn, miền không nới theo trị chỉ số: {:?}",
        verify(&b)
    );
}

#[test]
fn q1_mien_bang_so_lenh_ghi_khong_phai_tri_chi_so() {
    // Miền chấp nhận đúng bằng **số lệnh ghi** (giả sử liên tiếp). N là giới hạn cứng ⇒ một ô cao
    // chỉ mở được nếu module thực sự có đủ N lệnh ghi các ô thấp hơn.
    for n in 1..=8u16 {
        let writes: Vec<u16> = (0..n).collect();
        let mut tail = Vec::new();
        for k in 0..n {
            tail.push(Instr::LoadLocal(BASE + k));
            tail.push(Instr::Pop);
        }
        tail.push(Instr::PushUnit); // để lại 1 giá trị cho Return (Return pop 1)
        assert_eq!(verify(&module_voi(&writes, tail)), Ok(()), "n = {n}");
        assert!(matches!(
            verify(&module_voi(&writes, vec![Instr::LoadLocal(BASE + n)])),
            Err(VerifyError::GlobalOverflow { slot, nglobals, .. }) if slot == n && nglobals == n
        ));
    }
}

// ---------------------------------------------------------------------------
// (2) Chỉ số global nằm trọn trong miền mã hóa u16; phần vượt bị verifier chặn.
// ---------------------------------------------------------------------------

#[test]
fn q2_u16_la_gioi_han_ma_hoa_va_verifier_chan_o_bien() {
    // Toán hạng ô là u16 ⇒ offset global tối đa 0xFFFF - 0x8000 = 0x7FFF. Không có đường mã hóa
    // chỉ số > u16 (decode đọc đúng 2 byte little-endian).
    assert_eq!(u16::MAX - BASE, 0x7FFF);
    let doc = module_voi(&[0], vec![Instr::LoadLocal(u16::MAX)]);
    assert!(matches!(
        verify(&doc),
        Err(VerifyError::GlobalOverflow {
            slot: 0x7FFF,
            nglobals: 1,
            ..
        })
    ));

    // encode → decode giữ nguyên 0xFFFF (không cắt), rồi load_verified chặn ở verify.
    let ghi = module_voi(&[0], vec![Instr::PushUnit, Instr::StoreLocal(u16::MAX)]);
    let bytes = encode(&ghi);
    let decoded = decode(&bytes).expect("decode sạch");
    assert!(
        decoded.funcs[0].code.contains(&Instr::StoreLocal(u16::MAX)),
        "decode không được cắt u16"
    );
    assert!(
        matches!(
            load_verified(&bytes),
            Err(LoadError::Verify(VerifyError::GlobalOverflow {
                slot: 0x7FFF,
                ..
            }))
        ),
        "cache/path nạp từ byte phải chặn ô vượt miền"
    );
}

// ---------------------------------------------------------------------------
// (3) Lỗ hổng (ô chưa khởi tạo) không được chấp nhận — chỉ số phải liên tục từ 0.
// ---------------------------------------------------------------------------

#[test]
fn q3_lo_hong_bi_tu_choi_khong_doc_thanh_unit() {
    // Ghi {0, 2} ⇒ lỗ hổng ở 1. Số global đếm được là 2 ⇒ chính lệnh ghi offset 2 bị từ chối
    // (2 >= 2). Nghĩa là module có lỗ hổng KHÔNG thể qua verifier; không có ô "hở" để đọc.
    let lo = module_voi(&[0, 2], vec![Instr::LoadLocal(BASE + 1)]);
    assert!(
        matches!(
            verify(&lo),
            Err(VerifyError::GlobalOverflow {
                slot: 2,
                nglobals: 2,
                ..
            })
        ),
        "module có lỗ hổng phải bị chặn ở lệnh ghi cao nhất: {:?}",
        verify(&lo)
    );

    // Đọc thẳng vào lỗ hổng (offset 1) khi mới chỉ ghi offset 0: nglobals = 1 ⇒ chặn.
    let doc_lo = module_voi(&[0], vec![Instr::LoadLocal(BASE + 1)]);
    assert!(matches!(
        verify(&doc_lo),
        Err(VerifyError::GlobalOverflow {
            slot: 1,
            nglobals: 1,
            ..
        })
    ));

    // Đối chứng: ghi liên tiếp {0,1,2} ⇒ đọc 0..2 đều hợp lệ, không có ô chưa khởi tạo trong miền.
    let lien = module_voi(
        &[0, 1, 2],
        vec![
            Instr::LoadLocal(BASE),
            Instr::Pop,
            Instr::LoadLocal(BASE + 2),
        ],
    );
    assert_eq!(verify(&lien), Ok(()));
}

// ---------------------------------------------------------------------------
// (4) Ghi trong hàm không gọi / nhánh không chạy vẫn nới miền (tĩnh) — chỗ này verifier KHÔNG
//     kiểm khởi tạo theo thứ tự chạy. Bằng chứng runtime nằm ở test của vis-vm.
// ---------------------------------------------------------------------------

#[test]
fn q4_ghi_trong_ham_khong_goi_van_nor_mien_verify() {
    // f0 (entry): ghi global 0 rồi ĐỌC global 1. f1 (không bao giờ được gọi): ghi global 1.
    let f0 = f(
        vec![
            Instr::PushUnit,
            Instr::StoreLocal(BASE),
            Instr::LoadLocal(BASE + 1),
            Instr::Return,
        ],
        0,
        1,
    );
    let f1 = named(
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
    // Verifier đếm TĨNH lệnh ghi ở f1 ⇒ nglobals = 2 ⇒ đọc global 1 ở f0 vượt kiểm CHỈ SỐ.
    // Đây là lỗ hổng "hợp lệ chỉ số nhưng chưa khởi tạo": runtime trả Unit (xem test vis-vm).
    assert_eq!(
        verify(&modu),
        Ok(()),
        "verifier chấp nhận đọc ô global chỉ được ghi trong hàm không gọi"
    );
}

#[test]
fn q4_ghi_trong_nhanh_khong_chay_van_nor_mien_verify() {
    // consts[0] = false. main: ghi global 0; nhánh false bỏ qua lệnh ghi global 1; rồi đọc global 1.
    let code = vec![
        Instr::PushUnit,             // 0
        Instr::StoreLocal(BASE),     // 1  ghi global 0
        Instr::Const(0),             // 2  false
        Instr::JumpIfFalse(3),       // 3  -> pc 6 (bỏ qua pc 4..5)
        Instr::PushUnit,             // 4
        Instr::StoreLocal(BASE + 1), // 5  ghi global 1 (KHÔNG chạy)
        Instr::LoadLocal(BASE + 1),  // 6  đọc global 1 (chưa khởi tạo lúc chạy)
        Instr::Return,               // 7
    ];
    let modu = m(
        vec![f(code, 0, 1)],
        vec![Const {
            kind: 9, // Bool
            text: "false".into(),
        }],
    );
    assert_eq!(
        verify(&modu),
        Ok(()),
        "verifier chấp nhận đọc ô global chỉ được ghi trong nhánh không chạy"
    );
}

// ---------------------------------------------------------------------------
// (5) Miền verifier khớp miền runtime khi (và chỉ khi) mọi lệnh ghi đều nằm trên đường chạy.
// ---------------------------------------------------------------------------

#[test]
fn q5_mien_verify_khop_mien_cap_phat_voi_codegen() {
    // Codegen cấp ô liên tiếp từ BASE cho mỗi let/var cấp tệp và phát lệnh ghi trong main ⇒ mọi ô
    // trong [0, N) đều được resize ở runtime; mọi truy cập đều nằm trong miền đã cấp. Không có ô nào
    // nằm ngoài [0, N) để VM phải `.unwrap_or(Unit)`.
    for n in 1..=8u16 {
        let writes: Vec<u16> = (0..n).collect();
        let mut tail = Vec::new();
        for k in 0..n {
            tail.push(Instr::LoadLocal(BASE + k));
            tail.push(Instr::Pop);
        }
        tail.push(Instr::PushUnit); // để lại 1 giá trị cho Return (Return pop 1)
        let modu = module_voi(&writes, tail);
        assert_eq!(verify(&modu), Ok(()), "n = {n}");
        // Và mọi ô ngoài [0, N) bị chặn trước khi VM kịp resize/đọc Unit — khớp hai đầu.
        assert!(matches!(
            verify(&module_voi(&writes, vec![Instr::LoadLocal(BASE + n)])),
            Err(VerifyError::GlobalOverflow { .. })
        ));
    }
}
