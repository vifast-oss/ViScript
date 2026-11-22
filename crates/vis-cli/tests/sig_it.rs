// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296a** — binary gộp miền: Ed25519 registry signature (feature `pkg-sig`).
//!
//! Các tệp ca gốc nằm trong thư mục cùng tên; `#[path]` giữ nguyên nội dung, không copy test.
//! Tổng số ca/assertion không đổi — chỉ đổi crate test binary.

mod common;

#[path = "sig_it/vs3_registry_sig.rs"]
mod vs3_registry_sig;
