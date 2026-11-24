// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-311d (VD-286)** — test đơn vị của HTTP client (`http_client/mod.rs`), tách ra cùng thư mục.

use super::{
    chuan_hoa_target, doc_header, doc_header_day_du, ghep_chunked, noi_location, tach_url, GiaoThuc,
};

#[test]
fn tach_url_dung_va_chan_ngoai_loopback() {
    let u = tach_url("http://127.0.0.1:8080/a?b=1").unwrap();
    assert_eq!(u.giao_thuc, GiaoThuc::Http);
    assert_eq!(u.authority(), "127.0.0.1:8080");
    assert_eq!(u.target, "/a?b=1");
    assert_eq!(u.host, "127.0.0.1");
    assert_eq!(u.cong, 8080);
    assert_eq!(
        tach_url("http://localhost").unwrap().authority(),
        "localhost".to_string()
    );
    assert_eq!(
        tach_url("http://localhost").unwrap().dia_chi(),
        "localhost:80".to_string()
    );
    assert_eq!(
        tach_url("http://127.0.0.1:81").unwrap().target,
        "/".to_string()
    );
    assert!(tach_url("http://example.com/").is_err());
    assert!(tach_url("http://127.0.0.1:99999/").is_err());
    assert!(tach_url("ftp://127.0.0.1/").is_err());
    assert!(tach_url("http://user@127.0.0.1/").is_err());
    assert!(tach_url("http://[::1]:8080/").is_ok());
    assert!(tach_url("http://127.0.0.2:8080/").is_ok());
    assert!(tach_url("http://10.0.0.1/").is_err());
}

/// **VS3-4a**: `https://` cho host bất kỳ; cổng mặc định 443; IPv6 bọc `[]` khi dựng authority.
#[test]
fn tach_url_https_va_cong_mac_dinh() {
    let u = tach_url("https://example.com/a?x=1").unwrap();
    assert_eq!(u.giao_thuc, GiaoThuc::Https);
    assert_eq!(u.authority(), "example.com");
    assert_eq!(u.target, "/a?x=1");
    assert_eq!(u.url(), "https://example.com/a?x=1");
    assert_eq!(u.dia_chi(), "example.com:443");

    let u = tach_url("https://127.0.0.1:8443").unwrap();
    assert_eq!(u.authority(), "127.0.0.1:8443");
    assert_eq!(u.target, "/");
    assert_eq!(u.url(), "https://127.0.0.1:8443/");

    let u = tach_url("https://[::1]:8443/x").unwrap();
    assert_eq!(u.host, "::1");
    assert_eq!(u.cong, 8443);
    assert_eq!(u.authority(), "[::1]:8443");
    assert_eq!(u.url(), "https://[::1]:8443/x");
    assert!(tach_url("https://[::1/x").is_err());
    assert!(tach_url("https://").is_err());
}

/// `..`/`.` chuẩn hoá, query-only giữ path, fragment/rỗng ⇒ **không theo**.
#[test]
fn chuan_hoa_va_location_day_du() {
    assert_eq!(chuan_hoa_target("/a/b/../c?x=1"), "/a/c?x=1");
    assert_eq!(chuan_hoa_target("/a/./b/"), "/a/b/");
    assert_eq!(chuan_hoa_target("/../x"), "/x");
    let co_so = tach_url("http://127.0.0.1:1/a").unwrap();
    assert_eq!(
        noi_location(&co_so, "/a/b/../c?x=1").unwrap(),
        Some("http://127.0.0.1:1/a/c?x=1".to_string())
    );
    let co_so_query = tach_url("http://127.0.0.1:1/a?old=1").unwrap();
    assert_eq!(
        noi_location(&co_so_query, "?y=2").unwrap(),
        Some("http://127.0.0.1:1/a?y=2".to_string())
    );
    assert_eq!(
        noi_location(&co_so, "b/c").unwrap(),
        Some("http://127.0.0.1:1/b/c".to_string())
    );
    assert_eq!(noi_location(&co_so, "#f").unwrap(), None);
    assert_eq!(noi_location(&co_so, "").unwrap(), None);
    assert_eq!(
        noi_location(&co_so, "http://127.0.0.1:2/x/../y").unwrap(),
        Some("http://127.0.0.1:2/y".to_string())
    );
}

/// **VS3-4a**: redirect giữ scheme; nâng cấp HTTP → HTTPS được, **hạ cấp** HTTPS → HTTP thì không.
#[test]
fn location_giu_scheme_va_chan_ha_cap() {
    let https = tach_url("https://127.0.0.1:1/a").unwrap();
    assert_eq!(
        noi_location(&https, "/x").unwrap(),
        Some("https://127.0.0.1:1/x".to_string())
    );
    let e = noi_location(&https, "http://127.0.0.1:2/x").unwrap_err();
    assert!(e.contains("HTTPS → HTTP"), "lỗi hạ cấp sai: {e}");
    let http = tach_url("http://127.0.0.1:1/a").unwrap();
    assert_eq!(
        noi_location(&http, "https://example.com/x").unwrap(),
        Some("https://example.com/x".to_string())
    );
}

/// **VS3-4b**: userinfo `user:pass@` → Basic auth; không lộ credential trong lỗi/authority.
#[test]
fn tach_url_userinfo_basic_auth() {
    let u = tach_url("http://user:pass@127.0.0.1:8080/a").unwrap();
    assert_eq!(u.authority(), "127.0.0.1:8080");
    assert_eq!(u.authorization().as_deref(), Some("Basic dXNlcjpwYXNz"));
    assert_eq!(u.host, "127.0.0.1");
    let e = tach_url("http://user@127.0.0.1/").unwrap_err();
    assert!(!e.contains("user@"), "lỗi không được lộ userinfo: {e}");
    let e = tach_url("http://user:pa\r\nss@127.0.0.1/").unwrap_err();
    assert!(!e.contains("pa"), "lỗi không được lộ credential: {e}");
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

/// **VS3-4b**: mặc định HTTP/1.1 giữ kết nối; `Connection: close`/HTTP1.0 không header thì không;
/// HTTP/1.0 + `keep-alive` thì có.
#[test]
fn header_giu_ket_noi_theo_phien_ban_va_connection() {
    assert!(
        doc_header_day_du(b"HTTP/1.1 200 OK\r\nContent-Length: 0")
            .unwrap()
            .giu_ket_noi
    );
    assert!(
        !doc_header_day_du(b"HTTP/1.1 200 OK\r\nConnection: close")
            .unwrap()
            .giu_ket_noi
    );
    assert!(
        !doc_header_day_du(b"HTTP/1.0 200 OK\r\nContent-Length: 0")
            .unwrap()
            .giu_ket_noi
    );
    assert!(
        doc_header_day_du(b"HTTP/1.0 200 OK\r\nConnection: keep-alive")
            .unwrap()
            .giu_ket_noi
    );
    assert!(
        !doc_header_day_du(b"HTTP/1.1 101 Switching Protocols")
            .unwrap()
            .giu_ket_noi
    );
}

/// **VS3-4b**: đọc hết **trailer** để kết nối chunked có thể tái dùng; trả đúng số byte đã dùng.
#[test]
fn chunked_doc_trailer_va_byte_da_dung() {
    let du = b"3\r\nabc\r\n0\r\nX-A: 1\r\n\r\nRAC";
    let (body, dung) = ghep_chunked(du).unwrap().expect("đủ chunk");
    assert_eq!(body, b"abc");
    assert_eq!(dung, du.len() - 3, "3 byte `RAC` phải còn lại");
    let du = b"3\r\nabc\r\n0\r\n\r\n";
    let (body, dung) = ghep_chunked(du).unwrap().expect("đủ chunk");
    assert_eq!(body, b"abc");
    assert_eq!(dung, du.len());
}
