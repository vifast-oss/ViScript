// SPDX-License-Identifier: MIT OR Apache-2.0
//! Manifest gói + `tree_sha` (**ST-70**, **ST-128**) — tách khỏi `pkg.rs` ở `ST-317b`.

use std::collections::BTreeSet;
use std::path::Path;

use super::semver::VersionReq;
use vis_core::value::Value;

use super::{bam, chuoi, kiem_path, la_hex64, lay, lay_chuoi};

/// `tree_sha` trên danh sách `(path, file_sha)` — sắp theo **byte** của path.
pub(crate) fn bam_cay(files: &[(String, String)]) -> String {
    let mut ds = files.to_vec();
    ds.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    let mut s = String::from("vispkg1\n");
    for (p, h) in ds {
        s.push_str(&p);
        s.push('\n');
        s.push_str(&h);
        s.push('\n');
    }
    bam(&s)
}

// ----- manifest -----

pub(crate) struct TepManifest {
    pub(crate) path: String,
    pub(crate) sha256: String,
}

pub(crate) struct Manifest {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) entry: String,
    pub(crate) deps: Vec<(String, String)>,
    pub(crate) files: Vec<TepManifest>,
}

pub(crate) fn doc_manifest(duong: &Path, entry_override: Option<&str>) -> Result<Manifest, String> {
    let bytes = std::fs::read(duong)
        .map_err(|e| format!("không đọc được manifest `{}`: {e}", duong.display()))?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|e| format!("manifest `{}` không phải UTF-8: {e}", duong.display()))?;
    let v = vis_core::json::decode(text)
        .map_err(|e| format!("manifest `{}` hỏng: {e}", duong.display()))?;
    let nguon = format!("manifest `{}`", duong.display());
    manifest_tu_gia_tri(&v, &nguon, entry_override)
}

/// Validate một manifest từ [`Value`] đã giải mã; `nguon` là mô tả cho chẩn đoán:
/// `manifest \`<đường dẫn>\`` (registry cục bộ) hoặc `index gói \`<tên>@<phiên bản>\`` (HTTP —
/// VS3-5a). Dùng chung để hai nguồn có **cùng** luật chấp nhận/từ chối.
pub(crate) fn manifest_tu_gia_tri(
    v: &Value,
    nguon: &str,
    entry_override: Option<&str>,
) -> Result<Manifest, String> {
    let Value::Map(kv) = v else {
        return Err(format!("{nguon} phải là object"));
    };
    let name = lay_chuoi(kv, "name").ok_or_else(|| format!("{nguon} thiếu trường `name`"))?;
    let version =
        lay_chuoi(kv, "version").ok_or_else(|| format!("{nguon} thiếu trường `version`"))?;
    // **ST-90**: `entry` của manifest vẫn là nguồn chính; `--entry` chỉ **đổi** khi manifest thiếu
    // hoặc người dùng muốn chỉ định khác (vẫn phải nằm trong `files`).
    let entry = lay_chuoi(kv, "entry")
        .or_else(|| entry_override.map(str::to_string))
        .ok_or_else(|| format!("{nguon} thiếu trường `entry`"))?;
    kiem_path(&entry)?;
    let fs_v = lay(kv, "files").ok_or_else(|| format!("{nguon} thiếu trường `files`"))?;
    let Value::List(ds) = fs_v else {
        return Err(format!("{nguon}: `files` phải là mảng"));
    };
    let mut files = Vec::new();
    let mut da = BTreeSet::new();
    for it in ds.iter() {
        let Value::Map(f) = it else {
            return Err(format!("{nguon}: mỗi `files[]` phải là object"));
        };
        let path =
            lay_chuoi(f, "path").ok_or_else(|| format!("{nguon}: `files[]` thiếu `path`"))?;
        kiem_path(&path)?;
        let sha =
            lay_chuoi(f, "sha256").ok_or_else(|| format!("{nguon}: `files[]` thiếu `sha256`"))?;
        if !la_hex64(&sha) {
            return Err(format!("{nguon}: `sha256` của `{path}` phải 64 chữ số hex"));
        }
        if !da.insert(path.clone()) {
            return Err(format!("{nguon}: trùng `files[].path` `{path}`"));
        }
        files.push(TepManifest {
            path,
            sha256: sha.to_ascii_lowercase(),
        });
    }
    if files.is_empty() {
        return Err(format!("{nguon}: `files` rỗng"));
    }
    if !files.iter().any(|f| f.path == entry) {
        return Err(format!(
            "{nguon}: `entry` `{entry}` không nằm trong `files`"
        ));
    }
    let mut deps = Vec::new();
    if let Some(dv) = lay(kv, "deps") {
        let Value::Map(dm) = dv else {
            return Err(format!("{nguon}: `deps` phải là object"));
        };
        for (k, v) in dm.iter() {
            let Some(dn) = chuoi(k) else {
                return Err(format!("{nguon}: tên dep phải là chuỗi"));
            };
            let Some(dver) = chuoi(v) else {
                return Err(format!("{nguon}: version của dep `{dn}` phải là chuỗi"));
            };
            // **ST-128 (VD-125)**: `deps` là **ràng buộc** semver (`^1.2`, `>=1, <2`, `1.0.0`…),
            // không còn bắt buộc là version chính xác. Chẩn đoán nêu đúng tên gói + chuỗi sai.
            VersionReq::parse(dver)
                .map_err(|e| format!("{nguon}: ràng buộc của dep `{dn}` không hợp lệ: {e}"))?;
            deps.push((dn.to_string(), dver.to_string()));
        }
    }
    Ok(Manifest {
        name,
        version,
        entry,
        deps,
        files,
    })
}

#[cfg(test)]
mod tests {
    use super::bam_cay;
    use crate::pkg::{bam, chuan_hoa, semver, so_phien_ban};

    /// CRLF/BOM khác nhau nhưng chuẩn hoá ⇒ **cùng** băm; đổi 1 byte ⇒ đổi băm; ghim 1 hex.
    #[test]
    fn bam_chuan_hoa_crlf_bom() {
        let a = chuan_hoa(b"x").unwrap();
        let b = chuan_hoa(b"x\r\n").unwrap();
        let c = chuan_hoa(b"\xef\xbb\xbfx\r\n").unwrap();
        assert_eq!(a, "x\n");
        assert_eq!(a, b);
        assert_eq!(a, c);
        let fs = bam(&a);
        assert_eq!(
            fs,
            "73cb3858a687a8494ca3323053016282f3dad39d42cf62ca4e79dda2aac7d9ac"
        );
        assert_eq!(
            bam_cay(&[("a.vis".to_string(), fs.clone())]),
            "510fb9cc152028b1f61fe4934b1a7a8555b1d0787cce5e70d590a8b4539ff392"
        );
        let khac = bam(&chuan_hoa(b"y").unwrap());
        assert_ne!(fs, khac);
    }

    /// `so_phien_ban` = parser **lỏng** cho lock cũ (đọc được `1.0`, `1.0.0`), dùng **chỉ** khi đọc
    /// lock/manifest cũ; phép so **thứ tự** và kiểm phiên bản **mới** nay do `semver::Version`
    /// (ST-127/ST-128 — `cmp_phien_ban` đã bỏ, ca so thứ tự ở `semver::tests::version_in_ra_va_thu_tu`
    /// và `resolver::tests::chon_cao_nhat_khi_moi_ban_deu_thoa`).
    #[test]
    fn so_phien_ban_long_cho_lock_cu() {
        assert_eq!(so_phien_ban("1.10.0").unwrap(), vec![1, 10, 0]);
        assert_eq!(so_phien_ban("1.0").unwrap(), vec![1, 0]);
        assert!(so_phien_ban("1.x").is_err());
        assert!(so_phien_ban("").is_err());
        // Tương thích: lock cũ ghi `1.0` vẫn đọc được; lock mới ghi `semver::Version` (3 thành phần).
        assert!(
            semver::Version::parse("1.0").is_err(),
            "Version **chặt** 3 thành phần"
        );
        assert!(semver::Version::parse("1.0.0").is_ok());
    }
}
