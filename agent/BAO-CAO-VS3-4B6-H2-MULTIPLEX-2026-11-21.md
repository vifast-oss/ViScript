<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-4b6 — prototype h2 multiplex (nhiều stream in-flight, test-only) (2026-11-21)

> **Baseline**: `main` `c85b523` (v0.3.1; `http2` default-on), nhánh fast-forward.
> **Phạm vi**: chỉ `crates/vis-cli/src/net/h2.rs` + unit test trong cùng tệp; không chạm
> `async_net.rs`, `net/tls.rs`, `net.rs`, `frame/**`, `value.rs`, `pkg.rs`, không dependency mới.

## 1. Design

- `PhienH2` đã có `SendRequest` + `Connection` h2 và executor một luồng.
- Thêm **`gui_nhieu(&[YeuCauH2], timeout) -> Vec<Result<KetQuaHttp2, String>>`** (chỉ
  `#[cfg(test)]`):
  1. gửi **tất cả** request trước (`send_request` nhiều lần) ⇒ h2 cấp **stream ID** và giữ nhiều
     stream in-flight;
  2. vòng poll chung: poll `Connection` + từng `ResponseFuture`, rồi `RecvStream`/trailer của từng
     stream; kết quả giữ đúng thứ tự đầu vào;
  3. deadline chung `timeout` (adapter `han` không reset); body/header cap như request thường;
  4. lỗi connection/GOAWAY → mọi stream đang chờ nhận `Err`, không treo.
- Flow control do `h2` xử lý (cửa sổ stream/connection + WINDOW_UPDATE khi `Bytes` drop); prototype
  không tự cấp phát thêm.
- **Production không đổi**: API ngôn ngữ `http_get/head` đồng bộ, `Net` vẫn gọi `PhienH2::request`
  tuần tự (hàng đợi) + reuse/4 authority/idle 15 s. `gui_nhieu` test-only chứng minh khả năng và
  làm nền cho API async sau (W1 async_net).

## 2. Test

Unit test `net::h2::tests::bon_stream_in_flight_mot_connection`:

- server TLS + ALPN h2 local, **chờ đủ 4 request trước khi trả response** (chứng minh client đã gửi
  4 stream in-flight);
- client `PhienH2::moi` + `gui_nhieu` 4 GET `/0../3`; assert 4 body đúng thứ tự, server đếm **4**
  stream, một connection;
- đóng client ⇒ server `poll_closed` thoát, không treo.

Giữ các test integration 4b2–4b5: 13 ca `vs3_http2` gồm 4 GET/1 connection, idle 15 s, GOAWAY/RST,
trailer, no-body/header lớn, server đóng giữa chừng, fallback h1, cookie/auth/registry h2.

## 3. Kết quả

```text
cargo test -p vis-cli --lib                              7 passed / 0 failed
cargo test -p vis-cli --test vs3_http2                  13 passed / 0 failed (15,3 s)
cargo test -p vis-cli --test vs3_tls                    17 passed / 0 failed
cargo test -p vis-cli --test http                       22 passed / 0 failed
cargo test -p vis-cli --test vs3_keepalive               8 passed / 0 failed
cargo check -p vis-cli (default/no-default/wasm-http2)  Finished
cargo clippy ... -D warnings                            Finished
./vis-dev rules                                         OK
```

Log: `agent/JOURNAL/vs3-h2-multiplex.log`.

## 4. Blocker / khuyến nghị

- **True multiplex production chưa mở**: API `http_get/head` đồng bộ, `Net` `&mut` một luồng; muốn
  nhiều caller in-flight phải có API async/thread (W1 async_net đang làm). Prototype test-only
  chứng minh h2 + executor hiện tại **đủ khả năng** multiplex 4 stream; không đưa API vào sản phẩm
  khi chưa có caller đồng bộ tương ứng.
- Idle cleanup vẫn lazy (request kế tiếp/Net drop), không reaper.
- Windows native h2 vẫn chưa đo; cross-check MSVC dừng ở ring/libsqlite (#156).
- Không thay đổi dependency/default-on; không push/merge/tag.
