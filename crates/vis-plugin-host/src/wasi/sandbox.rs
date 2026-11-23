// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-307/ST-310c (VD-285)** — chính sách quyền và sandbox preopen của plugin WASI.
//!
//! - [`ChinhSach`] tắt cứng plugin động khi `--strict-caps`/`--sandbox` — kiểm **trước** khi
//!   đọc bất kỳ tệp nào (`VS0604`).
//! - Cap `write = false` được preopen qua **bản sao tạm** hạ quyền (`r-x`/`r--`) vì WASI
//!   Preview 1 không có preopen chỉ đọc; cap `write = true` preopen thẳng thư mục gốc.
//! - Bản sao bỏ qua symlink, có trần tệp/byte/độ sâu; mọi lỗi là `Error` có mã, không panic.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use vis_plugin_abi::{Cap, Error, Result};

/// Chính sách quyền của phiên chạy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChinhSach {
    /// `--strict-caps`: plugin động **tắt cứng**.
    pub strict_caps: bool,
    /// `--sandbox`: plugin động **tắt cứng**.
    pub sandbox: bool,
}

impl ChinhSach {
    /// Chính sách mặc định (cho phép plugin động, vẫn chỉ trong `cap`).
    pub fn moi() -> Self {
        Self::default()
    }

    /// Dựng từ hai cờ CLI.
    pub fn tu_co(strict_caps: bool, sandbox: bool) -> Self {
        Self {
            strict_caps,
            sandbox,
        }
    }

    /// `true` nếu plugin động bị tắt.
    pub fn tat_plugin_dong(&self) -> bool {
        self.strict_caps || self.sandbox
    }

    /// Chặn sớm, **trước** mọi truy cập tệp.
    pub fn kiem_tra(&self) -> Result<()> {
        if self.strict_caps {
            return Err(Error::new(
                "VS0604",
                "plugin động bị tắt cứng khi bật `--strict-caps` (VD-278/VD-283)",
            ));
        }
        if self.sandbox {
            return Err(Error::new(
                "VS0604",
                "plugin động bị tắt cứng khi bật `--sandbox` (VD-278/VD-283)",
            ));
        }
        Ok(())
    }
}

/// Chuẩn hoá cap và dựng **bản sao chỉ-đọc** cho cap `write = false`.
pub(crate) fn chuan_hoa_caps(caps: Vec<Cap>) -> Result<(Vec<Cap>, Vec<PathBuf>, Vec<PathBuf>)> {
    let mut ra = Vec::with_capacity(caps.len());
    let mut preopens = Vec::with_capacity(caps.len());
    let mut tam = Vec::new();
    for (i, c) in caps.into_iter().enumerate() {
        if c.kind != "Dir" {
            don_tam(&mut tam);
            return Err(Error::new(
                "VS0604",
                format!(
                    "plugin WASI chỉ nhận `cap[Dir]` ở bản này (nhận `cap[{}]`)",
                    c.kind
                ),
            ));
        }
        let root = match fs::canonicalize(&c.root) {
            Ok(r) => r,
            Err(e) => {
                don_tam(&mut tam);
                return Err(Error::io(format!(
                    "không canonicalize được cap `{}`: {e}",
                    c.root.display()
                )));
            }
        };
        if !root.is_dir() {
            don_tam(&mut tam);
            return Err(Error::new(
                "VS0604",
                format!("cap `{}` không phải thư mục", c.root.display()),
            ));
        }
        if c.write {
            preopens.push(root.clone());
            ra.push(Cap {
                kind: c.kind,
                root,
                write: true,
            });
            continue;
        }
        let dich = std::env::temp_dir().join(format!(
            "vis-plugin-ro-{}-{i}-{}.d",
            std::process::id(),
            DEM.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dich);
        let mut dem = DemRo::default();
        if let Err(e) = sao_chep_ro(&root, &dich, &mut dem) {
            let _ = fs::remove_dir_all(&dich);
            don_tam(&mut tam);
            return Err(e);
        }
        if let Err(e) = ha_quyen_ro(&dich) {
            let _ = fs::remove_dir_all(&dich);
            don_tam(&mut tam);
            return Err(e);
        }
        preopens.push(dich.clone());
        tam.push(dich);
        ra.push(Cap {
            kind: c.kind,
            root,
            write: false,
        });
    }
    Ok((ra, preopens, tam))
}

/// Giữ bản sao chỉ-đọc trong lúc `nap` chạy; lỗi giữa chừng vẫn dọn sạch.
pub(crate) struct GiuTam(pub(crate) Vec<PathBuf>);

impl Drop for GiuTam {
    fn drop(&mut self) {
        don_tam(&mut self.0);
    }
}

/// Dọn bản sao chỉ-đọc (mở quyền rồi xoá).
pub(crate) fn don_tam(ds: &mut Vec<PathBuf>) {
    for d in ds.drain(..) {
        let _ = khoi_phuc_quyen(&d);
        let _ = fs::remove_dir_all(&d);
    }
}

#[derive(Default)]
struct DemRo {
    files: usize,
    bytes: u64,
}

const MAX_RO_FILES: usize = 4096;
const MAX_RO_BYTES: u64 = 64 * 1024 * 1024;
const MAX_RO_SAU: usize = 32;

fn sao_chep_ro(goc: &Path, dich: &Path, dem: &mut DemRo) -> Result<()> {
    fs::create_dir_all(dich).map_err(|e| {
        Error::io(format!(
            "không tạo được bản sao chỉ đọc `{}`: {e}",
            dich.display()
        ))
    })?;
    sao_chep_vao(goc, dich, dem, 0)
}

fn sao_chep_vao(goc: &Path, dich: &Path, dem: &mut DemRo, sau: usize) -> Result<()> {
    if sau > MAX_RO_SAU {
        return Err(Error::new(
            "VS0604",
            format!("cap chỉ đọc `{}` quá sâu (> {MAX_RO_SAU})", goc.display()),
        ));
    }
    let ds = fs::read_dir(goc).map_err(|e| {
        Error::io(format!(
            "không đọc được cap chỉ đọc `{}`: {e}",
            goc.display()
        ))
    })?;
    for muc in ds {
        let muc = muc.map_err(|e| Error::io(format!("lỗi đọc cap chỉ đọc: {e}")))?;
        let loai = muc
            .file_type()
            .map_err(|e| Error::io(format!("lỗi kiểu tệp cap chỉ đọc: {e}")))?;
        if loai.is_symlink() {
            // Không đi theo symlink: bản sao không được phép trỏ ra ngoài root.
            continue;
        }
        let dich_con = dich.join(muc.file_name());
        if loai.is_dir() {
            fs::create_dir_all(&dich_con)
                .map_err(|e| Error::io(format!("không tạo được `{}`: {e}", dich_con.display())))?;
            sao_chep_vao(&muc.path(), &dich_con, dem, sau + 1)?;
        } else if loai.is_file() {
            let meta = muc
                .metadata()
                .map_err(|e| Error::io(format!("lỗi metadata cap chỉ đọc: {e}")))?;
            dem.files += 1;
            dem.bytes = dem.bytes.saturating_add(meta.len());
            if dem.files > MAX_RO_FILES || dem.bytes > MAX_RO_BYTES {
                return Err(Error::new(
                    "VS0604",
                    format!(
                        "cap chỉ đọc `{}` quá lớn để sao chép (>{MAX_RO_FILES} tệp hoặc >{MAX_RO_BYTES} byte)",
                        goc.display()
                    ),
                ));
            }
            fs::copy(muc.path(), &dich_con).map_err(|e| {
                Error::io(format!(
                    "không sao chép được `{}`: {e}",
                    muc.path().display()
                ))
            })?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn ha_quyen_ro(d: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    for muc in fs::read_dir(d).map_err(|e| Error::io(format!("lỗi đọc `{}`: {e}", d.display())))?
    {
        let muc = muc.map_err(|e| Error::io(format!("lỗi đọc `{}`: {e}", d.display())))?;
        let p = muc.path();
        let loai = muc
            .file_type()
            .map_err(|e| Error::io(format!("lỗi kiểu tệp `{}`: {e}", p.display())))?;
        if loai.is_dir() {
            ha_quyen_ro(&p)?;
            fs::set_permissions(&p, fs::Permissions::from_mode(0o555))
                .map_err(|e| Error::io(format!("không hạ quyền `{}`: {e}", p.display())))?;
        } else {
            fs::set_permissions(&p, fs::Permissions::from_mode(0o444))
                .map_err(|e| Error::io(format!("không hạ quyền `{}`: {e}", p.display())))?;
        }
    }
    fs::set_permissions(d, fs::Permissions::from_mode(0o555))
        .map_err(|e| Error::io(format!("không hạ quyền `{}`: {e}", d.display())))?;
    Ok(())
}

#[cfg(not(unix))]
fn ha_quyen_ro(d: &Path) -> Result<()> {
    for muc in fs::read_dir(d).map_err(|e| Error::io(format!("lỗi đọc `{}`: {e}", d.display())))?
    {
        let muc = muc.map_err(|e| Error::io(format!("lỗi đọc `{}`: {e}", d.display())))?;
        let p = muc.path();
        if muc.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            ha_quyen_ro(&p)?;
        }
        let mut q = fs::metadata(&p)
            .map_err(|e| Error::io(format!("lỗi metadata `{}`: {e}", p.display())))?
            .permissions();
        q.set_readonly(true);
        fs::set_permissions(&p, q)
            .map_err(|e| Error::io(format!("không hạ quyền `{}`: {e}", p.display())))?;
    }
    Ok(())
}

#[cfg(unix)]
fn khoi_phuc_quyen(d: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    if !d.exists() {
        return Ok(());
    }
    fs::set_permissions(d, fs::Permissions::from_mode(0o755))
        .map_err(|e| Error::io(format!("không mở quyền `{}`: {e}", d.display())))?;
    for muc in fs::read_dir(d).map_err(|e| Error::io(format!("lỗi đọc `{}`: {e}", d.display())))?
    {
        let muc = muc.map_err(|e| Error::io(format!("lỗi đọc `{}`: {e}", d.display())))?;
        let p = muc.path();
        if muc.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            khoi_phuc_quyen(&p)?;
        } else {
            let _ = fs::set_permissions(&p, fs::Permissions::from_mode(0o644));
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn khoi_phuc_quyen(d: &Path) -> Result<()> {
    if !d.exists() {
        return Ok(());
    }
    for muc in fs::read_dir(d).map_err(|e| Error::io(format!("lỗi đọc `{}`: {e}", d.display())))?
    {
        let muc = muc.map_err(|e| Error::io(format!("lỗi đọc `{}`: {e}", d.display())))?;
        let p = muc.path();
        if muc.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            khoi_phuc_quyen(&p)?;
        } else if let Ok(meta) = fs::metadata(&p) {
            let mut q = meta.permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            q.set_readonly(false);
            let _ = fs::set_permissions(&p, q);
        }
    }
    Ok(())
}

static DEM: AtomicU64 = AtomicU64::new(0);
