<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-4b5 — hardening HTTP/2 (2026-11-21)

> **Baseline**: `main` `4d016c8` (v0.3.0 đã tag; default-on `http2`), nhánh fast-forward.
> **Phạm vi**: `net/h2.rs`, `net.rs`, `tests/vs3_http2.rs`; không đụng `async_net.rs`, `frame/**`,
> `value.rs`, `pkg.rs`. Không dependency mới.

## 1. Xử lý lỗi / biên

- **Trailers**: sau khi `poll_data` hết, client gọi `poll_trailers` và chỉ trả response khi stream
  đóng sạch; trailer bị bỏ nội dung nhưng tiêu thụ xong ⇒ connection còn tái dùng.
- **HEAD/response không body**: HEAD không trả sớm nữa; tiêu thụ data (rỗng) + trailer để đóng stream.
- **Header lớn**: cap **64 KiB** tổng `(name+value)`; vượt ⇒ `Err` rõ, không panic.
- **GOAWAY/RST_STREAM**: `h2::Error::is_go_away()/is_reset()` được phân loại trong thông điệp
  (`GOAWAY HTTP/2 …` / `RST_STREAM HTTP/2 …`).
- **Server đóng giữa chừng**: lỗi IO/response/body trả `Err`; adapter deadline 15 s/request (không
  reset trong một request), không treo.
- **Retry idempotent**: `Net::http2_get` bỏ phiên lỗi và thử lại **đúng một lần** cho lỗi
  GOAWAY/RST/connection/response/body/trailer/gửi; **không** retry lỗi header/body vượt cap hay
  timeout. Retry tạo phiên mới (không dùng lại connection đã chết).
- Giữ reuse tuần tự/4 authority/idle 15 s; h1 fallback và pool/cookie/auth giữ nguyên.

## 2. Test hermetic (`vs3_http2`, 13 ca)

Mới trong lát này:

| Ca | Nội dung |
|---|---|
| `h2_trailer_roi_moi_tai_su_dung` | response có trailer + request sau vẫn dùng 1 connection |
| `h2_goaway_retry_mot_lan` | server graceful GOAWAY sau response; request sau retry connection mới, đếm 2 |
| `h2_rst_stream_loi_ro` | RST_STREAM ⇒ lỗi rõ, không treo |
| `h2_khong_body_va_header_lon` | 204 không body; header 70 KB ⇒ lỗi cap |
| `h2_server_dong_giua_chung` | server khai Content-Length 100, gửi 3 byte rồi đóng ⇒ lỗi nhanh |

Giữ: 4 GET/1 connection, idle 15,2 s → connection mới, GET/HEAD, redirect+cookie, Basic auth,
fallback h1, timeout, registry h2.

## 3. Kết quả

```text
cargo test -p vis-cli --test vs3_http2       13 passed / 0 failed (15,3 s)
cargo test -p vis-cli --test vs3_tls        12 passed / 0 failed
cargo test -p vis-cli --test http           22 passed / 0 failed
cargo test -p vis-cli --test vs3_keepalive   8 passed / 0 failed
cargo check -p vis-cli (default/no-default/wasm-http2)  Finished
cargo clippy ... -D warnings                 Finished
./vis-dev rules                              OK
```

Log: `agent/JOURNAL/vs3-h2-harden.log`.

## 4. Blocker / còn lại

- **True multiplex** vẫn không khả thi với API `http_get/head` đồng bộ + `Net` `&mut` một luồng;
  giữ **hàng đợi tuần tự** trên một connection (4 authority, idle 15 s). Muốn nhiều stream
  in-flight phải mở API async/thread riêng.
- Idle cleanup **lazy** (khi request kế tiếp/Net drop), không có reaper.
- Header cap 64 KiB là ngưỡng product; frame size do h2 xử lý.
- Windows native h2 vẫn chưa đo; cross-check MSVC dừng ở ring/libsqlite (#156).
- Không push/merge/tag; không dependency mới. Đĩa gần ngưỡng 5G sau khi build — đã dừng test thêm.
