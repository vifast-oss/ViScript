// SPDX-License-Identifier: MIT OR Apache-2.0
//! Thư viện nội bộ của binary `vis` — **chỉ** tách phần cần test trực tiếp bằng thư viện.
//!
//! **VS3-4a (VD-271)**: `net` được tách thành lib để `tests/vs3_tls.rs` gọi thẳng
//! [`net::Net`] + [`vis_http::http_get`] với **server TLS thật** và **kho gốc test**.
//! Binary `vis` vẫn dùng `vis_cli::net::Net`; không có API/CLI mới cho người dùng.

#![forbid(unsafe_code)]

/// **ST-310b (VD-285)** — thân `vis repl` dùng chung cho binary và test (PTY giả/`--format viterm`).
pub mod repl;

#[cfg(any(unix, windows))]
pub use vis_host_native::async_net;
pub use vis_host_native::net;
/// **VS3-5b / ST-300b**: chữ ký Ed25519 cho registry HTTP nằm ở `vis-pkg`; re-export giữ
/// `vis_cli::pkg_sig` cho test hiện có.
pub use vis_pkg::pkg_sig;
