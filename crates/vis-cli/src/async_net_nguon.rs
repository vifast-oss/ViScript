// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-W4** — nguồn readiness cross-platform cho [`super::AsyncNet`]: đăng ký trực tiếp
//! `mio::net::TcpStream`/`TcpListener` (Unix epoll, Windows IOCP/AFD), bookkeeping nhiều token
//! trên cùng một nguồn (h2), và bất biến `host_poll` vs reactor trên Windows.
//!
//! Tách khỏi `async_net.rs` để giữ luật R1 (≤ 1500 dòng/tệp); là module con nên vẫn truy cập
//! được field private của `AsyncNet`.

use super::*;

/// `WouldBlock` thật trên nền đang chạy.
///
/// `ErrorKind` đã ánh xạ phần lớn, nhưng một số đường (mio/rustls/connect) trả mã thô:
/// Unix `EINPROGRESS` = **115**, Windows `WSAEWOULDBLOCK` = **10035**,
/// `WSAEINPROGRESS` = **10036**. Không dựa vào một mã duy nhất của Linux.
pub(super) fn la_would_block(e: &std::io::Error) -> bool {
    e.kind() == ErrorKind::WouldBlock || matches!(e.raw_os_error(), Some(115 | 10035 | 10036))
}

/// Nguồn readiness của backend cho socket TCP thuần.
///
/// Trên cả Unix lẫn Windows, `mio::net::{TcpStream,TcpListener}` implement `mio::event::Source`;
/// đăng ký trực tiếp trên đối tượng là cách duy nhất đúng trên Windows (trạng thái đăng ký nằm
/// trong đối tượng, không có `SourceFd`/fd thô). TLS và h2 có đường riêng trong
/// `AsyncNet::dang_ky_nguon`.
enum Nguon<'a> {
    Stream(&'a mut MioStream),
    Listen(&'a mut MioListener),
}

impl Nguon<'_> {
    fn dang_ky(&mut self, poll: &Poll, token: Token, interest: Interest) -> std::io::Result<()> {
        match self {
            Nguon::Stream(s) => poll.registry().register(&mut **s, token, interest),
            Nguon::Listen(l) => poll.registry().register(&mut **l, token, interest),
        }
    }

    fn dang_ky_lai(
        &mut self,
        poll: &Poll,
        token: Token,
        interest: Interest,
    ) -> std::io::Result<()> {
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

fn nguon_tcp<'a>(
    streams: &'a mut HashMap<String, MioStream>,
    listeners: &'a mut HashMap<String, MioListener>,
    key: &str,
) -> Option<Nguon<'a>> {
    if let Some(s) = streams.get_mut(key) {
        return Some(Nguon::Stream(s));
    }
    listeners.get_mut(key).map(Nguon::Listen)
}

/// Đổi cờ hướng chờ sang `mio::Interest`; luôn có ít nhất READABLE.
pub(super) fn interest_tu_huong(doc: bool, ghi: bool) -> Interest {
    match (doc, ghi) {
        (true, true) => Interest::READABLE | Interest::WRITABLE,
        (false, true) => Interest::WRITABLE,
        _ => Interest::READABLE,
    }
}

impl AsyncNet {
    /// Hợp interest **hiện tại** của mọi token còn chờ trên khoá nguồn `key`.
    pub(super) fn interest_cua_nguon(&self, key: &str) -> (bool, bool) {
        let mut doc = false;
        let mut ghi = false;
        if let Some(ds) = self.nguon_tokens.get(key) {
            for t in ds {
                if let Some((r, w)) = self.token_wants.get(t) {
                    doc |= *r;
                    ghi |= *w;
                }
            }
        }
        (doc, ghi)
    }

    /// Gỡ token khỏi sổ (dùng khi register lỗi) — không đụng epoll.
    pub(super) fn bo_token(&mut self, token: u64) {
        self.tokens.remove(&token);
        self.token_wants.remove(&token);
        if let Some(tra) = self.token_nguon.remove(&token) {
            if let Some(ds) = self.nguon_tokens.get_mut(&tra) {
                ds.remove(&token);
                if ds.is_empty() {
                    self.nguon_tokens.remove(&tra);
                }
            }
        }
    }

    /// Socket `tay` có đang được **reactor** đăng ký không?
    ///
    /// **Windows**: một `mio::net::TcpStream`/`TcpListener` chỉ thuộc **một** `Poll`; host call
    /// (dùng `host_poll` riêng) gặp socket reactor-owned phải trả lỗi rõ thay vì deregister mù
    /// (sẽ cướp selector, event stale). Unix nhiều epoll cùng theo dõi một fd được, nên giữ
    /// hành vi cũ (không chặn).
    pub(super) fn dang_reactor_win(&self, tay: &str) -> bool {
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
        // **ST-287** — h2 giữ session cho tái dùng, nhưng **user** phải về 0 sau huỷ.
        #[cfg(feature = "async-h2")]
        {
            n += self.h2_users.values().sum::<usize>();
        }
        n
    }

    /// Đăng ký khoá nguồn `key` với `poll` (đối tượng mio trực tiếp; TLS/h2 có đường riêng).
    pub(super) fn dang_ky_nguon(
        &mut self,
        key: &str,
        token: Token,
        interest: Interest,
    ) -> std::io::Result<()> {
        #[cfg(feature = "async-h2")]
        if let Some(p) = self.h2.get_mut(key) {
            return p.dang_ky_nguon(&self.poll, token, interest);
        }
        #[cfg(feature = "tls")]
        if let Some(t) = self.tls.get_mut(key) {
            return self.poll.registry().register(&mut t.sock, token, interest);
        }
        #[cfg(feature = "tls")]
        if let Some(s) = self.tls_connects.get_mut(key) {
            return self.poll.registry().register(&mut s.sock, token, interest);
        }
        if let Some(mut n) = nguon_tcp(&mut self.streams, &mut self.listeners, key) {
            return n.dang_ky(&self.poll, token, interest);
        }
        Err(std::io::Error::new(
            ErrorKind::NotFound,
            format!("không tìm thấy `{key}`"),
        ))
    }

    pub(super) fn dang_ky_lai_nguon(
        &mut self,
        key: &str,
        token: Token,
        interest: Interest,
    ) -> std::io::Result<()> {
        #[cfg(feature = "async-h2")]
        if let Some(p) = self.h2.get_mut(key) {
            return p.dang_ky_lai_nguon(&self.poll, token, interest);
        }
        #[cfg(feature = "tls")]
        if let Some(t) = self.tls.get_mut(key) {
            return self
                .poll
                .registry()
                .reregister(&mut t.sock, token, interest);
        }
        #[cfg(feature = "tls")]
        if let Some(s) = self.tls_connects.get_mut(key) {
            return self
                .poll
                .registry()
                .reregister(&mut s.sock, token, interest);
        }
        if let Some(mut n) = nguon_tcp(&mut self.streams, &mut self.listeners, key) {
            return n.dang_ky_lai(&self.poll, token, interest);
        }
        Err(std::io::Error::new(
            ErrorKind::NotFound,
            format!("không tìm thấy `{key}`"),
        ))
    }

    pub(super) fn huy_nguon(&mut self, key: &str) -> std::io::Result<()> {
        #[cfg(feature = "async-h2")]
        if let Some(p) = self.h2.get_mut(key) {
            return p.huy_nguon(&self.poll);
        }
        #[cfg(feature = "tls")]
        if let Some(t) = self.tls.get_mut(key) {
            return self.poll.registry().deregister(&mut t.sock);
        }
        #[cfg(feature = "tls")]
        if let Some(s) = self.tls_connects.get_mut(key) {
            return self.poll.registry().deregister(&mut s.sock);
        }
        if let Some(mut n) = nguon_tcp(&mut self.streams, &mut self.listeners, key) {
            return n.huy(&self.poll);
        }
        Err(std::io::Error::new(
            ErrorKind::NotFound,
            format!("không tìm thấy `{key}`"),
        ))
    }

    /// **Windows** — socket đổi khoá (`N:…`→`C<n>`, `T:…`→`C<n>`/`H<n>`): chuyển mọi token/
    /// interest/registration nội bộ sang khoá mới. Trên Windows đăng ký gắn với **đối tượng**,
    /// nên không được để khoá cũ giữ token stale.
    pub(super) fn doi_khoa_nguon(&mut self, cu: &str, moi: &str) {
        if cu == moi {
            return;
        }
        if let Some(ds) = self.nguon_tokens.remove(cu) {
            self.nguon_tokens
                .entry(moi.to_string())
                .or_default()
                .extend(ds.iter().copied());
            for t in ds {
                self.token_nguon.insert(t, moi.to_string());
            }
        }
        if let Some(ep) = self.poll_token.remove(cu) {
            self.poll_token.insert(moi.to_string(), ep);
            self.poll_nguon.insert(ep, moi.to_string());
        }
    }

    /// Dọn sổ sách của một khoá nguồn **đã bị drop** (không gọi deregister trên đối tượng).
    pub(super) fn bo_sach_nguon(&mut self, key: &str) {
        if let Some(ep) = self.poll_token.remove(key) {
            self.poll_nguon.remove(&ep);
        }
        if let Some(ds) = self.nguon_tokens.remove(key) {
            for t in ds {
                self.token_nguon.remove(&t);
                self.token_wants.remove(&t);
            }
        }
    }
}

/// Vì sao một lần chờ host TLS thất bại.
#[cfg(feature = "tls")]
pub(super) enum LoiChoSanSang {
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
pub(super) fn cho_san_sang(
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
