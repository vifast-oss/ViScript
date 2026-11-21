// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-1d/W1f** — host + reactor production cho chương trình **async** trên frame scheduler.
//!
//! Đây là bản production hoá adapter `Net2` của test `lat256_net`: cùng `mio` (epoll) + socket
//! nonblocking, nhưng nằm trong binary `vis`. Chỉ dùng cho nhánh **async**; runtime sync vẫn dùng
//! [`crate::net::Net`] nguyên vẹn. Linux/Unix trước; Windows/wasm không có backend này (đường cũ).
//!
//! Handle giữ đúng quy ước của `stdlib`: `C<n>` kết nối, `L<n>` listener, `N:<addr>` connect,
//! `W:<tay>` write. `FrameReactorBackend` chỉ đăng ký fd; deadline/token/wake do `rt` quản.
//!
//! **VS3-1d/W1f-TLS**: `connect_tls` dùng rustls nonblocking (TCP + handshake trong **một** hạn
//! monotonic), dùng chung cấu hình gốc/chứng chỉ với runtime sync (`net/tls.rs`). Kênh TLS là
//! handle `C<n>`; `read`/`write`/`close` và reactor đều hiểu nó, không fallback socket blocking.
//! `http.get/head` của nhánh async đi qua host này nên `https://` không còn bị từ chối.
#![cfg(unix)]

use mio::net::{TcpListener as MioListener, TcpStream as MioStream};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::os::fd::AsRawFd;
#[cfg(feature = "tls")]
use std::os::fd::RawFd;
use std::rc::Rc;
#[cfg(feature = "tls")]
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(feature = "tls")]
use rustls::pki_types::ServerName;
#[cfg(feature = "tls")]
use rustls::{ClientConfig, ClientConnection};

use mio::unix::SourceFd;
use mio::{Events, Interest, Poll, Token};

use vis_interp::builtin::NetHost;
use vis_interp::rt::{FrameReactorBackend, ReactorIoOutcome, ReactorOp, ReactorRawEvent};
use vis_interp::stdlib::{FRAME_NET_CONNECT, FRAME_NET_WRITE, NET_READ_CAP};

struct WriteState {
    tay: String,
    data: Vec<u8>,
    off: usize,
}

/// **VS3-1d/W1f-TLS** — một kênh TLS đã handshake; socket luôn nonblocking.
#[cfg(feature = "tls")]
struct TlsChan {
    conn: ClientConnection,
    sock: MioStream,
}

/// Host mạng production cho đường async + backend reactor của nó (cùng một đối tượng).
pub struct AsyncNet {
    streams: HashMap<String, MioStream>,
    listeners: HashMap<String, MioListener>,
    #[cfg(feature = "tls")]
    tls: HashMap<String, TlsChan>,
    #[cfg(feature = "tls")]
    cau_hinh_tls: Option<Result<Arc<ClientConfig>, String>>,
    poll: Poll,
    events: Events,
    /// Poll riêng cho các lời gọi host đồng bộ (`http.get/head`) trên kênh TLS; không dùng chung
    /// token với reactor để không đụng đăng ký của scheduler.
    #[cfg(feature = "tls")]
    host_poll: Poll,
    #[cfg(feature = "tls")]
    host_events: Events,
    tokens: HashMap<u64, String>,
    /// `N:<addr>` đã khởi tạo connect, chờ WRITABLE/kiểm SO_ERROR.
    connects: HashSet<String>,
    writes: HashMap<String, WriteState>,
    next_conn: u32,
    next_listen: u32,
    giai_thich: bool,
}

impl AsyncNet {
    pub fn new(giai_thich: bool) -> Self {
        Self {
            streams: HashMap::new(),
            listeners: HashMap::new(),
            #[cfg(feature = "tls")]
            tls: HashMap::new(),
            #[cfg(feature = "tls")]
            cau_hinh_tls: None,
            poll: Poll::new().expect("mio::Poll::new"),
            events: Events::with_capacity(128),
            #[cfg(feature = "tls")]
            host_poll: Poll::new().expect("mio::Poll::new (host TLS)"),
            #[cfg(feature = "tls")]
            host_events: Events::with_capacity(16),
            tokens: HashMap::new(),
            connects: HashSet::new(),
            writes: HashMap::new(),
            next_conn: 0,
            next_listen: 0,
            giai_thich,
        }
    }

    fn nhat_ky(&self, thao_tac: &str, p: &str) {
        if self.giai_thich {
            eprintln!("NET {thao_tac} {p}");
        }
    }

    fn target_of(h: &str) -> &str {
        if let Some(x) = h.strip_prefix(FRAME_NET_WRITE) {
            x
        } else {
            h
        }
    }

    fn dat_c(&mut self, s: MioStream) -> String {
        self.next_conn += 1;
        let h = format!("C{}", self.next_conn);
        self.streams.insert(h.clone(), s);
        h
    }

    fn dat_l(&mut self, l: MioListener) -> String {
        self.next_listen += 1;
        let h = format!("L{}", self.next_listen);
        self.listeners.insert(h.clone(), l);
        h
    }

    #[cfg(feature = "tls")]
    fn dat_tls(&mut self, t: TlsChan) -> String {
        self.next_conn += 1;
        let h = format!("C{}", self.next_conn);
        self.tls.insert(h.clone(), t);
        h
    }

    fn try_connect(&mut self, key: &str) -> ReactorIoOutcome {
        let addr_text = &key[FRAME_NET_CONNECT.len()..];
        let addr = match loopback_addr(addr_text) {
            Ok(a) => a,
            Err(e) => return ReactorIoOutcome::Err(e),
        };
        if !self.connects.contains(key) {
            let s = match mio::net::TcpStream::connect(addr) {
                Ok(s) => s,
                Err(e) => {
                    return ReactorIoOutcome::Err(format!("nối tới `{addr_text}` thất bại: {e}"))
                }
            };
            self.streams.insert(key.to_string(), s);
            self.connects.insert(key.to_string());
            return ReactorIoOutcome::WouldBlock;
        }
        let Some(s) = self.streams.get(key) else {
            self.connects.remove(key);
            return ReactorIoOutcome::Err(format!("tay nối `{addr_text}` đã đóng"));
        };
        if let Err(e) = s.take_error() {
            self.streams.remove(key);
            self.connects.remove(key);
            return ReactorIoOutcome::Err(format!("lỗi kiểm tra nối `{addr_text}`: {e}"));
        }
        // take_error() chỉ báo lỗi; phải kiểm peer_addr để chắc đã nối xong.
        match s.peer_addr() {
            Ok(_) => {
                let Some(s) = self.streams.remove(key) else {
                    return ReactorIoOutcome::Err(format!("tay nối `{addr_text}` biến mất"));
                };
                self.connects.remove(key);
                let h = self.dat_c(s);
                ReactorIoOutcome::Accepted(h)
            }
            Err(e)
                if e.kind() == ErrorKind::WouldBlock
                    || e.kind() == ErrorKind::NotConnected
                    || e.raw_os_error() == Some(115) =>
            {
                ReactorIoOutcome::WouldBlock
            }
            Err(e) => {
                self.streams.remove(key);
                self.connects.remove(key);
                ReactorIoOutcome::Err(format!("nối tới `{addr_text}` thất bại: {e}"))
            }
        }
    }

    fn try_write(&mut self, key: &str) -> ReactorIoOutcome {
        #[cfg(feature = "tls")]
        if self.tls.contains_key(Self::target_of(key)) {
            return self.try_write_tls(key);
        }
        if !self.writes.contains_key(key) {
            let Some(data) = vis_interp::stdlib::frame_net_take_write(key) else {
                return ReactorIoOutcome::Err("buffer write không có (tay đã đóng?)".into());
            };
            self.writes.insert(
                key.to_string(),
                WriteState {
                    tay: Self::target_of(key).to_string(),
                    data,
                    off: 0,
                },
            );
        }
        let (tay, len, off) = {
            let w = self.writes.get(key).expect("write vừa tạo");
            (w.tay.clone(), w.data.len(), w.off)
        };
        let Some(s) = self.streams.get_mut(&tay) else {
            self.writes.remove(key);
            return ReactorIoOutcome::Err(format!("tay `{tay}` không tồn tại (đã đóng)"));
        };
        if off >= len {
            self.writes.remove(key);
            return ReactorIoOutcome::Data(Vec::new());
        }
        let w = self.writes.get_mut(key).expect("write");
        match s.write(&w.data[w.off..]) {
            Ok(0) => ReactorIoOutcome::Err(format!("ghi `{tay}` trả 0 byte")),
            Ok(n) => {
                w.off += n;
                if w.off >= w.data.len() {
                    self.writes.remove(key);
                    ReactorIoOutcome::Data(Vec::new())
                } else {
                    ReactorIoOutcome::WouldBlock
                }
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::Interrupted => {
                ReactorIoOutcome::WouldBlock
            }
            Err(e) => ReactorIoOutcome::Err(format!("lỗi ghi `{tay}`: {e}")),
        }
    }
}

/// Chờ **một** fd sẵn sàng trong hạn `han`; `Err(())` = hết hạn/lỗi poll.
///
/// Dùng `mio::Poll` riêng của host TLS, không đụng token của reactor scheduler. Mỗi lần đăng ký
/// xong đều gỡ ngay để lần sau đăng ký lại được (kể cả cùng fd).
#[cfg(feature = "tls")]
fn cho_san_sang(
    poll: &mut Poll,
    events: &mut Events,
    fd: RawFd,
    interest: Interest,
    han: Instant,
) -> Result<(), ()> {
    let Some(con) = han.checked_duration_since(Instant::now()) else {
        return Err(());
    };
    let mut src = SourceFd(&fd);
    if poll
        .registry()
        .register(&mut src, Token(usize::MAX), interest)
        .is_err()
    {
        return Err(());
    }
    let r = poll.poll(events, Some(con));
    let _ = poll.registry().deregister(&mut src);
    match r {
        // `mio 1.2` trả `Ok(())`; timeout = không có event nào và đã chạm hạn.
        Ok(()) => {
            if events.is_empty() && Instant::now() >= han {
                Err(())
            } else {
                Ok(())
            }
        }
        Err(e) if e.kind() == ErrorKind::Interrupted => Ok(()),
        Err(_) => Err(()),
    }
}

#[cfg(feature = "tls")]
impl AsyncNet {
    /// **CHỈ TEST/EMBEDDING** — như [`crate::net::Net::moi_voi_goc_tls`], tin đúng các DER cho trước.
    /// Binary `vis` không gọi hàm này; test TLS local dùng để chạy offline.
    #[doc(hidden)]
    pub fn moi_voi_goc_tls(giai_thich: bool, goc_der: &[&[u8]]) -> Result<Self, String> {
        let cau_hinh = crate::net::tls::tu_goc_der(goc_der)?;
        let mut n = Self::new(giai_thich);
        n.cau_hinh_tls = Some(Ok(cau_hinh));
        Ok(n)
    }

    fn cau_hinh(&mut self) -> Result<Arc<ClientConfig>, String> {
        if self.cau_hinh_tls.is_none() {
            self.cau_hinh_tls = Some(crate::net::tls::mac_dinh());
        }
        match self.cau_hinh_tls.as_ref().expect("vừa khởi tạo") {
            Ok(c) => Ok(Arc::clone(c)),
            Err(e) => Err(e.clone()),
        }
    }

    /// Nối TCP nonblocking + handshake rustls, **một** hạn monotonic cho cả hai.
    fn ket_noi_tls(&mut self, dia_chi: &str, timeout_ms: u64) -> Result<TlsChan, String> {
        let timeout_ms = timeout_ms.max(1);
        let han = Instant::now() + Duration::from_millis(timeout_ms);
        let host = crate::net::tls::ten_may_chu(dia_chi)?;
        let ten = ServerName::try_from(host.to_string())
            .map_err(|e| format!("tên máy chủ TLS `{host}` không hợp lệ: {e}"))?;
        let mut ds: Vec<SocketAddr> = dia_chi
            .to_socket_addrs()
            .map_err(|e| format!("địa chỉ `{dia_chi}` không phân giải được: {e}"))?
            .collect();
        // IPv4 trước: Windows có thể trả `::1` trước cho `localhost`.
        ds.sort_by_key(|a| !a.is_ipv4());
        if ds.is_empty() {
            return Err(format!("địa chỉ `{dia_chi}` không có kết quả phân giải"));
        }
        let cau_hinh = self.cau_hinh()?;
        let mut loi_cuoi: Option<String> = None;
        for addr in ds {
            if han.checked_duration_since(Instant::now()).is_none() {
                break;
            }
            let mut sock = match MioStream::connect(addr) {
                Ok(s) => s,
                Err(e) => {
                    loi_cuoi = Some(format!("{addr}: {e}"));
                    continue;
                }
            };
            let fd = sock.as_raw_fd();
            if cho_san_sang(
                &mut self.host_poll,
                &mut self.host_events,
                fd,
                Interest::WRITABLE,
                han,
            )
            .is_err()
            {
                loi_cuoi = Some(format!("{addr}: hết hạn chờ nối"));
                continue;
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
            // `take_error` rỗng chưa đủ: phải chắc fd đã nối xong trước khi handshake.
            if let Err(e) = sock.peer_addr() {
                loi_cuoi = Some(format!("{addr}: {e}"));
                continue;
            }
            let _ = sock.set_nodelay(true);
            let mut conn = ClientConnection::new(Arc::clone(&cau_hinh), ten.clone())
                .map_err(|e| format!("lỗi tạo phiên TLS tới `{dia_chi}`: {e}"))?;
            Self::bap_tay(
                &mut conn,
                &mut sock,
                dia_chi,
                timeout_ms,
                han,
                &mut self.host_poll,
                &mut self.host_events,
            )?;
            return Ok(TlsChan { conn, sock });
        }
        Err(match loi_cuoi {
            Some(e) => format!("lỗi nối TLS tới `{dia_chi}`: {e}"),
            None => format!("quá thời gian chờ nối TLS tới `{dia_chi}` sau {timeout_ms} ms"),
        })
    }

    /// Handshake **nonblocking**: chỉ `read_tls`/`write_tls` khi fd sẵn sàng; không đặt timeout
    /// chặn lên socket. Mọi lỗi chứng chỉ/alert được trả nguyên nhân, có tiền tố `TLS`.
    #[allow(clippy::too_many_arguments)]
    fn bap_tay(
        conn: &mut ClientConnection,
        sock: &mut MioStream,
        dia_chi: &str,
        timeout_ms: u64,
        han: Instant,
        poll: &mut Poll,
        events: &mut Events,
    ) -> Result<(), String> {
        while conn.is_handshaking() {
            let mut tien = false;
            loop {
                if !conn.wants_write() {
                    break;
                }
                match conn.write_tls(sock) {
                    Ok(0) => break,
                    Ok(_) => tien = true,
                    Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                    Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                    Err(e) => {
                        return Err(format!("lỗi TLS handshake tới `{dia_chi}`: {e}"));
                    }
                }
            }
            if conn.wants_read() {
                match conn.read_tls(sock) {
                    Ok(0) => {
                        return Err(format!(
                            "lỗi TLS handshake tới `{dia_chi}`: kết nối đóng trước khi hoàn tất"
                        ));
                    }
                    Ok(_) => {
                        conn.process_new_packets()
                            .map_err(|e| format!("lỗi TLS handshake tới `{dia_chi}`: {e}"))?;
                        tien = true;
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                    Err(e) if e.kind() == ErrorKind::Interrupted => {}
                    Err(e) => {
                        return Err(format!("lỗi TLS handshake tới `{dia_chi}`: {e}"));
                    }
                }
            }
            if !conn.is_handshaking() {
                break;
            }
            if !tien {
                let interest = if conn.wants_write() {
                    Interest::READABLE | Interest::WRITABLE
                } else {
                    Interest::READABLE
                };
                let fd = sock.as_raw_fd();
                if cho_san_sang(poll, events, fd, interest, han).is_err() {
                    return Err(format!(
                        "quá thời gian TLS handshake tới `{dia_chi}` sau {timeout_ms} ms"
                    ));
                }
            }
        }
        // Gửi nốt Finished/alert còn treo; nếu socket chưa sẵn sàng thì để lần I/O sau đẩy tiếp.
        while conn.wants_write() {
            match conn.write_tls(sock) {
                Ok(0) => break,
                Ok(_) => {}
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(format!("lỗi TLS handshake tới `{dia_chi}`: {e}")),
            }
        }
        Ok(())
    }

    /// Đọc plaintext TLS cho API host đồng bộ (`http.get/head`), vẫn nonblocking + `Poll`.
    fn doc_tls(&mut self, tay: &str, toi_da: usize, timeout_ms: u64) -> Result<Vec<u8>, String> {
        if toi_da == 0 {
            return Ok(Vec::new());
        }
        let timeout_ms = timeout_ms.max(1);
        let han = Instant::now() + Duration::from_millis(timeout_ms);
        let mut buf = vec![0u8; toi_da.min(NET_READ_CAP as usize)];
        loop {
            let (fd, tien) = {
                let Some(t) = self.tls.get_mut(tay) else {
                    return Err(format!("tay mạng `{tay}` không tồn tại"));
                };
                match t.conn.reader().read(&mut buf) {
                    Ok(n) => {
                        buf.truncate(n);
                        return Ok(buf);
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                    Err(e) => return Err(format!("lỗi TLS khi đọc từ `{tay}`: {e}")),
                }
                let fd = t.sock.as_raw_fd();
                match t.conn.read_tls(&mut t.sock) {
                    Ok(0) => {
                        return Err(format!(
                            "lỗi TLS khi đọc từ `{tay}`: kết nối đóng bất ngờ (thiếu close_notify)"
                        ));
                    }
                    Ok(_) => {
                        t.conn
                            .process_new_packets()
                            .map_err(|e| format!("lỗi TLS khi đọc từ `{tay}`: {e}"))?;
                        (fd, true)
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => (fd, false),
                    Err(e) if e.kind() == ErrorKind::Interrupted => (fd, false),
                    Err(e) => return Err(format!("lỗi đọc TLS từ `{tay}`: {e}")),
                }
            };
            if tien {
                continue;
            }
            if cho_san_sang(
                &mut self.host_poll,
                &mut self.host_events,
                fd,
                Interest::READABLE,
                han,
            )
            .is_err()
            {
                return Err(format!(
                    "quá thời gian chờ dữ liệu TLS từ `{tay}` sau {timeout_ms} ms"
                ));
            }
        }
    }

    /// Ghi plaintext TLS cho API host đồng bộ; giữ offset plaintext + buffer TLS của rustls.
    fn ghi_tls(&mut self, tay: &str, du_lieu: &[u8], timeout_ms: u64) -> Result<(), String> {
        if du_lieu.is_empty() {
            return Ok(());
        }
        let timeout_ms = timeout_ms.max(1);
        let han = Instant::now() + Duration::from_millis(timeout_ms);
        let mut off = 0usize;
        loop {
            let (fd, done) = {
                let Some(t) = self.tls.get_mut(tay) else {
                    return Err(format!("tay mạng `{tay}` không tồn tại"));
                };
                while off < du_lieu.len() {
                    match t.conn.writer().write(&du_lieu[off..]) {
                        Ok(0) => break,
                        Ok(n) => off += n,
                        Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                        Err(e) => {
                            return Err(format!("lỗi TLS khi ghi tới `{tay}`: {e}"));
                        }
                    }
                }
                while t.conn.wants_write() {
                    match t.conn.write_tls(&mut t.sock) {
                        Ok(0) => break,
                        Ok(_) => {}
                        Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                        Err(e) if e.kind() == ErrorKind::Interrupted => break,
                        Err(e) => {
                            return Err(format!("lỗi TLS khi ghi tới `{tay}`: {e}"));
                        }
                    }
                }
                let done = off >= du_lieu.len() && !t.conn.wants_write();
                (t.sock.as_raw_fd(), done)
            };
            if done {
                return Ok(());
            }
            if cho_san_sang(
                &mut self.host_poll,
                &mut self.host_events,
                fd,
                Interest::WRITABLE,
                han,
            )
            .is_err()
            {
                return Err(format!(
                    "quá thời gian ghi TLS tới `{tay}` sau {timeout_ms} ms"
                ));
            }
        }
    }

    /// `FrameReactorBackend::try_read` cho kênh TLS.
    fn try_read_tls(&mut self, handle: &str, max: usize) -> ReactorIoOutcome {
        let mut buf = vec![0u8; max.min(NET_READ_CAP as usize)];
        loop {
            let Some(t) = self.tls.get_mut(handle) else {
                return ReactorIoOutcome::Err(format!(
                    "tay mạng `{handle}` không tồn tại (đã đóng)"
                ));
            };
            match t.conn.reader().read(&mut buf) {
                Ok(n) => {
                    buf.truncate(n);
                    return ReactorIoOutcome::Data(buf);
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                Err(e) => {
                    return ReactorIoOutcome::Err(format!("lỗi TLS khi đọc `{handle}`: {e}"));
                }
            }
            match t.conn.read_tls(&mut t.sock) {
                Ok(0) => return ReactorIoOutcome::Data(Vec::new()),
                Ok(_) => {
                    if let Err(e) = t.conn.process_new_packets() {
                        return ReactorIoOutcome::Err(format!("lỗi TLS khi đọc `{handle}`: {e}"));
                    }
                }
                Err(e)
                    if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::Interrupted =>
                {
                    return ReactorIoOutcome::WouldBlock;
                }
                Err(e) => {
                    return ReactorIoOutcome::Err(format!("lỗi đọc TLS `{handle}`: {e}"));
                }
            }
        }
    }

    /// `FrameReactorBackend::try_write` cho kênh TLS — partial write giữ offset, không gửi lại.
    fn try_write_tls(&mut self, key: &str) -> ReactorIoOutcome {
        if !self.writes.contains_key(key) {
            let Some(data) = vis_interp::stdlib::frame_net_take_write(key) else {
                return ReactorIoOutcome::Err("buffer write không có (tay đã đóng?)".into());
            };
            self.writes.insert(
                key.to_string(),
                WriteState {
                    tay: Self::target_of(key).to_string(),
                    data,
                    off: 0,
                },
            );
        }
        let tay = Self::target_of(key).to_string();
        let Some(mut w) = self.writes.remove(key) else {
            return ReactorIoOutcome::Err(format!("buffer write `{key}` biến mất"));
        };
        let Some(t) = self.tls.get_mut(&tay) else {
            return ReactorIoOutcome::Err(format!("tay `{tay}` không tồn tại (đã đóng)"));
        };
        while w.off < w.data.len() {
            match t.conn.writer().write(&w.data[w.off..]) {
                Ok(0) => break,
                Ok(n) => w.off += n,
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) => {
                    return ReactorIoOutcome::Err(format!("lỗi TLS khi ghi `{tay}`: {e}"));
                }
            }
        }
        while t.conn.wants_write() {
            match t.conn.write_tls(&mut t.sock) {
                Ok(0) => break,
                Ok(_) => {}
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == ErrorKind::Interrupted => break,
                Err(e) => {
                    return ReactorIoOutcome::Err(format!("lỗi ghi TLS `{tay}`: {e}"));
                }
            }
        }
        if w.off >= w.data.len() && !t.conn.wants_write() {
            ReactorIoOutcome::Data(Vec::new())
        } else {
            self.writes.insert(key.to_string(), w);
            ReactorIoOutcome::WouldBlock
        }
    }
}

impl NetHost for AsyncNet {
    fn connect(&mut self, dia_chi: &str, timeout_ms: u64) -> Result<String, String> {
        self.nhat_ky("connect", dia_chi);
        let addr = loopback_addr(dia_chi)?;
        let s = TcpStream::connect_timeout(&addr, Duration::from_millis(timeout_ms.max(1)))
            .map_err(|e| format!("nối tới `{dia_chi}` thất bại: {e}"))?;
        let _ = s.set_nodelay(true);
        let s = MioStream::from_std(s);
        Ok(self.dat_c(s))
    }

    /// **VS3-1d/W1f-TLS** — TLS thật qua rustls; TCP + handshake đều nonblocking trong một hạn.
    /// Chính sách gốc/chứng chỉ dùng **chung** `net/tls.rs` với runtime sync (`webpki-roots`, hook
    /// `VIS_TLS_TEST_ROOT_DER` chỉ bản debug).
    #[cfg(feature = "tls")]
    fn connect_tls(&mut self, dia_chi: &str, timeout_ms: u64) -> Result<String, String> {
        self.nhat_ky("connect_tls", dia_chi);
        let t = self.ket_noi_tls(dia_chi, timeout_ms)?;
        Ok(self.dat_tls(t))
    }

    #[cfg(not(feature = "tls"))]
    fn connect_tls(&mut self, dia_chi: &str, _timeout_ms: u64) -> Result<String, String> {
        Err(format!(
            "`https://` cần TLS: feature `tls` của `vis-cli` phải bật và nền không phải wasm32 (đang gọi `{dia_chi}`)"
        ))
    }

    fn listen(&mut self, dia_chi: &str) -> Result<String, String> {
        self.nhat_ky("listen", dia_chi);
        let l = TcpListener::bind(dia_chi).map_err(|e| format!("lỗi `listen` `{dia_chi}`: {e}"))?;
        let l = MioListener::from_std(l);
        Ok(self.dat_l(l))
    }

    fn local_addr(&mut self, tay: &str) -> Result<String, String> {
        if let Some(l) = self.listeners.get(tay) {
            return l
                .local_addr()
                .map(|a| a.to_string())
                .map_err(|e| e.to_string());
        }
        if let Some(s) = self.streams.get(tay) {
            return s
                .local_addr()
                .map(|a| a.to_string())
                .map_err(|e| e.to_string());
        }
        #[cfg(feature = "tls")]
        if let Some(t) = self.tls.get(tay) {
            return t
                .sock
                .local_addr()
                .map(|a| a.to_string())
                .map_err(|e| e.to_string());
        }
        Err(format!("tay mạng `{tay}` không tồn tại"))
    }

    fn accept(&mut self, tay: &str, timeout_ms: u64) -> Result<String, String> {
        let Some(l) = self.listeners.get(tay) else {
            return Err(format!("tay listener `{tay}` không tồn tại"));
        };
        let han = Instant::now() + Duration::from_millis(timeout_ms.max(1));
        loop {
            match l.accept() {
                Ok((s, _)) => {
                    return Ok(self.dat_c(s));
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    if Instant::now() >= han {
                        return Err(format!("quá thời gian chờ `accept` sau {timeout_ms} ms"));
                    }
                    std::thread::sleep(Duration::from_millis(2));
                }
                Err(e) => return Err(format!("lỗi `accept`: {e}")),
            }
        }
    }

    fn read(&mut self, tay: &str, toi_da: usize, timeout_ms: u64) -> Result<Vec<u8>, String> {
        #[cfg(feature = "tls")]
        if self.tls.contains_key(tay) {
            return self.doc_tls(tay, toi_da, timeout_ms);
        }
        let Some(s) = self.streams.get_mut(tay) else {
            return Err(format!("tay mạng `{tay}` không tồn tại"));
        };
        let mut buf = vec![0u8; toi_da.min(NET_READ_CAP as usize)];
        let han = Instant::now() + Duration::from_millis(timeout_ms.max(1));
        loop {
            match s.read(&mut buf) {
                Ok(n) => {
                    buf.truncate(n);
                    return Ok(buf);
                }
                Err(e)
                    if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::Interrupted =>
                {
                    if Instant::now() >= han {
                        return Err(format!("quá thời gian đọc `{tay}`"));
                    }
                    std::thread::sleep(Duration::from_millis(2));
                }
                Err(e) => return Err(format!("lỗi đọc `{tay}`: {e}")),
            }
        }
    }

    fn write(&mut self, tay: &str, du_lieu: &[u8]) -> Result<(), String> {
        #[cfg(feature = "tls")]
        if self.tls.contains_key(tay) {
            // Host API không có tham số timeout cho `write`; HTTP đã cắt deadline ở read/connect.
            // Vẫn dùng hạn rộng để không treo vô hạn nếu peer ngừng đọc.
            return self.ghi_tls(tay, du_lieu, 5_000);
        }
        let Some(s) = self.streams.get_mut(tay) else {
            return Err(format!("tay mạng `{tay}` không tồn tại"));
        };
        let mut off = 0;
        while off < du_lieu.len() {
            match s.write(&du_lieu[off..]) {
                Ok(0) => return Err(format!("ghi `{tay}` trả 0 byte")),
                Ok(n) => off += n,
                Err(e)
                    if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::Interrupted =>
                {
                    std::thread::sleep(Duration::from_millis(2));
                }
                Err(e) => return Err(format!("lỗi ghi `{tay}`: {e}")),
            }
        }
        s.flush().map_err(|e| format!("lỗi flush `{tay}`: {e}"))
    }

    fn close(&mut self, tay: &str) -> Result<(), String> {
        let mut existed = self.streams.remove(tay).is_some()
            || self.listeners.remove(tay).is_some()
            || self.connects.remove(tay);
        #[cfg(feature = "tls")]
        {
            existed |= self.tls.remove(tay).is_some();
        }
        // `net.close` có thể đóng một tay đang nối/write (khoá N:/W:).
        if let Some(k) = self
            .writes
            .keys()
            .find(|k| Self::target_of(k) == tay)
            .cloned()
        {
            self.writes.remove(&k);
            existed = true;
        }
        if let Some(k) = self
            .connects
            .iter()
            .find(|k| Self::target_of(k) == tay)
            .cloned()
        {
            self.streams.remove(&k);
            self.connects.remove(&k);
            existed = true;
        }
        if existed {
            Ok(())
        } else {
            Err(format!("tay mạng `{tay}` không tồn tại"))
        }
    }

    fn ghi_nhat_ky(&mut self, dong: &str) {
        if self.giai_thich {
            eprintln!("{dong}");
        }
    }
}

impl FrameReactorBackend for AsyncNet {
    fn try_read(&mut self, handle: &str, max: usize) -> ReactorIoOutcome {
        if handle.starts_with(FRAME_NET_CONNECT) {
            return self.try_connect(handle);
        }
        if handle.starts_with(FRAME_NET_WRITE) {
            return self.try_write(handle);
        }
        if max == 0 {
            return ReactorIoOutcome::Data(Vec::new());
        }
        #[cfg(feature = "tls")]
        if self.tls.contains_key(handle) {
            return self.try_read_tls(handle, max);
        }
        let Some(s) = self.streams.get_mut(handle) else {
            return ReactorIoOutcome::Err(format!("tay mạng `{handle}` không tồn tại (đã đóng)"));
        };
        let mut buf = vec![0u8; max.min(NET_READ_CAP as usize)];
        match s.read(&mut buf) {
            Ok(0) => ReactorIoOutcome::Data(Vec::new()),
            Ok(n) => {
                buf.truncate(n);
                ReactorIoOutcome::Data(buf)
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::Interrupted => {
                ReactorIoOutcome::WouldBlock
            }
            Err(e) => ReactorIoOutcome::Err(format!("lỗi đọc `{handle}`: {e}")),
        }
    }

    fn try_accept(&mut self, handle: &str) -> ReactorIoOutcome {
        let Some(l) = self.listeners.get_mut(handle) else {
            return ReactorIoOutcome::Err(format!("tay listener `{handle}` không tồn tại"));
        };
        match l.accept() {
            Ok((s, _)) => {
                let h = self.dat_c(s);
                ReactorIoOutcome::Accepted(h)
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::Interrupted => {
                ReactorIoOutcome::WouldBlock
            }
            Err(e) => ReactorIoOutcome::Err(format!("lỗi `accept` `{handle}`: {e}")),
        }
    }

    fn register(&mut self, token: u64, handle: &str, _op: ReactorOp) -> Result<(), String> {
        let key = if handle.starts_with(FRAME_NET_WRITE) {
            Self::target_of(handle).to_string()
        } else {
            handle.to_string()
        };
        let doc_hoac_ghi =
            handle.starts_with(FRAME_NET_CONNECT) || handle.starts_with(FRAME_NET_WRITE);
        // Chỉ đăng ký đúng chiều cần: TLS read mà đăng ký luôn WRITABLE sẽ bị đánh thức liên tục
        // (socket gần như luôn ghi được), làm scheduler quay vòng. Alert/Finished còn treo được
        // đẩy tiếp ở lần I/O kế cận hoặc khi đóng.
        let interest = if doc_hoac_ghi {
            Interest::WRITABLE
        } else {
            Interest::READABLE
        };
        #[cfg(feature = "tls")]
        let fd = self
            .tls
            .get(&key)
            .map(|t| t.sock.as_raw_fd())
            .or_else(|| self.streams.get(&key).map(AsRawFd::as_raw_fd))
            .or_else(|| self.listeners.get(&key).map(AsRawFd::as_raw_fd));
        #[cfg(not(feature = "tls"))]
        let fd = self
            .streams
            .get(&key)
            .map(AsRawFd::as_raw_fd)
            .or_else(|| self.listeners.get(&key).map(AsRawFd::as_raw_fd));
        let Some(fd) = fd else {
            return Err(format!("không tìm thấy `{key}` để đăng ký readiness"));
        };
        let mut src = SourceFd(&fd);
        let r = self
            .poll
            .registry()
            .register(&mut src, Token(token as usize), interest);
        match r {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {}
            Err(e) => return Err(format!("register `{key}` thất bại: {e}")),
        }
        self.tokens.insert(token, key);
        Ok(())
    }

    fn deregister(&mut self, token: u64, handle: &str, _op: ReactorOp) -> Result<(), String> {
        let Some(key) = self.tokens.remove(&token) else {
            return Ok(());
        };
        #[cfg(feature = "tls")]
        let fd = self
            .tls
            .get(&key)
            .map(|t| t.sock.as_raw_fd())
            .or_else(|| self.streams.get(&key).map(AsRawFd::as_raw_fd))
            .or_else(|| self.listeners.get(&key).map(AsRawFd::as_raw_fd));
        #[cfg(not(feature = "tls"))]
        let fd = self
            .streams
            .get(&key)
            .map(AsRawFd::as_raw_fd)
            .or_else(|| self.listeners.get(&key).map(AsRawFd::as_raw_fd));
        let Some(fd) = fd else {
            return Ok(());
        };
        let mut src = SourceFd(&fd);
        let r = self.poll.registry().deregister(&mut src);
        if handle.starts_with(FRAME_NET_WRITE) {
            self.writes.remove(handle);
        }
        if handle.starts_with(FRAME_NET_CONNECT) {
            self.connects.remove(handle);
            self.streams.remove(handle);
        }
        match r {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("deregister `{key}` thất bại: {e}")),
        }
    }

    fn wait(&mut self, timeout: Duration) -> Result<Vec<ReactorRawEvent>, String> {
        self.poll
            .poll(&mut self.events, Some(timeout))
            .map_err(|e| format!("epoll_wait thất bại: {e}"))?;
        Ok(self
            .events
            .iter()
            .map(|ev| ReactorRawEvent {
                token: ev.token().0 as u64,
                readable: ev.is_readable() || ev.is_writable(),
                hup: ev.is_read_closed() || ev.is_write_closed(),
                error: ev.is_error(),
            })
            .collect())
    }

    fn close(&mut self, handle: &str) -> Result<(), String> {
        self.streams.remove(handle);
        self.listeners.remove(handle);
        self.connects.remove(handle);
        #[cfg(feature = "tls")]
        {
            self.tls.remove(handle);
        }
        let keys: Vec<u64> = self
            .tokens
            .iter()
            .filter(|(_, k)| k.as_str() == handle)
            .map(|(t, _)| *t)
            .collect();
        for t in keys {
            let _ = <Self as FrameReactorBackend>::deregister(self, t, handle, ReactorOp::Read);
        }
        // write treo theo tay bị đóng.
        self.writes.retain(|_, w| w.tay != handle);
        Ok(())
    }
}

/// Proxy `NetHost` chia sẻ cùng `AsyncNet` với backend đã gắn vào scheduler.
pub struct AsyncNetHost {
    pub inner: Rc<RefCell<AsyncNet>>,
}

impl NetHost for AsyncNetHost {
    fn connect(&mut self, d: &str, t: u64) -> Result<String, String> {
        self.inner.borrow_mut().connect(d, t)
    }
    fn connect_tls(&mut self, d: &str, t: u64) -> Result<String, String> {
        let mut n = self.inner.borrow_mut();
        NetHost::connect_tls(&mut *n, d, t)
    }
    fn listen(&mut self, d: &str) -> Result<String, String> {
        self.inner.borrow_mut().listen(d)
    }
    fn local_addr(&mut self, t: &str) -> Result<String, String> {
        self.inner.borrow_mut().local_addr(t)
    }
    fn accept(&mut self, t: &str, timeout: u64) -> Result<String, String> {
        self.inner.borrow_mut().accept(t, timeout)
    }
    fn read(&mut self, t: &str, n: usize, timeout: u64) -> Result<Vec<u8>, String> {
        self.inner.borrow_mut().read(t, n, timeout)
    }
    fn write(&mut self, t: &str, d: &[u8]) -> Result<(), String> {
        self.inner.borrow_mut().write(t, d)
    }
    fn close(&mut self, t: &str) -> Result<(), String> {
        let mut n = self.inner.borrow_mut();
        NetHost::close(&mut *n, t)
    }
    fn ghi_nhat_ky(&mut self, d: &str) {
        self.inner.borrow_mut().ghi_nhat_ky(d)
    }
}

/// Resolve **loopback only** (giữ chính sách VD-100/105; async net không mở Internet).
fn loopback_addr(dia_chi: &str) -> Result<SocketAddr, String> {
    let (host, cong) = tach(dia_chi)?;
    if !(host == "localhost"
        || host
            .parse::<std::net::IpAddr>()
            .map(|ip| ip.is_loopback())
            .unwrap_or(false))
    {
        return Err(format!(
            "chỉ cho phép địa chỉ loopback (`127.0.0.0/8`, `::1`, `localhost`), được `{host}`"
        ));
    }
    let mut ds: Vec<SocketAddr> = dia_chi
        .to_socket_addrs()
        .map_err(|e| format!("địa chỉ `{dia_chi}` không phân giải được: {e}"))?
        .filter(|a| a.ip().is_loopback())
        .collect();
    if host == "localhost" {
        let v4 = SocketAddr::from(([127, 0, 0, 1], cong));
        if !ds.contains(&v4) {
            ds.push(v4);
        }
    }
    ds.sort_by_key(|a| !a.is_ipv4());
    ds.into_iter()
        .next()
        .ok_or_else(|| format!("địa chỉ `{dia_chi}` không có kết quả loopback"))
}

fn tach(dia_chi: &str) -> Result<(&str, u16), String> {
    let (host, cong) = if let Some(rest) = dia_chi.strip_prefix('[') {
        let (h, sau) = rest
            .split_once(']')
            .ok_or_else(|| format!("địa chỉ IPv6 `{dia_chi}` thiếu `]`"))?;
        let c = sau
            .strip_prefix(':')
            .ok_or_else(|| format!("địa chỉ `{dia_chi}` phải có dạng `host:port`"))?;
        (h, c)
    } else {
        let (h, c) = dia_chi
            .rsplit_once(':')
            .ok_or_else(|| format!("địa chỉ `{dia_chi}` phải có dạng `host:port`"))?;
        if h.contains(':') {
            return Err(format!("địa chỉ IPv6 `{dia_chi}` phải bọc `[IPv6]:port`"));
        }
        (h, c)
    };
    if host.is_empty() {
        return Err(format!("địa chỉ `{dia_chi}` thiếu host"));
    }
    let cong = cong
        .parse::<u16>()
        .map_err(|_| format!("cổng `{cong}` không hợp lệ (0..65535)"))?;
    Ok((host, cong))
}
