// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-1d/W1f** — host + reactor production cho chương trình **async** trên frame scheduler.
//!
//! Đây là bản production hoá adapter `Net2` của test `lat256_net`: cùng `mio` (epoll) + socket
//! nonblocking, nhưng nằm trong binary `vis`. Chỉ dùng cho nhánh **async**; runtime sync vẫn dùng
//! [`crate::net::Net`] nguyên vẹn. **Linux/Unix và Windows** dùng backend này (mio `Source` trực
//! tiếp: epoll / IOCP-AFD); wasm không có backend (đường cũ). Async-h2 vẫn Unix-only.
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

#[cfg(feature = "async-h2")]
#[path = "async_net_h2.rs"]
mod h2_impl;
#[path = "async_net_host.rs"]
mod host_impl;
#[path = "async_net_nguon.rs"]
mod nguon_impl;

use host_impl::loopback_addr;
pub use host_impl::AsyncNetHost;
use nguon_impl::interest_tu_huong;
use nguon_impl::la_would_block;
#[cfg(feature = "tls")]
use nguon_impl::{cho_san_sang, LoiChoSanSang};

use mio::net::{TcpListener as MioListener, TcpStream as MioStream};
use std::collections::{HashMap, HashSet};
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
#[cfg(feature = "tls")]
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(feature = "tls")]
use rustls::pki_types::ServerName;
#[cfg(feature = "tls")]
use rustls::{ClientConfig, ClientConnection};

use mio::{Events, Interest, Poll, Token, Waker};

use vis_interp::builtin::NetHost;
#[cfg(feature = "async-h2")]
use vis_interp::rt::ReactorH2Outcome;
use vis_interp::rt::{FrameReactorBackend, ReactorIoOutcome, ReactorOp, ReactorRawEvent};
use vis_interp::stdlib::{FRAME_NET_CONNECT, FRAME_NET_WRITE, FRAME_TLS_CONNECT, NET_READ_CAP};

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
    /// **VS3-W2** — connection HTTP/2 dùng chung, handle `H<n>`.
    #[cfg(feature = "async-h2")]
    h2: HashMap<String, crate::net::h2_async::H2Async>,
    /// Số frame đang giữ `H<n>` (mỗi lần `try_connect_tls` trả handle là +1, `close` là -1).
    #[cfg(feature = "async-h2")]
    h2_users: HashMap<String, usize>,
    /// Authority → `H<n>` còn sống (tái dùng cho request concurrent/sequential).
    #[cfg(feature = "async-h2")]
    h2_theo_authority: HashMap<String, String>,
    /// Khoá `T:…|frame` → khoá canonical `T:…` trong lúc handshake dùng chung.
    #[cfg(feature = "async-h2")]
    tls_alias: HashMap<String, String>,
    /// Authority đã handshake ra HTTP/1.1: các connect sau dùng khoá riêng từng frame.
    #[cfg(feature = "async-h2")]
    h1_authority: HashSet<String>,
    /// Thời điểm phiên h2 rảnh (0 user); tái dùng nếu < 15 s như sync pool.
    #[cfg(feature = "async-h2")]
    h2_idle: HashMap<String, Instant>,
    #[cfg(feature = "async-h2")]
    next_h2: u32,
    poll: Poll,
    events: Events,
    /// Waker để đánh thức `wait` khi một frame h2 vừa lái session và làm xong stream
    /// của frame khác (không còn readiness socket nào để wake tự nhiên).
    #[allow(dead_code)] // chỉ dùng khi feature async-h2 bật
    waker: Waker,
    /// Khoá nguồn cần phát event tổng hợp ngay ở `wait` kế tiếp.
    wake_nguon: HashSet<String>,
    /// Poll riêng cho các lời gọi host đồng bộ (`http.get/head`) trên kênh TLS; không dùng chung
    /// token với reactor để không đụng đăng ký của scheduler.
    #[cfg(feature = "tls")]
    host_poll: Poll,
    #[cfg(feature = "tls")]
    host_events: Events,
    tokens: HashMap<u64, String>,
    /// Token scheduler → khoá nguồn canonical; khoá nguồn → mọi token cùng chờ.
    token_nguon: HashMap<u64, String>,
    nguon_tokens: HashMap<String, HashSet<u64>>,
    /// Khoá nguồn → token nội bộ của `Poll` (một registration cho mỗi socket).
    poll_token: HashMap<String, usize>,
    poll_nguon: HashMap<usize, String>,
    token_wants: HashMap<u64, (bool, bool)>,
    next_poll: usize,
    /// `N:<addr>` đã khởi tạo connect, chờ WRITABLE/kiểm SO_ERROR.
    connects: HashSet<String>,
    writes: HashMap<String, WriteState>,
    next_conn: u32,
    next_listen: u32,
    giai_thich: bool,
}

impl AsyncNet {
    pub fn new(giai_thich: bool) -> Self {
        let poll = Poll::new().expect("mio::Poll::new");
        let waker = Waker::new(poll.registry(), Token(usize::MAX)).expect("mio::Waker");
        Self {
            streams: HashMap::new(),
            listeners: HashMap::new(),
            #[cfg(feature = "tls")]
            tls: HashMap::new(),
            #[cfg(feature = "tls")]
            tls_connects: HashMap::new(),
            #[cfg(feature = "tls")]
            cau_hinh_tls: None,
            #[cfg(feature = "async-h2")]
            h2: HashMap::new(),
            #[cfg(feature = "async-h2")]
            h2_users: HashMap::new(),
            #[cfg(feature = "async-h2")]
            h2_theo_authority: HashMap::new(),
            #[cfg(feature = "async-h2")]
            tls_alias: HashMap::new(),
            #[cfg(feature = "async-h2")]
            h1_authority: HashSet::new(),
            #[cfg(feature = "async-h2")]
            h2_idle: HashMap::new(),
            #[cfg(feature = "async-h2")]
            next_h2: 0,
            poll,
            events: Events::with_capacity(128),
            waker,
            wake_nguon: HashSet::new(),
            #[cfg(feature = "tls")]
            host_poll: Poll::new().expect("mio::Poll::new (host TLS)"),
            #[cfg(feature = "tls")]
            host_events: Events::with_capacity(16),
            tokens: HashMap::new(),
            token_nguon: HashMap::new(),
            nguon_tokens: HashMap::new(),
            poll_token: HashMap::new(),
            poll_nguon: HashMap::new(),
            token_wants: HashMap::new(),
            next_poll: 0,
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

    /// **VS3-W2** — khoá thực để tra fd/state: alias `T:…|frame` → canonical trong lúc handshake.
    fn khoa_thuc(&self, key: &str) -> String {
        #[cfg(feature = "async-h2")]
        if key.starts_with('H') {
            return Self::h2_base(key).to_string();
        }
        #[cfg(feature = "async-h2")]
        if key.starts_with(FRAME_TLS_CONNECT) {
            if let Some(c) = self.tls_alias.get(key) {
                return c.clone();
            }
            return self.khoa_tls_canonical(key);
        }
        key.to_string()
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
                self.doi_khoa_nguon(key, &h);
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
        #[cfg(feature = "async-h2")]
        {
            // Đã có phiên h2 sẵn sàng cho authority này ⇒ dùng chung connection.
            let now = Instant::now();
            let dia_chi = Self::dia_chi_cua_khoa_noi(key, FRAME_TLS_CONNECT).to_string();
            if !self.h1_authority.contains(&dia_chi) {
                if let Some(h) = self.h2_theo_authority.get(&dia_chi).cloned() {
                    let con_idle = self
                        .h2_idle
                        .get(&h)
                        .map(|t| now.saturating_duration_since(*t) < crate::net::h2::H2_IDLE)
                        .unwrap_or(true);
                    if con_idle && self.h2.contains_key(&h) {
                        self.h2_idle.remove(&h);
                        *self.h2_users.entry(h.clone()).or_insert(0) += 1;
                        let duoi = key.rsplit_once('|').map(|(_, d)| d).unwrap_or("0");
                        return ReactorIoOutcome::Accepted(format!("{h}#{duoi}"));
                    }
                    self.h2.remove(&h);
                    self.h2_users.remove(&h);
                    self.h2_idle.remove(&h);
                    self.h2_theo_authority.remove(&dia_chi);
                }
            }
            let ck = self.khoa_thuc(key);
            if ck != key {
                self.tls_alias.insert(key.to_string(), ck.clone());
            }
            self.try_connect_tls_khoa(key, &ck)
        }
        #[cfg(not(feature = "async-h2"))]
        self.try_connect_tls_khoa(key, key)
    }

    /// Thân handshake trên khoá canonical `ck`; `key` là handle core đang giữ.
    fn try_connect_tls_khoa(&mut self, key: &str, ck: &str) -> ReactorIoOutcome {
        if !self.tls_connects.contains_key(ck) {
            let dia_chi = Self::dia_chi_cua_khoa_noi(ck, FRAME_TLS_CONNECT).to_string();
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
                ck.to_string(),
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
            .get(ck)
            .map(|s| s.conn.is_none())
            .unwrap_or(false);
        let cfg = if can_cfg {
            match self.cau_hinh() {
                Ok(c) => Some({
                    #[cfg(feature = "async-h2")]
                    {
                        crate::net::tls::cau_hinh_h2(&c)
                    }
                    #[cfg(not(feature = "async-h2"))]
                    {
                        c
                    }
                }),
                Err(e) => {
                    self.tls_connects.remove(ck);
                    return ReactorIoOutcome::Err(e);
                }
            }
        } else {
            None
        };
        let Some(st) = self.tls_connects.get_mut(ck) else {
            return ReactorIoOutcome::Err(format!("TLS connect `{key}` biến mất"));
        };
        let kq_bap = Self::bap_tay_reactor(st, cfg);
        match kq_bap {
            Ok(true) => {
                let Some(st) = self.tls_connects.remove(ck) else {
                    return ReactorIoOutcome::Err(format!("TLS connect `{key}` biến mất"));
                };
                let Some(conn) = st.conn else {
                    return ReactorIoOutcome::Err(format!("TLS connect `{key}` thiếu phiên"));
                };
                #[cfg(feature = "async-h2")]
                if conn.alpn_protocol() == Some(&b"h2"[..]) {
                    let sock: std::net::TcpStream = st.sock.into();
                    let kenh = rustls::StreamOwned::new(conn, sock);
                    let phien = crate::net::h2_async::H2Async::moi_cho_bap(kenh, &st.dia_chi);
                    let h = self.dat_h2(phien);
                    self.doi_khoa_nguon(ck, &h);
                    self.h2_theo_authority.insert(st.dia_chi.clone(), h.clone());
                    *self.h2_users.entry(h.clone()).or_insert(0) += 1;
                    self.tls_alias.retain(|_, v| v != ck);
                    let duoi = key.rsplit_once('|').map(|(_, d)| d).unwrap_or("0");
                    return ReactorIoOutcome::Accepted(format!("{h}#{duoi}"));
                }
                #[cfg(feature = "async-h2")]
                {
                    // Server không chọn h2: các connect sau của authority này đi khoá riêng.
                    self.h1_authority.insert(st.dia_chi.clone());
                    self.tls_alias.retain(|_, v| v != ck);
                }
                let h = self.dat_tls(TlsChan {
                    conn,
                    sock: st.sock,
                });
                self.doi_khoa_nguon(ck, &h);
                ReactorIoOutcome::Accepted(h)
            }
            Ok(false) => ReactorIoOutcome::WouldBlock,
            Err(e) => {
                self.tls_connects.remove(ck);
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
            self.bo_sach_nguon(tay);
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

    fn register(&mut self, token: u64, handle: &str, op: ReactorOp) -> Result<(), String> {
        let key = if handle.starts_with(FRAME_NET_WRITE) {
            Self::target_of(handle).to_string()
        } else {
            handle.to_string()
        };
        let tra = self.khoa_thuc(&key);
        #[cfg(feature = "tls")]
        let la_tls_noi = tra.starts_with(FRAME_TLS_CONNECT);
        #[cfg(not(feature = "tls"))]
        let la_tls_noi = false;
        let doc_hoac_ghi =
            key.starts_with(FRAME_NET_CONNECT) || key.starts_with(FRAME_NET_WRITE) || la_tls_noi;
        let (want_doc, want_ghi) = match op {
            ReactorOp::ReadWrite => {
                #[cfg(feature = "async-h2")]
                {
                    if let Some(p) = self.h2.get(&tra) {
                        let i = p.interest();
                        (i.is_readable(), i.is_writable())
                    } else {
                        (true, false)
                    }
                }
                #[cfg(not(feature = "async-h2"))]
                {
                    let _ = &tra;
                    (true, false)
                }
            }
            _ => {
                #[cfg(feature = "tls")]
                if la_tls_noi {
                    let i = self
                        .tls_connects
                        .get(&tra)
                        .map(|s| s.interest)
                        .unwrap_or(Interest::WRITABLE);
                    (i.is_readable(), i.is_writable())
                } else if doc_hoac_ghi {
                    (false, true)
                } else {
                    (true, false)
                }
                #[cfg(not(feature = "tls"))]
                if doc_hoac_ghi {
                    (false, true)
                } else {
                    (true, false)
                }
            }
        };
        self.token_nguon.insert(token, tra.clone());
        self.nguon_tokens
            .entry(tra.clone())
            .or_default()
            .insert(token);
        self.token_wants.insert(token, (want_doc, want_ghi));
        let (er, ew) = self.interest_cua_nguon(&tra);
        let interest = interest_tu_huong(er, ew);
        if let Some(&ep) = self.poll_token.get(&tra) {
            if let Err(e) = self.dang_ky_lai_nguon(&tra, Token(ep), interest) {
                self.bo_token(token);
                return Err(format!("register `{tra}` thất bại: {e}"));
            }
        } else {
            let ep = self.next_poll;
            self.next_poll += 1;
            if let Err(e) = self.dang_ky_nguon(&tra, Token(ep), interest) {
                self.bo_token(token);
                return Err(format!("register `{tra}` thất bại: {e}"));
            }
            self.poll_token.insert(tra.clone(), ep);
            self.poll_nguon.insert(ep, tra.clone());
        }
        self.tokens.insert(token, handle.to_string());
        Ok(())
    }

    fn deregister(&mut self, token: u64, _handle: &str, _op: ReactorOp) -> Result<(), String> {
        self.tokens.remove(&token);
        self.token_wants.remove(&token);
        let Some(tra) = self.token_nguon.remove(&token) else {
            return Ok(());
        };
        let con_lai = match self.nguon_tokens.get_mut(&tra) {
            Some(ds) => {
                ds.remove(&token);
                !ds.is_empty()
            }
            None => false,
        };
        if !con_lai {
            self.nguon_tokens.remove(&tra);
            if let Some(ep) = self.poll_token.remove(&tra) {
                self.poll_nguon.remove(&ep);
                match self.huy_nguon(&tra) {
                    Ok(()) => {}
                    Err(e) if e.kind() == ErrorKind::NotFound => {}
                    Err(e) => return Err(format!("deregister `{tra}` thất bại: {e}")),
                }
            }
        } else {
            // Còn token khác: thu hẹp interest về đúng nhu cầu còn lại.
            let (er, ew) = self.interest_cua_nguon(&tra);
            if let Some(&ep) = self.poll_token.get(&tra) {
                if let Err(e) = self.dang_ky_lai_nguon(&tra, Token(ep), interest_tu_huong(er, ew)) {
                    return Err(format!("deregister `{tra}` thất bại: {e}"));
                }
            }
        }
        Ok(())
    }

    fn wait(&mut self, timeout: Duration) -> Result<Vec<ReactorRawEvent>, String> {
        self.poll
            .poll(&mut self.events, Some(timeout))
            .map_err(|e| format!("poll readiness thất bại: {e}"))?;
        let mut out = Vec::new();
        let mut da_co: HashSet<u64> = HashSet::new();
        for ev in self.events.iter() {
            let ep = ev.token().0;
            let Some(tra) = self.poll_nguon.get(&ep) else {
                continue;
            };
            let readable = ev.is_readable() || ev.is_writable();
            let hup = ev.is_read_closed() || ev.is_write_closed();
            let error = ev.is_error();
            // Phát cho **mọi** token scheduler đang chờ cùng một khoá nguồn: nhiều frame cùng
            // một connection h2 phải được đánh thức cùng lúc, không token nào bị ghi đè.
            if let Some(ds) = self.nguon_tokens.get(tra) {
                for t in ds {
                    if da_co.insert(*t) {
                        out.push(ReactorRawEvent {
                            token: *t,
                            readable,
                            hup,
                            error,
                        });
                    }
                }
            }
        }
        // Frame vừa poll h2 có thể đã hoàn tất stream của frame khác; phát event tổng hợp
        // cho mọi token cùng nguồn để không mất wake.
        for key in std::mem::take(&mut self.wake_nguon) {
            if let Some(ds) = self.nguon_tokens.get(&key) {
                for t in ds {
                    if da_co.insert(*t) {
                        out.push(ReactorRawEvent {
                            token: *t,
                            readable: true,
                            hup: false,
                            error: false,
                        });
                    }
                }
            }
        }
        Ok(out)
    }

    fn close(&mut self, handle: &str) -> Result<(), String> {
        #[cfg(feature = "async-h2")]
        if self.dong_h2(handle) {
            return Ok(());
        }
        self.streams.remove(handle);
        self.listeners.remove(handle);
        self.connects.remove(handle);
        #[cfg(feature = "tls")]
        {
            self.tls.remove(handle);
            #[cfg(not(feature = "async-h2"))]
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
        #[cfg(feature = "async-h2")]
        let la_h2 = handle.starts_with('H');
        #[cfg(not(feature = "async-h2"))]
        let la_h2 = false;
        if !la_h2 {
            self.bo_sach_nguon(handle);
        }
        Ok(())
    }

    #[cfg(feature = "async-h2")]
    fn h2_ready(&mut self, handle: &str) -> Result<bool, String> {
        self.h2_ready_impl(handle)
    }

    #[cfg(feature = "async-h2")]
    fn h2_send(
        &mut self,
        handle: &str,
        method: &str,
        target: &str,
        headers: &[(String, String)],
    ) -> Result<u64, String> {
        self.h2_send_impl(handle, method, target, headers)
    }

    #[cfg(feature = "async-h2")]
    fn h2_poll(&mut self, handle: &str, id: u64) -> ReactorH2Outcome {
        self.h2_poll_impl(handle, id)
    }

    #[cfg(feature = "async-h2")]
    fn h2_cancel(&mut self, handle: &str, id: u64) {
        self.h2_cancel_impl(handle, id);
    }
}
