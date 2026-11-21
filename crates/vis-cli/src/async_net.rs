// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-1d/W1f** — host + reactor production cho chương trình **async** trên frame scheduler.
//!
//! Đây là bản production hoá adapter `Net2` của test `lat256_net`: cùng `mio` (epoll) + socket
//! nonblocking, nhưng nằm trong binary `vis`. Chỉ dùng cho nhánh **async**; runtime sync vẫn dùng
//! [`crate::net::Net`] nguyên vẹn. Linux/Unix trước; Windows/wasm không có backend này (đường cũ).
//!
//! Handle giữ đúng quy ước của `stdlib`: `C<n>` kết nối, `L<n>` listener, `N:<addr>` connect,
//! `W:<tay>` write. `FrameReactorBackend` chỉ đăng ký fd; deadline/token/wake do `rt` quản.
#![cfg(unix)]

use mio::net::{TcpListener as MioListener, TcpStream as MioStream};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::os::fd::AsRawFd;
use std::rc::Rc;
use std::time::{Duration, Instant};

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

/// Host mạng production cho đường async + backend reactor của nó (cùng một đối tượng).
pub struct AsyncNet {
    streams: HashMap<String, MioStream>,
    listeners: HashMap<String, MioListener>,
    poll: Poll,
    events: Events,
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
            poll: Poll::new().expect("mio::Poll::new"),
            events: Events::with_capacity(128),
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
        let interest =
            if handle.starts_with(FRAME_NET_CONNECT) || handle.starts_with(FRAME_NET_WRITE) {
                Interest::WRITABLE
            } else {
                Interest::READABLE
            };
        let r = if let Some(s) = self.streams.get_mut(&key) {
            let fd = s.as_raw_fd();
            let mut src = SourceFd(&fd);
            self.poll
                .registry()
                .register(&mut src, Token(token as usize), interest)
        } else if let Some(l) = self.listeners.get_mut(&key) {
            let fd = l.as_raw_fd();
            let mut src = SourceFd(&fd);
            self.poll
                .registry()
                .register(&mut src, Token(token as usize), interest)
        } else {
            return Err(format!("không tìm thấy `{key}` để đăng ký readiness"));
        };
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
        let r = if let Some(s) = self.streams.get_mut(&key) {
            let fd = s.as_raw_fd();
            let mut src = SourceFd(&fd);
            self.poll.registry().deregister(&mut src)
        } else if let Some(l) = self.listeners.get_mut(&key) {
            let fd = l.as_raw_fd();
            let mut src = SourceFd(&fd);
            self.poll.registry().deregister(&mut src)
        } else {
            return Ok(());
        };
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
