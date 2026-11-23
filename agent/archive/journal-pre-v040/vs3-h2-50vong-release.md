<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# 50 vòng stress HTTP/2 release-mode — 2026-11-21 (W2)

- **Baseline**: `main` `71d52b4` (CLI `--version`/`doctor` đã merge; gate 3530/3), nhánh
  `vs3/w2-tls` fast-forward `c833c0e..71d52b4`, không conflict. `http2` default-on.
- **Không sửa production**: chỉ chạy test release + ghi journal/log; không đổi `net/`,
  `async_net.rs`, `frame/**`, không dependency mới, không GitHub Actions.
- Môi trường local hermetic: TLS + ALPN h2, fixture cert, không Internet.
- Log thô: [`vs3-h2-50vong-release.log`](vs3-h2-50vong-release.log).

## 1. Kịch bản mỗi vòng

`stress_100_request_4_authority` (release):

1. 4 server/authority h2 local; **100 GET tuần tự** (25/authority) trên một `Net`.
2. Mỗi authority phải **1 connection**.
3. Nghỉ **15,2 s** (idle timeout 15 s), rồi 1 GET/authority; mỗi authority mở **connection 2**.
4. Vòng lặp dừng ngay nếu exit khác 0 (thực tế không xảy ra).

Wall đo bằng `/usr/bin/time` quanh `cargo test --release`; số liệu trong test in bằng `--nocapture`.

## 2. Kết quả 50 vòng

| Vòng | Stress | Test (s) | Wall (s) | 100 req (ms) | Conn sau 100 | Sau idle (ms) | Conn sau idle |
|---:|---|---:|---:|---:|---|---:|---|
| 1 | PASS | 15,60 | 24,18 | 282 | 1/1/1/1 | 102 | 2/2/2/2 |
| 2 | PASS | 15,77 | 16,15 | 380 | 1/1/1/1 | 176 | 2/2/2/2 |
| 3 | PASS | 15,56 | 15,64 | 180 | 1/1/1/1 | 177 | 2/2/2/2 |
| 4 | PASS | 15,55 | 15,64 | 180 | 1/1/1/1 | 161 | 2/2/2/2 |
| 5 | PASS | 15,62 | 15,76 | 308 | 1/1/1/1 | 99 | 2/2/2/2 |
| 6 | PASS | 15,43 | 15,59 | 166 | 1/1/1/1 | 58 | 2/2/2/2 |
| 7 | PASS | 15,44 | 15,62 | 150 | 1/1/1/1 | 84 | 2/2/2/2 |
| 8 | PASS | 15,37 | 15,64 | 63 | 1/1/1/1 | 95 | 2/2/2/2 |
| 9 | PASS | 15,52 | 15,60 | 180 | 1/1/1/1 | 133 | 2/2/2/2 |
| 10 | PASS | 15,56 | 15,64 | 181 | 1/1/1/1 | 176 | 2/2/2/2 |
| 11 | PASS | 15,38 | 15,47 | 139 | 1/1/1/1 | 29 | 2/2/2/2 |
| 12 | PASS | 15,52 | 15,77 | 137 | 1/1/1/1 | 175 | 2/2/2/2 |
| 13 | PASS | 15,49 | 15,60 | 179 | 1/1/1/1 | 109 | 2/2/2/2 |
| 14 | PASS | 15,42 | 15,64 | 148 | 1/1/1/1 | 60 | 2/2/2/2 |
| 15 | PASS | 15,45 | 15,62 | 184 | 1/1/1/1 | 57 | 2/2/2/2 |
| 16 | PASS | 15,58 | 15,72 | 232 | 1/1/1/1 | 140 | 2/2/2/2 |
| 17 | PASS | 15,44 | 15,58 | 139 | 1/1/1/1 | 92 | 2/2/2/2 |
| 18 | PASS | 15,48 | 15,56 | 178 | 1/1/1/1 | 93 | 2/2/2/2 |
| 19 | PASS | 15,52 | 15,60 | 141 | 1/1/1/1 | 175 | 2/2/2/2 |
| 20 | PASS | 15,57 | 15,64 | 181 | 1/1/1/1 | 175 | 2/2/2/2 |
| 21 | PASS | 15,44 | 15,51 | 183 | 1/1/1/1 | 52 | 2/2/2/2 |
| 22 | PASS | 15,52 | 15,59 | 182 | 1/1/1/1 | 133 | 2/2/2/2 |
| 23 | PASS | 15,44 | 15,52 | 183 | 1/1/1/1 | 52 | 2/2/2/2 |
| 24 | PASS | 15,52 | 15,59 | 184 | 1/1/1/1 | 133 | 2/2/2/2 |
| 25 | PASS | 15,52 | 15,60 | 179 | 1/1/1/1 | 132 | 2/2/2/2 |
| 26 | PASS | 15,56 | 15,65 | 181 | 1/1/1/1 | 177 | 2/2/2/2 |
| 27 | PASS | 15,56 | 15,63 | 177 | 1/1/1/1 | 174 | 2/2/2/2 |
| 28 | PASS | 15,48 | 15,59 | 180 | 1/1/1/1 | 91 | 2/2/2/2 |
| 29 | PASS | 15,48 | 15,58 | 179 | 1/1/1/1 | 92 | 2/2/2/2 |
| 30 | PASS | 15,48 | 15,58 | 183 | 1/1/1/1 | 90 | 2/2/2/2 |
| 31 | PASS | 15,43 | 15,50 | 175 | 1/1/1/1 | 51 | 2/2/2/2 |
| 32 | PASS | 15,44 | 15,53 | 179 | 1/1/1/1 | 52 | 2/2/2/2 |
| 33 | PASS | 15,38 | 15,49 | 140 | 1/1/1/1 | 26 | 2/2/2/2 |
| 34 | PASS | 15,55 | 15,77 | 205 | 1/1/1/1 | 136 | 2/2/2/2 |
| 35 | PASS | 15,46 | 15,55 | 181 | 1/1/1/1 | 66 | 2/2/2/2 |
| 36 | PASS | 15,55 | 15,79 | 278 | 1/1/1/1 | 57 | 2/2/2/2 |
| 37 | PASS | 15,43 | 15,62 | 128 | 1/1/1/1 | 94 | 2/2/2/2 |
| 38 | PASS | 15,59 | 15,74 | 213 | 1/1/1/1 | 174 | 2/2/2/2 |
| 39 | PASS | 15,44 | 15,54 | 182 | 1/1/1/1 | 51 | 2/2/2/2 |
| 40 | PASS | 15,48 | 15,56 | 179 | 1/1/1/1 | 91 | 2/2/2/2 |
| 41 | PASS | 15,48 | 15,55 | 181 | 1/1/1/1 | 93 | 2/2/2/2 |
| 42 | PASS | 15,52 | 15,61 | 179 | 1/1/1/1 | 136 | 2/2/2/2 |
| 43 | PASS | 15,48 | 15,57 | 179 | 1/1/1/1 | 92 | 2/2/2/2 |
| 44 | PASS | 15,52 | 15,60 | 181 | 1/1/1/1 | 134 | 2/2/2/2 |
| 45 | PASS | 15,40 | 15,47 | 140 | 1/1/1/1 | 52 | 2/2/2/2 |
| 46 | PASS | 15,44 | 15,51 | 181 | 1/1/1/1 | 51 | 2/2/2/2 |
| 47 | PASS | 15,48 | 15,58 | 179 | 1/1/1/1 | 93 | 2/2/2/2 |
| 48 | PASS | 15,55 | 15,63 | 175 | 1/1/1/1 | 172 | 2/2/2/2 |
| 49 | PASS | 15,52 | 15,61 | 178 | 1/1/1/1 | 133 | 2/2/2/2 |
| 50 | PASS | 15,52 | 15,58 | 135 | 1/1/1/1 | 175 | 2/2/2/2 |

## 3. Tổng hợp

- **50/50 vòng PASS**, `stress_exit=0` toàn bộ; dừng-khi-lỗi không kích hoạt.
- Tổng **5.000 request stress + 200 request sau idle**; tổng wall 50 vòng **789 s**
  (gồm release rebuild ở vòng 1).
- Vòng 1 wall **24,18 s** do cargo rebuild release sau khi merge CLI; vòng 2 **16,15 s**;
  từ vòng 3 trở đi wall **15,47–15,79 s** (TB 15,60 s; median 15,60 s) ⇒ không treo.
- 100 request: **63–380 ms** toàn bộ (TB 182,1; median 179,5); bỏ 2 vòng warm-up:
  **63–308 ms** (TB 175,9; median 179). Các outlier 278–380 ms không gây fail/timeout.
- Sau idle: **26–177 ms** (TB 108,4; median 94,5) cho 4 request mở connection mới.
- Connection count: **1/1/1/1** sau 100 request và **2/2/2/2** sau idle, đủ 50 vòng
  ⇒ reuse + idle cleanup đúng, **không rò connection**.
- Full release `vs3_http2` sau soak: **14 passed / 0 failed** (15,48 s).
- So với 10 vòng release trước (`main` `14310f4`: wall TB 15,62 s, 100 req median 180 ms)
  và debug 50 vòng (wall TB 15,71 s, 100 req median 216 ms): release ổn định, không regression;
  phần CPU của 100 request nhanh hơn debug ~17%.

## 4. Blocker

- **True multiplex production** vẫn chưa mở: `http_get/head` đồng bộ + `Net` `&mut` một luồng;
  prototype test-only `net/h2.rs::gui_nhieu` đã chứng minh 4 stream in-flight/1 connection.
  API async h2 vẫn chờ W1 (design ở `agent/JOURNAL/vs3-h2-soak.md` §3 và
  `agent/VS3-W1-ASYNC-H2-DESIGN.md`).
- Idle cleanup lazy (request kế tiếp/Net drop), không reaper.
- Windows native h2/release chưa đo; cross-check MSVC dừng ở `ring`/`libsqlite3-sys` (#156).
- Đĩa sau soak: **17G trống** (> ngưỡng 5G).
- Không push/merge/tag; không dependency mới.
