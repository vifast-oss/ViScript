// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-4a/4b (VD-271)** — cầu `cap[Net]` cho mọi thao tác mạng của CLI + parser `host:port`.
//!
//! Tách khỏi `quyen.rs` (R1 ≤ 1500 dòng) nhưng vẫn là một phần của cùng cơ chế quyền: `NetGuard`
//! kiểm `cap[Net]` **trước** khi gọi host thật, kể cả `connect_tls`, `connect_http` và `tra_http`.
//! `host_cua` chuẩn hoá `[::1]:443`, `[::1]`, `::1` về cùng một host `::1` để so với `cap[Net]`;
//! **không** tự coi `::1` là loopback (không nới quyền).

use vis_core::host::NetHost;

use crate::quyen::Quyen;

/// Bọc `NetHost` để kiểm `cap[Net]`.
pub struct NetGuard<'a> {
    pub trong: &'a mut dyn NetHost,
    pub quyen: &'a Quyen,
}

impl NetHost for NetGuard<'_> {
    fn connect(&mut self, dia_chi: &str, timeout_ms: u64) -> Result<String, String> {
        self.quyen.kiem_net(dia_chi)?;
        self.trong.connect(dia_chi, timeout_ms)
    }
    /// **VS3-4a (VD-271)**: TLS dùng cùng cổng `cap[Net]` trên `host:port` như TCP trần.
    fn connect_tls(&mut self, dia_chi: &str, timeout_ms: u64) -> Result<String, String> {
        self.quyen.kiem_net(dia_chi)?;
        self.trong.connect_tls(dia_chi, timeout_ms)
    }
    /// **VS3-1d/W1f-HTTP**: đường async tự gọi reactor nên CLI kiểm `cap[Net]` qua hook này
    /// trước khi mở kết nối; không thực hiện I/O.
    fn check_net(&mut self, dia_chi: &str) -> Result<(), String> {
        self.quyen.kiem_net(dia_chi)
    }

    /// **VS3-4b**: kể cả lấy kết nối từ pool, mỗi lần `connect_http` vẫn kiểm `cap[Net]` trên host.
    fn connect_http(
        &mut self,
        dia_chi: &str,
        tls: bool,
        cho_phep_pool: bool,
        timeout_ms: u64,
    ) -> Result<(String, bool), String> {
        self.quyen.kiem_net(dia_chi)?;
        self.trong
            .connect_http(dia_chi, tls, cho_phep_pool, timeout_ms)
    }
    /// Trả kết nối HTTP về host; không cần kiểm cap lần nữa (đã kiểm lúc mở).
    fn tra_http(&mut self, tay: &str, giu_lai: bool) -> Result<(), String> {
        self.trong.tra_http(tay, giu_lai)
    }
    fn cookie_jar(&mut self) -> Option<&mut vis_core::http_client::CookieJar> {
        self.trong.cookie_jar()
    }
    /// **VS3-4b2**: HTTP/2 cũng là truy cập mạng ⇒ kiểm `cap[Net]` trước khi host mở TLS.
    #[cfg(all(feature = "http2", not(target_arch = "wasm32")))]
    fn http2_get(
        &mut self,
        dia_chi: &str,
        target: &str,
        method: &str,
        headers: &[(String, String)],
        timeout_ms: u64,
    ) -> Result<Option<vis_core::host::KetQuaHttp2>, String> {
        self.quyen.kiem_net(dia_chi)?;
        self.trong
            .http2_get(dia_chi, target, method, headers, timeout_ms)
    }
    fn listen(&mut self, dia_chi: &str) -> Result<String, String> {
        self.quyen.kiem_net(dia_chi)?;
        self.trong.listen(dia_chi)
    }
    fn local_addr(&mut self, tay: &str) -> Result<String, String> {
        self.trong.local_addr(tay)
    }
    fn accept(&mut self, tay: &str, timeout_ms: u64) -> Result<String, String> {
        self.trong.accept(tay, timeout_ms)
    }
    fn read(&mut self, tay: &str, toi_da: usize, timeout_ms: u64) -> Result<Vec<u8>, String> {
        self.trong.read(tay, toi_da, timeout_ms)
    }
    fn write(&mut self, tay: &str, du_lieu: &[u8]) -> Result<(), String> {
        self.trong.write(tay, du_lieu)
    }
    fn close(&mut self, tay: &str) -> Result<(), String> {
        self.trong.close(tay)
    }
    fn ghi_nhat_ky(&mut self, dong: &str) {
        self.trong.ghi_nhat_ky(dong);
    }
}

/// Lấy **host** thuần từ `host`, `host:port`, `[IPv6]`, `[IPv6]:port` — không cấp quyền, chỉ tách.
///
/// - `[::1]:443` / `[::1]` → `::1`
/// - `::1` (IPv6 trần, không cổng) → `::1`
/// - `example.com:443` → `example.com`
/// - chuỗi hỏng `[::1` → giữ nguyên (không khớp cap để tránh đoán sai).
pub(crate) fn host_cua(dia_chi: &str) -> &str {
    let s = dia_chi.trim();
    if let Some(rest) = s.strip_prefix('[') {
        return match rest.split_once(']') {
            Some((host, _)) => host,
            None => s,
        };
    }
    if s.matches(':').count() == 1 {
        if let Some((host, _)) = s.rsplit_once(':') {
            return host;
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::{host_cua, NetGuard};
    use crate::quyen::{Quyen, QuyenCap};
    use std::path::PathBuf;
    use vis_core::host::NetHost;

    fn quyen_co_net(roots: &[&str]) -> Quyen {
        Quyen {
            moc: roots
                .iter()
                .enumerate()
                .map(|(i, root)| QuyenCap {
                    pos: i as u32,
                    kind: "Net".to_string(),
                    root: (*root).to_string(),
                    write: false,
                    allow: Vec::new(),
                })
                .collect(),
            bat: true,
            siet: true,
            goc: PathBuf::new(),
            allow_proc: None,
        }
    }

    #[test]
    fn host_cua_tach_dung_ipv4_ipv6_va_cong() {
        assert_eq!(host_cua("127.0.0.1:8080"), "127.0.0.1");
        assert_eq!(host_cua("localhost"), "localhost");
        assert_eq!(host_cua("[::1]:443"), "::1");
        assert_eq!(host_cua("[::1]"), "::1");
        assert_eq!(host_cua("::1"), "::1");
        assert_eq!(host_cua("example.com:443"), "example.com");
        assert_eq!(host_cua("[::1"), "[::1");
    }

    /// **VS3-4b**: `cap[Net]` khai `::1`/`[::1]` phải khớp `[::1]:443` và `::1`; **không** nới
    /// quyền: không có cap thì IPv6 vẫn bị từ chối (không tự coi là loopback).
    #[test]
    fn kiem_net_khop_cap_ipv6_va_khong_noi_quyen() {
        let q = quyen_co_net(&["::1"]);
        assert!(
            q.kiem_net("[::1]:443").is_ok(),
            "cap ::1 phải khớp [::1]:443"
        );
        assert!(q.kiem_net("::1").is_ok(), "cap ::1 phải khớp ::1");
        assert!(q.kiem_net("[::1]").is_ok(), "cap ::1 phải khớp [::1]");
        assert!(
            q.kiem_net("[::2]:443").is_err(),
            "host khác cap phải bị từ chối"
        );

        let q = quyen_co_net(&["[::1]"]);
        assert!(q.kiem_net("[::1]:443").is_ok(), "cap [::1] phải chuẩn hoá");

        let q = quyen_co_net(&[]);
        assert!(
            q.kiem_net("[::1]:443").is_err(),
            "không có cap Net thì IPv6 không được tự cho qua"
        );
        assert!(
            q.kiem_net("127.0.0.1:80").is_err(),
            "không có cap Net thì IPv4 cũng không được tự cho qua"
        );
    }

    /// `NetGuard` chuyển tiếp `connect_tls` + `connect_http`/`tra_http` (thêm ở VS3-4b) qua cùng
    /// một lần kiểm cap; ca này khoá việc guard không bỏ qua cap với TLS.
    #[test]
    fn net_guard_kiem_cap_truoc_khi_connect() {
        struct Gia;
        impl NetHost for Gia {
            fn connect(&mut self, _: &str, _: u64) -> Result<String, String> {
                Ok("C1".to_string())
            }
            fn connect_tls(&mut self, _: &str, _: u64) -> Result<String, String> {
                Ok("C2".to_string())
            }
            fn listen(&mut self, _: &str) -> Result<String, String> {
                Ok("L1".to_string())
            }
            fn local_addr(&mut self, _: &str) -> Result<String, String> {
                Ok("127.0.0.1:1".to_string())
            }
            fn accept(&mut self, _: &str, _: u64) -> Result<String, String> {
                Ok("C3".to_string())
            }
            fn read(&mut self, _: &str, _: usize, _: u64) -> Result<Vec<u8>, String> {
                Ok(Vec::new())
            }
            fn write(&mut self, _: &str, _: &[u8]) -> Result<(), String> {
                Ok(())
            }
            fn close(&mut self, _: &str) -> Result<(), String> {
                Ok(())
            }
        }
        let q = quyen_co_net(&["::1"]);
        let mut trong = Gia;
        let mut g = NetGuard {
            trong: &mut trong,
            quyen: &q,
        };
        assert_eq!(g.connect_tls("[::1]:443", 10).unwrap(), "C2");
        assert!(g.connect_tls("[::2]:443", 10).is_err());
        assert_eq!(g.connect("[::1]:80", 10).unwrap(), "C1");
        let (tay, tai_su_dung) = g.connect_http("[::1]:443", true, true, 10).unwrap();
        assert_eq!(tay, "C2");
        assert!(!tai_su_dung, "host giả không có pool");
        assert!(g.connect_http("[::2]:443", true, true, 10).is_err());
        assert!(g.tra_http("C2", true).is_ok());
    }
}
