// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-4b2 (VD-271)** — HTTP/2 client + ALPN, server h2 local hermetic; feature `http2`.
//!
//! Ca: GET/HEAD h2, body/cap, fallback HTTP/1.1 khi server không chọn ALPN h2, timeout rõ.

#![cfg(all(feature = "http2", feature = "tls", not(target_arch = "wasm32")))]

use std::future::Future;
use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

use bytes::Bytes;
use http::{HeaderMap, Response};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ServerConfig, ServerConnection, StreamOwned};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use vis_cli::net::Net;
use vis_interp::net::{http_get, http_head};

const CERT: &[u8] = include_bytes!("fixtures/vs3_tls/localhost.cert.der");
const KEY: &[u8] = include_bytes!("fixtures/vs3_tls/localhost.key.der");

fn cau_hinh_tls(alpn: bool) -> Arc<ServerConfig> {
    let nha = Arc::new(rustls::crypto::ring::default_provider());
    let cert = CertificateDer::from(CERT.to_vec());
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(KEY.to_vec()));
    let mut cfg = ServerConfig::builder_with_provider(nha)
        .with_safe_default_protocol_versions()
        .expect("TLS versions")
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)
        .expect("server cert");
    if alpn {
        cfg.alpn_protocols = vec![b"h2".to_vec()];
    }
    Arc::new(cfg)
}

/// Kết quả route: `(status, headers, body)`.
type PhanHoi = (u16, Vec<(String, String)>, Vec<u8>);

struct MayChu {
    addr: String,
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
    so_ket_noi: Arc<AtomicUsize>,
}

impl MayChu {
    fn moi(
        alpn: bool,
        route: impl Fn(&str, &str, &HeaderMap) -> PhanHoi + Send + Sync + 'static,
    ) -> Self {
        let l = TcpListener::bind("127.0.0.1:0").expect("bind");
        l.set_nonblocking(true).expect("nonblocking");
        let addr = l.local_addr().expect("addr").to_string();
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = Arc::clone(&stop);
        let so_ket_noi = Arc::new(AtomicUsize::new(0));
        let so_ket_noi2 = Arc::clone(&so_ket_noi);
        let cfg = cau_hinh_tls(alpn);
        let route = Arc::new(route);
        let tay = std::thread::spawn(move || {
            while !stop2.load(Ordering::Relaxed) {
                match l.accept() {
                    Ok((tcp, _)) => {
                        so_ket_noi2.fetch_add(1, Ordering::SeqCst);
                        let _ = tcp.set_nonblocking(false);
                        let _ = tcp.set_read_timeout(Some(Duration::from_secs(2)));
                        let _ = tcp.set_write_timeout(Some(Duration::from_secs(2)));
                        let Ok(conn) = ServerConnection::new(Arc::clone(&cfg)) else {
                            continue;
                        };
                        let mut s = StreamOwned::new(conn, tcp);
                        if alpn {
                            let _ = serve_h2(s, &*route);
                        } else {
                            let _ = serve_h1(&mut s, &*route);
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(2));
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            addr,
            stop,
            handle: Some(tay),
            so_ket_noi,
        }
    }

    fn so_ket_noi(&self) -> usize {
        self.so_ket_noi.load(Ordering::SeqCst)
    }

    fn url(&self, path: &str) -> String {
        format!("https://{}{path}", self.addr)
    }
}

impl Drop for MayChu {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

type Route = dyn Fn(&str, &str, &HeaderMap) -> PhanHoi + Send + Sync;

fn serve_h1(s: &mut StreamOwned<ServerConnection, TcpStream>, route: &Route) -> io::Result<()> {
    let mut req = Vec::new();
    let mut buf = [0u8; 4096];
    while req.len() < 65_536 && !req.windows(4).any(|w| w == b"\r\n\r\n") {
        match s.read(&mut buf) {
            Ok(0) => return Ok(()),
            Ok(n) => req.extend_from_slice(&buf[..n]),
            Err(e) => return Err(e),
        }
    }
    let text = String::from_utf8_lossy(&req).to_string();
    let path = text
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("/");
    let (status, headers, body) = route("GET", path, &HeaderMap::new());
    let mut resp = format!(
        "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    for (k, v) in headers {
        resp.push_str(&format!("{k}: {v}\r\n"));
    }
    resp.push_str("\r\n");
    s.write_all(resp.as_bytes())?;
    s.write_all(&body)?;
    s.flush()?;
    let _ = s.sock.shutdown(Shutdown::Both);
    Ok(())
}

fn gui_than(send: &mut ::h2::SendStream<Bytes>, body: &[u8], end: bool) -> io::Result<()> {
    const CHUNK: usize = 64 * 1024;
    if body.is_empty() {
        if !end {
            send.send_data(Bytes::new(), false)
                .map_err(|e| io::Error::other(format!("h2 data rỗng: {e}")))?;
        }
        return Ok(());
    }
    let mut du = body;
    while !du.is_empty() {
        let n = du.len().min(CHUNK);
        let cuoi = n == du.len();
        send.send_data(Bytes::copy_from_slice(&du[..n]), end && cuoi)
            .map_err(|e| io::Error::other(format!("h2 data chunk: {e}")))?;
        du = &du[n..];
    }
    Ok(())
}

fn serve_h2(s: StreamOwned<ServerConnection, TcpStream>, route: &Route) -> io::Result<()> {
    s.sock.set_nonblocking(true)?;
    let io = Adapter {
        s,
        han: Instant::now() + Duration::from_secs(30),
    };
    let mut conn = cho(::h2::server::handshake(io))
        .map_err(|e| io::Error::other(format!("h2 server handshake: {e}")))?;
    let w = Waker::noop();
    let mut cx = Context::from_waker(w);
    loop {
        let (req, mut respond) = match conn.poll_accept(&mut cx) {
            Poll::Ready(Some(Ok(x))) => x,
            Poll::Ready(Some(Err(e))) => return Err(io::Error::other(format!("h2 accept: {e}"))),
            Poll::Ready(None) => return Ok(()),
            Poll::Pending => {
                std::thread::yield_now();
                continue;
            }
        };
        let method = req.method().as_str().to_string();
        let path = req.uri().path().to_string();
        let (status, headers, body) = route(&method, &path, req.headers());
        let mut rb = Response::builder().status(status);
        for (k, v) in headers {
            rb = rb.header(k, v);
        }
        let dac_biet = path.as_str();
        let end = body.is_empty() && !matches!(dac_biet, "/trailer" | "/cut" | "/rst");
        let mut send = respond
            .send_response(rb.body(()).map_err(io::Error::other)?, end)
            .map_err(|e| io::Error::other(format!("h2 response: {e}")))?;
        match dac_biet {
            "/rst" => {
                send.send_reset(::h2::Reason::CANCEL);
                continue;
            }
            "/cut" => {
                // Header khai Content-Length 100 nhưng chỉ gửi 3 byte rồi bỏ connection.
                send.send_data(Bytes::from_static(b"abc"), false)
                    .map_err(|e| io::Error::other(format!("h2 cut: {e}")))?;
                return Ok(());
            }
            "/goaway-mid" => {
                // Gửi header + một phần body rồi GOAWAY khi stream còn mở.
                send.send_data(Bytes::from_static(b"abc"), false)
                    .map_err(|e| io::Error::other(format!("h2 goaway-mid: {e}")))?;
                // Đẩy frame data đã xếp hàng ra socket trước khi GOAWAY.
                let _ = conn.poll_accept(&mut cx);
                conn.abrupt_shutdown(::h2::Reason::NO_ERROR);
                return Ok(());
            }
            "/stall-body" => {
                // Gửi header + một phần body rồi treo quá timeout của client.
                send.send_data(Bytes::from_static(b"abc"), false)
                    .map_err(|e| io::Error::other(format!("h2 stall-body: {e}")))?;
                std::thread::sleep(Duration::from_millis(900));
                return Ok(());
            }
            "/trailer" => {
                gui_than(&mut send, &body, false)?;
                let mut hm = HeaderMap::new();
                hm.insert("x-trailer", http::HeaderValue::from_static("1"));
                send.send_trailers(hm)
                    .map_err(|e| io::Error::other(format!("h2 trailer: {e}")))?;
            }
            _ => {
                gui_than(&mut send, &body, true)?;
            }
        }
        if dac_biet == "/goaway" {
            // Đóng mềm sau response: client nhận body rồi mới thấy GOAWAY ở request kế tiếp.
            conn.graceful_shutdown();
        }
    }
}

struct Adapter {
    s: StreamOwned<ServerConnection, TcpStream>,
    han: Instant,
}

impl AsyncRead for Adapter {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if buf.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        if this.han <= Instant::now() {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "quá thời gian h2 server",
            )));
        }
        match this.s.read(buf.initialize_unfilled()) {
            Ok(n) => {
                buf.advance(n);
                Poll::Ready(Ok(()))
            }
            Err(e) if tam_thoi(&e) => Poll::Pending,
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

impl AsyncWrite for Adapter {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        if this.han <= Instant::now() {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "quá thời gian h2 server",
            )));
        }
        match this.s.write(buf) {
            Ok(n) => Poll::Ready(Ok(n)),
            Err(e) if tam_thoi(&e) => Poll::Pending,
            Err(e) => Poll::Ready(Err(e)),
        }
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        match this.s.flush() {
            Ok(()) => Poll::Ready(Ok(())),
            Err(e) if tam_thoi(&e) => Poll::Pending,
            Err(e) => Poll::Ready(Err(e)),
        }
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_flush(cx)
    }
}

fn tam_thoi(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut | io::ErrorKind::Interrupted
    )
}

fn cho<F: Future>(f: F) -> F::Output {
    let w = Waker::noop();
    let mut cx = Context::from_waker(w);
    let mut f = Box::pin(f);
    loop {
        match f.as_mut().poll(&mut cx) {
            Poll::Ready(v) => return v,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

fn tra_loi(body: &str) -> PhanHoi {
    (200, vec![], body.as_bytes().to_vec())
}

#[test]
fn h2_tai_su_dung_ket_noi() {
    let s = MayChu::moi(true, |method, path, _| match (method, path) {
        ("GET", "/a") => (200, vec![], b"a".to_vec()),
        ("GET", "/b") => (200, vec![], b"b".to_vec()),
        ("GET", "/c") => (200, vec![], b"c".to_vec()),
        ("GET", "/d") => (200, vec![], b"d".to_vec()),
        _ => (404, vec![], vec![]),
    });
    let mut net = Net::moi_voi_goc_tls(false, &[CERT]).expect("root test");
    assert_eq!(http_get(&mut net, &s.url("/a"), 3_000).expect("a"), "a");
    assert_eq!(http_get(&mut net, &s.url("/b"), 3_000).expect("b"), "b");
    assert_eq!(http_get(&mut net, &s.url("/c"), 3_000).expect("c"), "c");
    assert_eq!(http_get(&mut net, &s.url("/d"), 3_000).expect("d"), "d");
    assert_eq!(
        s.so_ket_noi(),
        1,
        "bốn GET h2 cùng authority phải dùng lại một connection (hàng đợi tuần tự)"
    );
}

#[test]
fn h2_redirect_va_cookie() {
    let s = MayChu::moi(true, |method, path, headers| match (method, path) {
        ("GET", "/a") => (
            302,
            vec![
                ("location".to_string(), "/b".to_string()),
                ("set-cookie".to_string(), "sid=1; Path=/".to_string()),
            ],
            vec![],
        ),
        ("GET", "/b") => {
            assert_eq!(
                headers.get("cookie").and_then(|v| v.to_str().ok()),
                Some("sid=1"),
                "cookie từ hop redirect phải được gửi"
            );
            (200, vec![], b"dich".to_vec())
        }
        _ => (404, vec![], vec![]),
    });
    let mut net = Net::moi_voi_goc_tls(false, &[CERT]).expect("root test");
    assert_eq!(
        http_get(&mut net, &s.url("/a"), 3_000).expect("h2 redirect"),
        "dich"
    );
}

#[test]
fn h2_basic_auth() {
    let s = MayChu::moi(true, |method, path, headers| match (method, path) {
        ("GET", "/auth") => {
            assert_eq!(
                headers.get("authorization").and_then(|v| v.to_str().ok()),
                Some("Basic dXNlcjpwYXNz")
            );
            (200, vec![], b"auth".to_vec())
        }
        _ => (404, vec![], vec![]),
    });
    let mut net = Net::moi_voi_goc_tls(false, &[CERT]).expect("root test");
    let u = format!("https://user:pass@{}/auth", s.addr);
    assert_eq!(http_get(&mut net, &u, 3_000).expect("h2 auth"), "auth");
}

#[test]
fn h2_get_va_head() {
    let s = MayChu::moi(true, |method, path, _headers| match (method, path) {
        ("GET", "/a") => (
            200,
            vec![("x-h2".to_string(), "1".to_string())],
            b"h2-body".to_vec(),
        ),
        ("HEAD", "/a") => (204, vec![("x-h2".to_string(), "head".to_string())], vec![]),
        _ => (404, vec![], vec![]),
    });
    let mut net = Net::moi_voi_goc_tls(false, &[CERT]).expect("root test");
    assert_eq!(
        http_get(&mut net, &s.url("/a"), 3_000).expect("h2 GET"),
        "h2-body"
    );
    let (st, hs) = http_head(&mut net, &s.url("/a"), 3_000).expect("h2 HEAD");
    assert_eq!(st, 204);
    assert!(hs.iter().any(|(k, v)| k == "x-h2" && v == "head"));
}

#[test]
fn fallback_http1_khi_khong_alpn() {
    let s = MayChu::moi(false, |_, path, _headers| match path {
        "/a" => tra_loi("h1-body"),
        _ => (404, vec![], vec![]),
    });
    let mut net = Net::moi_voi_goc_tls(false, &[CERT]).expect("root test");
    assert_eq!(
        http_get(&mut net, &s.url("/a"), 3_000).expect("fallback"),
        "h1-body"
    );
}

#[test]
fn h2_timeout_ro_rang() {
    let s = MayChu::moi(true, |_, _, _headers| {
        std::thread::sleep(Duration::from_millis(900));
        tra_loi("muon")
    });
    let mut net = Net::moi_voi_goc_tls(false, &[CERT]).expect("root test");
    let bat = Instant::now();
    let e = http_get(&mut net, &s.url("/cham"), 250).expect_err("phải hết giờ");
    assert!(bat.elapsed() < Duration::from_millis(2_000));
    assert!(e.contains("HTTP/2") || e.contains("thời gian"), "{e}");
}

fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

fn b64(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
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

fn tar(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut out = Vec::new();
    for (path, noi_dung) in entries {
        let data = noi_dung.as_bytes();
        let mut h = [0u8; 512];
        let p = path.as_bytes();
        h[..p.len()].copy_from_slice(p);
        h[100..108].copy_from_slice(b"0000644\0");
        h[124..136].copy_from_slice(format!("{:011o}\0", data.len()).as_bytes());
        h[136..148].copy_from_slice(b"00000000000\0");
        for x in &mut h[148..156] {
            *x = b' ';
        }
        h[156] = b'0';
        h[257..263].copy_from_slice(b"ustar\0");
        h[263..265].copy_from_slice(b"00");
        let sum: u32 = h.iter().map(|&x| u32::from(x)).sum();
        h[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
        out.extend_from_slice(&h);
        out.extend_from_slice(data);
        let pad = (512 - (data.len() % 512)) % 512;
        out.extend(std::iter::repeat_n(0u8, pad));
    }
    out.extend(std::iter::repeat_n(0u8, 1_024));
    out
}

#[test]
fn h2_trailer_roi_moi_tai_su_dung() {
    let s = MayChu::moi(true, |method, path, _| match (method, path) {
        ("GET", "/trailer") => (200, vec![], b"t".to_vec()),
        ("GET", "/b") => (200, vec![], b"b".to_vec()),
        _ => (404, vec![], vec![]),
    });
    let mut net = Net::moi_voi_goc_tls(false, &[CERT]).expect("root test");
    assert_eq!(
        http_get(&mut net, &s.url("/trailer"), 3_000).expect("trailer"),
        "t"
    );
    assert_eq!(http_get(&mut net, &s.url("/b"), 3_000).expect("b"), "b");
    assert_eq!(s.so_ket_noi(), 1, "trailer xong phải tái dùng connection");
}

#[test]
fn h2_goaway_retry_mot_lan() {
    let s = MayChu::moi(true, |method, path, _| match (method, path) {
        ("GET", "/goaway") => (200, vec![], b"g".to_vec()),
        ("GET", "/b") => (200, vec![], b"b".to_vec()),
        _ => (404, vec![], vec![]),
    });
    let mut net = Net::moi_voi_goc_tls(false, &[CERT]).expect("root test");
    assert_eq!(
        http_get(&mut net, &s.url("/goaway"), 3_000).expect("goaway"),
        "g"
    );
    assert_eq!(
        http_get(&mut net, &s.url("/b"), 3_000).expect("retry sau GOAWAY"),
        "b"
    );
    assert_eq!(s.so_ket_noi(), 2, "GOAWAY phải mở connection mới");
}

#[test]
fn h2_rst_stream_loi_ro() {
    let s = MayChu::moi(true, |_, _, _| (200, vec![], b"x".to_vec()));
    let mut net = Net::moi_voi_goc_tls(false, &[CERT]).expect("root test");
    let bat = Instant::now();
    let e = http_get(&mut net, &s.url("/rst"), 2_000).expect_err("RST phải lỗi");
    assert!(
        e.contains("RST_STREAM") || e.contains("HTTP/2"),
        "lỗi phải rõ: {e}"
    );
    assert!(bat.elapsed() < Duration::from_millis(4_000), "không treo");
}

#[test]
fn h2_khong_body_va_header_lon() {
    let s = MayChu::moi(true, |method, path, _| match (method, path) {
        ("GET", "/empty") => (204, vec![], vec![]),
        ("GET", "/big") => (200, vec![("x-big".to_string(), "a".repeat(70_000))], vec![]),
        _ => (404, vec![], vec![]),
    });
    let mut net = Net::moi_voi_goc_tls(false, &[CERT]).expect("root test");
    assert_eq!(
        http_get(&mut net, &s.url("/empty"), 3_000).expect("empty"),
        ""
    );
    let e = http_get(&mut net, &s.url("/big"), 3_000).expect_err("header lớn phải lỗi");
    assert!(
        e.contains("header HTTP/2") || e.contains("HTTP/2"),
        "lỗi phải rõ: {e}"
    );
}

#[test]
fn h2_server_dong_giua_chung() {
    let s = MayChu::moi(true, |_, _, _| {
        (
            200,
            vec![("content-length".to_string(), "100".to_string())],
            vec![],
        )
    });
    let mut net = Net::moi_voi_goc_tls(false, &[CERT]).expect("root test");
    let bat = Instant::now();
    let e = http_get(&mut net, &s.url("/cut"), 600).expect_err("server đóng giữa chừng phải lỗi");
    assert!(
        e.contains("HTTP/2") || e.contains("body") || e.contains("kết nối"),
        "{e}"
    );
    assert!(bat.elapsed() < Duration::from_millis(3_000), "không treo");
}

/// **VS3-4b4**: phiên h2 idle quá 15 s bị đóng; request sau mở connection mới.
#[test]
fn h2_idle_timeout_dong_phien() {
    let s = MayChu::moi(true, |method, path, _| match (method, path) {
        ("GET", "/a") => (200, vec![], b"a".to_vec()),
        ("GET", "/b") => (200, vec![], b"b".to_vec()),
        _ => (404, vec![], vec![]),
    });
    let mut net = Net::moi_voi_goc_tls(false, &[CERT]).expect("root test");
    assert_eq!(http_get(&mut net, &s.url("/a"), 3_000).expect("a"), "a");
    std::thread::sleep(Duration::from_millis(15_200));
    assert_eq!(http_get(&mut net, &s.url("/b"), 3_000).expect("b"), "b");
    assert_eq!(s.so_ket_noi(), 2, "sau idle 15 s phải mở connection mới");
}

/// **VS3-W2**: registry `vis pkg` thật sự thương lượng h2 (không fallback h1).
#[test]
fn registry_https_thuong_luong_h2() {
    if !cfg!(debug_assertions) {
        eprintln!("bỏ qua registry_https_thuong_luong_h2: hook root chỉ có ở bản debug");
        return;
    }
    let noi_dung = "fn util() -> i64:\n    7\n";
    let file_sha = hex(&vis_bytecode::bytes_hash(noi_dung.as_bytes()));
    let tar_bytes = tar(&[("src/util.vis", noi_dung)]);
    let tar_sha = hex(&vis_bytecode::bytes_hash(&tar_bytes));
    let manifest = format!(
        "{{\"name\":\"util\",\"version\":\"0.1.0\",\"entry\":\"src/util.vis\",\"deps\":{{}},\"files\":[{{\"path\":\"src/util.vis\",\"sha256\":\"{file_sha}\"}}]}}"
    );
    let index = format!(
        "{{\"format\":\"vispkg-http1\",\"packages\":{{\"util\":{{\"0.1.0\":{{\"tarball\":\"util/0.1.0.tar.b64\",\"tarball_sha256\":\"{tar_sha}\",\"manifest\":{manifest}}}}}}}}}\n"
    );
    let tar_b64 = b64(&tar_bytes);
    let s = MayChu::moi(true, move |_, path, _| match path {
        "/index.json" => (200, vec![], index.as_bytes().to_vec()),
        "/util/0.1.0.tar.b64" => (200, vec![], tar_b64.as_bytes().to_vec()),
        _ => (404, vec![], vec![]),
    });
    let d = std::env::temp_dir().join(format!("vis-h2-reg-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tmp");
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vs3_tls/localhost.cert.der");
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .current_dir(&d)
        .args(["pkg", "add", "util", "--registry", &s.url("")])
        .env("VIS_TLS_TEST_ROOT_DER", root)
        .env_remove("VIS_REGISTRY")
        .env_remove("VIS_REGISTRY_KEY")
        .output()
        .expect("chạy vis pkg");
    assert_eq!(
        o.status.code(),
        Some(0),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(
        d.join(".vis/pkg/util/0.1.0/src/util.vis").is_file(),
        "không cài được qua h2"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// **VS3-4b7 stress**: 100 request tuần tự qua 4 authority h2; mỗi authority **1 connection**;
/// idle 15,2 s đóng cả 4 phiên; request sau mở connection mới; không treo/rò.
#[test]
fn stress_100_request_4_authority() {
    let mut ds = Vec::new();
    for i in 0..4 {
        ds.push(MayChu::moi(true, move |_, path, _| {
            (200, vec![], format!("{i}:{path}").into_bytes())
        }));
    }
    let mut net = Net::moi_voi_goc_tls(false, &[CERT]).expect("root test");
    let bd = Instant::now();
    for i in 0..100 {
        let a = i % 4;
        let path = format!("/r{i}");
        let body = http_get(&mut net, &ds[a].url(&path), 3_000)
            .unwrap_or_else(|e| panic!("request {i} authority {a}: {e}"));
        assert_eq!(body, format!("{a}:{path}"));
    }
    for (i, s) in ds.iter().enumerate() {
        assert_eq!(
            s.so_ket_noi(),
            1,
            "authority {i} phải chỉ 1 connection cho 25 request"
        );
    }
    eprintln!(
        "100 request: {:.0} ms; connection/authority = {:?}",
        bd.elapsed().as_secs_f64() * 1_000.0,
        ds.iter().map(MayChu::so_ket_noi).collect::<Vec<_>>()
    );

    // Idle 15 s: lần request kế tiếp của mỗi authority phải mở connection mới.
    std::thread::sleep(Duration::from_millis(15_200));
    let bd_idle = Instant::now();
    for (i, s) in ds.iter().enumerate() {
        let path = format!("/idle{i}");
        let body = http_get(&mut net, &s.url(&path), 3_000).expect("sau idle");
        assert_eq!(body, format!("{i}:{path}"));
    }
    for (i, s) in ds.iter().enumerate() {
        assert_eq!(s.so_ket_noi(), 2, "authority {i} phải mở lại sau idle");
    }
    eprintln!(
        "sau idle 15,2 s: {:.0} ms; connection/authority = {:?}",
        bd_idle.elapsed().as_secs_f64() * 1_000.0,
        ds.iter().map(MayChu::so_ket_noi).collect::<Vec<_>>()
    );
}

/// **VS3-W2 release limits**: body đúng 8 MiB phải nhận đủ; vượt 1 byte phải lỗi rõ,
/// không treo; phiên lỗi bị bỏ và request sau mở connection mới.
#[test]
fn h2_body_8mib_va_vuot_cap() {
    const MAX: usize = 8 * 1024 * 1024;
    let s = MayChu::moi(true, |method, path, _| match (method, path) {
        ("GET", "/full") => (200, vec![], vec![b'x'; MAX]),
        ("GET", "/over") => (200, vec![], vec![b'y'; MAX + 1]),
        ("GET", "/b") => (200, vec![], b"b".to_vec()),
        _ => (404, vec![], vec![]),
    });
    let mut net = Net::moi_voi_goc_tls(false, &[CERT]).expect("root test");
    let body = http_get(&mut net, &s.url("/full"), 30_000).expect("body đúng 8 MiB");
    assert_eq!(body.len(), MAX, "phải nhận đủ 8 MiB");
    let raw = body.as_bytes();
    assert_eq!(raw[0], b'x');
    assert_eq!(raw[MAX - 1], b'x');

    let bat = Instant::now();
    let e = http_get(&mut net, &s.url("/over"), 20_000).expect_err("body vượt cap phải lỗi");
    assert!(
        e.contains("8 MiB") || e.contains("vượt giới hạn") || e.contains("HTTP/2"),
        "lỗi phải rõ: {e}"
    );
    assert!(bat.elapsed() < Duration::from_millis(25_000), "không treo");
    assert_eq!(
        s.so_ket_noi(),
        1,
        "sau 8 MiB, request vượt cap phải dùng lại connection cũ"
    );

    assert_eq!(
        http_get(&mut net, &s.url("/b"), 5_000).expect("sau body lỗi"),
        "b"
    );
    assert_eq!(
        s.so_ket_noi(),
        2,
        "phiên lỗi phải bị bỏ, request sau mở connection mới"
    );
}

/// **VS3-W2 release limits**: header tổng đúng 64 KiB phải nhận; vượt 1 byte phải lỗi rõ;
/// connection sạch sau lỗi được thay bằng connection mới.
#[test]
fn h2_header_sat_cap_va_vuot() {
    const MAX: usize = 64 * 1024;
    let ten = "x-near";
    let gia_tri = "a".repeat(MAX - ten.len());
    let ten_vuot = "x-over";
    let gia_tri_vuot = "b".repeat(MAX - ten_vuot.len() + 1);
    let s = MayChu::moi(true, move |method, path, _| match (method, path) {
        ("HEAD", "/near") => (200, vec![(ten.to_string(), gia_tri.clone())], vec![]),
        ("GET", "/over") => (
            200,
            vec![(ten_vuot.to_string(), gia_tri_vuot.clone())],
            vec![],
        ),
        ("GET", "/b") => (200, vec![], b"b".to_vec()),
        _ => (404, vec![], vec![]),
    });
    let mut net = Net::moi_voi_goc_tls(false, &[CERT]).expect("root test");
    let (st, hs) = http_head(&mut net, &s.url("/near"), 10_000).expect("header sát cap");
    assert_eq!(st, 200);
    let got = hs
        .iter()
        .find(|(k, _)| k == ten)
        .map(|(_, v)| v.len())
        .expect("header sát cap phải có");
    assert_eq!(got, MAX - ten.len(), "tổng name+value đúng 64 KiB");

    let bat = Instant::now();
    let e = http_get(&mut net, &s.url("/over"), 10_000).expect_err("header vượt cap phải lỗi");
    assert!(
        e.contains("header HTTP/2") || e.contains("64") || e.contains("HTTP/2"),
        "lỗi phải rõ: {e}"
    );
    assert!(bat.elapsed() < Duration::from_millis(10_000), "không treo");

    assert_eq!(
        http_get(&mut net, &s.url("/b"), 5_000).expect("sau header lỗi"),
        "b"
    );
    assert_eq!(
        s.so_ket_noi(),
        2,
        "phiên lỗi phải bị bỏ, request sau mở connection mới"
    );
}

/// **VS3-W2 release limits**: body 1 MiB + trailer phải đọc hết trailer và tái dùng
/// connection cho request kế tiếp.
#[test]
fn h2_trailer_body_lon_va_tai_su_dung() {
    const N: usize = 1024 * 1024;
    let s = MayChu::moi(true, |method, path, _| match (method, path) {
        ("GET", "/trailer") => (200, vec![], vec![b't'; N]),
        ("GET", "/b") => (200, vec![], b"b".to_vec()),
        _ => (404, vec![], vec![]),
    });
    let mut net = Net::moi_voi_goc_tls(false, &[CERT]).expect("root test");
    let body = http_get(&mut net, &s.url("/trailer"), 15_000).expect("trailer + body lớn");
    assert_eq!(body.len(), N);
    assert_eq!(body.as_bytes()[0], b't');
    assert_eq!(body.as_bytes()[N - 1], b't');
    assert_eq!(
        http_get(&mut net, &s.url("/b"), 3_000).expect("sau trailer"),
        "b"
    );
    assert_eq!(s.so_ket_noi(), 1, "trailer xong phải tái dùng connection");
}

/// **VS3-W2 release limits**: GOAWAY khi stream còn body ⇒ lỗi rõ, không treo; request sau
/// dùng connection mới (không dùng lại phiên đã GOAWAY).
#[test]
fn h2_goaway_giua_body() {
    let s = MayChu::moi(true, |method, path, _| match (method, path) {
        ("GET", "/goaway-mid") => (200, vec![], b"abc".to_vec()),
        ("GET", "/b") => (200, vec![], b"b".to_vec()),
        _ => (404, vec![], vec![]),
    });
    let mut net = Net::moi_voi_goc_tls(false, &[CERT]).expect("root test");
    let bat = Instant::now();
    let e =
        http_get(&mut net, &s.url("/goaway-mid"), 1_500).expect_err("GOAWAY giữa body phải lỗi");
    assert!(
        e.contains("GOAWAY") || e.contains("HTTP/2") || e.contains("body"),
        "lỗi phải rõ: {e}"
    );
    assert!(bat.elapsed() < Duration::from_millis(4_000), "không treo");
    let truoc = s.so_ket_noi();
    assert_eq!(
        http_get(&mut net, &s.url("/b"), 3_000).expect("sau GOAWAY"),
        "b"
    );
    assert!(
        s.so_ket_noi() > truoc,
        "request sau GOAWAY phải mở connection mới ({truoc} -> {})",
        s.so_ket_noi()
    );
}

/// **VS3-W2 release limits**: timeout khi server đã gửi header + một phần body ⇒ lỗi rõ,
/// không treo; request sau mở connection mới.
#[test]
fn h2_timeout_giua_body_va_cleanup() {
    let s = MayChu::moi(true, |method, path, _| match (method, path) {
        ("GET", "/stall-body") => (200, vec![], b"abc".to_vec()),
        ("GET", "/b") => (200, vec![], b"b".to_vec()),
        _ => (404, vec![], vec![]),
    });
    let mut net = Net::moi_voi_goc_tls(false, &[CERT]).expect("root test");
    let bat = Instant::now();
    let e = http_get(&mut net, &s.url("/stall-body"), 250).expect_err("phải timeout");
    assert!(
        e.contains("thời gian") || e.contains("HTTP/2") || e.contains("TimedOut"),
        "lỗi phải rõ: {e}"
    );
    assert!(bat.elapsed() < Duration::from_millis(3_000), "không treo");
    let truoc = s.so_ket_noi();
    assert_eq!(
        http_get(&mut net, &s.url("/b"), 3_000).expect("sau timeout"),
        "b"
    );
    assert!(
        s.so_ket_noi() > truoc,
        "request sau timeout phải mở connection mới ({truoc} -> {})",
        s.so_ket_noi()
    );
}

/// **VS3-W2 spike** — HTTP/2 reactor-native (`feature = "async-h2"`, default-off).
/// Chỉ chạy khi bật feature; 19 test sync phía trên không đổi.
#[cfg(feature = "async-h2")]
mod h2_async_spike {
    use super::*;
    use std::fs;
    use vis_cli::net::h2_async::{h2_get_async_voi_goc, phien_voi_goc};

    #[derive(Clone, Copy)]
    enum Kieu {
        Thuong,
        Cho4,
        Warm4,
        GoAway,
        Treo,
    }

    struct MayChuSpike {
        addr: String,
        stop: Arc<AtomicBool>,
        conns: Arc<AtomicUsize>,
        streams: Arc<AtomicUsize>,
        handle: Option<std::thread::JoinHandle<()>>,
    }

    impl MayChuSpike {
        fn moi(kieu: Kieu) -> Self {
            let l = TcpListener::bind("127.0.0.1:0").expect("bind");
            l.set_nonblocking(true).expect("nonblocking");
            let addr = l.local_addr().expect("addr").to_string();
            let stop = Arc::new(AtomicBool::new(false));
            let stop2 = Arc::clone(&stop);
            let conns = Arc::new(AtomicUsize::new(0));
            let conns2 = Arc::clone(&conns);
            let streams = Arc::new(AtomicUsize::new(0));
            let streams2 = Arc::clone(&streams);
            let handle = std::thread::spawn(move || {
                while !stop2.load(Ordering::Relaxed) {
                    match l.accept() {
                        Ok((tcp, _)) => {
                            conns2.fetch_add(1, Ordering::SeqCst);
                            let _ = serve_spike(tcp, kieu, Arc::clone(&streams2));
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(2));
                        }
                        Err(_) => break,
                    }
                }
            });
            Self {
                addr,
                stop,
                conns,
                streams,
                handle: Some(handle),
            }
        }

        fn url(&self, path: &str) -> String {
            format!("https://{}{path}", self.addr)
        }

        fn conns(&self) -> usize {
            self.conns.load(Ordering::SeqCst)
        }

        fn streams(&self) -> usize {
            self.streams.load(Ordering::SeqCst)
        }
    }

    impl Drop for MayChuSpike {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(h) = self.handle.take() {
                let _ = h.join();
            }
        }
    }

    fn serve_spike(tcp: TcpStream, kieu: Kieu, streams: Arc<AtomicUsize>) -> io::Result<()> {
        tcp.set_nonblocking(false)?;
        tcp.set_read_timeout(Some(Duration::from_secs(5)))?;
        tcp.set_write_timeout(Some(Duration::from_secs(5)))?;
        let conn = ServerConnection::new(cau_hinh_tls(true)).map_err(io::Error::other)?;
        let s = StreamOwned::new(conn, tcp);
        s.sock.set_nonblocking(true)?;
        let io = Adapter {
            s,
            han: Instant::now() + Duration::from_secs(30),
        };
        let mut conn = cho(::h2::server::handshake(io))
            .map_err(|e| io::Error::other(format!("h2 server: {e}")))?;
        let w = Waker::noop();
        let mut cx = Context::from_waker(w);
        match kieu {
            Kieu::Thuong => loop {
                match conn.poll_accept(&mut cx) {
                    Poll::Ready(Some(Ok((req, mut respond)))) => {
                        streams.fetch_add(1, Ordering::SeqCst);
                        let head = req.method() == http::Method::HEAD;
                        let path = req.uri().path().to_string();
                        let body: Vec<u8> = if path == "/big" {
                            vec![b'z'; 8 * 1024 * 1024 + 1]
                        } else if path == "/hello" {
                            b"hello".to_vec()
                        } else {
                            path.clone().into_bytes()
                        };
                        let mut rb = Response::builder().status(200);
                        let chuyen = path == "/redir";
                        if chuyen {
                            rb = rb.status(302).header("location", "/hello");
                        }
                        let resp = rb.body(()).map_err(io::Error::other)?;
                        let mut send = respond
                            .send_response(resp, head || chuyen)
                            .map_err(|e| io::Error::other(format!("h2 response: {e}")))?;
                        if !head && !chuyen {
                            gui_than(&mut send, &body, true)?;
                        }
                    }
                    Poll::Ready(Some(Err(e))) => {
                        return Err(io::Error::other(format!("h2 accept: {e}")))
                    }
                    Poll::Ready(None) => return Ok(()),
                    Poll::Pending => std::thread::yield_now(),
                }
            },
            Kieu::Cho4 => {
                // Nhận đủ 4 stream TRƯỚC khi trả bất kỳ response nào ⇒ chứng minh in-flight.
                let mut ds = Vec::new();
                let han = Instant::now() + Duration::from_secs(10);
                while ds.len() < 4 {
                    if Instant::now() >= han {
                        return Err(io::Error::other("server không nhận đủ 4 stream"));
                    }
                    match conn.poll_accept(&mut cx) {
                        Poll::Ready(Some(Ok(x))) => {
                            streams.fetch_add(1, Ordering::SeqCst);
                            ds.push(x);
                        }
                        Poll::Ready(Some(Err(e))) => {
                            return Err(io::Error::other(format!("h2 accept: {e}")))
                        }
                        Poll::Ready(None) => return Ok(()),
                        Poll::Pending => std::thread::yield_now(),
                    }
                }
                // Trả ngược thứ tự để client không thể dựa vào thứ tự stream.
                for (req, mut respond) in ds.into_iter().rev() {
                    let body = Bytes::from(req.uri().path().as_bytes().to_vec());
                    let resp = Response::builder()
                        .status(200)
                        .body(())
                        .map_err(io::Error::other)?;
                    let mut send = respond
                        .send_response(resp, false)
                        .map_err(|e| io::Error::other(format!("h2 response: {e}")))?;
                    send.send_data(body, true)
                        .map_err(|e| io::Error::other(format!("h2 data: {e}")))?;
                }
                loop {
                    match conn.poll_closed(&mut cx) {
                        Poll::Ready(_) => return Ok(()),
                        Poll::Pending => std::thread::yield_now(),
                    }
                }
            }
            Kieu::Warm4 => {
                // Trả ngay `/warm`; sau đó chờ đủ 4 stream khác rồi mới trả (multiplex thật
                // trên connection đã ấm, không phụ thuộc race handshake đầu tiên).
                let mut ds = Vec::new();
                let han = Instant::now() + Duration::from_secs(10);
                while ds.len() < 4 {
                    if Instant::now() >= han {
                        return Err(io::Error::other("server không nhận đủ 4 stream ấm"));
                    }
                    match conn.poll_accept(&mut cx) {
                        Poll::Ready(Some(Ok((req, mut respond)))) => {
                            streams.fetch_add(1, Ordering::SeqCst);
                            let path = req.uri().path().to_string();
                            if path == "/warm" {
                                let resp = Response::builder()
                                    .status(200)
                                    .body(())
                                    .map_err(io::Error::other)?;
                                let mut send = respond
                                    .send_response(resp, false)
                                    .map_err(|e| io::Error::other(format!("h2 warm: {e}")))?;
                                send.send_data(Bytes::from_static(b"/warm"), true)
                                    .map_err(|e| io::Error::other(format!("h2 warm data: {e}")))?;
                            } else {
                                ds.push((req, respond));
                            }
                        }
                        Poll::Ready(Some(Err(e))) => {
                            return Err(io::Error::other(format!("h2 accept: {e}")))
                        }
                        Poll::Ready(None) => return Ok(()),
                        Poll::Pending => std::thread::yield_now(),
                    }
                }
                for (req, mut respond) in ds.into_iter().rev() {
                    let body = Bytes::from(req.uri().path().as_bytes().to_vec());
                    let resp = Response::builder()
                        .status(200)
                        .body(())
                        .map_err(io::Error::other)?;
                    let mut send = respond
                        .send_response(resp, false)
                        .map_err(|e| io::Error::other(format!("h2 response: {e}")))?;
                    send.send_data(body, true)
                        .map_err(|e| io::Error::other(format!("h2 data: {e}")))?;
                }
                loop {
                    match conn.poll_closed(&mut cx) {
                        Poll::Ready(_) => return Ok(()),
                        Poll::Pending => std::thread::yield_now(),
                    }
                }
            }
            Kieu::GoAway => {
                // GOAWAY ngay sau handshake, trước khi accept stream nào: last_stream_id = 0.
                conn.abrupt_shutdown(::h2::Reason::INTERNAL_ERROR);
                let han = Instant::now() + Duration::from_secs(2);
                while Instant::now() < han {
                    match conn.poll_closed(&mut cx) {
                        Poll::Ready(_) => break,
                        Poll::Pending => std::thread::yield_now(),
                    }
                }
                // Giữ socket thêm một nhịp để GOAWAY đã xếp hàng tới client trước FIN/RST.
                std::thread::sleep(Duration::from_millis(100));
                Ok(())
            }
            Kieu::Treo => loop {
                match conn.poll_closed(&mut cx) {
                    Poll::Ready(_) => return Ok(()),
                    Poll::Pending => std::thread::yield_now(),
                }
            },
        }
    }

    fn dem_fd() -> usize {
        fs::read_dir("/proc/self/fd")
            .map(|d| d.count())
            .unwrap_or(0)
    }

    #[test]
    fn async_h2_get_va_head() {
        let s = MayChuSpike::moi(Kieu::Thuong);
        let kq = h2_get_async_voi_goc(&s.url("/hello"), 5_000, &[CERT]).expect("GET async h2");
        assert_eq!(kq.status, 200);
        assert_eq!(kq.body, b"hello");
        assert_eq!(s.conns(), 1);

        let mut p = phien_voi_goc(&s.addr, 5_000, &[CERT]).expect("phiên h2");
        let id = p.gui("HEAD", "/hello", &[]).expect("gửi HEAD");
        p.tai(5_000).expect("HEAD xong");
        let kq = p.ket_qua(id).expect("có kết quả").expect("HEAD ok");
        assert_eq!(kq.status, 200);
        assert!(kq.body.is_empty(), "HEAD không có body");
        assert_eq!(s.streams(), 2);
    }

    #[test]
    fn async_h2_bon_stream_mot_connection() {
        let s = MayChuSpike::moi(Kieu::Cho4);
        let mut p = phien_voi_goc(&s.addr, 5_000, &[CERT]).expect("phiên h2");
        let ids: Vec<u64> = (0..4)
            .map(|i| p.gui("GET", &format!("/{i}"), &[]).expect("gửi"))
            .collect();
        let bat = Instant::now();
        p.tai(5_000).expect("4 stream xong");
        eprintln!(
            "4 stream/1 connection: {:.2} ms, connection={}, stream={}",
            bat.elapsed().as_secs_f64() * 1_000.0,
            s.conns(),
            s.streams()
        );
        for (i, id) in ids.into_iter().enumerate() {
            let kq = p.ket_qua(id).expect("có kết quả").expect("stream ok");
            assert_eq!(kq.body, format!("/{i}").into_bytes(), "stream {i}");
        }
        assert_eq!(s.conns(), 1, "chỉ một connection");
        assert_eq!(s.streams(), 4, "server nhận đủ 4 stream trước khi trả");
    }

    #[test]
    fn async_h2_goaway_loi_ro() {
        let s = MayChuSpike::moi(Kieu::GoAway);
        let mut p = phien_voi_goc(&s.addr, 5_000, &[CERT]).expect("phiên h2");
        let bat = Instant::now();
        // GOAWAY có thể tới trước hoặc sau khi request được xếp; cả hai đều phải lỗi rõ,
        // không được treo và không được "thành công im lặng".
        let id = match p.gui("GET", "/x", &[]) {
            Ok(id) => id,
            Err(e) => {
                assert!(
                    e.contains("GOAWAY") || e.contains("HTTP/2"),
                    "lỗi phải rõ: {e}"
                );
                return;
            }
        };
        let kq = match p.tai(3_000) {
            Ok(()) => p.ket_qua(id).expect("có kết quả").map(|_| ()),
            Err(e) => Err(e),
        };
        let e = kq.expect_err("GOAWAY phải lỗi");
        eprintln!("GOAWAY: {e}");
        assert!(e.contains("GOAWAY"), "lỗi phải nêu GOAWAY: {e}");
        assert!(
            !e.contains("quá thời gian") && bat.elapsed() < Duration::from_secs(3),
            "GOAWAY phải được reactor đánh thức, không chờ hết hạn: {e}"
        );
    }

    #[test]
    fn async_h2_timeout_khong_treo() {
        let s = MayChuSpike::moi(Kieu::Treo);
        let bat = Instant::now();
        let e = h2_get_async_voi_goc(&s.url("/x"), 400, &[CERT])
            .map(|_| ())
            .expect_err("phải timeout");
        eprintln!(
            "timeout sau {:.1} ms: {e}",
            bat.elapsed().as_secs_f64() * 1_000.0
        );
        assert!(e.contains("thời gian"), "lỗi phải rõ: {e}");
        assert!(bat.elapsed() < Duration::from_secs(3), "không treo: {e}");
    }

    #[test]
    fn async_h2_cap_body() {
        let s = MayChuSpike::moi(Kieu::Thuong);
        let bat = Instant::now();
        let e = h2_get_async_voi_goc(&s.url("/big"), 30_000, &[CERT])
            .map(|_| ())
            .expect_err("body vượt 8 MiB phải lỗi");
        eprintln!(
            "cap body sau {:.1} ms: {e}",
            bat.elapsed().as_secs_f64() * 1_000.0
        );
        assert!(
            e.contains("8 MiB") || e.contains("vượt giới hạn") || e.contains("HTTP/2"),
            "lỗi phải rõ: {e}"
        );
        assert!(bat.elapsed() < Duration::from_secs(30), "không treo");
        let kq = h2_get_async_voi_goc(&s.url("/hello"), 5_000, &[CERT])
            .expect("request sau lỗi cap vẫn chạy");
        assert_eq!(kq.body, b"hello");
    }

    /// `/proc/self/fd` là **process-wide**, nên phép đếm phải chạy trong tiến trình con
    /// riêng; chạy song song với 24 test khác sẽ đếm cả fd của chúng (không phải rò của spike).
    fn chay_chuong_trinh(
        ten: &str,
        src: &str,
        args: &[String],
        vm: bool,
    ) -> (i32, String, String, Duration) {
        let d = std::env::temp_dir().join(format!("vs3-w2-h2-{}", std::process::id()));
        fs::create_dir_all(&d).expect("thư mục tạm");
        let p = d.join(format!("{ten}.vis"));
        fs::write(&p, src).expect("ghi chương trình");
        let goc = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/vs3_tls/localhost.cert.der");
        let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
        c.arg("run");
        if vm {
            c.arg("--vm").arg("--no-cache");
        }
        c.env("VIS_TLS_TEST_ROOT_DER", goc);
        c.arg(&p).arg("--");
        for a in args {
            c.arg(a);
        }
        let bat = Instant::now();
        let o = c.output().expect("chạy vis run");
        (
            o.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&o.stdout).to_string(),
            String::from_utf8_lossy(&o.stderr).to_string(),
            bat.elapsed(),
        )
    }

    const BON_TAC_VU: &str = r#"use std.http

async fn lay(url: str) -> str:
    match http.get_timeout(url, 3000):
        Ok(s): s
        Err(e): "loi: {e}"

fn main(args: list[str]):
    let am = await spawn lay(args[0])
    let a = spawn lay(args[1])
    let b = spawn lay(args[2])
    let c = spawn lay(args[3])
    let d = spawn lay(args[4])
    print(am)
    print(await a)
    print(await b)
    print(await c)
    print(await d)
"#;

    /// **Production** (không gọi backend trực tiếp): 4 task async cùng authority, server chỉ
    /// nhận đủ 4 stream trên một connection rồi mới trả ⇒ chứng minh multiplex qua `http.get`.
    #[test]
    fn async_h2_production_bon_stream_mot_connection() {
        let s = MayChuSpike::moi(Kieu::Warm4);
        let mut tong_dong = 0usize;
        for vm in [false, true] {
            let mut args = vec![s.url("/warm")];
            args.extend((0..4).map(|i| s.url(&format!("/p{i}"))));
            let (code, out, err, bat) = chay_chuong_trinh("bon-tac-vu", BON_TAC_VU, &args, vm);
            assert_eq!(code, 0, "4 task vm={vm}: {err}");
            assert!(bat < Duration::from_secs(10), "không treo vm={vm}");
            let mut dong: Vec<&str> = out.lines().collect();
            dong.sort_unstable();
            assert_eq!(
                dong,
                vec!["/p0", "/p1", "/p2", "/p3", "/warm"],
                "vm={vm}: {out:?}"
            );
            tong_dong += dong.len();
        }
        assert_eq!(s.conns(), 2, "mỗi engine một connection (ấm + 4 task)");
        assert_eq!(s.streams(), 10, "mỗi connection: 1 warm + 4 stream");
        assert_eq!(tong_dong, 10);
    }

    #[test]
    fn async_h2_production_timeout_va_goaway() {
        let treo = MayChuSpike::moi(Kieu::Treo);
        let src = r#"use std.http

async fn lay(url: str) -> str:
    match http.get_timeout(url, 400):
        Ok(s): s
        Err(e): "loi: {e}"

fn main(args: list[str]):
    print(await spawn lay(args[0]))
"#;
        let (code, out, err, bat) = chay_chuong_trinh("timeout", src, &[treo.url("/x")], false);
        assert_eq!(code, 0, "timeout: {err}");
        assert!(out.contains("thời gian"), "phải timeout rõ: {out:?}");
        assert!(bat < Duration::from_secs(3), "timeout không treo: {bat:?}");

        let goaway = MayChuSpike::moi(Kieu::GoAway);
        let (code, out, err, bat) = chay_chuong_trinh("goaway", src, &[goaway.url("/x")], false);
        assert_eq!(code, 0, "goaway: {err}");
        assert!(
            out.contains("GOAWAY") || out.contains("HTTP/2"),
            "GOAWAY rõ: {out:?}"
        );
        assert!(bat < Duration::from_secs(3), "GOAWAY không treo: {bat:?}");
    }

    #[test]
    fn async_h2_production_cap_body() {
        let s = MayChuSpike::moi(Kieu::Thuong);
        let src = r#"use std.http

async fn lay(url: str) -> str:
    match http.get_timeout(url, 20000):
        Ok(s): s
        Err(e): "loi: {e}"

fn main(args: list[str]):
    print(await spawn lay(args[0]))
"#;
        let (code, out, err, bat) = chay_chuong_trinh("cap", src, &[s.url("/big")], false);
        assert_eq!(code, 0, "cap: {err}");
        assert!(
            out.contains("8 MiB") || out.contains("vượt giới hạn") || out.contains("HTTP/2"),
            "cap body rõ: out={out:?} err={err:?}"
        );
        assert!(bat < Duration::from_secs(30), "cap không treo");
    }

    #[test]
    fn async_h2_production_redirect() {
        let s = MayChuSpike::moi(Kieu::Thuong);
        let src = r#"use std.http

async fn lay(url: str) -> str:
    match http.get_timeout(url, 3000):
        Ok(s): s
        Err(e): "loi: {e}"

fn main(args: list[str]):
    print(await spawn lay(args[0]))
"#;
        let (code, out, err, bat) = chay_chuong_trinh("redir", src, &[s.url("/redir")], false);
        assert_eq!(code, 0, "redirect: {err}");
        assert_eq!(out, "hello\n", "h2 phải theo redirect: {out:?}");
        assert!(bat < Duration::from_secs(3), "redirect không treo");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn async_h2_timeout_lap_khong_ro_fd() {
        let exe = std::env::current_exe().expect("current_exe");
        let out = std::process::Command::new(exe)
            .args([
                "--exact",
                "h2_async_spike::async_h2_timeout_lap_khong_ro_fd_con",
                "--ignored",
                "--nocapture",
            ])
            .output()
            .expect("chạy tiến trình con");
        assert!(
            out.status.success(),
            "tiến trình con FAIL:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "chạy trong tiến trình con bởi async_h2_timeout_lap_khong_ro_fd"]
    fn async_h2_timeout_lap_khong_ro_fd_con() {
        let s = MayChuSpike::moi(Kieu::Treo);
        let truoc = dem_fd();
        for _ in 0..20 {
            let e = h2_get_async_voi_goc(&s.url("/x"), 150, &[CERT])
                .map(|_| ())
                .expect_err("timeout");
            assert!(e.contains("thời gian"), "lỗi phải rõ: {e}");
        }
        let sau = dem_fd();
        eprintln!("fd sau 20 timeout: {truoc} -> {sau}");
        assert!(sau <= truoc + 4, "rò fd sau 20 timeout: {truoc} -> {sau}");
    }
}
