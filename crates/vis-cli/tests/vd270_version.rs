// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `vis version [--json]`.

use std::process::Command;

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
fn version_plain_mot_dong() {
    let (ma, out, err) = chay(&["version"]);
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(out, format!("vis {}\n", env!("CARGO_PKG_VERSION")));
    assert!(err.is_empty(), "stderr: {err}");
}

#[test]
fn version_json_mot_dong() {
    let (ma, out, err) = chay(&["version", "--json"]);
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(
        out,
        format!(
            "{{\"name\":\"vis\",\"version\":\"{}\"}}\n",
            env!("CARGO_PKG_VERSION")
        )
    );
    assert!(err.is_empty(), "stderr: {err}");
}

#[test]
fn version_sai_dung_exit_2_stdout_rong() {
    for args in [&["version", "x"][..], &["version", "--wat"][..]] {
        let (ma, out, err) = chay(args);
        assert_eq!(ma, 2, "args {args:?}; stderr: {err}");
        assert_eq!(out, "", "args {args:?}");
        assert!(err.contains("vis version"), "args {args:?}: {err}");
    }
}

/// Top-level `--version`/`-V` (hardening): in `vis <ver>`, exit 0, stderr rỗng.
#[test]
fn top_level_version_va_v() {
    for co in ["--version", "-V"] {
        let (ma, out, err) = chay(&[co]);
        assert_eq!(ma, 0, "{co}: stderr: {err}");
        assert_eq!(out, format!("vis {}\n", env!("CARGO_PKG_VERSION")), "{co}");
        assert!(err.is_empty(), "{co}: stderr: {err}");
    }
}

/// Top-level version không được nuốt cờ của lệnh con: `vis run --version` vẫn lỗi dùng như cũ.
#[test]
fn top_level_version_khong_anh_huong_lenh_con() {
    let (ma, out, err) = chay(&["run", "--version"]);
    assert_eq!(ma, 2, "stderr: {err}");
    assert_eq!(out, "", "không được in version cấp cao nhất");
    assert!(err.contains("vis run"), "stderr: {err}");
}
