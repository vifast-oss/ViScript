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

#[path = "async_net_conn.rs"]
mod conn_impl;
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
use std::net::{TcpListener, TcpStream};
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
    /// **ST-287 (VD-273)** — tay HTTP async (`C<n>`/`H<n>#frame`/`N:`/`T:`) theo frame, để
    /// safe point huỷ đóng đúng tay. Chỉ `net/async_http.rs` ghi sổ; language handle không vào đây.
    frame_tay: HashMap<u32, Vec<String>>,
    /// **ST-287** — stream h2 theo frame: huỷ gửi `RST_STREAM` đúng stream, session dùng chung
    /// với frame khác không bị đóng.
    frame_h2: HashMap<u32, Vec<(String, u64)>>,
    next_conn: u32,
    next_listen: u32,
    giai_thich: bool,
}

impl AsyncNet {
    /// **ST-287 (VD-273)** — bỏ `handle` khỏi sổ tay HTTP async của mọi frame (đã đóng/hoàn tất).
    fn xoa_so_tay(&mut self, handle: &str) {
        // Chỉ xoá **đúng** handle: `H<n>#frame` của frame khác dùng chung session phải giữ.
        self.frame_tay.retain(|_, ds| {
            ds.retain(|h| h != handle);
            !ds.is_empty()
        });
        self.frame_h2.retain(|_, ds| {
            ds.retain(|(h, _)| h != handle);
            !ds.is_empty()
        });
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
        // **ST-287 (VD-273)**: tay đã đóng/hoàn tất không còn trong sổ huỷ của frame.
        self.xoa_so_tay(handle);
        #[cfg(feature = "async-h2")]
        if self.dong_h2(handle) {
            return Ok(());
        }
        // **ST-287 (Windows)**: deregister TRƯỚC khi drop `MioStream`. Trên Windows, drop/
        // `closesocket` huỷ I/O đang chờ; deregister sau đó có thể trả lỗi/abort (10053).
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
        self.streams.remove(handle);
        self.listeners.remove(handle);
        self.connects.remove(handle);
        #[cfg(feature = "tls")]
        {
            self.tls.remove(handle);
            #[cfg(not(feature = "async-h2"))]
            self.tls_connects.remove(handle);
        }
        #[cfg(feature = "async-h2")]
        let la_h2 = handle.starts_with('H');
        #[cfg(not(feature = "async-h2"))]
        let la_h2 = false;
        if !la_h2 {
            self.bo_sach_nguon(handle);
        }
        Ok(())
    }

    fn note_task_handle(&mut self, frame_id: u32, handle: &str) {
        let ds = self.frame_tay.entry(frame_id).or_default();
        // Connect xong (`C`/`H`) thì khoá chờ `N:`/`T:` không còn là tay sống.
        if handle.starts_with('C') || handle.starts_with('H') {
            ds.retain(|h| !(h.starts_with(FRAME_NET_CONNECT) || h.starts_with(FRAME_TLS_CONNECT)));
        }
        if !ds.iter().any(|h| h == handle) {
            ds.push(handle.to_string());
        }
    }

    fn note_task_h2(&mut self, frame_id: u32, handle: &str, id: u64) {
        let ds = self.frame_h2.entry(frame_id).or_default();
        if !ds.iter().any(|(h, i)| h == handle && *i == id) {
            ds.push((handle.to_string(), id));
        }
    }

    fn cancel_task_handles(&mut self, frame_id: u32) {
        let h2 = self.frame_h2.remove(&frame_id).unwrap_or_default();
        let mut tay = self.frame_tay.remove(&frame_id).unwrap_or_default();
        for (h, id) in h2 {
            <Self as FrameReactorBackend>::h2_cancel(self, &h, id);
            if !tay.iter().any(|x| x == &h) {
                tay.push(h);
            }
        }
        for h in tay {
            let _ = <Self as FrameReactorBackend>::close(self, &h);
        }
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
        // Stream đã huỷ/hoàn tất không còn trong sổ safe point.
        for ds in self.frame_h2.values_mut() {
            ds.retain(|(h, i)| !(h == handle && *i == id));
        }
        self.h2_cancel_impl(handle, id);
    }
}
