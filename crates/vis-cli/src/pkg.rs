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

/// **VS3-5b/5c (VD-271)**: dọn blob/thư mục gói không còn được **bất kỳ** `vis.lock` nào tham chiếu
/// (một lock mặc định, `--lock` lặp lại, hoặc `--root` quét có kiểm soát).
mod gc;
/// **VS3-5a (VD-271)**: registry **HTTP/HTTPS** (index + tarball, SHA-256 trước khi cài) — dựng trên
/// HTTP client hiện có của `vis-interp`; `https://` dùng TLS của `vis-cli::net` (VS3-4a).
mod registry_http;
/// **ST-128 (VD-125)**: resolver **tất định** trên registry nhiều phiên bản (chọn cao nhất thoả,
/// quay lui khi xung đột; không giải được ⇒ `VS0704` kèm chuỗi ràng buộc).
pub mod resolver;
/// **ST-127 (VD-125)**: semver tối giản (`Version`/`VersionReq`) — resolver dùng ở ST-128, lock ghi
/// `version_req` ở ST-129. Không crate mới.
pub mod semver;
/// **VS3-5a**: base64 + tar ustar tối giản — cùng lát, tách tệp cho R1.
mod tar_b64;

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
    "dùng: vis pkg add <tên>[@<ver>] [--entry <path>] [--registry <dir|http://host:port|https://host[:port][/path]>] [--registry-key <64-hex>]... | vis pkg install | vis pkg update [<tên>] [--registry <dir|http://host:port|https://host[:port][/path]>] [--registry-key <64-hex>]... | vis pkg list [--json] | vis pkg remove <tên> | vis pkg gc [--dry-run] [--lock <path>]... [--root <dir>]... | vis pkg migrate"
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
        "gc" => gc::lenh(&args[1..]),
        "migrate" => lenh_migrate(&args[1..]),
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
    let nguon = format!("manifest `{}`", duong.display());
    manifest_tu_gia_tri(&v, &nguon, entry_override)
}

/// Validate một manifest từ [`Value`] đã giải mã; `nguon` là mô tả cho chẩn đoán:
/// `manifest \`<đường dẫn>\`` (registry cục bộ) hoặc `index gói \`<tên>@<phiên bản>\`` (HTTP —
/// VS3-5a). Dùng chung để hai nguồn có **cùng** luật chấp nhận/từ chối.
fn manifest_tu_gia_tri(
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
    for it in ds {
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
        for (k, v) in dm {
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
    /// **ST-129 (VD-125)**: ràng buộc đã chọn ra phiên bản này (dạng **chuẩn hoá**);
    /// `None` = lock **định dạng cũ** (không có trường) ⇒ khi ghi lại phải sinh `=<version>`.
    pub(crate) version_req: Option<String>,
}

/// Định dạng **hiện tại** của `vis.lock` (ST-129).
pub(crate) const LOCK_PHIEN_BAN: u16 = 2;

/// Toàn bộ `vis.lock` đã đọc: định dạng + **ràng buộc gốc** + các gói (ST-129, VD-125).
pub(crate) struct Khoa {
    /// `1` = định dạng cũ (ST-70..ST-128), `2` = định dạng có `roots`/`version_req`.
    pub(crate) phien_ban: u16,
    /// Ràng buộc **gốc** người dùng đòi (`pkg add`), đã chuẩn hoá — chỉ có ở định dạng 2.
    pub(crate) roots: BTreeMap<String, String>,
    pub(crate) goi: Vec<GoiKhoa>,
}

impl Khoa {
    /// Lock định dạng cũ (hoặc thiếu trường mới) ⇒ cần **migrate** khi ghi.
    pub(crate) fn can_migrate(&self) -> bool {
        self.phien_ban < LOCK_PHIEN_BAN || self.goi.iter().any(|g| g.version_req.is_none())
    }
}

/// **VS3-5b**: nâng lock cũ lên [`LOCK_PHIEN_BAN`] **an toàn** — mỗi gói thiếu `version_req` được
/// **ghim** đúng phiên bản đang có (`=<version>`), không nới ràng buộc; `roots` giữ nguyên (lock cũ
/// không có `roots` ⇒ để trống, `roots_tu_lock` đã ghim khi cần). Trả `true` nếu cần ghi.
fn nang_cap_lock(khoa: &mut Khoa) -> bool {
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

fn doc_lock(duong: &Path, bat_buoc: bool) -> Result<Khoa, String> {
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
    let v = vis_interp::json::decode(text).map_err(|e| format!("`vis.lock` hỏng: {e}"))?;
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
        for (k, v) in rm {
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
fn doc_khoa(duong: &Path, bat_buoc: bool) -> Result<Vec<GoiKhoa>, String> {
    Ok(doc_lock(duong, bat_buoc)?.goi)
}

/// Đọc + validate `vis.lock` trong `dir` — dùng cho `use pkg.*` (ST-71).
pub(crate) fn doc_khoa_cua(dir: &Path) -> Result<Vec<GoiKhoa>, String> {
    doc_khoa(&dir.join("vis.lock"), true)
}

fn gia_tri_khoa(khoa: &Khoa) -> Value {
    let ps: Vec<Value> = khoa
        .goi
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
            kv.push((Value::Str("files".into()), Value::List(fs)));
            Value::Map(kv)
        })
        .collect();
    let rm: Vec<(Value, Value)> = khoa
        .roots
        .iter()
        .map(|(k, v)| (Value::Str(k.clone()), Value::Str(v.clone())))
        .collect();
    Value::Map(vec![
        (
            Value::Str("lock".into()),
            Value::Int(i64::from(LOCK_PHIEN_BAN)),
        ),
        (Value::Str("roots".into()), Value::Map(rm)),
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

fn ghi_lock(p: &Path, khoa: &Khoa) -> Result<(), String> {
    let mut s = String::new();
    // `false` = **không** sắp lại khoá: thứ tự khoá **cố định** (lock/roots/packages, rồi từng gói)
    // ⇒ ghi hai lần cho **cùng byte** (VD-125).
    json(&gia_tri_khoa(khoa), false, &mut s);
    s.push('\n');
    ghi_atomic(p, s.as_bytes())
}

// ----- khoá: ràng buộc hiệu lực + kiểm "còn thoả" (ST-129) -----

/// Ràng buộc **hiệu lực** của từng gói trong một lời giải = **AND** của ràng buộc gốc và mọi ràng
/// buộc dep trỏ tới nó. In ra **chuẩn hoá** (bỏ trùng, sắp theo chuỗi) ⇒ tất định, ghi được vào
/// `version_req` của lock (ST-129, VD-125).
fn rang_buoc_hieu_luc(
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
fn khoa_thoa(version: &str, req: &VersionReq) -> bool {
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
fn roots_tu_lock(khoa: &Khoa) -> Result<BTreeMap<String, VersionReq>, String> {
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
fn lock_con_thoa(khoa: &Khoa) -> Result<(), String> {
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
fn tao_khoa(
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

/// Giải **đồ thị** từ tập ràng buộc gốc (chưa đọc nội dung gói).
///
/// **VS3-5a** tách khỏi [`doc_tat_ca`] để registry HTTP có thể giải trước, **chỉ tải tarball của
/// phiên bản đã chọn**, rồi mới đọc nội dung — cùng resolver, cùng ngữ nghĩa với registry cục bộ.
fn giai_roots(
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
fn doc_tat_ca(
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
fn giai_va_doc(
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
fn ghi_cay(cwd: &Path, ds: &[GoiCai]) -> Result<(), String> {
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

/// Đối số CLI của `pkg add`/`update` (và kiểm “không nhận” cho `install`/`list`).
struct TuyChon {
    ten: Option<String>,
    reg: Option<String>,
    entry: Option<String>,
    /// **VS3-5b/5c**: khoá công khai Ed25519 (mỗi khoá 64 hex) cho registry HTTP.
    /// Lặp `--registry-key` để hỗ trợ **key rotation**; khoá cũ vẫn verify bản cũ.
    khoa: Vec<String>,
}

fn tuy_chon(args: &[String], cho_registry: bool) -> Result<TuyChon, String> {
    let mut tc = TuyChon {
        ten: None,
        reg: None,
        entry: None,
        khoa: Vec::new(),
    };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--registry" if cho_registry => {
                i += 1;
                let Some(v) = args.get(i) else {
                    return Err("thiếu giá trị sau `--registry`".to_string());
                };
                tc.reg = Some(v.clone());
            }
            "--entry" if cho_registry => {
                i += 1;
                let Some(v) = args.get(i) else {
                    return Err("thiếu giá trị sau `--entry`".to_string());
                };
                kiem_path(v)?;
                tc.entry = Some(v.clone());
            }
            "--registry-key" if cho_registry => {
                i += 1;
                let Some(v) = args.get(i) else {
                    return Err("thiếu giá trị sau `--registry-key`".to_string());
                };
                tc.khoa.push(v.clone());
            }
            a if a.starts_with('-') => return Err(format!("cờ lạ `{a}`")),
            a => {
                if tc.ten.is_some() {
                    return Err(format!("thừa đối số `{a}`"));
                }
                tc.ten = Some(a.to_string());
            }
        }
        i += 1;
    }
    Ok(tc)
}

/// **VS3-5a (VD-271)**: nguồn gói — thư mục cục bộ (mặc định, giữ nguyên ST-70) hoặc registry HTTP.
enum Registry {
    CucBo(PathBuf),
    Http(String),
}

/// Chọn nguồn từ `--registry` / `VIS_REGISTRY`; `http://`/`https://` ⇒ registry HTTP(S), còn lại là
/// **đường dẫn thư mục** như trước. `http://` vẫn chỉ loopback (do `http_get`); `https://` host bất
/// kỳ nhưng vẫn qua `cap[Net]`.
fn registry_tu_chon(chon: Option<String>) -> Result<Registry, String> {
    let s = match chon {
        Some(s) => s,
        // Giữ nguyên đường dẫn cục bộ **không** phải UTF-8 (hành vi cũ dùng `var_os`).
        None => match std::env::var_os("VIS_REGISTRY") {
            Some(os) => match os.to_str() {
                Some(s) => s.to_string(),
                None => return Ok(Registry::CucBo(PathBuf::from(os))),
            },
            None => "registry".to_string(),
        },
    };
    if s.starts_with("http://") || s.starts_with("https://") {
        let scheme = if s.starts_with("https://") {
            "https://"
        } else {
            "http://"
        };
        let base = s.trim_end_matches('/');
        if base.len() <= scheme.len() {
            return Err(format!("registry `{s}` thiếu host"));
        }
        Ok(Registry::Http(base.to_string()))
    } else if s.contains("://") {
        Err(format!(
            "registry `{s}` có giao thức lạ — chỉ hỗ trợ thư mục, `http://host:port` hoặc `https://host[:port][/path]`"
        ))
    } else {
        Ok(Registry::CucBo(PathBuf::from(s)))
    }
}

/// **VS3-5b/5c**: danh sách khoá công khai Ed25519 (32 byte) từ `--registry-key` lặp lại /
/// `VIS_REGISTRY_KEY`.
///
/// Danh sách phân tách bằng dấu phẩy hoặc khoảng trắng (mỗi phần tử vẫn phải đúng 64 hex).
/// Cờ tường minh + registry cục bộ ⇒ lỗi (tránh hiểu nhầm là đã kiểm chữ ký); biến môi trường +
/// registry cục bộ ⇒ bỏ qua. Không có khoá ⇒ vec rỗng ⇒ registry HTTP **không** đòi chữ ký.
fn khoa_cong_tu_chon(chon: &[String], reg: &Registry) -> Result<Vec<[u8; 32]>, String> {
    let (raws, tu_co) = if !chon.is_empty() {
        (chon.to_vec(), true)
    } else {
        match std::env::var("VIS_REGISTRY_KEY") {
            Ok(s) => (vec![s], false),
            Err(_) => return Ok(Vec::new()),
        }
    };
    if matches!(reg, Registry::CucBo(_)) {
        return if tu_co {
            Err(
                "`--registry-key` chỉ áp dụng cho registry HTTP(S) (`--registry http://...` hoặc `https://...`)"
                    .to_string(),
            )
        } else {
            Ok(Vec::new())
        };
    }

    let mut out = Vec::new();
    let mut da = BTreeSet::new();
    // **ST-288 (VD-273)**: khoá tin cậy cấu hình sai là lỗi **VS0708** (cùng mã với lúc xác minh).
    let ma = vis_diag::code(708);
    for raw in &raws {
        let mut co = false;
        for phan in raw
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|p| !p.is_empty())
        {
            co = true;
            let b = vis_cli::pkg_sig::hex_giai(phan, 32).map_err(|e| {
                format!(
                    "{ma}: khoá công khai Ed25519 thứ {} (`--registry-key`/`VIS_REGISTRY_KEY`): {e}",
                    out.len() + 1
                )
            })?;
            let mut khoa = [0u8; 32];
            khoa.copy_from_slice(&b);
            // Giữ thứ tự xuất hiện; bỏ khoá trùng hoàn toàn.
            if da.insert(khoa) {
                out.push(khoa);
            }
        }
        if !co {
            return Err(format!(
                "{ma}: khoá công khai Ed25519 thứ {} (`--registry-key`/`VIS_REGISTRY_KEY`): cần ít nhất một khoá 64 hex",
                out.len() + 1
            ));
        }
    }
    if out.is_empty() {
        return Err(format!(
            "{ma}: cần ít nhất một khoá công khai Ed25519 (`--registry-key`/`VIS_REGISTRY_KEY`)"
        ));
    }
    Ok(out)
}

/// Giải + đọc gói từ nguồn đã chọn. Cục bộ đọc thẳng thư mục; HTTP tải **index**, giải bằng cùng
/// resolver, rồi chỉ tải tarball của phiên bản đã chọn (xem [`registry_http`]). `khoa_cong` chỉ có
/// tác dụng với HTTP (**VS3-5b**). `uu_tien` là **soft pin** cho `pkg update <tên>` (**#142**).
fn nap_goi(
    reg: &Registry,
    roots: &BTreeMap<String, VersionReq>,
    entry_override: Option<(&str, &str)>,
    khoa_cong: &[[u8; 32]],
    uu_tien: &BTreeMap<String, Version>,
) -> Result<(BTreeMap<String, Version>, Vec<GoiCai>), String> {
    match reg {
        Registry::CucBo(p) => giai_va_doc(p, roots, entry_override, uu_tien),
        Registry::Http(base) => {
            registry_http::tai_va_giai(base, roots, entry_override, khoa_cong, uu_tien)
        }
    }
}

fn cwd() -> Result<PathBuf, String> {
    std::env::current_dir().map_err(|e| format!("không lấy được thư mục hiện tại: {e}"))
}

/// `vis pkg add <tên>[@<ràng buộc>] [--registry <dir>]`.
///
/// **ST-128 (VD-125)**: giải đồ thị bằng [`resolver::Resolver`] — mỗi gói **đúng một** phiên bản,
/// chọn **cao nhất thoả mọi ràng buộc**, quay lui khi xung đột; không giải được ⇒ `VS0704`.
///
/// **ST-129**: ghi `vis.lock` **định dạng 2** — `roots` (ràng buộc **gốc**) + `version_req` của
/// từng gói (ràng buộc **hiệu lực** = AND của mọi ràng buộc trỏ tới nó). Không ghi `@` ⇒ root là
/// `^<phiên bản đã chọn>` (giống "lấy bản mới nhất, ở lại major đó").
fn lenh_add(args: &[String]) -> Result<(), String> {
    let TuyChon {
        ten,
        reg,
        entry: entry_override,
        khoa,
    } = tuy_chon(args, true)?;
    let Some(spec) = ten else {
        return Err(format!("thiếu tên gói — {}", dung()));
    };
    let (ten, req_nguoi_dung) = tach_ten(&spec)?;
    let reg = registry_tu_chon(reg)?;
    let khoa_cong = khoa_cong_tu_chon(&khoa, &reg)?;
    let cwd = cwd()?;
    // Đọc lock cũ **trước** khi ghi gì (lock hỏng ⇒ dừng, không nửa vời).
    let khoa_cu = doc_lock(&cwd.join("vis.lock"), false)?;
    let mut roots = roots_tu_lock(&khoa_cu)?;
    // Ràng buộc **tạm** để giải: không ghi `@` ⇒ "mọi phiên bản" rồi sẽ thu về `^<đã chọn>`.
    let req_tam = req_nguoi_dung.clone().unwrap_or_else(req_moi_phien_ban);
    roots.insert(ten.clone(), req_tam);
    // Giải **trước** khi ghi bất cứ thứ gì: xung đột ⇒ không tạo `.vis/`, không sửa `vis.lock`.
    let (giai, ds) = nap_goi(
        &reg,
        &roots,
        entry_override.as_deref().map(|e| (ten.as_str(), e)),
        &khoa_cong,
        &BTreeMap::new(),
    )?;
    // Root **ghi vào lock**: `@req` ⇒ đúng req đó; không `@` ⇒ `^<phiên bản đã chọn>`.
    if req_nguoi_dung.is_none() {
        let v = giai
            .get(&ten)
            .ok_or_else(|| format!("lời giải thiếu gói gốc `{ten}`"))?;
        let r = VersionReq::parse(&format!("^{v}"))
            .map_err(|e| format!("không tạo được ràng buộc `^{v}`: {e}"))?;
        roots.insert(ten.clone(), r);
    }
    // 1) blob nội dung + 2) cây cài đặt (sau khi **mọi** manifest hợp lệ).
    ghi_cay(&cwd, &ds)?;
    // 3) lock định dạng 2: `roots` + `version_req` hiệu lực của từng gói.
    let hieu_luc = rang_buoc_hieu_luc(&roots, &ds);
    let khoa = tao_khoa(&roots, &ds, &hieu_luc);
    ghi_lock(&cwd.join("vis.lock"), &khoa)
}

/// `vis pkg install` — **chỉ** đọc `vis.lock` + `.vis/store`; không mở registry.
///
/// **ST-129 (VD-125)**: lock hiện có **còn thoả** ⇒ **giữ nguyên byte** (không ghi lại). Lock **định
/// dạng cũ** ⇒ đọc được, **ghi lại định dạng mới** (migration, `version_req` = `=<version>` từng gói).
/// Lock định dạng mới nhưng **không** còn thoả ⇒ lỗi rõ, chỉ đường `vis pkg update` (install không
/// có registry nên **không** tự giải lại).
fn lenh_install(args: &[String]) -> Result<(), String> {
    let tc = tuy_chon(args, false)?;
    if tc.ten.is_some() || tc.entry.is_some() || !tc.khoa.is_empty() {
        return Err(format!("`install` không nhận đối số — {}", dung()));
    }
    if tc.reg.is_some() {
        return Err("`install` không nhận `--registry`".to_string());
    }
    let cwd = cwd()?;
    let lock_path = cwd.join("vis.lock");
    let mut khoa = doc_lock(&lock_path, true)?;
    // Migration: lock định dạng cũ ⇒ sinh `version_req` = `=<version>` cho từng gói rồi ghi lại
    // **một lần**; lock định dạng 2 còn thoả ⇒ **không** chạm tệp (byte y hệt).
    let can_ghi = nang_cap_lock(&mut khoa);
    lock_con_thoa(&khoa)?;
    // Kiểm **toàn bộ** blob trước khi chạm `.vis/pkg` (lock hỏng/thiếu blob ⇒ cây giữ nguyên).
    struct GoiLai {
        name: String,
        version: String,
        files: Vec<(String, String)>,
    }
    let mut noi_dung: Vec<GoiLai> = Vec::new();
    for g in &khoa.goi {
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
    if can_ghi {
        ghi_lock(&lock_path, &khoa)?;
    }
    Ok(())
}

/// `vis pkg update [<tên>] [--registry <dir>]`.
///
/// **ST-129 (VD-125)**: **giải lại** từ `roots` của lock (lock cũ ⇒ mỗi gói thành root `=<version>`);
/// gói được chọn lại theo ràng buộc, nội dung đổi ⇒ cài lại + cập nhật sha; ghi lại lock **định dạng
/// mới**.
///
/// **ST-288 (#142)**: có `<tên>` ⇒ **soft pin** các gói khác ở phiên bản đang có — resolver *thử
/// trước* bản đó để gói không liên quan giữ nguyên, nhưng **không** ghim cứng `=<version>`: nếu
/// manifest của gói được update đòi nâng một dep chung thì dep đó được nâng trong phạm vi ràng buộc
/// (thay vì báo `VS0704`/bỏ qua bản mới). Gói không liên quan vẫn giữ phiên bản.
fn lenh_update(args: &[String]) -> Result<(), String> {
    let TuyChon {
        ten,
        reg,
        entry,
        khoa,
    } = tuy_chon(args, true)?;
    if entry.is_some() {
        return Err("`update` không nhận `--entry`".to_string());
    }
    let cwd = cwd()?;
    let lock_path = cwd.join("vis.lock");
    let khoa_cu = doc_lock(&lock_path, true)?;
    if let Some(t) = &ten {
        if !khoa_cu.goi.iter().any(|g| &g.name == t) {
            return Err(format!("chưa cài gói `{t}` — không có trong `vis.lock`"));
        }
    }
    let reg = registry_tu_chon(reg)?;
    let khoa_cong = khoa_cong_tu_chon(&khoa, &reg)?;
    let roots = roots_tu_lock(&khoa_cu)?;
    // **ST-288 (#142)**: soft pin (không hard pin) — xem doc hàm. Gói được update **không** ưu tiên.
    let mut uu_tien: BTreeMap<String, Version> = BTreeMap::new();
    if let Some(t) = &ten {
        for g in &khoa_cu.goi {
            if &g.name != t {
                if let Ok(v) = Version::parse(&g.version) {
                    uu_tien.insert(g.name.clone(), v);
                }
            }
        }
    }
    // Giải **lại** toàn bộ (tất định) rồi so với lock cũ để biết gói nào đổi.
    let (giai, ds) = nap_goi(&reg, &roots, None, &khoa_cong, &uu_tien)?;
    let cu: BTreeMap<&str, &GoiKhoa> = khoa_cu.goi.iter().map(|g| (g.name.as_str(), g)).collect();
    let mut doi = khoa_cu.can_migrate() || khoa_cu.roots.is_empty();
    let mut can_cai: Vec<GoiCai> = Vec::new();
    for g in &ds {
        let v = giai.get(&g.name).map(|v| v.to_string()).unwrap_or_default();
        match cu.get(g.name.as_str()) {
            Some(c) if c.version == g.version && c.sha256 == g.sha256 => {
                println!("không đổi {} {}", g.name, g.version);
            }
            _ => {
                println!("đã cập nhật {} {}", g.name, g.version);
                can_cai.push(GoiCai {
                    name: g.name.clone(),
                    version: g.version.clone(),
                    entry: g.entry.clone(),
                    sha256: g.sha256.clone(),
                    files: g
                        .files
                        .iter()
                        .map(|f| TepCai {
                            path: f.path.clone(),
                            sha256: f.sha256.clone(),
                            noi_dung: f.noi_dung.clone(),
                        })
                        .collect(),
                    deps: g.deps.clone(),
                });
                let _ = v;
                doi = true;
            }
        }
    }
    if !can_cai.is_empty() {
        ghi_cay(&cwd, &can_cai)?;
    }
    let hieu_luc = rang_buoc_hieu_luc(&roots, &ds);
    let khoa = tao_khoa(&roots, &ds, &hieu_luc);
    if doi {
        ghi_lock(&lock_path, &khoa)?;
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
    let tc = tuy_chon(&args, false)?;
    if tc.ten.is_some() || tc.reg.is_some() || tc.entry.is_some() || !tc.khoa.is_empty() {
        return Err(format!("`list` không nhận đối số — {}", dung()));
    }
    let cwd = cwd()?;
    let khoa = doc_lock(&cwd.join("vis.lock"), true)?.goi;
    if json_ra {
        let mut out = String::from("{\"packages\":[");
        for (i, p) in khoa.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            // **ST-129 (VD-125)**: thêm `version_req`; lock cũ chưa có ⇒ in `=<version>` (giá trị
            // hiệu lực khi migrate) để đầu ra **không phụ thuộc** việc lock đã migrate hay chưa.
            let req = p
                .version_req
                .clone()
                .unwrap_or_else(|| format!("={}", p.version));
            out.push_str(&format!(
                "{{\"name\":\"{}\",\"version\":\"{}\",\"version_req\":\"{}\",\"entry\":\"{}\",\"sha256\":\"{}\"}}",
                crate::json_esc::esc(&p.name),
                crate::json_esc::esc(&p.version),
                crate::json_esc::esc(&req),
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
    let mut khoa = doc_lock(&khoa_path, true)?;
    let Some(pos) = khoa.goi.iter().position(|g| g.name == *ten) else {
        return Err(format!("chưa cài gói `{ten}` — không có trong `vis.lock`"));
    };
    let goi = khoa.goi.remove(pos);
    // **ST-129**: bỏ cả ràng buộc gốc (nếu là gói người dùng đòi) — lock vẫn nhất quán.
    khoa.roots.remove(ten);
    // Blob còn dùng bởi gói khác (so **sha**, không theo tên).
    let con_dung: BTreeSet<String> = khoa
        .goi
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

/// `vis pkg migrate` — nâng `vis.lock` cũ lên định dạng hiện tại **một cách tường minh** (cùng logic
/// `install`/`update`: ghim đúng phiên bản đang có, không nới ràng buộc).
///
/// **VS3-5b (VD-271)**: không mạng, không đụng `.vis/pkg`/store; lock đã hiện tại ⇒ **không ghi lại**
/// (byte y hệt); thiếu lock ⇒ lỗi rõ.
fn lenh_migrate(args: &[String]) -> Result<(), String> {
    if !args.is_empty() {
        return Err(format!("`migrate` không nhận đối số — {}", dung()));
    }
    let cwd = cwd()?;
    let lock_path = cwd.join("vis.lock");
    if !lock_path.is_file() {
        return Err("chưa có `vis.lock` — không có gì để migrate".to_string());
    }
    let mut khoa = doc_lock(&lock_path, true)?;
    let can_ghi = nang_cap_lock(&mut khoa);
    // Kể cả lock đã mới: vẫn kiểm nhất quán trước khi báo “không đổi”.
    lock_con_thoa(&khoa)?;
    if !can_ghi {
        println!("vis.lock đã ở định dạng {LOCK_PHIEN_BAN} — không đổi");
        return Ok(());
    }
    ghi_lock(&lock_path, &khoa)?;
    println!("đã nâng cấp vis.lock lên định dạng {LOCK_PHIEN_BAN}");
    Ok(())
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
