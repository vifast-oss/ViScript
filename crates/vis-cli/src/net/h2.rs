// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-4b2 (VD-271)** — client HTTP/2 tối thiểu cho `https://` (feature `http2`, default-off).
//!
//! - TLS handshake dùng ALPN `["h2","http/1.1"]`; nếu server không chọn `h2` thì trả lại **cùng**
//!   `KenhTls` để tầng HTTP/1.1 dùng tiếp (không mở thêm kết nối/không đốt một slot server test).
//! - `h2` cần `AsyncRead/AsyncWrite` + executor; module này bọc `KenhTls` **đồng bộ** bằng adapter
//!   chặn theo timeout và một executor một luồng tối thiểu (`poll` + `yield`). Không bật tokio
//!   runtime/network feature, không thêm async vào đường HTTP/1.1.
//! - Một request/kết nối; không multiplex/pool h2 (đó là phần mở rộng sau, không làm hồi quy h1).

use std::future::Future;
use std::io::{self, Read, Write};
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

use bytes::Bytes;
use http::Request;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use vis_interp::builtin::KetQuaHttp2;
use vis_interp::net::MAX_HTTP;

use super::tls::{self, CauHinh, KenhTls};

/// Kết quả bước “thử h2”: hoặc đã có response h2, hoặc trả lại kết nối TLS để chạy HTTP/1.1.
pub(super) enum KqH2 {
    Xong(KetQuaHttp2),
    Http1(Box<KenhTls>),
}

/// Thử HTTP/2 trên `dia_chi` (authority TLS). `Ok(Http1(k))` = ALPN không chọn h2; caller dùng `k`.
pub(super) fn request(
    cfg: &CauHinh,
    dia_chi: &str,
    target: &str,
    method: &str,
    headers: &[(String, String)],
    timeout_ms: u64,
) -> Result<KqH2, String> {
    let con = timeout_ms.max(1);
    let han = Instant::now() + Duration::from_millis(con);
    let k = tls::noi(cfg, dia_chi, con)?;
    if k.conn.alpn_protocol() != Some(&b"h2"[..]) {
        return Ok(KqH2::Http1(Box::new(k)));
    }
    // h2 cần `poll` không chặn: socket nonblocking + adapter trả `Pending`; executor một luồng tự
    // chạy lại. Nếu fallback h1 ở trên, socket vẫn blocking cho tầng HTTP/1.1.
    k.sock
        .set_nonblocking(true)
        .map_err(|e| format!("không đặt non-blocking cho HTTP/2 `{dia_chi}`: {e}"))?;

    let io = DongBo { k, han };
    let (mut send, conn) = cho(::h2::client::handshake(io))
        .map_err(|e| format!("lỗi HTTP/2 handshake tới `{dia_chi}`: {e}"))?;

    let uri = format!("https://{dia_chi}{target}");
    let mut builder = Request::builder().method(method).uri(uri);
    for (k, v) in headers {
        builder = builder.header(k.as_str(), v.as_str());
    }
    let req = builder
        .body(())
        .map_err(|e| format!("request HTTP/2 không hợp lệ tới `{dia_chi}`: {e}"))?;
    let (resp, _body) = send
        .send_request(req, true)
        .map_err(|e| format!("lỗi gửi HTTP/2 tới `{dia_chi}`: {e}"))?;

    let kq = doc_phan_hoi(conn, resp, method, dia_chi)?;
    Ok(KqH2::Xong(kq))
}

fn doc_phan_hoi(
    conn: ::h2::client::Connection<DongBo, Bytes>,
    resp: ::h2::client::ResponseFuture,
    method: &str,
    dia_chi: &str,
) -> Result<KetQuaHttp2, String> {
    let w = Waker::noop();
    let mut cx = Context::from_waker(w);
    let mut conn = Box::pin(conn);
    let mut resp = Box::pin(resp);
    let mut recv: Option<::h2::RecvStream> = None;
    let mut status = 0u16;
    let mut headers: Vec<(String, String)> = Vec::new();
    let mut body: Vec<u8> = Vec::new();

    loop {
        let mut tien = false;
        match conn.as_mut().poll(&mut cx) {
            Poll::Ready(Ok(())) => {}
            Poll::Ready(Err(e)) => {
                return Err(format!("lỗi kết nối HTTP/2 tới `{dia_chi}`: {e}"));
            }
            Poll::Pending => {}
        }
        if recv.is_none() {
            match resp.as_mut().poll(&mut cx) {
                Poll::Ready(Ok(r)) => {
                    let (parts, b) = r.into_parts();
                    status = parts.status.as_u16();
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
                    if method.eq_ignore_ascii_case("HEAD") {
                        return Ok(KetQuaHttp2 {
                            status,
                            headers,
                            body,
                        });
                    }
                    recv = Some(b);
                    tien = true;
                }
                Poll::Ready(Err(e)) => {
                    return Err(format!("lỗi response HTTP/2 từ `{dia_chi}`: {e}"));
                }
                Poll::Pending => {}
            }
        }
        if let Some(r) = recv.as_mut() {
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
                Poll::Ready(Some(Err(e))) => {
                    return Err(format!("lỗi đọc body HTTP/2 từ `{dia_chi}`: {e}"));
                }
                Poll::Ready(None) => {
                    return Ok(KetQuaHttp2 {
                        status,
                        headers,
                        body,
                    });
                }
                Poll::Pending => {}
            }
        }
        if !tien {
            std::thread::yield_now();
        }
    }
}

/// Adapter `AsyncRead/AsyncWrite` trên [`KenhTls`] đồng bộ; mỗi lần chờ dùng phần thời gian còn lại.
struct DongBo {
    k: KenhTls,
    han: Instant,
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
        if this.han <= Instant::now() {
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
        if this.han <= Instant::now() {
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
                if this.han <= Instant::now() {
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

/// Executor một luồng: IO adapter không bao giờ trả `Pending`, nên chỉ cần poll + yield khi
/// `h2` chưa sẵn sàng (không timer/runtime).
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
