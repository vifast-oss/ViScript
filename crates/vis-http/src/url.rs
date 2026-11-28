// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-323b** — tách nguyên văn từ `lib.rs`: URL/authority/Basic auth (R1 headroom).

use std::net::IpAddr;

/// Giao thức của URL — quyết định cổng mặc định + đường nối (`connect` hay `connect_tls`).
#[doc(hidden)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GiaoThuc {
    Http,
    Https,
}

impl GiaoThuc {
    #[doc(hidden)]
    pub fn ten(self) -> &'static str {
        match self {
            GiaoThuc::Http => "http",
            GiaoThuc::Https => "https",
        }
    }

    fn cong_mac_dinh(self) -> u16 {
        match self {
            GiaoThuc::Http => 80,
            GiaoThuc::Https => 443,
        }
    }
}

/// URL đã tách: `scheme://[user:pass@]host[:port][/path][?query]`.
#[doc(hidden)]
#[derive(Debug)]
pub struct UrlHttp {
    pub giao_thuc: GiaoThuc,
    /// `user:pass` (chưa base64) nếu URL có userinfo; không bao giờ đưa vào log/`Host`.
    pub userinfo: Option<(String, String)>,
    /// Host **không** kèm `[]` của IPv6 (dùng làm `ServerName` cho TLS).
    pub host: String,
    pub cong: u16,
    pub target: String,
}

impl UrlHttp {
    /// `host[:port]` cho `Host:` và `cap[Net]`; bỏ cổng mặc định, IPv6 bọc `[]`.
    #[doc(hidden)]
    pub fn authority(&self) -> String {
        let host = if self.host.contains(':') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        if self.cong == self.giao_thuc.cong_mac_dinh() {
            host
        } else {
            format!("{host}:{}", self.cong)
        }
    }

    /// `host:port` **luôn có cổng** — dạng `connect`/`connect_tls` cần để phân giải.
    #[doc(hidden)]
    pub fn dia_chi(&self) -> String {
        let host = if self.host.contains(':') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        format!("{host}:{}", self.cong)
    }

    /// Ghép URL với target cho trước, giữ userinfo nếu có (redirect tương đối cùng origin).
    pub(crate) fn url_voi_target(&self, target: &str) -> String {
        let auth = self
            .userinfo
            .as_ref()
            .map(|(u, p)| format!("{u}:{p}@"))
            .unwrap_or_default();
        format!(
            "{}://{auth}{}{target}",
            self.giao_thuc.ten(),
            self.authority()
        )
    }

    /// Header `Authorization: Basic …` nếu URL có `user:pass@`.
    #[doc(hidden)]
    pub fn authorization(&self) -> Option<String> {
        let (u, p) = self.userinfo.as_ref()?;
        Some(format!("Basic {}", ma_b64(format!("{u}:{p}").as_bytes())))
    }

    #[cfg(test)]
    pub(crate) fn url(&self) -> String {
        self.url_voi_target(&self.target)
    }
}

/// Base64 chuẩn (RFC 4648) cho Basic auth — cố định, không cần crate.
fn ma_b64(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let b = [c[0], *c.get(1).unwrap_or(&0), *c.get(2).unwrap_or(&0)];
        out.push(A[(b[0] >> 2) as usize] as char);
        out.push(A[(((b[0] & 0b11) << 4) | (b[1] >> 4)) as usize] as char);
        out.push(if c.len() > 1 {
            A[(((b[1] & 0b1111) << 2) | (b[2] >> 6)) as usize] as char
        } else {
            '='
        });
        out.push(if c.len() > 2 {
            A[(b[2] & 0b111111) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// URL an toàn để đưa vào thông điệp lỗi: thay userinfo bằng `***`.
fn an_toan(url: &str) -> String {
    let Some((truoc, sau)) = url.rsplit_once('@') else {
        return url.to_string();
    };
    match truoc.split_once("://") {
        Some((scheme, _)) => format!("{scheme}://***@{sau}"),
        None => "***".to_string(),
    }
}

/// Tách authority `host[:port]`, hỗ trợ IPv6 `[::1]:443`; cổng thiếu ⇒ `mac_dinh`.
fn tach_authority(authority: &str, mac_dinh: u16) -> Result<(String, u16), String> {
    let (host, cong) = if let Some(rest) = authority.strip_prefix('[') {
        let (h, sau) = rest
            .split_once(']')
            .ok_or_else(|| format!("authority IPv6 thiếu `]`: `{authority}`"))?;
        let c = if sau.is_empty() {
            None
        } else {
            Some(
                sau.strip_prefix(':')
                    .ok_or_else(|| format!("authority IPv6 sai dạng: `{authority}`"))?,
            )
        };
        (h, c)
    } else if authority.matches(':').count() == 1 {
        let (h, c) = authority.rsplit_once(':').expect("đã kiểm một dấu `:`");
        (h, Some(c))
    } else {
        // Không có cổng (hoặc IPv6 trần — URL chuẩn phải bọc `[]`).
        (authority, None)
    };
    if host.is_empty() {
        return Err(format!("URL thiếu host: `{authority}`"));
    }
    let cong = match cong {
        Some(c) => c
            .parse::<u16>()
            .map_err(|_| format!("cổng `{c}` không hợp lệ (0..65535)"))?,
        None => mac_dinh,
    };
    Ok((host.to_string(), cong))
}

/// Host có phải loopback cho `http://`: `localhost`, `127.0.0.0/8`, `::1`.
fn la_host_loopback(host: &str) -> bool {
    host == "localhost"
        || host
            .parse::<IpAddr>()
            .map(|ip| ip.is_loopback())
            .unwrap_or(false)
}

/// Tách `http|https://host[:port][/path][?query]`.
///
/// `http://` **chỉ loopback** (`localhost`/`127.0.0.0/8`/`::1`, giữ nguyên VD-100/105); `https://`
/// cho host bất kỳ (TLS + `cap[Net]`).
#[doc(hidden)]
pub fn tach_url(url: &str) -> Result<UrlHttp, String> {
    // **ST-314a (W3 H1)**: URL chứa CR/LF/CTL từng chèn được header/request thứ hai vào
    // `request-target` (CRLF injection / request smuggling). Từ chối sớm toàn bộ CTL.
    if url.bytes().any(|b| b < 0x20 || b == 0x7f) {
        return Err("URL chứa ký tự điều khiển (CR/LF/CTL) — từ chối".to_string());
    }
    let (giao_thuc, rest) = if let Some(r) = url.strip_prefix("http://") {
        (GiaoThuc::Http, r)
    } else if let Some(r) = url.strip_prefix("https://") {
        (GiaoThuc::Https, r)
    } else {
        return Err(format!(
            "chỉ hỗ trợ `http://` hoặc `https://`, được `{}`",
            an_toan(url)
        ));
    };
    let i = rest.find(['/', '?']).unwrap_or(rest.len());
    let (authority, duong) = (&rest[..i], &rest[i..]);
    let (userinfo, authority) = match authority.rsplit_once('@') {
        Some((u, host)) => (Some(u), host),
        None => (None, authority),
    };
    if authority.is_empty() {
        return Err(format!("URL thiếu host: `{}`", an_toan(url)));
    }
    let userinfo = match userinfo {
        Some(u) => {
            let Some((ten, mat_khau)) = u.split_once(':') else {
                return Err("URL userinfo phải dạng `user:pass@host` (Basic auth)".to_string());
            };
            if ten.bytes().any(|b| b < 0x20 || b == 0x7f)
                || mat_khau.bytes().any(|b| b < 0x20 || b == 0x7f)
            {
                return Err("URL userinfo chứa ký tự điều khiển".to_string());
            }
            Some((ten.to_string(), mat_khau.to_string()))
        }
        None => None,
    };
    let (host, cong) = tach_authority(authority, giao_thuc.cong_mac_dinh())?;
    if giao_thuc == GiaoThuc::Http && !la_host_loopback(&host) {
        return Err(format!(
            "`http://` chỉ cho phép loopback (`localhost`/`127.0.0.0/8`/`::1`), được `{host}`; dùng `https://` cho host ngoài"
        ));
    }
    let target = if duong.is_empty() {
        "/".to_string()
    } else if duong.starts_with('/') {
        duong.to_string()
    } else {
        format!("/{duong}")
    };
    Ok(UrlHttp {
        giao_thuc,
        userinfo,
        host,
        cong,
        target,
    })
}
