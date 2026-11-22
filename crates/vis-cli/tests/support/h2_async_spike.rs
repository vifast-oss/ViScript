// SPDX-License-Identifier: MIT OR Apache-2.0
//! Test HTTP/2 async reactor-native (feature `async-h2`, default-off).
//! Tách khỏi `vs3_http2.rs` để giữ luật R1; `super::*` là crate root của test target.

use super::*;
use std::fs;
use std::path::Path;
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

/// **ST-296 (test-harness)**: các ca dưới đây chạy binary `vis` con với
/// `VIS_TLS_TEST_ROOT_DER`; hook chỉ có ở bản `debug_assertions` (`net/tls.rs`).
/// Release **bỏ qua có lý do** (khớp `vs3_registry_https.rs`), không đánh đổi coverage debug.
fn bo_qua_release(ten: &str) -> bool {
    if cfg!(debug_assertions) {
        return false;
    }
    eprintln!(
        "bỏ qua {ten}: VIS_TLS_TEST_ROOT_DER chỉ có ở bản debug (release không tin cert fixture)"
    );
    true
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
    // Luôn exec bản snapshot: `target/debug/vis` có thể bị cargo build khác ghi đè.
    let bin = snapshot_vis();
    chay_bang_bin(&bin, ten, src, args, vm)
}

/// Bản sao binary ổn định: tránh cargo build đồng thời (feature khác) ghi đè
/// `target/debug/vis` giữa các vòng soak.
fn snapshot_vis() -> PathBuf {
    // **ST-294 (triage soak cycle 31)**: cache theo **danh tính binary nguồn**
    // (đường dẫn + size + mtime), không theo PID. Bản cũ copy ~108 MB cho **mỗi process**
    // (`/tmp/vs3-w2-bin-<pid>`) và không dọn ⇒ soak 31 cycle làm đầy tmpfs 7,2 G, khiến
    // nhiều ca h2 fail rc101 vì hết chỗ — không phải lỗi sản phẩm.
    //
    // Ghi qua file tạm `vis.tmp-<pid>` rồi `rename` (atomic) để hai process song song không
    // exec file đang bị ghi (ETXTBSY); nếu đích đã có (process khác thắng) thì dùng đích.
    static SNAP: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    SNAP.get_or_init(|| {
        let src = Path::new(env!("CARGO_BIN_EXE_vis"));
        let meta = fs::metadata(src).expect("metadata vis");
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in src.to_string_lossy().bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x100_0000_01b3);
        }
        for b in meta.len().to_le_bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x100_0000_01b3);
        }
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        for b in mtime.to_le_bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x100_0000_01b3);
        }
        let d = std::env::temp_dir().join(format!("vs3-w2-bin-cache-{h:016x}"));
        let dst = d.join("vis");
        if !dst.is_file() {
            fs::create_dir_all(&d).expect("thư mục binary");
            let tmp = d.join(format!("vis.tmp-{}", std::process::id()));
            fs::copy(src, &tmp).expect("copy vis");
            if fs::rename(&tmp, &dst).is_err() {
                let _ = fs::remove_file(&tmp);
                assert!(dst.is_file(), "snapshot vis không tồn tại sau rename");
            }
        }
        dst
    })
    .clone()
}

fn chay_bang_bin(
    bin: &Path,
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
    let mut c = Command::new(bin);
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
    match http.get_timeout(url, 10000):
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
    if bo_qua_release("async_h2_production_bon_stream_mot_connection") {
        return;
    }
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
    if bo_qua_release("async_h2_production_timeout_va_goaway") {
        return;
    }
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
    if bo_qua_release("async_h2_production_cookie_redirect") {
        return;
    }
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
    if bo_qua_release("async_h2_production_cap_body") {
        return;
    }
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
    match http.get_timeout(url, 10000):
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
    if bo_qua_release("async_h2_production_bon_stream_cold") {
        return;
    }
    let s = MayChuSpike::moi(Kieu::Cho4);
    for vm in [false, true] {
        let args: Vec<String> = (0..4).map(|i| s.url(&format!("/c{i}"))).collect();
        let (code, out, err, bat) = chay_chuong_trinh("cold4", BON_TAC_VU_COLD, &args, vm);
        assert_eq!(code, 0, "cold4 vm={vm}: {err}");
        assert!(bat < Duration::from_secs(15), "cold4 treo vm={vm}");
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
    if bo_qua_release("async_h2_cold_multiplex_20_vong") {
        return;
    }
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
        assert!(bat < Duration::from_secs(15), "vòng {i} treo");
        let mut dong: Vec<String> = out.lines().map(str::to_string).collect();
        dong.sort_unstable();
        let mong: Vec<String> = (0..4).map(|j| format!("/r{i}-{j}")).collect();
        assert_eq!(dong, mong, "vòng {i} vm={vm} err={err:?}");
    }
    assert_eq!(s.conns(), rounds, "mỗi vòng đúng một connection");
    assert_eq!(s.streams(), rounds * 4, "mỗi connection đủ 4 stream");
}

#[test]
fn async_h2_production_redirect() {
    if bo_qua_release("async_h2_production_redirect") {
        return;
    }
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

// ===========================================================================
// **VS3-W2 soak (test-only, env-gated)** — không sửa production.
// `VS3_W2_SOAK_SECS` (mặc định 5 giây để L0 không chậm; soak thật 1800).
// ===========================================================================

fn proc_stats() -> (usize, u64, usize) {
    let fd = dem_fd();
    let status = fs::read_to_string("/proc/self/status").unwrap_or_default();
    let mut rss_kb = 0u64;
    let mut threads = 0usize;
    for dong in status.lines() {
        if let Some(v) = dong.strip_prefix("VmRSS:") {
            rss_kb = v
                .split_whitespace()
                .next()
                .and_then(|x| x.parse().ok())
                .unwrap_or(0);
        } else if let Some(v) = dong.strip_prefix("Threads:") {
            threads = v.trim().parse().unwrap_or(0);
        }
    }
    (fd, rss_kb, threads)
}

fn pct(ds: &mut [f64], p: f64) -> f64 {
    if ds.is_empty() {
        return 0.0;
    }
    ds.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let i = (((ds.len() - 1) as f64) * p).round() as usize;
    ds[i]
}

const SOAK_TIMEOUT_SRC: &str = r#"use std.http

async fn lay(url: str) -> str:
    match http.get_timeout(url, 400):
        Ok(s): s
        Err(e): "loi: {e}"

fn main(args: list[str]):
    print(await spawn lay(args[0]))
"#;

const SOAK_GOAWAY_SRC: &str = r#"use std.http

async fn lay(url: str) -> str:
    match http.get_timeout(url, 3000):
        Ok(s): s
        Err(e): "loi: {e}"

fn main(args: list[str]):
    print(await spawn lay(args[0]))
"#;

const SOAK_CAP_SRC: &str = r#"use std.http

async fn lay(url: str) -> str:
    match http.get_timeout(url, 20000):
        Ok(s): s
        Err(e): "loi: {e}"

fn main(args: list[str]):
    print(await spawn lay(args[0]))
"#;

const SOAK_REDIRECT_SRC: &str = r#"use std.http

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

#[test]
fn async_h2_soak_env_gated() {
    if bo_qua_release("async_h2_soak_env_gated") {
        return;
    }
    let secs: u64 = std::env::var("VS3_W2_SOAK_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);
    if secs == 0 {
        eprintln!("soak: bỏ qua (VS3_W2_SOAK_SECS=0)");
        return;
    }
    // Mặc định không chạm cap trong smoke ngắn; soak thật đặt `..._CAP_EVERY=5`.
    let cap_moi: usize = std::env::var("VS3_W2_SOAK_CAP_EVERY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(50)
        .max(1);
    let batch = 30usize;
    let cold = MayChuSpike::moi(Kieu::Cho4);
    let thuong = MayChuSpike::moi(Kieu::Thuong);
    let treo = MayChuSpike::moi(Kieu::Treo);
    let goaway = MayChuSpike::moi(Kieu::GoAway);
    let (fd0, rss0, th0) = proc_stats();
    let bin = snapshot_vis();
    let bat = Instant::now();
    let mut cold_wall: Vec<f64> = Vec::new();
    let mut cold_runs = 0usize;
    let mut redirect_runs = 0usize;
    let mut cap_runs = 0usize;
    let mut timeout_runs = 0usize;
    let mut goaway_runs = 0usize;
    let mut i = 0usize;
    let mut luc_batch = Instant::now();
    let mut loi: Option<String> = None;
    while bat.elapsed() < Duration::from_secs(secs) {
        let vm = i % 2 == 1;
        let c0 = (cold.conns(), cold.streams());
        let args: Vec<String> = (0..4).map(|j| cold.url(&format!("/s{i}-{j}"))).collect();
        let (code, out, err, wall) = chay_bang_bin(&bin, "soak-cold", BON_TAC_VU_COLD, &args, vm);
        if code != 0 {
            loi = Some(format!("cold vòng {i} vm={vm}: {err}"));
            break;
        }
        let mut dong: Vec<String> = out.lines().map(str::to_string).collect();
        dong.sort_unstable();
        let mong: Vec<String> = (0..4).map(|j| format!("/s{i}-{j}")).collect();
        if dong != mong {
            loi = Some(format!("cold vòng {i} sai output: {out:?}"));
            break;
        }
        if (cold.conns() - c0.0, cold.streams() - c0.1) != (1, 4) {
            loi = Some(format!(
                "cold vòng {i}: connection/stream = {:?}/{:?}",
                cold.conns() - c0.0,
                cold.streams() - c0.1
            ));
            break;
        }
        cold_wall.push(wall.as_secs_f64() * 1_000.0);
        cold_runs += 1;

        // Redirect + Set-Cookie: 4 request tuần tự trên một connection (idle pool).
        let t0 = (thuong.conns(), thuong.streams());
        let args = vec![
            thuong.url("/redir-cookie"),
            thuong.url("/echo-cookie"),
            thuong.url("/set-cookie"),
            thuong.url("/echo-cookie"),
        ];
        let (code, out, err, _) = chay_chuong_trinh("soak-cookie", SOAK_REDIRECT_SRC, &args, vm);
        if code != 0 || out != "sid=xyz\nsid=xyz\nset\nsid=abc\n" {
            loi = Some(format!(
                "redirect/cookie vòng {i} vm={vm}: code={code} out={out:?} err={err}"
            ));
            break;
        }
        if (thuong.conns() - t0.0, thuong.streams() - t0.1) != (1, 5) {
            loi = Some(format!(
                "redirect vòng {i}: connection/stream = {:?}/{:?}",
                thuong.conns() - t0.0,
                thuong.streams() - t0.1
            ));
            break;
        }
        redirect_runs += 1;

        // Cap 8 MiB (mỗi `cap_moi` vòng để soak không chỉ đo allocator).
        if i.is_multiple_of(cap_moi) {
            let c0 = (thuong.conns(), thuong.streams());
            let (code, out, err, _) =
                chay_chuong_trinh("soak-cap", SOAK_CAP_SRC, &[thuong.url("/big")], vm);
            if code != 0
                || !(out.contains("8 MiB")
                    || out.contains("vượt giới hạn")
                    || out.contains("HTTP/2"))
            {
                loi = Some(format!(
                    "cap vòng {i} vm={vm}: code={code} out={out:?} err={err}"
                ));
                break;
            }
            if (thuong.conns() - c0.0, thuong.streams() - c0.1) != (1, 1) {
                loi = Some(format!("cap vòng {i}: connection/stream sai"));
                break;
            }
            cap_runs += 1;
        }

        // Timeout 400 ms + GOAWAY.
        let (code, out, err, _) = chay_bang_bin(
            &bin,
            "soak-timeout",
            SOAK_TIMEOUT_SRC,
            &[treo.url("/x")],
            vm,
        );
        if code != 0 || !out.contains("thời gian") {
            loi = Some(format!(
                "timeout vòng {i} vm={vm}: code={code} out={out:?} err={err}"
            ));
            break;
        }
        timeout_runs += 1;
        let (code, out, err, _) = chay_bang_bin(
            &bin,
            "soak-goaway",
            SOAK_GOAWAY_SRC,
            &[goaway.url("/x")],
            vm,
        );
        if code != 0 || !(out.contains("GOAWAY") || out.contains("HTTP/2")) {
            loi = Some(format!(
                "goaway vòng {i} vm={vm}: code={code} out={out:?} err={err}"
            ));
            break;
        }
        goaway_runs += 1;

        i += 1;
        if i.is_multiple_of(batch) || luc_batch.elapsed() >= Duration::from_secs(30) {
            let (fd, rss, th) = proc_stats();
            let p50 = pct(&mut cold_wall.clone(), 0.50);
            let p95 = pct(&mut cold_wall.clone(), 0.95);
            eprintln!(
                "soak batch: vòng={i} giây={:.0} fd={fd} rss={:.1}MB threads={th} \
                 cold={} redirect={} cap={} timeout={} goaway={} \
                 conns(cold/thuong)={}/{} streams={}/{} cold_p50={p50:.0}ms cold_p95={p95:.0}ms",
                bat.elapsed().as_secs_f64(),
                rss as f64 / 1024.0,
                cold_runs,
                redirect_runs,
                cap_runs,
                timeout_runs,
                goaway_runs,
                cold.conns(),
                thuong.conns(),
                cold.streams(),
                thuong.streams()
            );
            luc_batch = Instant::now();
        }
    }
    let (fd1, rss1, th1) = proc_stats();
    let p50 = pct(&mut cold_wall.clone(), 0.50);
    let p95 = pct(&mut cold_wall.clone(), 0.95);
    eprintln!(
        "soak KẾT THÚC: vòng={i} cold={cold_runs} redirect={redirect_runs} cap={cap_runs} \
         timeout={timeout_runs} goaway={goaway_runs} giây={:.1} \
         fd={fd0}->{fd1} rss={:.1}->{:.1}MB threads={th0}->{th1} \
         conns(cold/thuong/treo/goaway)={}/{}/{}/{} streams(cold/thuong)={}/{} \
         cold_p50={p50:.0}ms cold_p95={p95:.0}ms",
        bat.elapsed().as_secs_f64(),
        rss0 as f64 / 1024.0,
        rss1 as f64 / 1024.0,
        cold.conns(),
        thuong.conns(),
        treo.conns(),
        goaway.conns(),
        cold.streams(),
        thuong.streams()
    );
    if let Some(e) = loi {
        panic!("soak FAIL: {e}");
    }
    assert_eq!(cold.conns(), cold_runs, "cold: mỗi vòng đúng 1 connection");
    assert_eq!(
        cold.streams(),
        cold_runs * 4,
        "cold: mỗi connection 4 stream"
    );
    assert_eq!(
        thuong.conns(),
        redirect_runs + cap_runs,
        "thuong: 1 connection/lần"
    );
    assert_eq!(
        thuong.streams(),
        redirect_runs * 5 + cap_runs,
        "thuong: 5 stream/lần redirect-cookie + 1 cap"
    );
    assert!(fd1 <= fd0 + 8, "rò fd: {fd0} -> {fd1}");
    assert!(th1 <= th0 + 2, "rò thread: {th0} -> {th1}");
    assert!(
        rss1 <= rss0 + 262_144,
        "RSS tăng quá 256 MB: {rss0} -> {rss1} KB"
    );
    eprintln!("soak: PASS");
}

/// Rà riêng cap 8 MiB qua production (test-only) để đếm tần suất lỗi scheduler.
/// `VS3_W2_CAP_ROUNDS` (mặc định 5 để gate ngắn).
#[test]
fn async_h2_cap_loop_env_gated() {
    // 0 = bỏ qua: đây là test chẩn đoán cap, không chạy mặc định trong gate.
    let n: usize = std::env::var("VS3_W2_CAP_ROUNDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    if n == 0 {
        return;
    }
    let s = MayChuSpike::moi(Kieu::Thuong);
    for i in 0..n {
        let (code, out, err, _) =
            chay_chuong_trinh("cap-loop", SOAK_CAP_SRC, &[s.url("/big")], i % 2 == 1);
        assert_eq!(code, 0, "cap vòng {i} vm={}: {err}", i % 2 == 1);
        assert!(
            out.contains("8 MiB") || out.contains("vượt giới hạn") || out.contains("HTTP/2"),
            "cap vòng {i}: out={out:?} err={err:?}"
        );
    }
    eprintln!("cap loop: {n}/{n} PASS");
}
