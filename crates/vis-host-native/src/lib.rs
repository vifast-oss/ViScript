// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-300b (VD-283 §2)** — host native của ViScript: quyền `cap[...]`, FS thật/mock,
//! net/TLS/HTTP + async reactor, SQLite. Tách khỏi `vis-cli` để CLI chỉ còn điều phối;
//! API/feature giữ nguyên qua re-export ở `vis-cli` (`0 đổi hành vi`).

#![forbid(unsafe_code)]

pub mod db;
pub mod fsmock;
pub mod net;
// **ST-310f**: gom tệp theo miền; giữ nguyên tên module công khai (`net_quyen`, `quyen*`)
// để không đổi API/bề mặt hành vi.
#[path = "net/quyen.rs"]
pub mod net_quyen;
pub mod ngoai;
pub mod quyen;
#[path = "quyen/chuan.rs"]
pub mod quyen_chuan;

#[cfg(any(unix, windows))]
pub mod async_net;

/// **ST-295 (VD-276)** — `openat`/`openat2` an toàn cho đường đọc/ghi Unix.
#[cfg(unix)]
#[path = "quyen/mo.rs"]
pub mod quyen_mo;

/// **ST-174 (VD-176 §2)** — ca tất định cho điểm chèn `quyen::chen` (chỉ test).
#[cfg(test)]
#[path = "quyen/st174.rs"]
pub mod quyen_st174;

/// **ST-295 (VD-276)** — ca tất định đường ghi `openat` (chỉ test).
#[cfg(all(test, unix))]
#[path = "quyen/st295.rs"]
pub mod quyen_st295;
