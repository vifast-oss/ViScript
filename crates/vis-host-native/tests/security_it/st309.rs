// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-309 (kiểm toán bảo mật)** — ca âm **hữu hạn, tất định** ở biên CLI:
//!
//! - symlink ở **tên cuối** trỏ ra ngoài `cap[Dir]`: đọc **và** ghi đều `VS0604`, tệp ngoài
//!   không đổi (bản cũ ghi xuyên link — `PluginCtx` và `FsGuard` đã vá);
//! - `--sandbox` (Landlock) chặn ghi ngoài khi chương trình **không** khai `cap` — chứng minh
//!   kernel cưỡng chế, không chỉ tầng runtime (đối chứng: không `--sandbox` thì ghi được).
//!
//! Các ca `..`, thiếu `cap`, `--strict-caps`, preopen WASI đã có ở `strict_caps.rs`, `cap.rs`,
//! `vs3_plugin.rs`; tệp này bổ sung đúng phần kiểm toán ST-309.

// Toàn bộ ca trong tệp chỉ chạy trên Unix/Linux (symlink + Landlock); trên Windows module rỗng
// để clippy `-D warnings` không bắt dead_code cho các helper.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;

fn tam(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!(
        "vis-st309-{tag}-{}-{}",
        std::process::id(),
        DEM.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    d
}

/// Đường dẫn viết được trong nguồn ViScript (Windows: `/`; Linux/macOS: giữ nguyên).
fn duong(p: &Path) -> String {
    p.display().to_string().replace('\\', "/")
}

fn ghi(p: &Path, s: &str) {
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).expect("mkdir");
    }
    std::fs::write(p, s).expect("ghi tệp");
}

fn chay(d: &Path, them: &[&str]) -> (i32, String, String) {
    let mut a = vec!["run", "--no-cache"];
    a.extend_from_slice(them);
    a.push("f.vis");
    let o = Command::new(crate::common::vis_bin())
        .current_dir(d)
        .args(&a)
        .output()
        .expect("chạy `vis`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// **Symlink tên cuối** trỏ ra ngoài `cap[Dir]`: `fs.read` **và** `fs.write` phải là `VS0604`
/// (trap, exit 2), không in nội dung ngoài, không ghi đè tệp ngoài. Chỉ Unix có symlink.
#[cfg(unix)]
#[test]
fn symlink_ten_cuoi_ra_ngoai_root_bi_chan_ca_doc_va_ghi() {
    let d = tam("symlink");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    let ngoai = d.join("ngoai.txt");
    ghi(&ngoai, "NGOAI");
    let link = root.join("link.txt");
    std::os::unix::fs::symlink(&ngoai, &link).expect("symlink");

    // Đọc: cap chỉ đọc, link trỏ ra ngoài.
    ghi(
        &d.join("f.vis"),
        &format!(
            "use std.fs\n\nfn main(d: cap[Dir] @ p\"{}\"):\n    match fs.read(\"{}\"):\n        Ok(s):\n            print(s)\n        Err(e):\n            print(\"ERR\")\n",
            duong(&root),
            duong(&link)
        ),
    );
    let (c, out, err) = chay(&d, &[]);
    assert_eq!(c, 2, "đọc qua symlink phải trap: {err}");
    assert!(err.contains("VS0604"), "phải là VS0604: {err}");
    assert_eq!(out, "", "không được rò nội dung ngoài: {out}");
    assert_eq!(std::fs::read_to_string(&ngoai).unwrap(), "NGOAI");

    // Ghi: cap có `(write)`, nhưng tên cuối là symlink ⇒ không được ghi xuyên link.
    ghi(
        &d.join("f.vis"),
        &format!(
            "use std.fs\n\nfn main(d: cap[Dir] @ p\"{}\" (write)):\n    match fs.write(\"{}\", \"GHI-DE\"):\n        Ok(_):\n            print(\"OK\")\n        Err(e):\n            print(\"ERR\")\n",
            duong(&root),
            duong(&link)
        ),
    );
    let (c, out, err) = chay(&d, &[]);
    assert_eq!(c, 2, "ghi qua symlink phải trap: {err}");
    assert!(err.contains("VS0604"), "phải là VS0604: {err}");
    assert_eq!(out, "", "không được báo thành công: {out}");
    assert_eq!(
        std::fs::read_to_string(&ngoai).unwrap(),
        "NGOAI",
        "tệp ngoài cap không được bị ghi đè"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// **Landlock**: không `cap`, không `--sandbox` ⇒ ghi ngoài được; có `--sandbox` ⇒ kernel chặn
/// (`Err` dữ liệu, chương trình in `ERR`, tệp ngoài không xuất hiện). Chỉ Linux có Landlock.
#[cfg(target_os = "linux")]
#[test]
fn sandbox_chan_ghi_ngoai_khi_khong_khai_cap() {
    // Nền không bật Landlock trong LSM: giữ nhánh bỏ qua rõ ràng như `sandbox.rs` (không xanh giả).
    let lsm = std::fs::read_to_string("/sys/kernel/security/lsm").unwrap_or_default();
    if !lsm.split(',').any(|x| x.trim() == "landlock") {
        eprintln!("BLOCKED-ENV: LSM không có landlock — bỏ ca sandbox (đã ghi lý do)");
        return;
    }
    let d = tam("sandbox");
    let ngoai = d.join("ngoai.txt");
    ghi(
        &d.join("f.vis"),
        &format!(
            "use std.fs\n\nfn main():\n    match fs.write(\"{}\", \"GHI\"):\n        Ok(_):\n            print(\"OK\")\n        Err(e):\n            print(\"ERR\")\n",
            duong(&ngoai)
        ),
    );

    // Đối chứng: không sandbox ⇒ OS cho ghi (tệp nằm trong thư mục tạm của chính test).
    let (c, out, err) = chay(&d, &[]);
    assert_eq!(c, 0, "đối chứng phải chạy: {err}");
    assert_eq!(out, "OK\n", "đối chứng phải ghi được: {err}");
    assert_eq!(std::fs::read_to_string(&ngoai).unwrap(), "GHI");
    std::fs::remove_file(&ngoai).expect("xoá đối chứng");

    // Trong sandbox: ruleset rỗng ⇒ mọi truy cập fs bị kernel từ chối.
    let (c, out, err) = chay(&d, &["--sandbox"]);
    assert_eq!(c, 0, "chương trình phải bắt `Err` dữ liệu: {err}");
    assert_eq!(out, "ERR\n", "kernel phải chặn ghi: {out} / {err}");
    assert!(
        err.contains("sandbox: landlock ABI="),
        "phải in dòng báo Landlock: {err}"
    );
    assert!(
        !ngoai.exists(),
        "tệp ngoài không được xuất hiện dưới sandbox"
    );
    let _ = std::fs::remove_dir_all(&d);
}
