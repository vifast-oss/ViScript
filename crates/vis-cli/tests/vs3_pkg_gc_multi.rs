// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-5c (VD-271)** — `vis pkg gc` với **nhiều lock/workspace**:
//! `--lock` lặp lại, `--root` quét có kiểm soát; hợp tham chiếu của mọi lock; không xoá blob
//! được bất kỳ lock nào dùng; từ chối khi không có lock; dry-run/idempotent; không theo symlink.
//!
//! Hermetic: registry cục bộ `test/pkg/registry`, thư mục tạm riêng mỗi ca.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn tmp_dir(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vs3-gc-multi-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    d
}

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test/pkg/registry")
}

fn vis(cwd: &Path, args: &[&str]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("chạy `vis pkg`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

fn doc(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_else(|e| panic!("đọc {}: {e}", p.display()))
}

fn ghi(p: &Path, s: &str) {
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).expect("mkdir");
    }
    std::fs::write(p, s).expect("ghi");
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn blob_rác(tag: &str) -> String {
    hex(&vis_bytecode::bytes_hash(tag.as_bytes()))
}

fn them_goi(ws: &Path, spec: &str) {
    let fx = fixture();
    let (c, _o, e) = vis(
        ws,
        &[
            "pkg",
            "add",
            spec,
            "--registry",
            fx.to_str().expect("utf-8"),
        ],
    );
    assert_eq!(c, 0, "add {spec} tại {}: {e}", ws.display());
}

fn ten_blob_trong(ws: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let store = ws.join(".vis/store");
    if !store.is_dir() {
        return out;
    }
    for e in std::fs::read_dir(store).expect("read store") {
        let p = e.expect("entry").path();
        if let Some(n) = p.file_name().and_then(|s| s.to_str()) {
            if n.len() == 64 && n.bytes().all(|b| b.is_ascii_hexdigit()) {
                out.insert(n.to_string());
            }
        }
    }
    out
}

fn liet_ke(dir: &Path) -> Vec<String> {
    fn di(base: &Path, d: &Path, out: &mut Vec<String>) {
        for e in std::fs::read_dir(d).expect("read_dir") {
            let p = e.expect("entry").path();
            let rel = p
                .strip_prefix(base)
                .expect("rel")
                .to_string_lossy()
                .replace('\\', "/");
            out.push(rel);
            if p.is_dir() {
                di(base, &p, out);
            }
        }
    }
    let mut out = Vec::new();
    di(dir, dir, &mut out);
    out.sort();
    out
}

/// Ca 1 — `--root` quét hai workspace + store chung ở root: blob chia sẻ bởi 2 lock **không** bị
/// xoá; blob mồ côi trong cả 3 store bị xoá; gói lạ bị xoá; dry-run không đổi; lần hai 0/0.
#[test]
fn gc_nhieu_lock_root_giu_blob_chia_se() {
    let root = tmp_dir("root");
    let a = root.join("a");
    let b = root.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    them_goi(&a, "http@1.0.0");
    them_goi(&b, "http@1.0.0");

    let blob_a = ten_blob_trong(&a);
    let blob_b = ten_blob_trong(&b);
    let chung: Vec<String> = blob_a.intersection(&blob_b).cloned().collect();
    assert!(!chung.is_empty(), "hai lock phải chia sẻ ít nhất một blob");
    let shared = chung[0].clone();

    // Store chung ở root (mô phỏng monorepo): cùng blob + một blob mồ côi.
    std::fs::create_dir_all(root.join(".vis/store")).unwrap();
    std::fs::copy(
        a.join(".vis/store").join(&shared),
        root.join(".vis/store").join(&shared),
    )
    .unwrap();
    let rac_a = blob_rác("rac-a");
    let rac_b = blob_rác("rac-b");
    let rac_root = blob_rác("rac-root");
    ghi(&a.join(".vis/store").join(&rac_a), "x");
    ghi(&b.join(".vis/store").join(&rac_b), "x");
    ghi(&root.join(".vis/store").join(&rac_root), "x");
    ghi(&a.join(".vis/pkg/rac/9.9.9/src/main.vis"), "fn rac()\n");

    let truoc = liet_ke(&root);
    let (c, o, e) = vis(&root, &["pkg", "gc", "--root", ".", "--dry-run"]);
    assert_eq!(c, 0, "dry-run: {e}");
    assert!(o.contains("sẽ xoá"), "{o}");
    assert_eq!(liet_ke(&root), truoc, "dry-run không được đổi cây");

    let (c, o, e) = vis(&root, &["pkg", "gc", "--root", "."]);
    assert_eq!(c, 0, "gc --root: {e}");
    assert!(o.contains("đã xoá 3 blob, 1 mục gói"), "stdout: {o}");
    assert!(
        o.contains("2 lock, 3 workspace"),
        "phải báo nhiều lock/ws: {o}"
    );

    // Blob chia sẻ bởi hai lock (và bản copy ở store root) phải còn.
    assert!(a.join(".vis/store").join(&shared).is_file());
    assert!(b.join(".vis/store").join(&shared).is_file());
    assert!(root.join(".vis/store").join(&shared).is_file());
    // Blob mồ côi bị xoá ở cả ba store; gói lạ bị xoá.
    assert!(!a.join(".vis/store").join(&rac_a).exists());
    assert!(!b.join(".vis/store").join(&rac_b).exists());
    assert!(!root.join(".vis/store").join(&rac_root).exists());
    assert!(!a.join(".vis/pkg/rac").exists());
    // Bản đang dùng của cả hai workspace còn; lock không bị sửa.
    assert!(a.join(".vis/pkg/http/1.0.0").is_dir());
    assert!(b.join(".vis/pkg/http/1.0.0").is_dir());
    let lock_a = doc(&a.join("vis.lock"));
    let lock_b = doc(&b.join("vis.lock"));
    assert!(lock_a.contains("\"lock\":2"));
    assert!(lock_b.contains("\"lock\":2"));

    // Idempotent: lần hai 0/0, cây không đổi.
    let sau = liet_ke(&root);
    let (c, o, e) = vis(&root, &["pkg", "gc", "--root", "."]);
    assert_eq!(c, 0, "lần hai: {e}");
    assert!(o.contains("đã xoá 0 blob, 0 mục gói"), "{o}");
    assert_eq!(liet_ke(&root), sau);

    // Lặp `--lock`: cũng giữ blob chia sẻ, xoá blob mồ côi mới.
    let rac3 = blob_rác("rac-lock");
    ghi(&a.join(".vis/store").join(&rac3), "x");
    let (c, o, e) = vis(
        &root,
        &["pkg", "gc", "--lock", "a/vis.lock", "--lock", "b/vis.lock"],
    );
    assert_eq!(c, 0, "gc --lock: {e}");
    assert!(o.contains("đã xoá 1 blob, 0 mục gói"), "{o}");
    assert!(a.join(".vis/store").join(&shared).is_file());
    assert!(b.join(".vis/store").join(&shared).is_file());
    assert!(!a.join(".vis/store").join(&rac3).exists());

    let _ = std::fs::remove_dir_all(&root);
}

/// Ca 4 — `--root` **lặp lại, lồng nhau** (`.` chứa `a`,`b`) ⇒ lock/workspace **không đếm trùng**;
/// root **không có `.vis`** thì GC là no-op và **không được tạo** `.vis`/store giả.
#[test]
fn gc_lap_root_long_nhau_khong_dem_trung_va_khong_tao_vis() {
    let root = tmp_dir("root-lap");
    let a = root.join("a");
    let b = root.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    them_goi(&a, "http@1.0.0");
    them_goi(&b, "http@1.0.0");

    let blob_a = ten_blob_trong(&a);
    let blob_b = ten_blob_trong(&b);
    let shared = blob_a
        .intersection(&blob_b)
        .next()
        .expect("hai lock phải chia sẻ blob")
        .clone();
    let rac_b = blob_rác("rac-root-lap");
    ghi(&b.join(".vis/store").join(&rac_b), "x");

    let args = ["pkg", "gc", "--root", "a", "--root", "b", "--root", "."];
    let (c, o, e) = vis(&root, &args);
    assert_eq!(c, 0, "{e}");
    assert!(
        o.contains("2 lock, 3 workspace"),
        "phải dedupe lock (2) và gồm root + a + b (3): {o}"
    );
    assert!(o.contains("đã xoá 1 blob, 0 mục gói"), "{o}");
    assert!(a.join(".vis/store").join(&shared).is_file());
    assert!(b.join(".vis/store").join(&shared).is_file());
    assert!(!b.join(".vis/store").join(&rac_b).exists());
    // Root không có `.vis`: no-op, **không tạo** thư mục/store giả.
    assert!(!root.join(".vis").exists(), "không được tạo `.vis` ở root");
    assert!(!root.join(".vis/store").exists());

    // Idempotent: lần hai 0/0, vẫn không tạo `.vis`.
    let (c, o, e) = vis(&root, &args);
    assert_eq!(c, 0, "{e}");
    assert!(o.contains("đã xoá 0 blob, 0 mục gói"), "{o}");
    assert!(!root.join(".vis").exists());
    let _ = std::fs::remove_dir_all(&root);
}

/// Ca 2 — không có lock ⇒ từ chối, không xoá; `--lock` thiếu tệp/`--root` không có lock cũng vậy.
#[test]
fn gc_nhieu_lock_tu_choi_khi_thieu_lock() {
    let d = tmp_dir("thieu");
    let blob = blob_rác("giu-khi-thieu-lock");
    ghi(&d.join(".vis/store").join(&blob), "x");

    let (c, _o, e) = vis(&d, &["pkg", "gc"]);
    assert_eq!(c, 2);
    assert!(e.contains("vis.lock"), "{e}");
    assert!(d.join(".vis/store").join(&blob).is_file());

    let (c, _o, e) = vis(&d, &["pkg", "gc", "--lock", "khong-co.lock"]);
    assert_eq!(c, 2);
    assert!(e.contains("khong-co.lock"), "{e}");
    assert!(d.join(".vis/store").join(&blob).is_file());

    let empty = d.join("rong");
    std::fs::create_dir_all(&empty).unwrap();
    let (c, _o, e) = vis(&d, &["pkg", "gc", "--root", "rong"]);
    assert_eq!(c, 2);
    assert!(e.contains("không tìm thấy"), "{e}");
    assert!(d.join(".vis/store").join(&blob).is_file());
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 3 — symlink: `.vis/store` là symlink ra ngoài ⇒ từ chối **trước khi** xoá; blob ngoài còn.
/// Symlink lẻ trong store/bản gói chỉ bị xoá **link**, không đụng đích ngoài.
#[cfg(unix)]
#[test]
fn gc_khong_theo_symlink_va_khong_xoa_ngoai_store() {
    use std::os::unix::fs::symlink;

    let d = tmp_dir("symlink");
    let ws = d.join("ws");
    std::fs::create_dir_all(&ws).unwrap();
    them_goi(&ws, "http@1.0.0");

    // .vis/store -> thư mục ngoài: từ chối, không xoá gì.
    let ngoai = d.join("ngoai");
    std::fs::create_dir_all(&ngoai).unwrap();
    let blob_ngoai = blob_rác("ngoai-store");
    ghi(&ngoai.join(&blob_ngoai), "x");
    let store_that = ws.join(".vis/store-that");
    std::fs::rename(ws.join(".vis/store"), &store_that).unwrap();
    symlink(&ngoai, ws.join(".vis/store")).unwrap();
    let truoc = liet_ke(&ws);
    let (c, _o, e) = vis(&ws, &["pkg", "gc"]);
    assert_eq!(c, 2, "symlink store phải bị từ chối");
    assert!(e.contains("symlink"), "{e}");
    assert!(ngoai.join(&blob_ngoai).is_file(), "không xoá ngoài store");
    assert_eq!(liet_ke(&ws), truoc, "từ chối trước khi xoá");
    std::fs::remove_file(ws.join(".vis/store")).unwrap();
    std::fs::rename(&store_that, ws.join(".vis/store")).unwrap();

    // Symlink lẻ tên hex64 trong store: chỉ xoá link, đích ngoài còn.
    let blob_link = "e".repeat(64);
    let dich = d.join("dich-ngoai");
    ghi(&dich, "x");
    symlink(&dich, ws.join(".vis/store").join(&blob_link)).unwrap();
    let (c, o, e) = vis(&ws, &["pkg", "gc"]);
    assert_eq!(c, 0, "{e}");
    assert!(o.contains("đã xoá"), "{o}");
    assert!(!ws.join(".vis/store").join(&blob_link).exists());
    assert!(dich.is_file(), "không xoá đích symlink");

    // Bản gói là symlink ra ngoài: chỉ xoá link, thư mục ngoài còn.
    let ngoai_goi = d.join("ngoai-goi");
    std::fs::create_dir_all(&ngoai_goi).unwrap();
    let ver = ws.join(".vis/pkg/http/9.9.9");
    std::fs::create_dir_all(ver.parent().unwrap()).unwrap();
    symlink(&ngoai_goi, &ver).unwrap();
    let (c, o, e) = vis(&ws, &["pkg", "gc"]);
    assert_eq!(c, 0, "{e}");
    assert!(o.contains("đã xoá"), "{o}");
    assert!(!ver.exists(), "symlink bản gói phải bị xoá");
    assert!(ngoai_goi.is_dir(), "không xoá đích ngoài");

    let _ = std::fs::remove_dir_all(&d);
}
