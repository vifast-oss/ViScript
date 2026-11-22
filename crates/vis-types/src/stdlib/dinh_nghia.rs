// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — `StdFn`, danh sách module hỗ trợ/hoãn.

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

/// Module `std` được hỗ trợ ở ST-26 (mỗi phần tử là tên ngắn).
pub const MODULES: [&str; 10] = [
    "str", "list", "map", "path", "time", "json", "fs", "net", "http",
    // **VD-241 (Gate 1)** — `std.sqlite` MVP (prototype nội bộ, chưa freeze API).
    "sqlite",
];

/// `std.io` **chưa** làm — giữ tên để thông điệp rõ (ST-33: `std.fs` **đã có**).
pub const MODULES_HOAN: [&str; 1] = ["io"];
