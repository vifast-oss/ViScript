<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Release-mode HTTP/2 — 2026-11-21 (W2)

- **Baseline**: `main` `14310f4` (v0.3.2 tagged), nhánh `vs3/w2-tls` fast-forward
  `81a4cda..14310f4`, không conflict. `http2` default-on.
- **Không sửa production**: chỉ chạy test release + 10 vòng stress, ghi journal/log;
  không đổi `net/`, `async_net.rs`, `frame/**`, không dependency mới, không GitHub Actions.
- Môi trường local hermetic: TLS + ALPN h2, fixture cert, không Internet.
- Log thô: [`vs3-h2-release.log`](vs3-h2-release.log) (256 dòng).

## 1. Build + test release

```text
cargo test --release -p vis-cli --test vs3_http2 --test vs3_tls --test http
```

| Binary | Kết quả | Thời gian |
|---|---|---:|
| `http` | **22 passed / 0 failed** | 5,25 s |
| `vs3_http2` | **14 passed / 0 failed** | 15,61 s |
| `vs3_tls` | **21 passed / 0 failed** | 4,37 s |

- Build release (lần đầu): **3 phút 32 giây**; tổng wall cả lệnh test: **237,51 s**.
- Không warning, không test fail, default feature `http2` bật.
- `vs3_http2` gồm đủ hardening: GOAWAY retry, RST_STREAM, trailer, no-body/header lớn,
  server đóng giữa chừng, timeout, registry h2, cookie/auth, reuse, idle 15 s, fallback h1.

## 2. 10 vòng stress release

Mỗi vòng `stress_100_request_4_authority`: 100 GET/4 authority + idle 15,2 s + 4 GET.

| Vòng | Stress | Test (s) | Wall (s) | 100 req (ms) | Conn sau 100 | Sau idle (ms) | Conn sau idle |
|---:|---|---:|---:|---:|---|---:|---|
| 1 | PASS | 15,57 | 15,72 | 190 | 1/1/1/1 | 171 | 2/2/2/2 |
| 2 | PASS | 15,56 | 15,63 | 180 | 1/1/1/1 | 175 | 2/2/2/2 |
| 3 | PASS | 15,56 | 15,65 | 180 | 1/1/1/1 | 173 | 2/2/2/2 |
| 4 | PASS | 15,56 | 15,63 | 179 | 1/1/1/1 | 175 | 2/2/2/2 |
| 5 | PASS | 15,44 | 15,51 | 179 | 1/1/1/1 | 51 | 2/2/2/2 |
| 6 | PASS | 15,56 | 15,64 | 179 | 1/1/1/1 | 174 | 2/2/2/2 |
| 7 | PASS | 15,57 | 15,64 | 181 | 1/1/1/1 | 178 | 2/2/2/2 |
| 8 | PASS | 15,48 | 15,57 | 182 | 1/1/1/1 | 92 | 2/2/2/2 |
| 9 | PASS | 15,56 | 15,64 | 180 | 1/1/1/1 | 174 | 2/2/2/2 |
| 10 | PASS | 15,56 | 15,61 | 176 | 1/1/1/1 | 176 | 2/2/2/2 |

**Tổng**: 10/10 PASS, `stress_exit=0`; 1.000 request stress + 40 request sau idle;
wall 10 vòng **156 s**.

- Wall stress: **15,51–15,72 s** (TB 15,62 s; median 15,64 s).
- 100 request: **176–190 ms** (TB 180,6 ms; median 180 ms).
- Sau idle: **51–178 ms** (TB 153,9 ms) cho 4 request mở connection mới.
- Connection: **1/1/1/1** sau 100 request, **2/2/2/2** sau idle, đủ 10 vòng ⇒ không rò.
- Không treo, không lỗi, không process còn sống sau mỗi vòng.

## 3. Khác biệt release vs debug

Baseline debug: 50 vòng `agent/JOURNAL/vs3-h2-50vong.md` (cùng máy, cùng main `d1f8f70`,
debug build; main `14310f4` chỉ thêm docs nên code không đổi).

| Chỉ số | Debug (50 vòng) | Release (10 vòng) | Nhận xét |
|---|---:|---:|---|
| Wall stress TB | 15,71 s | **15,62 s** | nhanh ~0,09 s/vòng; phần lớn là nhiễu vì idle 15,2 s cố định |
| Wall stress min–max | 15,60–16,03 s | **15,51–15,72 s** | release ổn định hơn, không outlier >16 s |
| 100 request TB | 231,8 ms | **180,6 ms** | release nhanh ~22% (debug có outlier 476/479 ms) |
| 100 request median | 216 ms | **180 ms** | release nhanh ~17% |
| Sau idle TB | 153,6 ms | **153,9 ms** | tương đương (4 handshake/request, dao động lớn) |
| `vs3_http2` suite | 15,54 s | **15,61 s** | tương đương; bị chi phối bởi idle 15,2 s |
| `http` suite | 5,05 s | **5,25 s** | tương đương/nhiễu |
| `vs3_tls` suite | 6,22 s (lần gần nhất; có lần 4,36 s) | **4,37 s** | trong dải dao động debug |

**Kết luận**: release không đổi hành vi (connection count, idle, fallback, hardening đều PASS);
chủ yếu giảm CPU/parse/alloc của 100 request (~17–22% ở median/TB), nhưng wall mỗi vòng gần như
không đổi vì 15,2 s idle chiếm phần lớn. Không thấy khác biệt logic release vs debug.

## 4. Blocker

- **True multiplex production** vẫn chưa mở: `http_get/head` đồng bộ + `Net` `&mut` một luồng;
  prototype test-only `net/h2.rs::gui_nhieu` đã chứng minh 4 stream in-flight/1 connection.
  API async h2 vẫn chờ W1 (design ở `agent/JOURNAL/vs3-h2-soak.md` §3 và
  `agent/VS3-W1-ASYNC-H2-DESIGN.md`).
- Idle cleanup lazy (request kế tiếp/Net drop), không reaper.
- Windows native h2/release chưa đo; cross-check MSVC dừng ở `ring`/`libsqlite3-sys` (#156).
- Release build lần đầu 3m32s; đĩa sau test còn **19G** (> ngưỡng 5G).
- Không push/merge/tag; không dependency mới.
