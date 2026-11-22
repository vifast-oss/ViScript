// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-5b (VD-271)** — `vis pkg gc` (dọn blob/bản cũ, idempotent) và `vis pkg migrate`
//! (nâng `vis.lock` định dạng 1 → 2 an toàn, không đổi byte khi đã mới).
//!
//! Hermetic: fixture registry cục bộ `test/pkg/registry`, thư mục tạm riêng mỗi ca.

use std::path::{Path, PathBuf};
use std::process::Command;

fn tmp_dir(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vs3-gc-{tag}-{}-{n}", std::process::id()));
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

/// Đổi lock định dạng 2 → định dạng **1** (bỏ `roots` + mọi `version_req`, `lock` = 1).
fn ve_dinh_dang_cu(lock: &str) -> String {
    let mut s = lock.replace("\"lock\":2", "\"lock\":1");
    if let Some(i) = s.find("\"roots\":{") {
        let j = s[i..]
            .find("},")
            .map(|k| i + k + 2)
            .expect("roots kết thúc");
        s.replace_range(i..j, "");
    }
    while let Some(i) = s.find("\"version_req\":\"") {
        let j = s[i..].find("\",\"").map(|k| i + k + 1).expect("kết thúc");
        s.replace_range(i..j + 1, "");
    }
    s
}

/// Liệt kê **mọi** đường dẫn tương đối dưới `dir` (tệp + thư mục), sắp xếp — so cây trước/sau.
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

/// Ca 1 — GC xoá **blob lạ + bản cũ** của gói, giữ bản đang dùng; `--dry-run` không xoá;
/// chạy lại ⇒ 0/0; `install` sau GC vẫn dựng được cây từ blob còn lại.
#[test]
fn gc_don_ban_cu_va_blob_mo_coi() {
    let d = tmp_dir("gc");
    let fx = fixture();
    let fx = fx.to_str().unwrap();
    for ver in ["http@1.0.0", "http@1.9.0", "http@1.10.0"] {
        let (c, _o, e) = vis(&d, &["pkg", "add", ver, "--registry", fx]);
        assert_eq!(c, 0, "{ver}: {e}");
    }
    // Bản cũ còn trên đĩa (add không dọn) + bản mới đang dùng.
    assert!(d.join(".vis/pkg/http/1.0.0").is_dir());
    assert!(d.join(".vis/pkg/http/1.9.0").is_dir());
    assert!(d.join(".vis/pkg/http/1.10.0").is_dir());
    assert!(d.join(".vis/pkg/util/0.1.0").is_dir());

    // Rác: 1 blob mồ côi, 1 gói lạ; tệp không-phải-hex trong store phải được giữ.
    let mo_coi = hex(&vis_bytecode::bytes_hash(b"mo-coi"));
    ghi(&d.join(".vis/store").join(&mo_coi), "x");
    ghi(&d.join(".vis/pkg/rac/9.9.9/src/main.vis"), "fn rac()\n");
    ghi(&d.join(".vis/store/README"), "khong phai blob\n");

    let lock = doc(&d.join("vis.lock"));
    let truoc = liet_ke(&d);
    let (c, o, e) = vis(&d, &["pkg", "gc", "--dry-run"]);
    assert_eq!(c, 0, "stderr: {e}");
    assert!(o.contains("sẽ xoá"), "stdout: {o}");
    assert_eq!(liet_ke(&d), truoc, "dry-run KHÔNG được xoá gì");

    let (c, o, e) = vis(&d, &["pkg", "gc"]);
    assert_eq!(c, 0, "stderr: {e}");
    assert!(
        o.contains("đã xoá 3 blob, 3 mục gói"),
        "đếm sai (2 blob http cũ + blob mồ côi, 2 bản http cũ + rac): {o}"
    );
    assert_eq!(doc(&d.join("vis.lock")), lock, "GC không được sửa lock");
    assert!(!d.join(".vis/pkg/http/1.0.0").exists());
    assert!(!d.join(".vis/pkg/http/1.9.0").exists());
    assert!(!d.join(".vis/pkg/rac").exists());
    assert!(!d.join(".vis/store").join(&mo_coi).exists());
    assert!(
        d.join(".vis/store/README").exists(),
        "giữ tệp không-phải-hex"
    );
    assert!(d.join(".vis/pkg/http/1.10.0").is_dir());
    assert!(d.join(".vis/pkg/util/0.1.0").is_dir());

    // Blob của bản đang dùng còn ⇒ `install` dựng lại cây không cần registry.
    let (c, _o, e) = vis(&d, &["pkg", "install"]);
    assert_eq!(c, 0, "install sau gc: {e}");
    assert!(d.join(".vis/pkg/http/1.10.0/src/main.vis").is_file());

    // Idempotent: lần hai 0/0, cây không đổi.
    let sau = liet_ke(&d);
    let (c, o, e) = vis(&d, &["pkg", "gc"]);
    assert_eq!(c, 0, "stderr: {e}");
    assert!(o.contains("đã xoá 0 blob, 0 mục gói"), "stdout: {o}");
    assert_eq!(liet_ke(&d), sau, "GC lần hai phải không đổi cây");
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 2 — thiếu `vis.lock` ⇒ từ chối, **không** dọn mù.
#[test]
fn gc_thieu_lock_tu_choi() {
    let d = tmp_dir("gc-khong-lock");
    let blob = hex(&vis_bytecode::bytes_hash(b"giu"));
    ghi(&d.join(".vis/store").join(&blob), "x");
    let (c, _o, e) = vis(&d, &["pkg", "gc"]);
    assert_eq!(c, 2, "stderr: {e}");
    assert!(e.contains("vis.lock"), "{e}");
    assert!(d.join(".vis/store").join(&blob).exists(), "không được xoá");
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 3 — `migrate` lock cũ: ghim `=<version>`, giữ `entry`, lần hai cùng byte; `install` sau đó OK.
#[test]
fn migrate_lock_cu_va_idempotent() {
    let d = tmp_dir("migrate");
    let fx = fixture();
    let (c, _o, e) = vis(
        &d,
        &[
            "pkg",
            "add",
            "http@1.0.0",
            "--registry",
            fx.to_str().unwrap(),
        ],
    );
    assert_eq!(c, 0, "stderr: {e}");
    let cu = ve_dinh_dang_cu(&doc(&d.join("vis.lock")));
    assert!(cu.contains("\"lock\":1"), "{cu}");
    assert!(!cu.contains("version_req"), "{cu}");
    assert!(!cu.contains("\"roots\""), "{cu}");
    ghi(&d.join("vis.lock"), &cu);

    let (c, o, e) = vis(&d, &["pkg", "migrate"]);
    assert_eq!(c, 0, "stderr: {e}");
    assert!(o.contains("đã nâng cấp"), "stdout: {o}");
    let moi = doc(&d.join("vis.lock"));
    assert!(moi.contains("\"lock\":2"), "{moi}");
    assert!(moi.contains("\"version_req\":\"=1.0.0\""), "{moi}");
    assert!(moi.contains("\"version_req\":\"=0.1.0\""), "{moi}");
    assert!(moi.contains("\"entry\":\"src/main.vis\""), "{moi}");

    let (c, _o, e) = vis(&d, &["pkg", "install"]);
    assert_eq!(c, 0, "install sau migrate: {e}");
    assert_eq!(doc(&d.join("vis.lock")), moi, "install không đổi byte");

    let (c, o, e) = vis(&d, &["pkg", "migrate"]);
    assert_eq!(c, 0, "stderr: {e}");
    assert!(o.contains("không đổi"), "stdout: {o}");
    assert_eq!(doc(&d.join("vis.lock")), moi, "migrate lần hai cùng byte");
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 4 — `gc` **không** tự migrate/ghi lock (chỉ đọc) — lock cũ giữ nguyên byte.
#[test]
fn gc_khong_sua_lock_cu() {
    let d = tmp_dir("gc-lock-cu");
    let fx = fixture();
    let (c, _o, e) = vis(
        &d,
        &["pkg", "add", "util", "--registry", fx.to_str().unwrap()],
    );
    assert_eq!(c, 0, "stderr: {e}");
    let cu = ve_dinh_dang_cu(&doc(&d.join("vis.lock")));
    ghi(&d.join("vis.lock"), &cu);
    let (c, o, e) = vis(&d, &["pkg", "gc"]);
    assert_eq!(c, 0, "stderr: {e}");
    assert!(o.contains("đã xoá"), "stdout: {o}");
    assert_eq!(doc(&d.join("vis.lock")), cu, "gc không được sửa lock cũ");
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 5 — thiếu `vis.lock` ⇒ `migrate` lỗi rõ; lock hỏng ⇒ lỗi, **không** ghi đè.
#[test]
fn migrate_thieu_hoac_hong_lock() {
    let d = tmp_dir("migrate-loi");
    let (c, _o, e) = vis(&d, &["pkg", "migrate"]);
    assert_eq!(c, 2, "stderr: {e}");
    assert!(e.contains("vis.lock"), "{e}");
    ghi(&d.join("vis.lock"), "{khong-phai-json");
    let (c, _o, e) = vis(&d, &["pkg", "migrate"]);
    assert_eq!(c, 2, "stderr: {e}");
    assert!(e.contains("hỏng"), "{e}");
    assert_eq!(doc(&d.join("vis.lock")), "{khong-phai-json");
    let _ = std::fs::remove_dir_all(&d);
}
