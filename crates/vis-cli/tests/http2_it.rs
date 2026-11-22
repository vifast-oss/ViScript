// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296a** — binary gộp miền: HTTP/2 + ALPN (feature `http2`).
//!
//! Các tệp ca gốc nằm trong thư mục cùng tên; `#[path]` giữ nguyên nội dung, không copy test.
//! Tổng số ca/assertion không đổi — chỉ đổi crate test binary.

mod common;

#[path = "http2_it/vs3_http2.rs"]
mod vs3_http2;
