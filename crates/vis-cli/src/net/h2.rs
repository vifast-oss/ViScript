// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-4b2/4b3 (VD-271)** — client HTTP/2 cho `https://` (feature `http2`, default-off).
//!
//! - TLS handshake dùng ALPN `["h2","http/1.1"]`; nếu server không chọn `h2` thì trả lại **cùng**
//!   `KenhTls` để tầng HTTP/1.1 dùng tiếp (không mở thêm kết nối/không đốt một slot server test).
//! - **4b3**: giữ một `PhienH2` (một connection + `SendRequest`) cho mỗi authority; nhiều request
//!   **tuần tự** dùng lại connection, không multiplex đồng thời. Net giới hạn số phiên và đóng khi
//!   hết hạn/lỗi.
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
