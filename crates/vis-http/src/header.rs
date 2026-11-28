// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-323b** — tách nguyên văn từ `lib.rs`: header/status/chunk parsing (R1 headroom).

use std::time::Instant;

use crate::{MAX_HEADER, MAX_HTTP};

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
pub(crate) fn doc_header(dau: &[u8]) -> Result<(u16, Option<usize>), String> {
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
