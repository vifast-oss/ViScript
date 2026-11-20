// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `vis test --json-out <path>` (TAP stdout + JSON report file).
//!
//! Red-before @ `580123b`: `--json-out` là cờ lạ ⇒ exit 2 (ghi trong doc test).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam() -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vtj-{}-{n}", std::process::id()));
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

fn ghi_test(d: &Path, ten: &str, than: &str) -> PathBuf {
    let p = d.join(format!("{ten}.vis"));
    std::fs::write(&p, format!("test fn {ten}():\n    {than}\n")).expect("ghi");
    p
}

#[test]
fn json_out_ghi_tep_stdout_van_tap() {
    let d = tam();
    ghi_test(&d, "mot", "assert true");
    let r = d.join("report.json");
    let ds = d.to_string_lossy().to_string();
    let rs = r.to_string_lossy().to_string();
    let (ma, out, err) = chay(&["test", "--json-out", &rs, &ds]);
    assert_eq!(ma, 0, "stderr: {err}");
    assert!(out.starts_with("TAP version 13"), "stdout TAP: {out}");
    assert!(out.contains("ok 1 - mot"), "stdout: {out}");
    let body = std::fs::read_to_string(&r).expect("đọc report");
    assert!(body.starts_with("{\"total\":1"), "json: {body}");
    assert!(body.contains("\"name\":\"mot\""), "json: {body}");
    assert!(body.contains("\"failed\":0"), "json: {body}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn json_out_kem_json_stdout_va_tep_giong() {
    let d = tam();
    ghi_test(&d, "mot", "assert true");
    let r = d.join("r.json");
    let ds = d.to_string_lossy().to_string();
    let rs = r.to_string_lossy().to_string();
    let (ma, out, err) = chay(&["test", "--json", "--json-out", &rs, &ds]);
    let body = std::fs::read_to_string(&r).expect("đọc");
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(out, body, "stdout JSON phải bằng tệp");
}

#[test]
fn json_out_ca_hong_ghi_failed() {
    let d = tam();
    ghi_test(&d, "hong", "panic(\"no\")");
    let r = d.join("r.json");
    let ds = d.to_string_lossy().to_string();
    let rs = r.to_string_lossy().to_string();
    let (ma, out, err) = chay(&["test", "--json-out", &rs, &ds]);
    let body = std::fs::read_to_string(&r).expect("đọc");
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(ma, 1, "stderr: {err}");
    assert!(out.contains("not ok 1 - hong"), "stdout: {out}");
    assert!(body.contains("\"failed\":1"), "json: {body}");
}

#[test]
fn json_out_loc_va_repeat() {
    let d = tam();
    ghi_test(&d, "mot", "assert true");
    ghi_test(&d, "hai", "assert true");
    let r = d.join("r.json");
    let ds = d.to_string_lossy().to_string();
    let rs = r.to_string_lossy().to_string();
    let (ma, _out, err) = chay(&["test", "--json-out", &rs, "--filter", "mo", &ds]);
    let body = std::fs::read_to_string(&r).expect("đọc");
    assert_eq!(ma, 0, "stderr: {err}");
    assert!(body.contains("\"total\":1"), "json: {body}");
    let (ma, _out, err) = chay(&["test", "--json-out", &rs, "--repeat", "2", &ds]);
    let body = std::fs::read_to_string(&r).expect("đọc");
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(ma, 0, "stderr: {err}");
    assert!(body.contains("\"total\":4"), "json: {body}");
    assert!(body.contains("[1/2]"), "json: {body}");
}

#[test]
fn json_out_dung_sai_exit_2() {
    let d = tam();
    ghi_test(&d, "mot", "assert true");
    let ds = d.to_string_lossy().to_string();
    let r = d.join("r.json");
    let rs = r.to_string_lossy().to_string();
    for args in [
        &["test", "--json-out"][..],
        &["test", "--json-out=", &ds][..],
        &["test", "--json-out", &rs, "--json-out", &rs, &ds][..],
        &["test", "--json-out", &rs, "--list", &ds][..],
        &["test", "--json-out", &ds, &ds][..], // đích là thư mục
    ] {
        let (ma, out, err) = chay(args);
        assert_eq!(ma, 2, "args {args:?} phải exit 2; stderr: {err}");
        assert_eq!(out, "", "args {args:?}");
        assert!(err.contains("vis test"), "args {args:?}: {err}");
    }
    let _ = std::fs::remove_dir_all(&d);
}
