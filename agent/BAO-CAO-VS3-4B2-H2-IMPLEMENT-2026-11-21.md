<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-4b2 — dependency review `h2` + TLS/ALPN (implement, 2026-11-21)

> **Phạm vi**: feature `http2` của `vis-cli`, **default-off**; A0 pre-approve `h2 =0.4.20` + transitive.
> **Kết quả**: client h2 tối thiểu chạy hermetic; HTTP/1.1 vẫn mặc định; không unsafe trong mã dự án.

## 1. Trực tiếp (chỉ feature `http2`)

| Crate | Version | License | Vai trò |
|---|---|---|---|
| `h2` | `=0.4.20` | MIT | framing/stream HTTP/2 client+server cho test |
| `tokio` | `=1.54.1` | MIT | **chỉ** `AsyncRead`/`AsyncWrite` (`io-util`), **không** bật `rt`/`net`; executor tự viết |
| `http` | `=1.5.0` | MIT OR Apache-2.0 | `Request`/`Response`/`HeaderMap` |
| `bytes` | `=1.12.2` | MIT | `Bytes` body của h2 |

Cả bốn nằm trong `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]` + `optional = true`;
`http2 = ["tls", "dep:h2", "dep:tokio", "dep:http", "dep:bytes"]`, **không** nằm trong `default`.
`cargo tree --target wasm32 ... --features http2` không thấy h2/tokio/http/bytes.

## 2. Transitive của `h2` (normal)

| Crate | License | Unsafe (ghi nhận) |
|---|---|---|
| `atomic-waker` | Apache-2.0 OR MIT | có (waker) |
| `bytes` | MIT | có (slice/ptr) |
| `fnv` | Apache-2.0/MIT | 0 |
| `futures-core` / `futures-sink` | MIT OR Apache-2.0 | có nội bộ |
| `http` | MIT OR Apache-2.0 | có nội bộ |
| `itoa` | MIT OR Apache-2.0 | có nội bộ |
| `indexmap` / `equivalent` / `hashbrown` | Apache-2.0/MIT | `hashbrown` nhiều unsafe |
| `slab` | MIT | có nội bộ |
| `tokio` | MIT | nhiều unsafe (runtime/io) |
| `tokio-util` | MIT | có nội bộ |
| `pin-project-lite` | Apache-2.0 OR MIT | macro/unsafe |
| `tracing` / `tracing-core` | MIT | có nội bộ |
| `once_cell` | MIT OR Apache-2.0 | đã có trong cây (rustls) |

Không GPL/AGPL; toàn bộ permissive. `h2` có **1 unsafe thật** (`from_utf8_unchecked` HPACK);
`tokio`/`bytes`/`hashbrown` có unsafe nội bộ. **Mã dự án**: `vis-cli/src/lib.rs` giữ
`#![forbid(unsafe_code)]`; `net/h2.rs` không unsafe; R2 `./vis-dev rules` xanh.

## 3. Thiết kế / giới hạn

- TLS: `tls::cau_hinh_h2` clone cấu hình hiện có, đặt ALPN `["h2","http/1.1"]`; client bình thường
  **không** đặt ALPN. Server không chọn h2 ⇒ trả **cùng** `KenhTls` cho HTTP/1.1 (không mở thêm
  kết nối, không đốt slot server test).
- Executor: adapter chặn/nonblocking (socket nonblocking + `Pending`) + vòng `poll`/`yield` một
  luồng; **không** tokio runtime, không timer, không thread nền.
- Một request/kết nối h2; không multiplex/pool h2 (mở rộng sau, không hồi quy h1). GET/HEAD; body
  cap 8 MiB; timeout chung với connect/handshake; cookie/auth vẫn do tầng HTTP xử lý (header được
  truyền vào h2).
- H2 lỗi/protocol sai ⇒ `Err` rõ; server không ALPN h2 ⇒ fallback h1. Không HTTP/2 push/trailers.

## 4. Test

`crates/vis-cli/tests/vs3_http2.rs` (feature `http2`): h2 server local TLS + ALPN h2 (6 ca):
- `h2_get_va_head`: GET body/header, HEAD status/header;
- `h2_redirect_va_cookie`: 302 + Set-Cookie → hop sau gửi Cookie;
- `h2_basic_auth`: `user:pass@` → Authorization Basic;
- `fallback_http1_khi_khong_alpn`: TLS server không ALPN → dùng lại kết nối, h1 chạy;
- `h2_timeout_ro_rang`: timeout rõ;
- `registry_https_thuong_luong_h2`: `vis pkg add` qua registry TLS h2 (index+tarball, SHA-256).
Kèm default `vs3_tls`/`vs3_registry_https` dưới feature h2 (fallback) và toàn bộ test h1.

## 5. Kết luận

| Mục | Kết quả |
|---|---|
| License | MIT/Apache/BSD-style; không GPL |
| Unsafe | h2 1 + transitive; mã dự án 0 (`forbid`) |
| Feature | `http2` default-off, non-wasm; `h2`/tokio không vào default |
| H1 | giữ nguyên mặc định + test cũ (kể cả khi bật feature, fallback cùng connection) |
| Blocker | không; h2 chỉ tối thiểu (một request/kết nối, không multiplex) |
