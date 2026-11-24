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
//!
//! **VS3-4b (VD-271)**: pool **HTTP keep-alive** theo `scheme|host:port` (tối đa 8 kết nối idle,
//! hết hạn sau 15 s). Chỉ tầng HTTP dùng pool (`connect_http`/`tra_http`); `net.connect` thô vẫn
//! mở kết nối mới. Server đóng kết nối idle ⇒ tầng HTTP thử lại một lần (GET/HEAD) trên kết nối mới.

use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use vis_core::cookie::CookieJar;
use vis_core::host::NetHost;

#[cfg(all(feature = "http2", not(target_arch = "wasm32")))]
pub(crate) mod h2;
/// **VS3-W2** — HTTP/2 reactor-native cho async, feature `async-h2` (default-off).
#[cfg(all(
    feature = "async-h2",
    feature = "http2",
    feature = "tls",
    not(target_arch = "wasm32")
))]
pub mod h2_async;
/// **VS3-W2** — protocol HTTP/2 dùng chung sync + async (`H2Core`, cap, lỗi).
#[cfg(all(feature = "http2", not(target_arch = "wasm32")))]
pub(crate) mod h2_core;
#[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
pub(crate) mod tls;

/// Kênh đã nối: TCP trần, hoặc TLS (chỉ khi bật feature `tls` trên nền không phải wasm32).
enum Kenh {
    Tho(TcpStream),
    #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
    Tls(Box<tls::KenhTls>),
}

/// **VS3-4b**: một handle đang mở + khoá pool (`tcp|host:port` / `tls|host:port`) để trả về đúng chỗ.
struct Tay {
    ten: String,
    kenh: Kenh,
    khoa: String,
}

/// **VS3-4b**: trần kết nối idle giữ lại cho HTTP keep-alive (tổng, không theo host).
const POOL_TOI_DA: usize = 8;
/// **VS3-4b**: kết nối idle quá hạn bị đóng khi lần kế tiếp chạm pool.
const POOL_IDLE: Duration = Duration::from_secs(15);

/// Khoá tái dùng: **tách TLS/trần** để không bao giờ trả nhầm kênh.
fn khoa_http(tls: bool, dia_chi: &str) -> String {
    format!("{}|{dia_chi}", if tls { "tls" } else { "tcp" })
}

/// Lỗi h2 đáng thử lại một lần (connection/GOAWAY/RST/EOF sau request GET/HEAD idempotent).
/// Không thử lại lỗi cú pháp/header/body vượt cap/timeout (timeout không có tiền tố này).
#[cfg(all(feature = "http2", not(target_arch = "wasm32")))]
fn h2_nen_thu_lai(e: &str) -> bool {
    e.contains("GOAWAY HTTP/2")
        || e.contains("RST_STREAM HTTP/2")
        || e.contains("lỗi HTTP/2 khi kết nối")
        || e.contains("lỗi HTTP/2 khi response")
        || e.contains("lỗi HTTP/2 khi đọc body")
        || e.contains("lỗi HTTP/2 khi trailer")
        || e.contains("lỗi HTTP/2 khi gửi request")
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
    tay: Vec<Tay>,
    tiep: u32,
    /// **ST-65**: listener đang mở — handle `"L<n>"` (tách khỏi `"C<n>"` của kết nối).
    nghe: Vec<(String, TcpListener)>,
    tiep_nghe: u32,
    /// **VS3-4b**: kết nối HTTP idle (khoá, lúc trả về, kênh) — giới hạn [`POOL_TOI_DA`] +
    /// [`POOL_IDLE`]. Không chứa handle đang dùng; mọi thứ đóng khi `Net` drop.
    ho: Vec<(String, Instant, Kenh)>,
    /// **VS3-4b**: cookie jar nội bộ (không ra đĩa, không API ngôn ngữ).
    cookie: CookieJar,
    /// **VS3-4a**: cấu hình TLS dựng **một lần** cho cả tiến trình; `Err` được giữ lại để mọi lần
    /// gọi HTTPS sau đó báo cùng lý do (không panic, không thử lại vô ích).
    #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
    cau_hinh_tls: Option<Result<tls::CauHinh, String>>,
    /// **VS3-4b2**: đã biết authority có h2? `false` = server không chọn ALPN h2, khỏi thử lại.
    #[cfg(all(feature = "http2", not(target_arch = "wasm32")))]
    h2_biet: std::collections::HashMap<String, bool>,
    /// **VS3-4b3**: phiên h2 dùng lại theo authority (tuần tự, tối đa vài authority).
    #[cfg(all(feature = "http2", not(target_arch = "wasm32")))]
    h2_phien: std::collections::HashMap<String, h2::PhienH2>,
    /// Kết nối TLS đã handshake khi thử h2 nhưng server chọn HTTP/1.1; tầng HTTP/1.1 dùng lại.
    #[cfg(all(feature = "http2", not(target_arch = "wasm32")))]
    h2_http1: Option<(String, tls::KenhTls)>,
}

impl Net {
    pub fn moi(giai_thich: bool) -> Self {
        Self {
            giai_thich,
            tay: Vec::new(),
            tiep: 0,
            nghe: Vec::new(),
            tiep_nghe: 0,
            ho: Vec::new(),
            cookie: CookieJar::new(),
            #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
            cau_hinh_tls: None,
            #[cfg(all(feature = "http2", not(target_arch = "wasm32")))]
            h2_biet: std::collections::HashMap::new(),
            #[cfg(all(feature = "http2", not(target_arch = "wasm32")))]
            h2_phien: std::collections::HashMap::new(),
            #[cfg(all(feature = "http2", not(target_arch = "wasm32")))]
            h2_http1: None,
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
            .find(|t| t.ten == tay)
            .map(|t| &mut t.kenh)
            .ok_or_else(|| format!("tay mạng `{tay}` không tồn tại (chưa nối hoặc đã `close`)"))
    }

    fn giu_k(&mut self, s: Kenh, khoa: String) -> String {
        self.tiep += 1;
        let tay = format!("C{}", self.tiep);
        self.tay.push(Tay {
            ten: tay.clone(),
            kenh: s,
            khoa,
        });
        tay
    }

    fn giu(&mut self, s: TcpStream, khoa: String) -> String {
        self.giu_k(Kenh::Tho(s), khoa)
    }

    /// **VS3-4b**: bỏ kết nối idle quá [`POOL_IDLE`] và cắt về [`POOL_TOI_DA`]. Gọi ở **mọi** lần
    /// chạm pool (lấy/trả) nên không cần thread dọn riêng; kênh bị bỏ tự đóng khi drop.
    fn don_ho(&mut self) {
        self.ho.retain(|(_, luc, _)| luc.elapsed() <= POOL_IDLE);
        while self.ho.len() > POOL_TOI_DA {
            self.ho.remove(0);
        }
    }
}

/// Tách `host:port`/`[IPv6]:port`; trả host + cổng **chưa** phân giải.
fn tach_dia_chi(dia_chi: &str) -> Result<(&str, u16), String> {
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

/// **Chỉ loopback** (VD-100/105 (4)): host phải là `localhost`/`127.0.0.0/8`/`::1`, và **tất cả**
/// địa chỉ phân giải được cũng phải loopback. Trả **mọi** địa chỉ để caller thử lần lượt (Windows
/// thường trả `::1` trước `127.0.0.1` cho `localhost`).
fn chi_loopback(dia_chi: &str) -> Result<Vec<SocketAddr>, String> {
    let (host, cong) = tach_dia_chi(dia_chi)?;
    let ten_loopback = host == "localhost"
        || host
            .parse::<IpAddr>()
            .map(|ip| ip.is_loopback())
            .unwrap_or(false);
    if !ten_loopback {
        return Err(format!(
            "chỉ cho phép địa chỉ loopback (`127.0.0.0/8`, `::1`, `localhost`), được `{host}`"
        ));
    }
    let mut ds: Vec<SocketAddr> = dia_chi
        .to_socket_addrs()
        .map_err(|e| format!("địa chỉ `{dia_chi}` không phân giải được: {e}"))?
        .filter(|a| a.ip().is_loopback())
        .collect();
    // Windows có thể trả `::1` trước/không trả 127 cho `localhost`; vẫn phải nối được server
    // bind `127.0.0.1` (ca release blocker). Chỉ thêm địa chỉ loopback.
    if host == "localhost" {
        let v4 = SocketAddr::from(([127, 0, 0, 1], cong));
        if !ds.contains(&v4) {
            ds.push(v4);
        }
    }
    // Ưu tiên IPv4 để Windows/`localhost` không phải chờ `::1` bị từ chối trước.
    ds.sort_by_key(|a| !a.is_ipv4());
    if ds.is_empty() {
        return Err(format!(
            "địa chỉ `{dia_chi}` không có kết quả phân giải loopback"
        ));
    }
    Ok(ds)
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
        let ds = chi_loopback(dia_chi)?;
        let han = Instant::now() + Duration::from_millis(timeout_ms.max(1));
        let mut loi: Option<std::io::Error> = None;
        for addr in ds {
            let Some(con) = han.checked_duration_since(Instant::now()) else {
                break;
            };
            match TcpStream::connect_timeout(&addr, con.max(Duration::from_millis(1))) {
                Ok(s) => {
                    let _ = s.set_nodelay(true);
                    return Ok(self.giu(s, khoa_http(false, dia_chi)));
                }
                Err(e) => loi = Some(e),
            }
        }
        let e = loi.ok_or_else(|| format!("quá thời gian chờ nối tới `{dia_chi}`"))?;
        Err(if la_het_gio(&e) {
            format!("quá thời gian chờ nối tới `{dia_chi}`")
        } else if e.kind() == std::io::ErrorKind::ConnectionRefused {
            format!("bị từ chối kết nối tới `{dia_chi}`: {e}")
        } else {
            format!("lỗi nối tới `{dia_chi}`: {e}")
        })
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
        Ok(self.giu_k(Kenh::Tls(Box::new(k)), khoa_http(true, dia_chi)))
    }

    #[cfg(not(all(feature = "tls", not(target_arch = "wasm32"))))]
    fn connect_tls(&mut self, dia_chi: &str, _timeout_ms: u64) -> Result<String, String> {
        Err(format!(
            "`https://` cần TLS: feature `tls` của `vis-cli` phải bật và nền không phải wasm32 (đang gọi `{dia_chi}`)"
        ))
    }

    /// **VS3-4b**: lấy kết nối idle đúng `scheme|host:port` nếu có; nếu không thì mở mới. Trả cờ
    /// `tái_sử_dụng` để tầng HTTP biết có được thử lại một lần khi kết nối cũ đã bị server đóng.
    fn connect_http(
        &mut self,
        dia_chi: &str,
        tls: bool,
        cho_phep_pool: bool,
        timeout_ms: u64,
    ) -> Result<(String, bool), String> {
        let khoa = khoa_http(tls, dia_chi);
        // **VS3-4b2**: nếu vừa thử ALPN h2 mà server chọn HTTP/1.1, dùng lại đúng kết nối TLS đó.
        #[cfg(all(feature = "http2", not(target_arch = "wasm32")))]
        {
            let dung = self.h2_http1.as_ref().is_some_and(|(k, _)| k == &khoa);
            if dung {
                let (_, kenh) = self.h2_http1.take().expect("đã kiểm Some");
                return Ok((self.giu_k(Kenh::Tls(Box::new(kenh)), khoa), false));
            }
            self.h2_http1 = None;
        }
        if cho_phep_pool {
            self.don_ho();
            if let Some(i) = self.ho.iter().rposition(|(k, _, _)| k == &khoa) {
                let (_, _, kenh) = self.ho.remove(i);
                self.nhat_ky("connect_http_reuse", dia_chi);
                return Ok((self.giu_k(kenh, khoa), true));
            }
        }
        let tay = if tls {
            self.connect_tls(dia_chi, timeout_ms)?
        } else {
            self.connect(dia_chi, timeout_ms)?
        };
        Ok((tay, false))
    }

    fn tra_http(&mut self, tay: &str, giu_lai: bool) -> Result<(), String> {
        let i =
            self.tay.iter().position(|t| t.ten == tay).ok_or_else(|| {
                format!("tay mạng `{tay}` không tồn tại (chưa nối hoặc đã `close`)")
            })?;
        let t = self.tay.remove(i);
        if giu_lai {
            self.ho.push((t.khoa, Instant::now(), t.kenh));
            self.don_ho();
        }
        Ok(())
    }

    fn cookie_jar(&mut self) -> Option<&mut CookieJar> {
        Some(&mut self.cookie)
    }

    /// **VS3-4b2/4b3**: thử/tái dùng phiên ALPN h2; server chọn h1 ⇒ giữ kết nối TLS cho
    /// `connect_http` dùng lại. Request lỗi ⇒ bỏ phiên rồi thử mở phiên mới một lần (GET/HEAD
    /// idempotent).
    #[cfg(all(feature = "http2", not(target_arch = "wasm32")))]
    fn http2_get(
        &mut self,
        dia_chi: &str,
        target: &str,
        method: &str,
        headers: &[(String, String)],
        timeout_ms: u64,
    ) -> Result<Option<vis_core::host::KetQuaHttp2>, String> {
        const PHEN_TOI_DA: usize = 4;
        let khoa = khoa_http(true, dia_chi);
        if self.h2_biet.get(&khoa) == Some(&false) {
            return Ok(None);
        }
        // **VS3-4b4**: đóng mọi phiên h2 đã idle quá 15 s trước khi tái dùng.
        let now = Instant::now();
        self.h2_phien.retain(|_, p| !p.qua_idle(now));
        let mut da_thu = false;
        loop {
            if let Some(phien) = self.h2_phien.get_mut(&khoa) {
                match phien.request(target, method, headers, timeout_ms) {
                    Ok(kq) => return Ok(Some(kq)),
                    Err(e) => {
                        // Connection cũ đã chết/GOAWAY: bỏ rồi mở phiên mới bên dưới.
                        self.h2_phien.remove(&khoa);
                        if da_thu || !h2_nen_thu_lai(&e) {
                            return Err(e);
                        }
                        da_thu = true;
                    }
                }
            }
            let cau_hinh = match self.cau_hinh_tls.get_or_insert_with(tls::mac_dinh) {
                Ok(c) => c.clone(),
                Err(e) => return Err(e.clone()),
            };
            let cau_hinh = tls::cau_hinh_h2(&cau_hinh);
            match h2::PhienH2::moi(&cau_hinh, dia_chi, timeout_ms)? {
                h2::MoH2::Phien(mut phien) => {
                    match phien.request(target, method, headers, timeout_ms) {
                        Ok(kq) => {
                            self.h2_biet.insert(khoa.clone(), true);
                            self.h2_phien.insert(khoa, phien);
                            while self.h2_phien.len() > PHEN_TOI_DA {
                                let Some(k) = self.h2_phien.keys().next().cloned() else {
                                    break;
                                };
                                self.h2_phien.remove(&k);
                            }
                            return Ok(Some(kq));
                        }
                        Err(e) => {
                            if !da_thu && h2_nen_thu_lai(&e) {
                                da_thu = true;
                                continue;
                            }
                            return Err(e);
                        }
                    }
                }
                h2::MoH2::Http1(k) => {
                    self.h2_biet.insert(khoa.clone(), false);
                    self.h2_http1 = Some((khoa, *k));
                    return Ok(None);
                }
            }
        }
    }

    fn listen(&mut self, dia_chi: &str) -> Result<String, String> {
        let mut ds = chi_loopback(dia_chi)?;
        let addr = ds.remove(0);
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
        Ok(self.giu(s, format!("accept|{tay}")))
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
        let mut buf = vec![0u8; toi_da.min(vis_core::reactor::NET_READ_CAP as usize)];
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
        if let Some(i) = self.tay.iter().position(|t| t.ten == tay) {
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

#[cfg(test)]
mod tests {
    use super::{chi_loopback, Kenh, Net, POOL_IDLE, POOL_TOI_DA};
    use std::net::{TcpListener, TcpStream};
    use std::time::{Duration, Instant};

    fn kenh_moi() -> Kenh {
        let l = TcpListener::bind("127.0.0.1:0").expect("bind");
        let s = TcpStream::connect(l.local_addr().expect("addr")).expect("connect");
        let _ = l.accept();
        Kenh::Tho(s)
    }

    /// **Windows/localhost**: trả **mọi** địa chỉ loopback (IPv4 trước) và chặn host không loopback.
    #[test]
    fn chi_loopback_tra_moi_dia_chi_va_chan_remote() {
        let ds = chi_loopback("localhost:80").expect("localhost loopback");
        assert!(!ds.is_empty());
        assert!(
            ds.iter().all(|a| a.ip().is_loopback()),
            "chỉ loopback: {ds:?}"
        );
        assert!(
            ds.iter()
                .any(|a| a.ip() == std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
            "localhost phải có 127.0.0.1: {ds:?}"
        );
        if ds.iter().any(|a| a.is_ipv4()) {
            assert!(ds[0].is_ipv4(), "phải ưu tiên IPv4: {ds:?}");
        }
        assert!(chi_loopback("[::1]:80")
            .expect("IPv6 literal")
            .iter()
            .all(|a| a.ip().is_loopback()));
        assert!(
            chi_loopback("127.0.0.2:80").is_ok(),
            "127.0.0.0/8 là loopback"
        );
        assert!(chi_loopback("127.0.0.1:99999").is_err());
        assert!(chi_loopback("example.com:80").is_err());
        assert!(chi_loopback("10.0.0.1:80").is_err());
        assert!(chi_loopback("[::2]:80").is_err());
    }

    /// **VS3-4b**: kết nối idle quá hạn bị bỏ; pool bị cắt về trần.
    #[test]
    fn pool_het_han_va_gioi_han() {
        let mut net = Net::moi(false);
        if let Some(cu) = Instant::now().checked_sub(POOL_IDLE + Duration::from_secs(1)) {
            net.ho.push(("tcp|cu".to_string(), cu, kenh_moi()));
            net.don_ho();
            assert!(net.ho.is_empty(), "kết nối quá hạn phải bị bỏ");
        }
        for i in 0..POOL_TOI_DA + 2 {
            net.ho
                .push((format!("tcp|h{i}"), Instant::now(), kenh_moi()));
        }
        net.don_ho();
        assert_eq!(net.ho.len(), POOL_TOI_DA, "pool phải bị cắt về trần");
    }
}
