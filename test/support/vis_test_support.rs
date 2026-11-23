// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310e** — helper dùng chung cho các suite test e2e đã chuyển về crate chủ.
//!
//! Trước `ST-310e`, các suite nằm trong `crates/vis-cli/tests/` nên dùng trực tiếp
//! `env!("CARGO_BIN_EXE_vis")` / `env!("CARGO_BIN_EXE_test-child")`. Khi chuyển sang
//! `vis-lsp`/`vis-pkg`/`vis-loader`/`vis-host-native`/`vis-plugin-host`, Cargo **không** cấp
//! `CARGO_BIN_EXE_*` của package khác. Helper này tìm binary trong target của workspace:
//!
//! 1. biến môi trường `VIS_BIN` / `VIS_TEST_CHILD` (do người chạy đặt, kể cả CI);
//! 2. `CARGO_BIN_EXE_*` (khi file được include từ chính `vis-cli`);
//! 3. `<workspace>/target/{debug,release}/<tên>` (kể cả target triple và `CARGO_TARGET_DIR`).
//!
//! `cargo test --workspace` / `cargo nextest run --workspace` build `vis` + `test-child` trước
//! khi chạy test binary, nên đường 3 là đường chuẩn của cổng. Chạy riêng một package (không qua
//! workspace) cần build trước: `cargo build -p vis-cli --bins`.

#![allow(dead_code)]

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

/// Gốc repo (`crates/<crate>` → `../..`).
pub fn goc_repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn ung_vien_target() -> Vec<PathBuf> {
    let goc = goc_repo();
    let mut ds = Vec::new();
    if let Some(td) = std::env::var_os("CARGO_TARGET_DIR") {
        let p = PathBuf::from(td);
        ds.push(if p.is_absolute() { p } else { goc.join(p) });
    }
    ds.push(goc.join("target"));
    ds
}

fn tim_binary(ten: &str, bien: &str, cargobin: Option<&'static str>) -> String {
    if let Some(p) = std::env::var_os(bien) {
        return p.to_string_lossy().into_owned();
    }
    if let Some(p) = cargobin {
        return p.to_string();
    }
    let ten_exe = format!("{ten}{}", std::env::consts::EXE_SUFFIX);
    for target in ung_vien_target() {
        for profile in ["debug", "release"] {
            let thang = target.join(profile).join(&ten_exe);
            if thang.is_file() {
                return thang.to_string_lossy().into_owned();
            }
            // target triple: `<target>/<triple>/<profile>/<bin>`
            let Ok(ds) = target.read_dir() else {
                continue;
            };
            for muc in ds.flatten() {
                let thang = muc.path().join(profile).join(&ten_exe);
                if thang.is_file() {
                    return thang.to_string_lossy().into_owned();
                }
            }
        }
    }
    panic!(
        "ST-310e: không thấy binary `{ten}` trong target của workspace.\n\
         Đặt `{bien}=<đường dẫn>` hoặc build trước: `cargo build -p vis-cli --bins` \
         (cổng `./vis-dev gate --fast` tự build cả workspace)."
    )
}

/// Đường dẫn binary `vis` dùng cho test e2e.
pub fn vis_bin() -> &'static str {
    static P: OnceLock<String> = OnceLock::new();
    P.get_or_init(|| tim_binary("vis", "VIS_BIN", option_env!("CARGO_BIN_EXE_vis")))
        .as_str()
}

/// Đường dẫn binary `test-child` (helper POSIX đa nền).
pub fn test_child_bin() -> &'static str {
    static P: OnceLock<String> = OnceLock::new();
    P.get_or_init(|| {
        tim_binary(
            "test-child",
            "VIS_TEST_CHILD",
            option_env!("CARGO_BIN_EXE_test-child"),
        )
    })
    .as_str()
}

/// **ST-310e** — `const TC: TcBin = TcBin;` giữ nguyên mọi cách dùng `TC` cũ trong ca test
/// (`format!("{TC} …")`, `Path::new(&TC)`, `fs::copy(TC, …)`, `Command::new(TC)`).
#[derive(Clone, Copy, Debug)]
pub struct TcBin;

impl std::fmt::Display for TcBin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(test_child_bin())
    }
}

impl AsRef<std::ffi::OsStr> for TcBin {
    fn as_ref(&self) -> &std::ffi::OsStr {
        std::ffi::OsStr::new(test_child_bin())
    }
}

impl AsRef<std::path::Path> for TcBin {
    fn as_ref(&self) -> &std::path::Path {
        std::path::Path::new(test_child_bin())
    }
}

/// **ST-307** — dựng (một lần/binary test) plugin WASI mẫu `test/plugin-wasi/mau.rs`.
///
/// `None` ⇒ môi trường thiếu `rustc`/target `wasm32-wasip1`; ca gọi in `BLOCKED-ENV` (cùng quy
/// ước ST-299a/b), không giả vờ đã chạy. Không dùng cargo/không tải gì thêm.
pub fn plugin_mau_wasm() -> Option<PathBuf> {
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
