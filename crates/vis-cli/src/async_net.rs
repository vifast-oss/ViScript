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
// **VS3-W4 (spike)** — mở cho Windows: mio `TcpStream`/`TcpListener` implement `Source` trên
// cả Unix (epoll) lẫn Windows (IOCP/AFD). Đăng ký trực tiếp trên đối tượng mio, không qua
// `SourceFd`/fd thô: trên Windows trạng thái đăng ký nằm trong chính đối tượng.
#![cfg(any(unix, windows))]

use mio::net::{TcpListener as MioListener, TcpStream as MioStream};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::rc::Rc;
#[cfg(feature = "tls")]
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(feature = "tls")]
use rustls::pki_types::ServerName;
#[cfg(feature = "tls")]
use rustls::{ClientConfig, ClientConnection};

use mio::{Events, Interest, Poll, Token};

use vis_interp::builtin::NetHost;
use vis_interp::rt::{FrameReactorBackend, ReactorIoOutcome, ReactorOp, ReactorRawEvent};
use vis_interp::stdlib::{FRAME_NET_CONNECT, FRAME_NET_WRITE, FRAME_TLS_CONNECT, NET_READ_CAP};

/// `WouldBlock` thật trên nền đang chạy.
///
/// `ErrorKind` đã ánh xạ phần lớn, nhưng một số đường (mio/rustls/connect) trả mã thô:
/// Unix `EINPROGRESS` = **115**, Windows `WSAEWOULDBLOCK` = **10035**,
/// `WSAEINPROGRESS` = **10036**. Không dựa vào một mã duy nhất của Linux.
fn la_would_block(e: &std::io::Error) -> bool {
    e.kind() == ErrorKind::WouldBlock || matches!(e.raw_os_error(), Some(115 | 10035 | 10036))
}

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

/// **VS3-1d/W1f-HTTP** — TLS connect đang chạy trên reactor (`T:<addr>`):
/// TCP nonblocking trước, rồi handshake rustls; `interest` quyết định chiều chờ kế tiếp.
#[cfg(feature = "tls")]
struct DangTls {
    sock: MioStream,
    conn: Option<ClientConnection>,
    interest: Interest,
    dia_chi: String,
}

/// Nguồn readiness của backend: stream hoặc listener.
///
/// Trên cả Unix lẫn Windows, `mio::net::{TcpStream,TcpListener}` implement `mio::event::Source`;
/// đăng ký trực tiếp trên đối tượng là cách duy nhất đúng trên Windows (trạng thái đăng ký nằm
/// trong đối tượng, không có `SourceFd`/fd thô).
enum Nguon<'a> {
    Stream(&'a mut MioStream),
    Listen(&'a mut MioListener),
}

impl Nguon<'_> {
    fn dang_ky(&mut self, poll: &Poll, token: u64, interest: Interest) -> std::io::Result<()> {
        let token = Token(token as usize);
        match self {
            Nguon::Stream(s) => poll.registry().register(&mut **s, token, interest),
            Nguon::Listen(l) => poll.registry().register(&mut **l, token, interest),
        }
    }

    fn dang_ky_lai(&mut self, poll: &Poll, token: u64, interest: Interest) -> std::io::Result<()> {
        let token = Token(token as usize);
        match self {
            Nguon::Stream(s) => poll.registry().reregister(&mut **s, token, interest),
            Nguon::Listen(l) => poll.registry().reregister(&mut **l, token, interest),
        }
    }

    fn huy(&mut self, poll: &Poll) -> std::io::Result<()> {
        match self {
            Nguon::Stream(s) => poll.registry().deregister(&mut **s),
            Nguon::Listen(l) => poll.registry().deregister(&mut **l),
        }
    }
}

#[cfg(feature = "tls")]
fn nguon_mut<'a>(
    streams: &'a mut HashMap<String, MioStream>,
    listeners: &'a mut HashMap<String, MioListener>,
    tls: &'a mut HashMap<String, TlsChan>,
    tls_connects: &'a mut HashMap<String, DangTls>,
    key: &str,
) -> Option<Nguon<'a>> {
    if let Some(t) = tls.get_mut(key) {
        return Some(Nguon::Stream(&mut t.sock));
    }
    if let Some(s) = tls_connects.get_mut(key) {
        return Some(Nguon::Stream(&mut s.sock));
    }
    if let Some(s) = streams.get_mut(key) {
        return Some(Nguon::Stream(s));
    }
    listeners.get_mut(key).map(Nguon::Listen)
}

#[cfg(not(feature = "tls"))]
fn nguon_mut<'a>(
    streams: &'a mut HashMap<String, MioStream>,
    listeners: &'a mut HashMap<String, MioListener>,
    key: &str,
) -> Option<Nguon<'a>> {
    if let Some(s) = streams.get_mut(key) {
        return Some(Nguon::Stream(s));
    }
    listeners.get_mut(key).map(Nguon::Listen)
}

/// Host mạng production cho đường async + backend reactor của nó (cùng một đối tượng).
pub struct AsyncNet {
    streams: HashMap<String, MioStream>,
    listeners: HashMap<String, MioListener>,
    #[cfg(feature = "tls")]
    tls: HashMap<String, TlsChan>,
    #[cfg(feature = "tls")]
    tls_connects: HashMap<String, DangTls>,
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
            tls_connects: HashMap::new(),
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

    /// Socket `tay` có đang được **reactor** đăng ký không?
    ///
    /// **Windows**: một `mio::net::TcpStream`/`TcpListener` chỉ thuộc **một** `Poll`; host call
    /// (dùng `host_poll` riêng) gặp socket reactor-owned phải trả lỗi rõ thay vì deregister mù
    /// (sẽ cướp selector, event stale). Unix nhiều epoll cùng theo dõi một fd được, nên giữ
    /// hành vi cũ (không chặn).
    fn dang_reactor_win(&self, tay: &str) -> bool {
        #[cfg(windows)]
        {
            self.tokens.values().any(|k| k == tay)
        }
        #[cfg(not(windows))]
        {
            let _ = tay;
            false
        }
    }

    /// **CHỈ TEST** — số tay/handle nội bộ còn giữ (đo rò rỉ sau nhiều vòng mở/đóng).
    #[doc(hidden)]
    pub fn dem_tay_dang_mo(&self) -> usize {
        let mut n =
            self.streams.len() + self.listeners.len() + self.writes.len() + self.connects.len();
        #[cfg(feature = "tls")]
        {
            n += self.tls.len() + self.tls_connects.len();
        }
        n
    }

    fn target_of(h: &str) -> &str {
        if let Some(x) = h.strip_prefix(FRAME_NET_WRITE) {
            x
        } else {
            h
        }
    }

    /// Khoá connect của HTTP async có hậu tố `|frame` để hai task cùng host không chung state.
    fn dia_chi_cua_khoa_noi<'a>(key: &'a str, tien_to: &str) -> &'a str {
        let raw = key.strip_prefix(tien_to).unwrap_or(key);
        raw.split_once('|').map(|(a, _)| a).unwrap_or(raw)
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
        let addr_text = Self::dia_chi_cua_khoa_noi(key, FRAME_NET_CONNECT);
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
        match s.take_error() {
            Ok(Some(e)) => {
                self.streams.remove(key);
                self.connects.remove(key);
                return ReactorIoOutcome::Err(format!("nối tới `{addr_text}` thất bại: {e}"));
            }
            Err(e) => {
                self.streams.remove(key);
                self.connects.remove(key);
                return ReactorIoOutcome::Err(format!("lỗi kiểm tra nối `{addr_text}`: {e}"));
            }
            Ok(None) => {}
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
            Err(e) if la_would_block(&e) || e.kind() == ErrorKind::NotConnected => {
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
            Ok(0) => {
                self.writes.remove(key);
                ReactorIoOutcome::Err(format!("ghi `{tay}` trả 0 byte"))
            }
            Ok(n) => {
                w.off += n;
                if w.off >= w.data.len() {
                    self.writes.remove(key);
                    ReactorIoOutcome::Data(Vec::new())
                } else {
                    ReactorIoOutcome::WouldBlock
                }
            }
            Err(e) if la_would_block(&e) || e.kind() == ErrorKind::Interrupted => {
                ReactorIoOutcome::WouldBlock
            }
            Err(e) => {
                self.writes.remove(key);
                ReactorIoOutcome::Err(format!("lỗi ghi `{tay}`: {e}"))
            }
        }
    }
}

/// Chờ **một** socket sẵn sàng trong hạn `han`; `Err(())` = hết hạn/lỗi poll.
///
/// Dùng `mio::Poll` riêng của host TLS, không đụng token của reactor scheduler. Mỗi lần đăng ký
/// xong đều gỡ ngay để lần sau đăng ký lại được (kể cả cùng socket).
///
/// **Windows**: đăng ký trực tiếp `MioStream` (mio lưu trạng thái đăng ký trong đối tượng, một
/// đối tượng chỉ thuộc **một** `Poll`); không có `SourceFd`.
/// Vì sao một lần chờ host TLS thất bại.
#[cfg(feature = "tls")]
enum LoiChoSanSang {
    /// Hết hạn monotonic.
    HetHan,
    /// Socket đang thuộc reactor scheduler (Windows: mỗi socket chỉ thuộc một `Poll`).
    DangReactor,
}

/// Chờ **một** socket sẵn sàng trong hạn `han`.
///
/// Dùng `mio::Poll` riêng của host TLS, không đụng token của reactor scheduler. Mỗi lần đăng ký
/// xong đều gỡ ngay để lần sau đăng ký lại được (kể cả cùng socket).
///
/// **Windows**: đăng ký trực tiếp `MioStream` (mio lưu trạng thái đăng ký trong đối tượng, một
/// đối tượng chỉ thuộc **một** `Poll`); không có `SourceFd`. `dang_reactor` do caller kiểm từ
/// `AsyncNet::dang_reactor_win`; `AlreadyExists` cũng bị coi là socket đang thuộc `Poll` khác
/// (không deregister mù).
#[cfg(feature = "tls")]
fn cho_san_sang(
    poll: &mut Poll,
    events: &mut Events,
    sock: &mut MioStream,
    interest: Interest,
    han: Instant,
    dang_reactor: bool,
) -> Result<(), LoiChoSanSang> {
    if dang_reactor {
        return Err(LoiChoSanSang::DangReactor);
    }
    let Some(con) = han.checked_duration_since(Instant::now()) else {
        return Err(LoiChoSanSang::HetHan);
    };
    match poll.registry().register(sock, Token(usize::MAX), interest) {
        Ok(()) => {}
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {
            return Err(LoiChoSanSang::DangReactor);
        }
        Err(_) => return Err(LoiChoSanSang::HetHan),
    }
    let r = poll.poll(events, Some(con));
    let _ = poll.registry().deregister(sock);
    match r {
        // `mio 1.2` trả `Ok(())`; timeout = không có event nào và đã chạm hạn.
        Ok(()) => {
            if events.is_empty() && Instant::now() >= han {
                Err(LoiChoSanSang::HetHan)
            } else {
                Ok(())
            }
        }
        Err(e) if e.kind() == ErrorKind::Interrupted => Ok(()),
        Err(_) => Err(LoiChoSanSang::HetHan),
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
            match cho_san_sang(
                &mut self.host_poll,
                &mut self.host_events,
                &mut sock,
                Interest::WRITABLE,
                han,
                false,
            ) {
                Ok(()) => {}
                Err(LoiChoSanSang::DangReactor) => {
                    loi_cuoi = Some(format!("{addr}: socket đang thuộc reactor scheduler"));
                    continue;
                }
                Err(LoiChoSanSang::HetHan) => {
                    loi_cuoi = Some(format!("{addr}: hết hạn chờ nối"));
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
                    Err(e) if la_would_block(&e) => break,
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
                    Err(e) if la_would_block(&e) => {}
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
                match cho_san_sang(poll, events, sock, interest, han, false) {
                    Ok(()) => {}
                    Err(LoiChoSanSang::DangReactor) => {
                        return Err(format!(
                            "TLS handshake tới `{dia_chi}`: socket đang thuộc reactor scheduler"
                        ));
                    }
                    Err(LoiChoSanSang::HetHan) => {
                        return Err(format!(
                            "quá thời gian TLS handshake tới `{dia_chi}` sau {timeout_ms} ms"
                        ));
                    }
                }
            }
        }
        // Gửi nốt Finished/alert còn treo; nếu socket chưa sẵn sàng thì để lần I/O sau đẩy tiếp.
        while conn.wants_write() {
            match conn.write_tls(sock) {
                Ok(0) => break,
                Ok(_) => {}
                Err(e) if la_would_block(&e) => break,
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(format!("lỗi TLS handshake tới `{dia_chi}`: {e}")),
            }
        }
        Ok(())
    }

    /// **HTTP async**: connect TLS trên reactor, key `T:<dia_chi>`. TCP connect + handshake
    /// nhiều bước; trả `Accepted("C<n>")` chỉ khi handshake xong. Deadline/token do core giữ.
    fn try_connect_tls(&mut self, key: &str) -> ReactorIoOutcome {
        if !self.tls_connects.contains_key(key) {
            let dia_chi = Self::dia_chi_cua_khoa_noi(key, FRAME_TLS_CONNECT).to_string();
            let mut ds: Vec<SocketAddr> = match dia_chi.to_socket_addrs() {
                Ok(x) => x.collect(),
                Err(e) => {
                    return ReactorIoOutcome::Err(format!(
                        "địa chỉ TLS `{dia_chi}` không phân giải được: {e}"
                    ))
                }
            };
            ds.sort_by_key(|a| !a.is_ipv4());
            let Some(addr) = ds.into_iter().next() else {
                return ReactorIoOutcome::Err(format!(
                    "địa chỉ TLS `{dia_chi}` không có kết quả phân giải"
                ));
            };
            let sock = match MioStream::connect(addr) {
                Ok(s) => s,
                Err(e) => {
                    return ReactorIoOutcome::Err(format!("nối TLS tới `{dia_chi}` thất bại: {e}"))
                }
            };
            self.tls_connects.insert(
                key.to_string(),
                DangTls {
                    sock,
                    conn: None,
                    interest: Interest::WRITABLE,
                    dia_chi,
                },
            );
            return ReactorIoOutcome::WouldBlock;
        }
        let can_cfg = self
            .tls_connects
            .get(key)
            .map(|s| s.conn.is_none())
            .unwrap_or(false);
        let cfg = if can_cfg {
            match self.cau_hinh() {
                Ok(c) => Some(c),
                Err(e) => {
                    self.tls_connects.remove(key);
                    return ReactorIoOutcome::Err(e);
                }
            }
        } else {
            None
        };
        let Some(st) = self.tls_connects.get_mut(key) else {
            return ReactorIoOutcome::Err(format!("TLS connect `{key}` biến mất"));
        };
        match Self::bap_tay_reactor(st, cfg) {
            Ok(true) => {
                let Some(st) = self.tls_connects.remove(key) else {
                    return ReactorIoOutcome::Err(format!("TLS connect `{key}` biến mất"));
                };
                let Some(conn) = st.conn else {
                    return ReactorIoOutcome::Err(format!("TLS connect `{key}` thiếu phiên"));
                };
                let h = self.dat_tls(TlsChan {
                    conn,
                    sock: st.sock,
                });
                ReactorIoOutcome::Accepted(h)
            }
            Ok(false) => ReactorIoOutcome::WouldBlock,
            Err(e) => {
                self.tls_connects.remove(key);
                ReactorIoOutcome::Err(e)
            }
        }
    }

    /// Một bước không chặn của TLS connect: hoàn tất TCP (nếu cần), rồi bơm handshake.
    /// `Ok(true)` = handshake xong; `Ok(false)` = chờ `st.interest`; `Err` = lỗi TLS/TCP.
    fn bap_tay_reactor(st: &mut DangTls, cfg: Option<Arc<ClientConfig>>) -> Result<bool, String> {
        if st.conn.is_none() {
            match st.sock.take_error() {
                Ok(Some(e)) => return Err(format!("nối TLS tới `{}` thất bại: {e}", st.dia_chi)),
                Err(e) => return Err(format!("kiểm tra nối TLS `{}`: {e}", st.dia_chi)),
                Ok(None) => {}
            }
            if let Err(e) = st.sock.peer_addr() {
                // Chưa nối xong (EINPROGRESS/NotConnected): chờ WRITABLE tiếp.
                let _ = e;
                st.interest = Interest::WRITABLE;
                return Ok(false);
            }
            let _ = st.sock.set_nodelay(true);
            let host = crate::net::tls::ten_may_chu(&st.dia_chi)?;
            let ten = ServerName::try_from(host.to_string())
                .map_err(|e| format!("tên máy chủ TLS `{host}` không hợp lệ: {e}"))?;
            let cfg = cfg.ok_or_else(|| {
                format!(
                    "thiếu cấu hình TLS cho `{}` (trạng thái nội bộ)",
                    st.dia_chi
                )
            })?;
            let conn = ClientConnection::new(cfg, ten)
                .map_err(|e| format!("lỗi tạo phiên TLS tới `{}`: {e}", st.dia_chi))?;
            st.conn = Some(conn);
        }
        let conn = st.conn.as_mut().expect("vừa tạo");
        loop {
            let mut tien = false;
            loop {
                if !conn.wants_write() {
                    break;
                }
                match conn.write_tls(&mut st.sock) {
                    Ok(0) => break,
                    Ok(_) => tien = true,
                    Err(e) if la_would_block(&e) => break,
                    Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                    Err(e) => return Err(format!("lỗi TLS handshake tới `{}`: {e}", st.dia_chi)),
                }
            }
            if conn.wants_read() {
                match conn.read_tls(&mut st.sock) {
                    Ok(0) => {
                        return Err(format!(
                            "lỗi TLS handshake tới `{}`: kết nối đóng trước khi hoàn tất",
                            st.dia_chi
                        ));
                    }
                    Ok(_) => {
                        conn.process_new_packets()
                            .map_err(|e| format!("lỗi TLS handshake tới `{}`: {e}", st.dia_chi))?;
                        tien = true;
                    }
                    Err(e) if la_would_block(&e) => {}
                    Err(e) if e.kind() == ErrorKind::Interrupted => {}
                    Err(e) => {
                        return Err(format!("lỗi TLS handshake tới `{}`: {e}", st.dia_chi));
                    }
                }
            }
            if !conn.is_handshaking() {
                // Đẩy nốt Finished nếu socket còn ghi được; nếu không, lần I/O sau sẽ đẩy.
                while conn.wants_write() {
                    match conn.write_tls(&mut st.sock) {
                        Ok(0) => break,
                        Ok(_) => {}
                        Err(e) if la_would_block(&e) => break,
                        Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                        Err(e) => {
                            return Err(format!("lỗi TLS handshake tới `{}`: {e}", st.dia_chi))
                        }
                    }
                }
                return Ok(true);
            }
            if !tien {
                st.interest = if conn.wants_write() {
                    Interest::WRITABLE
                } else {
                    Interest::READABLE
                };
                return Ok(false);
            }
        }
    }

    /// Đọc plaintext TLS cho API host đồng bộ (`http.get/head`), vẫn nonblocking + `Poll`.
    fn doc_tls(&mut self, tay: &str, toi_da: usize, timeout_ms: u64) -> Result<Vec<u8>, String> {
        if toi_da == 0 {
            return Ok(Vec::new());
        }
        if self.dang_reactor_win(tay) {
            return Err(format!(
                "tay `{tay}` đang thuộc reactor scheduler (Windows: một socket chỉ thuộc một Poll)"
            ));
        }
        let timeout_ms = timeout_ms.max(1);
        let han = Instant::now() + Duration::from_millis(timeout_ms);
        let mut buf = vec![0u8; toi_da.min(NET_READ_CAP as usize)];
        loop {
            let tien = {
                let Some(t) = self.tls.get_mut(tay) else {
                    return Err(format!("tay mạng `{tay}` không tồn tại"));
                };
                match t.conn.reader().read(&mut buf) {
                    Ok(n) => {
                        buf.truncate(n);
                        return Ok(buf);
                    }
                    Err(e) if la_would_block(&e) => {}
                    Err(e) => return Err(format!("lỗi TLS khi đọc từ `{tay}`: {e}")),
                }
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
                        true
                    }
                    Err(e) if la_would_block(&e) => false,
                    Err(e) if e.kind() == ErrorKind::Interrupted => false,
                    Err(e) => return Err(format!("lỗi đọc TLS từ `{tay}`: {e}")),
                }
            };
            if tien {
                continue;
            }
            // Mượn rời từng trường: socket để chờ, `host_poll`/`host_events` để poll.
            let dang_reactor = self.dang_reactor_win(tay);
            let host_poll = &mut self.host_poll;
            let host_events = &mut self.host_events;
            let Some(t) = self.tls.get_mut(tay) else {
                return Err(format!("tay mạng `{tay}` không tồn tại"));
            };
            match cho_san_sang(
                host_poll,
                host_events,
                &mut t.sock,
                Interest::READABLE,
                han,
                dang_reactor,
            ) {
                Ok(()) => {}
                Err(LoiChoSanSang::DangReactor) => {
                    return Err(format!(
                        "tay `{tay}` đang thuộc reactor scheduler (Windows: một socket chỉ thuộc một Poll)"
                    ));
                }
                Err(LoiChoSanSang::HetHan) => {
                    return Err(format!(
                        "quá thời gian chờ dữ liệu TLS từ `{tay}` sau {timeout_ms} ms"
                    ));
                }
            }
        }
    }

    /// Ghi plaintext TLS cho API host đồng bộ; giữ offset plaintext + buffer TLS của rustls.
    fn ghi_tls(&mut self, tay: &str, du_lieu: &[u8], timeout_ms: u64) -> Result<(), String> {
        if du_lieu.is_empty() {
            return Ok(());
        }
        if self.dang_reactor_win(tay) {
            return Err(format!(
                "tay `{tay}` đang thuộc reactor scheduler (Windows: một socket chỉ thuộc một Poll)"
            ));
        }
        let timeout_ms = timeout_ms.max(1);
        let han = Instant::now() + Duration::from_millis(timeout_ms);
        let mut off = 0usize;
        loop {
            let done = {
                let Some(t) = self.tls.get_mut(tay) else {
                    return Err(format!("tay mạng `{tay}` không tồn tại"));
                };
                while off < du_lieu.len() {
                    match t.conn.writer().write(&du_lieu[off..]) {
                        Ok(0) => break,
                        Ok(n) => off += n,
                        Err(e) if la_would_block(&e) => break,
                        Err(e) => {
                            return Err(format!("lỗi TLS khi ghi tới `{tay}`: {e}"));
                        }
                    }
                }
                while t.conn.wants_write() {
                    match t.conn.write_tls(&mut t.sock) {
                        Ok(0) => break,
                        Ok(_) => {}
                        Err(e) if la_would_block(&e) => break,
                        Err(e) if e.kind() == ErrorKind::Interrupted => break,
                        Err(e) => {
                            return Err(format!("lỗi TLS khi ghi tới `{tay}`: {e}"));
                        }
                    }
                }
                off >= du_lieu.len() && !t.conn.wants_write()
            };
            if done {
                return Ok(());
            }
            let dang_reactor = self.dang_reactor_win(tay);
            let host_poll = &mut self.host_poll;
            let host_events = &mut self.host_events;
            let Some(t) = self.tls.get_mut(tay) else {
                return Err(format!("tay mạng `{tay}` không tồn tại"));
            };
            match cho_san_sang(
                host_poll,
                host_events,
                &mut t.sock,
                Interest::WRITABLE,
                han,
                dang_reactor,
            ) {
                Ok(()) => {}
                Err(LoiChoSanSang::DangReactor) => {
                    return Err(format!(
                        "tay `{tay}` đang thuộc reactor scheduler (Windows: một socket chỉ thuộc một Poll)"
                    ));
                }
                Err(LoiChoSanSang::HetHan) => {
                    return Err(format!(
                        "quá thời gian ghi TLS tới `{tay}` sau {timeout_ms} ms"
                    ));
                }
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
                Err(e) if la_would_block(&e) => {}
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
                Err(e) if la_would_block(&e) || e.kind() == ErrorKind::Interrupted => {
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
                Err(e) if la_would_block(&e) => break,
                Err(e) => {
                    return ReactorIoOutcome::Err(format!("lỗi TLS khi ghi `{tay}`: {e}"));
                }
            }
        }
        while t.conn.wants_write() {
            match t.conn.write_tls(&mut t.sock) {
                Ok(0) => break,
                Ok(_) => {}
                Err(e) if la_would_block(&e) => break,
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
        // `mio::net::TcpStream::from_std` KHÔNG tự đặt nonblocking (cả Unix lẫn Windows);
        // thiếu bước này host đọc/ghi có thể chặn và IOCP không re-arm đúng.
        s.set_nonblocking(true)
            .map_err(|e| format!("đặt nonblocking `{dia_chi}` thất bại: {e}"))?;
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
        // Như `connect`: `from_std` không tự đặt nonblocking; Windows cần cho accept/AFD.
        l.set_nonblocking(true)
            .map_err(|e| format!("đặt nonblocking `{dia_chi}` thất bại: {e}"))?;
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
                Err(e) if la_would_block(&e) => {
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
                Err(e) if la_would_block(&e) || e.kind() == ErrorKind::Interrupted => {
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
                Err(e) if la_would_block(&e) || e.kind() == ErrorKind::Interrupted => {
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
            existed |= self.tls_connects.remove(tay).is_some();
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
        #[cfg(feature = "tls")]
        if handle.starts_with(FRAME_TLS_CONNECT) {
            return self.try_connect_tls(handle);
        }
        #[cfg(not(feature = "tls"))]
        if handle.starts_with(FRAME_TLS_CONNECT) {
            return ReactorIoOutcome::Err(
                "`https://` cần TLS: feature `tls` của `vis-cli` phải bật và nền không phải wasm32"
                    .to_string(),
            );
        }
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
            Err(e) if la_would_block(&e) || e.kind() == ErrorKind::Interrupted => {
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
            Err(e) if la_would_block(&e) || e.kind() == ErrorKind::Interrupted => {
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
        #[cfg(feature = "tls")]
        let la_tls_noi = key.starts_with(FRAME_TLS_CONNECT);
        #[cfg(not(feature = "tls"))]
        let la_tls_noi = false;
        let doc_hoac_ghi = handle.starts_with(FRAME_NET_CONNECT)
            || handle.starts_with(FRAME_NET_WRITE)
            || la_tls_noi;
        // Chỉ đăng ký đúng chiều cần: TLS read mà đăng ký luôn WRITABLE sẽ bị đánh thức liên tục
        // (socket gần như luôn ghi được), làm scheduler quay vòng. Alert/Finished còn treo được
        // đẩy tiếp ở lần I/O kế cận hoặc khi đóng.
        #[cfg(feature = "tls")]
        let interest = if la_tls_noi {
            self.tls_connects
                .get(&key)
                .map(|s| s.interest)
                .unwrap_or(Interest::WRITABLE)
        } else if doc_hoac_ghi {
            Interest::WRITABLE
        } else {
            Interest::READABLE
        };
        #[cfg(not(feature = "tls"))]
        let interest = if doc_hoac_ghi {
            Interest::WRITABLE
        } else {
            Interest::READABLE
        };
        #[cfg(feature = "tls")]
        let nguon = nguon_mut(
            &mut self.streams,
            &mut self.listeners,
            &mut self.tls,
            &mut self.tls_connects,
            &key,
        );
        #[cfg(not(feature = "tls"))]
        let nguon = nguon_mut(&mut self.streams, &mut self.listeners, &key);
        let Some(mut nguon) = nguon else {
            return Err(format!("không tìm thấy `{key}` để đăng ký readiness"));
        };
        let mut r = nguon.dang_ky(&self.poll, token, interest);
        if matches!(&r, Err(e) if e.kind() == ErrorKind::AlreadyExists) {
            // Cùng nguồn vừa được adapter chuyển từ handle tạm (`N:`/`T:`) sang `C<n>`:
            // registration cũ còn trong selector. Trên Windows `reregister` cập nhật token/
            // interest của **chính** selector đã đăng ký; trên Unix cũng là epoll MOD.
            r = nguon.dang_ky_lai(&self.poll, token, interest);
        }
        match r {
            Ok(()) => {}
            Err(e) => return Err(format!("register `{key}` thất bại: {e}")),
        }
        self.tokens.insert(token, key);
        Ok(())
    }

    fn deregister(&mut self, token: u64, _handle: &str, _op: ReactorOp) -> Result<(), String> {
        let Some(key) = self.tokens.remove(&token) else {
            return Ok(());
        };
        #[cfg(feature = "tls")]
        let nguon = nguon_mut(
            &mut self.streams,
            &mut self.listeners,
            &mut self.tls,
            &mut self.tls_connects,
            &key,
        );
        #[cfg(not(feature = "tls"))]
        let nguon = nguon_mut(&mut self.streams, &mut self.listeners, &key);
        let Some(mut nguon) = nguon else {
            return Ok(());
        };
        match nguon.huy(&self.poll) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("deregister `{key}` thất bại: {e}")),
        }
    }

    fn wait(&mut self, timeout: Duration) -> Result<Vec<ReactorRawEvent>, String> {
        self.poll
            .poll(&mut self.events, Some(timeout))
            .map_err(|e| format!("poll readiness thất bại: {e}"))?;
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
            self.tls_connects.remove(handle);
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
