<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W2 — tích hợp HTTP/2 async reactor-native (feature `async-h2`, default-off)

**Trạng thái**: **ĐÃ NỐI production async HTTP** (checkpoint 1–3 xanh) + test production
(checkpoint 4); feature `async-h2` **default-off**, không push/merge/tag, không dependency mới,
không đổi API sync.
**Nhánh**: `vs3/w2-tls` · merge `main` `73af17b` (fast-forward tới `5e61898` + merge commit).
**Log**: [`vs3-w2-async-h2-integration.log`](vs3-w2-async-h2-integration.log).

## 1. Kết quả theo checkpoint

| # | Nội dung | Kết quả |
|---|---|---|
| 1 | Tách `net/h2_core.rs` dùng chung sync+async | **PASS** — sync `vs3_http2` **19/19**; spike cũ **25 pass**; không đổi cap/hành vi sync |
| 2 | `AsyncNet`/`FrameReactorBackend`/ALPN/`H<n>` | **PASS** — `ReactorOp::ReadWrite`, structured meta, fd dùng chung nhiều token, fallback h1 |
| 3 | `http.get/head` async qua h2 | **PASS** — meta `status/headers/body`, cap 8 MiB, timeout/GOAWAY/RST, redirect, auth/cookie header |
| 4 | Test production 4 stream/1 connection | **PASS** — 4 task `spawn` cùng authority trên **một** connection; thêm timeout/GOAWAY/cap/redirect |

## 2. Checkpoint 1 — `h2_core` dùng chung

- `crates/vis-cli/src/net/h2_core.rs` (mới): `H2Core<T: AsyncRead+AsyncWrite+Unpin>`, `Song`,
  `MAX_HEADER`, `loi_h2`. Chỉ protocol: HPACK/framing/flow-control của crate `h2`, cap/lỗi/trailer.
- `net/h2.rs` sync thành wrapper: `PhienH2 { core: H2Core<DongBo>, han, luc_dung }`; `request`
  = `core.gui` + `core.buoc` + `core.ket_qua`; `gui_nhieu` test-only dùng cùng lõi.
- `net/h2_async.rs` async thành wrapper: `H2Core<IoH2>` + adapter reactor + `mio`; thêm
  `moi_cho_bap` (TLS xong, h2 handshake chưa chạy) và **xếp request** trước handshake.
- Feature đổi tên `h2-async-spike` → **`async-h2`**, vẫn **default-off**.
- Sync giữ nguyên cap 64 KiB/8 MiB, timeout, GOAWAY retry (`GOAWAY HTTP/2…`).

**Số đo**: sync `vs3_http2` 19/19 (15,6 s); feature 25 pass + 1 ignored.

## 3. Checkpoint 2 — reactor/backend

### 3.1 `vis-interp/src/rt.rs`

- `ReactorOp::ReadWrite` (READABLE|WRITABLE theo bước).
- `ReactorH2Response { status, headers, body }` + `ReactorH2Outcome { WouldBlock, Response, Err }`
  — core nhận **meta có cấu trúc**, không nhồi HTTP/1.1.
- Trait `FrameReactorBackend` thêm default method `h2_ready/h2_send/h2_poll/h2_cancel` (adapter
  test cũ không phải sửa; backend không hỗ trợ trả lỗi rõ).
- Tách `frame_real_wait` dùng chung cho mọi op; `frame_reactor_h2_poll`/`h2_send`/`h2_cancel`
  đăng ký bằng `ReactorOp::ReadWrite`.

### 3.2 `AsyncNet` (`crates/vis-cli/src/async_net.rs`)

- `H<n>` = connection h2; mỗi frame nhận `H<n>#<frame>` để có khoá reactor riêng nhưng chung
  session. `h2_base()` chuẩn hoá khi gọi backend/close.
- TLS async đặt ALPN `["h2","http/1.1"]` (`cau_hinh_h2`) khi feature bật; ALPN h2 ⇒ tạo
  `H2Async` + handle `H<n>`; còn lại ⇒ `C<n>` (HTTP/1.1) như cũ, đánh dấu authority h1 để các
  connect sau dùng khoá riêng từng frame (mỗi task một socket h1).
- Chia sẻ handshake TLS đầu tiên: alias `T:addr|frame` → canonical `T:addr`; session theo
  authority; đếm `h2_users`; giữ phiên idle 15 s cho request sau.
- **fd dùng chung nhiều token**: `token_fd`/`fd_tokens`/`epoll_token`/`fd_wants`; epoll đăng ký
  **một lần/fd**, `wait` phát event cho **mọi** token scheduler của fd — nhiều frame cùng chờ
  một connection h2 không ghi đè registration.
- `h2_send` nhận method/target/headers; `h2_poll` lái `tien()` tới `WouldBlock`, trả
  `Response`/`Err`; `h2_cancel` RST riêng stream.
- Tách `async_net_h2.rs` (impl con, truy cập field private) + `async_net_host.rs` (proxy
  `NetHost`/loopback) để `async_net.rs` còn **1479 dòng** — qua luật R1 (≤ 1500).

## 4. Checkpoint 3 — core async

`vis-interp/src/net/async_http.rs`:

- `HttpAsync` thêm `h2_id`; handle `H…` đi nhánh `drive_h2`.
- `drive_h2`: gửi request qua `frame_reactor_h2_send`, poll `frame_reactor_h2_poll` với **trọn
  hạn còn lại** (một poll bao cả response, khác h1 chunk 2 s), nhận `ReactorH2Response`.
- HEAD → `{status, headers}`; 3xx + `location` → redirect tối đa 5 (đóng session, mở lại);
  non-2xx → `Err("HTTP <status>")`; body → `Ok(str)` lossy (cùng h1).
- Request header `authorization`/`cookie` dùng **chung** `header_h2` của sync.
- Timeout: hết hạn ⇒ `frame_reactor_h2_cancel` (RST stream) + `Err` "quá thời gian…";
  `dong()` cũng cancel stream h2 trước khi close.

## 5. Checkpoint 4 — test production

Thêm vào `tests/vs3_http2.rs` (module `async-h2`):

| Test | Nội dung | Kết quả |
|---|---|---|
| `async_h2_production_bon_stream_mot_connection` | `vis run` (interp + VM), 1 request "làm ấm" rồi **4 task `spawn` đồng thời** cùng authority; server chỉ trả 4 stream sau khi nhận đủ; đếm connection/stream server-side | **PASS** — 2 engine × (1 warm + 4 concurrent); `conns=2` (1/engine), `streams=10`; < 10 s |
| `async_h2_production_timeout_va_goaway` | server treo 400 ms; server GOAWAY trước accept | **PASS** — "thời gian" và "GOAWAY HTTP/2…", < 3 s |
| `async_h2_production_cap_body` | server 8 MiB + 1 qua `http.get_timeout` | **PASS** — `Err` 8 MiB, < 30 s |
| `async_h2_production_redirect` | 302 `Location: /hello` | **PASS** — body `hello` |
| backend cũ `async_h2_*` | 4 stream cold-start, GET/HEAD, cap, GOAWAY, timeout, fd | **PASS** (6 test + 1 ignored chạy tiến trình con) |

**Fallback h1**: `vs3_tls` async https (server không ALPN) **7/7**, full `vs3_tls` **21/21**;
`vs3_default` nhóm async http **9/9**.

## 6. Tổng hợp lệnh / kết quả

```text
cargo test -p vis-cli --test vs3_http2                      19 passed
cargo test -p vis-cli --features async-h2 --test vs3_http2   29 passed, 1 ignored
cargo test ... async_h2_timeout_lap_khong_ro_fd_con --ignored  fd 5 -> 5
cargo test -p vis-cli --features async-h2 --test vs3_tls     21 passed
cargo test -p vis-cli --features async-h2 --test vs3_default async_http  9 passed
cargo fmt --all -- --check                                   OK
cargo clippy --workspace --all-targets -- -D warnings        OK
cargo clippy -p vis-cli --all-targets --features async-h2 -- -D warnings  OK
./vis-dev check                                              PASS
df: 17 G trống (> 5 G)
```

## 7. Còn lại / blocker (trung thực)

1. **Cold-start 4 task cùng authority**: chia sẻ TLS handshake canonical có race khi cả 4 frame
   cùng chờ một handshake rồi re-arm (một frame hoàn tất, các frame còn lại đôi khi timeout).
   Test production dùng **1 request làm ấm** (đúng mô hình pool idle 15 s) rồi 4 stream đồng
   thời ⇒ chứng minh multiplex/1 connection. Cold-start 4 stream **vẫn được chứng minh ở tầng
   backend** (`H2Async` trực tiếp, test `async_h2_bon_stream_mot_connection`). Cần thêm
   "join nhiều waiter trên cùng canonical key + re-arm không giành registration" (~80–120 dòng
   scheduler/backend) nếu A0 muốn cold-start qua vis tasks.
2. **Auth/cookie**: header request dùng chung `header_h2` (authorization/cookie) và redirect đã
   test; **Set-Cookie của response chưa lưu** — giống h1 async hiện tại (cùng nợ, không hồi quy).
   Basic auth qua URL chưa có test riêng cho h2.
3. **Cancel task giữa request**: timeout có RST + cleanup; hook hủy task của scheduler vẫn là nợ
   chung với HTTP/1.1 (không thêm trong đợt này).
4. **Windows**: nhánh `async-h2` vẫn `#![cfg(unix)]`.
5. Không đổi API sync, không bật default, không dependency mới, không GitHub Actions.

## 8. Lệnh tái lập

```bash
cd /home/ps/.local/share/opencode/worktree/cd45e5/vs3-w2-tls
CARGO_TARGET_DIR=/home/ps/ViScript/target \
  ~/.cargo/bin/cargo test -p vis-cli --features async-h2 --test vs3_http2
CARGO_TARGET_DIR=/home/ps/ViScript/target \
  ~/.cargo/bin/cargo test -p vis-cli --features async-h2 --test vs3_tls
CARGO_TARGET_DIR=/home/ps/ViScript/target \
  ~/.cargo/bin/cargo test -p vis-cli --test vs3_http2
CARGO_TARGET_DIR=/home/ps/ViScript/target ./vis-dev check
```
