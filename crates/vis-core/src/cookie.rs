// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-4b (VD-271)** — cookie jar **nội bộ** của HTTP client.
//!
//! Không phải API ngôn ngữ: `http.get/head` dùng qua [`crate::host::NetHost::cookie_jar`]; mỗi
//! host giữ một jar cho một tiến trình. Chỉ hỗ trợ phần tối thiểu của RFC 6265:
//! `name=value`, `Path`, `Domain`, `Secure`, `HttpOnly`, `Max-Age`, `Expires` (IMF-fixdate).
//! Không cookie store ra đĩa, không API JS, không gửi cross-host (domain-match RFC).
//!
//! Bảo vệ: tên/giá trị có ký tự điều khiển hoặc `;` bị bỏ; tối đa 64 cookie; header
//! `Set-Cookie`/`Cookie` không bao giờ đi vào log của `Net` (chỉ tầng HTTP thấy).

use std::net::IpAddr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Trần số cookie một jar giữ (tránh server làm phình bộ nhớ).
const TOI_DA: usize = 64;
/// Trần tên+giá trị (byte) của một cookie.
const DAI_TOI_DA: usize = 4096;

#[derive(Debug, Clone)]
struct Cookie {
    name: String,
    value: String,
    /// Domain đã hạ chữ thường, **không** có dấu `.` đầu.
    domain: String,
    path: String,
    secure: bool,
    /// Chỉ để kiểm/ghi nhận; client không có JS nên không ảnh hưởng gửi đi.
    #[allow(dead_code)]
    http_only: bool,
    expires: Option<SystemTime>,
}

/// Cookie jar của một host HTTP; mặc định rỗng.
#[derive(Debug, Default)]
pub struct CookieJar {
    ds: Vec<Cookie>,
}

impl CookieJar {
    pub fn new() -> Self {
        Self::default()
    }

    /// Giá trị header `Cookie` cho request tới `host`/`path` (`secure` = URL https), hoặc `None`.
    pub fn header(&self, host: &str, path: &str, secure: bool) -> Option<String> {
        let host = chuan_host(host);
        let path = duong(path);
        let now = SystemTime::now();
        let mut ds: Vec<&Cookie> = self
            .ds
            .iter()
            .filter(|c| {
                (!c.secure || secure)
                    && c.expires.is_none_or(|t| t > now)
                    && domain_khop(&host, &c.domain)
                    && path_khop(path, &c.path)
            })
            .collect();
        // RFC 6265 §5.4: path dài hơn trước; ổn định theo tên cho kết quả tất định.
        ds.sort_by(|a, b| b.path.len().cmp(&a.path.len()).then(a.name.cmp(&b.name)));
        if ds.is_empty() {
            None
        } else {
            Some(
                ds.iter()
                    .map(|c| format!("{}={}", c.name, c.value))
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        }
    }

    /// Nhận các header `Set-Cookie` của response cho `host`/`request_path`.
    pub fn nhan_set_cookie(&mut self, host: &str, request_path: &str, set_cookies: &[String]) {
        let host = chuan_host(host);
        let path = duong(request_path).to_string();
        for raw in set_cookies {
            self.nhan_mot(&host, &path, raw);
        }
        while self.ds.len() > TOI_DA {
            self.ds.remove(0);
        }
    }

    fn nhan_mot(&mut self, host: &str, request_path: &str, raw: &str) {
        let mut seg = raw.split(';');
        let Some(cap) = seg.next() else { return };
        let Some((name, value)) = cap.split_once('=') else {
            return;
        };
        let name = name.trim();
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .unwrap_or(value);
        if !ten_hop_le(name) || !gia_tri_hop_le(value) || name.len() + value.len() > DAI_TOI_DA {
            return;
        }

        let mut domain: Option<String> = None;
        let mut path: Option<String> = None;
        let mut secure = false;
        let mut http_only = false;
        let mut expires: Option<SystemTime> = None;
        let mut max_age: Option<i64> = None;
        for attr in seg {
            let (k, v) = attr.split_once('=').unwrap_or((attr, ""));
            match k.trim().to_ascii_lowercase().as_str() {
                "domain" => {
                    let d = v
                        .trim()
                        .trim_start_matches('.')
                        .trim_end_matches('.')
                        .to_ascii_lowercase();
                    if !d.is_empty() {
                        domain = Some(d);
                    }
                }
                "path" => {
                    let p = v.trim();
                    if p.starts_with('/') {
                        path = Some(p.to_string());
                    }
                }
                "secure" => secure = true,
                "httponly" => http_only = true,
                "max-age" => {
                    if let Ok(n) = v.trim().parse::<i64>() {
                        max_age = Some(n);
                    }
                }
                "expires" => {
                    if let Some(t) = doc_http_date(v.trim()) {
                        expires = Some(t);
                    }
                }
                _ => {}
            }
        }

        let domain = match domain {
            Some(d) => {
                if !domain_khop(host, &d) {
                    return;
                }
                d
            }
            None => host.to_string(),
        };
        let cookie_path = path.unwrap_or_else(|| duong_mac_dinh(request_path));
        let now = SystemTime::now();

        // Max-Age thắng Expires (RFC 6265 §5.3); <= 0 là xoá cookie cùng (domain,path,name).
        let het_han = if let Some(n) = max_age {
            if n <= 0 {
                true
            } else {
                expires = now.checked_add(Duration::from_secs(n as u64));
                false
            }
        } else {
            expires.is_some_and(|t| t <= now)
        };
        let moi = Cookie {
            name: name.to_string(),
            value: value.to_string(),
            domain,
            path: cookie_path,
            secure,
            http_only,
            expires,
        };
        if het_han {
            self.ds
                .retain(|c| !(c.name == moi.name && c.domain == moi.domain && c.path == moi.path));
            return;
        }
        if let Some(i) = self
            .ds
            .iter()
            .position(|c| c.name == moi.name && c.domain == moi.domain && c.path == moi.path)
        {
            self.ds[i] = moi;
        } else {
            self.ds.push(moi);
        }
    }
}

/// Host so cookie: hạ chữ thường, bỏ dấu `.` cuối (FQDN).
fn chuan_host(host: &str) -> String {
    host.trim().trim_end_matches('.').to_ascii_lowercase()
}

/// Path để so/đặt cookie: bỏ query/fragment, mặc định `/`.
fn duong(path: &str) -> &str {
    let p = path.split(['?', '#']).next().unwrap_or("/");
    if p.is_empty() {
        "/"
    } else {
        p
    }
}

/// Default-path RFC 6265 §5.1.4.
fn duong_mac_dinh(path: &str) -> String {
    let p = duong(path);
    if !p.starts_with('/') {
        return "/".to_string();
    }
    match p.rfind('/') {
        Some(0) | None => "/".to_string(),
        Some(i) => p[..i].to_string(),
    }
}

/// Domain-match RFC 6265 §5.1.3; IP chỉ khớp chính xác, không cho hậu tố lạ.
fn domain_khop(host: &str, domain: &str) -> bool {
    if domain.is_empty() {
        return false;
    }
    if host == domain {
        return true;
    }
    if host.parse::<IpAddr>().is_ok() || domain.parse::<IpAddr>().is_ok() {
        return false;
    }
    host.len() > domain.len()
        && host.ends_with(domain)
        && host.as_bytes()[host.len() - domain.len() - 1] == b'.'
}

/// Path-match RFC 6265 §5.1.4.
fn path_khop(req: &str, cookie: &str) -> bool {
    if req == cookie {
        return true;
    }
    if !req.starts_with(cookie) {
        return false;
    }
    cookie.ends_with('/') || req.as_bytes().get(cookie.len()) == Some(&b'/')
}

fn ten_hop_le(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b > 0x20 && b < 0x7f && b != b'=' && b != b';')
}

fn gia_tri_hop_le(value: &str) -> bool {
    value
        .bytes()
        .all(|b| b >= 0x20 && b != 0x7f && b != b';' && b != b'\r' && b != b'\n')
}

/// Parse `Expires` dạng IMF-fixdate: `Wed, 21 Oct 2015 07:28:00 GMT`.
fn doc_http_date(s: &str) -> Option<SystemTime> {
    let mut p = s.split_whitespace();
    let _thu = p.next()?;
    let ngay = p.next()?.trim_end_matches(',').parse::<u32>().ok()?;
    let thang = thang_so(p.next()?)?;
    let nam = p.next()?.parse::<i32>().ok()?;
    let gio = p.next()?;
    let mut t = gio.split(':');
    let h = t.next()?.parse::<u32>().ok()?;
    let m = t.next()?.parse::<u32>().ok()?;
    let giay = t.next()?.parse::<u32>().ok()?;
    if ngay == 0 || ngay > 31 || h > 23 || m > 59 || giay > 60 {
        return None;
    }
    let giay_unix = ngay_den_giay(nam, thang, ngay)? * 86_400
        + i64::from(h) * 3600
        + i64::from(m) * 60
        + i64::from(giay);
    if giay_unix < 0 {
        return None;
    }
    UNIX_EPOCH.checked_add(Duration::from_secs(giay_unix as u64))
}

fn thang_so(s: &str) -> Option<u32> {
    Some(match s.to_ascii_lowercase().as_str() {
        "jan" => 1,
        "feb" => 2,
        "mar" => 3,
        "apr" => 4,
        "may" => 5,
        "jun" => 6,
        "jul" => 7,
        "aug" => 8,
        "sep" => 9,
        "oct" => 10,
        "nov" => 11,
        "dec" => 12,
        _ => return None,
    })
}

/// Số ngày từ 1970-01-01 tới `y-m-d` (thuật toán days_from_civil, không phụ thuộc crate).
fn ngay_den_giay(y: i32, m: u32, d: u32) -> Option<i64> {
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let d = d as i32;
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = ((m + 9) % 12) as i32;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(i64::from(era) * 146_097 + i64::from(doe) - 719_468)
}

#[cfg(test)]
mod tests {
    use super::CookieJar;

    fn set(j: &mut CookieJar, host: &str, path: &str, raw: &str) {
        j.nhan_set_cookie(host, path, &[raw.to_string()]);
    }

    #[test]
    fn path_secure_va_http_only() {
        let mut j = CookieJar::new();
        set(
            &mut j,
            "example.com",
            "/a/b",
            "sid=abc; Path=/a; Secure; HttpOnly",
        );
        assert_eq!(
            j.header("example.com", "/a/c", true).as_deref(),
            Some("sid=abc")
        );
        assert_eq!(
            j.header("example.com", "/a/c", false),
            None,
            "Secure không gửi http"
        );
        assert_eq!(j.header("example.com", "/b", true), None, "Path không khớp");
        assert_eq!(j.header("other.example", "/a", true), None);
        assert!(j.ds[0].http_only);
    }

    #[test]
    fn domain_va_default_path() {
        let mut j = CookieJar::new();
        set(
            &mut j,
            "sub.example.com",
            "/x/y",
            "d=1; Domain=.example.com",
        );
        set(&mut j, "example.com", "/", "root=1");
        assert_eq!(
            j.header("sub.example.com", "/x", true).as_deref(),
            Some("d=1; root=1")
        );
        assert_eq!(j.header("other.com", "/x", true), None);
        assert_eq!(j.header("badexample.com", "/x", true), None);
        // Default path của `/x/y` là `/x`; request `/x` khớp, `/y` không.
        let mut k = CookieJar::new();
        set(&mut k, "example.com", "/x/y", "p=1");
        assert_eq!(k.header("example.com", "/x", true).as_deref(), Some("p=1"));
        assert_eq!(k.header("example.com", "/y", true), None);
    }

    #[test]
    fn het_han_max_age_va_expires() {
        let mut j = CookieJar::new();
        set(&mut j, "example.com", "/", "a=1; Max-Age=0");
        assert_eq!(
            j.header("example.com", "/", true),
            None,
            "Max-Age=0 không lưu"
        );

        set(&mut j, "example.com", "/", "b=1");
        assert!(j.header("example.com", "/", true).is_some());
        set(&mut j, "example.com", "/", "b=2; Max-Age=0");
        assert_eq!(j.header("example.com", "/", true), None, "Max-Age=0 xoá");

        set(
            &mut j,
            "example.com",
            "/",
            "c=1; Expires=Thu, 01 Jan 1970 00:00:00 GMT",
        );
        assert_eq!(j.header("example.com", "/", true), None, "Expires quá khứ");
        set(
            &mut j,
            "example.com",
            "/",
            "d=1; Expires=Thu, 01 Jan 2100 00:00:00 GMT",
        );
        assert!(
            !j.ds.is_empty(),
            "cookie tương lai phải được lưu: {:?}",
            j.ds
        );
        assert!(
            j.header("example.com", "/", true).is_some(),
            "Expires tương lai"
        );
    }

    #[test]
    fn doc_http_date_co_ban() {
        let t = super::doc_http_date("Thu, 01 Jan 2100 00:00:00 GMT").unwrap();
        assert!(t > std::time::SystemTime::now());
        assert!(super::doc_http_date("Thu, 01 Jan 1970 00:00:00 GMT").is_some());
        assert!(super::doc_http_date("khong-phai-ngay").is_none());
    }

    #[test]
    fn tu_choi_domain_la_va_ky_tu_dieu_khien() {
        let mut j = CookieJar::new();
        set(&mut j, "example.com", "/", "x=1; Domain=other.com");
        assert_eq!(j.header("example.com", "/", true), None);
        set(&mut j, "example.com", "/", "x=1; Domain=example.com");
        assert!(j.header("example.com", "/", true).is_some());
        set(&mut j, "example.com", "/", "bad=1\r\nInjected: x");
        assert_eq!(j.header("example.com", "/", true).as_deref(), Some("x=1"));
        set(&mut j, "example.com", "/", "=no-name; Path=/");
        set(&mut j, "example.com", "/", "no-eq");
        assert!(!j
            .header("example.com", "/", true)
            .unwrap()
            .contains("no-name"));
    }
}
