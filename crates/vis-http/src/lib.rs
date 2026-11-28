// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-64 (VD-100/VD-105)** — **giao thức HTTP client** nằm trong `vis-http` (không ở CLI):
//! socket thô qua [`NetHost`], tự dựng/đọc **HTTP/1.1**. Nhờ vậy interp ↔ VM **cùng byte** và codegen
//! **không** phải sửa (`std.http.*` hạ theo tên như mọi `std.*`).
//!
//! Phạm vi: `http://` **chỉ loopback** (`localhost`/`127.0.0.0/8`/`::1`, giữ nguyên VD-100/105) và
//! **VS3-4a** `https://` qua `NetHost::connect_tls` (TLS nằm ở host, không ở crate lõi); **không**
//! HTTP/2; GET theo redirect tối đa 5 (chỉ GET), HEAD **không** theo redirect; phản hồi cap
//! **8 MiB** (mẫu `MAX_STDIN`, ST-34) ⇒ vượt cap là `Err`, không OOM. Body giữ **byte** tới khi
//! chuyển `str` (lossy) — ngôn ngữ chưa có kiểu byte. **Chunk trailer được đọc hết nhưng bỏ nội dung**.
//!
//! **VS3-4a (VD-271)**: thêm `https://` — host TLS xác thực chứng chỉ theo root store; `https://`
//! được phép host ngoài loopback (đi qua `cap[Net]` như mọi host khác), còn `http://` **vẫn** chỉ
//! loopback để không mở thêm bề mặt trần. Không cho chuyển hướng **HTTPS → HTTP** (Q2 (3)).
//!
//! **VS3-4b (VD-271)**: HTTP/1.1 **keep-alive** — request gửi `Connection: keep-alive`; response
//! cho phép (HTTP/1.1 không `close`, hoặc HTTP/1.0 + `keep-alive`) và thân đã đọc hết ⇒ trả kết nối
//! về host (`NetHost::tra_http`) để tái dùng; server `close`/EOF/khung lạ ⇒ đóng. Host không có pool
//! (mặc định của trait) vẫn one-shot. Kết nối idle chết được thử lại **một lần** trên kết nối mới
//! (GET/HEAD idempotent), trong cùng deadline.

// Giữ `[`NetHost`]` trong doc crate-root phân giải được sau khi tách client sang module riêng.
#[allow(unused_imports)]
use vis_core::host::NetHost;

/// Cap **phản hồi** (8 MiB, mẫu `MAX_STDIN` của ST-34) — vượt ⇒ `Err`, không OOM.
pub const MAX_HTTP: usize = 8 * 1024 * 1024;
/// Cap **header** (64 KiB) — header lớn hơn ⇒ `Err` (không đọc vô hạn).
#[doc(hidden)]
pub const MAX_HEADER: usize = 64 * 1024;

mod client;
mod header;
mod request;
mod url;

pub use client::{http_get, http_head};

#[cfg(test)]
pub(crate) use header::doc_header;
pub(crate) use header::{con_lai_ms, doc_header_day_du, ghep_chunked, tim_header, DauHttp};
#[cfg(test)]
pub(crate) use request::chuan_hoa_target;
pub(crate) use request::{header_h2, noi_location};
pub(crate) use url::{tach_url, GiaoThuc, UrlHttp};

#[cfg(test)]
mod tests;

pub mod async_http;
pub use async_http::{http_get_async, http_head_async, xoa_trang_thai_frame, AsyncHttpEnv};
pub use vis_core::reactor::{FrameReactorBackend, ReactorH2Response, ReactorOp};
