// SPDX-License-Identifier: MIT OR Apache-2.0
//! `vis.lock` định dạng 2: ràng buộc hiệu lực, kiểm "còn thoả", dựng + ghi lock tất định
//! (**ST-129**, **VD-125**) — tách khỏi `pkg.rs` ở `ST-317b`.

use std::collections::BTreeMap;
use std::path::Path;

use super::semver::{Version, VersionReq};
use vis_core::json::json;
use vis_core::value::Value;

use super::goi::GoiCai;
use super::{so_phien_ban, GoiKhoa, Khoa, TepKhoa, LOCK_PHIEN_BAN};

pub(crate) fn gia_tri_khoa(khoa: &Khoa) -> Value {
    let ps: Vec<Value> = khoa
        .goi
        .iter()
        .map(|p| {
            let fs: Vec<Value> = p
                .files
                .iter()
                .map(|f| {
                    Value::map(vec![
                        (Value::Str("path".into()), Value::Str(f.path.clone())),
                        (Value::Str("sha256".into()), Value::Str(f.sha256.clone())),
                    ])
                })
                .collect();
            let mut kv = vec![
                (Value::Str("name".into()), Value::Str(p.name.clone())),
                (Value::Str("version".into()), Value::Str(p.version.clone())),
            ];
            // **ST-129 (VD-125)**: luôn ghi `version_req` (định dạng 2).
            let req = p
                .version_req
                .clone()
                .unwrap_or_else(|| format!("={}", p.version));
            kv.push((Value::Str("version_req".into()), Value::Str(req)));
            if let Some(e) = &p.entry {
                kv.push((Value::Str("entry".into()), Value::Str(e.clone())));
            }
            kv.push((Value::Str("sha256".into()), Value::Str(p.sha256.clone())));
            kv.push((Value::Str("files".into()), Value::list(fs)));
            Value::map(kv)
        })
        .collect();
    let rm: Vec<(Value, Value)> = khoa
        .roots
        .iter()
        .map(|(k, v)| (Value::Str(k.clone()), Value::Str(v.clone())))
        .collect();
    Value::map(vec![
        (
            Value::Str("lock".into()),
            Value::Int(i64::from(LOCK_PHIEN_BAN)),
        ),
        (Value::Str("roots".into()), Value::map(rm)),
        (Value::Str("packages".into()), Value::list(ps)),
    ])
}

pub(crate) fn ghi_atomic(p: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d)
            .map_err(|e| format!("không tạo được thư mục `{}`: {e}", d.display()))?;
    }
    let tmp = p.with_extension("tmp");
    std::fs::write(&tmp, bytes).map_err(|e| format!("lỗi ghi `{}`: {e}", tmp.display()))?;
    std::fs::rename(&tmp, p).map_err(|e| format!("lỗi đổi tên `{}`: {e}", p.display()))
}

pub(crate) fn ghi_lock(p: &Path, khoa: &Khoa) -> Result<(), String> {
    let mut s = String::new();
    // `false` = **không** sắp lại khoá: thứ tự khoá **cố định** (lock/roots/packages, rồi từng gói)
    // ⇒ ghi hai lần cho **cùng byte** (VD-125).
    json(&gia_tri_khoa(khoa), false, &mut s);
    s.push('\n');
    ghi_atomic(p, s.as_bytes())
}
/// Ràng buộc **hiệu lực** của từng gói trong một lời giải = **AND** của ràng buộc gốc và mọi ràng
/// buộc dep trỏ tới nó. In ra **chuẩn hoá** (bỏ trùng, sắp theo chuỗi) ⇒ tất định, ghi được vào
/// `version_req` của lock (ST-129, VD-125).
pub(crate) fn rang_buoc_hieu_luc(
    roots: &BTreeMap<String, VersionReq>,
    ds: &[GoiCai],
) -> BTreeMap<String, String> {
    let mut m: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (ten, req) in roots {
        m.entry(ten.clone()).or_default().push(req.chuan_hoa());
    }
    for g in ds {
        for (dn, dreq) in &g.deps {
            if let Ok(r) = VersionReq::parse(dreq) {
                m.entry(dn.clone()).or_default().push(r.chuan_hoa());
            }
        }
    }
    m.into_iter()
        .map(|(k, mut v)| {
            v.sort();
            v.dedup();
            (k, v.join(", "))
        })
        .collect()
}

/// `version` trong lock có thoả `req` không — **lỏng** với lock cũ (thiếu thành phần ⇒ điền 0).
pub(crate) fn khoa_thoa(version: &str, req: &VersionReq) -> bool {
    let Ok(ds) = so_phien_ban(version) else {
        return false;
    };
    if ds.len() > 3 {
        return false;
    }
    let mut v = [0u64; 3];
    for (i, x) in ds.iter().enumerate() {
        v[i] = *x;
    }
    req.khop(&Version::moi(v[0], v[1], v[2]))
}

/// Ràng buộc **gốc** suy từ lock: dùng `roots` nếu có; lock cũ ⇒ **ghim** `=<version>` từng gói
/// (bảo toàn đúng phiên bản đang có — VD-125: "còn thoả thì giữ nguyên").
pub(crate) fn roots_tu_lock(khoa: &Khoa) -> Result<BTreeMap<String, VersionReq>, String> {
    let mut out = BTreeMap::new();
    if !khoa.roots.is_empty() {
        for (ten, req) in &khoa.roots {
            let r = VersionReq::parse(req)
                .map_err(|e| format!("`vis.lock` hỏng: `roots.{ten}`: {e}"))?;
            out.insert(ten.clone(), r);
        }
        return Ok(out);
    }
    for g in &khoa.goi {
        let r = VersionReq::parse(&format!("={}", g.version))
            .map_err(|e| format!("`vis.lock` hỏng: version `{}`: {e}", g.version))?;
        out.insert(g.name.clone(), r);
    }
    Ok(out)
}

/// Kiểm **toàn bộ** lock còn thoả: mỗi gói thoả `version_req` của chính nó và mỗi root có gói + thoả.
pub(crate) fn lock_con_thoa(khoa: &Khoa) -> Result<(), String> {
    for g in &khoa.goi {
        if let Some(req_s) = &g.version_req {
            let req = VersionReq::parse(req_s)
                .map_err(|e| format!("`vis.lock` hỏng: `version_req` của `{}`: {e}", g.name))?;
            if !khoa_thoa(&g.version, &req) {
                return Err(format!(
                    "gói `{}@{}` không còn thoả ràng buộc `{req_s}` — chạy `vis pkg update`",
                    g.name, g.version
                ));
            }
        }
    }
    for (ten, req_s) in &khoa.roots {
        let req =
            VersionReq::parse(req_s).map_err(|e| format!("`vis.lock` hỏng: `roots.{ten}`: {e}"))?;
        let Some(g) = khoa.goi.iter().find(|g| &g.name == ten) else {
            return Err(format!(
                "`vis.lock` thiếu gói gốc `{ten}` (ràng buộc `{req_s}`) — chạy `vis pkg update`"
            ));
        };
        if !khoa_thoa(&g.version, &req) {
            return Err(format!(
                "gói gốc `{ten}@{version}` không còn thoả ràng buộc `{req_s}` — chạy `vis pkg update`",
                version = g.version
            ));
        }
    }
    Ok(())
}

/// Dựng [`Khoa`] định dạng **hiện tại** từ lời giải (đã có `GoiCai` + ràng buộc hiệu lực).
pub(crate) fn tao_khoa(
    roots: &BTreeMap<String, VersionReq>,
    ds: &[GoiCai],
    hieu_luc: &BTreeMap<String, String>,
) -> Khoa {
    let mut goi: Vec<GoiKhoa> = ds
        .iter()
        .map(|g| GoiKhoa {
            name: g.name.clone(),
            version: g.version.clone(),
            sha256: g.sha256.clone(),
            entry: Some(g.entry.clone()),
            files: g
                .files
                .iter()
                .map(|f| TepKhoa {
                    path: f.path.clone(),
                    sha256: f.sha256.clone(),
                })
                .collect(),
            version_req: Some(
                hieu_luc
                    .get(&g.name)
                    .cloned()
                    .unwrap_or_else(|| format!("={}", g.version)),
            ),
        })
        .collect();
    goi.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
    Khoa {
        phien_ban: LOCK_PHIEN_BAN,
        roots: roots
            .iter()
            .map(|(k, v)| (k.clone(), v.chuan_hoa()))
            .collect(),
        goi,
    }
}
