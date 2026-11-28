// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-323b** — tách nguyên văn từ `lib.rs`: HTTP/1.1 + h2 client (R1 headroom).

use std::time::{Duration, Instant};

use vis_core::host::NetHost;

use crate::header::{con_lai_ms, doc_header_day_du, ghep_chunked, tim_header, DauHttp};
use crate::request::{
    header_gia_tri, header_h2, loi_doc, noi_location, set_cookie_cua, yeu_cau_http, HeaderThem,
    KetQuaGet, KetQuaHead, LoiHttp,
};
use crate::url::{tach_url, GiaoThuc, UrlHttp};
use crate::{MAX_HEADER, MAX_HTTP};

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
