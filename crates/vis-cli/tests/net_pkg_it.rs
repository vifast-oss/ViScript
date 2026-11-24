// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296a** — binary gộp miền: Mạng HTTP + registry/package (`vis pkg`).
//!
//! Các tệp ca gốc nằm trong thư mục cùng tên; `#[path]` giữ nguyên nội dung, không copy test.
//! Tổng số ca/assertion không đổi — chỉ đổi crate test binary.

mod common;

#[path = "net_pkg_it/http.rs"]
mod http;
#[path = "net_pkg_it/net.rs"]
mod net;
#[path = "net_pkg_it/net_server.rs"]
mod net_server;
#[path = "net_pkg_it/pkg.rs"]
mod pkg;
#[path = "net_pkg_it/pkg_lock.rs"]
mod pkg_lock;
#[path = "net_pkg_it/pkg_resolver.rs"]
mod pkg_resolver;
#[path = "net_pkg_it/siet_caps_pkg.rs"]
mod siet_caps_pkg;
#[path = "net_pkg_it/vs3_cookie_auth.rs"]
mod vs3_cookie_auth;
#[path = "net_pkg_it/vs3_default.rs"]
mod vs3_default;
#[path = "net_pkg_it/vs3_keepalive.rs"]
mod vs3_keepalive;
#[path = "net_pkg_it/vs3_pkg_gc.rs"]
mod vs3_pkg_gc;
#[path = "net_pkg_it/vs3_pkg_gc_multi.rs"]
mod vs3_pkg_gc_multi;
#[path = "net_pkg_it/vs3_registry.rs"]
mod vs3_registry;
