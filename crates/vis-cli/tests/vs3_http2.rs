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
        let end = body.is_empty();
        let mut send = respond
            .send_response(rb.body(()).map_err(io::Error::other)?, end)
            .map_err(|e| io::Error::other(format!("h2 response: {e}")))?;
        if !body.is_empty() {
            send.send_data(Bytes::from(body), true)
                .map_err(|e| io::Error::other(format!("h2 data: {e}")))?;
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
