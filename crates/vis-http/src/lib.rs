// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-64 (VD-100/VD-105)** — **giao thức HTTP client** nằm trong `vis-interp` (không ở CLI):
//! socket thô qua [`NetHost`], tự dựng/đọc **HTTP/1.1**. Nhờ vậy interp ↔ VM **cùng byte** và codegen
//! **không** phải sửa (`std.http.*` hạ theo tên như mọi `std.*`).
//!
//! Phạm vi: `http://` **chỉ loopback** (`localhost`/`127.0.0.0/8`/`::1`, giữ nguyên VD-100/105) và
//! **VS3-4a** `https://` qua `NetHost::connect_tls` (TLS nằm ở host, không ở crate lõi); **không**
//! HTTP/2; GET theo redirect tối đa 5 (chỉ GET), HEAD **không** theo redirect; phản hồi cap
//! **8 MiB** (mẫu `MAX_STDIN`, ST-34) ⇒ vượt cap là `Err`, không OOM. Body giữ **byte** tới khi
//! chuyển `str` (lossy) — ngôn ngữ chưa có kiểu byte. **Chunk trailer được đọc hết nhưng bỏ nội dung**.
//!
//! **VS3-4a (VD-271)**: thêm `https://` — host TLS xác thực chứng chỉ theo root store; `https://`
//! được phép host ngoài loopback (đi qua `cap[Net]` như mọi host khác), còn `http://` **vẫn** chỉ
//! loopback để không mở thêm bề mặt trần. Không cho chuyển hướng **HTTPS → HTTP** (Q2 (3)).
//!
//! **VS3-4b (VD-271)**: HTTP/1.1 **keep-alive** — request gửi `Connection: keep-alive`; response
//! cho phép (HTTP/1.1 không `close`, hoặc HTTP/1.0 + `keep-alive`) và thân đã đọc hết ⇒ trả kết nối
//! về host (`NetHost::tra_http`) để tái dùng; server `close`/EOF/khung lạ ⇒ đóng. Host không có pool
//! (mặc định của trait) vẫn one-shot. Kết nối idle chết được thử lại **một lần** trên kết nối mới
//! (GET/HEAD idempotent), trong cùng deadline.

use std::net::IpAddr;
use std::time::{Duration, Instant};

use vis_core::host::NetHost;

/// Cap **phản hồi** (8 MiB, mẫu `MAX_STDIN` của ST-34) — vượt ⇒ `Err`, không OOM.
pub const MAX_HTTP: usize = 8 * 1024 * 1024;
/// Cap **header** (64 KiB) — header lớn hơn ⇒ `Err` (không đọc vô hạn).
#[doc(hidden)]
pub const MAX_HEADER: usize = 64 * 1024;

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
    fn url_voi_target(&self, target: &str) -> String {
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
    fn url(&self) -> String {
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

/// Tìm `\r\n\r\n` trong `du` — vị trí **byte đầu** của dấu phân cách.
#[doc(hidden)]
pub fn tim_header(du: &[u8]) -> Option<usize> {
    du.windows(4).position(|w| w == b"\r\n\r\n")
}

/// **ST-265 (VD-250, #148)**: số ms **còn lại** tới deadline monotonic `han`; `None` khi đã hết
/// hạn. Mọi chờ chặn (connect, mỗi lần read) chỉ được dùng phần thời gian **còn lại** này — deadline
/// được tính **một lần** ở đầu `http_get`/`http_head` và **không** đặt lại cho mỗi hop redirect.
/// Luôn trả ≥ 1 khi còn hạn (tránh truyền 0 cho `set_read_timeout`/`connect_timeout`).
#[doc(hidden)]
pub fn con_lai_ms(han: Instant) -> Option<u64> {
    let con = han.saturating_duration_since(Instant::now());
    if con.is_zero() {
        None
    } else {
        Some(con.as_millis().clamp(1, u64::MAX as u128) as u64)
    }
}

/// Thông tin header cần cho giao thức.
#[derive(Default)]
#[doc(hidden)]
pub struct DauHttp {
    pub status: u16,
    pub cl: Option<usize>,
    pub chunked: bool,
    pub location: Option<String>,
    /// **VS3-4b**: response cho phép giữ kết nối? (HTTP/1.1 mặc định có, `Connection: close` tắt;
    /// HTTP/1.0 chỉ khi `Connection: keep-alive`).
    pub giu_ket_noi: bool,
    /// Header đã hạ **chữ thường**, giữ thứ tự; dùng cho `http.head`.
    pub headers: Vec<(String, String)>,
}

/// Đọc status + `Content-Length` + `Transfer-Encoding` + `Location` + `Connection` từ khối header
/// (không gồm `\r\n\r\n`).
#[doc(hidden)]
pub fn doc_header_day_du(dau: &[u8]) -> Result<DauHttp, String> {
    let text = String::from_utf8_lossy(dau);
    let mut lines = text.split("\r\n");
    let status_line = lines.next().unwrap_or("");
    let mut phan = status_line.split_whitespace();
    let phien_ban = phan.next().unwrap_or("");
    let status: u16 = phan
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| format!("status line HTTP không hợp lệ: `{status_line}`"))?;
    let mut out = DauHttp {
        status,
        ..Default::default()
    };
    let mut ket_noi = String::new();
    for l in lines {
        if let Some((k, v)) = l.split_once(':') {
            let k = k.trim();
            let v = v.trim();
            if k.eq_ignore_ascii_case("content-length") {
                out.cl = Some(
                    v.parse::<usize>()
                        .map_err(|_| format!("Content-Length không hợp lệ: `{v}`"))?,
                );
            } else if k.eq_ignore_ascii_case("transfer-encoding") {
                out.chunked = v
                    .split(',')
                    .any(|x| x.trim().eq_ignore_ascii_case("chunked"));
            } else if k.eq_ignore_ascii_case("location") {
                out.location = Some(v.to_string());
            } else if k.eq_ignore_ascii_case("connection") {
                ket_noi = v.to_ascii_lowercase();
            }
            out.headers.push((k.to_ascii_lowercase(), v.to_string()));
        }
    }
    let dong = ket_noi.split(',').any(|x| x.trim() == "close");
    let giu = ket_noi.split(',').any(|x| x.trim() == "keep-alive");
    out.giu_ket_noi = if phien_ban.eq_ignore_ascii_case("HTTP/1.1") {
        !dong
    } else {
        giu
    };
    // 101 Switching Protocols không phải HTTP/1.1 thường ⇒ không tái dùng.
    if out.status == 101 {
        out.giu_ket_noi = false;
    }
    Ok(out)
}

/// Giữ API cũ cho unit test: `(status, Content-Length)`.
#[cfg(test)]
fn doc_header(dau: &[u8]) -> Result<(u16, Option<usize>), String> {
    let d = doc_header_day_du(dau)?;
    Ok((d.status, d.cl))
}

/// Ghép thân **chunked** (RFC 7230 §4.1). `None` = còn thiếu byte; `Err` = cú pháp/chunk quá cap.
/// Hỗ trợ **chunk mở rộng** `<hex>;ext=1\r\n` (bỏ phần sau `;`); chunk `0` kết thúc, **bỏ trailer**
/// nhưng đọc hết trailer để kết nối có thể tái dùng. Trả `(body, số byte đã dùng)`.
#[doc(hidden)]
pub fn ghep_chunked(du: &[u8]) -> Result<Option<(Vec<u8>, usize)>, String> {
    let mut i = 0usize;
    let mut out = Vec::new();
    loop {
        let Some(pos) = du[i..].windows(2).position(|w| w == b"\r\n") else {
            return Ok(None);
        };
        let line = &du[i..i + pos];
        let size_txt = line
            .split(|b| *b == b';')
            .next()
            .unwrap_or(line)
            .trim_ascii();
        let n = usize::from_str_radix(
            std::str::from_utf8(size_txt).map_err(|_| "chunk size không phải ASCII".to_string())?,
            16,
        )
        .map_err(|_| {
            format!(
                "chunk size không hợp lệ: `{}`",
                String::from_utf8_lossy(size_txt)
            )
        })?;
        i += pos + 2;
        if n == 0 {
            // Trailer: 0+ dòng header rồi dòng trống; bỏ nội dung, nhưng phải đọc hết.
            let bat_dau = i;
            loop {
                let Some(pos) = du[i..].windows(2).position(|w| w == b"\r\n") else {
                    return Ok(None);
                };
                let dong = &du[i..i + pos];
                i += pos + 2;
                if dong.is_empty() {
                    return Ok(Some((out, i)));
                }
                if i - bat_dau > MAX_HEADER {
                    return Err(format!("chunk trailer quá lớn (> {MAX_HEADER} byte)"));
                }
            }
        }
        if out.len().saturating_add(n) > MAX_HTTP {
            return Err(format!("phản hồi vượt giới hạn {MAX_HTTP} byte (8 MiB)"));
        }
        if du.len() < i + n + 2 {
            return Ok(None);
        }
        out.extend_from_slice(&du[i..i + n]);
        i += n;
        if &du[i..i + 2] != b"\r\n" {
            return Err("chunk thiếu `\\r\\n` sau dữ liệu".to_string());
        }
        i += 2;
    }
}

/// Chuẩn hoá path+query: bỏ `.`/`..`, giữ `/` đầu và query; `..` vượt gốc bị kẹp.
fn chuan_hoa_target(target: &str) -> String {
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
struct LoiHttp {
    thong_diep: String,
    retry_duoc: bool,
}

impl LoiHttp {
    fn khong_thu_lai(s: impl Into<String>) -> Self {
        Self {
            thong_diep: s.into(),
            retry_duoc: false,
        }
    }

    fn ket_noi(s: impl Into<String>) -> Self {
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
fn loi_doc(e: String) -> LoiHttp {
    if e.contains("quá thời gian") {
        LoiHttp::khong_thu_lai(e)
    } else {
        LoiHttp::ket_noi(e)
    }
}

/// Kết quả một response GET: `(status, Location, body, giữ_kết_nối, set-cookie)`.
type KetQuaGet = (u16, Option<String>, String, bool, Vec<String>);
/// Kết quả một response HEAD: `(status, headers, giữ_kết_nối)`.
type KetQuaHead = (u16, Vec<(String, String)>, bool);

/// Header phụ của request (không log): Basic auth + Cookie.
#[derive(Clone, Copy, Default)]
struct HeaderThem<'a> {
    auth: Option<&'a str>,
    cookie: Option<&'a str>,
}

/// Ghép request HTTP/1.1; `auth`/`cookie` chỉ là giá trị header (không log).
fn yeu_cau_http(method: &str, target: &str, authority: &str, them: HeaderThem<'_>) -> String {
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

fn set_cookie_cua(d: &DauHttp) -> Vec<String> {
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

fn header_gia_tri(headers: &[(String, String)], ten: &str) -> Option<String> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(ten))
        .map(|(_, v)| v.clone())
}

/// Xử lý một response HTTP/2 (status/Location/body) theo đúng luật redirect của HTTP/1.1.
fn xu_ly_h2(
    net: &mut dyn NetHost,
    u: &UrlHttp,
    path: &str,
    kq: vis_core::host::KetQuaHttp2,
    so: &mut u8,
    hien_tai: &mut String,
    da_thu_lai: &mut bool,
) -> Result<Option<String>, String> {
    let set_cookie: Vec<String> = kq
        .headers
        .iter()
        .filter(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
        .map(|(_, v)| v.clone())
        .collect();
    if let Some(j) = net.cookie_jar() {
        j.nhan_set_cookie(&u.host, path, &set_cookie);
    }
    if (300..400).contains(&kq.status) {
        let Some(loc) = header_gia_tri(&kq.headers, "location") else {
            return Err(format!("HTTP {}", kq.status));
        };
        let Some(url) = noi_location(u, &loc)? else {
            return Err(format!("HTTP {}", kq.status));
        };
        *so += 1;
        if *so > 5 {
            return Err("quá 5 lần chuyển hướng HTTP (tối đa 5)".to_string());
        }
        *hien_tai = url;
        *da_thu_lai = false;
        return Ok(None);
    }
    if !(200..300).contains(&kq.status) {
        return Err(format!("HTTP {}", kq.status));
    }
    Ok(Some(String::from_utf8_lossy(&kq.body).into_owned()))
}

/// Gửi GET + đọc **một** phản hồi (đã nối). Trả `(status, Location, body, giữ_kết_nối, set-cookie)`.
///
/// `han` là deadline **chung của cả lời gọi** (do `http_get` tạo một lần); `timeout` chỉ dùng để
/// giữ nguyên thông điệp lỗi lịch sử. Mỗi lần đọc chỉ dùng phần thời gian **còn lại** tới `han`.
fn doc_http_mot(
    net: &mut dyn NetHost,
    tay: &str,
    authority: &str,
    target: &str,
    han: Instant,
    timeout: u64,
    them: HeaderThem<'_>,
) -> Result<KetQuaGet, LoiHttp> {
    let req = yeu_cau_http("GET", target, authority, them);
    net.write(tay, req.as_bytes()).map_err(LoiHttp::ket_noi)?;
    let mut du: Vec<u8> = Vec::new();
    let mut info: Option<DauHttp> = None;
    let mut h = 0usize;
    loop {
        if let Some(d) = &info {
            if d.chunked {
                if let Some((body, dung)) = ghep_chunked(&du[h..])? {
                    // Còn byte lạ sau chunk cuối ⇒ không tái dùng (tránh lệch frame).
                    let giu = d.giu_ket_noi && h + dung == du.len();
                    return Ok((
                        d.status,
                        d.location.clone(),
                        String::from_utf8_lossy(&body).into_owned(),
                        giu,
                        set_cookie_cua(d),
                    ));
                }
            } else if let Some(n) = d.cl {
                if du.len() >= h + n {
                    let cuoi = (h + n).min(du.len());
                    let giu = d.giu_ket_noi && cuoi == du.len();
                    return Ok((
                        d.status,
                        d.location.clone(),
                        String::from_utf8_lossy(&du[h..cuoi]).into_owned(),
                        giu,
                        set_cookie_cua(d),
                    ));
                }
            }
        }
        let con = con_lai_ms(han).ok_or_else(|| {
            LoiHttp::khong_thu_lai(format!(
                "quá thời gian chờ HTTP từ `{authority}` sau {} ms",
                timeout.max(1)
            ))
        })?;
        // Giữ nguyên clamp 2 000 ms **mỗi lần đọc** (VD-183) — không nới, không bỏ.
        let doc = net
            .read(tay, 16 * 1024, con.clamp(1, 2_000))
            .map_err(loi_doc)?;
        if doc.is_empty() {
            let d = info.ok_or_else(|| {
                LoiHttp::ket_noi("phản hồi HTTP không đủ header (kết nối đóng sớm)")
            })?;
            let set = set_cookie_cua(&d);
            if d.chunked {
                return match ghep_chunked(&du[h..])? {
                    Some((body, _)) => Ok((
                        d.status,
                        d.location,
                        String::from_utf8_lossy(&body).into_owned(),
                        false,
                        set,
                    )),
                    None => Err(LoiHttp::ket_noi("chunked: kết nối đóng khi chưa đủ chunk")),
                };
            }
            return match d.cl {
                Some(n) if du.len() < h + n => Err(LoiHttp::ket_noi(format!(
                    "Content-Length {n} nhưng mới đọc {} byte (kết nối đóng sớm)",
                    du.len() - h
                ))),
                Some(n) => Ok((
                    d.status,
                    d.location,
                    String::from_utf8_lossy(&du[h..h + n]).into_owned(),
                    false,
                    set.clone(),
                )),
                None => Ok((
                    d.status,
                    d.location,
                    String::from_utf8_lossy(&du[h..]).into_owned(),
                    false,
                    set.clone(),
                )),
            };
        }
        du.extend_from_slice(&doc);
        if info.is_none() {
            if let Some(i) = tim_header(&du) {
                h = i + 4;
                let d = doc_header_day_du(&du[..i])?;
                if let Some(n) = d.cl {
                    if n > MAX_HTTP {
                        return Err(LoiHttp::khong_thu_lai(format!(
                            "phản hồi vượt giới hạn {MAX_HTTP} byte (8 MiB)"
                        )));
                    }
                }
                info = Some(d);
            } else if du.len() > MAX_HEADER {
                return Err(LoiHttp::khong_thu_lai(format!(
                    "phản hồi HTTP header quá lớn (> {MAX_HEADER} byte)"
                )));
            }
        }
        if info.is_some() && du.len().saturating_sub(h) > MAX_HTTP {
            return Err(LoiHttp::khong_thu_lai(format!(
                "phản hồi vượt giới hạn {MAX_HTTP} byte (8 MiB)"
            )));
        }
    }
}

/// `http.get`/`get_timeout` — nối qua `NetHost`, chạy giao thức, **trả kết nối về pool** khi response
/// cho phép keep-alive; lỗi/hết giờ đóng ngay. Vẫn one-shot với host không có pool.
///
/// **ST-265 (VD-250, #148)**: một deadline monotonic duy nhất cho **toàn bộ** lời gọi — tính ở đây
/// một lần, truyền xuống connect và từng hop `doc_http_mot`; **không** đặt lại khi chuyển hướng.
/// **VS3-4b**: kết nối idle lấy từ pool có thể đã bị server đóng ⇒ thử lại **một lần** trên kết nối
/// mới (GET idempotent), không tính là redirect.
pub fn http_get(net: &mut dyn NetHost, url: &str, timeout_ms: u64) -> Result<String, String> {
    let timeout = timeout_ms.max(1);
    let han = Instant::now() + Duration::from_millis(timeout);
    let mut hien_tai = url.to_string();
    let mut so = 0u8;
    let mut da_thu_lai = false;
    loop {
        let u = tach_url(&hien_tai)?;
        let authority = u.authority();
        let la_https = u.giao_thuc == GiaoThuc::Https;
        let path = u.target.split('?').next().unwrap_or("/");
        let auth = u.authorization();
        let cookie = net
            .cookie_jar()
            .and_then(|j| j.header(&u.host, path, la_https));
        // Connect chỉ được dùng phần thời gian còn lại; hết hạn ⇒ lỗi, không mở kết nối mới.
        let con = con_lai_ms(han).ok_or_else(|| {
            format!(
                "quá thời gian chờ HTTP từ `{authority}` sau {} ms",
                timeout.max(1)
            )
        })?;
        // **VS3-4b2**: thử HTTP/2 (ALPN) trước; `None` ⇒ fallback HTTP/1.1 trên cùng kết nối TLS.
        if la_https {
            let hs = header_h2(auth.as_deref(), cookie.as_deref());
            if let Some(kq) = net.http2_get(&u.dia_chi(), &u.target, "GET", &hs, con)? {
                net.ghi_nhat_ky(&format!(
                    "NET {} GET {authority}{}",
                    u.giao_thuc.ten(),
                    u.target
                ));
                if let Some(body) =
                    xu_ly_h2(net, &u, path, kq, &mut so, &mut hien_tai, &mut da_thu_lai)?
                {
                    return Ok(body);
                }
                continue;
            }
        }
        // **VS3-4a**: `https://` đi qua TLS của host; **VS3-4b**: dùng lại kết nối idle nếu có.
        let (tay, tai_su_dung) = net.connect_http(&u.dia_chi(), la_https, !da_thu_lai, con)?;
        net.ghi_nhat_ky(&format!(
            "NET {} GET {authority}{}",
            u.giao_thuc.ten(),
            u.target
        ));
        let kq = doc_http_mot(
            net,
            &tay,
            &authority,
            &u.target,
            han,
            timeout,
            HeaderThem {
                auth: auth.as_deref(),
                cookie: cookie.as_deref(),
            },
        );
        match kq {
            Ok((status, location, body, giu_lai, set_cookie)) => {
                if let Some(j) = net.cookie_jar() {
                    j.nhan_set_cookie(&u.host, path, &set_cookie);
                }
                let _ = net.tra_http(&tay, giu_lai);
                if (300..400).contains(&status) {
                    let Some(loc) = location else {
                        return Err(format!("HTTP {status}"));
                    };
                    let Some(url) = noi_location(&u, &loc)? else {
                        // `Location` rỗng/fragment cùng URL ⇒ **trả mã gốc**, không lỗi, không theo.
                        return Err(format!("HTTP {status}"));
                    };
                    so += 1;
                    if so > 5 {
                        return Err("quá 5 lần chuyển hướng HTTP (tối đa 5)".to_string());
                    }
                    hien_tai = url;
                    da_thu_lai = false;
                    continue;
                }
                if !(200..300).contains(&status) {
                    return Err(format!("HTTP {status}"));
                }
                return Ok(body);
            }
            Err(e) => {
                let _ = net.tra_http(&tay, false);
                if tai_su_dung && !da_thu_lai && e.retry_duoc {
                    da_thu_lai = true;
                    continue;
                }
                return Err(e.thong_diep);
            }
        }
    }
}

/// `HEAD` — gửi `HEAD`, chỉ đọc **header**, trả `(status, headers, giữ_kết_nối)`. **Không** đọc thân
/// (kể cả khi server gửi `Content-Length`/chunked); **không** theo redirect (trả mã gốc + `Location`),
/// khớp tài liệu “redirect chỉ GET”.
fn head_mot(
    net: &mut dyn NetHost,
    tay: &str,
    authority: &str,
    target: &str,
    han: Instant,
    timeout: u64,
    them: HeaderThem<'_>,
) -> Result<KetQuaHead, LoiHttp> {
    let req = yeu_cau_http("HEAD", target, authority, them);
    net.write(tay, req.as_bytes()).map_err(LoiHttp::ket_noi)?;
    let mut du: Vec<u8> = Vec::new();
    loop {
        let con = con_lai_ms(han).ok_or_else(|| {
            LoiHttp::khong_thu_lai(format!(
                "quá thời gian chờ HTTP từ `{authority}` sau {} ms",
                timeout.max(1)
            ))
        })?;
        // Giữ nguyên clamp 2 000 ms **mỗi lần đọc** (VD-183) — không nới, không bỏ.
        let doc = net
            .read(tay, 16 * 1024, con.clamp(1, 2_000))
            .map_err(loi_doc)?;
        if doc.is_empty() {
            let i = tim_header(&du).ok_or_else(|| {
                LoiHttp::ket_noi("phản hồi HTTP không đủ header (kết nối đóng sớm)")
            })?;
            let d = doc_header_day_du(&du[..i])?;
            return Ok((d.status, d.headers, false));
        }
        du.extend_from_slice(&doc);
        if let Some(i) = tim_header(&du) {
            let d = doc_header_day_du(&du[..i])?;
            // Byte sau header (server gửi thân cho HEAD) ⇒ không tái dùng để tránh lệch frame.
            let giu = d.giu_ket_noi && du.len() == i + 4;
            return Ok((d.status, d.headers, giu));
        }
        if du.len() > MAX_HEADER {
            return Err(LoiHttp::khong_thu_lai(format!(
                "phản hồi HTTP header quá lớn (> {MAX_HEADER} byte)"
            )));
        }
    }
}

/// `http.head` — một request, không đọc thân, không theo redirect.
///
/// **ST-265 (VD-250, #148)**: deadline monotonic tính một lần ở đầu lời gọi, dùng chung cho
/// connect + đọc header (dù HEAD không có redirect, vẫn nhất quán với `http_get`).
/// **VS3-4b**: dùng pool như GET; kết nối idle chết được thử lại một lần.
pub fn http_head(
    net: &mut dyn NetHost,
    url: &str,
    timeout_ms: u64,
) -> Result<(u16, Vec<(String, String)>), String> {
    let timeout = timeout_ms.max(1);
    let han = Instant::now() + Duration::from_millis(timeout);
    let u = tach_url(url)?;
    let authority = u.authority();
    let la_https = u.giao_thuc == GiaoThuc::Https;
    let path = u.target.split('?').next().unwrap_or("/");
    let auth = u.authorization();
    let mut da_thu_lai = false;
    loop {
        let cookie = net
            .cookie_jar()
            .and_then(|j| j.header(&u.host, path, la_https));
        let con = con_lai_ms(han).ok_or_else(|| {
            format!(
                "quá thời gian chờ HTTP từ `{authority}` sau {} ms",
                timeout.max(1)
            )
        })?;
        if la_https {
            let hs = header_h2(auth.as_deref(), cookie.as_deref());
            if let Some(kq) = net.http2_get(&u.dia_chi(), &u.target, "HEAD", &hs, con)? {
                net.ghi_nhat_ky(&format!(
                    "NET {} HEAD {authority}{}",
                    u.giao_thuc.ten(),
                    u.target
                ));
                let set_cookie: Vec<String> = kq
                    .headers
                    .iter()
                    .filter(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
                    .map(|(_, v)| v.clone())
                    .collect();
                if let Some(j) = net.cookie_jar() {
                    j.nhan_set_cookie(&u.host, path, &set_cookie);
                }
                return Ok((kq.status, kq.headers));
            }
        }
        let (tay, tai_su_dung) = net.connect_http(&u.dia_chi(), la_https, !da_thu_lai, con)?;
        net.ghi_nhat_ky(&format!(
            "NET {} HEAD {authority}{}",
            u.giao_thuc.ten(),
            u.target
        ));
        match head_mot(
            net,
            &tay,
            &authority,
            &u.target,
            han,
            timeout,
            HeaderThem {
                auth: auth.as_deref(),
                cookie: cookie.as_deref(),
            },
        ) {
            Ok((status, headers, giu_lai)) => {
                let set_cookie: Vec<String> = headers
                    .iter()
                    .filter(|(k, _)| k == "set-cookie")
                    .map(|(_, v)| v.clone())
                    .collect();
                if let Some(j) = net.cookie_jar() {
                    j.nhan_set_cookie(&u.host, path, &set_cookie);
                }
                let _ = net.tra_http(&tay, giu_lai);
                return Ok((status, headers));
            }
            Err(e) => {
                let _ = net.tra_http(&tay, false);
                if tai_su_dung && !da_thu_lai && e.retry_duoc {
                    da_thu_lai = true;
                    continue;
                }
                return Err(e.thong_diep);
            }
        }
    }
}

#[cfg(test)]
mod tests;

pub mod async_http;
pub use async_http::{http_get_async, http_head_async, xoa_trang_thai_frame, AsyncHttpEnv};
pub use vis_core::reactor::{FrameReactorBackend, ReactorH2Response, ReactorOp};
