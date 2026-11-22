// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296a** — helper dùng chung cho các binary test đã gộp miền.
//!
//! Chỉ chứa tiện ích **không đổi hành vi**; test gốc vẫn tự khai helper riêng trong module của
//! mình (không copy sang đây để tránh sửa 272 tệp ca). `#![allow(dead_code)]` vì một số binary
//! gộp không dùng hết helper.

#![allow(dead_code)]

use std::path::PathBuf;

/// Gốc repo (`crates/vis-cli` → `../..`).
pub fn goc_repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Đường dẫn binary `vis` của package (Cargo cấp cho integration test).
pub fn vis_bin() -> &'static str {
    env!("CARGO_BIN_EXE_vis")
}

/// Đường dẫn binary `test-child` (helper POSIX của test).
pub fn test_child_bin() -> &'static str {
    env!("CARGO_BIN_EXE_test-child")
}
