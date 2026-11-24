// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310a (VD-285)** — trait **host** dùng chung: chương trình ngoài, hệ tệp, mạng, cơ sở dữ
//! liệu. `vis-core` **không** biết OS; host thật cài các trait này (native/CLI/test).
//!
//! Nguồn gốc lịch sử: `vis-interp/src/builtin.rs`, `builtin_fs.rs`, `builtin_db.rs` (đã tách).
//! Trait `Host` của engine (có `Scheduler`) nay ở `vis-runtime/src/builtin/mod.rs`; `impl Host`
//! cho interp ở `vis-interp/src/host.rs`. `vis-core` chỉ giữ **bề mặt host** (FS/net/db/proc).

use vis_syntax::Span;

use crate::trap::{Abort, Trap, R};
use crate::value::Value;

pub mod db;
pub mod fs;

pub use db::DbHost;
pub use fs::{chia_dong, goi_metadata, DocQuaHandle, DuongDanHandle, FsHost, MucFs};

/// **ST-72b (VD-103/VD-106)**: dấu **cưỡng chế quyền** trong `Err` của host — `hop_nhat_fs` đổi
/// thành trap `VS0604` (không bắt được), thay vì `Err(str)` dữ liệu (vượt quyền là lỗi **môi
/// trường**, không phải dữ liệu chương trình).
pub const DAU_QUYEN: &str = "\u{1}VS0604:";

/// Kết quả **đầy đủ** của một chương trình ngoài (ST-34, cho `| complete`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KetQua {
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
}

pub trait Ngoai {
    /// Chạy `ten` với `args`; `stdin` (ST-34) ghi vào stdin rồi **đóng**;
    /// `raw` ⇒ trả `str` (nguyên văn) thay vì `list[str]`.
    fn chay(&mut self, ten: &str, args: &[String], raw: bool, stdin: &str, span: Span) -> R<Value>;

    /// Chạy và trả **đủ** `stdout/stderr/code` (ST-34) — cho stage `| complete`; exit≠0 **không** là lỗi.
    /// Mặc định: chưa hỗ trợ (host tự cài).
    fn chay_day_du(
        &mut self,
        ten: &str,
        args: &[String],
        stdin: &str,
        span: Span,
    ) -> Result<KetQua, Abort> {
        let _ = (ten, args, stdin);
        Err(Abort::Trap(Trap::new(
            &vis_diag::COMMAND_CODES[3],
            "`complete` chưa hỗ trợ ở host này",
            span,
        )))
    }
}

/// **VS3-4b2 (VD-271)**: kết quả một request HTTP/2 (không phải API ngôn ngữ).
pub struct KetQuaHttp2 {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// **Host mạng** (ST-63, VD-100/VD-105) — `vis-interp`/`vis-vm` **không** biết `std::net`; `vis-cli`
/// cài TCP thật. Thiếu host ⇒ `VS0604` (cùng mẫu [`FsHost`], VD-60 (2)). Handle là **chuỗi mờ** —
/// chương trình không thấy địa chỉ/socket; body **nhị phân** (`Vec<u8>`) để không cắt sai UTF-8.
pub trait NetHost {
    /// Nối TCP tới `dia_chi` (`host:port`), chờ tối đa `timeout_ms`. Trả **handle mờ**.
    fn connect(&mut self, dia_chi: &str, timeout_ms: u64) -> Result<String, String>;
    /// **VS3-4a (VD-271)**: nối **TLS** tới `dia_chi` (`host:port`), xác thực chứng chỉ theo kho
    /// gốc của host, chờ tối đa `timeout_ms` cho cả TCP + handshake. Trả **handle mờ** dùng chung
    /// `read`/`write`/`close` với [`connect`](Self::connect).
    ///
    /// Mặc định **từ chối** (host không có TLS) — lỗi dữ liệu rõ ràng, không panic; host nào hỗ trợ
    /// thì cài đặt. `vis-cli` bật TLS mặc định trên nền không phải wasm32.
    fn connect_tls(&mut self, dia_chi: &str, timeout_ms: u64) -> Result<String, String> {
        let _ = (dia_chi, timeout_ms);
        Err("HTTPS không được hỗ trợ bởi host mạng này".to_string())
    }

    /// **VS3-1d/W1f-HTTP** — kiểm quyền `cap[Net]` **không mở kết nối**. Đường async tự
    /// đăng ký reactor nên phải gọi hook này trước khi connect; host không có policy trả `Ok`.
    fn check_net(&mut self, _dia_chi: &str) -> Result<(), String> {
        Ok(())
    }

    /// **VS3-4b (VD-271)**: nối cho **HTTP** — `tls` chọn `connect_tls`/`connect`,
    /// `cho_phep_pool` cho phép host tái dùng kết nối idle. Trả `(handle, tái_sử_dụng)`; host
    /// không có pool (mặc định) luôn trả `false` và hoạt động one-shot như cũ.
    fn connect_http(
        &mut self,
        dia_chi: &str,
        tls: bool,
        cho_phep_pool: bool,
        timeout_ms: u64,
    ) -> Result<(String, bool), String> {
        let _ = cho_phep_pool;
        let tay = if tls {
            self.connect_tls(dia_chi, timeout_ms)?
        } else {
            self.connect(dia_chi, timeout_ms)?
        };
        Ok((tay, false))
    }

    /// **VS3-4b (VD-271)**: kết thúc một request HTTP. `giu_lai = true` ⇒ host **có thể** đưa kết
    /// nối về pool idle (chỉ khi response cho phép keep-alive); mặc định **đóng** như one-shot cũ.
    fn tra_http(&mut self, tay: &str, giu_lai: bool) -> Result<(), String> {
        let _ = giu_lai;
        self.close(tay)
    }

    /// **VS3-4b (VD-271)**: cookie jar của host (nếu có). Mặc định `None` ⇒ `http.get/head` không
    /// gửi/lưu cookie; host hỗ trợ trả jar để tầng HTTP đọc/ghi. Không phải API ngôn ngữ.
    fn cookie_jar(&mut self) -> Option<&mut crate::cookie::CookieJar> {
        None
    }

    /// **VS3-4b2**: thử một request qua **HTTP/2** nếu host/feature hỗ trợ. `Ok(None)` = host không
    /// dùng h2 (feature tắt, hoặc server không chọn ALPN `h2`) ⇒ tầng HTTP fallback HTTP/1.1.
    /// Mặc định `None`; `vis-cli` cài khi bật feature `http2`.
    fn http2_get(
        &mut self,
        _dia_chi: &str,
        _target: &str,
        _method: &str,
        _headers: &[(String, String)],
        _timeout_ms: u64,
    ) -> Result<Option<KetQuaHttp2>, String> {
        Ok(None)
    }
    /// **ST-65**: lắng nghe trên `dia_chi`. Trả **handle mờ**.
    fn listen(&mut self, dia_chi: &str) -> Result<String, String>;
    /// **ST-65**: địa chỉ local của tay.
    fn local_addr(&mut self, tay: &str) -> Result<String, String>;
    /// **ST-65**: nhận một kết nối, chờ tối đa `timeout_ms`. Trả **handle mờ**.
    fn accept(&mut self, tay: &str, timeout_ms: u64) -> Result<String, String>;
    /// Đọc tối đa `toi_da` **byte**; `Ok(vec![])` = **EOF** (không phải lỗi). Hết giờ ⇒ `Err`.
    fn read(&mut self, tay: &str, toi_da: usize, timeout_ms: u64) -> Result<Vec<u8>, String>;
    /// Ghi cả `du_lieu` (body nhị phân).
    fn write(&mut self, tay: &str, du_lieu: &[u8]) -> Result<(), String>;
    /// Đóng tay; tay lạ ⇒ `Err`.
    fn close(&mut self, tay: &str) -> Result<(), String>;

    /// **ST-64**: nhật ký cho **tầng giao thức** (HTTP) khi `--explain-perms`; mặc định **không in**.
    /// CLI cài để in `NET http GET …` ra stderr; interp/VM không biết stderr.
    fn ghi_nhat_ky(&mut self, _dong: &str) {}
}
