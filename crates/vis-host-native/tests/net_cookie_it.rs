// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-4b (VD-271)** — cookie jar + Basic auth của `http.get/head` (API thư viện).
//!
//! Tách từ `vis-cli/tests/net_pkg_it/vs3_cookie_auth.rs` (ST-313c/VD-289);
//! 2 ca CLI (`--explain-perms`, `--strict-caps`) giữ nguyên ở crate `vis-cli`.

// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-4b (VD-271)** — cookie jar + Basic auth của `http.get/head`, hermetic local server.
//!
//! Ca phủ: Set-Cookie → redirect → cookie; giữ giữa hai lời gọi; Secure không qua http; Path;
//! cross-host không gửi; hết hạn; Authorization Basic đúng; credential không lộ ra log
//! `--explain-perms`; auth không đi theo redirect khác host.

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use vis_host_native::net::Net;
use vis_http::http_get;

struct Server {
    addr: String,
    log: Arc<Mutex<Vec<String>>>,
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
        let log = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let log2 = Arc::clone(&log);
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
                        log2.lock().expect("log").push(text.clone());
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
            log,
            stop,
            handle: Some(tay),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.addr)
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

fn path_cua(req: &str) -> &str {
    req.lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("/")
}

#[test]
fn set_cookie_qua_redirect_duoc_gui_lai() {
    let s = Server::moi(|i, req| match i {
        0 => {
            assert_eq!(path_cua(req), "/a");
            dap(
                "302 Found",
                "",
                "Set-Cookie: sid=abc; Path=/\r\nLocation: /b\r\n",
            )
        }
        1 => {
            assert_eq!(path_cua(req), "/b");
            assert_eq!(header(req, "Cookie"), Some("sid=abc"));
            dap("200 OK", "dich", "")
        }
        _ => dap("500 Internal Server Error", "", ""),
    });
    let mut net = Net::moi(false);
    assert_eq!(
        http_get(&mut net, &s.url("/a"), 3_000).expect("redirect + cookie"),
        "dich"
    );
    assert_eq!(s.log.lock().expect("log").len(), 2);
}

#[test]
fn cookie_giu_giua_hai_loi_goi() {
    let s = Server::moi(|i, req| match i {
        0 => dap("200 OK", "ok", "Set-Cookie: p=1; Path=/\r\n"),
        1 => {
            assert_eq!(header(req, "Cookie"), Some("p=1"));
            dap("200 OK", "ok2", "")
        }
        _ => dap("500 Internal Server Error", "", ""),
    });
    let mut net = Net::moi(false);
    assert_eq!(
        http_get(&mut net, &s.url("/set"), 3_000).expect("set"),
        "ok"
    );
    assert_eq!(
        http_get(&mut net, &s.url("/get"), 3_000).expect("get"),
        "ok2"
    );
}

#[test]
fn secure_khong_gui_qua_http() {
    let s = Server::moi(|i, req| match i {
        0 => dap("200 OK", "", "Set-Cookie: sec=1; Secure; Path=/\r\n"),
        1 => {
            assert_eq!(
                header(req, "Cookie"),
                None,
                "Secure không được gửi qua http"
            );
            dap("200 OK", "ok", "")
        }
        _ => dap("500 Internal Server Error", "", ""),
    });
    let mut net = Net::moi(false);
    assert_eq!(http_get(&mut net, &s.url("/set"), 3_000).expect("set"), "");
    assert_eq!(
        http_get(&mut net, &s.url("/get"), 3_000).expect("get"),
        "ok"
    );
}

#[test]
fn cookie_path_va_cross_host() {
    let s = Server::moi(|i, req| match i {
        0 => dap("200 OK", "", "Set-Cookie: pa=1; Path=/a\r\n"),
        1 => {
            assert_eq!(path_cua(req), "/a/b");
            assert_eq!(header(req, "Cookie"), Some("pa=1"));
            dap("200 OK", "a", "")
        }
        2 => {
            assert_eq!(path_cua(req), "/b");
            assert_eq!(header(req, "Cookie"), None, "Path /a không khớp /b");
            dap("200 OK", "b", "")
        }
        3 => {
            assert_eq!(path_cua(req), "/a/b");
            assert_eq!(
                header(req, "Cookie"),
                None,
                "host khác không được thấy cookie"
            );
            dap("200 OK", "a2", "")
        }
        4 => {
            assert_eq!(path_cua(req), "/a/b");
            assert_eq!(
                header(req, "Cookie"),
                Some("pa=1"),
                "cookie host cũ vẫn còn"
            );
            dap("200 OK", "a3", "")
        }
        _ => dap("500 Internal Server Error", "", ""),
    });
    let mut net = Net::moi(false);
    assert_eq!(http_get(&mut net, &s.url("/set"), 3_000).expect("set"), "");
    assert_eq!(http_get(&mut net, &s.url("/a/b"), 3_000).expect("a"), "a");
    assert_eq!(http_get(&mut net, &s.url("/b"), 3_000).expect("b"), "b");
    assert_eq!(
        http_get(
            &mut net,
            &format!(
                "http://localhost:{}/a/b",
                s.addr.rsplit_once(':').unwrap().1
            ),
            3_000
        )
        .expect("localhost"),
        "a2"
    );
    assert_eq!(
        http_get(&mut net, &s.url("/a/b"), 3_000).expect("lại 127"),
        "a3"
    );
}

#[test]
fn cookie_het_han_bi_bo() {
    let s = Server::moi(|i, req| match i {
        0 => dap("200 OK", "", "Set-Cookie: tmp=1; Path=/\r\n"),
        1 => dap("200 OK", "", "Set-Cookie: tmp=; Path=/; Max-Age=0\r\n"),
        2 => {
            assert_eq!(header(req, "Cookie"), None, "Max-Age=0 phải xoá cookie");
            dap("200 OK", "ok", "")
        }
        _ => dap("500 Internal Server Error", "", ""),
    });
    let mut net = Net::moi(false);
    assert_eq!(http_get(&mut net, &s.url("/set"), 3_000).expect("set"), "");
    assert_eq!(http_get(&mut net, &s.url("/xoa"), 3_000).expect("xoá"), "");
    assert_eq!(
        http_get(&mut net, &s.url("/get"), 3_000).expect("get"),
        "ok"
    );
}

#[test]
fn auth_khong_di_theo_redirect_khac_host() {
    let b = Server::moi(|_, req| {
        assert_eq!(header(req, "Authorization"), None, "không được chuyển auth");
        dap("200 OK", "B", "")
    });
    let location = format!("http://{}/b", b.addr);
    let a = Server::moi(move |_, _| dap("302 Found", "", &format!("Location: {location}\r\n")));
    let mut net = Net::moi(false);
    let u = format!("http://user:pass@{}/a", a.addr);
    assert_eq!(http_get(&mut net, &u, 3_000).expect("redirect"), "B");
}

#[test]
fn head_cung_luu_cookie() {
    let s = Server::moi(|i, req| match i {
        0 => {
            assert_eq!(path_cua(req), "/h");
            b"HTTP/1.1 204 No Content\r\nSet-Cookie: h=1; Path=/\r\nConnection: close\r\n\r\n"
                .to_vec()
        }
        1 => {
            assert_eq!(header(req, "Cookie"), Some("h=1"));
            dap("200 OK", "ok", "")
        }
        _ => dap("500 Internal Server Error", "", ""),
    });
    let mut net = Net::moi(false);
    let (st, _) = vis_http::http_head(&mut net, &s.url("/h"), 3_000).expect("HEAD");
    assert_eq!(st, 204);
    assert_eq!(
        http_get(&mut net, &s.url("/get"), 3_000).expect("get"),
        "ok"
    );
}

#[test]
fn localhost_bind_127_va_ipv6() {
    let s = Server::moi(|_, req| {
        assert_eq!(path_cua(req), "/x");
        dap("200 OK", "ok", "")
    });
    let port = s.addr.rsplit_once(':').expect("port").1;
    let mut net = Net::moi(false);
    assert_eq!(http_get(&mut net, &s.url("/x"), 3_000).expect("127"), "ok");
    assert_eq!(
        http_get(&mut net, &format!("http://localhost:{port}/x"), 3_000)
            .expect("localhost thử mọi địa chỉ"),
        "ok"
    );

    // `[::1]` chỉ chạy khi môi trường bật IPv6 loopback; server bind riêng.
    match TcpListener::bind("[::1]:0") {
        Ok(l) => drop(l),
        Err(e) => {
            eprintln!("bỏ qua [::1]: IPv6 loopback không dùng được ({e})");
            return;
        }
    }
    let s6 = Server::moi_tren("[::1]:0", |_, req| {
        assert_eq!(path_cua(req), "/x");
        dap("200 OK", "ok6", "")
    });
    assert!(s6.addr.starts_with("[::1]:"), "địa chỉ: {}", s6.addr);
    assert_eq!(
        http_get(&mut net, &s6.url("/x"), 3_000).expect("[::1]"),
        "ok6"
    );

    // Host không loopback bị từ chối trước khi nối.
    let e = http_get(&mut net, "http://example.com/", 300).expect_err("remote");
    assert!(e.contains("loopback"), "{e}");
    let e = http_get(&mut net, "http://10.0.0.1/", 300).expect_err("remote ip");
    assert!(e.contains("loopback"), "{e}");
}

#[test]
fn userinfo_sai_khong_panic_va_khong_lo() {
    let mut net = Net::moi(false);
    let e = http_get(&mut net, "http://user@127.0.0.1:1/", 300).expect_err("phải lỗi");
    assert!(!e.contains("user@"), "lỗi lộ userinfo: {e}");
    assert!(e.contains("user:pass") || e.contains("userinfo"), "{e}");
}
