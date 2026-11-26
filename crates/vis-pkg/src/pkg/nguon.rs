// SPDX-License-Identifier: MIT OR Apache-2.0
//! Đối số CLI, nguồn gói (thư mục / HTTP) và khoá công khai Ed25519 (**ST-128**, **VS3-5a/b/c**)
//! — tách khỏi `pkg.rs` ở `ST-317b`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use super::semver::{Version, VersionReq};

use super::goi::{giai_va_doc, GoiCai};
use super::{kiem_path, registry_http};

/// Đọc phần sau `@` của spec CLI thành **ràng buộc** (ST-128, VD-125).
///
/// Luật giữ **tương thích** với `pkg add tên@ver` của ST-70: nếu phần sau `@` là một phiên bản
/// **đủ ba thành phần, không toán tử** thì hiểu là **ghim đúng** (`=ver`); mọi dạng khác là ràng
/// buộc (`^1.2`, `~1`, `>=1, <2`, `1.2`…). Không có `@` ⇒ ràng buộc "mọi phiên bản"
/// (`>=0.0.0`), resolver chọn **cao nhất**.
pub(crate) fn doc_req_cli(ver: &str) -> Result<VersionReq, String> {
    let t = ver.trim();
    if Version::parse(t).is_ok() {
        let ghim = format!("={t}");
        return VersionReq::parse(&ghim).map_err(|e| e.to_string());
    }
    VersionReq::parse(t).map_err(|e| format!("ràng buộc `{ver}` không hợp lệ: {e}"))
}

/// Tách `tên[@ràng buộc]`.
pub(crate) fn tach_ten(spec: &str) -> Result<(String, Option<VersionReq>), String> {
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
pub(crate) fn req_moi_phien_ban() -> VersionReq {
    VersionReq::parse(">=0.0.0").expect("hằng ràng buộc `>=0.0.0` phải hợp lệ")
}

/// Đối số CLI của `pkg add`/`update` (và kiểm “không nhận” cho `install`/`list`).
pub(crate) struct TuyChon {
    pub(crate) ten: Option<String>,
    pub(crate) reg: Option<String>,
    pub(crate) entry: Option<String>,
    /// **VS3-5b/5c**: khoá công khai Ed25519 (mỗi khoá 64 hex) cho registry HTTP.
    /// Lặp `--registry-key` để hỗ trợ **key rotation**; khoá cũ vẫn verify bản cũ.
    pub(crate) khoa: Vec<String>,
}

pub(crate) fn tuy_chon(args: &[String], cho_registry: bool) -> Result<TuyChon, String> {
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
pub(crate) enum Registry {
    CucBo(PathBuf),
    Http(String),
}

/// Chọn nguồn từ `--registry` / `VIS_REGISTRY`; `http://`/`https://` ⇒ registry HTTP(S), còn lại là
/// **đường dẫn thư mục** như trước. `http://` vẫn chỉ loopback (do `http_get`); `https://` host bất
/// kỳ nhưng vẫn qua `cap[Net]`.
pub(crate) fn registry_tu_chon(chon: Option<String>) -> Result<Registry, String> {
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
pub(crate) fn khoa_cong_tu_chon(chon: &[String], reg: &Registry) -> Result<Vec<[u8; 32]>, String> {
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
            let b = crate::pkg_sig::hex_giai(phan, 32).map_err(|e| {
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
pub(crate) fn nap_goi(
    reg: &Registry,
    roots: &BTreeMap<String, VersionReq>,
    entry_override: Option<(&str, &str)>,
    khoa_cong: &[[u8; 32]],
    uu_tien: &BTreeMap<String, Version>,
    net: &mut dyn vis_core::host::NetHost,
) -> Result<(BTreeMap<String, Version>, Vec<GoiCai>), String> {
    match reg {
        Registry::CucBo(p) => giai_va_doc(p, roots, entry_override, uu_tien),
        Registry::Http(base) => {
            registry_http::tai_va_giai(base, roots, entry_override, khoa_cong, uu_tien, net)
        }
    }
}

pub(crate) fn cwd() -> Result<PathBuf, String> {
    std::env::current_dir().map_err(|e| format!("không lấy được thư mục hiện tại: {e}"))
}
