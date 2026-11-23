// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-286 (R0c)** — quản lý kết nối của [`super::AsyncNet`]: cấp/thu hồi handle
//! `C<n>`/`L<n>`, connect `N:<addr>`, write `W:<tay>`, và toàn bộ kênh TLS (`T:<addr>`,
//! handshake rustls nonblocking, đọc/ghi plaintext cho host đồng bộ).
//!
//! Tách khỏi `async_net.rs` để giữ luật R1 (≤ 1500 dòng/tệp). Đây là module con nên vẫn
//! truy cập được field private của `AsyncNet`; method dùng từ module cha/sibling đánh dấu
//! `pub(super)`. **0 đổi hành vi** — chỉ di chuyển mã nguyên văn (ST-286).

use super::*;

#[cfg(feature = "tls")]
use std::net::{SocketAddr, ToSocketAddrs};

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
            frame_tay: HashMap::new(),
            frame_h2: HashMap::new(),
            next_conn: 0,
            next_listen: 0,
            giai_thich,
        }
    }

    pub(super) fn nhat_ky(&self, thao_tac: &str, p: &str) {
        if self.giai_thich {
            eprintln!("NET {thao_tac} {p}");
        }
    }

    pub(super) fn target_of(h: &str) -> &str {
        if let Some(x) = h.strip_prefix(FRAME_NET_WRITE) {
            x
        } else {
            h
        }
    }

    /// Khoá connect của HTTP async có hậu tố `|frame` để hai task cùng host không chung state.
    pub(super) fn dia_chi_cua_khoa_noi<'a>(key: &'a str, tien_to: &str) -> &'a str {
        let raw = key.strip_prefix(tien_to).unwrap_or(key);
        raw.split_once('|').map(|(a, _)| a).unwrap_or(raw)
    }

    pub(super) fn dat_c(&mut self, s: MioStream) -> String {
        self.next_conn += 1;
        let h = format!("C{}", self.next_conn);
        self.streams.insert(h.clone(), s);
        h
    }

    pub(super) fn dat_l(&mut self, l: MioListener) -> String {
        self.next_listen += 1;
        let h = format!("L{}", self.next_listen);
        self.listeners.insert(h.clone(), l);
        h
    }

    #[cfg(feature = "tls")]
    pub(super) fn dat_tls(&mut self, t: TlsChan) -> String {
        self.next_conn += 1;
        let h = format!("C{}", self.next_conn);
        self.tls.insert(h.clone(), t);
        h
    }

    /// **VS3-W2** — khoá thực để tra fd/state: alias `T:…|frame` → canonical trong lúc handshake.
    pub(super) fn khoa_thuc(&self, key: &str) -> String {
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

    pub(super) fn try_connect(&mut self, key: &str) -> ReactorIoOutcome {
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

    pub(super) fn try_write(&mut self, key: &str) -> ReactorIoOutcome {
        #[cfg(feature = "tls")]
        if self.tls.contains_key(Self::target_of(key)) {
            return self.try_write_tls(key);
        }
        if !self.writes.contains_key(key) {
            let Some(data) = vis_core::reactor::frame_net_take_write(key) else {
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
    pub(super) fn ket_noi_tls(
        &mut self,
        dia_chi: &str,
        timeout_ms: u64,
    ) -> Result<TlsChan, String> {
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
    pub(super) fn try_connect_tls(&mut self, key: &str) -> ReactorIoOutcome {
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
                    let phien =
                        crate::net::h2_async::H2Async::moi_cho_bap(conn, st.sock, &st.dia_chi);
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
    pub(super) fn doc_tls(
        &mut self,
        tay: &str,
        toi_da: usize,
        timeout_ms: u64,
    ) -> Result<Vec<u8>, String> {
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
    pub(super) fn ghi_tls(
        &mut self,
        tay: &str,
        du_lieu: &[u8],
        timeout_ms: u64,
    ) -> Result<(), String> {
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
    pub(super) fn try_read_tls(&mut self, handle: &str, max: usize) -> ReactorIoOutcome {
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
            let Some(data) = vis_core::reactor::frame_net_take_write(key) else {
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
