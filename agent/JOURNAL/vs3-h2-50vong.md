<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# 50 vòng stress HTTP/2 — 2026-11-21 (W2)

- **Baseline**: `main` `d1f8f70` (v0.3.2 ổn định), nhánh `vs3/w2-tls` fast-forward
  `143cc24..d1f8f70`, không conflict. `http2` default-on.
- **Không sửa production**: chỉ chạy test hiện có và ghi journal/log; không đổi `net/`,
  `async_net.rs`, `frame/**`, không dependency mới, không GitHub Actions.
- Môi trường local hermetic: TLS + ALPN h2, fixture cert, không Internet.
- Log thô: [`vs3-h2-50vong.log`](vs3-h2-50vong.log).

## 1. Kịch bản mỗi vòng

1. `stress_100_request_4_authority`: 4 server/authority h2 local; **100 GET tuần tự**
   (25/authority) trên một `Net`.
2. Kiểm connection: mỗi authority phải **1 connection**.
3. Nghỉ **15,2 s** (idle timeout phiên h2 = 15 s), rồi 1 GET/authority; mỗi authority phải mở
   **connection thứ 2**.
4. `fallback_http1_khi_khong_alpn`: server không chọn ALPN h2 ⇒ client rơi về HTTP/1.1, PASS.

Vòng lặp **dừng ngay** nếu stress hoặc fallback trả exit khác 0 (thực tế không xảy ra). Wall time
đo bằng `/usr/bin/time` quanh cả tiến trình `cargo test`; số liệu trong test in bằng `--nocapture`.

## 2. Kết quả 50 vòng

| Vòng | Stress | Test (s) | Wall (s) | 100 req (ms) | Conn sau 100 | Sau idle (ms) | Conn sau idle | Fallback wall (s) |
|---:|---|---:|---:|---:|---|---:|---|---:|
| 1 | PASS | 15,54 | 15,62 | 158 | 1/1/1/1 | 175 | 2/2/2/2 | 0,06 |
| 2 | PASS | 15,58 | 15,63 | 200 | 1/1/1/1 | 175 | 2/2/2/2 | 0,05 |
| 3 | PASS | 15,54 | 15,60 | 200 | 1/1/1/1 | 135 | 2/2/2/2 | 0,06 |
| 4 | PASS | 15,59 | 15,64 | 200 | 1/1/1/1 | 186 | 2/2/2/2 | 0,13 |
| 5 | PASS | 15,63 | 15,72 | 232 | 1/1/1/1 | 183 | 2/2/2/2 | 0,11 |
| 6 | PASS | 15,60 | 15,70 | 211 | 1/1/1/1 | 184 | 2/2/2/2 | 0,11 |
| 7 | PASS | 15,62 | 15,71 | 220 | 1/1/1/1 | 189 | 2/2/2/2 | 0,14 |
| 8 | PASS | 15,55 | 15,68 | 201 | 1/1/1/1 | 142 | 2/2/2/2 | 0,11 |
| 9 | PASS | 15,60 | 15,69 | 214 | 1/1/1/1 | 176 | 2/2/2/2 | 0,29 |
| 10 | PASS | 15,66 | 16,01 | 386 | 1/1/1/1 | 62 | 2/2/2/2 | 0,11 |
| 11 | PASS | 15,51 | 15,60 | 144 | 1/1/1/1 | 156 | 2/2/2/2 | 0,20 |
| 12 | PASS | 15,52 | 15,69 | 216 | 1/1/1/1 | 90 | 2/2/2/2 | 0,42 |
| 13 | PASS | 15,79 | 16,03 | 476 | 1/1/1/1 | 100 | 2/2/2/2 | 0,08 |
| 14 | PASS | 15,59 | 15,67 | 203 | 1/1/1/1 | 178 | 2/2/2/2 | 0,11 |
| 15 | PASS | 15,58 | 15,68 | 222 | 1/1/1/1 | 146 | 2/2/2/2 | 0,11 |
| 16 | PASS | 15,57 | 15,66 | 215 | 1/1/1/1 | 143 | 2/2/2/2 | 0,09 |
| 17 | PASS | 15,57 | 15,68 | 206 | 1/1/1/1 | 152 | 2/2/2/2 | 0,33 |
| 18 | PASS | 15,84 | 16,03 | 479 | 1/1/1/1 | 147 | 2/2/2/2 | 0,15 |
| 19 | PASS | 15,51 | 15,67 | 189 | 1/1/1/1 | 112 | 2/2/2/2 | 0,17 |
| 20 | PASS | 15,54 | 15,71 | 213 | 1/1/1/1 | 120 | 2/2/2/2 | 0,24 |
| 21 | PASS | 15,47 | 15,70 | 188 | 1/1/1/1 | 66 | 2/2/2/2 | 0,16 |
| 22 | PASS | 15,64 | 15,75 | 243 | 1/1/1/1 | 181 | 2/2/2/2 | 0,09 |
| 23 | PASS | 15,59 | 15,67 | 201 | 1/1/1/1 | 182 | 2/2/2/2 | 0,10 |
| 24 | PASS | 15,54 | 15,64 | 222 | 1/1/1/1 | 106 | 2/2/2/2 | 0,09 |
| 25 | PASS | 15,61 | 15,68 | 218 | 1/1/1/1 | 181 | 2/2/2/2 | 0,08 |
| 26 | PASS | 15,61 | 15,68 | 217 | 1/1/1/1 | 183 | 2/2/2/2 | 0,08 |
| 27 | PASS | 15,61 | 15,70 | 220 | 1/1/1/1 | 183 | 2/2/2/2 | 0,10 |
| 28 | PASS | 15,61 | 15,72 | 221 | 1/1/1/1 | 184 | 2/2/2/2 | 0,08 |
| 29 | PASS | 15,61 | 15,68 | 215 | 1/1/1/1 | 182 | 2/2/2/2 | 0,10 |
| 30 | PASS | 15,61 | 15,71 | 217 | 1/1/1/1 | 185 | 2/2/2/2 | 0,10 |
| 31 | PASS | 15,60 | 15,70 | 215 | 1/1/1/1 | 178 | 2/2/2/2 | 0,10 |
| 32 | PASS | 15,53 | 15,62 | 216 | 1/1/1/1 | 101 | 2/2/2/2 | 0,08 |
| 33 | PASS | 15,61 | 15,69 | 217 | 1/1/1/1 | 182 | 2/2/2/2 | 0,10 |
| 34 | PASS | 15,61 | 15,70 | 216 | 1/1/1/1 | 183 | 2/2/2/2 | 0,09 |
| 35 | PASS | 15,61 | 15,68 | 217 | 1/1/1/1 | 185 | 2/2/2/2 | 0,11 |
| 36 | PASS | 15,63 | 15,72 | 237 | 1/1/1/1 | 183 | 2/2/2/2 | 0,11 |
| 37 | PASS | 15,61 | 15,70 | 217 | 1/1/1/1 | 181 | 2/2/2/2 | 0,08 |
| 38 | PASS | 15,61 | 15,69 | 216 | 1/1/1/1 | 182 | 2/2/2/2 | 0,09 |
| 39 | PASS | 15,57 | 15,64 | 173 | 1/1/1/1 | 185 | 2/2/2/2 | 0,09 |
| 40 | PASS | 15,56 | 15,65 | 221 | 1/1/1/1 | 123 | 2/2/2/2 | 0,16 |
| 41 | PASS | 15,69 | 15,82 | 372 | 1/1/1/1 | 102 | 2/2/2/2 | 0,09 |
| 42 | PASS | 15,73 | 15,82 | 399 | 1/1/1/1 | 116 | 2/2/2/2 | 0,18 |
| 43 | PASS | 15,48 | 15,66 | 194 | 1/1/1/1 | 76 | 2/2/2/2 | 0,16 |
| 44 | PASS | 15,58 | 15,74 | 231 | 1/1/1/1 | 138 | 2/2/2/2 | 0,08 |
| 45 | PASS | 15,61 | 15,70 | 216 | 1/1/1/1 | 183 | 2/2/2/2 | 0,11 |
| 46 | PASS | 15,53 | 15,63 | 218 | 1/1/1/1 | 103 | 2/2/2/2 | 0,09 |
| 47 | PASS | 15,62 | 15,71 | 224 | 1/1/1/1 | 184 | 2/2/2/2 | 0,09 |
| 48 | PASS | 15,61 | 15,68 | 215 | 1/1/1/1 | 186 | 2/2/2/2 | 0,10 |
| 49 | PASS | 15,56 | 15,66 | 212 | 1/1/1/1 | 138 | 2/2/2/2 | 0,08 |
| 50 | PASS | 15,61 | 15,69 | 218 | 1/1/1/1 | 185 | 2/2/2/2 | 0,08 |

## 3. Tổng hợp

- **50/50 vòng PASS**, không vòng nào lỗi; `stress_exit=0`, `fallback_exit=0` toàn bộ.
- Tổng **5.000 request stress + 200 request sau idle + 50 fallback h1**.
- Tổng wall cả 50 vòng: **792 s** (13 phút 12 giây), gồm cả thời gian khởi động `cargo test`.
- Wall stress: **15,60–16,03 s** (trung bình 15,71 s; median 15,69 s), khớp 15,2 s idle
  + xử lý request ⇒ **không treo**.
- 100 request: **144–479 ms** (trung bình 231,8 ms; median 216 ms). Hai vòng 13/18 có outlier
  ~476/479 ms nhưng tổng wall vẫn ≤ 16,03 s, không lỗi; các vòng còn lại 144–399 ms.
- Sau idle: **62–189 ms** cho 4 request mở connection mới (trung bình 153,6 ms).
- Connection count: sau 100 request **1/1/1/1** ở cả 50 vòng; sau idle + 4 request
  **2/2/2/2** ở cả 50 vòng — đúng reuse + idle cleanup, **không rò connection**.
- Fallback h1: **0,05–0,42 s** (trung bình 0,12 s), PASS mọi vòng.
- Không process/test nào còn sống sau mỗi vòng (cargo trả về, exit 0).

## 4. Test tổng hợp sau soak

```text
cargo test -p vis-cli --test vs3_http2       14 passed / 0 failed (15,54 s)
cargo clippy -p vis-cli --bin vis --lib --test vs3_http2 -- -D warnings
                                             Finished
./vis-dev rules                              OK
```

## 5. Blocker

- **True multiplex production** vẫn chưa mở: `http_get/head` đồng bộ + `Net` `&mut` một luồng;
  prototype test-only `net/h2.rs::gui_nhieu` đã chứng minh 4 stream in-flight/1 connection.
  API async h2 vẫn chờ W1 (design ở `agent/JOURNAL/vs3-h2-soak.md` §3 và
  `agent/VS3-W1-ASYNC-H2-DESIGN.md`).
- Idle cleanup lazy (request kế tiếp/Net drop), không reaper; 50 vòng xác nhận hành vi nhất quán.
- Windows native h2 chưa đo; cross-check MSVC dừng ở `ring`/`libsqlite3-sys` (#156).
- Đĩa sau soak: **19G trống** (> ngưỡng 5G).
- Không push/merge/tag; không dependency mới.
