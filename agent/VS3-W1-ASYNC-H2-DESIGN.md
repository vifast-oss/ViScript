<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W1 — thiết kế async HTTP/2 cho frame scheduler (2026-11-21)

> **Baseline**: `main` `77e2899` (tag v0.3.2), nhánh `vs3/w1-runtime`.
> **Trạng thái**: **DESIGN + TEST PLAN**, chưa đưa code h2 vào async HTTP.
> **Phạm vi W1**: `async_net.rs`, `net/async_http.rs`, phần async của HTTP; **không sửa**
> `crates/vis-cli/src/net/h2.rs` (W2 đang giữ), `pkg.rs`, `value.rs`.
> **Giữ xanh**: async HTTP/1.1, sync HTTP, sync TLS/h2, async net/TLS hiện có.

## 1. Kết luận ngắn

Chưa an toàn để nối h2 vào `net/async_http.rs` trong vòng này:

1. `PhienH2` của W2 là API **đồng bộ**. `request`/`gui_nhieu` tự poll `h2::Connection` +
   `ResponseFuture`/`RecvStream` trong một vòng lặp, dùng `DongBo` (adapter `AsyncRead/AsyncWrite`)
   và executor một luồng `cho()` với `std::thread::yield_now()`. Nó **không** đăng ký fd với
   reactor của frame scheduler và **không** trả `Pending` cho caller.
2. Gọi `PhienH2::request` từ state machine async sẽ **chặn frame hiện tại** (dù socket
   nonblocking) cho tới khi h2 xong; mất tính hợp tác đã đạt ở HTTP/1.1.
3. `net/h2.rs` vừa được W2 hardening/multiplex (4b6) và đang được giữ. Sửa nó thành
   reactor-native là thay đổi W2, không phải W1; copy nó sang `async_net.rs` sẽ tạo hai client h2
   lệch nhau (ALPN, flow-control, GOAWAY, trailer, idle).
4. Vì vậy: giữ async HTTP/1.1 (không ALPN) và ghi **design + test plan** dưới đây. Khi W2 mở
   API h2 reactor-native, W1 nối theo Option A.

## 2. Hiện trạng liên quan

### 2.1 Async HTTP/1.1 (đã có)

- `crates/vis-interp/src/net/async_http.rs`: state machine theo `frame.current`, stage
  `Connect → Write → Read`, dùng `frame_reactor_poll_io`/`frame_reactor_poll_bytes`.
- `crates/vis-cli/src/async_net.rs`:
  - plain `nối` handle `N:<addr>|frame`;
  - TLS `T:<addr>|frame` (TCP nonblocking + rustls handshake nhiều bước, `interest` READABLE/
    WRITABLE);
  - TLS đã handshake trả `C<n>`; request dùng `W:<C-n>`; đọc dùng `C<n>`.
- TLS config hiện **không** đặt ALPN ⇒ server luôn dùng HTTP/1.1 với async client.
- Timeout: một deadline chung; `frame_reactor_poll_bytes` trả byte thô (không lossy giữa chừng).
- `cap[Net]`: `NetHost::check_net` trước mỗi connect.

### 2.2 Sync HTTP/2 của W2 (chỉ đọc)

- `PhienH2` giữ `h2::client::SendRequest<Bytes>` + `Connection<DongBo, Bytes>`, một session/authority.
- `PhienH2::moi`: TLS + ALPN `["h2","http/1.1"]`; server không chọn h2 ⇒ trả `MoH2::Http1`.
- `PhienH2::request`: tuần tự một request/lần; `doc_phan_hoi` poll connection/response/body/trailer
  tới khi xong. `gui_nhieu` là prototype `#[cfg(test)]` gửi nhiều stream trước khi đọc.
- `DongBo::poll_read/poll_write` trả `Pending` khi WouldBlock, nhưng executor `cho()` lại
  `yield_now()` tới khi future Ready; không có reactor wake.
- `Net` sync gọi `http2_get` trong `net.rs`; state machine async **không** đi qua `Net`.

## 3. Phương án

### 3.1 Option A (khuyến nghị) — W2 mở H2 session reactor-native

W2 tách phần protocol khỏi I/O đồng bộ:

```text
PhienH2 (sync, giữ nguyên cho Net)
        │  (refactor nội bộ, không đổi API sync)
        ▼
H2Session (reactor-native)
  - send_request(headers, end_stream) -> stream_id
  - poll_response(stream_id) -> NeedRead | NeedWrite | Headers | Data | Trailers | End
  - poll_connection() -> NeedRead | NeedWrite | Closed/GOAWAY
  - poll_close()
```

- `H2Session` nhận một `AsyncRead/AsyncWrite` **do reactor cấp**; mỗi lần `NeedRead/NeedWrite`,
  `AsyncNet` đăng ký fd với token hiện có và trả `WouldBlock` cho core.
- W1 thêm handle h2:
  - `T:` TLS handshake trả `C<n>` nếu ALPN không phải h2, `H<n>` nếu ALPN = h2.
  - `W:H<n>`: core đưa request (method/target/authority/headers, end_stream) vào backend;
    backend gọi `H2Session::send_request`, trả `WouldBlock`/`Data(empty)` như `net.write`.
  - `H<n>` đọc: backend poll response stream, trả **byte tổng hợp** cho parser HTTP/1.1 hiện có
    (`status + headers + body + optional trailer`) **hoặc** trả một protocol result riêng.
    Cách sạch hơn: mở rộng `FrameReactorBackend` bằng method `try_response_meta` để core nhận
    status/headers/trailer có cấu trúc, không nhồi lại thành HTTP/1.
- Multiplex: `AsyncNet` giữ `HashMap<authority, H2Session>`, mỗi request có `H<n>` riêng; nhiều
  task cùng authority dùng chung connection. Session chỉ đóng khi idle 15 s/GOAWAY/lỗi.
- Timeout: deadline **từng request**; session sống lâu hơn request; khi request timeout, gửi
  `RST_STREAM` (nếu còn), không đóng cả session.

**Ưu**: một client h2 duy nhất, không lệch W2, multiplex thật, kiểm soát flow-control/trailer.
**Nhược**: cần W2 mở API; thay đổi `FrameReactorBackend`/core state machine; ước lượng 700–1200
dòng + test.

### 3.2 Option B — client h2 riêng trong W1 (`async_h2.rs`)

Không sửa `net/h2.rs`; viết client h2 mới dùng crate `h2` + custom `AsyncRead/AsyncWrite` trên
`AsyncNet` TLS, chỉ nối vào async HTTP.

- Ưu: W1 tự chủ, không chờ W2.
- Nhược: duplicate `PhienH2`/ALPN/flow-control/trailer/GOAWAY; hai client h2 có hành vi khác
  nhau; rủi ro regression sync/h2 và test tốn gấp đôi. **Không khuyến nghị** cho v0.3.x.
- Nếu chọn, phải giữ sau feature riêng, mặc định tắt, và có test parity với `vs3_http2`.

### 3.3 Option C — bridge thread (loại)

Đẩy `PhienH2` sang thread và dùng channel. Không chấp nhận: biến async HTTP thành blocking ở
thread khác, phá mô hình một scheduler, khó hủy/cleanup, không giải quyết multiplex trong task.

## 4. Thiết kế chi tiết nếu chọn Option A

### 4.1 ALPN và handshake

- Async TLS `T:` dùng `cau_hinh_h2` **chỉ khi** feature `http2` bật và URL `https`.
- `cau_hinh_h2` đặt `alpn_protocols = ["h2", "http/1.1"]`.
- Sau `ClientConnection` handshake:
  - `alpn_protocol() == Some(b"h2")` ⇒ tạo `H<n>` + `H2Session`;
  - còn lại (kể cả None) ⇒ `C<n>` như hiện tại.
- Không đặt ALPN cho HTTP/1.1-only/feature tắt; fallback h1 không đổi.

### 4.2 Request/response h2

- Pseudo-header: `:method` (GET/HEAD), `:scheme=https`, `:authority=host[:port]`,
  `:path=target`.
- Header thường: `authorization`, `cookie` (nếu có jar), `user-agent` mặc định của h2 client.
- GET: end_stream sau header. HEAD: end_stream sau header, **không** đọc body.
- Response: status, headers, body; trailer đọc hết và bỏ nội dung (giữ hợp đồng HTTP/1.1 hiện tại).
- Cap header 64 KiB, body 8 MiB; lỗi vượt cap ⇒ `Err` dữ liệu, không OOM.
- Redirect: dùng lại `noi_location`/`tach_url` của sync; GET theo tối đa 5 hop; HEAD không theo.
- Cookie/auth: dùng chung `CookieJar`/Basic auth của sync nếu W2 cho phép; nếu chưa, ghi rõ
  async cookie còn mở.

### 4.3 Trạng thái scheduler

```text
ConnectAlpn ── h2 ──> H2Open ──> H2Send ──> H2RecvHeaders ──> H2RecvBody
     │                    │             │                 │
     └── h1 ──> Write/Read hiện tại     └── RST/GOAWAY ──> Loi
```

- `H2Open`/`H2Send`/`H2Recv*` lưu theo `frame.current` như `HttpAsync` hiện tại.
- Mỗi bước trả `Pending` nếu backend báo `WouldBlock`; deadline request lưu trong state.
- Khi response xong, trả `Value` và **không** đóng session nếu còn khả dụng.

### 4.4 Lỗi/cleanup

- GOAWAY: request đang chạy nhận lỗi rõ; đánh dấu authority cần session mới cho request sau.
- RST_STREAM: chỉ stream đó lỗi, session vẫn dùng được.
- Timeout: `RST_STREAM` + trả `Err`; session giữ nếu còn stream khác.
- Hủy task: cần hook scheduler → `AsyncNet` đóng/RST stream + xóa state; **điều kiện bắt buộc**
  trước khi bật production (hiện HTTP/1.1 cũng còn nợ cleanup khi hủy giữa request).

## 5. Test plan (khi W2 mở API)

### 5.1 File/test mới

- `crates/vis-cli/tests/vs3_async_h2.rs` (`required-features = ["http2"]`, Unix).
- Server TLS + ALPN h2 local (rustls + `h2::server`), fixture cert như `vs3_tls`.
- Chạy `vis run` và `vis run --vm --no-cache`, assert stdout/exit bằng nhau.

### 5.2 Ca bắt buộc

| # | Ca | Kỳ vọng |
|---|---|---|
| 1 | Async h2 GET loopback | body đúng; server thấy ALPN h2 |
| 2 | Async h2 HEAD | chỉ header; không đọc body |
| 3 | Body > 1 MiB | đủ byte, không cắt UTF-8/lossy giữa chừng |
| 4 | Trailer response | bỏ nội dung trailer, body/status đúng |
| 5 | Timeout body | `Err` chứa "quá thời gian"; RST stream nếu còn |
| 6 | Redirect GET | tối đa 5; HEAD không theo |
| 7 | ALPN không h2 | fallback HTTP/1.1, không lỗi |
| 8 | Server chỉ h2 | async h2 chạy; không treo |
| 9 | GOAWAY giữa request | lỗi rõ; request sau mở session mới |
| 10 | RST_STREAM | chỉ stream lỗi; session/slot còn lại dùng được |
| 11 | Hai task cùng authority | multiplex 2 stream in-flight trên **một** connection |
| 12 | Hai request tuần tự cùng authority | tái dùng connection |
| 13 | Idle 15 s | session đóng khi chạm idle, request sau mở mới |
| 14 | Cert sai/không tin | `Err` TLS rõ, không panic |
| 15 | `cap[Net]` | thiếu cap ⇒ VS0604 (qua task VS0506), không mở mạng |
| 16 | Interp vs VM | cùng stdout/exit |
| 17 | H2 + HTTP/1.1 async cùng chương trình | mỗi URL đúng protocol, không lẫn handle |
| 18 | Sync h2 + sync TLS + async h1 | toàn bộ test cũ xanh |

### 5.3 Lệnh

```bash
CARGO_TARGET_DIR=/home/ps/ViScript/target/vs3-w1 \
  cargo test -p vis-cli --test vs3_async_h2 --test vs3_default \
             --test vs3_tls --test vs3_http2 --test http
CARGO_TARGET_DIR=/home/ps/ViScript/target/vs3-w1 ./vis-dev gate --fast
```

### 5.4 Tiêu chí mở production

- Không gọi host blocking giữa request; chỉ reactor `Pending`/wake.
- Ca 1–18 xanh trên Linux; Windows tối thiểu compile/design W4.
- Không hồi quy `vs3_http2` (sync) và `vs3_tls`.
- Có cleanup khi hủy task/timeout (không rò session).

## 6. Việc cần W2/A0 quyết

1. W2 có mở `H2Session` reactor-native (Option A) hay W1 tự viết client riêng (Option B)?
2. Có cho phép mở rộng `FrameReactorBackend` bằng method response meta (status/headers/trailer)
   hay chỉ giữ handle byte tổng hợp?
3. Cookie async: dùng jar của `NetGuard` hay để mở tới v0.3.4?
4. Cleanup cancellation: thêm hook scheduler → backend trong W1 hay W2?

Cho tới khi chốt, **async HTTP/1.1 là đường production duy nhất**; `net/h2.rs` không bị sửa.
