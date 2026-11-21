// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-4a (VD-271)** — TLS client của `Net` bằng **rustls** + **webpki-roots**.
//!
//! - Root store mặc định là **bundle Mozilla** (`webpki_roots`), không phụ thuộc kho hệ điều hành;
//!   nhờ vậy hành vi giống nhau trên Linux/Windows và test không cần Internet.
//! - Chỉ **test** mới dựng được cấu hình với gốc riêng ([`tu_goc_der`]). Bản **release** của binary
//!   `vis` không có cờ/biến môi trường nào đổi root store; bản **debug** có hook
//!   `VIS_TLS_TEST_ROOT_DER` (một chứng chỉ DER) **chỉ để test hermetic** — xem `mac_dinh`.
//! - Handshake dùng **phần thời gian còn lại** của deadline chung (`http_get`/`http_head` tính một
//!   lần), không panic; mọi lỗi chứng chỉ/alert trả `Err` có tiền tố `TLS`.
//! - **Không** HTTP/2; HTTP/1.1 keep-alive do pool của `Net` (VS3-4b).

use std::net::TcpStream;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::{Duration, Instant};

use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};

/// Luồng TLS đã handshake xong; đọc/ghi qua `StreamOwned`, socket gốc ở trường `sock`.
pub(super) type KenhTls = StreamOwned<ClientConnection, TcpStream>;
/// Cấu hình TLS dùng chung cho mọi kết nối HTTPS của một `Net`.
pub(super) type CauHinh = Arc<ClientConfig>;

/// Dựng `ClientConfig` từ một root store; `ring` là provider mặc định (không kéo `aws-lc-rs`).
fn boi_canh(goc: RootCertStore) -> Result<CauHinh, String> {
    let nha = Arc::new(rustls::crypto::ring::default_provider());
    let cfg = ClientConfig::builder_with_provider(nha)
        .with_safe_default_protocol_versions()
        .map_err(|e| format!("không khởi tạo được phiên bản TLS: {e}"))?
        .with_root_certificates(goc)
        .with_no_client_auth();
    Ok(Arc::new(cfg))
}

/// Cấu hình **sản phẩm**: tin bundle gốc Mozilla (`webpki-roots`).
///
/// Bản **debug** có hook `goc_kiem_thu` để test hermetic (registry TLS local); bản **release**
/// không biên dịch hook đó, nên không có cách đổi root store từ môi trường.
pub(super) fn mac_dinh() -> Result<CauHinh, String> {
    #[cfg(debug_assertions)]
    if let Some(goc) = goc_kiem_thu()? {
        return boi_canh(goc);
    }
    let mut goc = RootCertStore::empty();
    goc.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    boi_canh(goc)
}

/// **CHỈ BẢN DEBUG**: nếu `VIS_TLS_TEST_ROOT_DER` trỏ tới một chứng chỉ DER, dùng nó làm root store
/// duy nhất. Release **không** biên dịch hàm này; mục đích là để integration test chạy `vis pkg` với
/// registry TLS local (chứng chỉ fixture) mà không cần Internet hay cài root vào máy.
#[cfg(debug_assertions)]
fn goc_kiem_thu() -> Result<Option<RootCertStore>, String> {
    let Some(p) = std::env::var_os("VIS_TLS_TEST_ROOT_DER") else {
        return Ok(None);
    };
    let duong = std::path::Path::new(&p);
    let der = std::fs::read(duong).map_err(|e| {
        format!(
            "VIS_TLS_TEST_ROOT_DER `{}` không đọc được: {e}",
            duong.display()
        )
    })?;
    let mut goc = RootCertStore::empty();
    goc.add(CertificateDer::from(der))
        .map_err(|e| format!("VIS_TLS_TEST_ROOT_DER không phải DER hợp lệ: {e}"))?;
    Ok(Some(goc))
}

/// Cấu hình **chỉ test/embedding**: tin đúng các chứng chỉ DER cho trước.
///
/// Binary `vis` không gọi hàm này; nó chỉ tồn tại để `tests/vs3_tls.rs` dựng server tự ký mà
/// không cần Internet hay đụng kho gốc của máy.
pub(super) fn tu_goc_der(ds: &[&[u8]]) -> Result<CauHinh, String> {
    if ds.is_empty() {
        return Err("cần ít nhất một chứng chỉ gốc TLS cho cấu hình test".to_string());
    }
    let mut goc = RootCertStore::empty();
    for der in ds {
        goc.add(CertificateDer::from(der.to_vec()))
            .map_err(|e| format!("chứng chỉ gốc TLS không hợp lệ: {e}"))?;
    }
    boi_canh(goc)
}

/// **VS3-4b2**: cấu hình TLS có ALPN `h2` rồi `http/1.1` (chỉ feature `http2`). Client HTTP/2 dùng
/// cấu hình này; HTTP/1.1 mặc định vẫn không đặt ALPN.
#[cfg(feature = "http2")]
pub(super) fn cau_hinh_h2(cfg: &CauHinh) -> CauHinh {
    let mut c = (**cfg).clone();
    c.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Arc::new(c)
}

/// Tách `host` khỏi `dia_chi` (`host:port`, IPv6 `[::1]:443`) để làm `ServerName`.
fn ten_may_chu(dia_chi: &str) -> Result<&str, String> {
    let host = if let Some(rest) = dia_chi.strip_prefix('[') {
        rest.split_once(']')
            .map(|(h, _)| h)
            .ok_or_else(|| format!("địa chỉ IPv6 TLS thiếu `]`: `{dia_chi}`"))?
    } else {
        match dia_chi.rsplit_once(':') {
            Some((h, _)) if !h.contains(':') => h,
            _ => dia_chi,
        }
    };
    if host.is_empty() {
        return Err(format!("địa chỉ TLS thiếu host: `{dia_chi}`"));
    }
    Ok(host)
}

fn con_lai(han: Instant) -> Option<Duration> {
    let con = han.saturating_duration_since(Instant::now());
    if con.is_zero() {
        None
    } else {
        Some(con)
    }
}

/// Nối TCP + handshake TLS trong **một** hạn `timeout_ms`; trả luồng đã sẵn sàng đọc/ghi.
pub(super) fn noi(cfg: &CauHinh, dia_chi: &str, timeout_ms: u64) -> Result<KenhTls, String> {
    let timeout_ms = timeout_ms.max(1);
    let han = Instant::now() + Duration::from_millis(timeout_ms);
    let host = ten_may_chu(dia_chi)?;
    let ten = ServerName::try_from(host.to_string())
        .map_err(|e| format!("tên máy chủ TLS `{host}` không hợp lệ: {e}"))?;

    let ds = dia_chi
        .to_socket_addrs()
        .map_err(|e| format!("địa chỉ `{dia_chi}` không phân giải được: {e}"))?;
    let mut ds: Vec<_> = ds.collect();
    // IPv4 trước: Windows có thể trả `::1` trước cho `localhost`.
    ds.sort_by_key(|a| !a.is_ipv4());
    let mut sock: Option<TcpStream> = None;
    let mut loi_cuoi: Option<String> = None;
    for a in ds {
        let Some(con) = con_lai(han) else { break };
        match TcpStream::connect_timeout(&a, con.max(Duration::from_millis(1))) {
            Ok(s) => {
                sock = Some(s);
                break;
            }
            Err(e) => loi_cuoi = Some(format!("{a}: {e}")),
        }
    }
    let mut sock = sock.ok_or_else(|| match loi_cuoi {
        Some(e) => format!("lỗi nối TLS tới `{dia_chi}`: {e}"),
        None => format!("quá thời gian chờ nối TLS tới `{dia_chi}` sau {timeout_ms} ms"),
    })?;
    let _ = sock.set_nodelay(true);

    let con = con_lai(han).ok_or_else(|| {
        format!("quá thời gian chờ TLS handshake tới `{dia_chi}` sau {timeout_ms} ms")
    })?;
    sock.set_read_timeout(Some(con))
        .map_err(|e| format!("không đặt được timeout đọc TLS cho `{dia_chi}`: {e}"))?;
    sock.set_write_timeout(Some(con))
        .map_err(|e| format!("không đặt được timeout ghi TLS cho `{dia_chi}`: {e}"))?;

    let mut conn = ClientConnection::new(Arc::clone(cfg), ten)
        .map_err(|e| format!("lỗi tạo phiên TLS tới `{dia_chi}`: {e}"))?;
    while conn.is_handshaking() {
        match conn.complete_io(&mut sock) {
            Ok(_) => {}
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                return Err(format!(
                    "quá thời gian TLS handshake tới `{dia_chi}` sau {timeout_ms} ms"
                ));
            }
            Err(e) => {
                return Err(format!("lỗi TLS handshake tới `{dia_chi}`: {e}"));
            }
        }
    }
    // Handshake xong: bỏ timeout ghi của handshake (đường `Net::write` hiện không đặt timeout).
    let _ = sock.set_write_timeout(None);
    Ok(StreamOwned::new(conn, sock))
}
