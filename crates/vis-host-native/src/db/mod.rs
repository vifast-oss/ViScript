// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-242 Q6 / ST-156** — host `std.sqlite` của CLI, tách theo feature `sqlite`.
//!
//! Hai cấu hình được đặt tên rõ (xem `agent/TARGET-MATRIX-2026-11-20.md`):
//!
//! - **`sqlite` (mặc định, cấu hình cũ)** — [`sqlite`] chứa toàn bộ triển khai `rusqlite`
//!   bundled nguyên vẹn (VD-241); hành vi không đổi.
//! - **`core` (`--no-default-features`)** — [`disabled`] vẫn cung cấp cùng chữ ký/host trait
//!   (`Db`, `DbGuard`) để `main.rs` không rẽ nhánh, nhưng **mọi** lời gọi `std.sqlite.*` bị từ
//!   chối rõ bằng một `Err(String)` có chẩn đoán (trở thành `Err(Error)` dữ liệu ở tầng stdlib,
//!   `hop_nhat_fs`), **không** panic, **không** hành vi giả.
//!
//! Chữ ký của `Db`/`DbGuard` và trait `vis_interp::builtin::DbHost` giữ nguyên ở cả hai nhánh.

#[cfg(feature = "sqlite")]
mod sqlite;
#[cfg(feature = "sqlite")]
pub use sqlite::*;

#[cfg(not(feature = "sqlite"))]
mod disabled;
#[cfg(not(feature = "sqlite"))]
pub use disabled::*;
