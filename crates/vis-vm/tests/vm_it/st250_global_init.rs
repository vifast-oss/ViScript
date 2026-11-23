// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-250 — global chưa khởi tạo ⇒ runtime trap** (VD-245 / `BLOCKED.md` §14).
//!
//! Tám ca nghiệm thu VD-245 §4. Khác ST-247 (chỉ chứng minh lỗi: đọc ô chưa khởi tạo trả `Unit`
//! "thành công"), file này chứng minh **hợp đồng mới**: mọi đường đọc global đi qua một kiểm tra
//! duy nhất; ô chưa khởi tạo trap có chẩn đoán (`exit != 0`), ô đã ghi `Unit` trả đúng `Unit`, ô đã
//! ghi giá trị khác trả đúng giá trị. Verifier **vẫn** giữ miền chỉ số; VM **không** resize bộ nhớ
//! theo chỉ số bytecode và **không** dùng `Unit` làm dấu chưa khởi tạo.

use vis_bytecode::verify::{load_verified, verify, VerifyError};
use vis_bytecode::{
    decode, encode, Const, Func, Instr, Module, Span, GLOBAL_SLOT_BASE, HASH_LEN, VERSION,
};
use vis_interp::RunOutput;

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

fn cint(text: &str) -> Const {
    Const {
        kind: 0, // Int
        text: text.into(),
    }
}

fn cbool(text: &str) -> Const {
    Const {
        kind: 9, // Bool
        text: text.into(),
    }
}

fn show(out: &RunOutput) -> Option<String> {
    out.value.as_ref().map(vis_interp::show)
}

/// Khẳng định đây là **trap runtime chưa khởi tạo**, có chẩn đoán xác định và `exit != 0`.
fn assert_chua_khoi_tao(out: &RunOutput, o: usize) {
    assert!(!out.is_clean(), "ca {o}: phải có chẩn đoán, got {out:?}");
    assert_ne!(out.exit, 0, "ca {o}: exit phải khác 0");
    assert!(out.value.is_none(), "ca {o}: trap không được trả giá trị");
    let diag = &out.diagnostics[0];
    assert_eq!(
        diag.code.short, "VS0501",
        "ca {o}: phải dùng mã runtime hiện hành"
    );
    assert!(
        diag.message.contains("global chưa được khởi tạo"),
        "ca {o}: thông điệp phải xác định global chưa khởi tạo: {:?}",
        diag.message
    );
    assert!(
        diag.message.contains(&format!("ô {o}")),
        "ca {o}: thông điệp phải chỉ đúng ô: {:?}",
        diag.message
    );
}

// ---------------------------------------------------------------------------
// (1) Ghi trong nhánh không chạy, sau đó đọc ⇒ trap.
// ---------------------------------------------------------------------------
#[test]
fn t1_ghi_nhanh_khong_chay_doc_tra_trap() {
    // pc1 nhảy tới pc4, bỏ qua pc2–pc3 (ghi global 0). Verifier vẫn đếm lệnh ghi ⇒ nglobals = 1.
    let modu = m(
        vec![f(
            "main",
            vec![
                Instr::Const(0),         // 0  false
                Instr::JumpIfFalse(3),   // 1  -> pc 4
                Instr::PushUnit,         // 2  (bỏ qua)
                Instr::StoreLocal(BASE), // 3  (bỏ qua)
                Instr::LoadLocal(BASE),  // 4  đọc global 0 — chưa khởi tạo lúc chạy
                Instr::Return,           // 5
            ],
            1,
        )],
        vec![cbool("false")],
    );
    assert_eq!(verify(&modu), Ok(()));
    let out = vis_vm::run(&modu, "", &[]);
    assert_chua_khoi_tao(&out, 0);
}

// ---------------------------------------------------------------------------
// (2) Ghi trong hàm chưa gọi, sau đó đọc ⇒ trap.
// ---------------------------------------------------------------------------
#[test]
fn t2_ghi_ham_khong_goi_doc_tra_trap() {
    // f1 (không gọi) mang lệnh ghi để verifier mở miền; runtime không bao giờ chạy nó.
    let f0 = f("main", vec![Instr::LoadLocal(BASE), Instr::Return], 1);
    let f1 = f(
        "khong_goi",
        vec![
            Instr::PushUnit,
            Instr::StoreLocal(BASE),
            Instr::PushUnit,
            Instr::Return,
        ],
        1,
    );
    let modu = m(vec![f0, f1], vec![]);
    assert_eq!(verify(&modu), Ok(()));
    let out = vis_vm::run(&modu, "", &[]);
    assert_chua_khoi_tao(&out, 0);
}

// ---------------------------------------------------------------------------
// (3) Gọi hàm thực hiện ghi rồi đọc ⇒ đúng giá trị.
// ---------------------------------------------------------------------------
#[test]
fn t3_goi_ham_ghi_roi_doc_tra_gia_tri_dung() {
    // f1 ghi 42 vào global 0 rồi trả `()`. f0 gọi f1 rồi đọc global 0.
    let f0 = f(
        "main",
        vec![Instr::Call(1, 0), Instr::LoadLocal(BASE), Instr::Return],
        2,
    );
    let f1 = f(
        "ghi",
        vec![
            Instr::Const(0), // 42
            Instr::StoreLocal(BASE),
            Instr::PushUnit,
            Instr::Return,
        ],
        1,
    );
    let modu = m(vec![f0, f1], vec![cint("42")]);
    assert_eq!(verify(&modu), Ok(()));
    let out = vis_vm::run(&modu, "", &[]);
    assert!(out.is_clean(), "diag: {:?}", out.diagnostics);
    assert_eq!(show(&out).as_deref(), Some("42"));
}

// ---------------------------------------------------------------------------
// (4) Ghi `Unit` hợp lệ ⇒ thành công, không nhầm với chưa khởi tạo.
// ---------------------------------------------------------------------------
#[test]
fn t4_ghi_unit_hop_le_khong_nham_chua_khoi_tao() {
    let modu = m(
        vec![f(
            "main",
            vec![
                Instr::PushUnit,
                Instr::StoreLocal(BASE),
                Instr::LoadLocal(BASE),
                Instr::Return,
            ],
            1,
        )],
        vec![],
    );
    assert_eq!(verify(&modu), Ok(()));
    let out = vis_vm::run(&modu, "", &[]);
    assert!(
        out.is_clean(),
        "Unit đã ghi không được trap: {:?}",
        out.diagnostics
    );
    assert_eq!(out.exit, 0);
    assert_eq!(show(&out).as_deref(), Some("()"));
}

// ---------------------------------------------------------------------------
// (5) Nhiều global, chỉ một phần khởi tạo ⇒ phân biệt đúng từng ô.
// ---------------------------------------------------------------------------

/// Module có 3 global (f1 mang đủ 3 lệnh ghi để verifier mở miền), nhưng f0 chỉ ghi ô 0 và 2.
fn module_ba_global(tail: Vec<Instr>) -> Module {
    let mut code = vec![
        Instr::Const(0),             // 10
        Instr::StoreLocal(BASE),     // global 0 = 10
        Instr::Const(1),             // 30
        Instr::StoreLocal(BASE + 2), // global 2 = 30 (ô 1 bị bỏ trống)
    ];
    code.extend(tail);
    code.push(Instr::Return);
    let f0 = f("main", code, 2);
    let f1 = f(
        "khong_goi",
        vec![
            Instr::PushUnit,
            Instr::StoreLocal(BASE),
            Instr::PushUnit,
            Instr::StoreLocal(BASE + 1),
            Instr::PushUnit,
            Instr::StoreLocal(BASE + 2),
            Instr::PushUnit,
            Instr::Return,
        ],
        1,
    );
    m(vec![f0, f1], vec![cint("10"), cint("30")])
}

#[test]
fn t5_nhieu_global_chi_mot_phan_khoi_tao() {
    let doc0 = module_ba_global(vec![Instr::LoadLocal(BASE)]);
    assert_eq!(verify(&doc0), Ok(()));
    let o0 = vis_vm::run(&doc0, "", &[]);
    assert!(
        o0.is_clean(),
        "ô 0 đã ghi phải đọc được: {:?}",
        o0.diagnostics
    );
    assert_eq!(show(&o0).as_deref(), Some("10"));

    let doc2 = module_ba_global(vec![Instr::LoadLocal(BASE + 2)]);
    assert_eq!(verify(&doc2), Ok(()));
    let o2 = vis_vm::run(&doc2, "", &[]);
    assert!(
        o2.is_clean(),
        "ô 2 đã ghi phải đọc được: {:?}",
        o2.diagnostics
    );
    assert_eq!(show(&o2).as_deref(), Some("30"));

    let doc1 = module_ba_global(vec![Instr::LoadLocal(BASE + 1)]);
    assert_eq!(verify(&doc1), Ok(()));
    let o1 = vis_vm::run(&doc1, "", &[]);
    assert_chua_khoi_tao(&o1, 1);
}

// ---------------------------------------------------------------------------
// (6) Đọc ngoài miền ⇒ bị chặn, không trả mặc định.
// ---------------------------------------------------------------------------
#[test]
fn t6_doc_ngoai_mien_bi_chan_khong_tra_mac_dinh() {
    // Chỉ có 1 lệnh ghi (ô 0) ⇒ nglobals = 1; đọc ô 1 là ngoài miền.
    let modu = m(
        vec![f(
            "main",
            vec![
                Instr::PushUnit,
                Instr::StoreLocal(BASE),
                Instr::LoadLocal(BASE + 1),
                Instr::Return,
            ],
            1,
        )],
        vec![],
    );
    assert!(matches!(
        verify(&modu),
        Err(VerifyError::GlobalOverflow {
            slot: 1,
            nglobals: 1,
            ..
        })
    ));
    let out = vis_vm::run(&modu, "", &[]);
    assert!(!out.is_clean(), "đọc ngoài miền phải bị chặn");
    assert_ne!(out.exit, 0);
    assert!(out.value.is_none(), "không được trả giá trị mặc định");
    assert!(
        out.diagnostics[0].message.contains("bytecode hỏng"),
        "phải là chặn bytecode: {:?}",
        out.diagnostics[0].message
    );

    // Cùng module đi qua path nạp từ byte cũng bị chặn ở verify.
    let bytes = encode(&modu);
    let decoded = decode(&bytes).expect("decode sạch");
    assert_eq!(decoded, modu);
    assert!(load_verified(&bytes).is_err());
}

// ---------------------------------------------------------------------------
// (7) Qua encode → decode → load_verified → execute, không dựng trực tiếp VM.
// ---------------------------------------------------------------------------
#[test]
fn t7_qua_encode_decode_load_verified_execute() {
    // Ca blocker ST-247: ghi trong hàm không gọi ⇒ verify chấp nhận nhưng runtime chưa khởi tạo.
    let f0 = f("main", vec![Instr::LoadLocal(BASE), Instr::Return], 1);
    let f1 = f(
        "khong_goi",
        vec![
            Instr::PushUnit,
            Instr::StoreLocal(BASE),
            Instr::PushUnit,
            Instr::Return,
        ],
        1,
    );
    let modu = m(vec![f0, f1], vec![]);

    let bytes = encode(&modu);
    let decoded = decode(&bytes).expect("decode sạch");
    assert_eq!(decoded, modu, "encode→decode phải giữ nguyên module");
    let loaded = load_verified(&bytes).expect("load_verified phải chấp nhận (đúng miền chỉ số)");
    assert_eq!(loaded, modu);

    let out = vis_vm::run(&loaded, "", &[]);
    assert_chua_khoi_tao(&out, 0);
}

// ---------------------------------------------------------------------------
// (8) Hai lần chạy độc lập không rò trạng thái khởi tạo.
// ---------------------------------------------------------------------------
#[test]
fn t8_hai_lan_chay_doc_lap_khong_ro_trang_thai() {
    // A: khởi tạo ô 0 = 7 rồi đọc ⇒ 7.
    let a = m(
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
        vec![cint("7")],
    );
    // B: cùng ô 0, nhưng chỉ có lệnh ghi trong hàm không gọi ⇒ phải trap dù A vừa chạy.
    let b = m(
        vec![
            f("main", vec![Instr::LoadLocal(BASE), Instr::Return], 1),
            f(
                "khong_goi",
                vec![
                    Instr::PushUnit,
                    Instr::StoreLocal(BASE),
                    Instr::PushUnit,
                    Instr::Return,
                ],
                1,
            ),
        ],
        vec![],
    );

    let oa1 = vis_vm::run(&a, "", &[]);
    assert!(oa1.is_clean() && show(&oa1).as_deref() == Some("7"));
    let ob1 = vis_vm::run(&b, "", &[]);
    assert_chua_khoi_tao(&ob1, 0);
    let ob2 = vis_vm::run(&b, "", &[]);
    assert_chua_khoi_tao(&ob2, 0);
    let oa2 = vis_vm::run(&a, "", &[]);
    assert!(oa2.is_clean() && show(&oa2).as_deref() == Some("7"));

    // Ngay cả khi tái dùng cùng một Vm, mỗi lần `run()` phải đặt lại kho global.
    let mut vm = vis_vm::Vm::new(&a, "");
    assert!(vm.run(&[]).is_clean());
    assert!(vm.run(&[]).is_clean());
}
