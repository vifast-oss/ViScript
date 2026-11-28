// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-323b** — tách nguyên văn từ `lib.rs`: request/redirect/error helpers (R1 headroom).

use crate::header::DauHttp;
use crate::url::{tach_url, GiaoThuc, UrlHttp};

/// Chuẩn hoá path+query: bỏ `.`/`..`, giữ `/` đầu và query; `..` vượt gốc bị kẹp.
pub(crate) fn chuan_hoa_target(target: &str) -> String {
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let trailing = path.ends_with('/') && path != "/";
    let mut seg: Vec<&str> = Vec::new();
    for p in path.split('/') {
        match p {
            "" | "." => {}
            ".." => {
                seg.pop();
            }
            x => seg.push(x),
        }
    }
    let mut out = format!("/{}", seg.join("/"));
    if trailing && out != "/" {
        out.push('/');
    }
    if !query.is_empty() {
        out.push('?');
        out.push_str(query);
    }
    out
}

/// Ghép `Location` với URL hiện tại (RFC 3986 mức đơn giản). `None` = **không theo** (rỗng/fragment
/// thuần, hoặc URL sau khi chuẩn hoá **trùng** URL cũ ⇒ tránh vòng). `Err` = `Location` không hỗ trợ,
/// hoặc **hạ cấp** HTTPS → HTTP (VS3-4a, Q2 (3)).
#[doc(hidden)]
pub fn noi_location(u: &UrlHttp, location: &str) -> Result<Option<String>, String> {
    if location.starts_with("http://") || location.starts_with("https://") {
        if u.giao_thuc == GiaoThuc::Https && location.starts_with("http://") {
            return Err("không cho phép chuyển hướng HTTPS → HTTP".to_string());
        }
        let moi = tach_url(location)?;
        return Ok(Some(moi.url_voi_target(&chuan_hoa_target(&moi.target))));
    }
    // Fragment không gửi lên server; `#f` một mình ⇒ cùng URL ⇒ không theo.
    let loc = location.split('#').next().unwrap_or("");
    if loc.is_empty() {
        return Ok(None);
    }
    let moi = if loc.starts_with('?') {
        // Query-only: giữ **path cũ**, thay query.
        let path = u.target.split('?').next().unwrap_or("/");
        format!("{path}{loc}")
    } else if loc.starts_with('/') {
        chuan_hoa_target(loc)
    } else {
        // Tương đối theo **thư mục** của target hiện tại.
        let path = u.target.split('?').next().unwrap_or("/");
        let thu_muc = path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        chuan_hoa_target(&format!("{thu_muc}/{loc}"))
    };
    Ok(Some(u.url_voi_target(&moi)))
}

/// Lỗi tầng HTTP kèm cờ **có nên thử lại một lần** trên kết nối mới (chỉ khi lỗi đến từ kết nối
/// idle đã bị server đóng, không phải lỗi cú pháp/timeout).
pub(crate) struct LoiHttp {
    pub(crate) thong_diep: String,
    pub(crate) retry_duoc: bool,
}

impl LoiHttp {
    pub(crate) fn khong_thu_lai(s: impl Into<String>) -> Self {
        Self {
            thong_diep: s.into(),
            retry_duoc: false,
        }
    }

    pub(crate) fn ket_noi(s: impl Into<String>) -> Self {
        Self {
            thong_diep: s.into(),
            retry_duoc: true,
        }
    }
}

impl From<String> for LoiHttp {
    fn from(s: String) -> Self {
        Self::khong_thu_lai(s)
    }
}

/// Lỗi `read` do hết giờ **không** thử lại (đã dùng hết hạn); lỗi kết nối khác thì thử lại được.
pub(crate) fn loi_doc(e: String) -> LoiHttp {
    if e.contains("quá thời gian") {
        LoiHttp::khong_thu_lai(e)
    } else {
        LoiHttp::ket_noi(e)
    }
}

/// Kết quả một response GET: `(status, Location, body, giữ_kết_nối, set-cookie)`.
pub(crate) type KetQuaGet = (u16, Option<String>, String, bool, Vec<String>);
/// Kết quả một response HEAD: `(status, headers, giữ_kết_nối)`.
pub(crate) type KetQuaHead = (u16, Vec<(String, String)>, bool);

/// Header phụ của request (không log): Basic auth + Cookie.
#[derive(Clone, Copy, Default)]
pub(crate) struct HeaderThem<'a> {
    pub(crate) auth: Option<&'a str>,
    pub(crate) cookie: Option<&'a str>,
}

/// Ghép request HTTP/1.1; `auth`/`cookie` chỉ là giá trị header (không log).
pub(crate) fn yeu_cau_http(
    method: &str,
    target: &str,
    authority: &str,
    them: HeaderThem<'_>,
) -> String {
    let mut req =
        format!("{method} {target} HTTP/1.1\r\nHost: {authority}\r\nConnection: keep-alive\r\n");
    if let Some(a) = them.auth {
        req.push_str("Authorization: ");
        req.push_str(a);
        req.push_str("\r\n");
    }
    if let Some(c) = them.cookie {
        req.push_str("Cookie: ");
        req.push_str(c);
        req.push_str("\r\n");
    }
    req.push_str("\r\n");
    req
}

pub(crate) fn set_cookie_cua(d: &DauHttp) -> Vec<String> {
    d.headers
        .iter()
        .filter(|(k, _)| k == "set-cookie")
        .map(|(_, v)| v.clone())
        .collect()
}

/// Header `(tên, giá trị)` gửi kèm h2 (đã hạ tên; giá trị không log).
#[doc(hidden)]
pub fn header_h2(auth: Option<&str>, cookie: Option<&str>) -> Vec<(String, String)> {
    let mut hs = Vec::new();
    if let Some(a) = auth {
        hs.push(("authorization".to_string(), a.to_string()));
    }
    if let Some(c) = cookie {
        hs.push(("cookie".to_string(), c.to_string()));
    }
    hs
}

pub(crate) fn header_gia_tri(headers: &[(String, String)], ten: &str) -> Option<String> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(ten))
        .map(|(_, v)| v.clone())
}
