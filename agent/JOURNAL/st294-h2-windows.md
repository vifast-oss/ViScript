<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-294 — `async-h2` chạy Windows + bật default non-wasm (`v0.4.0`)

> **Người làm**: subagent VS3-W2, 2026-11-22. **Base**: `a48d5b8` (merge ST-290).
> **Tag sản phẩm tại thời điểm soak**: `v0.3.6` / `8fa2c09` (code `0645700`); không tag/push/merge.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| `#![cfg(unix)]` / `AsRawFd` / `RawFd` / `SourceFd` trong `net/h2_async.rs` | **đã gỡ** |
| Adapter socket | `SockChiaSe(Rc<RefCell<mio::net::TcpStream>>)` + `rustls::StreamOwned`; đăng ký trực tiếp `MioStream` (Linux epoll + Windows IOCP/AFD) |
| `async-h2` default | **bật default non-wasm**: `["sqlite","tls","pkg-sig","http2","async-h2"]` |
| Cảnh báo `--no-default-features` (`async_net_nguon.rs`) | **0** — sửa `dem_tay_dang_mo` (shadow) + `allow(dead_code)` cho `dang_reactor_win` |
| Windows `cargo check --features async-h2` | **PASS** (psvn, target riêng `/c/Users/ps/vt-st294-target`) |
| Windows tests | `vs3_http2` **33/33**; `vs3_http_cancel` **8/8** (gồm cold-start 4 stream/1 conn, timeout/GOAWAY/cancel h2) |
| Linux gate | **3607 passed / 0 failed / 4 skipped**, PASS |
| Blocker | **không** (Windows workspace xem §5) |

## 1. Thay đổi

- `crates/vis-cli/src/net/h2_async.rs`:
  - bỏ `#![cfg(unix)]` → `#![cfg(all(feature = "async-h2", feature = "http2", feature = "tls", not(target_arch = "wasm32")))]`;
  - bỏ `std::os::fd::{AsRawFd, RawFd}` + `mio::unix::SourceFd`;
  - `H2Async` giữ `Rc<RefCell<MioStream>>`; `IoH2` bọc `StreamOwned<ClientConnection, SockChiaSe>`;
    `dang_ky_nguon`/`dang_ky_lai_nguon`/`huy_nguon` gọi `poll.registry()` trên chính `mio::net::TcpStream`;
  - `noi_tls` trả `(ClientConnection, MioStream)`; `bap_tay`/`cho_san_sang` nhận `&mut MioStream` —
    cùng một đường cho Linux và Windows, không fd thô.
- `crates/vis-cli/src/async_net_conn.rs`: chuyển `KenhTls` cũ sang `H2Async::moi_cho_bap(conn, st.sock, …)`
  (không đổi socket qua `std::net::TcpStream`).
- `crates/vis-cli/src/async_net_nguon.rs`: dọn 2 cảnh báo `--no-default-features` (non-TLS):
  `dem_tay_dang_mo` dùng shadow `let n = …` thay `mut`; `dang_reactor_win` có `allow(dead_code)` khi tắt TLS.
- `crates/vis-cli/Cargo.toml`: `default = [..., "async-h2"]`; comment cập nhật default-on non-wasm.
  Fallback HTTP/1.1 khi ALPN không chọn h2 **giữ nguyên**.

## 2. Linux

| Lệnh | Kết quả |
|---|---|
| `cargo test --test vs3_http2` (default, có async-h2) | **34 passed / 1 ignored** (16 ca h2 async) |
| `cargo test --test vs3_http2 --no-default-features --features http2` | **19/19** (sync h2 như trước) |
| `cargo test --test vs3_http_cancel` (default, có async-h2) | **8/8** |
| `cargo test --test vs3_http_cancel --no-default-features --features tls` | **5/5** (h1) |
| `cargo check --no-default-features` / clippy all-targets | **PASS, 0 cảnh báo** |
| `./vis-dev gate --fast` | **PASS — 3607 passed / 0 failed / 4 skipped** (xem §4) |

**Gate count**: trước ST-294 gate là **3589/3**; bật `async-h2` default làm chạy thêm
16 ca `vs3_http2` async + 3 ca h2 `vs3_http_cancel` (và 1 ca `#[ignore]` h2 được đếm skip) ⇒
**3607/4**. Không thêm test mới; đây là hệ quả trực tiếp của yêu cầu default-on.

## 3. Windows (psvn, Tier 2)

- `cargo check -p vis-cli --features async-h2`: **PASS** (`/c/Users/ps/vt-st294-target`).
- `cargo test -p vis-cli --test vs3_http2 --test vs3_http_cancel`:
  - `vs3_http2` **33/33** (Windows bỏ 2 ca Unix-only so Linux 35);
  - `vs3_http_cancel` **8/8**;
  - gồm `async_h2_production_bon_stream_mot_connection`, `…_cold`,
    `async_h2_cold_multiplex_20_vong`, timeout/GOAWAY/cancel h2.
- Trước ST-294, `cargo check --features async-h2` trên Windows **rc 101** `E0433 net::h2_async`;
  nay đã pass, không còn `SourceFd`/fd Unix.

## 4. Ghi chú

- Không `unsafe` mới; không crate mới; `mio`/rustls/h2 dùng lại dependency đã duyệt.
- wasm32: default đầy đủ vẫn cần `clang` cho ring (môi trường local thiếu); `cargo check
  --target wasm32-unknown-unknown --no-default-features` **PASS**. Module `h2_async` vẫn
  `not(target_arch = "wasm32")`.
- Windows workspace: xem §5.
