// SPDX-License-Identifier: MIT OR Apache-2.0
//! Đọc nội dung gói từ registry cục bộ, giải đồ thị và ghi cây `.vis` (**ST-70**, **ST-128**,
//! **VS3-5a**) — tách khỏi `pkg.rs` ở `ST-317b`.

use std::collections::BTreeMap;
use std::path::Path;

use super::semver::{Version, VersionReq};

use super::khoa::ghi_atomic;
use super::manifest::{bam_cay, doc_manifest};
use super::{bam, chuan_hoa, resolver};

/// Đổi [`resolver::LoiXungDot`] thành thông điệp CLI có **mã `VS0704`** + **chuỗi ràng buộc**.
pub(crate) fn loi_xung_dot(e: &resolver::LoiXungDot) -> String {
    // `vis_diag::code` = `VS` + 4 chữ số (hàm có sẵn của sổ mã) — không viết literal mã ở đây.
    format!("{}: {e}", vis_diag::code(704))
}

pub(crate) struct TepCai {
    pub(crate) path: String,
    pub(crate) sha256: String,
    pub(crate) noi_dung: String,
}

pub(crate) struct GoiCai {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) entry: String,
    pub(crate) sha256: String,
    pub(crate) files: Vec<TepCai>,
    /// **ST-128**: deps dạng **ràng buộc** (đường `pkg update`).
    pub(crate) deps: Vec<(String, String)>,
}

/// Đọc **một** gói cụ thể trong registry ⇒ [`GoiCai`] (kiểm symlink, chuẩn hoá, băm tệp, `tree_sha`).
pub(crate) fn doc_goi(
    reg: &Path,
    name: &str,
    version: &str,
    entry_override: Option<&str>,
) -> Result<GoiCai, String> {
    let dir = reg.join(name).join(version);
    let m = doc_manifest(&dir.join("manifest.json"), entry_override)?;
    if m.name != name || m.version != version {
        return Err(format!(
            "manifest `{}` khai `{}@{}` nhưng thư mục là `{name}@{version}`",
            dir.display(),
            m.name,
            m.version
        ));
    }
    let mut files = Vec::new();
    for f in &m.files {
        let full = dir.join(&f.path);
        let meta = std::fs::symlink_metadata(&full)
            .map_err(|e| format!("không thấy `{}`: {e}", full.display()))?;
        if meta.file_type().is_symlink() {
            return Err(format!("`{}` là symlink — không nhận", full.display()));
        }
        if !meta.file_type().is_file() {
            return Err(format!("`{}` không phải tệp thường", full.display()));
        }
        let bytes =
            std::fs::read(&full).map_err(|e| format!("lỗi đọc `{}`: {e}", full.display()))?;
        let noi_dung = chuan_hoa(&bytes).map_err(|e| format!("`{}`: {e}", full.display()))?;
        let sha = bam(&noi_dung);
        if sha != f.sha256 {
            return Err(format!(
                "`{name}@{version}`: băm `{}` lệch — manifest `{}`, nội dung thật `{sha}`",
                f.path, f.sha256
            ));
        }
        files.push(TepCai {
            path: f.path.clone(),
            sha256: sha,
            noi_dung,
        });
    }
    files.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
    let cap: Vec<(String, String)> = files
        .iter()
        .map(|f| (f.path.clone(), f.sha256.clone()))
        .collect();
    let sha256 = bam_cay(&cap);
    Ok(GoiCai {
        name: m.name.clone(),
        version: m.version.clone(),
        entry: m.entry.clone(),
        sha256,
        files,
        deps: m.deps.clone(),
    })
}

/// Giải **đồ thị** từ tập ràng buộc gốc (chưa đọc nội dung gói).
///
/// **VS3-5a** tách khỏi [`doc_tat_ca`] để registry HTTP có thể giải trước, **chỉ tải tarball của
/// phiên bản đã chọn**, rồi mới đọc nội dung — cùng resolver, cùng ngữ nghĩa với registry cục bộ.
pub(crate) fn giai_roots(
    reg: &Path,
    roots: &BTreeMap<String, VersionReq>,
    entry_override: Option<(&str, &str)>,
    uu_tien: &BTreeMap<String, Version>,
) -> Result<BTreeMap<String, Version>, String> {
    let ds_goc: Vec<(String, VersionReq)> =
        roots.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    let mut bo = resolver::Resolver::moi(reg).voi_uu_tien(uu_tien.clone());
    if let Some((ten, e)) = entry_override {
        bo = bo.voi_entry_goc(ten, e);
    }
    bo.giai(&ds_goc).map_err(|e| loi_xung_dot(&e))
}

/// Đọc nội dung các gói trong **lời giải** (manifest + tệp + `tree_sha`) từ registry cục bộ.
pub(crate) fn doc_tat_ca(
    reg: &Path,
    giai: &BTreeMap<String, Version>,
    entry_override: Option<(&str, &str)>,
) -> Result<Vec<GoiCai>, String> {
    // `--entry` chỉ áp cho gói gốc được gọi; lời giải trả về **toàn bộ** gói.
    let mut ds = Vec::with_capacity(giai.len());
    for (ten, ver) in giai {
        let eo = match entry_override {
            Some((t, e)) if t == ten.as_str() => Some(e),
            _ => None,
        };
        ds.push(doc_goi(reg, ten, &ver.to_string(), eo)?);
    }
    ds.sort_by(|a, b| a.name.cmp(&b.name).then(a.version.cmp(&b.version)));
    Ok(ds)
}

/// Giải **cả đồ thị** từ tập ràng buộc gốc, rồi đọc nội dung từng gói đã chọn.
pub(crate) fn giai_va_doc(
    reg: &Path,
    roots: &BTreeMap<String, VersionReq>,
    entry_override: Option<(&str, &str)>,
    uu_tien: &BTreeMap<String, Version>,
) -> Result<(BTreeMap<String, Version>, Vec<GoiCai>), String> {
    let giai = giai_roots(reg, roots, entry_override, uu_tien)?;
    let ds = doc_tat_ca(reg, &giai, entry_override)?;
    Ok((giai, ds))
}

/// Ghi blob + cây `.vis/pkg` cho danh sách gói (dùng chung `add`/`update`/`install`).
pub(crate) fn ghi_cay(cwd: &Path, ds: &[GoiCai]) -> Result<(), String> {
    for g in ds {
        for f in &g.files {
            ghi_atomic(
                &cwd.join(".vis/store").join(&f.sha256),
                f.noi_dung.as_bytes(),
            )?;
        }
    }
    for g in ds {
        let dich = cwd.join(".vis/pkg").join(&g.name).join(&g.version);
        let _ = std::fs::remove_dir_all(&dich);
        for f in &g.files {
            let p = dich.join(&f.path);
            if let Some(d) = p.parent() {
                std::fs::create_dir_all(d)
                    .map_err(|e| format!("không tạo được `{}`: {e}", d.display()))?;
            }
            std::fs::write(&p, f.noi_dung.as_bytes())
                .map_err(|e| format!("lỗi ghi `{}`: {e}", p.display()))?;
        }
    }
    Ok(())
}
