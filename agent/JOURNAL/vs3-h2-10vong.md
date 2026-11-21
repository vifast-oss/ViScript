<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# 10 vòng stress HTTP/2 — 2026-11-21 (W2)

- **Baseline**: `main` `70aeddc` (sau tag v0.3.2), nhánh `vs3/w2-tls` fast-forward
  `f5af18f..70aeddc`, không conflict. `http2` default-on.
- **Không sửa production**: chỉ thêm `eprintln!` số liệu vào test stress; không đổi `net/`,
  `async_net.rs`, `frame/**`, không dependency mới, không GitHub Actions.
- Môi trường local hermetic: TLS + ALPN h2, fixture cert, không Internet.
- Log thô: [`vs3-h2-10vong.log`](vs3-h2-10vong.log) (170 dòng).

## 1. Kịch bản mỗi vòng

1. `stress_100_request_4_authority`: 4 server/authority h2 local; **100 GET tuần tự**
   (25/authority) trên một `Net`.
2. Kiểm connection: mỗi authority phải **1 connection**.
3. Nghỉ **15,2 s** (idle timeout phiên h2 = 15 s), rồi 1 GET/authority; mỗi authority phải mở
   **connection thứ 2**.
4. `fallback_http1_khi_khong_alpn`: server không chọn ALPN h2 ⇒ client rơi về HTTP/1.1, PASS.

Số liệu in từ test (`--nocapture`): thời gian 100 request, connection/authority, thời gian sau
idle. Wall time đo bằng `/usr/bin/time` quanh cả tiến trình `cargo test`.

## 2. Kết quả 10 vòng

| Vòng | Stress | Test (s) | Wall (s) | 100 req (ms) | Conn sau 100 | Sau idle (ms) | Conn sau idle | Fallback h1 | Fallback wall (s) |
|---:|---|---:|---:|---:|---|---:|---|---|---:|
| 1 | PASS | 15,59 | 15,66 | 210 | 1/1/1/1 | 175 | 2/2/2/2 | PASS | 0,06 |
| 2 | PASS | 15,59 | 15,64 | 200 | 1/1/1/1 | 180 | 2/2/2/2 | PASS | 0,07 |
| 3 | PASS | 15,51 | 15,56 | 201 | 1/1/1/1 | 98 | 2/2/2/2 | PASS | 0,08 |
| 4 | PASS | 15,61 | 15,68 | 211 | 1/1/1/1 | 185 | 2/2/2/2 | PASS | 0,09 |
| 5 | PASS | 15,56 | 15,64 | 212 | 1/1/1/1 | 139 | 2/2/2/2 | PASS | 0,09 |
| 6 | PASS | 15,52 | 15,59 | 212 | 1/1/1/1 | 101 | 2/2/2/2 | PASS | 0,10 |
| 7 | PASS | 15,61 | 15,70 | 215 | 1/1/1/1 | 183 | 2/2/2/2 | PASS | 0,07 |
| 8 | PASS | 15,61 | 15,67 | 216 | 1/1/1/1 | 181 | 2/2/2/2 | PASS | 0,07 |
| 9 | PASS | 15,56 | 15,63 | 211 | 1/1/1/1 | 141 | 2/2/2/2 | PASS | 0,09 |
| 10 | PASS | 15,55 | 15,63 | 204 | 1/1/1/1 | 142 | 2/2/2/2 | PASS | 0,08 |

**Tổng**: 10/10 vòng PASS; **1000 request stress + 40 request sau idle + 10 fallback h1**;
`stress_exit=0`, `fallback_exit=0` mọi vòng.

- Wall stress: **15,56–15,70 s** (trung bình 15,63 s; chênh 0,14 s), khớp 15,2 s idle + xử lý
  request ⇒ không có dấu hiệu treo.
- 100 request: **200–216 ms** (trung bình ~209 ms, ~2,1 ms/request tuần tự).
- Sau idle: **98–185 ms** cho 4 request mở connection mới.
- Connection count: sau 100 request **1/1/1/1**; sau idle + 4 request **2/2/2/2** — đúng thiết kế
  reuse + idle cleanup, không rò thêm connection.
- Không process/test nào còn sống sau mỗi vòng (cargo trả về, exit 0).

## 3. Test tổng hợp (sau 10 vòng)

```text
cargo test -p vis-cli --test vs3_http2       14 passed / 0 failed (15,54 s)
cargo test -p vis-cli --test vs3_tls        21 passed / 0 failed
cargo test -p vis-cli --test http           22 passed / 0 failed
cargo test -p vis-cli --test vs3_keepalive   8 passed / 0 failed
cargo test -p vis-cli --lib                  7 passed / 0 failed
cargo check -p vis-cli (default/no-default/wasm-http2)  Finished
cargo clippy -p vis-cli ... -- -D warnings   Finished
./vis-dev rules                              OK
```

`vs3_http2` vẫn gồm GOAWAY retry, RST_STREAM, trailer, no-body/header lớn, server đóng giữa
chừng, timeout, registry h2, cookie/auth, reuse, idle, fallback h1.

## 4. Blocker

- **True multiplex production** vẫn chưa mở: `http_get/head` đồng bộ + `Net` `&mut` một luồng;
  prototype test-only `net/h2.rs::gui_nhieu` đã chứng minh 4 stream in-flight/1 connection.
  API async h2 vẫn chờ W1 (design ở `agent/JOURNAL/vs3-h2-soak.md` §3 và
  `agent/VS3-W1-ASYNC-H2-DESIGN.md`).
- Idle cleanup lazy (request kế tiếp/Net drop), không reaper; 10 vòng xác nhận hành vi nhất quán.
- Windows native h2 chưa đo; cross-check MSVC dừng ở `ring`/`libsqlite3-sys` (#156).
- Đĩa sau soak: **19G trống** (> ngưỡng 5G).
- Không push/merge/tag; không dependency mới.
