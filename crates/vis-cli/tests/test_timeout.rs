// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-105 (VD-114)** — `vis test --timeout <ms>`: chỉ áp cho **`test fn`** (không áp fixture
//! `.out/.err`); quá hạn ⇒ kill tiến trình con, `not ok` + “quá hạn <ms> ms”, **tiếp tục** ca sau.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::Instant;

fn kho() -> std::sync::MutexGuard<'static, ()> {
    static KHO: Mutex<()> = Mutex::new(());
    KHO.lock().unwrap_or_else(|e| e.into_inner())
}

fn tam(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vis-timeout-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tmp");
    d
}

fn ghi(d: &Path, ten: &str, src: &str) {
    std::fs::write(d.join(ten), src).expect("ghi");
}

fn vis(args: &[&str]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(args)
        .output()
        .expect("vis");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

const TREO: &str = "test fn treo():\n    var i = 0\n    while true:\n        i = i + 1\n";
const NHANH: &str = "test fn nhanh():\n    assert 1 == 1\n";

/// (a) ca nhanh + `--timeout 5000` ⇒ ok, kết quả không đổi.
#[test]
fn nhanh_voi_han_lon_van_ok() {
    let _k = kho();
    let d = tam("nhanh");
    ghi(&d, "a.vis", NHANH);
    let (c, o, e) = vis(&["test", "--timeout", "5000", d.to_str().unwrap()]);
    assert_eq!(c, 0, "{e}");
    assert_eq!(o, "TAP version 13\n1..1\nok 1 - nhanh\n", "{o:?}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (b) ca treo + `--timeout 300` ⇒ `not ok` “quá hạn 300 ms”, tổng < 3 s, exit 1; **không rò con**.
#[test]
fn treo_bi_giet_trong_3s() {
    let _k = kho();
    let d = tam("treo");
    ghi(&d, "a.vis", TREO);
    let t0 = Instant::now();
    let (c, o, e) = vis(&["test", "--timeout", "300", d.to_str().unwrap()]);
    let dt = t0.elapsed();
    assert_eq!(c, 1, "stderr: {e}");
    assert!(o.contains("not ok 1 - treo"), "{o}");
    assert!(o.contains("quá hạn 300 ms"), "{o}");
    assert!(dt.as_millis() < 3_000, "tổng {dt:?} ≥ 3 s (kể cả kill)");
    assert!(
        con_cua_minh().is_empty(),
        "rò tiến trình: {:?}",
        con_cua_minh()
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// (c) 1 treo + 1 nhanh ⇒ ca nhanh **vẫn chạy** (`ok 2`), exit 1, tiếp tục sau treo.
#[test]
fn tiep_tuc_sau_ca_treo() {
    let _k = kho();
    let d = tam("tiep");
    ghi(&d, "a_treo.vis", TREO);
    ghi(&d, "b_nhanh.vis", NHANH);
    let (c, o, e) = vis(&["test", "--timeout", "300", d.to_str().unwrap()]);
    assert_eq!(c, 1, "{e}");
    assert!(o.contains("not ok 1 - treo"), "{o}");
    assert!(o.contains("ok 2 - nhanh"), "ca nhanh phải chạy tiếp: {o}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (d) `--timeout 300 --json` ⇒ `message` đúng **`quá hạn 300 ms`**, failed=1.
#[test]
fn json_message_qua_han() {
    let _k = kho();
    let d = tam("json");
    ghi(&d, "a.vis", TREO);
    let (c, o, e) = vis(&["test", "--timeout", "300", "--json", d.to_str().unwrap()]);
    assert_eq!(c, 1, "{e}");
    assert_eq!(
        o,
        "{\"total\":1,\"ok\":0,\"failed\":1,\"cases\":[{\"name\":\"treo\",\"ok\":false,\"message\":\"quá hạn 300 ms\"}]}\n"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// (e) `--timeout 0` / `abc` ⇒ exit 2, stderr nêu `--timeout`; thiếu giá trị cũng 2.
#[test]
fn timeout_sai_exit2() {
    let _k = kho();
    let d = tam("sai");
    ghi(&d, "a.vis", NHANH);
    for args in [
        vec!["test", "--timeout", "0", d.to_str().unwrap()],
        vec!["test", "--timeout", "abc", d.to_str().unwrap()],
        vec!["test", "--timeout"],
    ] {
        let (c, _, e) = vis(&args);
        assert_eq!(c, 2, "{args:?}: {e}");
        assert!(e.contains("--timeout"), "{args:?}: {e}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// Đếm tiến trình con **còn sống** của chính test runner (macOS/Linux `ps`).
fn con_cua_minh() -> Vec<String> {
    let pid = std::process::id().to_string();
    let o = Command::new("ps")
        .args(["-axo", "pid=,ppid=,command="])
        .output()
        .expect("ps");
    String::from_utf8_lossy(&o.stdout)
        .lines()
        .filter(|l| {
            let mut it = l.split_whitespace();
            let _pid = it.next();
            let ppid = it.next().unwrap_or("");
            ppid == pid && l.contains("vis") && !l.contains("ps -axo")
        })
        .map(|l| l.to_string())
        .collect()
}
