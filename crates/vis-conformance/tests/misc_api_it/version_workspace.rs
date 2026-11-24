// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-289 / VD-272(3)** — mọi crate thành viên dùng `version.workspace = true`.
//!
//! Tách từ `vis-cli/tests/cli_it/st289_version.rs` (ST-313c/VD-289); test CLI
//! `vis --version` giữ ở `vis-cli`.

use std::path::{Path, PathBuf};

const BAN: &str = "0.6.0";

fn goc() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn moi_crate_thanh_vien_dung_version_workspace() {
    let goc = goc();
    let root = std::fs::read_to_string(goc.join("Cargo.toml")).expect("đọc Cargo.toml gốc");
    assert!(
        root.contains(&format!("version = \"{BAN}\"")),
        "root Cargo.toml phải có version = \"{BAN}\""
    );
    let start = root.find("members = [").expect("khối members");
    let end = start + root[start..].find(']').expect("đóng members");
    let thanh_vien: Vec<&str> = root[start..end]
        .lines()
        .filter_map(|l| {
            let t = l.trim().trim_end_matches(',');
            t.strip_prefix('"')?.strip_suffix('"')
        })
        .collect();
    assert!(thanh_vien.len() >= 8, "members: {thanh_vien:?}");
    for m in thanh_vien {
        let p = goc.join(m).join("Cargo.toml");
        let t = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("đọc {}: {e}", p.display()));
        assert!(
            t.lines().any(|l| l.trim() == "version.workspace = true"),
            "{}: thiếu `version.workspace = true`",
            p.display()
        );
        assert!(
            !t.lines().any(|l| l.trim().starts_with("version = ")),
            "{}: còn version chép tay",
            p.display()
        );
    }
}
