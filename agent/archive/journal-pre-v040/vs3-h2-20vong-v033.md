<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# 20 vòng stress HTTP/2 release — v0.3.3 (`bbf3484`) (2026-11-21)

- **Tag**: `bbf3484` (`v0.3.3`, fix h2 flow-control body >64 KiB).
- **Branch**: `vs3/w2-tls`, merge `main` `fefe9bd` fast-forward `f0fdf98..fefe9bd`,
  không conflict. `fefe9bd` chỉ hơn tag ở docs/example; các tệp
  `net/h2.rs`, `net.rs`, `tests/vs3_http2.rs`, `Cargo.toml` **giống hệt** tag
  (`git diff bbf3484..HEAD -- <các tệp này>` rỗng) ⇒ test chạy đúng code v0.3.3.
- **Không sửa production**: chỉ chạy release test + journal/log; không dependency mới,
  không GitHub Actions, không push/merge.
- Log thô: [`vs3-h2-20vong-v033.log`](vs3-h2-20vong-v033.log).

## 1. Kịch bản mỗi vòng

Một vòng = **toàn bộ 19 test release** của `vs3_http2`, gồm:

- `stress_100_request_4_authority`: 100 GET/4 authority + idle 15,2 s + 4 GET;
  connection mỗi authority **1 → 2**, body/status từng request.
- `fallback_http1_khi_khong_alpn`.
- `h2_body_8mib_va_vuot_cap`: body đúng 8 MiB + vượt 1 byte.
- `h2_header_sat_cap_va_vuot`: header đúng 64 KiB + vượt 1 byte.
- trailer (nhỏ + 1 MiB), GOAWAY giữa body, timeout giữa body, GOAWAY retry, RST,
  server đóng giữa chừng, no-body/header lớn, registry h2, cookie/auth, reuse, idle.

Chạy `cargo test -q --release -p vis-cli --test vs3_http2 -- --nocapture`; vòng lặp dừng nếu
exit khác 0 (thực tế không xảy ra).

## 2. Kết quả 20 vòng

| Vòng | Test | Wall (s) | 100 req (ms) | Conn sau 100 | Sau idle (ms) | Conn sau idle |
|---:|---|---:|---:|---|---:|---|
| 1 | 19/19 | 15,68 | 103 | 1/1/1/1 | 179 | 2/2/2/2 |
| 2 | 19/19 | 15,72 | 213 | 1/1/1/1 | 176 | 2/2/2/2 |
| 3 | 19/19 | 15,59 | 235 | 1/1/1/1 | 44 | 2/2/2/2 |
| 4 | 19/19 | 15,77 | 135 | 1/1/1/1 | 107 | 2/2/2/2 |
| 5 | 19/19 | 15,86 | 162 | 1/1/1/1 | 141 | 2/2/2/2 |
| 6 | 19/19 | 15,70 | 195 | 1/1/1/1 | 101 | 2/2/2/2 |
| 7 | 19/19 | 15,87 | 358 | 1/1/1/1 | 108 | 2/2/2/2 |
| 8 | 19/19 | 15,85 | 252 | 1/1/1/1 | 93 | 2/2/2/2 |
| 9 | 19/19 | 15,54 | 138 | 1/1/1/1 | 92 | 2/2/2/2 |
| 10 | 19/19 | 15,63 | 126 | 1/1/1/1 | 178 | 2/2/2/2 |
| 11 | 19/19 | 15,74 | 241 | 1/1/1/1 | 176 | 2/2/2/2 |
| 12 | 19/19 | 15,45 | 61 | 1/1/1/1 | 80 | 2/2/2/2 |
| 13 | 19/19 | 15,89 | 254 | 1/1/1/1 | 98 | 2/2/2/2 |
| 14 | 19/19 | 15,68 | 226 | 1/1/1/1 | 36 | 2/2/2/2 |
| 15 | 19/19 | 15,71 | 190 | 1/1/1/1 | 98 | 2/2/2/2 |
| 16 | 19/19 | 15,69 | 129 | 1/1/1/1 | 134 | 2/2/2/2 |
| 17 | 19/19 | 15,47 | 15 | 1/1/1/1 | 132 | 2/2/2/2 |
| 18 | 19/19 | 15,42 | 62 | 1/1/1/1 | 54 | 2/2/2/2 |
| 19 | 19/19 | 15,49 | 17 | 1/1/1/1 | 178 | 2/2/2/2 |
| 20 | 19/19 | 15,70 | 180 | 1/1/1/1 | 177 | 2/2/2/2 |

## 3. Tổng hợp

- **20/20 vòng PASS**, `exit=0`; tổng **380 lượt test** (19 × 20), không fail.
- Wall: **15,42–15,89 s** (TB 15,67; median 15,695); tổng 20 vòng **314 s**.
- 100 request: **15–358 ms** (TB 164,6; median 171,0); dao động do CPU/fuzz, không lỗi.
- Sau idle: **36–179 ms** (TB 119,1; median 107,5).
- Connection: **1/1/1/1** sau 100 request và **2/2/2/2** sau idle, đủ 20 vòng
  ⇒ reuse + idle cleanup đúng, **không rò connection**.
- Giới hạn chạy mỗi vòng: body đúng **8 MiB** nhận đủ + vượt 1 byte lỗi rõ;
  header đúng **64 KiB** nhận + vượt 1 byte lỗi rõ; trailer/GOAWAY/timeout đều PASS.
- Fallback h1 PASS 20/20.
- Không treo; không process/test còn sống sau mỗi vòng.

## 4. Blocker

- **True multiplex production** vẫn chưa mở: `http_get/head` đồng bộ + `Net` `&mut` một
  luồng; prototype test-only `gui_nhieu` đã chứng minh 4 stream/1 connection. API async h2
  vẫn chờ W1.
- Idle cleanup lazy (request kế tiếp/Net drop), không reaper.
- Windows native h2/release chưa đo; cross-check MSVC dừng ở `ring`/`libsqlite3-sys` (#156).
- Cap sản phẩm: body **8 MiB**, header **64 KiB** (bao gồm name+value), đúng cap nhận,
  vượt 1 byte từ chối.
- Đĩa sau soak: **17G trống** (> ngưỡng 5G). Không dependency mới, không push/merge/tag.
