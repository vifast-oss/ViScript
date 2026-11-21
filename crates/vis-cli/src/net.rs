// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-63/ST-65 (VD-100/VD-105)** — `NetHost` của CLI: **TCP thật**; `http://` chỉ **loopback**,
//! `https://` (VS3-4a) đi TLS tới host bất kỳ (vẫn qua `cap[Net]`).
//!
//! Handle là chuỗi **mờ**: `"C<n>"` cho kết nối, `"L<n>"` cho listener — chương trình ngôn ngữ không
//! thấy địa chỉ/socket. Coi **cả** `WouldBlock` (macOS trả os error 35 cho `set_read_timeout`) **và**
//! `TimedOut` là **hết giờ**. `accept` timeout 0 ⇒ **chặn**; ngược lại poll non-blocking tới hạn.
//!
//! **VS3-4a (VD-271)**: kênh `C<n>` có thể là TCP trần hoặc **TLS** (rustls + webpki-roots); TLS chỉ
//! bật khi feature `tls` (mặc định trên nền không phải wasm32). `connect_tls` làm luôn TCP +
//! handshake trong **một** hạn; lỗi chứng chỉ/alert trả `Err` có tiền tố `TLS`, không panic.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use vis_interp::builtin::NetHost;

#[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
mod tls;

/// Kênh đã nối: TCP trần, hoặc TLS (chỉ khi bật feature `tls` trên nền không phải wasm32).
enum Kenh {
    Tho(TcpStream),
    #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
    Tls(Box<tls::KenhTls>),
}

impl Read for Kenh {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Kenh::Tho(s) => s.read(buf),
            #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
            Kenh::Tls(s) => s.read(buf),
        }
    }
}

impl Write for Kenh {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Kenh::Tho(s) => s.write(buf),
            #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
            Kenh::Tls(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Kenh::Tho(s) => s.flush(),
            #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
            Kenh::Tls(s) => s.flush(),
        }
    }
}

impl Kenh {
    fn la_tls(&self) -> bool {
        match self {
            Kenh::Tho(_) => false,
            #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
            Kenh::Tls(_) => true,
        }
    }

    fn set_read_timeout(&self, d: Option<Duration>) -> std::io::Result<()> {
        match self {
            Kenh::Tho(s) => s.set_read_timeout(d),
            #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
            Kenh::Tls(s) => s.sock.set_read_timeout(d),
        }
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        match self {
            Kenh::Tho(s) => s.local_addr(),
            #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
            Kenh::Tls(s) => s.sock.local_addr(),
        }
    }
}

/// Host mạng của CLI. `--explain-perms` ⇒ in `NET …` ra **stderr** (không trộn stdout).
pub struct Net {
    giai_thich: bool,
    tay: Vec<(String, Kenh)>,
    tiep: u32,
    /// **ST-65**: listener đang mở — handle `"L<n>"` (tách khỏi `"C<n>"` của kết nối).
    nghe: Vec<(String, TcpListener)>,
    tiep_nghe: u32,
    /// **VS3-4a**: cấu hình TLS dựng **một lần** cho cả tiến trình; `Err` được giữ lại để mọi lần
    /// gọi HTTPS sau đó báo cùng lý do (không panic, không thử lại vô ích).
    #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
    cau_hinh_tls: Option<Result<tls::CauHinh, String>>,
}

impl Net {
    pub fn moi(giai_thich: bool) -> Self {
        Self {
            giai_thich,
            tay: Vec::new(),
            tiep: 0,
            nghe: Vec::new(),
            tiep_nghe: 0,
            #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
            cau_hinh_tls: None,
        }
    }

    /// **VS3-4a — chỉ test/embedding**: dựng `Net` tin đúng các chứng chỉ DER cho trước thay vì
    /// bundle Mozilla. Binary `vis` **không** gọi hàm này và không có cờ/biến môi trường nào đổi
    /// root store; nhờ vậy test local TLS chạy offline mà không mở backdoor cho người dùng.
    #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
    pub fn moi_voi_goc_tls(giai_thich: bool, goc_der: &[&[u8]]) -> Result<Self, String> {
        let cau_hinh = tls::tu_goc_der(goc_der)?;
        let mut net = Self::moi(giai_thich);
        net.cau_hinh_tls = Some(Ok(cau_hinh));
        Ok(net)
    }

    fn nhat_ky(&self, thao_tac: &str, p: &str) {
        if self.giai_thich {
            eprintln!("NET {thao_tac} {p}");
        }
    }

    fn tim(&mut self, tay: &str) -> Result<&mut Kenh, String> {
        self.tay
            .iter_mut()
            .find(|(t, _)| t == tay)
            .map(|(_, s)| s)
            .ok_or_else(|| format!("tay mạng `{tay}` không tồn tại (chưa nối hoặc đã `close`)"))
    }

    fn giu_k(&mut self, s: Kenh) -> String {
        self.tiep += 1;
        let tay = format!("C{}", self.tiep);
        self.tay.push((tay.clone(), s));
        tay
    }

    fn giu(&mut self, s: TcpStream) -> String {
        self.giu_k(Kenh::Tho(s))
    }
}

/// Chỉ **loopback** (VD-100/105 (4)): `127.0.0.1`/`localhost`; host khác ⇒ `Err`.
fn chi_loopback(dia_chi: &str) -> Result<SocketAddr, String> {
    let (host, cong) = dia_chi
        .rsplit_once(':')
        .ok_or_else(|| format!("địa chỉ `{dia_chi}` phải có dạng `host:port`"))?;
    if host != "127.0.0.1" && host != "localhost" {
        return Err(format!(
            "chỉ cho phép loopback (`127.0.0.1`/`localhost`) trong vòng này, được `{host}`"
        ));
    }
    if cong.parse::<u16>().is_err() {
        return Err(format!("cổng `{cong}` không hợp lệ (0..65535)"));
    }
    let mut ds = dia_chi
        .to_socket_addrs()
        .map_err(|e| format!("địa chỉ `{dia_chi}` không phân giải được: {e}"))?;
    ds.next()
        .ok_or_else(|| format!("địa chỉ `{dia_chi}` không có kết quả phân giải"))
}

/// **Cả** `WouldBlock` (macOS) **và** `TimedOut` là **hết giờ** (VD-100 (4)).
fn la_het_gio(e: &std::io::Error) -> bool {
    matches!(
        e.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    )
}

/// **ST-65**: `accept` trên listener; `timeout_ms = 0` ⇒ **chặn** tới khi có kết nối; ngược lại
/// poll **non-blocking** mỗi 5 ms tới hạn (macOS trả `WouldBlock` — cùng logic ST-63).
fn accept_tren(l: &TcpListener, timeout_ms: u64) -> Result<TcpStream, String> {
    if timeout_ms == 0 {
        let _ = l.set_nonblocking(false);
        let (s, _) = l.accept().map_err(|e| format!("lỗi `accept`: {e}"))?;
        let _ = s.set_nonblocking(false);
        return Ok(s);
    }
    let han = Instant::now() + Duration::from_millis(timeout_ms);
    l.set_nonblocking(true)
        .map_err(|e| format!("lỗi đặt non-blocking cho listener: {e}"))?;
    loop {
        match l.accept() {
            Ok((s, _)) => {
                let _ = l.set_nonblocking(false);
                let _ = s.set_nonblocking(false);
                return Ok(s);
            }
            Err(e) if la_het_gio(&e) => {
                if Instant::now() >= han {
                    let _ = l.set_nonblocking(false);
                    return Err(format!("quá thời gian chờ kết nối sau {timeout_ms} ms"));
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(e) => {
                let _ = l.set_nonblocking(false);
                return Err(format!("lỗi `accept`: {e}"));
            }
        }
    }
}

impl NetHost for Net {
    fn connect(&mut self, dia_chi: &str, timeout_ms: u64) -> Result<String, String> {
        self.nhat_ky("connect", dia_chi);
        let addr = chi_loopback(dia_chi)?;
        let s = TcpStream::connect_timeout(&addr, Duration::from_millis(timeout_ms.max(1)))
            .map_err(|e| {
                if la_het_gio(&e) {
                    format!("quá thời gian chờ nối tới `{dia_chi}`")
                } else if e.kind() == std::io::ErrorKind::ConnectionRefused {
                    format!("bị từ chối kết nối tới `{dia_chi}`: {e}")
                } else {
                    format!("lỗi nối tới `{dia_chi}`: {e}")
                }
            })?;
        let _ = s.set_nodelay(true);
        Ok(self.giu(s))
    }

    /// **VS3-4a (VD-271)**: TLS chỉ tồn tại khi bật feature `tls` và không phải wasm32; nếu không,
    /// trả lỗi dữ liệu rõ ràng thay vì "không nối được" mơ hồ.
    #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
    fn connect_tls(&mut self, dia_chi: &str, timeout_ms: u64) -> Result<String, String> {
        self.nhat_ky("connect_tls", dia_chi);
        let con = timeout_ms.max(1);
        let cau_hinh = match self.cau_hinh_tls.get_or_insert_with(tls::mac_dinh) {
            Ok(c) => c.clone(),
            Err(e) => return Err(e.clone()),
        };
        let k = tls::noi(&cau_hinh, dia_chi, con)?;
        Ok(self.giu_k(Kenh::Tls(Box::new(k))))
    }

    #[cfg(not(all(feature = "tls", not(target_arch = "wasm32"))))]
    fn connect_tls(&mut self, dia_chi: &str, _timeout_ms: u64) -> Result<String, String> {
        Err(format!(
            "`https://` cần TLS: feature `tls` của `vis-cli` phải bật và nền không phải wasm32 (đang gọi `{dia_chi}`)"
        ))
    }

    fn listen(&mut self, dia_chi: &str) -> Result<String, String> {
        let addr = chi_loopback(dia_chi)?;
        self.nhat_ky("listen", dia_chi);
        let l = TcpListener::bind(addr).map_err(|e| {
            if e.kind() == std::io::ErrorKind::AddrInUse {
                format!("cổng `{dia_chi}` đang bận (đã có tiến trình listen)")
            } else {
                format!("lỗi `listen` trên `{dia_chi}`: {e}")
            }
        })?;
        self.tiep_nghe += 1;
        let tay = format!("L{}", self.tiep_nghe);
        self.nghe.push((tay.clone(), l));
        Ok(tay)
    }

    fn local_addr(&mut self, tay: &str) -> Result<String, String> {
        // Ưu tiên listener (ST-65), sau đó kết nối (ST-63) — cùng một tên hàm ngôn ngữ.
        if let Some((_, l)) = self.nghe.iter().find(|(t, _)| t == tay) {
            return l
                .local_addr()
                .map(|a| a.to_string())
                .map_err(|e| format!("không lấy được địa chỉ local của `{tay}`: {e}"));
        }
        let s = self.tim(tay)?;
        let a = s
            .local_addr()
            .map_err(|e| format!("không lấy được địa chỉ local của `{tay}`: {e}"))?;
        Ok(a.to_string())
    }

    fn accept(&mut self, tay: &str, timeout_ms: u64) -> Result<String, String> {
        let i = self
            .nghe
            .iter()
            .position(|(t, _)| t == tay)
            .ok_or_else(|| {
                format!("tay listener `{tay}` không tồn tại (đã close hoặc chưa listen)")
            })?;
        self.nhat_ky("accept", tay);
        // Lách borrow: tạm rút listener ra, chờ, rồi trả lại (không đổi thứ tự handle).
        let (ten, l) = self.nghe.remove(i);
        let kq = accept_tren(&l, timeout_ms);
        self.nghe.push((ten, l));
        let s = kq?;
        Ok(self.giu(s))
    }

    fn read(&mut self, tay: &str, toi_da: usize, timeout_ms: u64) -> Result<Vec<u8>, String> {
        if toi_da == 0 {
            return Ok(Vec::new());
        }
        let s = self.tim(tay)?;
        s.set_read_timeout(Some(Duration::from_millis(timeout_ms.max(1))))
            .map_err(|e| format!("không đặt được timeout đọc cho `{tay}`: {e}"))?;
        // **ST-238 / VD-242 (nợ #167)**: hàng rào thứ hai — dù caller có truyền `usize` lớn (lỗi
        // chuyển kiểu/overflow), không bao giờ cấp phát quá trần ⇒ không OOM/abort do input.
        let mut buf = vec![0u8; toi_da.min(vis_interp::stdlib::NET_READ_CAP as usize)];
        match s.read(&mut buf) {
            Ok(0) => Ok(Vec::new()),
            Ok(n) => {
                buf.truncate(n);
                Ok(buf)
            }
            Err(e) if la_het_gio(&e) => Err(format!(
                "quá thời gian chờ dữ liệu từ `{tay}` sau {timeout_ms} ms"
            )),
            Err(e) if s.la_tls() => Err(format!("lỗi TLS khi đọc từ `{tay}`: {e}")),
            Err(e) => Err(format!("lỗi đọc từ `{tay}`: {e}")),
        }
    }

    fn write(&mut self, tay: &str, du_lieu: &[u8]) -> Result<(), String> {
        let s = self.tim(tay)?;
        let la_tls = s.la_tls();
        s.write_all(du_lieu).and_then(|()| s.flush()).map_err(|e| {
            if la_tls {
                format!("lỗi TLS khi ghi tới `{tay}`: {e}")
            } else {
                format!("lỗi ghi tới `{tay}`: {e}")
            }
        })
    }

    fn close(&mut self, tay: &str) -> Result<(), String> {
        // **ST-65**: `close` đóng được **cả** kết nối (`C…`) lẫn listener (`L…`); `close_listen`
        // của ngôn ngữ cũng đi qua đây.
        if let Some(i) = self.tay.iter().position(|(t, _)| t == tay) {
            self.tay.remove(i);
            return Ok(());
        }
        if let Some(i) = self.nghe.iter().position(|(t, _)| t == tay) {
            self.nghe.remove(i);
            return Ok(());
        }
        Err(format!(
            "tay mạng `{tay}` không tồn tại (chưa nối/listen hoặc đã `close`)"
        ))
    }

    /// **ST-64**: nhật ký tầng giao thức (HTTP) khi `--explain-perms`.
    fn ghi_nhat_ky(&mut self, dong: &str) {
        if self.giai_thich {
            eprintln!("{dong}");
        }
    }
}
