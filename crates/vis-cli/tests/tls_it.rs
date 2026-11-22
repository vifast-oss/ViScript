// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296a** — binary gộp miền: TLS client/registry HTTPS (feature `tls`).
//!
//! Các tệp ca gốc nằm trong thư mục cùng tên; `#[path]` giữ nguyên nội dung, không copy test.
//! Tổng số ca/assertion không đổi — chỉ đổi crate test binary.

mod common;

#[path = "tls_it/vs3_registry_https.rs"]
mod vs3_registry_https;
#[path = "tls_it/vs3_tls.rs"]
mod vs3_tls;
