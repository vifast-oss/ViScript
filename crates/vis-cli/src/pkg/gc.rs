// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-5b (VD-271)** — dọn `.vis/store` + `.vis/pkg` theo **một** `vis.lock` hiện tại.
//!
//! Chỉ xoá:
//! - blob `.vis/store/<64-hex>` **không** được bất kỳ tệp nào trong lock tham chiếu;
//! - thư mục `.vis/pkg/<tên>/<phiên bản>` không có mục tương ứng trong lock.
//!
//! **Không** xoá blob/thư mục đang được lock dùng; **không** đụng tệp không phải hex trong store;
//! không theo symlink khi xoá. Hàm **idempotent**: chạy lại khi đã sạch ⇒ 0/0, cây không đổi.
//! Tên gói chứa `/` (mô hình đường dẫn tương lai) bị từ chối ở lệnh gọi — an toàn hơn là đoán.

use std::collections::BTreeSet;
use std::path::Path;

/// Trả `(số blob đã/sẽ xoá, số mục gói đã/sẽ xoá)`. `dry` ⇒ chỉ đếm, không chạm đĩa.
pub(super) fn don(
    cwd: &Path,
    blobs: &BTreeSet<String>,
    goi: &BTreeSet<(String, String)>,
    dry: bool,
) -> Result<(usize, usize), String> {
    let mut so_blob = 0usize;
    let store = cwd.join(".vis/store");
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
    let pkg = cwd.join(".vis/pkg");
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
