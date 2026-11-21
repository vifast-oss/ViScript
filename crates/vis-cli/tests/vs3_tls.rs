// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-4a (VD-271)** — TLS client cho `http.get`/`http.head`: **server TLS thật trong test**
//! (rustls + chứng chỉ tự ký), **không** phụ thuộc Internet.
//!
//! - Ca thành công nạp **chứng chỉ test** vào kho gốc qua `Net::moi_voi_goc_tls` (chỉ test gọi;
//!   binary `vis` luôn dùng `webpki-roots`).
//! - Ca từ chối dùng cấu hình mặc định ⇒ chứng minh **validate theo root store** thật.
//! - Ca `vis run` chứng minh lỗi TLS ra `Err` dữ liệu, **không panic**, cả interp và VM.

#![cfg(all(feature = "tls", not(target_arch = "wasm32")))]

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ServerConfig, ServerConnection, StreamOwned};
use vis_cli::net::Net;
use vis_interp::net::{http_get, http_head};

const CERT_LOCALHOST: &[u8] = include_bytes!("fixtures/vs3_tls/localhost.cert.der");
const KEY_LOCALHOST: &[u8] = include_bytes!("fixtures/vs3_tls/localhost.key.der");
const CERT_SAI_TEN: &[u8] = include_bytes!("fixtures/vs3_tls/sai-ten.cert.der");
const KEY_SAI_TEN: &[u8] = include_bytes!("fixtures/vs3_tls/sai-ten.key.der");

/// **Chống flake**: ca timeout đo thời gian ⇒ tuần tự hoá các ca trong binary này (không nới ngưỡng).
fn kho() -> std::sync::MutexGuard<'static, ()> {
    static KHO: Mutex<()> = Mutex::new(());
    KHO.lock().unwrap_or_else(|e| e.into_inner())
}

struct MayChu {
    dia_chi: String,
    log: Arc<Mutex<Vec<u8>>>,
    tay: Option<std::thread::JoinHandle<()>>,
}

impl MayChu {
    fn dung(mut self) {
        if let Some(t) = self.tay.take() {
            t.join().expect("join server TLS");
        }
    }
}

/// Cấu hình rustls **server** từ chứng chỉ/khoá DER có sẵn (không sinh chứng chỉ lúc chạy).
fn cau_hinh_may_chu(cert: &[u8], key: &[u8]) -> Arc<ServerConfig> {
    let nha = Arc::new(rustls::crypto::ring::default_provider());
    let cert = CertificateDer::from(cert.to_vec());
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.to_vec()));
    Arc::new(
        ServerConfig::builder_with_provider(nha)
            .with_safe_default_protocol_versions()
            .expect("phiên bản TLS mặc định")
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)
            .expect("nạp chứng chỉ server"),
    )
}

/// Server TLS loopback: mỗi kết nối đọc request tới `\r\n\r\n`, ghi **mọi byte** vào log, (tuỳ) ngủ
/// `ngu_ms`, ghi phản hồi tương ứng (phản hồi cuối được lặp lại), gửi `close_notify` rồi đóng.
fn may_chu_tls(cert: &[u8], key: &[u8], phan_hoi: Vec<Vec<u8>>, ngu_ms: u64) -> MayChu {
    assert!(!phan_hoi.is_empty(), "cần ít nhất một phản hồi");
    let l = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let dia_chi = l.local_addr().expect("local_addr").to_string();
    let log = Arc::new(Mutex::new(Vec::new()));
    let log2 = Arc::clone(&log);
    let cfg = cau_hinh_may_chu(cert, key);
    let tay = std::thread::spawn(move || {
        for phan_hoi_mot in phan_hoi {
            let Ok((tcp, _)) = l.accept() else { break };
            let _ = tcp.set_read_timeout(Some(Duration::from_secs(5)));
            let conn = match ServerConnection::new(Arc::clone(&cfg)) {
                Ok(c) => c,
                Err(_) => break,
            };
            let mut s = StreamOwned::new(conn, tcp);
            let mut req = Vec::new();
            let mut buf = [0u8; 4096];
            while req.len() < 65_536 && !req.windows(4).any(|w| w == b"\r\n\r\n") {
                match s.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => req.extend_from_slice(&buf[..n]),
                    Err(_) => break,
                }
            }
            log2.lock().expect("log").extend_from_slice(&req);
            if ngu_ms > 0 {
                std::thread::sleep(Duration::from_millis(ngu_ms));
            }
            let _ = s.write_all(&phan_hoi_mot);
            s.conn.send_close_notify();
            let _ = s.flush();
            let _ = s.sock.shutdown(Shutdown::Both);
        }
    });
    MayChu {
        dia_chi,
        log,
        tay: Some(tay),
    }
}

fn dap_ung(status: &str, body: &[u8], them: &str) -> Vec<u8> {
    let mut r = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: text/plain; charset=utf-8\r\n{them}\r\n",
        body.len()
    )
    .into_bytes();
    r.extend_from_slice(body);
    r
}

fn net_tin_goc(cert: &[u8]) -> Net {
    Net::moi_voi_goc_tls(false, &[cert]).expect("cấu hình TLS test")
}

fn tam(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vis-tls-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    d
}

#[test]
fn https_get_tra_body_va_gui_host_dung() {
    let _k = kho();
    let s = may_chu_tls(
        CERT_LOCALHOST,
        KEY_LOCALHOST,
        vec![dap_ung("200 OK", "xin chào TLS".as_bytes(), "")],
        0,
    );
    let url = format!("https://{}/duong?x=1", s.dia_chi);
    let mut net = net_tin_goc(CERT_LOCALHOST);
    let body = http_get(&mut net, &url, 3_000).expect("HTTPS GET phải thành công");
    assert_eq!(body, "xin chào TLS");
    let log = s.log.lock().expect("log").clone();
    let req = String::from_utf8_lossy(&log);
    assert!(
        req.starts_with(&format!(
            "GET /duong?x=1 HTTP/1.1\r\nHost: {}\r\n",
            s.dia_chi
        )),
        "request HTTPS sai: {req:?}"
    );
    s.dung();
}

#[test]
fn https_head_chi_doc_header_khong_doc_than() {
    let _k = kho();
    let s = may_chu_tls(
        CERT_LOCALHOST,
        KEY_LOCALHOST,
        vec![b"HTTP/1.1 204 No Content\r\nX-Thu: 1\r\n\r\n".to_vec()],
        0,
    );
    let url = format!("https://{}/x", s.dia_chi);
    let mut net = net_tin_goc(CERT_LOCALHOST);
    let (status, headers) = http_head(&mut net, &url, 3_000).expect("HTTPS HEAD phải thành công");
    assert_eq!(status, 204);
    assert!(
        headers.iter().any(|(k, v)| k == "x-thu" && v == "1"),
        "thiếu header `x-thu`: {headers:?}"
    );
    let log = s.log.lock().expect("log").clone();
    assert!(
        String::from_utf8_lossy(&log).starts_with("HEAD /x HTTP/1.1\r\n"),
        "phải gửi HEAD: {:?}",
        String::from_utf8_lossy(&log)
    );
    s.dung();
}

/// Cấu hình **mặc định** (`webpki-roots`) không tin chứng chỉ tự ký ⇒ lỗi TLS rõ, không panic.
#[test]
fn https_tu_choi_chung_chi_tu_ky() {
    let _k = kho();
    let s = may_chu_tls(
        CERT_LOCALHOST,
        KEY_LOCALHOST,
        vec![dap_ung("200 OK", b"khong-duoc", "")],
        0,
    );
    let url = format!("https://{}/", s.dia_chi);
    let mut net = Net::moi(false);
    let e = http_get(&mut net, &url, 3_000).expect_err("chứng chỉ tự ký phải bị từ chối");
    assert!(e.contains("TLS"), "lỗi phải nêu TLS: {e}");
    assert!(
        e.contains("UnknownIssuer") || e.contains("certificate"),
        "lỗi phải nêu lý do chứng chỉ: {e}"
    );
    s.dung();
}

/// Gốc **đã tin** nhưng tên không khớp SAN ⇒ vẫn từ chối (`NotValidForName`).
#[test]
fn https_tu_choi_sai_ten_du_goc_da_tin() {
    let _k = kho();
    let s = may_chu_tls(
        CERT_SAI_TEN,
        KEY_SAI_TEN,
        vec![dap_ung("200 OK", b"khong-duoc", "")],
        0,
    );
    let url = format!("https://{}/", s.dia_chi);
    let mut net = net_tin_goc(CERT_SAI_TEN);
    let e = http_get(&mut net, &url, 3_000).expect_err("sai tên phải bị từ chối");
    assert!(e.contains("TLS"), "lỗi phải nêu TLS: {e}");
    assert!(
        e.contains("NotValidForName") || e.contains("certificate"),
        "lỗi phải nêu lý do tên: {e}"
    );
    s.dung();
}

#[test]
fn https_theo_redirect_giu_tls() {
    let _k = kho();
    let dich = may_chu_tls(
        CERT_LOCALHOST,
        KEY_LOCALHOST,
        vec![dap_ung("200 OK", b"dich", "")],
        0,
    );
    let location = format!("https://{}/b", dich.dia_chi);
    let nguon = may_chu_tls(
        CERT_LOCALHOST,
        KEY_LOCALHOST,
        vec![dap_ung(
            "302 Found",
            b"",
            &format!("Location: {location}\r\n"),
        )],
        0,
    );
    let url = format!("https://{}/a", nguon.dia_chi);
    let mut net = net_tin_goc(CERT_LOCALHOST);
    let body = http_get(&mut net, &url, 3_000).expect("redirect HTTPS phải theo");
    assert_eq!(body, "dich");
    assert!(
        String::from_utf8_lossy(&nguon.log.lock().expect("log")).starts_with("GET /a HTTP/1.1\r\n")
    );
    assert!(
        String::from_utf8_lossy(&dich.log.lock().expect("log")).starts_with("GET /b HTTP/1.1\r\n")
    );
    nguon.dung();
    dich.dung();
}

/// Q2 (3): **không** cho HTTPS → HTTP. Lỗi phải rõ trước khi mở kết nối trần.
#[test]
fn https_chan_ha_cap_ve_http() {
    let _k = kho();
    let s = may_chu_tls(
        CERT_LOCALHOST,
        KEY_LOCALHOST,
        vec![dap_ung(
            "302 Found",
            b"",
            "Location: http://127.0.0.1:1/x\r\n",
        )],
        0,
    );
    let url = format!("https://{}/a", s.dia_chi);
    let mut net = net_tin_goc(CERT_LOCALHOST);
    let e = http_get(&mut net, &url, 3_000).expect_err("phải chặn HTTPS → HTTP");
    assert!(e.contains("HTTPS"), "lỗi phải nêu HTTPS: {e}");
    assert!(e.contains("HTTP"), "lỗi phải nêu hạ cấp: {e}");
    s.dung();
}

#[test]
fn https_het_gio_ro_rang_va_khong_treo() {
    let _k = kho();
    let s = may_chu_tls(
        CERT_LOCALHOST,
        KEY_LOCALHOST,
        vec![dap_ung("200 OK", b"muon", "")],
        1_200,
    );
    let url = format!("https://{}/cham", s.dia_chi);
    let mut net = net_tin_goc(CERT_LOCALHOST);
    let bat_dau = Instant::now();
    let e = http_get(&mut net, &url, 300).expect_err("phải hết giờ");
    let mat = bat_dau.elapsed();
    assert!(e.contains("quá thời gian"), "lỗi phải nêu hết giờ: {e}");
    assert!(
        mat < Duration::from_millis(2_500),
        "hết giờ quá chậm: {mat:?}"
    );
    s.dung();
}

#[test]
fn https_head_khong_theo_redirect() {
    let _k = kho();
    let s = may_chu_tls(
        CERT_LOCALHOST,
        KEY_LOCALHOST,
        vec![dap_ung(
            "302 Found",
            b"",
            "Location: https://127.0.0.1:1/x\r\n",
        )],
        0,
    );
    let url = format!("https://{}/a", s.dia_chi);
    let mut net = net_tin_goc(CERT_LOCALHOST);
    let (status, headers) = http_head(&mut net, &url, 3_000).expect("HEAD phải trả mã gốc");
    assert_eq!(status, 302);
    assert!(
        headers.iter().any(|(k, _)| k == "location"),
        "HEAD phải giữ header Location: {headers:?}"
    );
    s.dung();
}

/// Server nhận TCP nhưng **không** handshake ⇒ hạn của `http_get` phải bao cả handshake.
#[test]
fn https_het_gio_handshake_ro_rang() {
    let _k = kho();
    let l = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let dia_chi = l.local_addr().expect("local_addr").to_string();
    let tay = std::thread::spawn(move || {
        if let Ok((s, _)) = l.accept() {
            std::thread::sleep(Duration::from_millis(1_200));
            let _ = s.set_nonblocking(true);
        }
    });
    let mut net = net_tin_goc(CERT_LOCALHOST);
    let bat_dau = Instant::now();
    let e = http_get(&mut net, &format!("https://{dia_chi}/"), 300).expect_err("phải hết giờ");
    assert!(
        e.contains("quá thời gian TLS handshake"),
        "lỗi phải nêu hết giờ handshake: {e}"
    );
    assert!(
        bat_dau.elapsed() < Duration::from_millis(2_500),
        "handshake hết giờ quá chậm"
    );
    tay.join().expect("join server treo");
}

#[test]
fn vis_run_https_loi_tls_khong_panic() {
    let _k = kho();
    let s = may_chu_tls(
        CERT_LOCALHOST,
        KEY_LOCALHOST,
        vec![
            dap_ung("200 OK", b"khong-duoc", ""),
            dap_ung("200 OK", b"khong-duoc", ""),
        ],
        0,
    );
    let d = tam("run");
    let p = d.join("https.vis");
    std::fs::write(
        &p,
        format!(
            "# SPDX-License-Identifier: MIT OR Apache-2.0\nuse std.http\n\
             match http.get(\"https://{}/\"):\n\
             \x20   Ok(s): print(s)\n\
             \x20   Err(e): print(e)\n",
            s.dia_chi
        ),
    )
    .expect("ghi chương trình");

    let chay = |vm: bool| {
        let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
        c.arg("run");
        if vm {
            c.arg("--vm").arg("--no-cache");
        }
        let o = c.arg(&p).output().expect("chạy `vis run`");
        (
            o.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&o.stdout).to_string(),
            String::from_utf8_lossy(&o.stderr).to_string(),
        )
    };
    let a = chay(false);
    let b = chay(true);
    assert_eq!(a, b, "interp ≠ VM: {a:?} / {b:?}");
    assert_eq!(a.0, 0, "chương trình phải thoát 0 khi `Err` dữ liệu: {a:?}");
    assert!(a.1.contains("TLS"), "stdout phải nêu lỗi TLS: {a:?}");
    assert!(
        !a.2.contains("panicked"),
        "không được panic: stderr={:?}",
        a.2
    );
    s.dung();
}
