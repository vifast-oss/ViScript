// SPDX-License-Identifier: MIT OR Apache-2.0
//! Thư viện nội bộ của binary `vis` — **chỉ** tách phần cần test trực tiếp bằng thư viện.
//!
//! **VS3-4a (VD-271)**: `net` được tách thành lib để `tests/vs3_tls.rs` gọi thẳng
//! [`net::Net`] + [`vis_interp::net::http_get`] với **server TLS thật** và **kho gốc test**.
//! Binary `vis` vẫn dùng `vis_cli::net::Net`; không có API/CLI mới cho người dùng.

#![forbid(unsafe_code)]

#[cfg(any(unix, windows))]
pub mod async_net;
pub mod net;
/// **VS3-5b (VD-271)**: chữ ký Ed25519 cho registry HTTP — tách vào lib để test dùng **chính**
/// hàm sinh thông điệp chuẩn (`vispkg-sig1`), không lặp định dạng trong test.
pub mod pkg_sig;
