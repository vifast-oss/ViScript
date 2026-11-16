// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-116b**: helper `vis run` (thoát/quyền/run-dry) — tách khỏi `main.rs`; `run_run` ở lại main
//! để giữ nguyên `cache::khoa_multi` (ca `cache_fastpath` kiểm `main.rs`).

use crate::*;
use std::process::ExitCode;

/// **ST-72b**: lỗi **vượt quyền** (`VS0604`, message đánh dấu `[quyền]`) là lỗi **môi trường/dùng**
/// ⇒ exit **2** (yêu cầu ST-72b); các lỗi runtime khác — kể cả `VS0604` **không do quyền** (thiếu
/// host) — giữ exit **1** như trước (tương thích `pipeline::vm_khong_host_bao_vs0604`).
pub(crate) fn ma_thoat_khi_loi(diags: &[(String, u32, u32, String)]) -> u8 {
    if diags.iter().any(|d| d.3.starts_with("[quyền]")) {
        2
    } else {
        1
    }
}

/// **ST-73 (VD-107)**: probe Landlock **chỉ đọc tệp** (không syscall, không `unsafe`).
/// Trả `(dòng in, mã thoát)`: Linux có ⇒ 0, không ⇒ 3, không đọc được ⇒ 4; nền khác ⇒ 3.
pub(crate) fn probe_landlock() -> (String, u8) {
    let os = std::env::consts::OS;
    if os != "linux" {
        return (format!("landlock: không hỗ trợ trên {os}"), 3);
    }
    match std::fs::read_to_string("/sys/kernel/security/lsm") {
        Ok(raw) => {
            let lsm: Vec<String> = raw
                .trim()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let co = lsm.iter().any(|x| x == "landlock");
            (
                format!(
                    "landlock: {} (LSM: {})",
                    if co { "có" } else { "không" },
                    lsm.join(",")
                ),
                if co { 0 } else { 3 },
            )
        }
        Err(e) => (format!("landlock: không rõ ({e})"), 4),
    }
}

/// `vis run --sandbox-check`: in **stdout** 3 dòng chẩn đoán, exit theo probe, **không** chạy.
pub(crate) fn in_sandbox_check(caps: &[quyen::CapKhai]) -> ExitCode {
    let (dong_landlock, ma) = probe_landlock();
    println!("{dong_landlock}");
    println!(
        "cưỡng chế runtime: {}",
        if caps.is_empty() { "tắt" } else { "bật" }
    );
    let mut dk: Vec<&str> = Vec::new();
    if caps.iter().any(|c| c.kind == "Dir" || c.kind == "File") {
        dk.push("fs");
    }
    if caps.iter().any(|c| c.kind == "Net") {
        dk.push("net");
    }
    println!(
        "cap điều kiện: {}",
        if dk.is_empty() {
            "không".to_string()
        } else {
            dk.join(",")
        }
    );
    ExitCode::from(ma)
}

/// **ST-72 (VD-106 (2))**: in danh sách quyền khai báo của `main` (từ **CST**), exit 0, không chạy.
///
/// Định dạng cố định (stdout **rỗng**; danh sách ra **stderr** như `--dump-deps`):
/// `tên: cap[Loại] đường-dẫn (write)`. Không có quyền ⇒ `(không khai báo quyền)`.
pub(crate) fn run_dry(tep: &str, text: &str, mode: Mode) -> ExitCode {
    let caps = quyen::ds_cap(text, mode);
    let _ = tep;
    if caps.is_empty() {
        eprintln!("(không khai báo quyền)");
        return ExitCode::SUCCESS;
    }
    for c in &caps {
        let quyen = if c.write { " (write)" } else { "" };
        if c.path.is_empty() {
            eprintln!("{}: cap[{}]{quyen}", c.name, c.kind);
        } else {
            eprintln!("{}: cap[{}] {}{quyen}", c.name, c.kind, c.path);
        }
    }
    ExitCode::SUCCESS
}

/// **ST-132 (VD-120)**: `vis run --sandbox` — cưỡng chế quyền bằng **kernel** (Landlock) trên Linux.
///
/// Áp **sau** khi đã nạp xong chương trình/module và **trước** khi chạy: Landlock chỉ chặn được
/// truy cập phát sinh **sau** lời gọi. Chỉ **hệ tệp** (`cap[Dir]`/`cap[File]`); `cap[Net]` và
/// `cap[Proc]` vẫn cưỡng chế ở runtime như cũ.
///
/// Trả `Err(exit)` khi phải dừng: **3** = nền không hỗ trợ (giống `--sandbox-check`), **2** = lỗi
/// dựng luật (đường dẫn không tồn tại, kernel từ chối).
#[cfg(target_os = "linux")]
pub(crate) fn ap_sandbox(q: &quyen::Quyen) -> Result<(), ExitCode> {
    use vis_host_linux::sandbox::{ap_dung, ChoPhep, LoiSandbox, Muc};
    let cho: Vec<ChoPhep> = q
        .moc
        .iter()
        .filter(|c| (c.kind == "Dir" || c.kind == "File") && !c.root.is_empty())
        .map(|c| ChoPhep {
            duong: std::path::PathBuf::from(&c.root),
            ghi: c.write,
            la_tep: c.kind == "File",
        })
        .collect();
    match ap_dung(&cho) {
        Ok(kq) => {
            eprintln!("{}", kq.dong());
            if matches!(kq.muc, Muc::Khong) {
                eprintln!("sandbox: kernel KHÔNG cưỡng chế được (exit 3)");
                return Err(ExitCode::from(3));
            }
            Ok(())
        }
        Err(LoiSandbox::KhongHoTro(s)) => {
            eprintln!("sandbox: {s} (exit 3)");
            Err(ExitCode::from(3))
        }
        Err(e) => {
            eprintln!("sandbox: {e}");
            Err(ExitCode::from(2))
        }
    }
}

/// Nền không phải Linux: `--sandbox` **không** cưỡng chế được ⇒ exit **3** như `--sandbox-check`.
#[cfg(not(target_os = "linux"))]
pub(crate) fn ap_sandbox(_q: &quyen::Quyen) -> Result<(), ExitCode> {
    eprintln!(
        "sandbox: chỉ hỗ trợ trên Linux (nền này: {}) (exit 3)",
        std::env::consts::OS
    );
    Err(ExitCode::from(3))
}
