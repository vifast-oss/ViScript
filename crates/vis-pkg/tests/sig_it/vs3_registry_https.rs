// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-W2 (VD-271)** — `vis pkg` qua **registry HTTPS** hermetic: server rustls local + fixture
//! cert, index + tarball, SHA-256; sai cert ⇒ lỗi TLS rõ, không ghi `.vis`.
//!
//! Bản debug có hook `VIS_TLS_TEST_ROOT_DER` (một chứng chỉ DER) để tiến trình `vis` tin cert fixture;
//! bản release không có hook, nên test thành công được bỏ qua ở `--release` (xem `tls::mac_dinh`).

#![cfg(all(feature = "tls", not(target_arch = "wasm32")))]

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ServerConfig, ServerConnection, StreamOwned};

const CERT_LOCALHOST: &[u8] = include_bytes!("../fixtures/vs3_tls/localhost.cert.der");
const KEY_LOCALHOST: &[u8] = include_bytes!("../fixtures/vs3_tls/localhost.key.der");
const CERT_SAI_TEN: &[u8] = include_bytes!("../fixtures/vs3_tls/sai-ten.cert.der");
const KEY_SAI_TEN: &[u8] = include_bytes!("../fixtures/vs3_tls/sai-ten.key.der");

fn tmp_dir(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering as O};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let n = DEM.fetch_add(1, O::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-reg-https-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    d
}

fn goc_kiem_thu() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vs3_tls/localhost.cert.der")
}

fn vis(cwd: &Path, args: &[&str], root: Option<&Path>) -> (i32, String, String) {
    let mut c = Command::new(crate::common::vis_bin());
    c.current_dir(cwd)
        .args(args)
        .env_remove("VIS_REGISTRY")
        .env_remove("VIS_REGISTRY_KEY")
        .env_remove("VIS_TLS_TEST_ROOT_DER");
    if let Some(r) = root {
        c.env("VIS_TLS_TEST_ROOT_DER", r);
    }
    let o = c.output().expect("chạy `vis pkg`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

fn cau_hinh_server(cert: &[u8], key: &[u8]) -> Arc<ServerConfig> {
    let nha = Arc::new(rustls::crypto::ring::default_provider());
    let cert = CertificateDer::from(cert.to_vec());
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.to_vec()));
    Arc::new(
        ServerConfig::builder_with_provider(nha)
            .with_safe_default_protocol_versions()
            .expect("phiên bản TLS")
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)
            .expect("chứng chỉ server"),
    )
}

struct Server {
    addr: String,
    stop: Arc<AtomicBool>,
    log: Arc<Mutex<Vec<String>>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Server {
    fn moi(cert: &[u8], key: &[u8], routes: BTreeMap<String, Vec<u8>>) -> Self {
        let l = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        l.set_nonblocking(true).expect("non-blocking");
        let addr = l.local_addr().expect("local_addr").to_string();
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = Arc::clone(&stop);
        let log = Arc::new(Mutex::new(Vec::new()));
        let log2 = Arc::clone(&log);
        let cfg = cau_hinh_server(cert, key);
        let handle = std::thread::spawn(move || {
            while !stop2.load(Ordering::Relaxed) {
                match l.accept() {
                    Ok((tcp, _)) => {
                        // Windows: socket do `accept` trả về có thể thừa hưởng non-blocking của
                        // listener ⇒ TLS handshake `read` trả `WouldBlock` ngay ⇒ 10053. Đặt lại
                        // blocking trước khi bọc `StreamOwned`.
                        let _ = tcp.set_nonblocking(false);
                        let _ = tcp.set_read_timeout(Some(Duration::from_secs(5)));
                        let Ok(conn) = ServerConnection::new(Arc::clone(&cfg)) else {
                            continue;
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
                        let text = String::from_utf8_lossy(&req).to_string();
                        let dong = text.lines().next().unwrap_or("").to_string();
                        let path = dong.split_whitespace().nth(1).unwrap_or("/").to_string();
                        log2.lock().expect("log").push(dong);
                        let resp = match routes.get(&path) {
                            Some(b) => {
                                let mut r = format!(
                                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                    b.len()
                                )
                                .into_bytes();
                                r.extend_from_slice(b);
                                r
                            }
                            None => b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                                .to_vec(),
                        };
                        let _ = s.write_all(&resp);
                        s.conn.send_close_notify();
                        let _ = s.flush();
                        let _ = s.sock.shutdown(Shutdown::Both);
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
            log,
            handle: Some(handle),
        }
    }

    fn url(&self) -> String {
        format!("https://{}", self.addr)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
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

/// Tar ustar tối giản đúng định dạng production đọc.
fn tar(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut out = Vec::new();
    for (path, noi_dung) in entries {
        let data = noi_dung.as_bytes();
        let mut h = [0u8; 512];
        let p = path.as_bytes();
        assert!(p.len() <= 100);
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

/// Index + tarball của một gói `util@0.1.0`.
fn fixture() -> BTreeMap<String, Vec<u8>> {
    let (routes, _) = fixture_va_khoa();
    routes
}

/// Fixture + khoá/signature Ed25519 (chỉ dùng khi `pkg-sig` bật). Trả thêm public key hex.
#[cfg(feature = "pkg-sig")]
fn fixture_va_khoa() -> (BTreeMap<String, Vec<u8>>, String) {
    use ring::rand::SystemRandom;
    use ring::signature::{Ed25519KeyPair, KeyPair};

    let noi_dung = "fn util() -> i64:\n    7\n";
    let file_sha = hex(&vis_bytecode::bytes_hash(noi_dung.as_bytes()));
    let tar_bytes = tar(&[("src/util.vis", noi_dung)]);
    let tar_sha = hex(&vis_bytecode::bytes_hash(&tar_bytes));
    let msg = vis_pkg::pkg_sig::thong_diep_ky(
        "util",
        "0.1.0",
        &tar_sha,
        "src/util.vis",
        &[],
        &[("src/util.vis".to_string(), file_sha.clone())],
    );
    let rng = SystemRandom::new();
    let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).expect("sinh khoá");
    let kp = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).expect("đọc khoá");
    let pub_hex = hex(kp.public_key().as_ref());
    let sig_hex = hex(kp.sign(&msg).as_ref());
    let manifest = format!(
        "{{\"name\":\"util\",\"version\":\"0.1.0\",\"entry\":\"src/util.vis\",\"deps\":{{}},\"files\":[{{\"path\":\"src/util.vis\",\"sha256\":\"{file_sha}\"}}]}}"
    );
    let index = format!(
        "{{\"format\":\"vispkg-http1\",\"packages\":{{\"util\":{{\"0.1.0\":{{\"tarball\":\"util/0.1.0.tar.b64\",\"tarball_sha256\":\"{tar_sha}\",\"signature_ed25519\":\"{sig_hex}\",\"manifest\":{manifest}}}}}}}}}\n"
    );
    let mut routes = BTreeMap::new();
    routes.insert("/index.json".to_string(), index.into_bytes());
    routes.insert(
        "/util/0.1.0.tar.b64".to_string(),
        b64(&tar_bytes).into_bytes(),
    );
    (routes, pub_hex)
}

#[cfg(not(feature = "pkg-sig"))]
fn fixture_va_khoa() -> (BTreeMap<String, Vec<u8>>, String) {
    (fixture_sach(), String::new())
}

#[cfg(not(feature = "pkg-sig"))]
fn fixture_sach() -> BTreeMap<String, Vec<u8>> {
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
    let mut routes = BTreeMap::new();
    routes.insert("/index.json".to_string(), index.into_bytes());
    routes.insert(
        "/util/0.1.0.tar.b64".to_string(),
        b64(&tar_bytes).into_bytes(),
    );
    routes
}

/// **VS3-W2**: registry HTTPS thật — index + tarball qua TLS, SHA-256, cài đúng nội dung.
#[test]
fn add_qua_https_roi_cai_dung() {
    if !cfg!(debug_assertions) {
        eprintln!("bỏ qua add_qua_https_roi_cai_dung: hook root chỉ có ở bản debug");
        return;
    }
    let d = tmp_dir("add");
    let srv = Server::moi(CERT_LOCALHOST, KEY_LOCALHOST, fixture());
    let (code, out, err) = vis(
        &d,
        &["pkg", "add", "util", "--registry", &srv.url()],
        Some(&goc_kiem_thu()),
    );
    assert_eq!(code, 0, "stdout: {out}\nstderr: {err}");
    assert!(d.join("vis.lock").is_file());
    assert_eq!(
        std::fs::read_to_string(d.join(".vis/pkg/util/0.1.0/src/util.vis")).expect("đọc tệp"),
        "fn util() -> i64:\n    7\n"
    );
    let log = srv.log.lock().expect("log").clone();
    assert!(
        log.iter().any(|l| l.starts_with("GET /index.json ")),
        "log: {log:?}"
    );
    assert!(
        log.iter()
            .any(|l| l.starts_with("GET /util/0.1.0.tar.b64 ")),
        "log: {log:?}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// **VS3-W2**: HTTPS + `--registry-key` — chữ ký Ed25519 hợp lệ vẫn được kiểm trước tarball.
#[cfg(feature = "pkg-sig")]
#[test]
fn add_https_co_chu_ky() {
    if !cfg!(debug_assertions) {
        eprintln!("bỏ qua add_https_co_chu_ky: hook root chỉ có ở bản debug");
        return;
    }
    let d = tmp_dir("chu-ky");
    let (routes, pub_hex) = fixture_va_khoa();
    let srv = Server::moi(CERT_LOCALHOST, KEY_LOCALHOST, routes);
    let (code, out, err) = vis(
        &d,
        &[
            "pkg",
            "add",
            "util",
            "--registry",
            &srv.url(),
            "--registry-key",
            &pub_hex,
        ],
        Some(&goc_kiem_thu()),
    );
    assert_eq!(code, 0, "stdout: {out}\nstderr: {err}");
    assert!(d.join(".vis/pkg/util/0.1.0/src/util.vis").is_file());
    let _ = std::fs::remove_dir_all(&d);
}

/// **VS3-W2**: cert server không nằm trong root test ⇒ lỗi TLS rõ, không ghi `.vis`.
#[test]
fn https_sai_cert_loi_ro() {
    let d = tmp_dir("sai-cert");
    let srv = Server::moi(CERT_SAI_TEN, KEY_SAI_TEN, fixture());
    let (code, _out, err) = vis(
        &d,
        &["pkg", "add", "util", "--registry", &srv.url()],
        Some(&goc_kiem_thu()),
    );
    assert_eq!(code, 2, "stderr: {err}");
    assert!(err.contains("TLS"), "phải nêu TLS: {err}");
    assert!(
        err.contains("UnknownIssuer") || err.contains("certificate"),
        "phải nêu lý do cert: {err}"
    );
    assert!(!d.join(".vis").exists(), "KHÔNG được tạo .vis");
    let _ = std::fs::remove_dir_all(&d);
}

/// **VS3-W2**: không có root test ⇒ cert tự ký bị từ chối (đường mặc định `webpki-roots`).
#[test]
fn https_khong_root_test_loi_ro() {
    let d = tmp_dir("khong-root");
    let srv = Server::moi(CERT_LOCALHOST, KEY_LOCALHOST, fixture());
    let (code, _out, err) = vis(&d, &["pkg", "add", "util", "--registry", &srv.url()], None);
    assert_eq!(code, 2, "stderr: {err}");
    assert!(err.contains("TLS"), "phải nêu TLS: {err}");
    assert!(
        err.contains("UnknownIssuer") || err.contains("certificate"),
        "phải nêu lý do cert: {err}"
    );
    assert!(!d.join(".vis").exists(), "KHÔNG được tạo .vis");
    let _ = std::fs::remove_dir_all(&d);
}

/// **VS3-W2**: gốc test tin cert nhưng URL dùng IP sai SAN ⇒ vẫn từ chối tên, không nới kiểm.
#[test]
fn https_sai_san_van_tu_choi() {
    let d = tmp_dir("sai-san");
    let cert =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vs3_tls/sai-ten.cert.der");
    let srv = Server::moi(CERT_SAI_TEN, KEY_SAI_TEN, fixture());
    let (code, _out, err) = vis(
        &d,
        &["pkg", "add", "util", "--registry", &srv.url()],
        Some(&cert),
    );
    assert_eq!(code, 2, "stderr: {err}");
    assert!(err.contains("TLS"), "phải nêu TLS: {err}");
    assert!(
        err.contains("NotValidForName") || err.contains("certificate"),
        "phải nêu lý do tên: {err}"
    );
    assert!(!d.join(".vis").exists(), "KHÔNG được tạo .vis");
    let _ = std::fs::remove_dir_all(&d);
}

/// **VS3-W2**: `https://` được parser nhận — host đóng cho lỗi nối TLS, không phải "chưa hỗ trợ".
#[test]
fn https_duoc_nhan_loi_noi_ro() {
    let d = tmp_dir("dong");
    let (code, _out, err) = vis(
        &d,
        &["pkg", "add", "util", "--registry", "https://127.0.0.1:1"],
        None,
    );
    assert_eq!(code, 2, "stderr: {err}");
    assert!(err.contains("TLS"), "phải nêu TLS: {err}");
    assert!(!err.contains("chưa hỗ trợ"), "https phải được nhận: {err}");
    assert!(!d.join(".vis").exists(), "KHÔNG được tạo .vis");
    let _ = std::fs::remove_dir_all(&d);
}
