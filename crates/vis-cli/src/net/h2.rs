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

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use vis_interp::builtin::KetQuaHttp2;

use super::h2_core::H2Core;
use super::tls::{self, CauHinh, KenhTls};

/// Kết quả bước mở phiên: hoặc phiên h2 dùng lại được, hoặc trả lại kết nối TLS cho HTTP/1.1.
pub(super) enum MoH2 {
    Phien(PhienH2),
    Http1(Box<KenhTls>),
}

/// Idle tối đa của một phiên h2 trước khi Net đóng (VS3-4b4).
pub(crate) const H2_IDLE: Duration = Duration::from_secs(15);

/// Request cho prototype multiplex test-only.
#[cfg(test)]
pub(super) struct YeuCauH2 {
    pub target: String,
    pub method: String,
    pub headers: Vec<(String, String)>,
}

/// Một connection h2 đã handshake, dùng cho nhiều request **tuần tự** cùng authority.
pub(super) struct PhienH2 {
    core: H2Core<DongBo>,
    han: Arc<Mutex<Instant>>,
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
            core: H2Core::moi(send, conn, dia_chi.to_string()),
            han,
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
        let id = self.core.gui(method, target, headers)?;
        loop {
            // `ket_qua` lấy một lần; kiểm trước khi poll để trả đúng kết quả stream.
            if let Some(kq) = self.core.ket_qua(id) {
                self.luc_dung = Instant::now();
                return kq;
            }
            if self.core.da_dong() {
                return Err("kết nối HTTP/2 đã đóng khi còn stream chờ".to_string());
            }
            let tien = self.core.buoc()?;
            if Instant::now() >= han {
                return Err("quá thời gian HTTP/2".to_string());
            }
            if !tien {
                std::thread::yield_now();
            }
        }
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
        let ids: Vec<Result<u64, String>> = ds
            .iter()
            .map(|y| self.core.gui(&y.method, &y.target, &y.headers))
            .collect();
        while self.core.con_cho() && !self.core.da_dong() && Instant::now() < han {
            if self.core.buoc().is_err() {
                break;
            }
            std::thread::yield_now();
        }
        self.luc_dung = Instant::now();
        ids.into_iter()
            .map(|r| match r {
                Ok(id) => self
                    .core
                    .ket_qua(id)
                    .unwrap_or_else(|| Err("HTTP/2 không hoàn tất".to_string())),
                Err(e) => Err(e),
            })
            .collect()
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
    use bytes::Bytes;
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
