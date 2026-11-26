// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — `StdFn` (một hàm `std.*`: tên + chữ ký).
//!
//! **ST-317c**: `MODULES`/`MODULES_HOAN` chuyển sang `registry.rs` (cùng nguồn với bảng hàm).

use super::ty::Ty;

/// Một hàm `std.*`: tên đầy đủ + tham số + kiểu trả.
#[derive(Debug, Clone, Copy)]
pub struct StdFn {
    /// Tên đầy đủ `"str.split"`.
    pub name: &'static str,
    pub params: &'static [Ty],
    pub ret: Ty,
}

impl StdFn {
    pub const fn so_doi(&self) -> usize {
        self.params.len()
    }
}
