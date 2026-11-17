// SPDX-License-Identifier: MIT OR Apache-2.0
//! **P0-3 (VD-170)** — `CHEAT-SHEET-ViScript.md` thành **doc-test**.
//!
//! Mỗi tệp `test/cheatsheet/*.vis` là một chương trình hoàn chỉnh trích từ ô ✔ của cheat sheet
//! (hoặc ghép lại thành chương trình hoàn chỉnh — thêm `print(...)`/khai báo cần thiết, **không**
//! đổi cú pháp trong ô). Harness chạy **qua binary** (`CARGO_BIN_EXE_vis`) như `sem.rs`:
//!
//! - `vis run <tệp>` phải thoát **0**;
//! - nếu tệp có dòng `# kỳ vọng: <dòng>` thì stdout (bỏ `\n` cuối) phải **bằng đúng** các dòng đó;
//! - nếu tệp có dòng `# kỳ vọng-test: <dòng>` thì `vis test <tệp>` phải thoát 0 và stdout chứa
//!   **đúng** dòng đó (ca `test fn` của §2).
//!
//! Khoá **đúng số ca** (như `sem.rs`) để không âm thầm bỏ ca; mỗi tệp phải có SPDX trong 5 dòng
//! đầu (R3, QD §9.3) + dòng nguồn `# CHEAT-SHEET §…` để đối chiếu ngược tài liệu.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Số ca (tệp `.vis`) — khoá cứng: thêm/bớt tệp phải sửa **có ý thức** (và dòng đầu cheat sheet).
const SO_CA: usize = 11;

fn vis() -> &'static str {
    env!("CARGO_BIN_EXE_vis")
}

fn thu_muc() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("test")
        .join("cheatsheet")
}

fn gom_tep() -> Vec<PathBuf> {
    let goc = thu_muc();
    let mut ds: Vec<PathBuf> = std::fs::read_dir(&goc)
        .unwrap_or_else(|e| panic!("không đọc được {}: {e}", goc.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "vis").unwrap_or(false))
        .collect();
    ds.sort();
    ds
}

/// Các dòng sau marker (giữ nguyên phần sau **một** dấu cách đầu tiên).
fn ky_vong(src: &str, marker: &str) -> Vec<String> {
    let mut ds = Vec::new();
    for line in src.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix(marker) {
            ds.push(rest.strip_prefix(' ').unwrap_or(rest).to_string());
        }
    }
    ds
}

fn chay(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(vis())
        .args(args)
        .output()
        .expect("phải chạy được binary `vis`");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

#[test]
fn cheatsheet_moi_vi_du_chay_duoc() {
    let ds = gom_tep();
    assert_eq!(
        ds.len(),
        SO_CA,
        "số ca `test/cheatsheet/*.vis` đổi: {} (khoá {SO_CA}) — cập nhật SO_CA + dòng đầu CHEAT-SHEET",
        ds.len()
    );
    let mut hong: Vec<String> = Vec::new();
    for f in &ds {
        let ten = f.file_name().unwrap().to_string_lossy().to_string();
        let src = std::fs::read_to_string(f).unwrap_or_else(|e| panic!("đọc {}: {e}", f.display()));
        // R3 (QD §9.3): SPDX trong 5 dòng đầu; nguồn trích dẫn để người đọc đối chiếu cheat sheet.
        assert!(
            src.lines()
                .take(5)
                .any(|l| l.contains("SPDX-License-Identifier")),
            "{ten}: thiếu SPDX trong 5 dòng đầu"
        );
        assert!(
            src.lines()
                .any(|l| l.trim_start().starts_with("# CHEAT-SHEET §")),
            "{ten}: thiếu dòng nguồn `# CHEAT-SHEET §…`"
        );

        let (ma, out, err) = chay(&["run", f.to_str().unwrap()]);
        if ma != 0 {
            hong.push(format!("{ten}: `vis run` thoát {ma}\nstderr: {err}"));
            continue;
        }
        let mong = ky_vong(&src, "# kỳ vọng:");
        if !mong.is_empty() {
            let thuc = out.strip_suffix('\n').unwrap_or(&out);
            let mong = mong.join("\n");
            if thuc != mong {
                hong.push(format!(
                    "{ten}: stdout khác kỳ vọng\n--- mong ---\n{mong}\n--- thực ---\n{thuc}"
                ));
            }
        }
        let mong_test = ky_vong(&src, "# kỳ vọng-test:");
        if !mong_test.is_empty() {
            let (ma, out, err) = chay(&["test", f.to_str().unwrap()]);
            if ma != 0 {
                hong.push(format!("{ten}: `vis test` thoát {ma}\nstderr: {err}"));
            } else {
                for d in &mong_test {
                    if !out.lines().any(|l| l == d) {
                        hong.push(format!("{ten}: `vis test` thiếu dòng `{d}`\nstdout: {out}"));
                    }
                }
            }
        }
    }
    assert!(
        hong.is_empty(),
        "{} ca hỏng:\n{}",
        hong.len(),
        hong.join("\n\n")
    );
}
