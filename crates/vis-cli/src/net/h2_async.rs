// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-W2 (VD-271)** — HTTP/2 **reactor-native** cho đường async, feature `async-h2`
//! (**default-off**).
//!
//! Session dùng chung protocol với client sync qua [`super::h2_core::H2Core`]; module này chỉ
//! thêm phần **I/O không chặn**: adapter [`IoH2`] trên rustls nonblocking, handshake nhiều bước,
//! và vòng chờ `mio` khi driver cần. Không executor tự quay, không thread bridge.
//!
//! - [`H2Async::moi_cho_bap`] nhận `KenhTls` đã handshake TLS (ALPN h2) và để h2 handshake ở
//!   trạng thái chờ; `tien()` lái một bước, trả `false` khi cần readiness.
//! - [`h2_get_async`] là API tối thiểu (`GET`) cho test/embedding. Đường production dùng
//!   [`H2Async`] qua `AsyncNet` (`h2_ready`/`h2_send`/`h2_poll`).
//! - Feature này **không** đổi API sync/`Net`; sync vẫn dùng `net/h2.rs` trên cùng `H2Core`.

#![cfg(all(feature = "async-h2", feature = "http2", feature = "tls", unix))]

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
use mio::unix::SourceFd;
use mio::{Events, Interest, Poll, Token};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use vis_interp::builtin::KetQuaHttp2;

use super::h2_core::H2Core;
use super::tls::{self, CauHinh, KenhTls};

/// Bit trong [`IoH2::co`]: lần `poll` gần nhất đã gặp `WouldBlock` ở chiều đọc/ghi.
const MUON_DOC: u8 = 1;
const MUON_GHI: u8 = 2;

/// Adapter `AsyncRead/AsyncWrite` **reactor-native** trên `KenhTls` nonblocking.
///
/// Không tự chờ: `poll_read`/`poll_write`/`poll_flush` trả `Pending` khi `WouldBlock` và đánh dấu
/// chiều cần đăng ký với reactor trong `co`.
pub(super) struct IoH2 {
    k: KenhTls,
    co: Rc<Cell<u8>>,
}

impl IoH2 {
    fn danh_dau(&self, bit: u8) {
        self.co.set(self.co.get() | bit);
    }

    /// Đẩy record TLS đang đệm ra socket; `WouldBlock` chỉ đánh dấu WRITE, không mất byte.
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
pub(super) fn bap_tay(
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

type BapH2 = Pin<
    Box<
        dyn Future<
            Output = Result<
                (
                    ::h2::client::SendRequest<Bytes>,
                    ::h2::client::Connection<IoH2, Bytes>,
                ),
                ::h2::Error,
            >,
        >,
    >,
>;

/// Request xếp trước handshake: id backend, method, target, headers.
type YeuCauCho = (u64, String, String, Vec<(String, String)>);

enum TrangThai {
    /// Đã có TLS, đang chờ SETTINGS/frame handshake h2.
    DangBap(BapH2),
    Song(H2Core<IoH2>),
    Dong,
}

/// Phiên HTTP/2 nhiều stream trên **một** connection; `tien()` không bao giờ block.
pub struct H2Async {
    trang_thai: TrangThai,
    authority: String,
    co: Rc<Cell<u8>>,
    fd: RawFd,
    /// Id request do backend cấp (ổn định kể cả khi còn chờ handshake).
    next_req: u64,
    /// Request xếp trước khi handshake xong: (id backend, method, target, headers).
    cho_gui: Vec<YeuCauCho>,
    /// id backend → id stream trong `H2Core`.
    map_id: HashMap<u64, u64>,
    /// Lỗi gửi của request đã xếp (id backend).
    loi_gui: HashMap<u64, String>,
}

impl H2Async {
    /// Bọc `KenhTls` đã handshake TLS (ALPN h2) và bắt đầu h2 handshake **chưa chạy**.
    pub(crate) fn moi_cho_bap(k: KenhTls, authority: &str) -> Self {
        let fd = k.sock.as_raw_fd();
        let co = Rc::new(Cell::new(0u8));
        let io = IoH2 {
            k,
            co: Rc::clone(&co),
        };
        Self {
            trang_thai: TrangThai::DangBap(Box::pin(::h2::client::handshake(io))),
            authority: authority.to_string(),
            co,
            fd,
            next_req: 0,
            cho_gui: Vec::new(),
            map_id: HashMap::new(),
            loi_gui: HashMap::new(),
        }
    }

    /// Mở TLS + ALPN h2 + handshake h2 trong hạn `timeout_ms` (test/embedding).
    pub(crate) fn moi(cfg: &CauHinh, dia_chi: &str, timeout_ms: u64) -> Result<Self, String> {
        let han = Instant::now() + Duration::from_millis(timeout_ms.max(1));
        let k = noi_tls(cfg, dia_chi, han)?;
        let mut p = Self::moi_cho_bap(k, dia_chi);
        p.tai_handshake(han)?;
        Ok(p)
    }

    #[allow(dead_code)] // checkpoint 2 (AsyncNet) dùng
    pub(crate) fn fd(&self) -> RawFd {
        self.fd
    }

    /// **VS3-W4** — `Nguon` của async_net: đăng ký readiness cho socket h2.
    ///
    /// `H2Async` giữ `KenhTls` (socket) **bên trong** connection h2 nên không thể mượn
    /// `&mut MioStream`; trên Unix dùng fd ổn định đã lưu (`SourceFd`) — đây là module Unix-only,
    /// Windows dùng đường `Nguon` trực tiếp cho stream/TLS (async-h2 default-off, chưa port).
    pub(crate) fn dang_ky_nguon(
        &mut self,
        poll: &Poll,
        token: Token,
        interest: Interest,
    ) -> io::Result<()> {
        let fd = self.fd;
        let mut src = SourceFd(&fd);
        poll.registry().register(&mut src, token, interest)
    }

    pub(crate) fn dang_ky_lai_nguon(
        &mut self,
        poll: &Poll,
        token: Token,
        interest: Interest,
    ) -> io::Result<()> {
        let fd = self.fd;
        let mut src = SourceFd(&fd);
        poll.registry().reregister(&mut src, token, interest)
    }

    pub(crate) fn huy_nguon(&mut self, poll: &Poll) -> io::Result<()> {
        let fd = self.fd;
        let mut src = SourceFd(&fd);
        poll.registry().deregister(&mut src)
    }

    fn trang_thai_da_san_sang(&self) -> bool {
        matches!(self.trang_thai, TrangThai::Song(_))
    }

    pub(crate) fn san_sang(&self) -> bool {
        self.trang_thai_da_san_sang()
    }

    #[allow(dead_code)] // checkpoint 2 (AsyncNet) dùng
    pub(crate) fn interest(&self) -> Interest {
        interest_tu_co(self.co.get())
    }

    #[allow(dead_code)] // checkpoint 2 (AsyncNet) dùng
    pub(crate) fn xoa_co(&self) {
        self.co.set(0);
    }

    /// Lái **một** bước: handshake h2, hoặc connection + mọi stream.
    ///
    /// `Ok(true)` = có tiến triển (gọi lại ngay); `Ok(false)` = cần chờ fd sẵn sàng.
    pub(crate) fn tien(&mut self) -> Result<bool, String> {
        let mut tt = std::mem::replace(&mut self.trang_thai, TrangThai::Dong);
        let kq = match &mut tt {
            TrangThai::DangBap(f) => {
                let w = Waker::noop();
                let mut cx = Context::from_waker(w);
                match f.as_mut().poll(&mut cx) {
                    TaskPoll::Ready(Ok((send, conn))) => {
                        let mut core = H2Core::moi(send, conn, self.authority.clone());
                        for (id, method, target, headers) in self.cho_gui.drain(..) {
                            match core.gui(&method, &target, &headers) {
                                Ok(core_id) => {
                                    self.map_id.insert(id, core_id);
                                }
                                Err(e) => {
                                    self.loi_gui.insert(id, e);
                                }
                            }
                        }
                        tt = TrangThai::Song(core);
                        Ok(true)
                    }
                    TaskPoll::Ready(Err(e)) => Err(format!(
                        "lỗi HTTP/2 handshake tới `{}`: {e}",
                        self.authority
                    )),
                    TaskPoll::Pending => Ok(false),
                }
            }
            TrangThai::Song(core) => core.buoc(),
            TrangThai::Dong => Ok(false),
        };
        self.trang_thai = if kq.is_err() { TrangThai::Dong } else { tt };
        kq
    }

    /// Đưa request vào phiên (end_stream ngay) hoặc xếp chờ handshake; trả id backend.
    pub fn gui(
        &mut self,
        method: &str,
        target: &str,
        headers: &[(String, String)],
    ) -> Result<u64, String> {
        let id = self.next_req;
        self.next_req += 1;
        match &mut self.trang_thai {
            TrangThai::Song(core) => {
                let core_id = core.gui(method, target, headers)?;
                self.map_id.insert(id, core_id);
                Ok(id)
            }
            TrangThai::DangBap(_) => {
                self.cho_gui
                    .push((id, method.to_string(), target.to_string(), headers.to_vec()));
                Ok(id)
            }
            TrangThai::Dong => Err("GOAWAY HTTP/2 khi gửi request: phiên đã đóng".to_string()),
        }
    }

    pub fn ket_qua(&mut self, id: u64) -> Option<Result<KetQuaHttp2, String>> {
        if let Some(e) = self.loi_gui.remove(&id) {
            return Some(Err(e));
        }
        let core_id = *self.map_id.get(&id)?;
        match &mut self.trang_thai {
            TrangThai::Song(core) => core.ket_qua(core_id),
            _ => None,
        }
    }

    /// Còn việc: handshake chưa xong hoặc còn stream chờ.
    pub(crate) fn con_viec(&self) -> bool {
        match &self.trang_thai {
            TrangThai::DangBap(_) => true,
            TrangThai::Song(core) => core.con_cho(),
            TrangThai::Dong => false,
        }
    }

    pub(crate) fn huy_stream(&mut self, id: u64) {
        if let Some(core_id) = self.map_id.remove(&id) {
            if let TrangThai::Song(core) = &mut self.trang_thai {
                core.huy_stream(core_id);
            }
        } else {
            self.cho_gui.retain(|(i, _, _, _)| *i != id);
            self.loi_gui.remove(&id);
        }
    }

    pub(crate) fn huy(&mut self, loi: String) {
        if let TrangThai::Song(core) = &mut self.trang_thai {
            core.huy(loi);
        }
        self.trang_thai = TrangThai::Dong;
    }

    fn tai_handshake(&mut self, han: Instant) -> Result<(), String> {
        let mut poll = Poll::new().map_err(|e| format!("không tạo được mio::Poll: {e}"))?;
        let mut events = Events::with_capacity(16);
        loop {
            self.co.set(0);
            while self.tien()? {
                if self.san_sang() {
                    return Ok(());
                }
                if Instant::now() >= han {
                    return Err("quá thời gian HTTP/2 (handshake)".to_string());
                }
            }
            if self.san_sang() {
                return Ok(());
            }
            if Instant::now() >= han {
                return Err("quá thời gian HTTP/2 (handshake)".to_string());
            }
            match cho_san_sang(
                &mut poll,
                &mut events,
                self.fd,
                interest_tu_co(self.co.get()),
                han,
            )? {
                Cho::SanSang => {}
                Cho::HetHan => return Err("quá thời gian HTTP/2 (handshake)".to_string()),
            }
        }
    }

    /// Lái mọi stream tới khi xong; hết hạn ⇒ RST các stream còn mở + `Err`.
    pub fn tai_trong_han(&mut self, han: Instant) -> Result<(), String> {
        let mut poll = Poll::new().map_err(|e| format!("không tạo được mio::Poll: {e}"))?;
        let mut events = Events::with_capacity(16);
        loop {
            self.co.set(0);
            while self.tien()? {
                if !self.con_viec() {
                    return Ok(());
                }
                // Server có thể bơm dữ liệu liên tục; deadline vẫn phải có hiệu lực trong
                // vòng "còn tiến triển", không chỉ lúc socket WouldBlock.
                if Instant::now() >= han {
                    let loi = "quá thời gian chờ phản hồi HTTP/2".to_string();
                    self.huy(loi.clone());
                    return Err(loi);
                }
            }
            if !self.con_viec() {
                return Ok(());
            }
            if Instant::now() >= han {
                let loi = "quá thời gian chờ phản hồi HTTP/2".to_string();
                self.huy(loi.clone());
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
                    self.huy(loi.clone());
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
