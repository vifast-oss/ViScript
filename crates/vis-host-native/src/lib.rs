// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-300b (VD-283 §2)** — host native của ViScript: quyền `cap[...]`, FS thật/mock,
//! net/TLS/HTTP + async reactor, SQLite. Tách khỏi `vis-cli` để CLI chỉ còn điều phối;
//! API/feature giữ nguyên qua re-export ở `vis-cli` (`0 đổi hành vi`).

#![forbid(unsafe_code)]

pub mod db;
pub mod fsmock;
pub mod net;
pub mod net_quyen;
pub mod ngoai;
pub mod quyen;
pub mod quyen_chuan;

#[cfg(any(unix, windows))]
pub mod async_net;

/// **ST-295 (VD-276)** — `openat`/`openat2` an toàn cho đường đọc/ghi Unix.
#[cfg(unix)]
pub mod quyen_mo;

/// **ST-174 (VD-176 §2)** — ca tất định cho điểm chèn `quyen::chen` (chỉ test).
#[cfg(test)]
pub mod quyen_st174;

/// **ST-295 (VD-276)** — ca tất định đường ghi `openat` (chỉ test).
#[cfg(all(test, unix))]
pub mod quyen_st295;
