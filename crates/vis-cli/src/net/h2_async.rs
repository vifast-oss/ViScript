// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-W2 SPIKE (2026-11-21)** — HTTP/2 **reactor-native** trên Unix.
//!
//! Feature `h2-async-spike` (**default-off**); không đổi API sync/`Net`/`PhienH2`.
//!
//! Mục tiêu spike: chứng minh có thể giữ **một** connection h2 và lái state machine của
//! `h2::client` bằng readiness của reactor, không executor tự quay (`yield_now`), không thread
//! bridge. Framing/HPACK/flow-control vẫn là crate `h2` (không copy), TLS vẫn là rustls + cấu
//! hình `cau_hinh_h2` của `net/tls.rs`.
//!
//! - [`H2Async`] là phiên nhiều stream: [`H2Async::gui`] đưa request vào **trước**, [`H2Async::tai`]
//!   lái tới khi mọi stream xong. Mỗi vòng `tai` chỉ `poll` future rồi chờ fd bằng `mio`; adapter
//!   [`IoH2`] trả `Pending` khi socket `WouldBlock` và ghi lại chiều cần chờ.
//! - [`h2_get_async`] là API tối thiểu (`GET`, một request) dựng trên phiên đó.
//! - Giới hạn: header 64 KiB, body 8 MiB; vượt cap ⇒ `RST_STREAM(CANCEL)` + lỗi rõ.
//! - Timeout: một hạn monotonic cho cả connect/handshake/response; hết hạn ⇒ RST các stream còn
//!   mở + `Err` chứa "quá thời gian", phiên bị bỏ (không tái dùng).
//!
//! **Chưa nối vào `FrameReactorBackend`/`async_net.rs`**: đây là spike chứng minh phần lõi
//! (session poll-driven). Xem `agent/JOURNAL/vs3-w2-async-h2-spike.md` cho gap analysis.

#![cfg(all(feature = "h2-async-spike", feature = "http2", feature = "tls", unix))]

use std::cell::Cell;
use std::collections::HashMap;
use std::future::Future;
use std::io::{self, ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::os::fd::{AsRawFd, RawFd};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll as TaskPoll, Waker};
use std::time::{Duration, Instant};

use bytes::Bytes;
use http::Request;
use mio::unix::SourceFd;
use mio::{Events, Interest, Poll, Token};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use vis_interp::builtin::KetQuaHttp2;
use vis_interp::net::MAX_HTTP;

use super::h2::{loi_h2, MAX_HEADER};
use super::tls::{self, CauHinh, KenhTls};

/// Bit trong [`IoH2::co`]: lần `poll` gần nhất đã gặp `WouldBlock` ở chiều đọc/ghi.
const MUON_DOC: u8 = 1;
const MUON_GHI: u8 = 2;

/// Một stream h2 đang chờ; tách khỏi connection để nhiều stream sống song song.
struct Song {
    /// Giữ `SendStream` để còn `RST_STREAM` khi timeout/cap/lỗi.
    gui: Option<::h2::SendStream<Bytes>>,
    /// Chờ header response; `None` sau khi đã nhận header.
    resp: Option<Pin<Box<::h2::client::ResponseFuture>>>,
    recv: Option<::h2::RecvStream>,
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    xong_data: bool,
    ket_qua: Option<Result<KetQuaHttp2, String>>,
}

impl Song {
    fn moi(resp: ::h2::client::ResponseFuture, gui: ::h2::SendStream<Bytes>) -> Self {
        Self {
            gui: Some(gui),
            resp: Some(Box::pin(resp)),
            recv: None,
            status: 0,
            headers: Vec::new(),
            body: Vec::new(),
            xong_data: false,
            ket_qua: None,
        }
    }

    fn dat_loi(&mut self, loi: String) {
        if let Some(s) = self.gui.as_mut() {
            s.send_reset(::h2::Reason::CANCEL);
        }
        self.ket_qua = Some(Err(loi));
        self.gui = None;
        self.resp = None;
        self.recv = None;
    }

    fn hoan_tat(&mut self) {
        self.ket_qua = Some(Ok(KetQuaHttp2 {
            status: self.status,
            headers: std::mem::take(&mut self.headers),
            body: std::mem::take(&mut self.body),
        }));
        self.gui = None;
        self.resp = None;
        self.recv = None;
    }

    /// Một lượt poll stream; lỗi **của stream** được ghi vào `ket_qua`, không phá session.
    fn buoc(&mut self, cx: &mut Context<'_>, tien: &mut bool) {
        if self.ket_qua.is_some() {
            return;
        }
        if self.recv.is_none() {
            let Some(resp) = self.resp.as_mut() else {
                return;
            };
            match resp.as_mut().poll(cx) {
                TaskPoll::Ready(Ok(r)) => {
                    *tien = true;
                    let (parts, b) = r.into_parts();
                    let tong: usize = parts
                        .headers
                        .iter()
                        .map(|(k, v)| k.as_str().len() + v.as_bytes().len())
                        .sum();
                    if tong > MAX_HEADER {
                        self.dat_loi(format!("header HTTP/2 quá lớn (> {MAX_HEADER} byte)"));
                        return;
                    }
                    self.status = parts.status.as_u16();
                    self.headers = parts
                        .headers
                        .iter()
                        .map(|(k, v)| {
                            (
                                k.as_str().to_string(),
                                String::from_utf8_lossy(v.as_bytes()).into_owned(),
                            )
                        })
                        .collect();
                    self.recv = Some(b);
                    self.resp = None;
                }
                TaskPoll::Ready(Err(e)) => {
                    *tien = true;
                    self.dat_loi(loi_h2(&e, "response"));
                    return;
                }
                TaskPoll::Pending => {}
            }
        }
        if self.ket_qua.is_some() {
            return;
        }
        let Some(mut r) = self.recv.take() else {
            return;
        };
        if !self.xong_data {
            match r.poll_data(cx) {
                TaskPoll::Ready(Some(Ok(b))) => {
                    *tien = true;
                    if self.body.len().saturating_add(b.len()) > MAX_HTTP {
                        self.dat_loi(format!(
                            "phản hồi HTTP/2 vượt giới hạn {MAX_HTTP} byte (8 MiB)"
                        ));
                        return;
                    }
                    let n = b.len();
                    self.body.extend_from_slice(&b);
                    drop(b);
                    // h2 không tự trả cửa sổ flow-control; phải release từng chunk.
                    if let Err(e) = r.flow_control().release_capacity(n) {
                        self.dat_loi(format!("lỗi HTTP/2 khi giải phóng flow-control: {e}"));
                        return;
                    }
                    self.recv = Some(r);
                }
                TaskPoll::Ready(Some(Err(e))) => {
                    *tien = true;
                    self.dat_loi(loi_h2(&e, "đọc body"));
                }
                TaskPoll::Ready(None) => {
                    *tien = true;
                    self.xong_data = true;
                    self.recv = Some(r);
                }
                TaskPoll::Pending => {
                    self.recv = Some(r);
                }
            }
        } else {
            match r.poll_trailers(cx) {
                TaskPoll::Ready(Ok(_)) => {
                    *tien = true;
                    self.hoan_tat();
                }
                TaskPoll::Ready(Err(e)) => {
                    *tien = true;
                    self.dat_loi(loi_h2(&e, "trailer"));
                }
                TaskPoll::Pending => {
                    self.recv = Some(r);
                }
            }
        }
    }
}

/// Adapter `AsyncRead/AsyncWrite` **reactor-native** trên `KenhTls` nonblocking.
///
/// Không tự chờ: `poll_read`/`poll_write`/`poll_flush` trả `Pending` khi `WouldBlock` và đánh dấu
/// chiều cần đăng ký với reactor trong `co`.
struct IoH2 {
    k: KenhTls,
    co: Rc<Cell<u8>>,
}

impl IoH2 {
    fn danh_dau(&self, bit: u8) {
        self.co.set(self.co.get() | bit);
    }

    /// Đẩy record TLS đang đệm ra socket; `Ok(false)` nếu `WouldBlock` (đã đánh dấu WRITE).
    fn thu_ghi(&mut self) -> io::Result<()> {
        while self.k.conn.wants_write() {
            match self.k.conn.write_tls(&mut self.k.sock) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "TLS write_tls trả 0 byte",
                    ));
                }
                Ok(_) => {}
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    self.danh_dau(MUON_GHI);
                    return Ok(());
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

impl AsyncRead for IoH2 {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> TaskPoll<io::Result<()>> {
        let this = self.get_mut();
        if buf.remaining() == 0 {
            return TaskPoll::Ready(Ok(()));
        }
        // Plaintext đã giải mã sẵn trong rustls.
        match this.k.conn.reader().read(buf.initialize_unfilled()) {
            Ok(0) => return TaskPoll::Ready(Ok(())),
            Ok(n) => {
                buf.advance(n);
                return TaskPoll::Ready(Ok(()));
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {}
            Err(e) => return TaskPoll::Ready(Err(e)),
        }
        loop {
            match this.k.conn.read_tls(&mut this.k.sock) {
                Ok(0) => return TaskPoll::Ready(Ok(())),
                Ok(_) => {
                    if let Err(e) = this.k.conn.process_new_packets() {
                        return TaskPoll::Ready(Err(io::Error::new(
                            ErrorKind::InvalidData,
                            format!("TLS record không hợp lệ: {e}"),
                        )));
                    }
                    if let Err(e) = this.thu_ghi() {
                        return TaskPoll::Ready(Err(e));
                    }
                    match this.k.conn.reader().read(buf.initialize_unfilled()) {
                        Ok(0) => return TaskPoll::Ready(Ok(())),
                        Ok(n) => {
                            buf.advance(n);
                            return TaskPoll::Ready(Ok(()));
                        }
                        Err(e) if e.kind() == ErrorKind::WouldBlock => continue,
                        Err(e) => return TaskPoll::Ready(Err(e)),
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    this.danh_dau(MUON_DOC);
                    return TaskPoll::Pending;
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return TaskPoll::Ready(Err(e)),
            }
        }
    }
}

impl AsyncWrite for IoH2 {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> TaskPoll<io::Result<usize>> {
        let this = self.get_mut();
        // Nhận vào buffer rustls; flush best-effort. `WouldBlock` chỉ đánh dấu chiều ghi,
        // KHÔNG làm mất byte đã nhận (hợp đồng `AsyncWrite`: Pending = chưa nhận byte nào).
        let n = match this.k.conn.writer().write(buf) {
            Ok(n) => n,
            Err(e) => return TaskPoll::Ready(Err(e)),
        };
        if let Err(e) = this.thu_ghi() {
            return TaskPoll::Ready(Err(e));
        }
        TaskPoll::Ready(Ok(n))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> TaskPoll<io::Result<()>> {
        let this = self.get_mut();
        if !this.k.conn.wants_write() {
            return TaskPoll::Ready(Ok(()));
        }
        this.co.set(this.co.get() & !MUON_GHI);
        match this.thu_ghi() {
            Ok(()) => {
                if this.k.conn.wants_write() {
                    this.danh_dau(MUON_GHI);
                    TaskPoll::Pending
                } else {
                    TaskPoll::Ready(Ok(()))
                }
            }
            Err(e) => TaskPoll::Ready(Err(e)),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> TaskPoll<io::Result<()>> {
        let this = self.get_mut();
        this.k.conn.send_close_notify();
        match this.thu_ghi() {
            Ok(()) if this.k.conn.wants_write() => {
                this.danh_dau(MUON_GHI);
                TaskPoll::Pending
            }
            Ok(()) => TaskPoll::Ready(Ok(())),
            Err(e) => TaskPoll::Ready(Err(e)),
        }
    }
}

fn interest_tu_co(co: u8) -> Interest {
    match (co & MUON_DOC != 0, co & MUON_GHI != 0) {
        (true, true) => Interest::READABLE | Interest::WRITABLE,
        (false, true) => Interest::WRITABLE,
        _ => Interest::READABLE,
    }
}

enum Cho {
    SanSang,
    HetHan,
}

/// Chờ **một** fd sẵn sàng; `mio::Poll` riêng, gỡ đăng ký ngay sau mỗi lần.
fn cho_san_sang(
    poll: &mut Poll,
    events: &mut Events,
    fd: RawFd,
    interest: Interest,
    han: Instant,
) -> Result<Cho, String> {
    let Some(con) = han.checked_duration_since(Instant::now()) else {
        return Ok(Cho::HetHan);
    };
    let mut src = SourceFd(&fd);
    poll.registry()
        .register(&mut src, Token(0), interest)
        .map_err(|e| format!("đăng ký readiness HTTP/2 thất bại: {e}"))?;
    let r = poll.poll(events, Some(con));
    let _ = poll.registry().deregister(&mut src);
    match r {
        Ok(()) => Ok(if events.is_empty() && Instant::now() >= han {
            Cho::HetHan
        } else {
            Cho::SanSang
        }),
        Err(e) if e.kind() == ErrorKind::Interrupted => Ok(Cho::SanSang),
        Err(e) => Err(format!("chờ readiness HTTP/2 thất bại: {e}")),
    }
}

/// Lái một future (handshake h2) bằng readiness; không executor, không thread.
fn cho_future<F: Future>(
    f: F,
    co: &Rc<Cell<u8>>,
    fd: RawFd,
    han: Instant,
) -> Result<F::Output, String> {
    let mut poll = Poll::new().map_err(|e| format!("không tạo được mio::Poll: {e}"))?;
    let mut events = Events::with_capacity(16);
    let w = Waker::noop();
    let mut cx = Context::from_waker(w);
    let mut f = Box::pin(f);
    loop {
        co.set(0);
        match f.as_mut().poll(&mut cx) {
            TaskPoll::Ready(v) => return Ok(v),
            TaskPoll::Pending => {}
        }
        if Instant::now() >= han {
            return Err("quá thời gian HTTP/2 (handshake)".to_string());
        }
        match cho_san_sang(&mut poll, &mut events, fd, interest_tu_co(co.get()), han)? {
            Cho::SanSang => {}
            Cho::HetHan => return Err("quá thời gian HTTP/2 (handshake)".to_string()),
        }
    }
}

/// Connect TCP nonblocking + TLS handshake trong **một** hạn; socket trả về nonblocking.
fn noi_tls(cfg: &CauHinh, dia_chi: &str, han: Instant) -> Result<KenhTls, String> {
    use rustls::pki_types::ServerName;
    use rustls::ClientConnection;

    let host = tls::ten_may_chu(dia_chi)?;
    let ten = ServerName::try_from(host.to_string())
        .map_err(|e| format!("tên máy chủ TLS `{host}` không hợp lệ: {e}"))?;
    let mut ds: Vec<SocketAddr> = dia_chi
        .to_socket_addrs()
        .map_err(|e| format!("địa chỉ `{dia_chi}` không phân giải được: {e}"))?
        .collect();
    ds.sort_by_key(|a| !a.is_ipv4());
    if ds.is_empty() {
        return Err(format!("địa chỉ `{dia_chi}` không có kết quả phân giải"));
    }
    let mut poll = Poll::new().map_err(|e| format!("không tạo được mio::Poll: {e}"))?;
    let mut events = Events::with_capacity(16);
    let mut loi_cuoi: Option<String> = None;
    for addr in ds {
        if han.checked_duration_since(Instant::now()).is_none() {
            break;
        }
        let sock = match mio::net::TcpStream::connect(addr) {
            Ok(s) => s,
            Err(e) => {
                loi_cuoi = Some(format!("{addr}: {e}"));
                continue;
            }
        };
        let fd = sock.as_raw_fd();
        match cho_san_sang(&mut poll, &mut events, fd, Interest::WRITABLE, han) {
            Ok(Cho::SanSang) => {}
            Ok(Cho::HetHan) => {
                loi_cuoi = Some(format!("{addr}: hết hạn chờ nối"));
                continue;
            }
            Err(e) => {
                loi_cuoi = Some(format!("{addr}: {e}"));
                continue;
            }
        }
        match sock.take_error() {
            Ok(Some(e)) => {
                loi_cuoi = Some(format!("{addr}: {e}"));
                continue;
            }
            Err(e) => {
                loi_cuoi = Some(format!("{addr}: {e}"));
                continue;
            }
            Ok(None) => {}
        }
        if let Err(e) = sock.peer_addr() {
            loi_cuoi = Some(format!("{addr}: {e}"));
            continue;
        }
        // Chuyển sang `std::net::TcpStream` (giữ nguyên nonblocking) để dùng `KenhTls` chung.
        let mut sock: TcpStream = sock.into();
        let _ = sock.set_nodelay(true);
        let mut conn = ClientConnection::new(std::sync::Arc::clone(cfg), ten.clone())
            .map_err(|e| format!("lỗi tạo phiên TLS tới `{dia_chi}`: {e}"))?;
        bap_tay(&mut conn, &mut sock, dia_chi, han, &mut poll, &mut events)?;
        if conn.alpn_protocol() != Some(&b"h2"[..]) {
            return Err(format!(
                "server `{dia_chi}` không chọn ALPN h2 (nhận {:?}); spike chỉ chạy h2",
                conn.alpn_protocol()
                    .map(|p| String::from_utf8_lossy(p).into_owned())
            ));
        }
        return Ok(rustls::StreamOwned::new(conn, sock));
    }
    Err(match loi_cuoi {
        Some(e) => format!("lỗi nối TLS tới `{dia_chi}`: {e}"),
        None => format!("quá thời gian chờ nối TLS tới `{dia_chi}`"),
    })
}

/// Handshake rustls không chặn socket: chỉ `read_tls`/`write_tls` khi reactor báo sẵn sàng.
fn bap_tay(
    conn: &mut rustls::ClientConnection,
    sock: &mut TcpStream,
    dia_chi: &str,
    han: Instant,
    poll: &mut Poll,
    events: &mut Events,
) -> Result<(), String> {
    let fd = sock.as_raw_fd();
    while conn.is_handshaking() {
        let mut tien = false;
        if conn.wants_write() {
            match conn.write_tls(sock) {
                Ok(_) => tien = true,
                Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(format!("lỗi TLS handshake tới `{dia_chi}`: {e}")),
            }
        }
        if conn.wants_read() {
            match conn.read_tls(sock) {
                Ok(0) => {
                    return Err(format!(
                        "server `{dia_chi}` đóng kết nối trong TLS handshake"
                    ))
                }
                Ok(_) => {
                    conn.process_new_packets()
                        .map_err(|e| format!("lỗi TLS handshake tới `{dia_chi}`: {e}"))?;
                    tien = true;
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(format!("lỗi TLS handshake tới `{dia_chi}`: {e}")),
            }
        }
        if !conn.is_handshaking() {
            break;
        }
        if !tien {
            let interest = if conn.wants_write() {
                Interest::WRITABLE
            } else {
                Interest::READABLE
            };
            match cho_san_sang(poll, events, fd, interest, han)? {
                Cho::SanSang => {}
                Cho::HetHan => {
                    return Err(format!(
                        "quá thời gian TLS handshake tới `{dia_chi}` (hết hạn)"
                    ))
                }
            }
        }
    }
    Ok(())
}

/// Phiên HTTP/2 nhiều stream trên **một** connection; caller tự quyết định thời điểm chờ.
pub struct H2Async {
    send: ::h2::client::SendRequest<Bytes>,
    conn: Pin<Box<::h2::client::Connection<IoH2, Bytes>>>,
    authority: String,
    songs: HashMap<u64, Song>,
    next_id: u64,
    co: Rc<Cell<u8>>,
    fd: RawFd,
    da_dong: bool,
}

impl H2Async {
    /// Mở TLS + ALPN h2 + handshake h2 trong hạn `timeout_ms`; server không chọn h2 ⇒ lỗi rõ.
    pub(crate) fn moi(cfg: &CauHinh, dia_chi: &str, timeout_ms: u64) -> Result<Self, String> {
        let han = Instant::now() + Duration::from_millis(timeout_ms.max(1));
        let k = noi_tls(cfg, dia_chi, han)?;
        let fd = k.sock.as_raw_fd();
        let co = Rc::new(Cell::new(0u8));
        let io = IoH2 {
            k,
            co: Rc::clone(&co),
        };
        let kq = cho_future(::h2::client::handshake(io), &co, fd, han)?;
        let (send, conn) = kq.map_err(|e| format!("lỗi HTTP/2 handshake tới `{dia_chi}`: {e}"))?;
        Ok(Self {
            send,
            conn: Box::pin(conn),
            authority: dia_chi.to_string(),
            songs: HashMap::new(),
            next_id: 0,
            co,
            fd,
            da_dong: false,
        })
    }

    /// Gửi request (end_stream ngay, body rỗng) và trả id nội bộ để lấy kết quả.
    pub fn gui(
        &mut self,
        method: &str,
        target: &str,
        headers: &[(String, String)],
    ) -> Result<u64, String> {
        if self.da_dong {
            return Err("phiên HTTP/2 đã đóng".to_string());
        }
        let uri = format!("https://{}{}", self.authority, target);
        let mut builder = Request::builder().method(method).uri(uri);
        for (k, v) in headers {
            builder = builder.header(k.as_str(), v.as_str());
        }
        let req = builder
            .body(())
            .map_err(|e| format!("request HTTP/2 không hợp lệ: {e}"))?;
        let (resp, gui) = self
            .send
            .send_request(req, true)
            .map_err(|e| loi_h2(&e, "gửi request"))?;
        let id = self.next_id;
        self.next_id += 1;
        self.songs.insert(id, Song::moi(resp, gui));
        Ok(id)
    }

    /// Còn stream chưa có kết quả?
    pub fn con_cho(&self) -> bool {
        self.songs.values().any(|s| s.ket_qua.is_none())
    }

    /// Poll connection + mọi stream **một lượt**; `true` = vừa có tiến triển.
    fn buoc(&mut self) -> Result<bool, String> {
        if self.da_dong {
            return Ok(false);
        }
        let mut tien = false;
        let w = Waker::noop();
        let mut cx = Context::from_waker(w);
        // Khi connection đóng/lỗi, poll nốt các stream **trước** để h2 kịp trả lỗi GOAWAY/RST
        // theo từng stream; chỉ stream còn treo mới nhận lỗi connection.
        let loi_conn = match self.conn.as_mut().poll(&mut cx) {
            TaskPoll::Ready(Ok(())) => {
                Some("kết nối HTTP/2 đã đóng khi còn stream chờ".to_string())
            }
            TaskPoll::Ready(Err(e)) => Some(loi_h2(&e, "kết nối")),
            TaskPoll::Pending => None,
        };
        if loi_conn.is_some() {
            self.da_dong = true;
            for song in self.songs.values_mut() {
                song.buoc(&mut cx, &mut tien);
            }
            if let Some(loi) = &loi_conn {
                self.loi_het(loi.clone());
            }
            return match &loi_conn {
                Some(loi) => Err(loi.clone()),
                None => Ok(tien),
            };
        }
        for song in self.songs.values_mut() {
            song.buoc(&mut cx, &mut tien);
        }
        Ok(tien)
    }

    fn loi_het(&mut self, loi: String) {
        for s in self.songs.values_mut() {
            if s.ket_qua.is_none() {
                s.dat_loi(loi.clone());
            }
        }
    }

    /// Lái mọi stream tới khi xong; hết hạn ⇒ RST các stream còn mở + `Err`.
    pub fn tai_trong_han(&mut self, han: Instant) -> Result<(), String> {
        let mut poll = Poll::new().map_err(|e| format!("không tạo được mio::Poll: {e}"))?;
        let mut events = Events::with_capacity(16);
        loop {
            self.co.set(0);
            while self.buoc()? {
                if !self.con_cho() {
                    return Ok(());
                }
                // Server có thể bơm dữ liệu liên tục; deadline vẫn phải có hiệu lực trong
                // vòng "còn tiến triển", không chỉ lúc socket WouldBlock.
                if Instant::now() >= han {
                    let loi = "quá thời gian chờ phản hồi HTTP/2".to_string();
                    self.loi_het(loi.clone());
                    self.da_dong = true;
                    return Err(loi);
                }
            }
            if !self.con_cho() {
                return Ok(());
            }
            if Instant::now() >= han {
                let loi = "quá thời gian chờ phản hồi HTTP/2".to_string();
                self.loi_het(loi.clone());
                self.da_dong = true;
                return Err(loi);
            }
            match cho_san_sang(
                &mut poll,
                &mut events,
                self.fd,
                interest_tu_co(self.co.get()),
                han,
            )? {
                Cho::SanSang => {}
                Cho::HetHan => {
                    let loi = "quá thời gian chờ phản hồi HTTP/2".to_string();
                    self.loi_het(loi.clone());
                    self.da_dong = true;
                    return Err(loi);
                }
            }
        }
    }

    /// Như [`Self::tai_trong_han`] với hạn tính từ lúc gọi.
    pub fn tai(&mut self, timeout_ms: u64) -> Result<(), String> {
        let han = Instant::now() + Duration::from_millis(timeout_ms.max(1));
        self.tai_trong_han(han)
    }

    /// Lấy kết quả của một stream (một lần).
    pub fn ket_qua(&mut self, id: u64) -> Option<Result<KetQuaHttp2, String>> {
        self.songs.get_mut(&id).and_then(|s| s.ket_qua.take())
    }
}

/// Tách URL `https://host[:port]/path`; chỉ h2 qua TLS (không h2c).
fn tach_url(url: &str) -> Result<(String, String), String> {
    let rest = url
        .strip_prefix("https://")
        .ok_or_else(|| format!("h2_get_async chỉ hỗ trợ `https://` (h2 qua TLS), nhận `{url}`"))?;
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    if authority.is_empty() {
        return Err(format!("URL `{url}` thiếu host"));
    }
    let co_port = if authority.starts_with('[') {
        authority.contains("]:")
    } else {
        authority.contains(':')
    };
    let authority = if co_port {
        authority.to_string()
    } else {
        format!("{authority}:443")
    };
    Ok((authority, path.to_string()))
}

fn get_voi_cau_hinh(url: &str, timeout_ms: u64, cfg: &CauHinh) -> Result<KetQuaHttp2, String> {
    let (authority, target) = tach_url(url)?;
    let han = Instant::now() + Duration::from_millis(timeout_ms.max(1));
    let mut phien = H2Async::moi(cfg, &authority, timeout_ms)?;
    let id = phien.gui("GET", &target, &[])?;
    phien.tai_trong_han(han)?;
    phien
        .ket_qua(id)
        .unwrap_or_else(|| Err("HTTP/2 không trả kết quả cho stream".to_string()))
}

/// **API spike tối thiểu** — GET async qua HTTP/2 reactor-native; trả body/status/header hoặc lỗi.
pub fn h2_get_async(url: &str, timeout_ms: u64) -> Result<KetQuaHttp2, String> {
    let cfg = tls::cau_hinh_h2(&tls::mac_dinh()?);
    get_voi_cau_hinh(url, timeout_ms, &cfg)
}

/// **CHỈ TEST/EMBEDDING** — như [`h2_get_async`] nhưng tin đúng các DER gốc cho trước.
#[doc(hidden)]
pub fn h2_get_async_voi_goc(
    url: &str,
    timeout_ms: u64,
    goc_der: &[&[u8]],
) -> Result<KetQuaHttp2, String> {
    let cfg = tls::cau_hinh_h2(&tls::tu_goc_der(goc_der)?);
    get_voi_cau_hinh(url, timeout_ms, &cfg)
}

/// **CHỈ TEST/EMBEDDING** — mở phiên với gốc DER cho trước (test multiplex/GOAWAY).
#[doc(hidden)]
pub fn phien_voi_goc(dia_chi: &str, timeout_ms: u64, goc_der: &[&[u8]]) -> Result<H2Async, String> {
    let cfg = tls::cau_hinh_h2(&tls::tu_goc_der(goc_der)?);
    H2Async::moi(&cfg, dia_chi, timeout_ms)
}
