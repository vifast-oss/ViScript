// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296a** — binary gộp miền: TLS client (feature `tls`).
//!
//! Các tệp ca gốc nằm trong thư mục cùng tên; `#[path]` giữ nguyên nội dung, không copy test.
//! Tổng số ca/assertion không đổi — chỉ đổi crate test binary.
//!
//! **ST-310e**: chuyển về crate chủ `vis-host-native/tests/`. Phần `vs3_registry_https` (HTTPS
//! registry + chữ ký) chuyển sang `vis-pkg/tests/sig_it/` vì phụ thuộc feature `pkg-sig`.

mod common;

#[path = "tls_it/vs3_tls.rs"]
mod vs3_tls;
