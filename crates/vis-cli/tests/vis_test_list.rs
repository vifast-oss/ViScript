// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-281** — `vis test --list` (liệt kê ca, không chạy).

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam() -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-test-list-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tmp");
    d
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
fn list_khong_chay_va_in_ten() {
    let d = tam();
    std::fs::write(d.join("a.vis"), "test fn mot():\n    assert true\n").expect("a");
    // Nếu bị chạy, case này panic ⇒ exit 101; --list phải chỉ in tên.
    std::fs::write(
        d.join("b.vis"),
        "test fn hong():\n    panic(\"khong-duoc-chay\")\n",
    )
    .expect("b");
    let ds = d.to_string_lossy().to_string();
    let (code, out, err) = chay(&["test", "--list", &ds]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(err, "", "stderr phải rỗng");
    assert!(out.lines().any(|l| l == "mot"), "thiếu mot: {out}");
    assert!(out.lines().any(|l| l == "hong"), "thiếu hong: {out}");
    assert!(
        !out.contains("khong-duoc-chay"),
        "không được chạy test: {out}"
    );
}

#[test]
fn list_loc_theo_filter() {
    let d = tam();
    std::fs::write(d.join("a.vis"), "test fn mot():\n    assert true\n").expect("a");
    std::fs::write(d.join("b.vis"), "test fn hai():\n    assert true\n").expect("b");
    let ds = d.to_string_lossy().to_string();
    let (code, out, err) = chay(&["test", "--list", "--filter", "mo", &ds]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(out.trim(), "mot", "chỉ còn mot: {out}");
}

#[test]
fn list_json_bi_tu_choi() {
    let d = tam();
    std::fs::write(d.join("a.vis"), "test fn mot():\n    assert true\n").expect("a");
    let ds = d.to_string_lossy().to_string();
    let (code, out, err) = chay(&["test", "--list", "--json", &ds]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(code, 2, "phải từ chối: out={out} err={err}");
    assert_eq!(out, "", "stdout rỗng");
    assert!(err.contains("--json"), "stderr nêu lý do: {err}");
}
