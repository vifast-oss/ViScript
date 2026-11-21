// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-4b2/4b3 (VD-271)** — client HTTP/2 cho `https://` (feature `http2`, default-off).
//!
//! - TLS handshake dùng ALPN `["h2","http/1.1"]`; nếu server không chọn `h2` thì trả lại **cùng**
//!   `KenhTls` để tầng HTTP/1.1 dùng tiếp (không mở thêm kết nối/không đốt một slot server test).
//! - **4b3**: giữ một `PhienH2` (một connection + `SendRequest`) cho mỗi authority; nhiều request
//!   **tuần tự** dùng lại connection.
//! - **4b6 prototype (test-only)**: `gui_nhieu` gửi 2–4 request **trước khi đọc** response ⇒ nhiều
//!   stream in-flight trên một connection (h2 quản stream id/flow-control; test local chứng minh).
//!   API production vẫn đồng bộ nên `Net` dùng hàng đợi tuần tự; `gui_nhieu` chỉ có ở `cfg(test)`.
//! - `h2` cần `AsyncRead/AsyncWrite` + executor; module này bọc `KenhTls` **đồng bộ** bằng adapter
//!   nonblocking + một executor một luồng tối thiểu (`poll` + `yield`). Không bật tokio runtime/
//!   network feature, không thêm async vào đường HTTP/1.1.

use std::future::Future;
use std::io::{self, Read, Write};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

use bytes::Bytes;
use http::Request;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use vis_interp::builtin::KetQuaHttp2;
use vis_interp::net::MAX_HTTP;

use super::tls::{self, CauHinh, KenhTls};

/// Cap header phản hồi HTTP/2 (cùng ngưỡng HTTP/1.1 64 KiB) để server không làm phình bộ nhớ.
const MAX_HEADER: usize = 64 * 1024;

fn loi_h2(e: &::h2::Error, viec: &str) -> String {
    let loai = if e.is_go_away() {
        "GOAWAY"
    } else if e.is_reset() {
        "RST_STREAM"
    } else {
        "lỗi"
    };
    format!("{loai} HTTP/2 khi {viec}: {e}")
}

/// Kết quả bước mở phiên: hoặc phiên h2 dùng lại được, hoặc trả lại kết nối TLS cho HTTP/1.1.
pub(super) enum MoH2 {
    Phien(PhienH2),
    Http1(Box<KenhTls>),
}

/// Idle tối đa của một phiên h2 trước khi Net đóng (VS3-4b4).
pub(super) const H2_IDLE: Duration = Duration::from_secs(15);

/// Request cho prototype multiplex test-only.
#[cfg(test)]
pub(super) struct YeuCauH2 {
    pub target: String,
    pub method: String,
    pub headers: Vec<(String, String)>,
}

/// Một connection h2 đã handshake, dùng cho nhiều request **tuần tự** cùng authority.
pub(super) struct PhienH2 {
    send: ::h2::client::SendRequest<Bytes>,
    conn: Pin<Box<::h2::client::Connection<DongBo, Bytes>>>,
    han: Arc<Mutex<Instant>>,
    /// Authority TLS (`host:port`) để dựng URI tuyệt đối/`:authority` cho mọi request.
    authority: String,
    /// Lần dùng thành công gần nhất; `Net` dọn khi quá [`H2_IDLE`].
    luc_dung: Instant,
}

impl PhienH2 {
    /// Mở TLS + h2 handshake. `Http1(k)` = server không chọn ALPN h2, caller dùng `k`.
    pub(super) fn moi(cfg: &CauHinh, dia_chi: &str, timeout_ms: u64) -> Result<MoH2, String> {
        let con = timeout_ms.max(1);
        let k = tls::noi(cfg, dia_chi, con)?;
        if k.conn.alpn_protocol() != Some(&b"h2"[..]) {
            return Ok(MoH2::Http1(Box::new(k)));
        }
        // h2 cần `poll` không chặn: socket nonblocking + adapter trả `Pending`; executor một luồng tự
        // chạy lại. Nếu fallback h1 ở trên, socket vẫn blocking cho tầng HTTP/1.1.
        k.sock
            .set_nonblocking(true)
            .map_err(|e| format!("không đặt non-blocking cho HTTP/2 `{dia_chi}`: {e}"))?;
        let han = Arc::new(Mutex::new(Instant::now() + Duration::from_millis(con)));
        let io = DongBo {
            k,
            han: Arc::clone(&han),
        };
        let (send, conn) = cho(::h2::client::handshake(io))
            .map_err(|e| format!("lỗi HTTP/2 handshake tới `{dia_chi}`: {e}"))?;
        Ok(MoH2::Phien(Self {
            send,
            conn: Box::pin(conn),
            han,
            authority: dia_chi.to_string(),
            luc_dung: Instant::now(),
        }))
    }

    /// Gửi một request tuần tự và đọc response; connection vẫn sống cho request sau.
    pub(super) fn request(
        &mut self,
        target: &str,
        method: &str,
        headers: &[(String, String)],
        timeout_ms: u64,
    ) -> Result<KetQuaHttp2, String> {
        let han = Instant::now() + Duration::from_millis(timeout_ms.max(1));
        if let Ok(mut h) = self.han.lock() {
            *h = han;
        }
        let uri = format!("https://{}{}", self.authority, target);
        let mut builder = Request::builder().method(method).uri(uri);
        for (k, v) in headers {
            builder = builder.header(k.as_str(), v.as_str());
        }
        let req = builder
            .body(())
            .map_err(|e| format!("request HTTP/2 không hợp lệ: {e}"))?;
        let (resp, _body) = self
            .send
            .send_request(req, true)
            .map_err(|e| loi_h2(&e, "gửi request"))?;
        let kq = self.doc_phan_hoi(resp, method)?;
        self.luc_dung = Instant::now();
        Ok(kq)
    }

    /// Phiên đã idle quá [`H2_IDLE`] tại `now`? Net gọi trước khi tái dùng.
    pub(super) fn qua_idle(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.luc_dung) >= H2_IDLE
    }

    /// **Prototype test-only (VS3-4b6)**: gửi nhiều request **trước khi đọc** response ⇒ nhiều stream
    /// h2 in-flight trên cùng connection. API production vẫn đồng bộ (một request/lần); hàm này chứng
    /// minh khả năng multiplex + flow-control của h2. `Vec` kết quả giữ đúng thứ tự đầu vào.
    #[cfg(test)]
    pub(super) fn gui_nhieu(
        &mut self,
        ds: &[YeuCauH2],
        timeout_ms: u64,
    ) -> Vec<Result<KetQuaHttp2, String>> {
        let han = Instant::now() + Duration::from_millis(timeout_ms.max(1));
        if let Ok(mut h) = self.han.lock() {
            *h = han;
        }
        struct DangGui {
            resp: Option<Pin<Box<::h2::client::ResponseFuture>>>,
            recv: Option<::h2::RecvStream>,
            status: u16,
            headers: Vec<(String, String)>,
            body: Vec<u8>,
            xong_data: bool,
            ket_qua: Option<Result<KetQuaHttp2, String>>,
        }
        let mut dang: Vec<DangGui> = Vec::with_capacity(ds.len());
        for y in ds {
            let uri = format!("https://{}{}", self.authority, y.target);
            let mut builder = Request::builder().method(y.method.as_str()).uri(uri);
            for (k, v) in &y.headers {
                builder = builder.header(k.as_str(), v.as_str());
            }
            let req = match builder.body(()) {
                Ok(r) => r,
                Err(e) => {
                    dang.push(DangGui {
                        resp: None,
                        recv: None,
                        status: 0,
                        headers: Vec::new(),
                        body: Vec::new(),
                        xong_data: false,
                        ket_qua: Some(Err(format!("request HTTP/2 không hợp lệ: {e}"))),
                    });
                    continue;
                }
            };
            match self.send.send_request(req, true) {
                Ok((resp, _)) => dang.push(DangGui {
                    resp: Some(Box::pin(resp)),
                    recv: None,
                    status: 0,
                    headers: Vec::new(),
                    body: Vec::new(),
                    xong_data: false,
                    ket_qua: None,
                }),
                Err(e) => dang.push(DangGui {
                    resp: None,
                    recv: None,
                    status: 0,
                    headers: Vec::new(),
                    body: Vec::new(),
                    xong_data: false,
                    ket_qua: Some(Err(loi_h2(&e, "gửi request"))),
                }),
            }
        }
        let w = Waker::noop();
        let mut cx = Context::from_waker(w);
        loop {
            if dang.iter().all(|d| d.ket_qua.is_some()) {
                break;
            }
            let mut tien = false;
            match self.conn.as_mut().poll(&mut cx) {
                Poll::Ready(Ok(())) => {}
                Poll::Ready(Err(e)) => {
                    let loi = loi_h2(&e, "kết nối");
                    for d in &mut dang {
                        if d.ket_qua.is_none() {
                            d.ket_qua = Some(Err(loi.clone()));
                        }
                    }
                    break;
                }
                Poll::Pending => {}
            }
            for d in &mut dang {
                if d.ket_qua.is_some() {
                    continue;
                }
                if d.recv.is_none() {
                    let Some(resp) = d.resp.as_mut() else {
                        continue;
                    };
                    match resp.as_mut().poll(&mut cx) {
                        Poll::Ready(Ok(r)) => {
                            let (parts, b) = r.into_parts();
                            d.status = parts.status.as_u16();
                            let tong: usize = parts
                                .headers
                                .iter()
                                .map(|(k, v)| k.as_str().len() + v.as_bytes().len())
                                .sum();
                            if tong > MAX_HEADER {
                                d.ket_qua = Some(Err(format!(
                                    "header HTTP/2 quá lớn (> {MAX_HEADER} byte)"
                                )));
                                continue;
                            }
                            d.headers = parts
                                .headers
                                .iter()
                                .map(|(k, v)| {
                                    (
                                        k.as_str().to_string(),
                                        String::from_utf8_lossy(v.as_bytes()).into_owned(),
                                    )
                                })
                                .collect();
                            d.recv = Some(b);
                            tien = true;
                        }
                        Poll::Ready(Err(e)) => {
                            d.ket_qua = Some(Err(loi_h2(&e, "response")));
                            continue;
                        }
                        Poll::Pending => {}
                    }
                }
                if let Some(r) = d.recv.as_mut() {
                    if !d.xong_data {
                        match r.poll_data(&mut cx) {
                            Poll::Ready(Some(Ok(b))) => {
                                if d.body.len().saturating_add(b.len()) > MAX_HTTP {
                                    d.ket_qua = Some(Err(format!(
                                        "phản hồi HTTP/2 vượt giới hạn {MAX_HTTP} byte (8 MiB)"
                                    )));
                                } else {
                                    d.body.extend_from_slice(&b);
                                    tien = true;
                                }
                            }
                            Poll::Ready(Some(Err(e))) => {
                                d.ket_qua = Some(Err(loi_h2(&e, "đọc body")));
                            }
                            Poll::Ready(None) => {
                                d.xong_data = true;
                                tien = true;
                            }
                            Poll::Pending => {}
                        }
                    } else {
                        match r.poll_trailers(&mut cx) {
                            Poll::Ready(Ok(_)) => {
                                d.ket_qua = Some(Ok(KetQuaHttp2 {
                                    status: d.status,
                                    headers: std::mem::take(&mut d.headers),
                                    body: std::mem::take(&mut d.body),
                                }));
                            }
                            Poll::Ready(Err(e)) => {
                                d.ket_qua = Some(Err(loi_h2(&e, "trailer")));
                            }
                            Poll::Pending => {}
                        }
                    }
                }
            }
            if !tien {
                std::thread::yield_now();
            }
        }
        self.luc_dung = Instant::now();
        dang.into_iter()
            .map(|d| {
                d.ket_qua
                    .unwrap_or_else(|| Err("HTTP/2 không hoàn tất".to_string()))
            })
            .collect()
    }

    fn doc_phan_hoi(
        &mut self,
        resp: ::h2::client::ResponseFuture,
        _method: &str,
    ) -> Result<KetQuaHttp2, String> {
        let w = Waker::noop();
        let mut cx = Context::from_waker(w);
        let mut resp = Box::pin(resp);
        let mut recv: Option<::h2::RecvStream> = None;
        let mut status = 0u16;
        let mut headers: Vec<(String, String)> = Vec::new();
        let mut body: Vec<u8> = Vec::new();
        let mut xong_data = false;

        loop {
            let mut tien = false;
            match self.conn.as_mut().poll(&mut cx) {
                Poll::Ready(Ok(())) => {}
                Poll::Ready(Err(e)) => return Err(loi_h2(&e, "kết nối")),
                Poll::Pending => {}
            }
            if recv.is_none() {
                match resp.as_mut().poll(&mut cx) {
                    Poll::Ready(Ok(r)) => {
                        let (parts, b) = r.into_parts();
                        status = parts.status.as_u16();
                        let tong: usize = parts
                            .headers
                            .iter()
                            .map(|(k, v)| k.as_str().len() + v.as_bytes().len())
                            .sum();
                        if tong > MAX_HEADER {
                            return Err(format!("header HTTP/2 quá lớn (> {MAX_HEADER} byte)"));
                        }
                        headers = parts
                            .headers
                            .iter()
                            .map(|(k, v)| {
                                (
                                    k.as_str().to_string(),
                                    String::from_utf8_lossy(v.as_bytes()).into_owned(),
                                )
                            })
                            .collect();
                        recv = Some(b);
                        tien = true;
                    }
                    Poll::Ready(Err(e)) => return Err(loi_h2(&e, "response")),
                    Poll::Pending => {}
                }
            }
            if let Some(r) = recv.as_mut() {
                if !xong_data {
                    match r.poll_data(&mut cx) {
                        Poll::Ready(Some(Ok(b))) => {
                            if body.len().saturating_add(b.len()) > MAX_HTTP {
                                return Err(format!(
                                    "phản hồi HTTP/2 vượt giới hạn {MAX_HTTP} byte (8 MiB)"
                                ));
                            }
                            body.extend_from_slice(&b);
                            tien = true;
                        }
                        Poll::Ready(Some(Err(e))) => return Err(loi_h2(&e, "đọc body")),
                        Poll::Ready(None) => {
                            xong_data = true;
                            tien = true;
                        }
                        Poll::Pending => {}
                    }
                } else {
                    // Đọc hết trailer (bỏ nội dung) để stream đóng sạch trước khi tái dùng.
                    match r.poll_trailers(&mut cx) {
                        Poll::Ready(Ok(_)) => {
                            return Ok(KetQuaHttp2 {
                                status,
                                headers,
                                body,
                            });
                        }
                        Poll::Ready(Err(e)) => return Err(loi_h2(&e, "trailer")),
                        Poll::Pending => {}
                    }
                }
            }
            if !tien {
                std::thread::yield_now();
            }
        }
    }
}

/// Adapter `AsyncRead/AsyncWrite` trên [`KenhTls`] đồng bộ; mỗi lần chờ dùng phần thời gian còn lại.
struct DongBo {
    k: KenhTls,
    han: Arc<Mutex<Instant>>,
}

impl DongBo {
    fn het_han(&self) -> bool {
        self.han
            .lock()
            .map(|h| *h <= Instant::now())
            .unwrap_or(true)
    }
}

impl AsyncRead for DongBo {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if buf.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        if this.het_han() {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "quá thời gian HTTP/2",
            )));
        }
        match this.k.read(buf.initialize_unfilled()) {
            Ok(n) => {
                buf.advance(n);
                Poll::Ready(Ok(()))
            }
            Err(e) if tam_thoi(&e) => Poll::Pending,
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

impl AsyncWrite for DongBo {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        if this.het_han() {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "quá thời gian HTTP/2",
            )));
        }
        match this.k.write(buf) {
            Ok(n) => Poll::Ready(Ok(n)),
            Err(e) if tam_thoi(&e) => Poll::Pending,
            Err(e) => Poll::Ready(Err(e)),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        match this.k.flush() {
            Ok(()) => Poll::Ready(Ok(())),
            Err(e) if tam_thoi(&e) => {
                if this.het_han() {
                    Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "quá thời gian HTTP/2",
                    )))
                } else {
                    Poll::Pending
                }
            }
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

/// Executor một luồng: IO adapter không bao giờ trả `Pending`; chỉ poll + yield khi h2 chưa sẵn sàng.
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

#[cfg(test)]
mod tests {
    use super::*;
    use http::Response;
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
    use rustls::{ServerConfig, ServerConnection, StreamOwned};
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

    const CERT: &[u8] = include_bytes!("../../tests/fixtures/vs3_tls/localhost.cert.der");
    const KEY: &[u8] = include_bytes!("../../tests/fixtures/vs3_tls/localhost.key.der");

    struct MayChuIo {
        s: StreamOwned<ServerConnection, TcpStream>,
        han: Instant,
    }

    impl MayChuIo {
        fn het_han(&self) -> bool {
            self.han <= Instant::now()
        }
    }

    impl AsyncRead for MayChuIo {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            let this = self.get_mut();
            if buf.remaining() == 0 {
                return Poll::Ready(Ok(()));
            }
            if this.het_han() {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "quá thời gian server test",
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

    impl AsyncWrite for MayChuIo {
        fn poll_write(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            let this = self.get_mut();
            if this.het_han() {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "quá thời gian server test",
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

    fn cau_hinh_server() -> Arc<ServerConfig> {
        let nha = Arc::new(rustls::crypto::ring::default_provider());
        let cert = CertificateDer::from(CERT.to_vec());
        let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(KEY.to_vec()));
        let mut cfg = ServerConfig::builder_with_provider(nha)
            .with_safe_default_protocol_versions()
            .expect("TLS")
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)
            .expect("cert");
        cfg.alpn_protocols = vec![b"h2".to_vec()];
        Arc::new(cfg)
    }

    /// Server chờ đủ 4 request **trước khi** trả response ⇒ chứng minh 4 stream in-flight.
    fn server_cho_4() -> (String, Arc<AtomicUsize>) {
        let l = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = l.local_addr().expect("addr").to_string();
        let dem = Arc::new(AtomicUsize::new(0));
        let dem2 = Arc::clone(&dem);
        let cfg = cau_hinh_server();
        std::thread::spawn(move || {
            let Ok((tcp, _)) = l.accept() else { return };
            tcp.set_nonblocking(true).expect("nonblocking");
            let Ok(conn) = ServerConnection::new(cfg) else {
                return;
            };
            let io = MayChuIo {
                s: StreamOwned::new(conn, tcp),
                han: Instant::now() + Duration::from_secs(5),
            };
            let mut conn = match cho(::h2::server::handshake(io)) {
                Ok(c) => c,
                Err(_) => return,
            };
            let w = Waker::noop();
            let mut cx = Context::from_waker(w);
            let mut ds = Vec::new();
            while ds.len() < 4 {
                match conn.poll_accept(&mut cx) {
                    Poll::Ready(Some(Ok(x))) => {
                        dem2.fetch_add(1, Ordering::SeqCst);
                        ds.push(x);
                    }
                    Poll::Ready(Some(Err(_))) | Poll::Ready(None) => return,
                    Poll::Pending => std::thread::yield_now(),
                }
            }
            for (req, mut respond) in ds {
                let body = req.uri().path().as_bytes().to_vec();
                let Ok(resp) = Response::builder().status(200).body(()) else {
                    return;
                };
                let Ok(mut send) = respond.send_response(resp, false) else {
                    return;
                };
                if send.send_data(Bytes::from(body), true).is_err() {
                    return;
                }
            }
            // Đẩy response ra rồi chờ client đóng.
            loop {
                match conn.poll_closed(&mut cx) {
                    Poll::Ready(_) => return,
                    Poll::Pending => std::thread::yield_now(),
                }
            }
        });
        (addr, dem)
    }

    #[test]
    fn bon_stream_in_flight_mot_connection() {
        let (addr, dem) = server_cho_4();
        let cfg = tls::tu_goc_der(&[CERT]).expect("root test");
        let cfg = tls::cau_hinh_h2(&cfg);
        let mut phien = match PhienH2::moi(&cfg, &addr, 3_000).expect("h2 session") {
            MoH2::Phien(p) => p,
            MoH2::Http1(_) => panic!("server phải chọn ALPN h2"),
        };
        let ds: Vec<YeuCauH2> = (0..4)
            .map(|i| YeuCauH2 {
                target: format!("/{i}"),
                method: "GET".to_string(),
                headers: Vec::new(),
            })
            .collect();
        let kq = phien.gui_nhieu(&ds, 3_000);
        assert_eq!(kq.len(), 4);
        for (i, k) in kq.into_iter().enumerate() {
            assert_eq!(
                k.expect("stream").body,
                format!("/{i}").as_bytes(),
                "stream {i}"
            );
        }
        assert_eq!(dem.load(Ordering::SeqCst), 4, "server phải nhận 4 stream");
        drop(phien);
    }
}
