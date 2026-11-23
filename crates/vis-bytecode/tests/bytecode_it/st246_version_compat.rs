// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-246 (VD-244 §2)** — bằng chứng `VERSION` là hàng rào tương thích bytecode.
//!
//! Bốn bằng chứng bắt buộc của VD-244 §2, phần bytecode:
//! 1. Runtime **mới** (`VERSION = 2`) từ chối artifact **v1** — [`runtime_moi_tu_choi_artifact_v1`].
//! 2. Runtime **cũ** (`VERSION = 1`, mô phỏng đúng cơ chế `decode`/`verify` so `VERSION`) từ chối
//!    artifact **v2** — [`runtime_cu_tu_choi_artifact_v2`].
//! 3. Việc từ chối xảy ra **trước thực thi**: `decode`/`load_verified`/`verify` đều trả lỗi ở cửa vào
//!    (không có đường vào VM).
//! 4. Bytecode v1 **không có source** bị từ chối rõ — [`v1_khong_source_bi_tu_choi_ro`].
//!
//! Không đổi `MAGIC`/bố cục/opcode; không tái sinh API-VBC1 (xem
//! [`version_moi_2_magic_va_bang_the_giuyen`]).

use vis_bytecode::decode::{decode, DecodeError};
use vis_bytecode::format::SO_THE;
use vis_bytecode::verify::{load_verified, verify, LoadError, VerifyError};
use vis_bytecode::{encode, Const, Func, Instr, Module, Span, HASH_LEN, MAGIC, VERSION};

/// Module **hợp lệ** tối thiểu (trùng dạng `verify.rs::hop_le`): in hằng rồi `Return`.
fn hop_le() -> Module {
    Module {
        version: VERSION,
        source_hash: [7u8; HASH_LEN],
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
        funcs: vec![Func {
            name: "main".into(),
            arity: 0,
            nparams: 0,
            nlocals: 1,
            max_stack: 2,
            code: vec![Instr::Const(0), Instr::CallBuiltin(1, 1), Instr::Return],
            spans: vec![Span { start: 0, end: 0 }; 3],
        }],
        entry: 0,
    }
}

/// Đóng dấu một **số phiên bản cố định** vào artifact (bypass `encode`'s `m.version.max(VERSION)`).
fn dat_version(bytes: &mut [u8], v: u16) {
    assert_eq!(&bytes[..4], MAGIC, "magic phải giữ nguyên VBC1");
    bytes[4..6].copy_from_slice(&v.to_le_bytes());
}

/// **ST-246**: VERSION phải là 2; `MAGIC`/bố cục/bảng thẻ không đổi ⇒ API-VBC1 giữ nguyên.
#[test]
fn version_moi_2_magic_va_bang_the_giuyen() {
    assert_eq!(VERSION, 2, "ST-246/VD-244: VERSION phải tăng 1 → 2");
    assert_ne!(VERSION, 1, "không được phép giữ VERSION cũ");
    assert_eq!(MAGIC, *b"VBC1", "ST-246 không đổi MAGIC");
    assert_eq!(SO_THE, 55, "ST-246 không đổi bảng opcode");

    let m = hop_le();
    let b = encode(&m);
    assert_eq!(
        u16::from_le_bytes([b[4], b[5]]),
        2,
        "source biên dịch lại phải đóng dấu VERSION = 2"
    );
    // Artifact v2 roundtrip đầy đủ qua mọi cửa vào.
    assert_eq!(decode(&b).expect("decode v2"), m);
    assert_eq!(verify(&m), Ok(()));
    assert_eq!(load_verified(&b).expect("load v2"), m);
    assert!(m.is_well_formed());
}

/// **Bằng chứng 1+3**: runtime mới từ chối artifact v1 ở cửa vào, không bao giờ tới VM.
#[test]
fn runtime_moi_tu_choi_artifact_v1() {
    let mut b = encode(&hop_le());
    dat_version(&mut b, 1);

    // decode: từ chối ngay khi đọc trường version (trước khi đọc hash/const/func/code).
    assert_eq!(decode(&b), Err(DecodeError::BadVersion(1)));
    // load_verified (đường cache dùng) cũng từ chối, đúng thông điệp version.
    assert_eq!(
        load_verified(&b),
        Err(LoadError::Decode(DecodeError::BadVersion(1)))
    );
    // verify trên Module có version = 1 (đường gọi trực tiếp) cũng từ chối.
    let mut m = hop_le();
    m.version = 1;
    assert_eq!(verify(&m), Err(VerifyError::BadVersion(1)));
    assert!(!m.is_well_formed(), "is_well_formed cũng khóa theo VERSION");

    // Thông điệp rõ: "version 1 ≠ 2".
    let msg = DecodeError::BadVersion(1).to_string();
    assert!(
        msg.contains('1') && msg.contains('2'),
        "thông điệp phải rõ: {msg}"
    );

    // Chỉ khác đúng 2 byte version ⇒ chính VERSION là nguyên nhân bị từ chối.
    dat_version(&mut b, VERSION);
    assert_eq!(decode(&b).expect("v2 hợp lệ"), hop_le());
}

/// Hàng rào VERSION của **runtime CŨ** (trước ST-246), sao chép nguyên cơ chế:
/// `decode.rs`: `if version != VERSION { return Err(BadVersion(version)); }` với `VERSION = 1`;
/// `verify.rs`: `if m.version != VERSION { return Err(BadVersion(m.version)); }` với `VERSION = 1`.
/// (Xem `git show 6eda337:crates/vis-bytecode/src/{decode,verify}.rs` — HEAD trước khi sửa.)
fn old_decode_gate(bytes: &[u8]) -> Result<u16, u16> {
    assert_eq!(&bytes[..4], MAGIC);
    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    const OLD_VERSION: u16 = 1;
    if version != OLD_VERSION {
        Err(version)
    } else {
        Ok(version)
    }
}

fn old_verify_gate(m: &Module) -> Result<(), u16> {
    const OLD_VERSION: u16 = 1;
    if m.version != OLD_VERSION {
        Err(m.version)
    } else {
        Ok(())
    }
}

/// **Bằng chứng 2**: runtime cũ (VERSION = 1) từ chối artifact v2 — **không chạy sai**.
#[test]
fn runtime_cu_tu_choi_artifact_v2() {
    let v2 = encode(&hop_le()); // artifact mới, version = 2
    assert_eq!(u16::from_le_bytes([v2[4], v2[5]]), 2);

    // Cả decode CŨ lẫn verify CŨ đều từ chối artifact v2.
    assert_eq!(
        old_decode_gate(&v2),
        Err(2),
        "decode v1 phải từ chối artifact v2"
    );
    assert_eq!(
        old_verify_gate(&hop_le()),
        Err(2),
        "verify v1 phải từ chối module v2"
    );
    // Trong khi runtime MỚI chấp nhận chính artifact đó ⇒ khác biệt duy nhất là VERSION.
    assert_eq!(decode(&v2).expect("decode v2"), hop_le());

    // Sanity: tấm lưới cũ chấp nhận v1 và từ chối v2 ⇒ nó thật sự phân biệt theo VERSION.
    let mut v1 = v2.clone();
    dat_version(&mut v1, 1);
    assert_eq!(old_decode_gate(&v1), Ok(1));
    let mut m1 = hop_le();
    m1.version = 1;
    assert_eq!(old_verify_gate(&m1), Ok(()));
    assert_eq!(old_decode_gate(&v2), Err(2));

    // Kết luận: hai chiều đối xứng qua cùng một phép so sánh VERSION.
    assert_eq!(decode(&v1), Err(DecodeError::BadVersion(1)));
    assert_eq!(old_decode_gate(&v2), Err(2));
}

/// **Bằng chứng 4**: bytecode v1 không có source không có đường tương thích — bị từ chối rõ.
/// VD-244 không duyệt đường dịch v1→v2; mọi nạp bytecode đều đi qua `load_verified`.
#[test]
fn v1_khong_source_bi_tu_choi_ro() {
    let mut b = encode(&hop_le());
    dat_version(&mut b, 1);
    // Không có source ⇒ không có gì để biên dịch lại; kết quả là lỗi rõ, không phải "chạy sai"
    // và không phải `Ok(Module)` im lặng.
    match load_verified(&b) {
        Err(LoadError::Decode(DecodeError::BadVersion(1))) => {}
        other => panic!("phải từ chối rõ artifact v1, được {other:?}"),
    }
}
