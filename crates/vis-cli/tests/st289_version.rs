// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-289 / VD-272(3)** — đồng bộ phiên bản: workspace là **một nguồn** (`0.3.6`), CLI in đúng
//! bản phát hành, mọi crate thành viên dùng `version.workspace = true` (không chép số).
//!
//! Test này **cố ý ghim** số phiên bản: bump phát hành phải sửa đúng một chỗ ở
//! `Cargo.toml` (`[workspace.package] version`) và hằng `BAN` ở đây. Không đọc `Cargo.lock`
//! (không thêm dependency); chỉ đọc `Cargo.toml` dạng văn bản.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Bản phát hành hiện hành — phải khớp `[workspace.package] version`.
const BAN: &str = "0.3.6";

fn goc() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn chay(args: &[&str]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(args)
        .output()
        .expect("chạy vis");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

#[test]
fn cli_in_dung_ban_0_3_6() {
    assert_eq!(
        env!("CARGO_PKG_VERSION"),
        BAN,
        "vis-cli phải theo workspace"
    );
    for co in ["--version", "-V", "version"] {
        let (ma, out, err) = chay(&[co]);
        assert_eq!(ma, 0, "{co}: stderr: {err}");
        assert_eq!(out, format!("vis {BAN}\n"), "{co}");
        assert!(err.is_empty(), "{co}: stderr: {err}");
    }
    let (ma, out, err) = chay(&["version", "--json"]);
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(out, format!("{{\"name\":\"vis\",\"version\":\"{BAN}\"}}\n"));
}

/// Mọi crate thành viên khai `version.workspace = true`; root giữ đúng `version = "0.3.6"`.
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
