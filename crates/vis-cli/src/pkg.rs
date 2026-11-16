// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-70 (VD-101)** — `vis pkg`: registry **local** (không mạng), cài vào `.vis/pkg/<tên>/<ver>/`.
//!
//! Mô hình: `manifest.json` của mỗi gói khai **tường minh** `files[{path,sha256}]` (không glob,
//! không `read_dir` khi cài); nội dung được **chuẩn hoá** (bỏ BOM, CRLF→LF, đúng một LF cuối) **khi
//! ingest** ⇒ băm ổn định. `vis.lock` ghi **tất cả** gói đã cài (khoá sắp theo tên, không timestamp,
//! không đường dẫn tuyệt đối). `install` **chỉ** đọc `vis.lock` + kho blob `.vis/store/<sha256>` ⇒
//! không cần registry.
//!
//! Băm: `file_sha = SHA256(nội dung chuẩn hoá)`;
//! `tree_sha = SHA256("vispkg1\n" + Σ sorted(path+"\n"+file_sha+"\n"))` — dùng
//! [`vis_bytecode::source_hash`] (SHA-256 thật, không thêm crate).

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use vis_interp::stdlib::json;
use vis_interp::value::Value;

/// **ST-128 (VD-125)**: resolver **tất định** trên registry nhiều phiên bản (chọn cao nhất thoả,
/// quay lui khi xung đột; không giải được ⇒ `VS0704` kèm chuỗi ràng buộc).
pub mod resolver;
/// **ST-127 (VD-125)**: semver tối giản (`Version`/`VersionReq`) — resolver dùng ở ST-128, lock ghi
/// `version_req` ở ST-129. Không crate mới.
pub mod semver;

use std::collections::BTreeMap;

use semver::{Version, VersionReq};

/// Chạy `vis pkg …`; lỗi ⇒ `vis pkg: …` + exit **2** (mẫu `vis cache`, không mã mới).
pub fn run(args: &[String]) -> ExitCode {
    match chay(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("vis pkg: {e}");
            ExitCode::from(2)
        }
    }
}

fn dung() -> &'static str {
    "dùng: vis pkg add <tên>[@<ver>] [--entry <path>] [--registry <dir>] | vis pkg install | vis pkg update [<tên>] [--registry <dir>] | vis pkg list [--json] | vis pkg remove <tên>"
}

fn chay(args: &[String]) -> Result<(), String> {
    let Some(lenh) = args.first() else {
        return Err(dung().to_string());
    };
    match lenh.as_str() {
        "add" => lenh_add(&args[1..]),
        "install" => lenh_install(&args[1..]),
        "update" => lenh_update(&args[1..]),
        "list" => lenh_list(&args[1..]),
        "remove" => lenh_remove(&args[1..]),
        khac => Err(format!("lệnh lạ `{khac}` — {}", dung())),
    }
}

/// Băm hex của một chuỗi (SHA-256 thật, qua `vis-bytecode`).
pub(crate) fn bam(s: &str) -> String {
    let h = vis_bytecode::source_hash(s);
    let mut out = String::with_capacity(64);
    for b in h {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// Chuẩn hoá **khi ingest**: bỏ BOM, CRLF→LF, đúng **một** LF cuối; từ chối non-UTF-8.
pub(crate) fn chuan_hoa(bytes: &[u8]) -> Result<String, String> {
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

/// `tree_sha` trên danh sách `(path, file_sha)` — sắp theo **byte** của path.
fn bam_cay(files: &[(String, String)]) -> String {
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

// ----- JSON tiện ích -----

fn lay<'a>(kv: &'a [(Value, Value)], k: &str) -> Option<&'a Value> {
    kv.iter().find_map(|(kk, v)| match kk {
        Value::Str(s) if s == k => Some(v),
        _ => None,
    })
}

fn chuoi(v: &Value) -> Option<&str> {
    match v {
        Value::Str(s) => Some(s),
        _ => None,
    }
}

fn lay_chuoi(kv: &[(Value, Value)], k: &str) -> Option<String> {
    lay(kv, k).and_then(chuoi).map(str::to_string)
}

fn la_hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Đường dẫn tương đối an toàn: chỉ component `Normal`, không `..`/tuyệt đối/`\`.
fn kiem_path(p: &str) -> Result<(), String> {
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

// ----- manifest -----

struct TepManifest {
    path: String,
    sha256: String,
}

struct Manifest {
    name: String,
    version: String,
    entry: String,
    deps: Vec<(String, String)>,
    files: Vec<TepManifest>,
}

fn doc_manifest(duong: &Path, entry_override: Option<&str>) -> Result<Manifest, String> {
    let bytes = std::fs::read(duong)
        .map_err(|e| format!("không đọc được manifest `{}`: {e}", duong.display()))?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|e| format!("manifest `{}` không phải UTF-8: {e}", duong.display()))?;
    let v = vis_interp::json::decode(text)
        .map_err(|e| format!("manifest `{}` hỏng: {e}", duong.display()))?;
    let Value::Map(kv) = &v else {
        return Err(format!("manifest `{}` phải là object", duong.display()));
    };
    let name = lay_chuoi(kv, "name")
        .ok_or_else(|| format!("manifest `{}` thiếu trường `name`", duong.display()))?;
    let version = lay_chuoi(kv, "version")
        .ok_or_else(|| format!("manifest `{}` thiếu trường `version`", duong.display()))?;
    // **ST-90**: `entry` của manifest vẫn là nguồn chính; `--entry` chỉ **đổi** khi manifest thiếu
    // hoặc người dùng muốn chỉ định khác (vẫn phải nằm trong `files`).
    let entry = lay_chuoi(kv, "entry")
        .or_else(|| entry_override.map(str::to_string))
        .ok_or_else(|| format!("manifest `{}` thiếu trường `entry`", duong.display()))?;
    kiem_path(&entry)?;
    let fs_v = lay(kv, "files")
        .ok_or_else(|| format!("manifest `{}` thiếu trường `files`", duong.display()))?;
    let Value::List(ds) = fs_v else {
        return Err(format!(
            "manifest `{}`: `files` phải là mảng",
            duong.display()
        ));
    };
    let mut files = Vec::new();
    let mut da = BTreeSet::new();
    for it in ds {
        let Value::Map(f) = it else {
            return Err(format!(
                "manifest `{}`: mỗi `files[]` phải là object",
                duong.display()
            ));
        };
        let path = lay_chuoi(f, "path")
            .ok_or_else(|| format!("manifest `{}`: `files[]` thiếu `path`", duong.display()))?;
        kiem_path(&path)?;
        let sha = lay_chuoi(f, "sha256")
            .ok_or_else(|| format!("manifest `{}`: `files[]` thiếu `sha256`", duong.display()))?;
        if !la_hex64(&sha) {
            return Err(format!(
                "manifest `{}`: `sha256` của `{path}` phải 64 chữ số hex",
                duong.display()
            ));
        }
        if !da.insert(path.clone()) {
            return Err(format!(
                "manifest `{}`: trùng `files[].path` `{path}`",
                duong.display()
            ));
        }
        files.push(TepManifest {
            path,
            sha256: sha.to_ascii_lowercase(),
        });
    }
    if files.is_empty() {
        return Err(format!("manifest `{}`: `files` rỗng", duong.display()));
    }
    if !files.iter().any(|f| f.path == entry) {
        return Err(format!(
            "manifest `{}`: `entry` `{entry}` không nằm trong `files`",
            duong.display()
        ));
    }
    let mut deps = Vec::new();
    if let Some(dv) = lay(kv, "deps") {
        let Value::Map(dm) = dv else {
            return Err(format!(
                "manifest `{}`: `deps` phải là object",
                duong.display()
            ));
        };
        for (k, v) in dm {
            let Some(dn) = chuoi(k) else {
                return Err(format!(
                    "manifest `{}`: tên dep phải là chuỗi",
                    duong.display()
                ));
            };
            let Some(dver) = chuoi(v) else {
                return Err(format!(
                    "manifest `{}`: version của dep `{dn}` phải là chuỗi",
                    duong.display()
                ));
            };
            // **ST-128 (VD-125)**: `deps` là **ràng buộc** semver (`^1.2`, `>=1, <2`, `1.0.0`…),
            // không còn bắt buộc là version chính xác. Chẩn đoán nêu đúng tên gói + chuỗi sai.
            VersionReq::parse(dver).map_err(|e| {
                format!(
                    "manifest `{}`: ràng buộc của dep `{dn}` không hợp lệ: {e}",
                    duong.display()
                )
            })?;
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

// ----- phiên bản theo **số** -----

fn so_phien_ban(v: &str) -> Result<Vec<u64>, String> {
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

// ----- gom closure để cài -----

struct TepCai {
    path: String,
    sha256: String,
    noi_dung: String,
}

struct GoiCai {
    name: String,
    version: String,
    entry: String,
    sha256: String,
    files: Vec<TepCai>,
    /// **ST-128**: deps dạng **ràng buộc** (đường `pkg update`).
    deps: Vec<(String, String)>,
}

/// Đọc **một** gói cụ thể trong registry ⇒ [`GoiCai`] (kiểm symlink, chuẩn hoá, băm tệp, `tree_sha`).
fn doc_goi(
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

/// Gom **một gói + closure phụ thuộc** (đường `pkg update`): mỗi ràng buộc dep được giải thành
/// phiên bản **cao nhất thoả** ([`resolver::cao_nhat_thoa`]) — **ST-128**.
fn gom_goi(
    reg: &Path,
    name: &str,
    version: &str,
    entry_override: Option<&str>,
    ds: &mut Vec<GoiCai>,
    da: &mut BTreeSet<(String, String)>,
) -> Result<(), String> {
    if !da.insert((name.to_string(), version.to_string())) {
        return Ok(());
    }
    let g = doc_goi(reg, name, version, entry_override)?;
    let deps = g.deps.clone();
    ds.push(g);
    for (dn, dv) in &deps {
        // **ST-128 (VD-125)**: dep là ràng buộc ⇒ chọn phiên bản cao nhất thoả.
        let req = VersionReq::parse(dv)
            .map_err(|e| format!("ràng buộc của dep `{dn}` không hợp lệ: {e}"))?;
        let v = resolver::cao_nhat_thoa(reg, dn, &req).map_err(|e| loi_xung_dot(&e))?;
        // Dep không nhận `--entry` của gói đích.
        gom_goi(reg, dn, &v.to_string(), None, ds, da)?;
    }
    Ok(())
}

/// **ST-128**: gom **lời giải** đã chốt của resolver (mỗi gói **đúng một** phiên bản) ⇒ `GoiCai`.
fn gom_theo_giai(
    reg: &Path,
    giai: &BTreeMap<String, Version>,
    goc: &str,
    entry_override: Option<&str>,
) -> Result<Vec<GoiCai>, String> {
    let mut ds = Vec::with_capacity(giai.len());
    for (ten, ver) in giai {
        // `--entry` chỉ áp cho **gói gốc** người dùng gọi (như trước ST-128).
        let eo = if ten == goc { entry_override } else { None };
        ds.push(doc_goi(reg, ten, &ver.to_string(), eo)?);
    }
    Ok(ds)
}

/// Đổi [`resolver::LoiXungDot`] thành thông điệp CLI có **mã `VS0704`** + **chuỗi ràng buộc**.
fn loi_xung_dot(e: &resolver::LoiXungDot) -> String {
    // `vis_diag::code` = `VS` + 4 chữ số (hàm có sẵn của sổ mã) — không viết literal mã ở đây.
    format!("{}: {e}", vis_diag::code(704))
}

// ----- vis.lock -----

pub(crate) struct TepKhoa {
    pub(crate) path: String,
    pub(crate) sha256: String,
}

pub(crate) struct GoiKhoa {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) sha256: String,
    /// **ST-90 (VD-110)**: entry của gói; `None` = lock **cũ** (fallback quy ước).
    pub(crate) entry: Option<String>,
    pub(crate) files: Vec<TepKhoa>,
}

fn doc_khoa(duong: &Path, bat_buoc: bool) -> Result<Vec<GoiKhoa>, String> {
    let bytes = match std::fs::read(duong) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && !bat_buoc => return Ok(Vec::new()),
        Err(e) => return Err(format!("không đọc được `{}`: {e}", duong.display())),
    };
    let text =
        std::str::from_utf8(&bytes).map_err(|e| format!("`vis.lock` không phải UTF-8: {e}"))?;
    let v = vis_interp::json::decode(text).map_err(|e| format!("`vis.lock` hỏng: {e}"))?;
    let Value::Map(kv) = &v else {
        return Err("`vis.lock` hỏng: phải là object".to_string());
    };
    match lay(kv, "lock") {
        Some(Value::Int(1)) => {}
        _ => return Err("`vis.lock` hỏng: thiếu `lock` = 1".to_string()),
    }
    let Some(Value::List(ps)) = lay(kv, "packages") else {
        return Err("`vis.lock` hỏng: thiếu `packages`".to_string());
    };
    let mut out = Vec::new();
    for p in ps {
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
        for f in fs {
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
        out.push(GoiKhoa {
            name,
            version,
            sha256: sha256.to_ascii_lowercase(),
            entry,
            files,
        });
    }
    out.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
    Ok(out)
}

/// Đọc + validate `vis.lock` trong `dir` — dùng cho `use pkg.*` (ST-71).
pub(crate) fn doc_khoa_cua(dir: &Path) -> Result<Vec<GoiKhoa>, String> {
    doc_khoa(&dir.join("vis.lock"), true)
}

fn gia_tri_khoa(goi: &[GoiKhoa]) -> Value {
    let ps: Vec<Value> = goi
        .iter()
        .map(|p| {
            let fs: Vec<Value> = p
                .files
                .iter()
                .map(|f| {
                    Value::Map(vec![
                        (Value::Str("path".into()), Value::Str(f.path.clone())),
                        (Value::Str("sha256".into()), Value::Str(f.sha256.clone())),
                    ])
                })
                .collect();
            let mut kv = vec![
                (Value::Str("name".into()), Value::Str(p.name.clone())),
                (Value::Str("version".into()), Value::Str(p.version.clone())),
            ];
            if let Some(e) = &p.entry {
                kv.push((Value::Str("entry".into()), Value::Str(e.clone())));
            }
            kv.push((Value::Str("sha256".into()), Value::Str(p.sha256.clone())));
            kv.push((Value::Str("files".into()), Value::List(fs)));
            Value::Map(kv)
        })
        .collect();
    Value::Map(vec![
        (Value::Str("lock".into()), Value::Int(1)),
        (Value::Str("packages".into()), Value::List(ps)),
    ])
}

fn ghi_atomic(p: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d)
            .map_err(|e| format!("không tạo được thư mục `{}`: {e}", d.display()))?;
    }
    let tmp = p.with_extension("tmp");
    std::fs::write(&tmp, bytes).map_err(|e| format!("lỗi ghi `{}`: {e}", tmp.display()))?;
    std::fs::rename(&tmp, p).map_err(|e| format!("lỗi đổi tên `{}`: {e}", p.display()))
}

fn ghi_lock(p: &Path, goi: &[GoiKhoa]) -> Result<(), String> {
    let mut s = String::new();
    json(&gia_tri_khoa(goi), false, &mut s);
    s.push('\n');
    ghi_atomic(p, s.as_bytes())
}

// ----- lệnh -----

/// Đọc phần sau `@` của spec CLI thành **ràng buộc** (ST-128, VD-125).
///
/// Luật giữ **tương thích** với `pkg add tên@ver` của ST-70: nếu phần sau `@` là một phiên bản
/// **đủ ba thành phần, không toán tử** thì hiểu là **ghim đúng** (`=ver`); mọi dạng khác là ràng
/// buộc (`^1.2`, `~1`, `>=1, <2`, `1.2`…). Không có `@` ⇒ ràng buộc "mọi phiên bản"
/// (`>=0.0.0`), resolver chọn **cao nhất**.
fn doc_req_cli(ver: &str) -> Result<VersionReq, String> {
    let t = ver.trim();
    if Version::parse(t).is_ok() {
        let ghim = format!("={t}");
        return VersionReq::parse(&ghim).map_err(|e| e.to_string());
    }
    VersionReq::parse(t).map_err(|e| format!("ràng buộc `{ver}` không hợp lệ: {e}"))
}

/// Tách `tên[@ràng buộc]`.
fn tach_ten(spec: &str) -> Result<(String, Option<VersionReq>), String> {
    match spec.rsplit_once('@') {
        Some((ten, ver)) => {
            if ten.is_empty() || ver.is_empty() {
                return Err(format!("tên gói `{spec}` không hợp lệ (dạng `tên[@req]`)"));
            }
            Ok((ten.to_string(), Some(doc_req_cli(ver)?)))
        }
        None => Ok((spec.to_string(), None)),
    }
}

/// Ràng buộc "mọi phiên bản" — dùng khi `pkg add <tên>` không ghi `@` (resolver chọn **cao nhất**).
fn req_moi_phien_ban() -> VersionReq {
    VersionReq::parse(">=0.0.0").expect("hằng ràng buộc `>=0.0.0` phải hợp lệ")
}

/// `(tên, registry, entry)` từ đối số CLI.
type TuyChon = (Option<String>, Option<PathBuf>, Option<String>);

fn tuy_chon(args: &[String], cho_registry: bool) -> Result<TuyChon, String> {
    let mut ten = None;
    let mut reg = None;
    let mut entry = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--registry" if cho_registry => {
                i += 1;
                let Some(v) = args.get(i) else {
                    return Err("thiếu giá trị sau `--registry`".to_string());
                };
                reg = Some(PathBuf::from(v));
            }
            "--entry" if cho_registry => {
                i += 1;
                let Some(v) = args.get(i) else {
                    return Err("thiếu giá trị sau `--entry`".to_string());
                };
                kiem_path(v)?;
                entry = Some(v.clone());
            }
            a if a.starts_with('-') => return Err(format!("cờ lạ `{a}`")),
            a => {
                if ten.is_some() {
                    return Err(format!("thừa đối số `{a}`"));
                }
                ten = Some(a.to_string());
            }
        }
        i += 1;
    }
    Ok((ten, reg, entry))
}

fn thu_muc_registry(chon: Option<PathBuf>) -> PathBuf {
    chon.or_else(|| std::env::var_os("VIS_REGISTRY").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("registry"))
}

fn cwd() -> Result<PathBuf, String> {
    std::env::current_dir().map_err(|e| format!("không lấy được thư mục hiện tại: {e}"))
}

/// `vis pkg add <tên>[@<ràng buộc>] [--registry <dir>]`.
///
/// **ST-128 (VD-125)**: giải đồ thị bằng [`resolver::Resolver`] — mỗi gói **đúng một** phiên bản,
/// chọn **cao nhất thoả mọi ràng buộc**, quay lui khi xung đột; không giải được ⇒ `VS0704`.
fn lenh_add(args: &[String]) -> Result<(), String> {
    let (ten, reg, entry_override) = tuy_chon(args, true)?;
    let Some(spec) = ten else {
        return Err(format!("thiếu tên gói — {}", dung()));
    };
    let (ten, req) = tach_ten(&spec)?;
    let reg = thu_muc_registry(reg);
    let req = req.unwrap_or_else(req_moi_phien_ban);
    let cwd = cwd()?;
    // Đọc lock cũ **trước** khi ghi gì (lock hỏng ⇒ dừng, không nửa vời).
    let mut khoa = doc_khoa(&cwd.join("vis.lock"), false)?;
    // Giải **trước** khi ghi bất cứ thứ gì: xung đột ⇒ không tạo `.vis/`, không sửa `vis.lock`.
    let mut giai_bo = resolver::Resolver::moi(&reg);
    if let Some(e) = entry_override.as_deref() {
        // `--entry` chỉ áp cho **gói gốc** (manifest của nó có thể thiếu `entry`).
        giai_bo = giai_bo.voi_entry_goc(&ten, e);
    }
    let giai = giai_bo
        .giai(&[(ten.clone(), req.clone())])
        .map_err(|e| loi_xung_dot(&e))?;
    let mut ds = gom_theo_giai(&reg, &giai, &ten, entry_override.as_deref())?;
    ds.sort_by(|a, b| a.name.cmp(&b.name).then(a.version.cmp(&b.version)));
    // 1) blob nội dung (content-addressed) — chỉ sau khi **mọi** manifest hợp lệ.
    for g in &ds {
        for f in &g.files {
            ghi_atomic(
                &cwd.join(".vis/store").join(&f.sha256),
                f.noi_dung.as_bytes(),
            )?;
        }
    }
    // 2) cây cài đặt: xoá đích rồi ghi từng tệp theo thứ tự sort (không còn tệp thừa).
    for g in &ds {
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
    // 3) lock cuối cùng (merge theo tên; gói mới thắng).
    for g in ds {
        khoa.retain(|x| x.name != g.name);
        khoa.push(GoiKhoa {
            name: g.name,
            version: g.version,
            sha256: g.sha256,
            entry: Some(g.entry),
            files: g
                .files
                .iter()
                .map(|f| TepKhoa {
                    path: f.path.clone(),
                    sha256: f.sha256.clone(),
                })
                .collect(),
        });
    }
    khoa.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
    ghi_lock(&cwd.join("vis.lock"), &khoa)
}

/// `vis pkg install` — **chỉ** đọc `vis.lock` + `.vis/store`; không mở registry.
fn lenh_install(args: &[String]) -> Result<(), String> {
    let (ten, reg, entry) = tuy_chon(args, false)?;
    if ten.is_some() || entry.is_some() {
        return Err(format!("`install` không nhận đối số — {}", dung()));
    }
    if reg.is_some() {
        return Err("`install` không nhận `--registry`".to_string());
    }
    let cwd = cwd()?;
    let khoa = doc_khoa(&cwd.join("vis.lock"), true)?;
    // Kiểm **toàn bộ** blob trước khi chạm `.vis/pkg` (lock hỏng/thiếu blob ⇒ cây giữ nguyên).
    struct GoiLai {
        name: String,
        version: String,
        files: Vec<(String, String)>,
    }
    let mut noi_dung: Vec<GoiLai> = Vec::new();
    for g in &khoa {
        let mut fs = Vec::new();
        for f in &g.files {
            let blob = cwd.join(".vis/store").join(&f.sha256);
            let bytes = std::fs::read(&blob).map_err(|_| {
                format!(
                    "thiếu blob `{}` cho `{}@{}` — chạy `vis pkg add`",
                    f.sha256, g.name, g.version
                )
            })?;
            let text = std::str::from_utf8(&bytes)
                .map_err(|e| format!("blob `{}` không phải UTF-8: {e}", f.sha256))?;
            if bam(text) != f.sha256 {
                return Err(format!("blob `{}` hỏng (băm lệch)", f.sha256));
            }
            fs.push((f.path.clone(), text.to_string()));
        }
        noi_dung.push(GoiLai {
            name: g.name.clone(),
            version: g.version.clone(),
            files: fs,
        });
    }
    for g in noi_dung {
        let dich = cwd.join(".vis/pkg").join(&g.name).join(&g.version);
        let _ = std::fs::remove_dir_all(&dich);
        for (path, text) in g.files {
            let p = dich.join(&path);
            if let Some(d) = p.parent() {
                std::fs::create_dir_all(d)
                    .map_err(|e| format!("không tạo được `{}`: {e}", d.display()))?;
            }
            std::fs::write(&p, text.as_bytes())
                .map_err(|e| format!("lỗi ghi `{}`: {e}", p.display()))?;
        }
    }
    Ok(())
}

/// `vis pkg update [<tên>] [--registry <dir>]` — đọc lại nguồn registry **cùng version**; băm cây khác
/// ⇒ cài lại + cập nhật `sha256`/`files`/`entry`; **không** đổi version (không resolver), **không** mạng.
fn lenh_update(args: &[String]) -> Result<(), String> {
    let (ten, reg, entry) = tuy_chon(args, true)?;
    if entry.is_some() {
        return Err("`update` không nhận `--entry`".to_string());
    }
    let cwd = cwd()?;
    let mut khoa = doc_khoa(&cwd.join("vis.lock"), true)?;
    if let Some(t) = &ten {
        if !khoa.iter().any(|g| &g.name == t) {
            return Err(format!("chưa cài gói `{t}` — không có trong `vis.lock`"));
        }
    }
    let reg = thu_muc_registry(reg);
    let mut doi = false;
    for g in &mut khoa {
        if ten.as_ref().is_some_and(|t| t != &g.name) {
            continue;
        }
        let mut ds = Vec::new();
        let mut da = BTreeSet::new();
        gom_goi(&reg, &g.name, &g.version, None, &mut ds, &mut da)?;
        let Some(moi) = ds
            .into_iter()
            .find(|x| x.name == g.name && x.version == g.version)
        else {
            return Err(format!(
                "registry `{}` không có `{}@{}`",
                reg.display(),
                g.name,
                g.version
            ));
        };
        if moi.sha256 == g.sha256 {
            println!("không đổi {} {}", g.name, g.version);
            continue;
        }
        for f in &moi.files {
            ghi_atomic(
                &cwd.join(".vis/store").join(&f.sha256),
                f.noi_dung.as_bytes(),
            )?;
        }
        let dich = cwd.join(".vis/pkg").join(&moi.name).join(&moi.version);
        let _ = std::fs::remove_dir_all(&dich);
        for f in &moi.files {
            let p = dich.join(&f.path);
            if let Some(d) = p.parent() {
                std::fs::create_dir_all(d)
                    .map_err(|e| format!("không tạo được `{}`: {e}", d.display()))?;
            }
            std::fs::write(&p, f.noi_dung.as_bytes())
                .map_err(|e| format!("lỗi ghi `{}`: {e}", p.display()))?;
        }
        g.sha256 = moi.sha256;
        g.entry = Some(moi.entry);
        g.files = moi
            .files
            .iter()
            .map(|f| TepKhoa {
                path: f.path.clone(),
                sha256: f.sha256.clone(),
            })
            .collect();
        doi = true;
        println!("đã cập nhật {} {}", g.name, g.version);
    }
    if doi {
        ghi_lock(&cwd.join("vis.lock"), &khoa)?;
    }
    Ok(())
}

/// `vis pkg list [--json]` — không cờ: `tên ver sha256[:12]` (giữ nguyên); `--json`:
/// `{"packages":[{name,version,entry,sha256}]}` sắp theo **tên byte-order**.
fn lenh_list(args: &[String]) -> Result<(), String> {
    let json_ra = args.iter().any(|a| a == "--json");
    let args: Vec<String> = args
        .iter()
        .filter(|a| a.as_str() != "--json")
        .cloned()
        .collect();
    let (ten, reg, entry) = tuy_chon(&args, false)?;
    if ten.is_some() || reg.is_some() || entry.is_some() {
        return Err(format!("`list` không nhận đối số — {}", dung()));
    }
    let cwd = cwd()?;
    let khoa = doc_khoa(&cwd.join("vis.lock"), true)?;
    if json_ra {
        let mut out = String::from("{\"packages\":[");
        for (i, p) in khoa.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&format!(
                "{{\"name\":\"{}\",\"version\":\"{}\",\"entry\":\"{}\",\"sha256\":\"{}\"}}",
                crate::json_esc::esc(&p.name),
                crate::json_esc::esc(&p.version),
                crate::json_esc::esc(p.entry.as_deref().unwrap_or("")),
                crate::json_esc::esc(&p.sha256)
            ));
        }
        out.push_str("]}");
        println!("{out}");
        return Ok(());
    }
    for p in &khoa {
        println!(
            "{} {} {}",
            p.name,
            p.version,
            &p.sha256[..12.min(p.sha256.len())]
        );
    }
    Ok(())
}

/// `vis pkg remove <tên>` — xoá mục trong `vis.lock` + cây `.vis/pkg/<tên>/<ver>`; blob trong
/// `.vis/store` chỉ xoá khi **không** gói còn lại nào dùng. **Không** sửa `vis.toml`, không mạng.
fn lenh_remove(args: &[String]) -> Result<(), String> {
    if args.len() != 1 {
        return Err(format!("`remove` cần đúng **một** tên gói — {}", dung()));
    }
    let ten = &args[0];
    if ten.starts_with('-') {
        return Err(format!("cờ lạ `{ten}` — {}", dung()));
    }
    let cwd = cwd()?;
    let khoa_path = cwd.join("vis.lock");
    let mut khoa = doc_khoa(&khoa_path, true)?;
    let Some(pos) = khoa.iter().position(|g| g.name == *ten) else {
        return Err(format!("chưa cài gói `{ten}` — không có trong `vis.lock`"));
    };
    let goi = khoa.remove(pos);
    // Blob còn dùng bởi gói khác (so **sha**, không theo tên).
    let con_dung: BTreeSet<String> = khoa
        .iter()
        .flat_map(|g| g.files.iter().map(|f| f.sha256.clone()))
        .collect();
    for f in &goi.files {
        if !con_dung.contains(&f.sha256) {
            let _ = std::fs::remove_file(cwd.join(".vis/store").join(&f.sha256));
        }
    }
    let thu_muc = cwd.join(".vis/pkg").join(&goi.name);
    let _ = std::fs::remove_dir_all(thu_muc.join(&goi.version));
    let _ = std::fs::remove_dir(thu_muc);
    ghi_lock(&khoa_path, &khoa)
}

#[cfg(test)]
mod tests {
    use super::semver;
    use super::{bam, bam_cay, chuan_hoa, so_phien_ban};

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
