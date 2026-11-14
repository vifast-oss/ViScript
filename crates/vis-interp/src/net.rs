// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-64 (VD-100/VD-105)** — **giao thức HTTP client** nằm trong `vis-interp` (không ở CLI):
//! socket thô qua [`NetHost`], tự dựng/đọc **HTTP/1.1**. Nhờ vậy interp ↔ VM **cùng byte** và codegen
//! **không** phải sửa (`std.http.*` hạ theo tên như mọi `std.*`).
//!
//! Phạm vi vòng 11: **chỉ** `http://`, **chỉ loopback** (`127.0.0.1`/`localhost`); **không** TLS,
//! **không** redirect, **không** keep-alive; phản hồi cap **8 MiB** (mẫu `MAX_STDIN`, ST-34) ⇒ vượt
//! cap là `Err`, không OOM. Body giữ **byte** tới khi chuyển `str` (lossy) — ngôn ngữ chưa có kiểu byte.

use std::time::{Duration, Instant};

use crate::builtin::NetHost;

/// Cap **phản hồi** (8 MiB, mẫu `MAX_STDIN` của ST-34) — vượt ⇒ `Err`, không OOM.
pub const MAX_HTTP: usize = 8 * 1024 * 1024;
/// Cap **header** (64 KiB) — header lớn hơn ⇒ `Err` (không đọc vô hạn).
const MAX_HEADER: usize = 64 * 1024;

/// Tách `http://host[:port][/path][?query]` → `(authority, target)`; chỉ **loopback**.
fn tach_url(url: &str) -> Result<(String, String), String> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| format!("chỉ hỗ trợ `http://`, được `{url}`"))?;
    if rest.contains('@') {
        return Err(format!("URL có userinfo không hỗ trợ: `{url}`"));
    }
    let i = rest.find(['/', '?']).unwrap_or(rest.len());
    let (authority, duong) = (&rest[..i], &rest[i..]);
    if authority.is_empty() {
        return Err(format!("URL thiếu host: `{url}`"));
    }
    let (host, cong) = match authority.rsplit_once(':') {
        Some((h, c)) => (h, c),
        None => (authority, "80"),
    };
    if host != "127.0.0.1" && host != "localhost" {
        return Err(format!(
            "chỉ cho phép loopback (`127.0.0.1`/`localhost`), được `{host}`"
        ));
    }
    cong.parse::<u16>()
        .map_err(|_| format!("cổng `{cong}` không hợp lệ (0..65535)"))?;
    let authority = if cong == "80" {
        host.to_string()
    } else {
        format!("{host}:{cong}")
    };
    let target = if duong.is_empty() {
        "/".to_string()
    } else if duong.starts_with('/') {
        duong.to_string()
    } else {
        format!("/{duong}")
    };
    Ok((authority, target))
}

/// Tìm `\r\n\r\n` trong `du` — vị trí **byte đầu** của dấu phân cách.
fn tim_header(du: &[u8]) -> Option<usize> {
    du.windows(4).position(|w| w == b"\r\n\r\n")
}

/// Thông tin header cần cho giao thức.
#[derive(Default)]
struct DauHttp {
    status: u16,
    cl: Option<usize>,
    chunked: bool,
    location: Option<String>,
}

/// Đọc status + `Content-Length` + `Transfer-Encoding` + `Location` từ khối header (không gồm
/// `\r\n\r\n`).
fn doc_header_day_du(dau: &[u8]) -> Result<DauHttp, String> {
    let text = String::from_utf8_lossy(dau);
    let mut lines = text.split("\r\n");
    let status_line = lines.next().unwrap_or("");
    let mut phan = status_line.split_whitespace();
    let _ver = phan.next();
    let status: u16 = phan
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| format!("status line HTTP không hợp lệ: `{status_line}`"))?;
    let mut out = DauHttp {
        status,
        ..Default::default()
    };
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
            }
        }
    }
    Ok(out)
}

/// Giữ API cũ cho unit test: `(status, Content-Length)`.
#[cfg(test)]
fn doc_header(dau: &[u8]) -> Result<(u16, Option<usize>), String> {
    let d = doc_header_day_du(dau)?;
    Ok((d.status, d.cl))
}

/// Ghép thân **chunked** (RFC 7230 §4.1); `None` = còn thiếu byte, `Err` = cú pháp/chunk quá cap.
/// Hỗ trợ **chunk mở rộng** `<hex>;ext=1\r\n` (bỏ phần sau `;`); chunk `0` kết thúc.
fn ghep_chunked(du: &[u8]) -> Result<Option<Vec<u8>>, String> {
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
            return Ok(Some(out));
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
            return Err("chunk thiếu `\r\n` sau dữ liệu".to_string());
        }
        i += 2;
    }
}

/// Ghép URL `Location` với URL hiện tại: tuyệt đối `http://…` giữ nguyên (qua `tach_url` — vẫn chỉ
/// loopback), `/x` tuyệt đối theo authority, `x` tương đối theo thư mục target.
fn noi_location(authority: &str, target: &str, location: &str) -> Result<String, String> {
    if location.starts_with("http://") || location.starts_with("https://") {
        return Ok(location.to_string());
    }
    if location.starts_with('/') {
        return Ok(format!("http://{authority}{location}"));
    }
    let thu_muc = target.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
    Ok(format!("http://{authority}{thu_muc}/{location}"))
}

/// Gửi GET + đọc **một** phản hồi (đã nối). Trả `(status, Location, body)`.
fn doc_http_mot(
    net: &mut dyn NetHost,
    tay: &str,
    authority: &str,
    target: &str,
    timeout: u64,
) -> Result<(u16, Option<String>, String), String> {
    let req = format!("GET {target} HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\n\r\n");
    let han = Instant::now() + Duration::from_millis(timeout.max(1));
    net.write(tay, req.as_bytes())?;
    let mut du: Vec<u8> = Vec::new();
    let mut info: Option<DauHttp> = None;
    let mut h = 0usize;
    loop {
        if let Some(d) = &info {
            if d.chunked {
                if let Some(body) = ghep_chunked(&du[h..])? {
                    return Ok((
                        d.status,
                        d.location.clone(),
                        String::from_utf8_lossy(&body).into_owned(),
                    ));
                }
            } else if let Some(n) = d.cl {
                if du.len() >= h + n {
                    let cuoi = (h + n).min(du.len());
                    return Ok((
                        d.status,
                        d.location.clone(),
                        String::from_utf8_lossy(&du[h..cuoi]).into_owned(),
                    ));
                }
            }
        }
        let con = han.saturating_duration_since(Instant::now());
        if con.is_zero() {
            return Err(format!(
                "quá thời gian chờ HTTP từ `{authority}` sau {} ms",
                timeout.max(1)
            ));
        }
        let doc = net.read(tay, 16 * 1024, con.as_millis().clamp(1, 2_000) as u64)?;
        if doc.is_empty() {
            let d =
                info.ok_or_else(|| "phản hồi HTTP không đủ header (kết nối đóng sớm)".to_string())?;
            if d.chunked {
                return match ghep_chunked(&du[h..])? {
                    Some(body) => Ok((
                        d.status,
                        d.location,
                        String::from_utf8_lossy(&body).into_owned(),
                    )),
                    None => Err("chunked: kết nối đóng khi chưa đủ chunk".to_string()),
                };
            }
            return match d.cl {
                Some(n) if du.len() < h + n => Err(format!(
                    "Content-Length {n} nhưng mới đọc {} byte (kết nối đóng sớm)",
                    du.len() - h
                )),
                Some(n) => Ok((
                    d.status,
                    d.location,
                    String::from_utf8_lossy(&du[h..h + n]).into_owned(),
                )),
                None => Ok((
                    d.status,
                    d.location,
                    String::from_utf8_lossy(&du[h..]).into_owned(),
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
                        return Err(format!("phản hồi vượt giới hạn {MAX_HTTP} byte (8 MiB)"));
                    }
                }
                info = Some(d);
            } else if du.len() > MAX_HEADER {
                return Err(format!(
                    "phản hồi HTTP header quá lớn (> {MAX_HEADER} byte)"
                ));
            }
        }
        if info.is_some() && du.len().saturating_sub(h) > MAX_HTTP {
            return Err(format!("phản hồi vượt giới hạn {MAX_HTTP} byte (8 MiB)"));
        }
    }
}

/// `http.get`/`get_timeout` — nối qua `NetHost`, chạy giao thức, **đóng tay** (best-effort).
pub fn http_get(net: &mut dyn NetHost, url: &str, timeout_ms: u64) -> Result<String, String> {
    let timeout = timeout_ms.max(1);
    let mut hien_tai = url.to_string();
    let mut so = 0u8;
    loop {
        let (authority, target) = tach_url(&hien_tai)?;
        let tay = net.connect(&authority, timeout)?;
        net.ghi_nhat_ky(&format!("NET http GET {authority}{target}"));
        let kq = doc_http_mot(net, &tay, &authority, &target, timeout);
        let _ = net.close(&tay);
        let (status, location, body) = kq?;
        if (300..400).contains(&status) {
            let Some(loc) = location else {
                return Err(format!("HTTP {status}"));
            };
            so += 1;
            if so > 5 {
                return Err("quá 5 lần chuyển hướng HTTP (tối đa 5)".to_string());
            }
            hien_tai = noi_location(&authority, &target, &loc)?;
            continue;
        }
        if !(200..300).contains(&status) {
            return Err(format!("HTTP {status}"));
        }
        return Ok(body);
    }
}

#[cfg(test)]
mod tests {
    use super::{doc_header, tach_url};

    #[test]
    fn tach_url_dung_va_chan_ngoai_loopback() {
        assert_eq!(
            tach_url("http://127.0.0.1:8080/a?b=1").unwrap(),
            ("127.0.0.1:8080".to_string(), "/a?b=1".to_string())
        );
        assert_eq!(
            tach_url("http://localhost").unwrap(),
            ("localhost".to_string(), "/".to_string())
        );
        assert_eq!(tach_url("http://127.0.0.1:81").unwrap().1, "/".to_string());
        assert!(tach_url("https://127.0.0.1/").is_err());
        assert!(tach_url("http://example.com/").is_err());
        assert!(tach_url("http://127.0.0.1:99999/").is_err());
    }

    #[test]
    fn doc_header_doc_status_va_content_length() {
        assert_eq!(
            doc_header(b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\nX: y").unwrap(),
            (200, Some(9))
        );
        assert_eq!(
            doc_header(b"HTTP/1.1 404 Not Found\r\ncontent-length: 0").unwrap(),
            (404, Some(0))
        );
        assert_eq!(doc_header(b"HTTP/1.1 204 No Content").unwrap(), (204, None));
        assert!(doc_header(b"rac").is_err());
        assert!(doc_header(b"HTTP/1.1 200 OK\r\nContent-Length: x").is_err());
    }
}
