// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-5a (VD-271)** — registry **HTTP** cho `vis pkg`: tải `index.json`, chọn phiên bản bằng
//! resolver semver hiện có, tải tarball, kiểm **SHA-256** trước khi cài, rồi ghi lock/store như cũ.
//!
//! Hermetic: server loopback thật trong test (`std::net`), fixture dựng tại chỗ; **không** Internet.
//! Ca phủ: add/install qua HTTP, hai lần add cùng byte, hash tarball sai, hash tệp sai, index thiếu
//! gói, index 404/hỏng, `https://` được nhận (lỗi nối TLS rõ), và `update` chọn bản mới qua HTTP.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn tmp_dir(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering as O};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let n = DEM.fetch_add(1, O::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vs3-reg-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    d
}

fn vis(cwd: &Path, args: &[&str]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("chạy `vis pkg`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

fn doc(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_else(|e| panic!("đọc {}: {e}", p.display()))
}

fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

/// Chuẩn hoá **như ingest của `vis pkg`**: đúng một `\n` cuối, không CRLF/BOM trong fixture.
fn chuan(s: &str) -> String {
    let t = s.trim_end_matches('\n');
    if t.is_empty() {
        String::new()
    } else {
        format!("{t}\n")
    }
}

fn file_sha(s: &str) -> String {
    hex(&vis_bytecode::bytes_hash(chuan(s).as_bytes()))
}

fn json_esc(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c => o.push(c),
        }
    }
    o
}

// ----- server loopback (routing tối giản, tuần tự) -----

struct Server {
    addr: String,
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
    log: Arc<Mutex<Vec<String>>>,
}

impl Server {
    fn moi(routes: BTreeMap<String, Vec<u8>>) -> Self {
        let l = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        l.set_nonblocking(true).expect("non-blocking");
        let addr = l.local_addr().expect("local_addr").to_string();
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = Arc::clone(&stop);
        let log = Arc::new(Mutex::new(Vec::new()));
        let log2 = Arc::clone(&log);
        let handle = std::thread::spawn(move || {
            while !stop2.load(Ordering::Relaxed) {
                match l.accept() {
                    Ok((mut s, _)) => {
                        // Windows: socket do `accept` trả về có thể thừa hưởng non-blocking của
                        // listener ⇒ `read` trả `WouldBlock` ngay khi request chưa tới, server
                        // trả 404 oan (hoặc đóng sớm ⇒ client thấy 10053). Đặt lại blocking.
                        let _ = s.set_nonblocking(false);
                        let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
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
                        // Windows: `shutdown(Both)` ngay sau write có thể làm peer thấy
                        // WSAECONNABORTED (10053) khi còn byte trên đường. Đóng chiều ghi, rồi
                        // đọc nốt tới EOF/timeout ngắn trước khi drop để close êm.
                        let _ = s.shutdown(Shutdown::Write);
                        let _ = s.set_read_timeout(Some(Duration::from_millis(200)));
                        let mut rac = [0u8; 256];
                        while matches!(s.read(&mut rac), Ok(n) if n > 0) {}
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
            handle: Some(handle),
            log,
        }
    }

    fn url(&self) -> String {
        format!("http://{}", self.addr)
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

// ----- fixture: manifest + tar + index -----

struct Goi<'a> {
    ten: &'a str,
    version: &'a str,
    entry: &'a str,
    files: &'a [(&'a str, &'a str)],
    deps: &'a [(&'a str, &'a str)],
}

#[derive(Default)]
struct Hong {
    /// `tarball_sha256` trong index sai (hash tầng tarball).
    tar_sha_sai: bool,
    /// `files[].sha256` trong manifest sai (hash từng tệp).
    file_sha_sai: bool,
    /// Tarball có thêm tệp **không** khai trong manifest.
    them_tep_la: bool,
    /// Index thiếu hẳn `tarball_sha256` của phiên bản.
    thieu_tar_sha: bool,
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

/// Tar ustar tối giản (đủ cho decoder thật ở production): header 512 byte + data + padding.
fn tar(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut out = Vec::new();
    for (path, noi_dung) in entries {
        let data = noi_dung.as_bytes();
        let mut h = [0u8; 512];
        let p = path.as_bytes();
        assert!(p.len() <= 100, "fixture: đường dẫn tar phải ≤ 100 byte");
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

/// Dựng `index.json` + route tarball từ danh sách gói.
fn dung_index(gois: &[Goi], hong: &Hong) -> (String, BTreeMap<String, Vec<u8>>) {
    let mut routes = BTreeMap::new();
    let mut theo_ten: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for g in gois {
        let mut files_json = Vec::new();
        let mut tar_entries: Vec<(&str, &str)> = Vec::new();
        for (path, noi_dung) in g.files {
            let sha = if hong.file_sha_sai {
                "0".repeat(64)
            } else {
                file_sha(noi_dung)
            };
            files_json.push(format!(
                "{{\"path\":\"{}\",\"sha256\":\"{}\"}}",
                json_esc(path),
                sha
            ));
            tar_entries.push((path, noi_dung));
        }
        if hong.them_tep_la {
            tar_entries.push(("la.txt", "la\n"));
        }
        let deps_json: Vec<String> = g
            .deps
            .iter()
            .map(|(k, v)| format!("\"{}\":\"{}\"", json_esc(k), json_esc(v)))
            .collect();
        let manifest = format!(
            "{{\"name\":\"{}\",\"version\":\"{}\",\"entry\":\"{}\",\"deps\":{{{}}},\"files\":[{}]}}",
            json_esc(g.ten),
            json_esc(g.version),
            json_esc(g.entry),
            deps_json.join(","),
            files_json.join(",")
        );
        let tar_bytes = tar(&tar_entries);
        let tar_sha = if hong.tar_sha_sai {
            "0".repeat(64)
        } else {
            hex(&vis_bytecode::bytes_hash(&tar_bytes))
        };
        let tar_path = format!("{}/{}.tar.b64", g.ten, g.version);
        routes.insert(format!("/{tar_path}"), b64(&tar_bytes).into_bytes());
        let sha_truong = if hong.thieu_tar_sha {
            String::new()
        } else {
            format!("\"tarball_sha256\":\"{tar_sha}\",")
        };
        theo_ten.entry(g.ten).or_default().push(format!(
            "\"{}\":{{\"tarball\":\"{}\",{} \"manifest\":{}}}",
            json_esc(g.version),
            json_esc(&tar_path),
            sha_truong,
            manifest
        ));
    }
    let packages: Vec<String> = theo_ten
        .into_iter()
        .map(|(ten, ds)| format!("\"{}\":{{{}}}", json_esc(ten), ds.join(",")))
        .collect();
    let idx = format!(
        "{{\"format\":\"vispkg-http1\",\"packages\":{{{}}}}}\n",
        packages.join(",")
    );
    (idx, routes)
}

fn fixture_mot(hong: &Hong) -> (String, BTreeMap<String, Vec<u8>>) {
    let goi = Goi {
        ten: "util",
        version: "0.1.0",
        entry: "src/util.vis",
        files: &[("src/util.vis", "fn util() -> i64:\n    7\n")],
        deps: &[],
    };
    dung_index(&[goi], hong)
}

/// Thêm gói `bad` có manifest hỏng (`files` rỗng) vào index — **không** được chọn.
fn them_goi_manifest_hong(idx: &str) -> String {
    let mut s = idx
        .trim_end()
        .strip_suffix("}}")
        .expect("index kết thúc bằng `}}`")
        .to_string();
    s.push_str(",\"bad\":{\"1.0.0\":{\"tarball\":\"bad/1.0.0.tar.b64\",\"tarball_sha256\":\"");
    s.push_str(&"0".repeat(64));
    s.push_str(
        "\",\"manifest\":{\"name\":\"bad\",\"version\":\"1.0.0\",\"entry\":\"x.vis\",\"files\":[]}}}}}",
    );
    s
}

fn server_index(hong: &Hong) -> Server {
    let (idx, mut routes) = fixture_mot(hong);
    routes.insert("/index.json".to_string(), idx.into_bytes());
    Server::moi(routes)
}

// ----- ca -----

/// Ca 1 — `add` qua HTTP: index + tarball, cài đúng nội dung; `add` lần hai **cùng byte** lock;
/// tắt server rồi `install` vẫn dựng lại cây từ lock/store (không cần registry).
#[test]
fn add_qua_http_roi_install_khong_can_registry() {
    let d = tmp_dir("add");
    let srv = server_index(&Hong::default());
    let (code, out, err) = vis(&d, &["pkg", "add", "util", "--registry", &srv.url()]);
    assert_eq!(code, 0, "stdout: {out}\nstderr: {err}");
    let lock1 = doc(&d.join("vis.lock"));
    assert!(lock1.contains("\"util\""), "lock: {lock1}");
    assert!(lock1.contains("0.1.0"), "lock: {lock1}");
    let tep = d.join(".vis/pkg/util/0.1.0/src/util.vis");
    assert_eq!(doc(&tep), "fn util() -> i64:\n    7\n");

    // Lần hai, cùng server: nội dung/lock không đổi (tất định).
    let (code, _, err) = vis(&d, &["pkg", "add", "util", "--registry", &srv.url()]);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(
        doc(&d.join("vis.lock")),
        lock1,
        "add lần hai phải cùng byte"
    );

    // Log phải có cả index lẫn tarball.
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

    // Tắt mạng: `install` chỉ đọc `vis.lock` + `.vis/store`.
    drop(srv);
    std::fs::remove_dir_all(d.join(".vis/pkg")).expect("xoá cây cài");
    let (code, _, err) = vis(&d, &["pkg", "install"]);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(doc(&tep), "fn util() -> i64:\n    7\n");
    assert_eq!(doc(&d.join("vis.lock")), lock1, "install không đổi lock");
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 2 — hash **tarball** sai ⇒ `VS0707`, exit 2, **không** tạo `.vis`/`vis.lock` (kiểm trước khi cài).
#[test]
fn tu_choi_hash_tarball_sai() {
    let d = tmp_dir("tar-hash");
    let srv = server_index(&Hong {
        tar_sha_sai: true,
        ..Default::default()
    });
    let (code, _out, err) = vis(&d, &["pkg", "add", "util", "--registry", &srv.url()]);
    assert_eq!(code, 2, "stderr: {err}");
    assert!(err.contains("VS0707"), "phải nêu mã hash: {err}");
    assert!(!d.join(".vis").exists(), "KHÔNG được tạo .vis");
    assert!(!d.join("vis.lock").exists(), "KHÔNG được ghi vis.lock");
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 3 — hash **từng tệp** trong manifest sai ⇒ `VS0707`, không cài.
#[test]
fn tu_choi_hash_tep_sai() {
    let d = tmp_dir("file-hash");
    let srv = server_index(&Hong {
        file_sha_sai: true,
        ..Default::default()
    });
    let (code, _out, err) = vis(&d, &["pkg", "add", "util", "--registry", &srv.url()]);
    assert_eq!(code, 2, "stderr: {err}");
    assert!(err.contains("VS0707"), "phải nêu mã hash: {err}");
    assert!(!d.join(".vis").exists(), "KHÔNG được tạo .vis");
    assert!(!d.join("vis.lock").exists(), "KHÔNG được ghi vis.lock");
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 4 — tarball có tệp **lạ** (không khai trong manifest) ⇒ từ chối, không cài.
#[test]
fn tu_choi_tep_la_trong_tarball() {
    let d = tmp_dir("tep-la");
    let srv = server_index(&Hong {
        them_tep_la: true,
        ..Default::default()
    });
    let (code, _out, err) = vis(&d, &["pkg", "add", "util", "--registry", &srv.url()]);
    assert_eq!(code, 2, "stderr: {err}");
    assert!(err.contains("la.txt"), "phải nêu tệp lạ: {err}");
    assert!(!d.join(".vis").exists(), "KHÔNG được tạo .vis");
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 4b — manifest hỏng của gói **không được chọn** không chặn gói hợp lệ (giống resolver cục bộ
/// bỏ qua bản manifest hỏng).
#[test]
fn manifest_hong_khong_chan_goi_khac() {
    let d = tmp_dir("manifest-hong");
    let (idx, mut routes) = fixture_mot(&Hong::default());
    routes.insert(
        "/index.json".to_string(),
        them_goi_manifest_hong(&idx).into_bytes(),
    );
    let srv = Server::moi(routes);
    let (code, out, err) = vis(&d, &["pkg", "add", "util", "--registry", &srv.url()]);
    assert_eq!(code, 0, "stdout: {out}\nstderr: {err}");
    assert!(d.join("vis.lock").is_file());
    assert!(!d.join(".vis/pkg/bad").exists(), "KHÔNG được cài gói hỏng");
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 5b — index thiếu `tarball_sha256` của **phiên bản được chọn** ⇒ lỗi rõ, không ghi gì.
#[test]
fn index_thieu_tarball_sha_bi_tu_choi() {
    let d = tmp_dir("thieu-tar-sha");
    let srv = server_index(&Hong {
        thieu_tar_sha: true,
        ..Default::default()
    });
    let (code, _out, err) = vis(&d, &["pkg", "add", "util", "--registry", &srv.url()]);
    assert_eq!(code, 2, "stderr: {err}");
    assert!(
        err.contains("tarball_sha256"),
        "phải nêu trường thiếu: {err}"
    );
    assert!(!d.join(".vis").exists(), "KHÔNG được tạo .vis");
    assert!(!d.join("vis.lock").exists(), "KHÔNG được ghi vis.lock");
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 5 — index **thiếu gói** ⇒ resolver hiện có báo `VS0704` + chuỗi ràng buộc, không ghi gì.
#[test]
fn index_thieu_goi_bao_vs0704() {
    let d = tmp_dir("thieu-goi");
    let srv = server_index(&Hong::default());
    let (code, _out, err) = vis(&d, &["pkg", "add", "khong_co", "--registry", &srv.url()]);
    assert_eq!(code, 2, "stderr: {err}");
    assert!(err.contains("VS0704"), "phải nêu mã resolver: {err}");
    assert!(err.contains("khong_co"), "phải nêu tên gói: {err}");
    assert!(!d.join(".vis").exists(), "KHÔNG được tạo .vis");
    assert!(!d.join("vis.lock").exists(), "KHÔNG được ghi vis.lock");
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 6 — index **404** ⇒ lỗi mạng rõ URL/status, exit 2, không ghi gì.
#[test]
fn index_404_loi_ro() {
    let d = tmp_dir("index-404");
    let srv = Server::moi(BTreeMap::new());
    let (code, _out, err) = vis(&d, &["pkg", "add", "util", "--registry", &srv.url()]);
    assert_eq!(code, 2, "stderr: {err}");
    assert!(err.contains("index"), "phải nêu index: {err}");
    assert!(err.contains("404"), "phải nêu HTTP status: {err}");
    assert!(!d.join(".vis").exists(), "KHÔNG được tạo .vis");
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 7 — `index.json` hỏng ⇒ lỗi rõ, exit 2, không ghi gì.
#[test]
fn index_hong_loi_ro() {
    let d = tmp_dir("index-hong");
    let mut routes = BTreeMap::new();
    routes.insert("/index.json".to_string(), b"{khong-phai-json".to_vec());
    let srv = Server::moi(routes);
    let (code, _out, err) = vis(&d, &["pkg", "add", "util", "--registry", &srv.url()]);
    assert_eq!(code, 2, "stderr: {err}");
    assert!(err.contains("index"), "phải nêu index: {err}");
    assert!(err.contains("hỏng"), "phải nêu index hỏng: {err}");
    assert!(!d.join(".vis").exists(), "KHÔNG được tạo .vis");
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 8 — `https://` **được nhận**; host đóng ⇒ lỗi nối TLS rõ (không còn "chưa hỗ trợ").
#[test]
fn https_duoc_nhan_loi_noi_tls_ro() {
    let d = tmp_dir("https");
    let (code, _out, err) = vis(
        &d,
        &["pkg", "add", "util", "--registry", "https://127.0.0.1:1"],
    );
    assert_eq!(code, 2, "stderr: {err}");
    assert!(err.contains("TLS"), "phải nêu TLS: {err}");
    assert!(!err.contains("chưa hỗ trợ"), "https phải được nhận: {err}");
    assert!(!d.join(".vis").exists(), "KHÔNG được tạo .vis");
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 9 — `update` qua HTTP: lock ghim `^1.0.0`, registry mới có `1.1.0` ⇒ cập nhật nội dung + lock.
#[test]
fn update_qua_http_chon_ban_moi() {
    let d = tmp_dir("update");
    let v1 = Goi {
        ten: "util",
        version: "1.0.0",
        entry: "src/util.vis",
        files: &[("src/util.vis", "fn util() -> i64:\n    1\n")],
        deps: &[],
    };
    let (idx1, mut routes1) = dung_index(&[v1], &Hong::default());
    routes1.insert("/index.json".to_string(), idx1.into_bytes());
    let srv1 = Server::moi(routes1);
    let (code, _, err) = vis(&d, &["pkg", "add", "util", "--registry", &srv1.url()]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(doc(&d.join("vis.lock")).contains("1.0.0"));

    // Registry mới: giữ 1.0.0, thêm 1.1.0 (nội dung khác). Root `^1.0.0` ⇒ chọn 1.1.0.
    let v1b = Goi {
        ten: "util",
        version: "1.0.0",
        entry: "src/util.vis",
        files: &[("src/util.vis", "fn util() -> i64:\n    1\n")],
        deps: &[],
    };
    let v2 = Goi {
        ten: "util",
        version: "1.1.0",
        entry: "src/util.vis",
        files: &[("src/util.vis", "fn util() -> i64:\n    2\n")],
        deps: &[],
    };
    let (idx2, mut routes2) = dung_index(&[v1b, v2], &Hong::default());
    routes2.insert("/index.json".to_string(), idx2.into_bytes());
    let srv2 = Server::moi(routes2);
    let (code, out, err) = vis(&d, &["pkg", "update", "util", "--registry", &srv2.url()]);
    assert_eq!(code, 0, "stdout: {out}\nstderr: {err}");
    assert!(out.contains("đã cập nhật util 1.1.0"), "stdout: {out}");
    assert!(doc(&d.join("vis.lock")).contains("1.1.0"));
    assert_eq!(
        doc(&d.join(".vis/pkg/util/1.1.0/src/util.vis")),
        "fn util() -> i64:\n    2\n"
    );
    let _ = std::fs::remove_dir_all(&d);
}
