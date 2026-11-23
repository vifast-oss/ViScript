// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-311c (VD-286)** — binary gộp cho bộ kiểm thử tích hợp của `vis-bytecode`.
//!
//! `#[path]` giữ nguyên từng tệp ca gốc trong `bytecode_it/`; `st309_fuzz_verify.rs` giữ riêng
//! để chạy độc lập `VBC_FUZZ_CA=100000000`.

#[path = "bytecode_it/codegen.rs"]
mod codegen;
#[path = "bytecode_it/decode_neg.rs"]
mod decode_neg;
#[path = "bytecode_it/fee_side_table.rs"]
mod fee_side_table;
#[path = "bytecode_it/hash.rs"]
mod hash;
#[path = "bytecode_it/lat257_b_compat.rs"]
mod lat257_b_compat;
#[path = "bytecode_it/roundtrip.rs"]
mod roundtrip;
#[path = "bytecode_it/st245_boundary.rs"]
mod st245_boundary;
#[path = "bytecode_it/st246_version_compat.rs"]
mod st246_version_compat;
#[path = "bytecode_it/st247_global_domain.rs"]
mod st247_global_domain;
#[path = "bytecode_it/tin_tyck.rs"]
mod tin_tyck;
#[path = "bytecode_it/verify.rs"]
mod verify;
