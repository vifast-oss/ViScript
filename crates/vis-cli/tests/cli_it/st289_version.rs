// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-289 / VD-272(3)** — đồng bộ phiên bản: workspace là **một nguồn** (`0.5.5`), CLI in đúng
//! bản phát hành, mọi crate thành viên dùng `version.workspace = true` (không chép số).
//!
//! Test này **cố ý ghim** số phiên bản: bump phát hành phải sửa đúng một chỗ ở
//! `Cargo.toml` (`[workspace.package] version`) và hằng `BAN` ở đây. Không đọc `Cargo.lock`
//! (không thêm dependency); chỉ đọc `Cargo.toml` dạng văn bản.

use std::process::Command;

/// Bản phát hành hiện hành — phải khớp `[workspace.package] version`.
const BAN: &str = "0.6.0";

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
fn cli_in_dung_ban_0_5_5() {
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
