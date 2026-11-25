// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-151** — `bin/vis-linux` sync list là một nguồn (test đọc script, không ssh/rsync).
//!
//! Tách 4 ca API khỏi `vis-cli/tests/cli_it/sync_list.rs` (ST-313c/VD-289);
//! 5 ca CLI `--check-sync` giữ ở `vis-cli`.

use std::path::{Path, PathBuf};

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

fn goc_repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn script() -> PathBuf {
    goc_repo().join("bin/vis-linux")
}

fn doc_script(duong: &Path) -> String {
    std::fs::read_to_string(duong).unwrap_or_else(|e| panic!("đọc {}: {e}", duong.display()))
}

/// Số dòng (1-based) của dòng **đầu tiên** chứa `manh`.
fn so_dong(noi_dung: &str, manh: &str) -> usize {
    noi_dung
        .lines()
        .position(|l| l.contains(manh))
        .map(|i| i + 1)
        .unwrap_or_else(|| panic!("thiếu dòng chứa {manh:?} trong bin/vis-linux"))
}

/// Giá trị của dòng `TEN="…"` (bỏ nháy `"` hoặc `'`, tách theo khoảng trắng).
fn danh_sach(noi_dung: &str, ten: &str) -> Vec<String> {
    let dau = format!("{ten}=");
    let dong = noi_dung
        .lines()
        .find(|l| l.trim_start().starts_with(&dau))
        .unwrap_or_else(|| panic!("thiếu dòng `{dau}` trong bin/vis-linux"));
    let gia = dong
        .split_once('=')
        .map(|(_, v)| v)
        .expect("dòng có `=`")
        .trim();
    let gia = [('"', '"'), ('\'', '\'')]
        .iter()
        .find_map(|(a, b)| gia.strip_prefix(*a).and_then(|s| s.strip_suffix(*b)))
        .unwrap_or(gia);
    gia.split_whitespace().map(str::to_string).collect()
}

/// Tập (đã sắp xếp, bỏ trùng) để so **tập** chứ không so thứ tự.
fn tap(xs: &[String]) -> Vec<String> {
    let mut v = xs.to_vec();
    v.sort();
    v.dedup();
    v
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

#[test]
fn vis_linux_hop_le_va_hai_bien_nam_o_dau_tep() {
    let p = script();
    assert!(p.exists(), "thiếu {}", p.display());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&p)
            .expect("metadata")
            .permissions()
            .mode();
        assert!(mode & 0o111 != 0, "bin/vis-linux phải +x (mode {mode:o})");
    }
    let s = doc_script(&p);
    assert!(
        s.lines()
            .any(|l| l.starts_with("# SPDX-License-Identifier: MIT OR Apache-2.0")),
        "thiếu dòng SPDX `#`"
    );
    assert!(
        s.lines().any(|l| l.trim_end() == "set -u"),
        "thiếu `set -u`"
    );
    // "Một nguồn ở đầu tệp": cả hai biến phải khai báo TRƯỚC `sync_tree()`.
    let dong_sync_tree = so_dong(&s, "sync_tree() {");
    for ten in ["SYNC_DIRS=", "SYNC_FILES="] {
        let d = so_dong(&s, ten);
        assert!(
            d < dong_sync_tree,
            "`{ten}` ở dòng {d} phải TRƯỚC `sync_tree()` (dòng {dong_sync_tree})"
        );
    }
}

#[test]
fn sync_dirs_dung_tap() {
    let s = doc_script(&script());
    let co = danh_sach(&s, "SYNC_DIRS");
    assert_eq!(
        tap(&co),
        tap(&DIRS.map(str::to_string)),
        "SYNC_DIRS = {co:?} — phải đúng tập {DIRS:?}"
    );
}

#[test]
fn sync_files_dung_tap() {
    let s = doc_script(&script());
    let co = danh_sach(&s, "SYNC_FILES");
    assert_eq!(
        tap(&co),
        tap(&FILES.map(str::to_string)),
        "SYNC_FILES = {co:?} — phải đúng tập {FILES:?}"
    );
}

#[test]
fn sync_tree_chi_dung_hai_bien() {
    let s = doc_script(&script());
    let than = than_ham(&s, "sync_tree");
    let ma = dong_ma(&than);
    assert!(
        ma.iter().any(|l| l.contains("in $SYNC_DIRS")),
        "sync_tree phải lặp `for … in $SYNC_DIRS`, thân hàm: {than}"
    );
    assert!(
        ma.iter().any(|l| l.contains("in $SYNC_FILES")),
        "sync_tree phải lặp `for … in $SYNC_FILES`, thân hàm: {than}"
    );
    // Đường dẫn KHÔNG được rải lại trong thân hàm: mọi mục phải đi qua hai biến.
    for ten in TAT_CA {
        let manh = format!("$REPO_ROOT/{ten}");
        assert!(
            !ma.iter().any(|l| l.contains(&manh)),
            "sync_tree còn đường dẫn cứng `{manh}` — danh sách phải là một nguồn ở SYNC_*"
        );
    }
}
