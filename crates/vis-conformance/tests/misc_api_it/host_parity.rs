// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-315d (VD-291 §3.4)** — parity thư viện `interp ↔ VM` cho **66 hàm host** còn ngoài corpus
//! (`fs` 22, `sqlite` 28, `net` 10, `http` 4, `time` 2 — `JOURNAL/st314a-hau-kiem.md` §23.2).
//!
//! Mỗi ca chạy **cả hai đường** trên cùng tệp + cùng tempdir, đòi cùng stdout/exit; ca lỗi so cả
//! mã chẩn đoán + thông điệp. Chạy qua binary `vis` (đường CLI thật) để dùng đúng host native;
//! đường thư viện thuần không dựng được `FsGuard`/`NetGuard`/`DbGuard` nếu không lộ API nội bộ.
//!
//! Tệp nằm trong `misc_api_it/` và được khai `#[path]` ở `lib_api_it.rs` — **0 binary cấp 1 mới**
//! (R8). Không dùng máy `mps`; không đụng `docs/SPEC*`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

/// 66 hàm host bắt buộc phủ — khoá với `StdId::ALL` bằng test `phu_du_66_ham_host`.
pub const HOST_66: [&str; 66] = [
    // fs (22)
    "fs.append",
    "fs.canonicalize",
    "fs.copy",
    "fs.current_dir",
    "fs.exists",
    "fs.glob",
    "fs.is_dir",
    "fs.is_file",
    "fs.is_symlink",
    "fs.list",
    "fs.mkdir",
    "fs.mkdir_all",
    "fs.read",
    "fs.read_bytes",
    "fs.read_lines",
    "fs.read_link",
    "fs.remove",
    "fs.rename",
    "fs.size",
    "fs.walk",
    "fs.write",
    "fs.write_bytes",
    // sqlite (28)
    "sqlite.application_id",
    "sqlite.begin",
    "sqlite.changes",
    "sqlite.close",
    "sqlite.columns",
    "sqlite.commit",
    "sqlite.database_list",
    "sqlite.data_version",
    "sqlite.execute",
    "sqlite.execute_batch",
    "sqlite.explain",
    "sqlite.index_columns",
    "sqlite.indexes",
    "sqlite.index_exists",
    "sqlite.in_transaction",
    "sqlite.journal_mode",
    "sqlite.last_insert_rowid",
    "sqlite.open",
    "sqlite.open_with",
    "sqlite.primary_keys",
    "sqlite.query",
    "sqlite.query_one",
    "sqlite.rollback",
    "sqlite.schema_sql",
    "sqlite.schema_version",
    "sqlite.table_exists",
    "sqlite.tables",
    "sqlite.user_version",
    // net (10)
    "net.accept",
    "net.accept_timeout",
    "net.close",
    "net.close_listen",
    "net.connect",
    "net.connect_timeout",
    "net.listen",
    "net.local_addr",
    "net.read",
    "net.write",
    // http (4)
    "http.get",
    "http.get_timeout",
    "http.head",
    "http.head_timeout",
    // time (2)
    "time.now_ms",
    "time.monotonic_ms",
];

/// Thư mục tạm riêng cho từng ca (test chạy song song), tự dọn.
fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-host-parity-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

/// Đường dẫn kiểu VS (dấu `/`, kể cả Windows) để nhúng vào chương trình.
fn duong(p: &Path) -> String {
    p.display().to_string().replace('\\', "/")
}

/// Binary `vis` của workspace — tôn trọng `CARGO_TARGET_DIR` (worktree dùng chung target).
fn vis_bin() -> PathBuf {
    let goc = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| goc.join("target"));
    let ten = if cfg!(windows) { "vis.exe" } else { "vis" };
    let p = target.join("debug").join(ten);
    assert!(
        p.is_file(),
        "thiếu binary `{}` — build `cargo build -p vis-cli` trước khi chạy parity",
        p.display()
    );
    p
}

fn vis(args: &[&std::ffi::OsStr]) -> (i32, String, String) {
    let mut c = Command::new(vis_bin());
    for a in args {
        c.arg(a);
    }
    let o = c.output().expect("chạy `vis`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

fn viet_tep(p: &Path, src: &str) {
    std::fs::write(
        p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi chương trình");
}

/// Chạy **cả hai đường** (interp + `--vm --no-cache`) trên cùng tệp, đòi cùng stdout/exit.
/// Trả `(stdout, stderr)` để ca lỗi so tiếp mã/thông điệp.
fn hai_duong(d: &Path, tag: &str, src: &str) -> (String, String) {
    hai_duong_voi(d, tag, src, || {})
}

/// Như [`hai_duong`] nhưng gọi `setup` **trước mỗi đường** — ca ghi/xoá phải bắt đầu từ cùng
/// trạng thái fixture, không để đường interp làm bẩn dữ liệu của VM.
fn hai_duong_voi<F: Fn()>(d: &Path, tag: &str, src: &str, setup: F) -> (String, String) {
    let p = d.join(format!("{tag}.vis"));
    viet_tep(&p, src);
    setup();
    let (c1, o1, e1) = vis(&[std::ffi::OsStr::new("run"), p.as_os_str()]);
    setup();
    let (c2, o2, e2) = vis(&[
        std::ffi::OsStr::new("run"),
        std::ffi::OsStr::new("--vm"),
        std::ffi::OsStr::new("--no-cache"),
        p.as_os_str(),
    ]);
    assert_eq!(c1, c2, "[{tag}] exit lệch: interp {c1} / vm {c2}\nvm: {e2}");
    assert_eq!(o1, o2, "[{tag}] stdout lệch: vm [{e2}]");
    assert_eq!(e1, e2, "[{tag}] stderr lệch");
    (o1, e1)
}

/// **ST-315d** — bảng 66 tên phải khớp đúng tập host của `StdId::ALL`.
#[test]
fn phu_du_66_ham_host() {
    use vis_hir::stdlib::StdId;
    let mut tu_all: Vec<&str> = StdId::ALL
        .iter()
        .map(|id| id.as_str())
        .filter(|n| {
            ["fs.", "sqlite.", "net.", "http.", "time."]
                .iter()
                .any(|m| n.starts_with(m))
        })
        .collect();
    tu_all.sort_unstable();
    let mut bang = HOST_66.to_vec();
    bang.sort_unstable();
    assert_eq!(tu_all.len(), 66, "StdId host phải đúng 66: {tu_all:?}");
    assert_eq!(bang, tu_all, "bảng HOST_66 lệch StdId::ALL");
}

/// **ST-315d** — 2 hàm `time` chỉ so **kiểu + bất biến** (đồng hồ thật, không tất định).
#[test]
fn time_hai_ham_parity_bat_bien() {
    let d = tam("time");
    let (out, err) = hai_duong(
        &d,
        "time",
        "use std.time\nfn main():\n    let a = time.now_ms()\n    let b = time.monotonic_ms()\n    print(a >= 0)\n    print(b >= 0)\n    print(a > 1700000000000)\n",
    );
    assert_eq!(out, "true\ntrue\ntrue\n", "time: {err}");
    let _ = std::fs::remove_dir_all(&d);
}

fn dat_fixture(work: &Path) {
    let _ = std::fs::remove_dir_all(work);
    std::fs::create_dir_all(work.join("sub")).expect("mkdir sub");
    std::fs::write(work.join("a.txt"), "hi\n").expect("a.txt");
    std::fs::write(work.join("sub/b.txt"), "b").expect("b.txt");
    std::fs::write(work.join("bytes.bin"), [65u8, 66, 67]).expect("bytes.bin");
    #[cfg(unix)]
    std::os::unix::fs::symlink("a.txt", work.join("link.txt")).expect("symlink");
}

/// **ST-315d** — 20 hàm `fs` tất định trên tempdir (2 hàm symlink ở ca `#[cfg(unix)]` riêng).
#[test]
fn fs_20_ham_parity_tren_tempdir() {
    let d = tam("fs");
    let work = d.join("work");
    let w = duong(&work);
    let src = format!(
        r#"use std.fs
use std.path
fn main():
    let d = "{w}"
    print(fs.exists(d + "/a.txt"))
    print(fs.exists(d + "/missing"))
    match fs.is_file(d + "/a.txt"):
        Ok(b): print(b)
        Err(e): print("E")
    match fs.is_dir(d + "/sub"):
        Ok(b): print(b)
        Err(e): print("E")
    match fs.size(d + "/a.txt"):
        Ok(n): print(n)
        Err(e): print("E")
    match fs.read(d + "/a.txt"):
        Ok(s): print(s)
        Err(e): print("E")
    match fs.read_lines(d + "/a.txt"):
        Ok(xs): print(xs)
        Err(e): print("E")
    match fs.read_bytes(d + "/bytes.bin"):
        Ok(xs): print(xs)
        Err(e): print("E")
    match fs.write(d + "/w.txt", "w\n"):
        Ok(_): print("W")
        Err(e): print("E")
    match fs.write_bytes(d + "/w.bin", [68, 69]):
        Ok(_): print("B")
        Err(e): print("E")
    match fs.append(d + "/a.txt", "more\n"):
        Ok(_): print("A")
        Err(e): print("E")
    match fs.read(d + "/a.txt"):
        Ok(s): print(s)
        Err(e): print("E")
    match fs.copy(d + "/a.txt", d + "/cp.txt"):
        Ok(_): print("C")
        Err(e): print("E")
    match fs.rename(d + "/cp.txt", d + "/mv.txt"):
        Ok(_): print("R")
        Err(e): print("E")
    print(fs.exists(d + "/mv.txt"))
    match fs.mkdir(d + "/newdir"):
        Ok(_): print("M")
        Err(e): print("E")
    match fs.mkdir_all(d + "/deep/x"):
        Ok(_): print("MA")
        Err(e): print("E")
    match fs.is_dir(d + "/newdir"):
        Ok(b): print(b)
        Err(e): print("E")
    match fs.is_dir(d + "/deep/x"):
        Ok(b): print(b)
        Err(e): print("E")
    match fs.list(d + "/sub"):
        Ok(xs): print(xs)
        Err(e): print("E")
    match fs.walk(d):
        Ok(xs): print(xs.len())
        Err(e): print("E")
    match fs.glob(d, "*.txt"):
        Ok(xs): print(xs)
        Err(e): print("E")
    match fs.canonicalize(d + "/a.txt"):
        Ok(p): print(path.basename(p))
        Err(e): print("E")
    match fs.current_dir():
        Ok(p): print(p.len() > 0)
        Err(e): print("E")
    match fs.remove(d + "/mv.txt"):
        Ok(_): print("RM")
        Err(e): print("E")
    print(fs.exists(d + "/mv.txt"))
"#
    );
    let (out, err) = hai_duong_voi(&d, "fs", &src, || dat_fixture(&work));
    let dong: Vec<&str> = out.lines().collect();
    assert_eq!(
        &dong[..6],
        &["true", "false", "true", "true", "3", "hi"],
        "6 dòng đầu: {out}\nstderr: {err}"
    );
    for token in ["A", "C", "R", "M", "MA", "RM"] {
        assert!(dong.contains(&token), "thiếu `{token}`: {out}");
    }
    assert!(dong.contains(&"[b.txt]"), "fs.list: {out}");
    assert!(out.contains("a.txt"), "canonicalize/glob: {out}");
    assert_eq!(dong.last(), Some(&"false"), "remove phải xoá: {out}");
    let _ = std::fs::remove_dir_all(&d);
}

/// **ST-315d** — `fs.is_symlink` + `fs.read_link` (POSIX; Windows không tạo symlink không đặc quyền).
#[cfg(unix)]
#[test]
fn fs_symlink_parity() {
    let d = tam("fs-link");
    let work = d.join("work");
    let w = duong(&work);
    let src = format!(
        "use std.fs\nuse std.path\nfn main():\n    let d = \"{w}\"\n    match fs.is_symlink(d + \"/link.txt\"):\n        Ok(b): print(b)\n        Err(e): print(\"E\")\n    match fs.read_link(d + \"/link.txt\"):\n        Ok(p): print(path.basename(p))\n        Err(e): print(\"E\")\n"
    );
    let (out, err) = hai_duong_voi(&d, "fs-link", &src, || dat_fixture(&work));
    assert_eq!(out, "true\na.txt\n", "symlink: {err}");
    let _ = std::fs::remove_dir_all(&d);
}
