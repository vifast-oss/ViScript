// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-5b (VD-271)** — chữ ký **Ed25519** cho registry HTTP của `vis pkg`.
//!
//! Ca phủ: ký đúng ⇒ cài; chữ ký sai/tamper ⇒ từ chối; sửa manifest sau khi ký ⇒ từ chối (chữ ký
//! phủ metadata); thiếu chữ ký khi đã cấu hình khoá ⇒ từ chối; không cấu hình khoá ⇒ bỏ qua (tương
//! thích); khoá hex sai; khoá qua `VIS_REGISTRY_KEY`; `--registry-key` với registry cục bộ ⇒ lỗi.
//!
//! Ký bằng chính `ring` (feature `pkg-sig`) và dùng **đúng** hàm sinh thông điệp chuẩn
//! [`vis_cli::pkg_sig::thong_diep_ky`] — không lặp định dạng trong test.

#![cfg(all(feature = "pkg-sig", not(target_arch = "wasm32")))]

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use ring::rand::SystemRandom;
use ring::signature::{Ed25519KeyPair, KeyPair};

use vis_cli::pkg_sig::{hex_ma, thong_diep_ky};

const TEN: &str = "util";
const VER: &str = "0.1.0";
const ENTRY: &str = "src/util.vis";
const NOI_DUNG: &str = "fn util() -> i64:\n    7\n";

fn tmp_dir(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering as O};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let n = DEM.fetch_add(1, O::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vs3-sig-{tag}-{}-{n}", std::process::id()));
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

fn chuan(s: &str) -> String {
    let t = s.trim_end_matches('\n');
    if t.is_empty() {
        String::new()
    } else {
        format!("{t}\n")
    }
}

fn file_sha(s: &str) -> String {
    hex_ma(&vis_bytecode::bytes_hash(chuan(s).as_bytes()))
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

struct Server {
    addr: String,
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Server {
    fn moi(routes: BTreeMap<String, Vec<u8>>) -> Self {
        let l = TcpListener::bind("127.0.0.1:0").expect("bind");
        l.set_nonblocking(true).expect("non-blocking");
        let addr = l.local_addr().expect("addr").to_string();
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = Arc::clone(&stop);
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
                        let text = String::from_utf8_lossy(&req);
                        let path = text
                            .lines()
                            .next()
                            .and_then(|l| l.split_whitespace().nth(1))
                            .unwrap_or("/");
                        let resp = match routes.get(path) {
                            Some(b) => {
                                let mut r = format!(
                                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                    b.len()
                                )
                                .into_bytes();
                                r.extend_from_slice(b);
                                r
                            }
                            None => b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_vec(),
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

#[derive(Clone, Copy, PartialEq)]
enum LoaiKy {
    /// Chữ ký đúng trên manifest phục vụ.
    Dung,
    /// Ký một thông điệp khác (độ dài hợp lệ, xác minh sai).
    Sai,
    /// Sửa một ký tự hex của chữ ký đúng.
    Tamper,
    /// Độ dài hex sai.
    SaiDoDai,
}

struct TuyChon {
    loai: LoaiKy,
    bo_sig: bool,
    /// Ký trên manifest gốc nhưng **phục vụ** manifest có `files[].sha256` khác.
    sua_file_sha_sau_ky: bool,
}

impl Default for TuyChon {
    fn default() -> Self {
        Self {
            loai: LoaiKy::Dung,
            bo_sig: false,
            sua_file_sha_sau_ky: false,
        }
    }
}

/// Dựng `index.json` + route tarball; `kp` dùng để ký.
fn fixture(kp: &Ed25519KeyPair, tc: &TuyChon) -> BTreeMap<String, Vec<u8>> {
    let sha = file_sha(NOI_DUNG);
    let files = vec![(ENTRY.to_string(), sha.clone())];
    let deps: Vec<(String, String)> = Vec::new();
    let tar_bytes = tar(&[(ENTRY, NOI_DUNG)]);
    let tar_sha = hex_ma(&vis_bytecode::bytes_hash(&tar_bytes));
    let msg = thong_diep_ky(TEN, VER, &tar_sha, ENTRY, &deps, &files);
    let sig = if tc.bo_sig {
        None
    } else {
        Some(match tc.loai {
            LoaiKy::Dung => hex_ma(kp.sign(&msg).as_ref()),
            LoaiKy::Sai => {
                let mut khac = msg.clone();
                khac.extend_from_slice(b"\nsua");
                hex_ma(kp.sign(&khac).as_ref())
            }
            LoaiKy::Tamper => {
                let mut s = hex_ma(kp.sign(&msg).as_ref()).into_bytes();
                s[0] = if s[0] == b'a' { b'b' } else { b'a' };
                String::from_utf8(s).expect("hex")
            }
            LoaiKy::SaiDoDai => "abcd".to_string(),
        })
    };
    let sha_phuc_vu = if tc.sua_file_sha_sau_ky {
        "0".repeat(64)
    } else {
        sha.clone()
    };
    let manifest = format!(
        "{{\"name\":\"{TEN}\",\"version\":\"{VER}\",\"entry\":\"{ENTRY}\",\"files\":[{{\"path\":\"{ENTRY}\",\"sha256\":\"{sha_phuc_vu}\"}}]}}"
    );
    let sig_truong = match &sig {
        Some(s) => format!("\"signature_ed25519\":\"{s}\","),
        None => String::new(),
    };
    let idx = format!(
        "{{\"format\":\"vispkg-http1\",\"packages\":{{\"{TEN}\":{{\"{VER}\":{{\"tarball\":\"{TEN}/{VER}.tar.b64\",\"tarball_sha256\":\"{tar_sha}\",{sig_truong}\"manifest\":{manifest}}}}}}}}}\n"
    );
    let mut routes = BTreeMap::new();
    routes.insert("/index.json".to_string(), idx.into_bytes());
    routes.insert(
        format!("/{TEN}/{VER}.tar.b64"),
        b64(&tar_bytes).into_bytes(),
    );
    routes
}

struct Khoa {
    kp: Ed25519KeyPair,
    pub_hex: String,
}

fn khoa_moi() -> Khoa {
    let rng = SystemRandom::new();
    let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).expect("sinh khoá");
    let kp = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).expect("đọc khoá");
    let pub_hex = hex_ma(kp.public_key().as_ref());
    Khoa { kp, pub_hex }
}

fn server(kp: &Ed25519KeyPair, tc: &TuyChon) -> Server {
    Server::moi(fixture(kp, tc))
}

/// Ca 1 — chữ ký hợp lệ + khoá qua `--registry-key` ⇒ cài đúng.
#[test]
fn ky_dung_thi_cai() {
    let d = tmp_dir("dung");
    let k = khoa_moi();
    let srv = server(&k.kp, &TuyChon::default());
    let (code, out, err) = vis(
        &d,
        &[
            "pkg",
            "add",
            TEN,
            "--registry",
            &srv.url(),
            "--registry-key",
            &k.pub_hex,
        ],
    );
    assert_eq!(code, 0, "stdout: {out}\nstderr: {err}");
    assert!(d.join("vis.lock").is_file());
    assert_eq!(
        doc(&d.join(".vis/pkg").join(TEN).join(VER).join(ENTRY)),
        NOI_DUNG
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 2 — chữ ký sai ⇒ từ chối, không tạo `.vis`/lock.
#[test]
fn ky_sai_thi_tu_choi() {
    let d = tmp_dir("sai");
    let k = khoa_moi();
    let srv = server(
        &k.kp,
        &TuyChon {
            loai: LoaiKy::Sai,
            ..Default::default()
        },
    );
    let (code, _out, err) = vis(
        &d,
        &[
            "pkg",
            "add",
            TEN,
            "--registry",
            &srv.url(),
            "--registry-key",
            &k.pub_hex,
        ],
    );
    assert_eq!(code, 2, "stderr: {err}");
    assert!(err.contains("chữ ký"), "phải nêu chữ ký: {err}");
    assert!(!d.join(".vis").exists(), "KHÔNG được tạo .vis");
    assert!(!d.join("vis.lock").exists(), "KHÔNG được ghi lock");
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 3 — sửa `files[].sha256` trong manifest **sau khi ký** ⇒ chữ ký không còn hợp lệ (phủ metadata).
#[test]
fn sua_manifest_sau_ky_bi_tu_choi() {
    let d = tmp_dir("sua-manifest");
    let k = khoa_moi();
    let srv = server(
        &k.kp,
        &TuyChon {
            sua_file_sha_sau_ky: true,
            ..Default::default()
        },
    );
    let (code, _out, err) = vis(
        &d,
        &[
            "pkg",
            "add",
            TEN,
            "--registry",
            &srv.url(),
            "--registry-key",
            &k.pub_hex,
        ],
    );
    assert_eq!(code, 2, "stderr: {err}");
    assert!(
        err.contains("chữ ký"),
        "phải bắt ở chữ ký (trước hash tệp): {err}"
    );
    assert!(
        !err.contains("VS0707"),
        "không được rơi xuống kiểm hash: {err}"
    );
    assert!(!d.join(".vis").exists());
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 4 — đã cấu hình khoá mà index **thiếu** `signature_ed25519` ⇒ từ chối.
#[test]
fn thieu_chu_ky_khi_co_khoa() {
    let d = tmp_dir("thieu-sig");
    let k = khoa_moi();
    let srv = server(
        &k.kp,
        &TuyChon {
            bo_sig: true,
            ..Default::default()
        },
    );
    let (code, _out, err) = vis(
        &d,
        &[
            "pkg",
            "add",
            TEN,
            "--registry",
            &srv.url(),
            "--registry-key",
            &k.pub_hex,
        ],
    );
    assert_eq!(code, 2, "stderr: {err}");
    assert!(err.contains("signature_ed25519"), "{err}");
    assert!(err.contains("yêu cầu chữ ký"), "{err}");
    assert!(!d.join(".vis").exists());
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 5 — `signature_ed25519` sai định dạng hex ⇒ lỗi rõ.
#[test]
fn chu_ky_sai_dinh_dang() {
    let d = tmp_dir("sig-hex");
    let k = khoa_moi();
    let srv = server(
        &k.kp,
        &TuyChon {
            loai: LoaiKy::SaiDoDai,
            ..Default::default()
        },
    );
    let (code, _out, err) = vis(
        &d,
        &[
            "pkg",
            "add",
            TEN,
            "--registry",
            &srv.url(),
            "--registry-key",
            &k.pub_hex,
        ],
    );
    assert_eq!(code, 2, "stderr: {err}");
    assert!(err.contains("signature_ed25519"), "{err}");
    assert!(err.contains("hex"), "{err}");
    assert!(!d.join(".vis").exists());
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 6 — **không** cấu hình khoá ⇒ trường chữ ký bị bỏ qua (registry cũ vẫn cài được).
#[test]
fn khong_co_khoa_thi_bo_qua_chu_ky() {
    let d = tmp_dir("khong-khoa");
    let k = khoa_moi();
    let srv = server(
        &k.kp,
        &TuyChon {
            loai: LoaiKy::Tamper,
            ..Default::default()
        },
    );
    let (code, out, err) = vis(&d, &["pkg", "add", TEN, "--registry", &srv.url()]);
    assert_eq!(code, 0, "stdout: {out}\nstderr: {err}");
    assert!(d.join("vis.lock").is_file());
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 7 — khoá hex sai ⇒ lỗi cấu hình rõ, không ghi gì.
#[test]
fn khoa_sai_hex() {
    let d = tmp_dir("khoa-hex");
    let k = khoa_moi();
    let srv = server(&k.kp, &TuyChon::default());
    let (code, _out, err) = vis(
        &d,
        &[
            "pkg",
            "add",
            TEN,
            "--registry",
            &srv.url(),
            "--registry-key",
            "zz",
        ],
    );
    assert_eq!(code, 2, "stderr: {err}");
    assert!(err.contains("khoá công khai"), "{err}");
    assert!(!d.join(".vis").exists());
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 8 — khoá qua `VIS_REGISTRY_KEY` (cấu hình tương đương cờ) cũng xác minh.
#[test]
fn khoa_qua_bien_moi_truong() {
    let d = tmp_dir("khoa-env");
    let k = khoa_moi();
    let srv = server(&k.kp, &TuyChon::default());
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .current_dir(&d)
        .env("VIS_REGISTRY_KEY", &k.pub_hex)
        .args(["pkg", "add", TEN, "--registry", &srv.url()])
        .output()
        .expect("chạy vis");
    assert_eq!(
        o.status.code().unwrap_or(-1),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(d.join("vis.lock").is_file());
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 9 — `--registry-key` với registry **cục bộ** ⇒ lỗi rõ (không giả vờ đã kiểm chữ ký).
#[test]
fn khoa_voi_registry_cuc_bo_bi_tu_choi() {
    let d = tmp_dir("cuc-bo");
    let k = khoa_moi();
    let fx = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test/pkg/registry");
    let (code, _out, err) = vis(
        &d,
        &[
            "pkg",
            "add",
            TEN,
            "--registry",
            fx.to_str().unwrap(),
            "--registry-key",
            &k.pub_hex,
        ],
    );
    assert_eq!(code, 2, "stderr: {err}");
    assert!(err.contains("chỉ áp dụng cho registry"), "{err}");
    assert!(!d.join(".vis").exists());
    let _ = std::fs::remove_dir_all(&d);
}
