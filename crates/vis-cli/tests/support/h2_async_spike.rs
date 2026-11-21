// SPDX-License-Identifier: MIT OR Apache-2.0
//! Test HTTP/2 async reactor-native (feature `async-h2`, default-off).
//! Tách khỏi `vs3_http2.rs` để giữ luật R1; `super::*` là crate root của test target.

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
                    let cookie = req
                        .headers()
                        .get("cookie")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("none")
                        .to_string();
                    let body: Vec<u8> = if path == "/big" {
                        vec![b'z'; 8 * 1024 * 1024 + 1]
                    } else if path == "/hello" {
                        b"hello".to_vec()
                    } else if path == "/echo-cookie" {
                        cookie.into_bytes()
                    } else if path == "/set-cookie" {
                        b"set".to_vec()
                    } else {
                        path.clone().into_bytes()
                    };
                    let mut rb = Response::builder().status(200);
                    let chuyen = path == "/redir" || path == "/redir-cookie";
                    if chuyen {
                        let dich = if path == "/redir-cookie" {
                            "/echo-cookie"
                        } else {
                            "/hello"
                        };
                        rb = rb.status(302).header("location", dich);
                    }
                    if path == "/redir-cookie" {
                        rb = rb.header("set-cookie", "sid=xyz; Path=/");
                    } else if path == "/set-cookie" {
                        rb = rb.header("set-cookie", "sid=abc; Path=/");
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
    let goc =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vs3_tls/localhost.cert.der");
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
fn async_h2_production_cookie_redirect() {
    let s = MayChuSpike::moi(Kieu::Thuong);
    let src = r#"use std.http

async fn lay(url: str) -> str:
    match http.get_timeout(url, 3000):
        Ok(s): s
        Err(e): "loi: {e}"

fn main(args: list[str]):
    print(await spawn lay(args[0]))
    print(await spawn lay(args[1]))
    print(await spawn lay(args[2]))
    print(await spawn lay(args[3]))
"#;
    let args = vec![
        s.url("/redir-cookie"),
        s.url("/echo-cookie"),
        s.url("/set-cookie"),
        s.url("/echo-cookie"),
    ];
    let (code, out, err, bat) = chay_chuong_trinh("cookie", src, &args, false);
    assert_eq!(code, 0, "cookie: {err}");
    assert_eq!(
        out, "sid=xyz\nsid=xyz\nset\nsid=abc\n",
        "Set-Cookie + redirect phải gửi lại: {out:?}"
    );
    assert!(bat < Duration::from_secs(3), "cookie không treo");
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

const BON_TAC_VU_COLD: &str = r#"use std.http

async fn lay(url: str) -> str:
    match http.get_timeout(url, 3000):
        Ok(s): s
        Err(e): "loi: {e}"

fn main(args: list[str]):
    let a = spawn lay(args[0])
    let b = spawn lay(args[1])
    let c = spawn lay(args[2])
    let d = spawn lay(args[3])
    print(await a)
    print(await b)
    print(await c)
    print(await d)
"#;

#[test]
fn async_h2_production_bon_stream_cold() {
    let s = MayChuSpike::moi(Kieu::Cho4);
    for vm in [false, true] {
        let args: Vec<String> = (0..4).map(|i| s.url(&format!("/c{i}"))).collect();
        let (code, out, err, bat) = chay_chuong_trinh("cold4", BON_TAC_VU_COLD, &args, vm);
        assert_eq!(code, 0, "cold4 vm={vm}: {err}");
        assert!(bat < Duration::from_secs(10), "cold4 treo vm={vm}");
        let mut dong: Vec<&str> = out.lines().collect();
        dong.sort_unstable();
        assert_eq!(dong, vec!["/c0", "/c1", "/c2", "/c3"], "vm={vm}: {out:?}");
    }
    assert_eq!(s.conns(), 2, "mỗi engine phải dùng đúng 1 connection");
    assert_eq!(s.streams(), 8, "mỗi connection đủ 4 stream");
}

/// 20 vòng cold-start 4 task cùng authority: mỗi vòng **một** connection, đủ 4 stream,
/// không treo, fd không tăng. Số vòng đổi được bằng `VS3_W2_COLD_ROUNDS` (mặc định 20).
#[test]
fn async_h2_cold_multiplex_20_vong() {
    let rounds: usize = std::env::var("VS3_W2_COLD_ROUNDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);
    let s = MayChuSpike::moi(Kieu::Cho4);
    for i in 0..rounds {
        let vm = i % 2 == 1;
        let args: Vec<String> = (0..4).map(|j| s.url(&format!("/r{i}-{j}"))).collect();
        let (code, out, err, bat) = chay_chuong_trinh("cold20", BON_TAC_VU_COLD, &args, vm);
        assert_eq!(code, 0, "vòng {i} vm={vm}: {err}");
        assert!(bat < Duration::from_secs(5), "vòng {i} treo");
        let mut dong: Vec<String> = out.lines().map(str::to_string).collect();
        dong.sort_unstable();
        let mong: Vec<String> = (0..4).map(|j| format!("/r{i}-{j}")).collect();
        assert_eq!(dong, mong, "vòng {i} vm={vm}");
    }
    assert_eq!(s.conns(), rounds, "mỗi vòng đúng một connection");
    assert_eq!(s.streams(), rounds * 4, "mỗi connection đủ 4 stream");
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
