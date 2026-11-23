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

/// **ST-307** — dựng (một lần/binary test) plugin WASI mẫu `test/plugin-wasi/mau.rs`.
///
/// `None` ⇒ môi trường thiếu `rustc`/target `wasm32-wasip1`; ca gọi in `BLOCKED-ENV` (cùng quy
/// ước ST-299a/b), không giả vờ đã chạy. Không dùng cargo/không tải gì thêm.
pub fn plugin_mau_wasm() -> Option<PathBuf> {
    use std::process::Command;
    use std::sync::OnceLock;
    static MOT_LAN: OnceLock<Option<PathBuf>> = OnceLock::new();
    MOT_LAN
        .get_or_init(|| {
            let goc = goc_repo();
            let src = goc.join("test/plugin-wasi/mau.rs");
            let out = goc.join("target/plugin-wasi/mau.wasm");
            let moi_hon = || -> bool {
                let (Ok(a), Ok(b)) = (std::fs::metadata(&out), std::fs::metadata(&src)) else {
                    return false;
                };
                match (a.modified(), b.modified()) {
                    (Ok(a), Ok(b)) => a >= b,
                    _ => false,
                }
            };
            if !moi_hon() {
                std::fs::create_dir_all(out.parent()?).ok()?;
                let mut ung_vien = Vec::new();
                if let Some(p) = std::env::var_os("VIS_RUSTC") {
                    ung_vien.push(PathBuf::from(p));
                }
                if let Some(home) = std::env::var_os("HOME") {
                    ung_vien.push(PathBuf::from(home).join(".cargo/bin/rustc"));
                }
                ung_vien.push(PathBuf::from("rustc"));
                let mut thanh_cong = false;
                for rustc in ung_vien {
                    let st = Command::new(rustc)
                        .args(["--target", "wasm32-wasip1", "-O", "--edition", "2021"])
                        .arg(&src)
                        .arg("-o")
                        .arg(&out)
                        .current_dir(&goc)
                        .status();
                    if let Ok(st) = st {
                        if st.success() && out.is_file() {
                            thanh_cong = true;
                            break;
                        }
                    }
                }
                if !thanh_cong {
                    return None;
                }
            }
            Some(out)
        })
        .clone()
}

/// Đường dẫn binary `vis` của package (Cargo cấp cho integration test).
pub fn vis_bin() -> &'static str {
    env!("CARGO_BIN_EXE_vis")
}

/// Đường dẫn binary `test-child` (helper POSIX của test).
pub fn test_child_bin() -> &'static str {
    env!("CARGO_BIN_EXE_test-child")
}
