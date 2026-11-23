// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310a (VD-285)** — giao thức HTTP/1.1 đồng bộ chuyển sang
//! `vis-core::http_client`; đường async (reactor frame) ở lại đây.
//!
//! Đường dẫn công khai `vis_interp::net::{http_get, http_head, MAX_HTTP}` giữ nguyên.

pub use vis_core::http_client::{http_get, http_head, CookieJar, MAX_HTTP};

mod async_http;
pub(crate) use async_http::{http_get_async, http_head_async, xoa_trang_thai_frame};
