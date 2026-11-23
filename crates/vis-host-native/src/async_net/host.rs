// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-1d/W1f** — proxy `NetHost` + tiện ích địa chỉ loopback của [`super::AsyncNet`].
//! Tách khỏi `async_net.rs` để giữ luật R1 (≤ 1500 dòng/tệp).

use std::cell::RefCell;
use std::net::{SocketAddr, ToSocketAddrs};
use std::rc::Rc;

use vis_core::host::NetHost;
use vis_core::http_client::CookieJar;

use super::AsyncNet;

/// Proxy `NetHost` chia sẻ cùng `AsyncNet` với backend đã gắn vào scheduler.
pub struct AsyncNetHost {
    pub inner: Rc<RefCell<AsyncNet>>,
    /// **VS3-W2** — cookie jar của đường async (sync có jar riêng trong `HostNet`).
    pub cookie: CookieJar,
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
    fn cookie_jar(&mut self) -> Option<&mut CookieJar> {
        Some(&mut self.cookie)
    }
}

/// Resolve **loopback only** (giữ chính sách VD-100/105; async net không mở Internet).
pub(super) fn loopback_addr(dia_chi: &str) -> Result<SocketAddr, String> {
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

pub(super) fn tach(dia_chi: &str) -> Result<(&str, u16), String> {
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
