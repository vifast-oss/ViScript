// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-300b (VD-283 §2)** — gói ViScript: resolver/semver, registry HTTP(S), lock/store, GC và
//! chữ ký Ed25519. Tách khỏi `vis-cli`; chỉ phụ thuộc `vis-loader`/`vis-interp`/`vis-bytecode`
//! (+ `vis-host-native` cho net/TLS). Re-export giữ API `vis_cli::pkg`/`pkg_sig` cũ.

#![forbid(unsafe_code)]

pub mod pkg;
pub mod pkg_sig;

/// Giữ đường `crate::json_esc::…` của mã pkg đã chuyển nguyên văn.
pub(crate) use vis_loader::json_esc;
