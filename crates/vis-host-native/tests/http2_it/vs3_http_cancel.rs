// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-287 (VD-273)** — huỷ task giữa request HTTP: `FrameReactorBackend::cancel_task_handles`
//! đóng tay HTTP async tại safe point (`RST_STREAM` cho h2, đóng socket cho h1), không rò fd,
//! không treo, parity interp↔VM.
//!
//! Test chạy **frame scheduler thật** + backend production [`AsyncNet`]; server loopback chậm
//! gửi thân để task kịp đăng ký đọc rồi bị huỷ. Không đổi API sync/default; ca h2 chỉ biên dịch
//! khi bật `--features async-h2` (default-off).

use std::cell::RefCell;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use vis_core::frame::{FrameConfig, FrameOutput};
use vis_host_native::async_net::AsyncNet;
use vis_interp::frame::run_frame_for_test as run_i;
use vis_runtime::rt::{
    frame_cancel_after_bytes_for_test, frame_cancel_after_turns_for_test,
    frame_cancel_clear_triggers_for_test,
};
use vis_syntax::{lex, parse, Mode};
use vis_vm::vm_frame::run_frame_for_test as run_v;

fn tien_trinh(src: &str) -> (vis_hir::Hir, vis_hir::Symbols, vis_hir::TypeTable) {
    let p = parse(src, Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse: {:?}", p.diagnostics);
    let l = lex(src, Mode::Script);
    let h = vis_types::lower_with(
        &p.cst,
        &l,
        vis_types::LowerOptions {
            std_mods: vec!["http".into()],
            ..Default::default()
        },
    );
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    (h.hir, h.symbols, h.types)
}

/// Server HTTP/1.1 loopback: gửi header + một phần thân rồi **giữ** kết nối (thân chậm).
struct MayChuH1 {
    addr: String,
    stop: Arc<AtomicBool>,
    conns: Arc<AtomicUsize>,
    da_gui: Arc<AtomicUsize>,
    threads: Arc<Mutex<Vec<std::thread::JoinHandle<()>>>>,
    listener: Option<std::thread::JoinHandle<()>>,
}

impl MayChuH1 {
    fn moi() -> Self {
        let l = TcpListener::bind("127.0.0.1:0").expect("bind");
        l.set_nonblocking(true).expect("nonblocking");
        let addr = l.local_addr().expect("addr").to_string();
        let stop = Arc::new(AtomicBool::new(false));
        let conns = Arc::new(AtomicUsize::new(0));
        let da_gui = Arc::new(AtomicUsize::new(0));
        let threads = Arc::new(Mutex::new(Vec::new()));
        let stop2 = Arc::clone(&stop);
        let conns2 = Arc::clone(&conns);
        let gui2 = Arc::clone(&da_gui);
        let threads2 = Arc::clone(&threads);
        let listener = std::thread::spawn(move || {
            while !stop2.load(Ordering::Relaxed) {
                match l.accept() {
                    Ok((tcp, _)) => {
                        conns2.fetch_add(1, Ordering::SeqCst);
                        let stop3 = Arc::clone(&stop2);
                        let gui3 = Arc::clone(&gui2);
                        let t = std::thread::spawn(move || phuc_vu_h1(tcp, stop3, gui3));
                        threads2.lock().unwrap().push(t);
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
            conns,
            da_gui,
            threads,
            listener: Some(listener),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.addr)
    }

    fn conns(&self) -> usize {
        self.conns.load(Ordering::SeqCst)
    }

    fn da_gui(&self) -> usize {
        self.da_gui.load(Ordering::SeqCst)
    }
}

impl Drop for MayChuH1 {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.listener.take() {
            let _ = h.join();
        }
        let ds: Vec<_> = self.threads.lock().unwrap().drain(..).collect();
        for h in ds {
            let _ = h.join();
        }
    }
}

fn phuc_vu_h1(mut tcp: TcpStream, stop: Arc<AtomicBool>, da_gui: Arc<AtomicUsize>) {
    // **ST-287 (Windows)**: socket `accept` trên Windows **thừa hưởng nonblocking** từ listener;
    // handler này dùng API blocking + timeout. Thiếu bước này ⇒ `read`/`write_all` trả
    // `WSAEWOULDBLOCK`/abort (10053) giữa body, client thấy lỗi trước khi safe point huỷ chạy.
    tcp.set_nonblocking(false).expect("server blocking");
    let _ = tcp.set_nodelay(true);
    let _ = tcp.set_read_timeout(Some(Duration::from_secs(3)));
    let _ = tcp.set_write_timeout(Some(Duration::from_secs(1)));
    let mut buf = [0u8; 4096];
    let mut req = Vec::new();
    while !req.windows(4).any(|w| w == b"\r\n\r\n") {
        match tcp.read(&mut buf) {
            Ok(0) => return,
            Ok(n) => req.extend_from_slice(&buf[..n]),
            Err(_) => return,
        }
        if req.len() > 64 * 1024 {
            return;
        }
    }
    let dong = String::from_utf8_lossy(&req);
    let nhanh = dong
        .lines()
        .next()
        .map(|l| l.contains(" /nhanh "))
        .unwrap_or(false);
    if nhanh {
        let _ =
            tcp.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok");
        let _ = tcp.flush();
        let _ = tcp.shutdown(std::net::Shutdown::Both);
        return;
    }
    // `/im`: không bao giờ gửi header hoàn chỉnh, chỉ nhỏ giọt byte để client có wake.
    if dong
        .lines()
        .next()
        .map(|l| l.contains(" /im "))
        .unwrap_or(false)
    {
        let bat = Instant::now();
        while !stop.load(Ordering::Relaxed) && bat.elapsed() < Duration::from_secs(5) {
            if tcp.write_all(b"X").is_err() {
                return;
            }
            let _ = tcp.flush();
            std::thread::sleep(Duration::from_millis(10));
        }
        return;
    }
    let mut resp = b"HTTP/1.1 200 OK\r\nContent-Length: 1000000\r\n\r\n".to_vec();
    resp.extend(std::iter::repeat_n(b'x', 4096));
    if tcp.write_all(&resp).is_err() {
        return;
    }
    let _ = tcp.flush();
    da_gui.fetch_add(4096, Ordering::SeqCst);
    let bat = Instant::now();
    while !stop.load(Ordering::Relaxed) && bat.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(10));
        if tcp.write_all(&[b'x'; 1024]).is_err() {
            return;
        }
        let _ = tcp.flush();
        da_gui.fetch_add(1024, Ordering::SeqCst);
    }
}

const SO_TURN: u64 = 20;

fn cfg_huy(net: &Rc<RefCell<AsyncNet>>) -> FrameConfig {
    FrameConfig {
        backend: Some(net.clone()),
        ..Default::default()
    }
}

fn nguon_cham(url: &str) -> String {
    format!(
        r#"use std.http

async fn cham(url: str) -> int:
    print("V:pre")
    match http.get_timeout(url, 4000):
        Ok(s): print(s)
        Err(e): print("loi: {{e}}")
    print("V:post")
    1

fn main():
    let t = spawn cham("{url}")
    print("main")
"#
    )
}

fn chay_i(src: &str, net: &Rc<RefCell<AsyncNet>>, id: u32) -> FrameOutput {
    let (hir, syms, tys) = tien_trinh(src);
    frame_cancel_clear_triggers_for_test();
    // **ST-287 (P0)**: h1 huỷ theo **byte đã đọc trên handle C** (giữa body), không chỉ turn;
    // turn là fallback cho h2 (`H`) và server chưa gửi header (`/im`).
    frame_cancel_after_bytes_for_test("C", 8192, id);
    frame_cancel_after_turns_for_test(SO_TURN, id);
    let out = run_i(src, &hir, &syms, &tys, &[], cfg_huy(net));
    frame_cancel_clear_triggers_for_test();
    out
}

fn chay_v(src: &str, net: &Rc<RefCell<AsyncNet>>, id: u32) -> FrameOutput {
    let (hir, syms, _) = tien_trinh(src);
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    frame_cancel_clear_triggers_for_test();
    frame_cancel_after_bytes_for_test("C", 8192, id);
    frame_cancel_after_turns_for_test(SO_TURN, id);
    let out = run_v(&m, src, &[], cfg_huy(net));
    frame_cancel_clear_triggers_for_test();
    out
}

fn baseline() -> vis_core::frame::DiagSnapshot {
    vis_core::frame::DiagSnapshot {
        live_tasks: 0,
        frames: 0,
        waiters: 0,
        ready: 0,
        regs: 0,
        timers: 0,
    }
}

fn kiem_sau_huy(out: &FrameOutput) {
    assert_eq!(out.exit, 0, "huỷ không tạo lỗi: {:?}", out.diagnostics);
    assert!(out.stdout.contains("main"), "{:?}", out.stdout);
    assert!(out.stdout.contains("V:pre"), "{:?}", out.stdout);
    // **ST-287 (P0)**: kiểm dấu vết huỷ TRƯỚC — trigger tới muộn sẽ thiếu `cancel:1`
    // ngay tại đây thay vì chỉ thấy V:post.
    assert!(
        out.log.iter().any(|l| l == "cancel:1"),
        "phải huỷ task 1 trước khi nó chạy tiếp: stdout={:?} log={:?}",
        out.stdout,
        out.log
    );
    assert!(
        !out.stdout.contains("V:post"),
        "task bị huỷ phải dừng: stdout={:?} log={:?}",
        out.stdout,
        out.log
    );
    assert_eq!(out.snapshot, baseline(), "{:?}", out.log);
}

#[test]
fn cancel_http_h1_dong_socket_va_parity() {
    let s = MayChuH1::moi();
    let src = nguon_cham(&s.url("/cham"));
    let ni = Rc::new(RefCell::new(AsyncNet::new(false)));
    let i = chay_i(&src, &ni, 1);
    let nv = Rc::new(RefCell::new(AsyncNet::new(false)));
    let v = chay_v(&src, &nv, 1);

    for (ten, out, net) in [("interp", &i, &ni), ("vm", &v, &nv)] {
        kiem_sau_huy(out);
        assert_eq!(
            net.borrow().dem_tay_dang_mo(),
            0,
            "{ten}: rò tay sau huỷ: {:?}",
            out.log
        );
    }
    assert_eq!(i.stdout, v.stdout, "parity stdout");
    assert_eq!(i.snapshot, v.snapshot, "parity snapshot");
    assert_eq!(i.exit, v.exit, "parity exit");
    // Log thật có thể lệch một `rdone`/deadline do thời điểm EOF — so dấu vết huỷ + đóng tay.
    for out in [&i, &v] {
        assert!(
            out.log.iter().any(|l| l.starts_with("canceldereg:1:")),
            "phải gỡ đăng ký h1: {:?}",
            out.log
        );
        assert!(
            out.log.iter().any(|l| l == "cancelregs:1:1"),
            "phải gỡ đúng một đăng ký: {:?}",
            out.log
        );
    }
    assert!(s.da_gui() > 4096, "server phải đã gửi phần thân đầu");
    assert_eq!(s.conns(), 2, "mỗi engine một connection");
}

fn nguon_thuong(url: &str) -> String {
    format!(
        r#"use std.http

async fn lay(url: str) -> str:
    match http.get_timeout(url, 3000):
        Ok(s): s
        Err(e): "loi: {{e}}"

fn main():
    print(await spawn lay("{url}"))
"#
    )
}

#[test]
fn cancel_http_h1_lap_nhieu_vong_khong_ro() {
    let s = MayChuH1::moi();
    let src = nguon_cham(&s.url("/cham"));
    let net = Rc::new(RefCell::new(AsyncNet::new(false)));
    for vong in 0..5 {
        let bat = Instant::now();
        let out = chay_i(&src, &net, 1);
        kiem_sau_huy(&out);
        assert!(
            bat.elapsed() < Duration::from_secs(3),
            "vòng {vong} treo: {:?}",
            bat.elapsed()
        );
        assert_eq!(
            net.borrow().dem_tay_dang_mo(),
            0,
            "vòng {vong}: rò tay: {:?}",
            out.log
        );
    }
    assert!(
        s.da_gui() > 4096,
        "phải huỷ giữa thân chậm, không phải trước body"
    );
    assert_eq!(s.conns(), 5, "mỗi vòng một connection h1");
}

#[test]
fn cancel_http_h1_sau_huy_van_goi_duoc() {
    let s = MayChuH1::moi();
    let net = Rc::new(RefCell::new(AsyncNet::new(false)));
    let huy = chay_i(&nguon_cham(&s.url("/cham")), &net, 1);
    kiem_sau_huy(&huy);
    assert_eq!(net.borrow().dem_tay_dang_mo(), 0, "sau huỷ: {:?}", huy.log);

    let thuong = nguon_thuong(&s.url("/nhanh"));
    let (hir, syms, tys) = tien_trinh(&thuong);
    frame_cancel_clear_triggers_for_test();
    let out = run_i(
        &thuong,
        &hir,
        &syms,
        &tys,
        &[],
        FrameConfig {
            backend: Some(net.clone()),
            ..Default::default()
        },
    );
    assert_eq!(out.exit, 0, "{:?}", out.diagnostics);
    assert_eq!(out.stdout, "ok\n");
    assert_eq!(net.borrow().dem_tay_dang_mo(), 0, "{:?}", out.log);
    assert_eq!(s.conns(), 2, "hai connection: một bị huỷ, một request sau");
}

#[test]
fn cancel_http_h1_server_khong_tra_header() {
    let s = MayChuH1::moi();
    let src = nguon_cham(&s.url("/im"));
    let net = Rc::new(RefCell::new(AsyncNet::new(false)));
    let bat = Instant::now();
    let out = chay_i(&src, &net, 1);
    kiem_sau_huy(&out);
    assert!(
        bat.elapsed() < Duration::from_secs(3),
        "huỷ giữa header phải dừng nhanh: {:?}",
        bat.elapsed()
    );
    assert_eq!(net.borrow().dem_tay_dang_mo(), 0, "{:?}", out.log);
}

fn nguon_hai_task(slow: &str, fast: &str) -> String {
    format!(
        r#"use std.http

async fn cham(url: str) -> int:
    print("V:pre")
    match http.get_timeout(url, 4000):
        Ok(s): print(s)
        Err(e): print("loi: {{e}}")
    print("V:post")
    1

async fn nhanh(url: str) -> str:
    match http.get_timeout(url, 3000):
        Ok(s): s
        Err(e): "loi: {{e}}"

fn main():
    let a = spawn cham("{slow}")
    let b = spawn nhanh("{fast}")
    print(await b)
"#
    )
}

#[test]
fn cancel_http_h1_mot_task_khong_anh_huong_task_khac() {
    let s = MayChuH1::moi();
    let src = nguon_hai_task(&s.url("/cham"), &s.url("/nhanh"));
    let net = Rc::new(RefCell::new(AsyncNet::new(false)));
    let out = chay_i(&src, &net, 1);
    assert_eq!(out.exit, 0, "{:?}", out.diagnostics);
    assert!(out.stdout.contains("V:pre"), "{:?}", out.stdout);
    assert!(
        out.log.iter().any(|l| l == "cancel:1"),
        "phải huỷ task 1 trước khi nó chạy tiếp: stdout={:?} log={:?}",
        out.stdout,
        out.log
    );
    assert!(
        !out.stdout.contains("V:post"),
        "task bị huỷ phải dừng: stdout={:?} log={:?}",
        out.stdout,
        out.log
    );
    assert!(
        out.stdout.contains("ok"),
        "task còn lại phải xong: {:?}",
        out.stdout
    );
    assert!(out.log.iter().any(|l| l == "cancel:1"), "{:?}", out.log);
    assert_eq!(net.borrow().dem_tay_dang_mo(), 0, "{:?}", out.log);
    assert_eq!(s.conns(), 2, "mỗi task một connection");
}

// ===========================================================================
// **HTTP/2 async** (`--features async-h2`, default-off): huỷ giữa body phải gửi
// `RST_STREAM` đúng stream và nhả user của session, không đóng session dùng chung.
// ===========================================================================

#[cfg(feature = "async-h2")]
mod h2 {
    use super::*;

    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll, Waker};

    use bytes::Bytes;
    use http::Response;
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
    use rustls::{ServerConfig, ServerConnection, StreamOwned};
    use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

    const CERT: &[u8] = include_bytes!("../../../../test/fixtures/vs3_tls/localhost.cert.der");
    const KEY: &[u8] = include_bytes!("../../../../test/fixtures/vs3_tls/localhost.key.der");

    fn cau_hinh_tls() -> Arc<ServerConfig> {
        let nha = Arc::new(rustls::crypto::ring::default_provider());
        let cert = CertificateDer::from(CERT.to_vec());
        let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(KEY.to_vec()));
        let mut cfg = ServerConfig::builder_with_provider(nha)
            .with_safe_default_protocol_versions()
            .expect("TLS versions")
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)
            .expect("server cert");
        cfg.alpn_protocols = vec![b"h2".to_vec()];
        Arc::new(cfg)
    }

    struct MayChuH2 {
        addr: String,
        stop: Arc<AtomicBool>,
        conns: Arc<AtomicUsize>,
        streams: Arc<AtomicUsize>,
        /// Số stream mà `send_data` báo lỗi sau khi client gửi `RST_STREAM`.
        resets: Arc<AtomicUsize>,
        threads: Arc<Mutex<Vec<std::thread::JoinHandle<()>>>>,
        listener: Option<std::thread::JoinHandle<()>>,
    }

    impl MayChuH2 {
        fn moi() -> Self {
            let l = TcpListener::bind("127.0.0.1:0").expect("bind");
            l.set_nonblocking(true).expect("nonblocking");
            let addr = l.local_addr().expect("addr").to_string();
            let stop = Arc::new(AtomicBool::new(false));
            let conns = Arc::new(AtomicUsize::new(0));
            let streams = Arc::new(AtomicUsize::new(0));
            let resets = Arc::new(AtomicUsize::new(0));
            let threads = Arc::new(Mutex::new(Vec::new()));
            let stop2 = Arc::clone(&stop);
            let conns2 = Arc::clone(&conns);
            let streams2 = Arc::clone(&streams);
            let resets2 = Arc::clone(&resets);
            let threads2 = Arc::clone(&threads);
            let listener = std::thread::spawn(move || {
                while !stop2.load(Ordering::Relaxed) {
                    match l.accept() {
                        Ok((tcp, _)) => {
                            conns2.fetch_add(1, Ordering::SeqCst);
                            let stop3 = Arc::clone(&stop2);
                            let streams3 = Arc::clone(&streams2);
                            let resets3 = Arc::clone(&resets2);
                            let t = std::thread::spawn(move || {
                                let _ = phuc_vu_h2(tcp, stop3, streams3, resets3);
                            });
                            threads2.lock().unwrap().push(t);
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
                conns,
                streams,
                resets,
                threads,
                listener: Some(listener),
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

        fn resets(&self) -> usize {
            self.resets.load(Ordering::SeqCst)
        }
    }

    impl Drop for MayChuH2 {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(h) = self.listener.take() {
                let _ = h.join();
            }
            let ds: Vec<_> = self.threads.lock().unwrap().drain(..).collect();
            for h in ds {
                let _ = h.join();
            }
        }
    }

    fn phuc_vu_h2(
        tcp: TcpStream,
        stop: Arc<AtomicBool>,
        streams: Arc<AtomicUsize>,
        resets: Arc<AtomicUsize>,
    ) -> std::io::Result<()> {
        tcp.set_nonblocking(false)?;
        tcp.set_read_timeout(Some(Duration::from_secs(10)))?;
        tcp.set_write_timeout(Some(Duration::from_secs(10)))?;
        let conn = ServerConnection::new(cau_hinh_tls()).map_err(std::io::Error::other)?;
        let s = StreamOwned::new(conn, tcp);
        s.sock.set_nonblocking(true)?;
        let io = Adapter {
            s,
            han: Instant::now() + Duration::from_secs(20),
        };
        let mut conn = cho(::h2::server::handshake(io))
            .map_err(|e| std::io::Error::other(format!("h2 server handshake: {e}")))?;
        let w = Waker::noop();
        let mut cx = Context::from_waker(w);
        // Giữ `SendStream` mở (chưa END_STREAM) để client ở giữa body.
        let mut giu: Vec<::h2::SendStream<Bytes>> = Vec::new();
        let mut lan_gui = Instant::now();
        while !stop.load(Ordering::Relaxed) {
            match conn.poll_accept(&mut cx) {
                Poll::Ready(Some(Ok((_req, mut respond)))) => {
                    streams.fetch_add(1, Ordering::SeqCst);
                    let resp = Response::builder()
                        .status(200)
                        .body(())
                        .map_err(std::io::Error::other)?;
                    let mut send = respond
                        .send_response(resp, false)
                        .map_err(|e| std::io::Error::other(format!("h2 response: {e}")))?;
                    send.send_data(Bytes::from_static(b"abc"), false)
                        .map_err(|e| std::io::Error::other(format!("h2 data: {e}")))?;
                    giu.push(send);
                }
                Poll::Ready(Some(Err(e))) => {
                    return Err(std::io::Error::other(format!("h2 accept: {e}")))
                }
                Poll::Ready(None) => return Ok(()),
                Poll::Pending => {}
            }
            // Thân chậm: nhỏ giọt byte để reactor client có wake thật (đo turn kịp).
            if lan_gui.elapsed() >= Duration::from_millis(5) {
                lan_gui = Instant::now();
                giu.retain_mut(|send| {
                    if send.send_data(Bytes::from_static(b"x"), false).is_ok() {
                        true
                    } else {
                        resets.fetch_add(1, Ordering::SeqCst);
                        false
                    }
                });
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        Ok(())
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
        ) -> Poll<std::io::Result<()>> {
            let this = self.get_mut();
            if buf.remaining() == 0 {
                return Poll::Ready(Ok(()));
            }
            if this.han <= Instant::now() {
                return Poll::Ready(Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
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
        ) -> Poll<std::io::Result<usize>> {
            let this = self.get_mut();
            if this.han <= Instant::now() {
                return Poll::Ready(Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "quá thời gian h2 server",
                )));
            }
            match this.s.write(buf) {
                Ok(n) => Poll::Ready(Ok(n)),
                Err(e) if tam_thoi(&e) => Poll::Pending,
                Err(e) => Poll::Ready(Err(e)),
            }
        }
        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            let this = self.get_mut();
            match this.s.flush() {
                Ok(()) => Poll::Ready(Ok(())),
                Err(e) if tam_thoi(&e) => Poll::Pending,
                Err(e) => Poll::Ready(Err(e)),
            }
        }
        fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            self.poll_flush(cx)
        }
    }

    fn tam_thoi(e: &std::io::Error) -> bool {
        matches!(
            e.kind(),
            std::io::ErrorKind::WouldBlock
                | std::io::ErrorKind::TimedOut
                | std::io::ErrorKind::Interrupted
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

    fn net_h2() -> Rc<RefCell<AsyncNet>> {
        Rc::new(RefCell::new(
            AsyncNet::moi_voi_goc_tls(false, &[CERT]).expect("cấu hình TLS test"),
        ))
    }

    #[test]
    fn cancel_http_h2_rst_stream_va_parity() {
        let s = MayChuH2::moi();
        let src = nguon_cham(&s.url("/cham"));
        let ni = net_h2();
        let i = chay_i(&src, &ni, 1);
        let nv = net_h2();
        let v = chay_v(&src, &nv, 1);

        for (ten, out, net) in [("interp", &i, &ni), ("vm", &v, &nv)] {
            kiem_sau_huy(out);
            assert_eq!(
                net.borrow().dem_tay_dang_mo(),
                0,
                "{ten}: h2 còn user sau huỷ: {:?}",
                out.log
            );
        }
        assert_eq!(i.stdout, v.stdout, "parity stdout");
        assert_eq!(i.snapshot, v.snapshot, "parity snapshot");
        assert_eq!(i.exit, v.exit, "parity exit");
        // Log thật chứa deadline monotonic còn lại (`:399x`) nên chỉ so dấu vết huỷ.
        for out in [&i, &v] {
            assert!(
                out.log.iter().any(|l| l.starts_with("canceldereg:1:")),
                "phải gỡ đăng ký h2: {:?}",
                out.log
            );
            assert!(
                out.log.iter().any(|l| l == "cancelregs:1:1"),
                "phải gỡ đúng một đăng ký: {:?}",
                out.log
            );
        }
        assert_eq!(s.streams(), 2, "mỗi engine một stream h2");
        assert_eq!(s.conns(), 2, "mỗi engine một connection h2");
        // RST_STREAM đúng stream: server thấy stream chết (send_data lỗi) trong khi
        // connection vẫn được session h2 giữ cho request sau.
        let han = Instant::now() + Duration::from_secs(2);
        while s.resets() < 2 && Instant::now() < han {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            s.resets() >= 2,
            "client phải gửi RST_STREAM cho cả hai stream bị huỷ (reset={})",
            s.resets()
        );
    }

    fn nguon_hai_cham(url: &str) -> String {
        format!(
            r#"use std.http

async fn cham(url: str) -> int:
    print("V:pre")
    match http.get_timeout(url, 4000):
        Ok(s): print(s)
        Err(e): print("loi: {{e}}")
    print("V:post")
    1

fn main():
    let a = spawn cham("{url}")
    let b = spawn cham("{url}")
    print("main")
"#
        )
    }

    /// Hai task **dùng chung một session h2**: huỷ task 1 không được xoá sổ/đóng stream của
    /// task 2; huỷ cả hai phải `RST_STREAM` đủ hai stream và nhả hết user.
    #[test]
    fn cancel_http_h2_hai_task_chung_session() {
        let s = MayChuH2::moi();
        let src = nguon_hai_cham(&s.url("/cham"));
        let net = net_h2();
        let (hir, syms, tys) = tien_trinh(&src);
        frame_cancel_clear_triggers_for_test();
        frame_cancel_after_turns_for_test(20, 1);
        frame_cancel_after_turns_for_test(40, 2);
        let out = run_i(&src, &hir, &syms, &tys, &[], cfg_huy(&net));
        frame_cancel_clear_triggers_for_test();

        assert_eq!(out.exit, 0, "{:?}", out.diagnostics);
        assert_eq!(out.stdout.matches("V:pre").count(), 2, "{:?}", out.stdout);
        assert!(!out.stdout.contains("V:post"), "{:?}", out.stdout);
        assert!(out.log.iter().any(|l| l == "cancel:1"), "{:?}", out.log);
        assert!(out.log.iter().any(|l| l == "cancel:2"), "{:?}", out.log);
        assert_eq!(out.snapshot, baseline(), "{:?}", out.log);
        assert_eq!(net.borrow().dem_tay_dang_mo(), 0, "{:?}", out.log);
        assert_eq!(s.conns(), 1, "hai task cùng authority dùng chung session");
        assert_eq!(s.streams(), 2, "hai stream trên một session");
        let han = Instant::now() + Duration::from_secs(2);
        while s.resets() < 2 && Instant::now() < han {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            s.resets() >= 2,
            "phải RST cả hai stream (reset={})",
            s.resets()
        );
    }

    /// Ba vòng huỷ trên **cùng backend**: session h2 được tái dùng nhưng user/stream phải
    /// nhả hết mỗi vòng, không rò tay hay connection.
    #[test]
    fn cancel_http_h2_lap_khong_ro() {
        let s = MayChuH2::moi();
        let src = nguon_cham(&s.url("/cham"));
        let net = net_h2();
        for vong in 0..3 {
            let out = chay_i(&src, &net, 1);
            kiem_sau_huy(&out);
            assert_eq!(
                net.borrow().dem_tay_dang_mo(),
                0,
                "vòng {vong}: rò user h2: {:?}",
                out.log
            );
        }
        assert!(
            s.conns() <= 1,
            "session h2 phải được tái dùng sau RST: {} connection",
            s.conns()
        );
        assert_eq!(s.streams(), 3, "mỗi vòng một stream");
    }
}
