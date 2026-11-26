// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296b (VD-281/VD-282)** — metadata thuần cho `vis.lock` + băm/chuẩn hoá nội dung.
//!
//! Tách khỏi `vis-cli::pkg` để `vis-loader` (và WASI) dùng được mà không kéo OS/TLS/SQLite.
//! Đây là **chuyển nguyên** `{bam, chuan_hoa, doc_khoa_cua, GoiKhoa}` cùng các helper đọc lock;
//! `vis-cli::pkg` re-export lại nên mọi đường gọi cũ giữ nguyên hành vi.

use std::collections::BTreeMap;
use std::path::{Component, Path};

use vis_core::value::Value;

use crate::semver::VersionReq;

/// Băm hex của một chuỗi (SHA-256 thật, qua `vis-bytecode`).
pub fn bam(s: &str) -> String {
    let h = vis_bytecode::source_hash(s);
    let mut out = String::with_capacity(64);
    for b in h {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// Chuẩn hoá **khi ingest**: bỏ BOM, CRLF→LF, đúng **một** LF cuối; từ chối non-UTF-8.
pub fn chuan_hoa(bytes: &[u8]) -> Result<String, String> {
    let s = std::str::from_utf8(bytes).map_err(|e| format!("tệp không phải UTF-8: {e}"))?;
    let s = s.strip_prefix('\u{feff}').unwrap_or(s);
    let s = s.replace("\r\n", "\n");
    let s = s.trim_end_matches('\n');
    if s.is_empty() {
        Ok(String::new())
    } else {
        Ok(format!("{s}\n"))
    }
}

// ----- JSON tiện ích -----

pub fn lay<'a>(kv: &'a [(Value, Value)], k: &str) -> Option<&'a Value> {
    kv.iter().find_map(|(kk, v)| match kk {
        Value::Str(s) if s == k => Some(v),
        _ => None,
    })
}

pub fn chuoi(v: &Value) -> Option<&str> {
    match v {
        Value::Str(s) => Some(s),
        _ => None,
    }
}

pub fn lay_chuoi(kv: &[(Value, Value)], k: &str) -> Option<String> {
    lay(kv, k).and_then(chuoi).map(str::to_string)
}

pub fn la_hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Đường dẫn tương đối an toàn: chỉ component `Normal`, không `..`/tuyệt đối/`\`.
pub fn kiem_path(p: &str) -> Result<(), String> {
    if p.is_empty() || p.contains('\\') {
        return Err(format!("đường dẫn `{p}` không hợp lệ"));
    }
    let mut co = false;
    for c in Path::new(p).components() {
        match c {
            Component::Normal(_) => co = true,
            _ => return Err(format!("đường dẫn `{p}` phải tương đối, không `..`/gốc")),
        }
    }
    if co {
        Ok(())
    } else {
        Err(format!("đường dẫn `{p}` không hợp lệ"))
    }
}

// ----- phiên bản theo **số** -----

pub fn so_phien_ban(v: &str) -> Result<Vec<u64>, String> {
    if v.is_empty() {
        return Err("version rỗng".to_string());
    }
    let mut ds = Vec::new();
    for p in v.split('.') {
        ds.push(
            p.parse::<u64>()
                .map_err(|_| format!("version `{v}` không phải số (phần `{p}`)"))?,
        );
    }
    Ok(ds)
}

// ----- vis.lock -----

pub struct TepKhoa {
    pub path: String,
    pub sha256: String,
}

pub struct GoiKhoa {
    pub name: String,
    pub version: String,
    pub sha256: String,
    /// **ST-90 (VD-110)**: entry của gói; `None` = lock **cũ** (fallback quy ước).
    pub entry: Option<String>,
    pub files: Vec<TepKhoa>,
    /// **ST-129 (VD-125)**: ràng buộc đã chọn ra phiên bản này (dạng **chuẩn hoá**);
    /// `None` = lock **định dạng cũ** (không có trường) ⇒ khi ghi lại phải sinh `=<version>`.
    pub version_req: Option<String>,
}

/// Định dạng **hiện tại** của `vis.lock` (ST-129).
pub const LOCK_PHIEN_BAN: u16 = 2;

/// Toàn bộ `vis.lock` đã đọc: định dạng + **ràng buộc gốc** + các gói (ST-129, VD-125).
pub struct Khoa {
    /// `1` = định dạng cũ (ST-70..ST-128), `2` = định dạng có `roots`/`version_req`.
    pub phien_ban: u16,
    /// Ràng buộc **gốc** người dùng đòi (`pkg add`), đã chuẩn hoá — chỉ có ở định dạng 2.
    pub roots: BTreeMap<String, String>,
    pub goi: Vec<GoiKhoa>,
}

impl Khoa {
    /// Lock định dạng cũ (hoặc thiếu trường mới) ⇒ cần **migrate** khi ghi.
    pub fn can_migrate(&self) -> bool {
        self.phien_ban < LOCK_PHIEN_BAN || self.goi.iter().any(|g| g.version_req.is_none())
    }
}

/// **VS3-5b**: nâng lock cũ lên [`LOCK_PHIEN_BAN`] **an toàn** — mỗi gói thiếu `version_req` được
/// **ghim** đúng phiên bản đang có (`=<version>`), không nới ràng buộc; `roots` giữ nguyên (lock cũ
/// không có `roots` ⇒ để trống, `roots_tu_lock` đã ghim khi cần). Trả `true` nếu cần ghi.
pub fn nang_cap_lock(khoa: &mut Khoa) -> bool {
    let can = khoa.can_migrate();
    if can {
        for g in &mut khoa.goi {
            if g.version_req.is_none() {
                g.version_req = Some(format!("={}", g.version));
            }
        }
        khoa.phien_ban = LOCK_PHIEN_BAN;
    }
    can
}

pub fn doc_lock(duong: &Path, bat_buoc: bool) -> Result<Khoa, String> {
    let bytes = match std::fs::read(duong) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && !bat_buoc => {
            return Ok(Khoa {
                phien_ban: LOCK_PHIEN_BAN,
                roots: BTreeMap::new(),
                goi: Vec::new(),
            })
        }
        Err(e) => return Err(format!("không đọc được `{}`: {e}", duong.display())),
    };
    let text =
        std::str::from_utf8(&bytes).map_err(|e| format!("`vis.lock` không phải UTF-8: {e}"))?;
    let v = vis_core::json::decode(text).map_err(|e| format!("`vis.lock` hỏng: {e}"))?;
    let Value::Map(kv) = &v else {
        return Err("`vis.lock` hỏng: phải là object".to_string());
    };
    let phien_ban = match lay(kv, "lock") {
        Some(Value::Int(1)) => 1u16,
        Some(Value::Int(2)) => 2u16,
        _ => {
            return Err(format!(
                "`vis.lock` hỏng: thiếu `lock` = 1 hoặc {LOCK_PHIEN_BAN}"
            ))
        }
    };
    // **ST-129**: `roots` chỉ có ở định dạng 2; vắng ⇒ map rỗng (không phải lỗi).
    let mut roots: BTreeMap<String, String> = BTreeMap::new();
    if let Some(rv) = lay(kv, "roots") {
        let Value::Map(rm) = rv else {
            return Err("`vis.lock` hỏng: `roots` phải là object".to_string());
        };
        for (k, v) in rm.iter() {
            let Some(ten) = chuoi(k) else {
                return Err("`vis.lock` hỏng: tên trong `roots` phải là chuỗi".to_string());
            };
            let Some(req) = chuoi(v) else {
                return Err(format!("`vis.lock` hỏng: `roots.{ten}` phải là chuỗi"));
            };
            let chuan = VersionReq::parse(req)
                .map_err(|e| format!("`vis.lock` hỏng: `roots.{ten}`: {e}"))?
                .chuan_hoa();
            roots.insert(ten.to_string(), chuan);
        }
    }
    let Some(Value::List(ps)) = lay(kv, "packages") else {
        return Err("`vis.lock` hỏng: thiếu `packages`".to_string());
    };
    let mut out = Vec::new();
    for p in ps.iter() {
        let Value::Map(p) = p else {
            return Err("`vis.lock` hỏng: `packages[]` phải là object".to_string());
        };
        let name = lay_chuoi(p, "name")
            .ok_or_else(|| "`vis.lock` hỏng: package thiếu `name`".to_string())?;
        let version = lay_chuoi(p, "version")
            .ok_or_else(|| "`vis.lock` hỏng: package thiếu `version`".to_string())?;
        so_phien_ban(&version)?;
        let sha256 = lay_chuoi(p, "sha256")
            .ok_or_else(|| "`vis.lock` hỏng: package thiếu `sha256`".to_string())?;
        if !la_hex64(&sha256) {
            return Err("`vis.lock` hỏng: `sha256` package không phải 64 hex".to_string());
        }
        let Some(Value::List(fs)) = lay(p, "files") else {
            return Err("`vis.lock` hỏng: package thiếu `files`".to_string());
        };
        let mut files = Vec::new();
        for f in fs.iter() {
            let Value::Map(f) = f else {
                return Err("`vis.lock` hỏng: `files[]` phải là object".to_string());
            };
            let path = lay_chuoi(f, "path")
                .ok_or_else(|| "`vis.lock` hỏng: file thiếu `path`".to_string())?;
            kiem_path(&path)?;
            let sha = lay_chuoi(f, "sha256")
                .ok_or_else(|| "`vis.lock` hỏng: file thiếu `sha256`".to_string())?;
            if !la_hex64(&sha) {
                return Err("`vis.lock` hỏng: `sha256` file không phải 64 hex".to_string());
            }
            files.push(TepKhoa {
                path,
                sha256: sha.to_ascii_lowercase(),
            });
        }
        files.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
        let entry = match lay_chuoi(p, "entry") {
            Some(e) => {
                kiem_path(&e)?;
                Some(e)
            }
            None => None, // lock cũ: fallback quy ước (không cảnh báo)
        };
        // **ST-129**: `version_req` (định dạng cũ **không** có) — chuẩn hoá khi đọc.
        let version_req = match lay_chuoi(p, "version_req") {
            Some(r) => Some(
                VersionReq::parse(&r)
                    .map_err(|e| format!("`vis.lock` hỏng: `version_req` của `{name}`: {e}"))?
                    .chuan_hoa(),
            ),
            None => None,
        };
        out.push(GoiKhoa {
            name,
            version,
            sha256: sha256.to_ascii_lowercase(),
            entry,
            files,
            version_req,
        });
    }
    out.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
    Ok(Khoa {
        phien_ban,
        roots,
        goi: out,
    })
}

/// Như [`doc_lock`] nhưng chỉ trả danh sách gói (đường cũ của `use pkg.*`/LSP).
pub fn doc_khoa(duong: &Path, bat_buoc: bool) -> Result<Vec<GoiKhoa>, String> {
    Ok(doc_lock(duong, bat_buoc)?.goi)
}

/// Đọc + validate `vis.lock` trong `dir` — dùng cho `use pkg.*` (ST-71).
pub fn doc_khoa_cua(dir: &Path) -> Result<Vec<GoiKhoa>, String> {
    doc_khoa(&dir.join("vis.lock"), true)
}
