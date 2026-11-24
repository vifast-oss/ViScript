// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-4b (VD-271)** — CLI cookie/Basic auth: `--explain-perms` không lộ credential,
//! `--strict-caps` kiểm localhost.
//!
//! Phần API thư viện (9 test) đã sang `vis-host-native/tests/net_cookie_it.rs`
//! (ST-313c/VD-289).

// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-4b (VD-271)** — cookie jar + Basic auth của `http.get/head`, hermetic local server.
//!
//! Ca phủ: Set-Cookie → redirect → cookie; giữ giữa hai lời gọi; Secure không qua http; Path;
//! cross-host không gửi; hết hạn; Authorization Basic đúng; credential không lộ ra log
//! `--explain-perms`; auth không đi theo redirect khác host.

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use vis_cli::net::Net;
use vis_http::http_get;

fn tam(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vis-cookie-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    d
}

struct Server {
    addr: String,
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Server {
    fn moi(route: impl Fn(usize, &str) -> Vec<u8> + Send + 'static) -> Self {
        Self::moi_tren("127.0.0.1:0", route)
    }

    fn moi_tren(bind: &str, route: impl Fn(usize, &str) -> Vec<u8> + Send + 'static) -> Self {
        let l = TcpListener::bind(bind).expect("bind loopback");
        l.set_nonblocking(true).expect("non-blocking");
        let addr = l.local_addr().expect("local_addr").to_string();
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = Arc::clone(&stop);
        let dem = Arc::new(AtomicUsize::new(0));
        let tay = std::thread::spawn(move || {
            while !stop2.load(Ordering::Relaxed) {
                match l.accept() {
                    Ok((mut s, _)) => {
                        // Windows: socket do `accept` trả về có thể thừa hưởng non-blocking của
                        // listener ⇒ `read` trả `WouldBlock` ngay khi request chưa tới, server
                        // đọc rỗng (path "/") và đóng sớm ⇒ client thấy 10053. Đặt lại blocking.
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
                        let i = dem.fetch_add(1, Ordering::SeqCst);
                        let resp = route(i, &text);
                        let _ = s.write_all(&resp);
                        // Đóng chiều ghi + đọc nốt tới EOF/timeout ngắn để close êm trên Windows.
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
            handle: Some(tay),
        }
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

fn dap(status: &str, body: &str, them: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n{them}\r\n{body}",
        body.len()
    )
    .into_bytes()
}

fn header<'a>(req: &'a str, ten: &str) -> Option<&'a str> {
    req.lines().find_map(|l| {
        let (k, v) = l.split_once(':')?;
        if k.eq_ignore_ascii_case(ten) {
            Some(v.trim())
        } else {
            None
        }
    })
}

#[test]
fn basic_auth_dung_va_khong_lo_credential() {
    let s = Server::moi(|_, req| {
        assert_eq!(
            header(req, "Authorization"),
            Some("Basic dXNlcjpwYXNz"),
            "request: {req:?}"
        );
        dap("200 OK", "auth-ok", "")
    });
    let mut net = Net::moi(false);
    let u = format!("http://user:pass@{}/auth", s.addr);
    assert_eq!(http_get(&mut net, &u, 3_000).expect("auth"), "auth-ok");

    // Đường CLI + `--explain-perms`: log không được chứa credential.
    let d = tam("auth-log");
    let p = d.join("auth.vis");
    std::fs::write(
        &p,
        format!(
            "# SPDX-License-Identifier: MIT OR Apache-2.0\nuse std.http\n\
             match http.get(\"http://user:pass@{}/auth\"):\n\
             \x20   Ok(s): print(s)\n\
             \x20   Err(e): print(e)\n",
            s.addr
        ),
    )
    .expect("ghi chương trình");
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(["run", "--explain-perms"])
        .arg(&p)
        .output()
        .expect("chạy vis");
    assert_eq!(o.status.code(), Some(0), "stderr: {:?}", o.stderr);
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("NET http GET 127.0.0.1"), "stderr: {err}");
    assert!(!err.contains("user:pass"), "log lộ credential: {err}");
    assert!(!err.contains("dXNlcjpwYXNz"), "log lộ Authorization: {err}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn strict_caps_kiem_localhost() {
    let s = Server::moi(|_, _| dap("200 OK", "cap-ok", ""));
    let port = s.addr.rsplit_once(':').expect("port").1;
    let d = tam("cap");
    let p = d.join("cap.vis");
    std::fs::write(
        &p,
        format!(
            "# SPDX-License-Identifier: MIT OR Apache-2.0\nuse std.http\n\
             fn main(net: cap[Net] @ \"localhost\"):\n\
             \x20   match http.get(\"http://localhost:{port}/x\"):\n\
             \x20       Ok(s): print(s)\n\
             \x20       Err(e): print(e)\n"
        ),
    )
    .expect("ghi chương trình");
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(["run", "--strict-caps"])
        .arg(&p)
        .output()
        .expect("chạy vis");
    assert_eq!(o.status.code(), Some(0), "stderr: {:?}", o.stderr);
    assert_eq!(String::from_utf8_lossy(&o.stdout).trim(), "cap-ok");

    // Không khai cap ⇒ strict vẫn chặn (cap vẫn kiểm).
    let p2 = d.join("khong-cap.vis");
    std::fs::write(
        &p2,
        format!(
            "# SPDX-License-Identifier: MIT OR Apache-2.0\nuse std.http\n\
             fn main():\n\
             \x20   match http.get(\"http://localhost:{port}/x\"):\n\
             \x20       Ok(s): print(s)\n\
             \x20       Err(e): print(e)\n"
        ),
    )
    .expect("ghi chương trình");
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(["run", "--strict-caps"])
        .arg(&p2)
        .output()
        .expect("chạy vis");
    assert_eq!(o.status.code(), Some(2), "stderr: {:?}", o.stderr);
    assert!(
        String::from_utf8_lossy(&o.stderr).contains("VS0604"),
        "stderr: {:?}",
        o.stderr
    );
    let _ = std::fs::remove_dir_all(&d);
}
