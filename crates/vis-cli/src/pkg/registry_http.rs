// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-5a/VS3-5b (VD-271)** — registry **HTTP/HTTPS** cho `vis pkg` (index + tarball + chữ ký).
//!
//! Giao thức `vispkg-http1`:
//!
//! ```text
//! GET <base>/index.json
//! {"format":"vispkg-http1","packages":{
//!   "<tên>":{"<ver>":{"tarball":"<tên>/<ver>.tar.b64",
//!                      "tarball_sha256":"<sha256 của byte tar>",
//!                      "signature_ed25519":"<128 hex — tuỳ chọn/VS3-5b>",
//!                      "manifest":{...như registry cục bộ...}}}}}
//!
//! GET <base>/<tarball>   → body base64 của tar ustar
//! ```
//!
//! Khi người dùng cấu hình khoá công khai (`--registry-key`/`VIS_REGISTRY_KEY`), **mọi** phiên bản
//! được chọn phải có `signature_ed25519` hợp lệ trên thông điệp chuẩn
//! [`vis_cli::pkg_sig::thong_diep_ky`]; kiểm **trước khi tải tarball**. Không cấu hình khoá ⇒
//! trường chữ ký được bỏ qua (registry cục bộ cũng không đòi chữ ký).
//!
//! Luồng: tải index → ghi **manifest** vào thư mục tạm → dùng **đúng resolver semver hiện có** để
//! chọn phiên bản → **chỉ tải tarball của gói đã chọn** → kiểm **SHA-256 tarball** → giải tar, kiểm
//! **SHA-256 từng tệp** theo manifest → đọc bằng [`super::doc_tat_ca`] → `add`/`update` ghi lock/store
//! như cũ. Mọi kiểm tra xảy ra **trước** khi chạm `.vis/`; lỗi hash dùng **`VS0707`**, lỗi giải
//! ràng buộc dùng **`VS0704`** (mã hiện có, không cấp mã mới).
//!
//! TLS **đã có** (VS3-4a): `base` có thể là `https://host[:port][/path]`; client dùng root store,
//! deadline và keep-alive của `vis-cli::net` qua `http_get`. `http://` vẫn chỉ loopback. Tarball đi
//! qua base64 vì `http_get` trả `str`; byte tar sau giải mã vẫn được băm/kiểm đúng.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use vis_interp::json::{decode, json};
use vis_interp::value::Value;

use super::semver::{Version, VersionReq};
use super::{
    bam, chuan_hoa, chuoi, doc_tat_ca, giai_roots, kiem_path, la_hex64, lay, lay_chuoi,
    manifest_tu_gia_tri, GoiCai, Manifest,
};
use vis_cli::net::Net;

/// Hạn mỗi lời gọi HTTP của registry (connect + đọc) — đủ cho mạng cục bộ, không treo CLI.
const TIMEOUT_MS: u64 = 30_000;

/// Một phiên bản trong index. Manifest/đường dẫn/hash được **validate khi được chọn** (không chặn
/// các gói khác nếu một mục lỗi — cùng tinh thần resolver cục bộ: bản hỏng bị bỏ qua).
struct Muc {
    ten: String,
    version: Version,
    version_s: String,
    /// Manifest **nguyên văn** (Value) — resolver/`doc_goi` đọc lại từ thư mục tạm.
    manifest: Value,
    /// Đường dẫn tarball **tương đối** so với base (`None` ⇒ mặc định `<tên>/<ver>.tar.b64`).
    tar: Option<String>,
    /// SHA-256 của **byte tar** sau giải mã base64.
    tar_sha: Option<String>,
    /// **VS3-5b**: chữ ký Ed25519 (128 hex) trên thông điệp chuẩn `vispkg-sig1`; chỉ bắt buộc khi
    /// người dùng cấu hình khoá công khai.
    signature: Option<String>,
}

struct Index {
    ds: Vec<Muc>,
}

impl Index {
    fn tim(&self, ten: &str, version: &Version) -> Option<&Muc> {
        self.ds
            .iter()
            .find(|m| m.ten == ten && &m.version == version)
    }

    /// Ghi `manifest.json` vào registry tạm để resolver hiện có đọc được (không tải tệp nội dung).
    fn viet_manifest(&self, goc: &Path) -> Result<(), String> {
        for m in &self.ds {
            let dir = goc.join(&m.ten).join(&m.version_s);
            std::fs::create_dir_all(&dir)
                .map_err(|e| format!("không tạo được thư mục tạm `{}`: {e}", dir.display()))?;
            let p = dir.join("manifest.json");
            let mut text = String::new();
            json(&m.manifest, false, &mut text);
            std::fs::write(&p, text.as_bytes())
                .map_err(|e| format!("lỗi ghi manifest tạm `{}`: {e}", p.display()))?;
        }
        Ok(())
    }
}

/// Tải index, giải bằng resolver hiện có, tải tarball của **phiên bản đã chọn**, rồi đọc gói.
pub(super) fn tai_va_giai(
    base: &str,
    roots: &BTreeMap<String, VersionReq>,
    entry_override: Option<(&str, &str)>,
    khoa_cong: Option<&[u8; 32]>,
) -> Result<(BTreeMap<String, Version>, Vec<GoiCai>), String> {
    let mut net = Net::moi(false);
    let url = format!("{}/index.json", base.trim_end_matches('/'));
    let body = vis_interp::net::http_get(&mut net, &url, TIMEOUT_MS)
        .map_err(|e| format!("không tải được index registry `{url}`: {e}"))?;
    let index = doc_index(&body)?;
    let tmp = ThuMucTam::moi()?;
    index.viet_manifest(tmp.path())?;
    let giai = giai_roots(tmp.path(), roots, entry_override)?;
    for (ten, ver) in &giai {
        let muc = index
            .tim(ten, ver)
            .ok_or_else(|| format!("index registry `{base}` thiếu gói `{ten}@{ver}`"))?;
        let eo = match entry_override {
            Some((t, e)) if t == ten.as_str() => Some(e),
            _ => None,
        };
        tai_goi(&mut net, base, muc, tmp.path(), eo, khoa_cong)?;
    }
    let ds = doc_tat_ca(tmp.path(), &giai, entry_override)?;
    Ok((giai, ds))
}

/// Parse + validate **khung** `index.json`.
///
/// Manifest/đường dẫn/hash của từng phiên bản được kiểm **khi phiên bản đó được chọn** — một mục
/// hỏng không chặn gói khác (resolver cục bộ cũng bỏ qua bản manifest hỏng). Hỏng **khung** (JSON,
/// `format`, `packages`, khoá phiên bản, trùng mục) thì dừng ngay.
fn doc_index(body: &str) -> Result<Index, String> {
    let v = decode(body).map_err(|e| format!("index registry hỏng: {e}"))?;
    let Value::Map(kv) = &v else {
        return Err("index registry hỏng: phải là object".to_string());
    };
    let dinh_dang =
        lay_chuoi(kv, "format").ok_or_else(|| "index registry hỏng: thiếu `format`".to_string())?;
    if dinh_dang != "vispkg-http1" {
        return Err(format!(
            "index registry hỏng: `format` `{dinh_dang}` — cần `vispkg-http1`"
        ));
    }
    let Some(Value::Map(pkgs)) = lay(kv, "packages") else {
        return Err("index registry hỏng: thiếu `packages` (object)".to_string());
    };
    let mut ds = Vec::new();
    let mut da = BTreeSet::new();
    for (k_ten, v_ban) in pkgs {
        let Some(ten) = chuoi(k_ten) else {
            return Err("index registry hỏng: tên gói phải là chuỗi".to_string());
        };
        kiem_path(ten).map_err(|e| format!("index registry hỏng: tên gói {e}"))?;
        let Value::Map(ban) = v_ban else {
            return Err(format!("index registry hỏng: `{ten}` phải là object"));
        };
        for (k_ver, v_muc) in ban {
            let Some(ver_s) = chuoi(k_ver) else {
                return Err(format!(
                    "index registry hỏng: phiên bản của `{ten}` phải là chuỗi"
                ));
            };
            let version = Version::parse(ver_s)
                .map_err(|e| format!("index gói `{ten}`: phiên bản `{ver_s}` không hợp lệ: {e}"))?;
            let nguon = format!("index gói `{ten}@{ver_s}`");
            if !da.insert((ten.to_string(), ver_s.to_string())) {
                return Err(format!("{nguon}: trùng mục trong index"));
            }
            let Value::Map(muc) = v_muc else {
                return Err(format!("{nguon}: phải là object"));
            };
            let Some(manifest) = lay(muc, "manifest") else {
                return Err(format!("{nguon}: thiếu `manifest`"));
            };
            ds.push(Muc {
                ten: ten.to_string(),
                version,
                version_s: ver_s.to_string(),
                manifest: manifest.clone(),
                tar: lay_chuoi(muc, "tarball"),
                tar_sha: lay_chuoi(muc, "tarball_sha256"),
                signature: lay_chuoi(muc, "signature_ed25519"),
            });
        }
    }
    ds.sort_by(|a, b| a.ten.cmp(&b.ten).then(a.version.cmp(&b.version)));
    Ok(Index { ds })
}

/// Tải + kiểm + giải **một** tarball vào `<tạm>/<tên>/<ver>/` (chỉ tệp khai trong manifest).
fn tai_goi(
    net: &mut Net,
    base: &str,
    muc: &Muc,
    goc: &Path,
    entry_override: Option<&str>,
    khoa_cong: Option<&[u8; 32]>,
) -> Result<(), String> {
    let nguon = format!("index gói `{}@{}`", muc.ten, muc.version_s);
    let manifest = manifest_tu_gia_tri(&muc.manifest, &nguon, entry_override)?;
    if manifest.name != muc.ten || manifest.version != muc.version_s {
        return Err(format!(
            "{nguon}: manifest khai `{}@{}` — lệch khoá index",
            manifest.name, manifest.version
        ));
    }
    let tar_sha = muc
        .tar_sha
        .as_deref()
        .ok_or_else(|| format!("{nguon}: thiếu `tarball_sha256`"))?;
    if !la_hex64(tar_sha) {
        return Err(format!("{nguon}: `tarball_sha256` phải 64 chữ số hex"));
    }
    let tar_sha = tar_sha.to_ascii_lowercase();
    let tar_path = muc
        .tar
        .clone()
        .unwrap_or_else(|| format!("{}/{}.tar.b64", muc.ten, muc.version_s));
    kiem_path(&tar_path).map_err(|e| format!("{nguon}: `tarball` {e}"))?;
    // **VS3-5b**: xác minh chữ ký **trước khi** tải tarball (chữ ký phủ cả metadata manifest).
    if let Some(khoa) = khoa_cong {
        kiem_chu_ky(muc, &manifest, &tar_sha, khoa)?;
    }
    let url = format!("{}/{}", base.trim_end_matches('/'), tar_path);
    let body = vis_interp::net::http_get(net, &url, TIMEOUT_MS).map_err(|e| {
        format!(
            "không tải được tarball `{}@{}` từ `{url}`: {e}",
            muc.ten, muc.version_s
        )
    })?;
    let tar = super::tar_b64::base64_giai(&body)
        .map_err(|e| format!("tarball `{}@{}` (base64) hỏng: {e}", muc.ten, muc.version_s))?;
    let sha = hex(&vis_bytecode::bytes_hash(&tar));
    if sha != tar_sha {
        return Err(format!(
            "{}: tarball `{}@{}` lệch băm — index `{tar_sha}`, tarball thật `{sha}`",
            vis_diag::code(707),
            muc.ten,
            muc.version_s
        ));
    }
    let mut tep = super::tar_b64::tar_doc(&tar)
        .map_err(|e| format!("tarball `{}@{}` hỏng: {e}", muc.ten, muc.version_s))?;
    // Tệp lạ ⇒ từ chối (không ghi gì): tarball chỉ được chứa đúng `manifest.files`.
    let hop_le: BTreeSet<&str> = manifest.files.iter().map(|f| f.path.as_str()).collect();
    for p in tep.keys() {
        if !hop_le.contains(p.as_str()) {
            return Err(format!(
                "tarball `{}@{}` có tệp lạ `{p}` (không khai trong manifest)",
                muc.ten, muc.version_s
            ));
        }
    }
    // Kiểm **toàn bộ** hash từng tệp trước khi ghi (kể cả vào thư mục tạm).
    let mut noi_dung = Vec::with_capacity(manifest.files.len());
    for f in &manifest.files {
        let Some(bytes) = tep.remove(&f.path) else {
            return Err(format!(
                "tarball `{}@{}` thiếu tệp `{}`",
                muc.ten, muc.version_s, f.path
            ));
        };
        let text = chuan_hoa(&bytes).map_err(|e| {
            format!(
                "tarball `{}@{}` tệp `{}`: {e}",
                muc.ten, muc.version_s, f.path
            )
        })?;
        let sha_tep = bam(&text);
        if sha_tep != f.sha256 {
            return Err(format!(
                "{}: tệp `{}` của `{}@{}` lệch băm — manifest `{}`, nội dung thật `{sha_tep}`",
                vis_diag::code(707),
                f.path,
                muc.ten,
                muc.version_s,
                f.sha256
            ));
        }
        noi_dung.push((f.path.clone(), text));
    }
    let dir = goc.join(&muc.ten).join(&muc.version_s);
    for (path, text) in noi_dung {
        let p = dir.join(&path);
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d)
                .map_err(|e| format!("không tạo được `{}`: {e}", d.display()))?;
        }
        std::fs::write(&p, text.as_bytes())
            .map_err(|e| format!("lỗi ghi `{}`: {e}", p.display()))?;
    }
    Ok(())
}

/// **VS3-5b**: kiểm `signature_ed25519` của một phiên bản trên thông điệp chuẩn `vispkg-sig1`.
///
/// Không cấu hình khoá ⇒ không gọi hàm này (registry không bắt buộc chữ ký). Có khoá mà thiếu/sai
/// chữ ký ⇒ lỗi rõ, **không** cấp mã mới (đề xuất `VS0708` cho A0; hiện dùng thông điệp).
fn kiem_chu_ky(
    muc: &Muc,
    manifest: &Manifest,
    tar_sha: &str,
    khoa: &[u8; 32],
) -> Result<(), String> {
    let nguon = format!("index gói `{}@{}`", muc.ten, muc.version_s);
    let Some(sig) = muc.signature.as_deref() else {
        return Err(format!(
            "{nguon}: thiếu `signature_ed25519` — registry yêu cầu chữ ký (đã cấu hình khoá công khai)"
        ));
    };
    let sig = vis_cli::pkg_sig::hex_giai(sig, 64)
        .map_err(|e| format!("{nguon}: `signature_ed25519`: {e}"))?;
    let files: Vec<(String, String)> = manifest
        .files
        .iter()
        .map(|f| (f.path.clone(), f.sha256.clone()))
        .collect();
    let msg = vis_cli::pkg_sig::thong_diep_ky(
        &muc.ten,
        &muc.version_s,
        tar_sha,
        &manifest.entry,
        &manifest.deps,
        &files,
    );
    vis_cli::pkg_sig::xac_minh(khoa, &msg, &sig).map_err(|e| format!("{nguon}: {e} — từ chối cài"))
}

fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

/// Thư mục tạm duy nhất cho một lần giải qua HTTP; **luôn** dọn khi ra khỏi phạm vi (kể cả lỗi).
struct ThuMucTam {
    path: PathBuf,
}

impl ThuMucTam {
    fn moi() -> Result<Self, String> {
        static DEM: AtomicUsize = AtomicUsize::new(0);
        let n = DEM.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("vis-pkg-http-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path)
            .map_err(|e| format!("không tạo được thư mục tạm `{}`: {e}", path.display()))?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ThuMucTam {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
