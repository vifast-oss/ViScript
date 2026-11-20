// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-257 / stream B** — bằng chứng TƯƠNG THÍCH bytecode cho bốn trục phiên bản.
//!
//! Trục (1) module format: `VERSION = 2` (cơ sở) / `INSTRUMENTED_VERSION = 3` (có trailer VFEE).
//! Trục (2) event-table: `fee::VERSION = 4`. Trục (3) weight-table: `WEIGHT_TABLE_VERSION = 1`.
//! Trục (4) cache key: `vis-cli::cache` (kiểm trong test bin + report).
//!
//! Khẳng định trung tâm: **đổi khoá cache là chưa đủ**. Hàng rào nằm ở artifact (module version),
//! nên cả đường *direct load* không qua cache cũng bị bảo vệ — runtime cũ (so `VERSION = 2`) từ
//! chối artifact instrumented (3), không đọc lõi rồi lờ trailer.

use vis_bytecode::fee::{
    decode_fee, encode_with_fee, load_verified_with_fee, verify_fee, FeeDecodeError, FeeEntry,
    FeeKind, FeeTable, FeeVerifyError, EVENT_TABLE_VERSION, FEE_MAGIC, WEIGHT_TABLE_VERSION,
};
use vis_bytecode::{
    decode, encode, verify, Const, DecodeError, Func, Instr, LoadError, Module, Span, HASH_LEN,
    INSTRUMENTED_VERSION, MAGIC, VERSION,
};

fn f(code: Vec<Instr>, spans: Vec<Span>) -> Func {
    Func {
        name: "f".into(),
        arity: 0,
        nparams: 0,
        nlocals: 2,
        max_stack: 2,
        code,
        spans,
    }
}

fn hop_le() -> Module {
    Module {
        version: VERSION,
        source_hash: [0u8; HASH_LEN],
        consts: vec![
            Const {
                kind: 0,
                text: "1".into(),
            },
            Const {
                kind: 5,
                text: "print".into(),
            },
        ],
        funcs: vec![f(
            vec![Instr::Const(0), Instr::CallBuiltin(1, 1), Instr::Return],
            vec![
                Span { start: 10, end: 20 },
                Span { start: 20, end: 30 },
                Span { start: 30, end: 40 },
            ],
        )],
        entry: 0,
    }
}

fn feehop() -> FeeTable {
    let mut t = FeeTable::new(1);
    t.push(0, FeeEntry::new(0, FeeKind::Expr, 12));
    t.push(0, FeeEntry::new(1, FeeKind::Builtin, 22));
    t
}

/// Mô phỏng **đúng cơ chế** của runtime cũ (trước VD-257): `decode`/`verify` so với `VERSION = 2`
/// và **không biết gì về trailer `VFEE`**. Sao chép nguyên phép so sánh, không gọi hàm mới.
fn decode_runtime_cu(bytes: &[u8]) -> Result<u16, u16> {
    assert_eq!(&bytes[..4], MAGIC);
    let v = u16::from_le_bytes([bytes[4], bytes[5]]);
    if v != 2 {
        Err(v)
    } else {
        Ok(v)
    }
}

fn verify_runtime_cu(m: &Module) -> Result<(), u16> {
    if m.version != 2 {
        Err(m.version)
    } else {
        Ok(())
    }
}

#[test]
fn bon_truc_phien_ban_doc_lap() {
    // (1) Module: 2 (cơ sở), 3 (instrumented). (2) Event-table: 4. (3) Weight: 1.
    assert_eq!(VERSION, 2);
    assert_eq!(INSTRUMENTED_VERSION, 3);
    assert_eq!(EVENT_TABLE_VERSION, 4);
    assert_eq!(WEIGHT_TABLE_VERSION, 1);
    // "3 là sentinel lạ" trước đây KHÔNG đúng — git log: VERSION chỉ có 1 và 2 từng phát hành.
    assert_ne!(VERSION, INSTRUMENTED_VERSION);
    assert_ne!(EVENT_TABLE_VERSION, WEIGHT_TABLE_VERSION);
    assert_eq!(
        INSTRUMENTED_VERSION, 3,
        "3 chưa từng là bản phát hành ⇒ dùng làm module v3"
    );
}

#[test]
fn runtime_cu_tu_choi_artifact_moi_khong_lo_trailer() {
    let new = encode_with_fee(&hop_le(), &feehop());
    assert_eq!(&new[..4], MAGIC);
    assert_eq!(u16::from_le_bytes([new[4], new[5]]), INSTRUMENTED_VERSION);
    // Runtime cũ: từ chối NGAY ở decode (trước khi kịp lờ trailer) — không chạy sai.
    assert_eq!(decode_runtime_cu(&new), Err(3));
    let mut m = hop_le();
    m.version = INSTRUMENTED_VERSION;
    assert_eq!(verify_runtime_cu(&m), Err(3), "verify runtime cũ cũng chặn");
    // Sanity: runtime cũ vẫn chấp nhận artifact cũ của chính nó ⇒ phép so sánh không phải nhánh chết.
    let old = encode(&hop_le());
    assert_eq!(decode_runtime_cu(&old), Ok(2));
    assert_eq!(verify_runtime_cu(&hop_le()), Ok(()));
}

#[test]
fn duong_direct_load_cung_bi_bao_ve_khong_chi_cache() {
    let new = encode_with_fee(&hop_le(), &feehop());
    // `decode`/`load_verified` là đường mặc định (dùng cho cả direct load lẫn cache) — phải từ chối
    // artifact instrumented: nếu không, chương trình đòi budget sẽ chạy **không charge**.
    assert_eq!(decode(&new), Err(DecodeError::BadVersion(3)));
    assert!(matches!(
        vis_bytecode::load_verified(&new),
        Err(LoadError::Decode(DecodeError::BadVersion(3)))
    ));
    // verify trực tiếp trên ModuleInstrumented cũng chặn (field version mang hàng rào).
    let mut m = hop_le();
    m.version = INSTRUMENTED_VERSION;
    assert_eq!(verify(&m), Err(vis_bytecode::VerifyError::BadVersion(3)));
    // Cửa có budget thì chấp nhận artifact mới — HERE là nơi duy nhất được charge.
    assert!(load_verified_with_fee(&new).is_ok());
}

#[test]
fn runtime_moi_tu_choi_artifact_cu_voi_chinh_sach_ro_rang() {
    let old = encode(&hop_le()); // VERSION = 2, KHÔNG có trailer
    assert_eq!(decode_fee(&old), Err(FeeDecodeError::Absent));
    // Cửa nạp có budget từ chối artifact cũ ở decode (thông điệp BadVersion(2)), KHÔNG coi là
    // "đã instrumented / đã charge" và KHÔNG rơi vào đường mặc định im lặng.
    match load_verified_with_fee(&old) {
        Err(LoadError::Decode(DecodeError::BadVersion(2))) => {}
        other => panic!("phải từ chối artifact cũ rõ ràng, got {other:?}"),
    }
    // Artifact v3 nhưng THIẾU trailer ⇒ hỏng, không phải "zero-work".
    let mut bad = hop_le();
    bad.version = INSTRUMENTED_VERSION;
    let no_trailer = encode(&bad);
    assert!(matches!(
        load_verified_with_fee(&no_trailer),
        Err(LoadError::Verify(vis_bytecode::VerifyError::FeeDecode(_)))
    ));
}

#[test]
fn sai_event_table_hoac_weight_table_deu_bi_tu_choi() {
    let good = encode_with_fee(&hop_le(), &feehop());
    let at = good.windows(4).rposition(|w| w == FEE_MAGIC).unwrap();

    // Trục 2 — sửa event-table version (4 → 9).
    let mut bad_ev = good.clone();
    bad_ev[at + 4..at + 6].copy_from_slice(&9u16.to_le_bytes());
    assert_eq!(decode_fee(&bad_ev), Err(FeeDecodeError::BadVersion(9)));
    assert!(load_verified_with_fee(&bad_ev).is_err());

    // Trục 3 — sửa weight-table version (1 → 9).
    let mut bad_w = good.clone();
    bad_w[at + 6..at + 8].copy_from_slice(&9u16.to_le_bytes());
    assert_eq!(decode_fee(&bad_w), Err(FeeDecodeError::BadWeightVersion(9)));
    assert!(load_verified_with_fee(&bad_w).is_err());

    // Đúng hai số thì qua.
    assert_eq!(decode_fee(&good).unwrap(), feehop());
    assert!(load_verified_with_fee(&good).is_ok());
}

/// Artifact **chế tạo** có chu trình CFG nhưng bảng phí không charge trong chu trình ⇒ verifier
/// từ chối. Đây là hàng rào chống "loop evading charges" — không dựa vào fuel/Safety-net.
#[test]
fn loop_evading_charges_bi_verifier_chan() {
    let mut m = hop_le();
    // pc 1: Jump(0) tự nhảy về chính nó ⇒ SCC {1} có self-loop; pc 0 nằm ngoài chu trình.
    m.funcs[0].code = vec![Instr::Const(0), Instr::Jump(0)];
    m.funcs[0].spans = vec![Span { start: 0, end: 10 }, Span { start: 0, end: 10 }];
    // Bảng chỉ charge pc 0 (ngoài chu trình); pc 1 không có điểm phí.
    let mut fee = FeeTable::new(1);
    fee.push(0, FeeEntry::new(0, FeeKind::Stmt, 1));
    // Bộ kiểm chặn ở một trong hai hàng rào: cạnh lùi vào PC không có phí guard, hoặc SCC có
    // chu trình nhưng không điểm phí. Điều quan trọng: KHÔNG được Ok(()) (không lọt vào VM).
    let r = verify_fee(&m, &fee);
    assert!(
        matches!(
            r,
            Err(FeeVerifyError::CycleUncharged { .. })
                | Err(FeeVerifyError::LoopGuardUncharged { .. })
        ),
        "chu trình không charge phải bị từ chối, got {r:?}"
    );
    // Cửa nạp có budget cũng từ chối artifact chế tạo này (không vào VM).
    assert!(load_verified_with_fee(&encode_with_fee(&m, &fee)).is_err());

    // Cạnh lùi vào đầu vòng mà header không có phí guard ⇒ LoopGuardUncharged.
    let mut m2 = hop_le();
    m2.funcs[0].code = vec![Instr::Const(0), Instr::Pop, Instr::Jump(-2)];
    m2.funcs[0].spans = vec![Span { start: 0, end: 10 }; 3];
    let mut fee2 = FeeTable::new(1);
    fee2.push(0, FeeEntry::new(1, FeeKind::Expr, 2)); // pc 0 (đích nhảy lùi) trống
    assert!(matches!(
        verify_fee(&m2, &fee2),
        Err(FeeVerifyError::LoopGuardUncharged { .. })
    ));
}

#[test]
fn fee_table_sai_cau_truc_bi_chan_khong_vao_vm() {
    let m = hop_le();
    // Sai số hàm.
    assert_eq!(
        verify_fee(&m, &FeeTable::new(2)),
        Err(FeeVerifyError::FuncCount { got: 2, want: 1 })
    );
    // pc ngoài code.
    let mut bad_pc = FeeTable::new(1);
    bad_pc.push(0, FeeEntry::new(99, FeeKind::Expr, 1));
    assert!(matches!(
        verify_fee(&m, &bad_pc),
        Err(FeeVerifyError::BadPc { .. })
    ));
    // site nằm ngoài span của lệnh (artifact tự khai sai).
    let mut bad_site = FeeTable::new(1);
    bad_site.push(0, FeeEntry::new(0, FeeKind::Expr, 9999));
    assert!(matches!(
        verify_fee(&m, &bad_site),
        Err(FeeVerifyError::BadSite { .. })
    ));
    // Bảng rỗng nhưng có code ⇒ không được coi là "zero work".
    assert_eq!(
        verify_fee(&m, &FeeTable::new(1)),
        Err(FeeVerifyError::EmptyTable)
    );
    for t in [FeeTable::new(2), bad_pc, bad_site, FeeTable::new(1)] {
        assert!(load_verified_with_fee(&encode_with_fee(&m, &t)).is_err());
    }
}

/// Byte trailer bị sửa **tag sự kiện** (kind lạ) ⇒ decode_fee từ chối. Chứng minh artifact chế tạo
/// không lọt qua cửa nạp có budget dù module lõi hợp lệ.
#[test]
fn trailer_kind_la_bi_tu_choi() {
    use vis_bytecode::fee::FEE_MAGIC;
    let b = encode_with_fee(&hop_le(), &feehop());
    let at = b.windows(4).rposition(|w| w == FEE_MAGIC).unwrap();
    // Bố cục: magic(4), ev(2), wv(2), nf(4), n0(4), rồi entry đầu: pc(4), kind(1), site(4).
    let kind_at = at + 4 + 2 + 2 + 4 + 4 + 4;
    let mut bad = b.clone();
    bad[kind_at] = 200;
    assert_eq!(decode_fee(&bad), Err(FeeDecodeError::BadKind(200)));
    assert!(load_verified_with_fee(&bad).is_err());
}

#[test]
fn cache_key_chua_bon_truc_qua_nguon() {
    // Trục 4 nằm ở `vis-cli::cache` — kiểm bằng source (không mở API công để test).
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let s = std::fs::read_to_string(root.join("vis-cli/src/cache.rs")).expect("cache.rs");
    assert!(
        s.contains("khoa_multi_voi_version(files, mode, the, vis_bytecode::VERSION)"),
        "module format trong khoá"
    );
    assert!(
        s.contains("&vis_bytecode::fee::VERSION.to_string()"),
        "event-table trong khoá"
    );
    assert!(
        s.contains("WEIGHT_TABLE_VERSION.to_string()"),
        "weight-table trong khoá"
    );
    assert!(
        s.contains("INSTRUMENTED_VERSION.to_string()"),
        "instrumented version trong khoá"
    );
    assert!(
        s.contains("vd257-b-four-version-axes"),
        "nhãn logic VD-257 trong khoá"
    );
}
