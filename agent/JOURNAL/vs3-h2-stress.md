<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Stress HTTP/2 — 2026-11-21 (W2)

- **Baseline**: `main` `a33f46c`, nhánh `vs3/w2-tls` (default-on `http2`).
- **Không** đổi production; chỉ thêm test stress trong `crates/vis-cli/tests/vs3_http2.rs`.
- **Không** GitHub Actions; local hermetic, không Internet.

## 1. Kịch bản

`stress_100_request_4_authority` (test duy nhất của ca stress):

- 4 server TLS h2 local (ALPN h2), 4 authority khác nhau (4 cổng).
- **100 request GET tuần tự**, chia 25 request/authority, cùng một `Net`.
- Mỗi authority phải chỉ có **1 connection** cho 25 request (h2 reuse).
- Nghỉ **15,2 s** (idle timeout 15 s của `PhienH2`), rồi 1 request/authority:
  - mỗi authority phải mở **connection thứ 2** (phiên idle bị Net dọn).
- Test fail nếu request lỗi/treo/body sai/connection count sai.

Các ca hardening/fallback hiện có vẫn chạy trong cùng binary:

- `h2_tai_su_dung_ket_noi` (4 GET/1 connection)
- `h2_idle_timeout_dong_phien`
- `h2_goaway_retry_mot_lan`
- `h2_rst_stream_loi_ro`
- `h2_trailer_roi_moi_tai_su_dung`
- `h2_khong_body_va_header_lon`
- `h2_server_dong_giua_chung`
- `fallback_http1_khi_khong_alpn`
- registry h2, GET/HEAD, redirect+cookie, Basic auth, timeout.

## 2. Số liệu

```text
stress_100_request_4_authority: PASS (15,52 s, 100 request + 15,2 s idle)
cargo test -p vis-cli --test vs3_http2       14 passed / 0 failed (15,39 s)
cargo test -p vis-cli --test vs3_tls        17 passed / 0 failed
cargo test -p vis-cli --test http           22 passed / 0 failed
cargo test -p vis-cli --test vs3_keepalive   8 passed / 0 failed
cargo test -p vis-cli --lib                  7 passed / 0 failed (gồm prototype 4 stream in-flight)
cargo check -p vis-cli (default/no-default/wasm-http2)  Finished
cargo clippy ... -D warnings                 Finished
./vis-dev rules                              OK
```

Connection count quan sát:

| Giai đoạn | Authority 0..3 |
|---|---:|
| sau 100 request | 1 / 1 / 1 / 1 |
| sau idle 15,2 s + 4 request | 2 / 2 / 2 / 2 |

Không có request lỗi, không timeout bất thường, không treo; server test đóng sạch sau khi `Net` drop.

## 3. Blocker / ghi chú

- **True multiplex production** vẫn chưa mở vì API `http_get/head` đồng bộ + `Net` `&mut` một
  luồng; prototype `net/h2.rs::gui_nhieu` (test-only) đã chứng minh 4 stream in-flight/1 connection.
- Idle cleanup lazy (request kế tiếp/Net drop), không reaper; stress chứng minh lần request sau
  idle mở connection mới.
- Windows native h2 chưa đo; cross-check MSVC dừng ở ring/libsqlite (#156).
- Không push/merge/tag; không dependency mới.
