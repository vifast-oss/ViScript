<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Soak/stress HTTP/2 — 2026-11-21 (W2)

- **Baseline**: `main` `77e2899` (v0.3.2), nhánh `vs3/w2-tls`; `http2` default-on.
- **Không đổi production**: chỉ chạy lại stress/test và ghi tài liệu; không GitHub Actions.
- Môi trường: local hermetic, TLS h2 + rustls fixture cert; không Internet.

## 1. 5 vòng stress (100 request / 4 authority + idle + fallback h1)

Test `stress_100_request_4_authority`:

- 4 authority TLS h2; 100 GET tuần tự (25/authority), một `Net`.
- Sau 100 request: mỗi authority **1 connection**.
- Nghỉ **15,2 s** (idle timeout 15 s) → request lại mỗi authority: mỗi authority **2 connection**.
- Test fail nếu request lỗi/body sai/connection count sai/treo.

| Vòng | Stress test | Wall (gồm cargo/test) | Fallback h1 |
|---:|---|---:|---|
| 1 | PASS (15,41 s) | 22,05 s | PASS (0,18 s) |
| 2 | PASS (15,62 s) | 15,73 s | PASS (0,10 s) |
| 3 | PASS (15,41 s) | 15,52 s | PASS (0,32 s) |
| 4 | PASS (15,55 s) | 15,81 s | PASS (0,08 s) |
| 5 | PASS (15,56 s) | 15,63 s | PASS (0,08 s) |

**Kết luận**: 5/5 vòng xanh; 500 request stress + 20 request sau idle + fallback h1 mỗi vòng;
không lỗi, không treo, không rò connection (count đúng 1 → 2 mỗi authority).

Các test hardening khác trong binary cũng xanh: GOAWAY retry, RST_STREAM, trailer, no-body/
header lớn, server đóng giữa chừng, registry h2, cookie/auth, timeout.

## 2. Test tổng hợp

```text
cargo test -p vis-cli --test vs3_http2       14 passed / 0 failed (15,54 s)
cargo test -p vis-cli --test vs3_tls        21 passed / 0 failed
cargo test -p vis-cli --test http           22 passed / 0 failed
cargo test -p vis-cli --test vs3_keepalive   8 passed / 0 failed
cargo test -p vis-cli --test vs3_registry_https  (đã chạy trong suite h2/default)
cargo test -p vis-cli --lib                  7 passed / 0 failed
cargo check -p vis-cli (default/no-default/wasm-http2)  Finished
cargo clippy ... -D warnings                 Finished
./vis-dev rules                              OK
```

Raw log 5 vòng: `/tmp/opencode/vs3-soak/soak.log` (không commit; số liệu chép ở trên).

## 3. Design ngắn — async h2 (API cần W1)

Hiện trạng:

- `async_http.rs` chỉ HTTP/1.1: state machine `HttpAsync` giữ trong `frame.current`, mỗi bước gọi
  reactor `AsyncNet`; chưa có h2.
- `net/h2.rs` có `PhienH2` (sync) + prototype `gui_nhieu` (test-only) gửi 4 stream in-flight trên
  một connection; executor một luồng của W2.
- API sync `http_get/head` không có caller đồng thời ⇒ production vẫn queue tuần tự.

Để W2 mở async h2, W1 cần cung cấp:

1. **Byte stream async + readiness** cho một kết nối TLS đã handshake (fd/nonblocking), có
   `poll_read/poll_write` hoặc callback reactor; `AsyncNet` hiện có thể expose nếu thêm wrapper
   `AsyncRead/AsyncWrite` cho `TlsChan` (không dùng blocking `NetHost`).
2. **Executor/task cho driver h2**: một task sở hữu `h2::client::Connection` chạy tới GOAWAY/close;
   nó phải sống qua nhiều frame, không bị hủy khi một `http.get` trả về. Có thể là task scheduler
   frame hoặc một thread riêng; cần API spawn/join/cancel.
3. **Pool/stream handle async** theo authority: `SendRequest` + stream id; `http_get` async nhận
   handle stream, chờ response/body/trailer theo frame; timeout/cancel theo stream.
4. **Byte/flow-control**: trả `Bytes`/`ReadBuf` theo frame; WINDOW_UPDATE khi drop; cap body/header
   ở W2.
5. **cap/permission**: hook `check_net` trước khi mở session, như `async_http` hiện tại.

Đề xuất interface nội bộ (chưa code, để W1/W2 chốt):

```text
AsyncH2Pool::get(authority) -> H2SessionHandle
H2SessionHandle::send(request) -> StreamHandle
StreamHandle::poll_frame -> Headers | Data(Bytes) | Trailers | Done | Err
H2SessionHandle::close(reason)
```

W2 sẽ dùng `h2::client::SendRequest` async + `Connection` task; HTTP/1.1 async giữ nguyên làm
fallback khi ALPN không chọn h2. Không sửa `async_net.rs`/`async_http.rs` trong lát này.

## 4. Blocker

- Chưa có API async h2 từ W1 (mục 3) ⇒ true multiplex production vẫn chưa mở; prototype W2 đã
  chứng minh 4 stream/1 connection.
- Windows native h2 chưa đo; cross-check MSVC dừng ở `ring`/`libsqlite3-sys` (#156).
- Idle cleanup lazy (request kế tiếp/Net drop), không reaper.
- Không push/merge/tag; không dependency mới.
