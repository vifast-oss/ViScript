// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-5b/5c (VD-271)** — dọn `.vis/store` + `.vis/pkg` của **một workspace** theo hợp các
//! tham chiếu từ nhiều `vis.lock` (lệnh gọi gom union; xem `lenh_gc` trong `pkg.rs`).
//!
//! Chỉ xoá:
//! - blob `.vis/store/<64-hex>` **không** được bất kỳ tệp nào trong **bất kỳ** lock nào tham chiếu;
//! - thư mục `.vis/pkg/<tên>/<phiên bản>` không có mục tương ứng trong **bất kỳ** lock nào.
//!
//! **Không** xoá blob/thư mục đang được lock dùng; **không** đụng tệp không phải hex trong store;
//! không theo symlink khi xoá. `.vis`/`.vis/store`/`.vis/pkg` là symlink ⇒ **từ chối trước khi xoá**
//! (không xoá ngoài store). Hàm **idempotent**: chạy lại khi đã sạch ⇒ 0/0, cây không đổi.
//! Tên gói chứa `/` (mô hình đường dẫn tương lai) bị từ chối ở lệnh gọi — an toàn hơn là đoán.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Kiểm **trước** mọi thao tác xoá: `.vis`, `.vis/store`, `.vis/pkg` phải là thư mục thật, không
/// symlink. Trả lỗi ⇒ lệnh gọi **không được** xoá gì (kể cả workspace khác đã xử lý trước đó).
pub(super) fn kiem_truoc(workspace: &Path) -> Result<(), String> {
    for rel in [".vis", ".vis/store", ".vis/pkg"] {
        let p = workspace.join(rel);
        match std::fs::symlink_metadata(&p) {
            Ok(md) => {
                if md.file_type().is_symlink() {
                    return Err(format!(
                        "`{}` là symlink — từ chối GC (không xoá ngoài store)",
                        p.display()
                    ));
                }
                if !md.is_dir() {
                    return Err(format!("`{}` không phải thư mục — từ chối GC", p.display()));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("không đọc được metadata `{}`: {e}", p.display())),
        }
    }
    Ok(())
}

/// Trả `(số blob đã/sẽ xoá, số mục gói đã/sẽ xoá)`. `dry` ⇒ chỉ đếm, không chạm đĩa.
pub(super) fn don(
    workspace: &Path,
    blobs: &BTreeSet<String>,
    goi: &BTreeSet<(String, String)>,
    dry: bool,
) -> Result<(usize, usize), String> {
    kiem_truoc(workspace)?;
    let mut so_blob = 0usize;
    let store = workspace.join(".vis/store");
    if store.is_dir() {
        let mut ten = ten_trong(&store)?;
        ten.sort();
        for name in ten {
            if !la_hex64(&name) || blobs.contains(&name) {
                continue;
            }
            let p = store.join(&name);
            let md = std::fs::symlink_metadata(&p)
                .map_err(|e| format!("không đọc được metadata `{}`: {e}", p.display()))?;
            // Store là content-addressed **tệp**; thư mục cùng tên không phải blob ⇒ để yên.
            if md.is_dir() && !md.file_type().is_symlink() {
                continue;
            }
            if !dry {
                xoa(&p)?;
            }
            so_blob += 1;
        }
    }

    let mut so_goi = 0usize;
    let pkg = workspace.join(".vis/pkg");
    if pkg.is_dir() {
        let mut ten = ten_trong(&pkg)?;
        ten.sort();
        for name in ten {
            let p = pkg.join(&name);
            let md = std::fs::symlink_metadata(&p)
                .map_err(|e| format!("không đọc được metadata `{}`: {e}", p.display()))?;
            if md.file_type().is_symlink() {
                // Symlink ở cấp tên gói: chỉ xoá **link** khi không còn gói nào tên đó trong lock.
                if !goi.iter().any(|(n, _)| n == &name) {
                    if !dry {
                        xoa(&p)?;
                    }
                    so_goi += 1;
                }
                continue;
            }
            if !md.is_dir() {
                continue;
            }
            let mut ban = ten_trong(&p)?;
            ban.sort();
            for ver in ban {
                let q = p.join(&ver);
                if goi.contains(&(name.clone(), ver.clone())) {
                    continue;
                }
                let mdq = std::fs::symlink_metadata(&q)
                    .map_err(|e| format!("không đọc được metadata `{}`: {e}", q.display()))?;
                if !mdq.is_dir() && !mdq.file_type().is_symlink() {
                    continue; // tệp lạ trong thư mục gói: không phải phiên bản
                }
                if !dry {
                    xoa(&q)?;
                }
                so_goi += 1;
            }
            // Dọn thư mục tên gói rỗng sau khi bỏ hết phiên bản (không tính vào `so_goi`).
            if !dry {
                if let Ok(mut rd) = std::fs::read_dir(&p) {
                    if rd.next().is_none() {
                        let _ = std::fs::remove_dir(&p);
                    }
                }
            }
        }
    }
    Ok((so_blob, so_goi))
}

/// **VS3-5c**: `vis pkg gc [--dry-run] [--lock <path>]... [--root <dir>]...`.
///
/// Hợp tham chiếu của **mọi** lock (tệp `--lock` + `vis.lock` quét dưới `--root`, có kiểm soát);
/// chỉ xoá blob/mục gói không được **bất kỳ** lock nào tham chiếu. Không có lock ⇒ từ chối.
///
/// **Ngữ nghĩa `--root` (khoá ở ST-288)**:
/// - lặp lại được; mỗi root **quét đệ quy** tìm `vis.lock` (bỏ `.git`/`.vis`/`target`/
///   `node_modules`, không theo symlink, giới hạn độ sâu/số thư mục — quá giới hạn ⇒ lỗi, không
///   bỏ qua im lặng vì có thể bỏ sót lock đang dùng blob);
/// - **workspace được dọn** = chính các root (đã canonical hoá) **+** thư mục chứa từng lock; trùng
///   nhau (root lồng nhau, hai root cùng trỏ một cây) **chỉ tính một lần**;
/// - root **không có** `.vis`/store là **no-op**, không tạo thư mục; mọi tham chiếu được hợp trước
///   khi xoá; kiểm symlink **toàn bộ** workspace trước khi xoá mục đầu tiên;
/// - `--root` không tìm thấy `vis.lock` nào ⇒ lỗi (không dọn mù).
pub(super) fn lenh(args: &[String]) -> Result<(), String> {
    let mut dry = false;
    let mut lock_args: Vec<PathBuf> = Vec::new();
    let mut root_args: Vec<PathBuf> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--dry-run" => dry = true,
            "--lock" => {
                i += 1;
                let Some(v) = args.get(i) else {
                    return Err("thiếu giá trị sau `--lock`".to_string());
                };
                lock_args.push(PathBuf::from(v));
            }
            "--root" => {
                i += 1;
                let Some(v) = args.get(i) else {
                    return Err("thiếu giá trị sau `--root`".to_string());
                };
                root_args.push(PathBuf::from(v));
            }
            a if a.starts_with('-') => return Err(format!("cờ lạ `{a}` — {}", super::dung())),
            a => return Err(format!("thừa đối số `{a}` — {}", super::dung())),
        }
        i += 1;
    }

    let cwd = super::cwd()?;
    let mut lock_files: Vec<PathBuf> = Vec::new();
    let mut workspaces: BTreeSet<PathBuf> = BTreeSet::new();

    if lock_args.is_empty() && root_args.is_empty() {
        // Tương thích hành vi cũ: một `vis.lock` trong cwd, dọn store cwd.
        let p = cwd.join("vis.lock");
        kiem_lock_tep(&p)?;
        lock_files.push(p);
    } else {
        for l in lock_args {
            let p = neu_tuong_doi(&cwd, l);
            kiem_lock_tep(&p)?;
            lock_files.push(p);
        }
        for r in root_args {
            let root = neu_tuong_doi(&cwd, r);
            let found = quet_lock(&root)?;
            if found.is_empty() {
                return Err(format!(
                    "không tìm thấy `vis.lock` dưới `{}` — không dọn mù",
                    root.display()
                ));
            }
            let root = std::fs::canonicalize(&root)
                .map_err(|e| format!("không đọc được `{}`: {e}", root.display()))?;
            workspaces.insert(root);
            lock_files.extend(found);
        }
    }

    // Canonical hoá để dedup; lock cuối là symlink đã bị từ chối ở `kiem_lock_tep`/`quet_lock`.
    let mut lock_paths: BTreeSet<PathBuf> = BTreeSet::new();
    for p in lock_files {
        let canon = std::fs::canonicalize(&p)
            .map_err(|e| format!("không đọc được `{}`: {e}", p.display()))?;
        lock_paths.insert(canon);
    }
    if lock_paths.is_empty() {
        return Err("chưa có `vis.lock` — không dọn store để tránh xoá nhầm".to_string());
    }

    let mut blobs: BTreeSet<String> = BTreeSet::new();
    let mut goi: BTreeSet<(String, String)> = BTreeSet::new();
    for p in &lock_paths {
        let khoa = super::doc_lock(p, true)?;
        // Tên gói chứa `/` (mô hình đường dẫn tương lai) làm việc phân biệt tên gói/phiên bản
        // không an toàn ⇒ từ chối, **không xoá gì**.
        if khoa.goi.iter().any(|g| g.name.contains('/')) {
            return Err("GC chưa hỗ trợ tên gói chứa `/` — không xoá gì".to_string());
        }
        for g in khoa.goi {
            goi.insert((g.name, g.version));
            for f in g.files {
                blobs.insert(f.sha256);
            }
        }
        if let Some(ws) = p.parent() {
            workspaces.insert(ws.to_path_buf());
        }
    }

    // Kiểm **tất cả** workspace trước khi xoá bất cứ thứ gì (symlink ⇒ dừng, không xoá một phần).
    for ws in &workspaces {
        kiem_truoc(ws)?;
    }

    let mut so_blob = 0usize;
    let mut so_goi = 0usize;
    for ws in &workspaces {
        let (b, g) = don(ws, &blobs, &goi, dry)?;
        so_blob += b;
        so_goi += g;
    }
    let phu = if lock_paths.len() == 1 && workspaces.len() == 1 {
        String::new()
    } else {
        format!(
            " [{} lock, {} workspace]",
            lock_paths.len(),
            workspaces.len()
        )
    };
    if dry {
        println!("sẽ xoá {so_blob} blob, {so_goi} mục gói (không xoá gì){phu}");
    } else {
        println!("đã xoá {so_blob} blob, {so_goi} mục gói{phu}");
    }
    Ok(())
}

fn neu_tuong_doi(cwd: &Path, p: PathBuf) -> PathBuf {
    if p.is_absolute() {
        p
    } else {
        cwd.join(p)
    }
}

fn kiem_lock_tep(p: &Path) -> Result<(), String> {
    let md = std::fs::symlink_metadata(p)
        .map_err(|e| format!("không đọc được `{}`: {e}", p.display()))?;
    if md.file_type().is_symlink() {
        return Err(format!(
            "`{}` là symlink — từ chối đọc lock (không theo symlink)",
            p.display()
        ));
    }
    if !md.is_file() {
        return Err(format!("`{}` không phải tệp `vis.lock`", p.display()));
    }
    Ok(())
}

const MAX_DO: usize = 16;
const MAX_LOCK: usize = 512;
const MAX_DIR: usize = 10_000;

/// Quét `vis.lock` dưới `root` **có kiểm soát**: không theo symlink, bỏ `.git`/`.vis`/`target`/
/// `node_modules`, giới hạn độ sâu [`MAX_DO`] và số lock [`MAX_LOCK`]. Tệp `vis.lock` là symlink
/// bị **bỏ qua** (không đọc, không xoá); nếu không còn lock thật ⇒ lệnh gọi báo lỗi.
fn quet_lock(root: &Path) -> Result<Vec<PathBuf>, String> {
    let md = std::fs::symlink_metadata(root)
        .map_err(|e| format!("không đọc được `{}`: {e}", root.display()))?;
    if md.file_type().is_symlink() {
        return Err(format!(
            "`{}` là symlink — từ chối quét (không theo symlink)",
            root.display()
        ));
    }
    if !md.is_dir() {
        return Err(format!("`{}` không phải thư mục", root.display()));
    }
    let mut out = Vec::new();
    let mut so_dir = 0usize;
    quet_lock_trong(root, 0, &mut out, &mut so_dir)?;
    out.sort();
    Ok(out)
}

fn quet_lock_trong(
    dir: &Path,
    do_sau: usize,
    out: &mut Vec<PathBuf>,
    so_dir: &mut usize,
) -> Result<(), String> {
    *so_dir += 1;
    if *so_dir > MAX_DIR {
        return Err(format!(
            "quét quá {MAX_DIR} thư mục — thu hẹp `--root` hoặc dùng `--lock`"
        ));
    }
    if do_sau > MAX_DO {
        // Không bỏ qua im lặng: một lock sâu hơn giới hạn có thể tham chiếu blob trong store
        // đang dọn ⇒ từ chối để không xoá nhầm; người dùng dùng `--lock` tường minh.
        return Err(format!(
            "quét quá sâu (>{} thư mục) dưới `{}` — dùng `--lock` tường minh",
            MAX_DO,
            dir.display()
        ));
    }
    if out.len() > MAX_LOCK {
        return Err(format!("quét quá {MAX_LOCK} `vis.lock` — thu hẹp `--root`"));
    }
    let lock = dir.join("vis.lock");
    match std::fs::symlink_metadata(&lock) {
        Ok(md) => {
            if md.file_type().is_symlink() {
                // Không theo symlink; bỏ qua, không lỗi để quét cây có symlink lạ.
            } else if md.is_file() {
                out.push(lock);
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("không đọc được `{}`: {e}", lock.display())),
    }

    let rd = std::fs::read_dir(dir)
        .map_err(|e| format!("không đọc được thư mục `{}`: {e}", dir.display()))?;
    let mut ds: Vec<std::fs::DirEntry> = rd
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("lỗi đọc `{}`: {e}", dir.display()))?;
    ds.sort_by_key(|e| e.file_name());
    for e in ds {
        let p = e.path();
        let md = std::fs::symlink_metadata(&p)
            .map_err(|e| format!("không đọc được metadata `{}`: {e}", p.display()))?;
        if md.file_type().is_symlink() || !md.is_dir() {
            continue;
        }
        let ten = e.file_name();
        let ten = ten.to_string_lossy();
        if matches!(ten.as_ref(), ".git" | ".vis" | "target" | "node_modules") {
            continue;
        }
        quet_lock_trong(&p, do_sau + 1, out, so_dir)?;
    }
    Ok(())
}

/// Tên các mục trong một thư mục (không đệ quy), chỉ lấy tên UTF-8.
fn ten_trong(dir: &Path) -> Result<Vec<String>, String> {
    let rd = std::fs::read_dir(dir)
        .map_err(|e| format!("không đọc được thư mục `{}`: {e}", dir.display()))?;
    let mut out = Vec::new();
    for it in rd {
        let it = it.map_err(|e| format!("lỗi đọc `{}`: {e}", dir.display()))?;
        if let Some(s) = it.file_name().to_str() {
            out.push(s.to_string());
        }
    }
    Ok(out)
}

fn la_hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Xoá **không theo symlink**: symlink/tệp ⇒ `remove_file`, thư mục thật ⇒ `remove_dir_all`.
fn xoa(p: &Path) -> Result<(), String> {
    let md = std::fs::symlink_metadata(p)
        .map_err(|e| format!("không đọc được metadata `{}`: {e}", p.display()))?;
    let kq = if md.is_dir() && !md.file_type().is_symlink() {
        std::fs::remove_dir_all(p)
    } else {
        std::fs::remove_file(p)
    };
    kq.map_err(|e| format!("không xoá được `{}`: {e}", p.display()))
}

#[cfg(test)]
mod tests {
    use super::don;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    fn tmp(tag: &str) -> PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static DEM: AtomicUsize = AtomicUsize::new(0);
        let n = DEM.fetch_add(1, Ordering::Relaxed);
        let d = std::env::temp_dir().join(format!("vis-gc-{tag}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn ghi(p: &std::path::Path) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, b"x").unwrap();
    }

    #[test]
    fn don_dung_va_idempotent() {
        let d = tmp("unit");
        let giu = "a".repeat(64);
        let bo = "b".repeat(64);
        ghi(&d.join(".vis/store").join(&giu));
        ghi(&d.join(".vis/store").join(&bo));
        ghi(&d.join(".vis/store").join("README"));
        ghi(&d.join(".vis/pkg/http/1.0.0/src/main.vis"));
        ghi(&d.join(".vis/pkg/http/2.0.0/src/main.vis"));
        let blobs: BTreeSet<String> = [giu.clone()].into_iter().collect();
        let goi: BTreeSet<(String, String)> = [("http".to_string(), "2.0.0".to_string())]
            .into_iter()
            .collect();
        // Dry-run: không xoá gì.
        assert_eq!(don(&d, &blobs, &goi, true).unwrap(), (1, 1));
        assert!(d.join(".vis/store").join(&bo).exists());
        assert!(d.join(".vis/pkg/http/1.0.0").exists());
        // Thật: xoá blob lạ + bản cũ; giữ blob được lock dùng, bản đang dùng, README.
        assert_eq!(don(&d, &blobs, &goi, false).unwrap(), (1, 1));
        assert!(d.join(".vis/store").join(&giu).exists());
        assert!(!d.join(".vis/store").join(&bo).exists());
        assert!(d.join(".vis/store").join("README").exists());
        assert!(d.join(".vis/pkg/http/2.0.0/src/main.vis").exists());
        assert!(!d.join(".vis/pkg/http/1.0.0").exists());
        // Lần hai: 0/0, cây không đổi.
        assert_eq!(don(&d, &blobs, &goi, false).unwrap(), (0, 0));
        let _ = std::fs::remove_dir_all(&d);
    }
}
