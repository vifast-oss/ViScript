// SPDX-License-Identifier: MIT OR Apache-2.0
//! Thư viện nội bộ của binary `vis` — **chỉ** tách phần cần test trực tiếp bằng thư viện.
//!
//! **VS3-4a (VD-271)**: `net` được tách thành lib để `tests/vs3_tls.rs` gọi thẳng
//! [`net::Net`] + [`vis_interp::net::http_get`] với **server TLS thật** và **kho gốc test**.
//! Binary `vis` vẫn dùng `vis_cli::net::Net`; không có API/CLI mới cho người dùng.

#![forbid(unsafe_code)]

pub mod net;
