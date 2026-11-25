// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-151 (nợ #135)** — danh sách đồng bộ của `bin/vis-linux` là **một nguồn** (khuôn
//! `soak_script.rs`: test đọc script shell, không chạy soak/ssh/rsync).
//!
//! Khoá ba việc:
//! 1. `SYNC_DIRS`/`SYNC_FILES` ở đầu tệp đúng **tập** mong đợi (so tập, không phụ thuộc thứ tự);
//! 2. `sync_tree()` chỉ đọc hai biến ấy — không rải đường dẫn vào thân hàm (biến không phải trang trí);
//! 3. `--check-sync` in đủ 14 mục và trả **0** khi đủ / **2** khi thiếu, **không** ssh, **không** rsync.
//!
//! Bỏ một mục khỏi `SYNC_DIRS`/`SYNC_FILES`, hoặc rải lại đường dẫn vào `sync_tree()`, là test **đỏ**.

use std::path::{Path, PathBuf};
use std::process::Command;

/// `bash` thật cho script shell (`vis-linux`/`vis-soak`); trên Windows PATH trả WSL stub
/// (`execvpe(/bin/bash) failed`) nên dùng Git Bash tuyệt đối thay thế.
fn bash() -> PathBuf {
    #[cfg(windows)]
    {
        for c in [
            "C:/Program Files/Git/bin/bash.exe",
            "C:/Program Files (x86)/Git/bin/bash.exe",
        ] {
            if Path::new(c).is_file() {
                return PathBuf::from(c);
            }
        }
    }
    PathBuf::from("bash")
}

/// Tập mong đợi — đúng 14 mục: 9 thư mục + 5 tệp (ST-171: gốc repo mới **là** cây ViScript,
/// nên danh sách đồng bộ là toàn bộ cây nguồn thay cho `viscript/` + `docs/` cũ).
const DIRS: [&str; 9] = [
    "crates",
    "bin",
    "test",
    "agent",
    "infra",
    "docs",
    "vis-host-linux",
    "vis-host-wasm",
    ".git",
];
const FILES: [&str; 5] = ["AGENTS.md", "Cargo.toml", "Cargo.lock", "vis-dev", "vis"];
const TAT_CA: [&str; 14] = [
    "crates",
    "bin",
    "test",
    "agent",
    "infra",
    "docs",
    "vis-host-linux",
    "vis-host-wasm",
    ".git",
    "AGENTS.md",
    "Cargo.toml",
    "Cargo.lock",
    "vis-dev",
    "vis",
];

/// Gốc repo thật: `env!("CARGO_MANIFEST_DIR")` = `<repo>/crates/vis-cli`.
fn goc_repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn script() -> PathBuf {
    goc_repo().join("bin/vis-linux")
}

fn doc_script(duong: &Path) -> String {
    std::fs::read_to_string(duong).unwrap_or_else(|e| panic!("đọc {}: {e}", duong.display()))
}

/// Thân hàm `ten() { … }` (từ dòng khai báo tới `}` đóng ở cột 1).
fn than_ham(noi_dung: &str, ten: &str) -> String {
    let dau = format!("{ten}() {{");
    let bat_dau = noi_dung
        .find(&dau)
        .unwrap_or_else(|| panic!("thiếu khai báo `{dau}`"));
    let sau = &noi_dung[bat_dau..];
    let cuoi = sau
        .find("\n}\n")
        .unwrap_or_else(|| panic!("thiếu `}}` đóng `{ten}`"));
    sau[..cuoi + 3].to_string()
}

/// Chỉ các **dòng mã** của một đoạn script (bỏ dòng chú thích `#`).
fn dong_ma(than: &str) -> Vec<String> {
    than.lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .map(str::to_string)
        .collect()
}

/// Cây giả trong thư mục tạm — `bin/vis-linux` là **bản sao** script thật, để test
/// được cả nhánh thiếu mục mà **không** sửa cây thật.
struct CayGia(PathBuf);

impl CayGia {
    fn moi(ten: &str) -> Self {
        let goc = std::env::temp_dir().join(format!("vis-sync-list-{}-{ten}", std::process::id()));
        let _ = std::fs::remove_dir_all(&goc);
        std::fs::create_dir_all(goc.join("bin")).expect("tạo cây giả");
        std::fs::copy(script(), goc.join("bin/vis-linux")).expect("chép vis-linux");
        Self(goc)
    }

    fn goc(&self) -> &Path {
        &self.0
    }

    fn script(&self) -> PathBuf {
        self.0.join("bin/vis-linux")
    }
}

impl Drop for CayGia {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Dựng cây giả **đủ 14 mục** trừ những tên trong `bo` (cây giả luôn có `bin`: đó là chỗ chứa script).
fn dung_cay(cay: &CayGia, bo: &[&str]) {
    for d in DIRS {
        if bo.contains(&d) {
            continue;
        }
        std::fs::create_dir_all(cay.goc().join(d)).expect("tạo mục giả");
    }
    for f in FILES {
        if bo.contains(&f) {
            continue;
        }
        std::fs::write(cay.goc().join(f), b"x\n").expect("tạo tệp giả");
    }
}

/// Chạy `bash <script> --check-sync` với cwd = gốc repo.
fn chay_check_sync(script: &Path, goc: &Path) -> (Option<i32>, String, String) {
    let o = Command::new(bash())
        .arg(script)
        .arg("--check-sync")
        .current_dir(goc)
        .output()
        .expect("chạy --check-sync");
    (
        o.status.code(),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

/// Các dòng `trạng-thái tên` của stdout (`có`/`thiếu`).
fn trang_thai(stdout: &str) -> Vec<(String, String)> {
    stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let mut it = l.split_whitespace();
            let t = it.next().unwrap_or("").to_string();
            (t, it.collect::<Vec<_>>().join(" "))
        })
        .collect()
}

#[test]
fn check_sync_khong_ssh_khong_rsync() {
    let s = doc_script(&script());
    let than = than_ham(&s, "cmd_check_sync");
    let ma = dong_ma(&than);
    assert!(
        ma.iter().any(|l| l.contains("$SYNC_DIRS")),
        "cmd_check_sync phải đọc $SYNC_DIRS"
    );
    assert!(
        ma.iter().any(|l| l.contains("$SYNC_FILES")),
        "cmd_check_sync phải đọc $SYNC_FILES"
    );
    for cam in ["ssh", "rsync", "scp"] {
        assert!(
            !ma.iter().any(|l| l.contains(cam)),
            "cmd_check_sync KHÔNG được gọi `{cam}`: {than}"
        );
    }
    let nhanh = s
        .lines()
        .find(|l| l.trim_start().starts_with("--check-sync)"))
        .expect("thiếu nhánh `--check-sync)` trong khối case");
    assert!(
        !nhanh.contains("pick_backend") && !nhanh.contains("sync_tree"),
        "nhánh --check-sync không được chọn backend / đồng bộ: {nhanh}"
    );
}

#[test]
fn check_sync_tai_repo_that_in_du_8_muc_va_exit_0() {
    let (ma, out, err) = chay_check_sync(&script(), &goc_repo());
    assert_eq!(ma, Some(0), "stdout: {out:?}\nstderr: {err:?}");
    let tt = trang_thai(&out);
    let mong: Vec<(String, String)> = TAT_CA
        .iter()
        .map(|t| ("có".to_string(), t.to_string()))
        .collect();
    assert_eq!(tt, mong, "stdout: {out:?}");
    assert_eq!(tt.len(), 14, "phải in đúng 14 mục: {out:?}");
    assert!(
        !out.contains("thiếu"),
        "cây thật không được thiếu mục: {out:?}"
    );
}

#[test]
fn check_sync_thieu_mot_muc_thi_exit_2() {
    // Ca nợ #135: **bỏ một mục** khỏi cây (ở đây `vis-host-wasm`, đúng thư mục từng bị quên) ⇒ exit 2,
    // in đủ 14 mục và CHỈ đúng mục thiếu bị đánh dấu.
    let cay = CayGia::moi("thieu-mot");
    dung_cay(&cay, &["vis-host-wasm"]);
    let (ma, out, err) = chay_check_sync(&cay.script(), cay.goc());
    assert_eq!(
        ma,
        Some(2),
        "thiếu mục ⇒ exit 2. stdout: {out:?}\nstderr: {err:?}"
    );
    let tt = trang_thai(&out);
    let mong: Vec<(String, String)> = TAT_CA
        .iter()
        .map(|t| {
            let trang = if *t == "vis-host-wasm" {
                "thiếu"
            } else {
                "có"
            };
            (trang.to_string(), t.to_string())
        })
        .collect();
    assert_eq!(tt, mong, "stdout: {out:?}");
    assert!(
        err.contains("thiếu 1/14"),
        "phải nói rõ số mục thiếu (1/14): {err:?}"
    );
}

#[test]
fn check_sync_du_het_thi_exit_0_trong_cay_gia() {
    let cay = CayGia::moi("du");
    dung_cay(&cay, &[]);
    let (ma, out, err) = chay_check_sync(&cay.script(), cay.goc());
    assert_eq!(
        ma,
        Some(0),
        "đủ mục ⇒ exit 0. stdout: {out:?}\nstderr: {err:?}"
    );
    let tt = trang_thai(&out);
    assert_eq!(tt.len(), 14, "phải in đúng 14 mục: {out:?}");
    assert!(
        tt.iter().all(|(t, _)| t == "có"),
        "cây giả đủ ⇒ mọi mục `có`: {out:?}"
    );
}

#[test]
fn help_co_check_sync() {
    let o = Command::new(bash())
        .arg(script())
        .arg("--help")
        .current_dir(goc_repo())
        .output()
        .expect("chạy --help");
    assert_eq!(
        o.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    let out = String::from_utf8_lossy(&o.stdout);
    for manh in ["--check-sync", "Mã thoát", "SPDX-License-Identifier"] {
        assert!(out.contains(manh), "`--help` thiếu {manh:?}: {out}");
    }
}
