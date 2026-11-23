<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# H2 soak 2h — main `621fad0` (GĐ1, async-h2 default, snapshot cache mới)

> **Người làm**: subagent VS3-W2, 2026-11-22/23. **Mốc**: `621fad0` (GĐ1: budget 40M, default
> async-h2, select, openat; h2 cache fix). `TMPDIR=/home/ps/soak-tmp`; target `target/w2`.
> Không tag/push/merge; không GitHub Actions.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| `async_h2_soak_env_gated` 2h | **7155 vòng, 0 fail**, 7200,7 s |
| fd / RSS / threads | fd **8→9**, RSS **5,8→8,5 MB**, threads **6→6** |
| cold p50 / p95 | **14 ms / 83 ms** |
| Stream/connection | cold **7155 conn / 28 620 stream**; thường **8 586 conn / 37 206 stream**; redirect/timeout/goaway đủ mỗi vòng |
| Xen kẽ mỗi 15 phút ×8 | `vs3_select` **25/25**, `vs3_http2` **34+1**, `vs3_http_cancel` **8/8** — **8/8 vòng xanh** |
| Snapshot cache | 2 lần chạy trong TMPDIR riêng ⇒ **1 dir** `vs3-w2-bin-cache-…` |
| Gate | **3638 passed / 0 failed / 4 skipped** |
| Blocker | **không** |

## 1. Soak 2 giờ

- Lệnh: `TMPDIR=/home/ps/soak-tmp VS3_W2_SOAK_SECS=7200 VS3_W2_SOAK_CAP_EVERY=5
  vs3_http2 --exact h2_async_spike::async_h2_soak_env_gated --nocapture`.
- Kết quả cuối:
  ```
  soak KẾT THÚC: vòng=7155 cold=7155 redirect=7155 cap=1431 timeout=7155 goaway=7155
  giây=7200.7 fd=8->9 rss=5.8->8.5MB threads=6->6
  conns(cold/thuong/treo/goaway)=7155/8586/7155/7155
  streams(cold/thuong)=28620/37206 cold_p50=14ms cold_p95=83ms
  soak: PASS
  test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7200.82s
  ```
- Log: `JOURNAL/h2-soak-2h-621fad0.log`.

## 2. Xen kẽ mỗi 15 phút

Mỗi mốc 15 phút chạy lần lượt (TMPDIR=/home/ps/soak-tmp; `VS3_W2_SOAK_SECS=0` cho target
`vs3_http2` để không lặp soak trong tiến trình xen kẽ):

| Vòng | `vs3_select` | `vs3_http2` | `vs3_http_cancel` |
|---|---|---|---|
| 1..8 | **25/25** | **34+1** | **8/8** |

Cả **8/8 mốc** đều xanh, 0 fail/flake. Log: `JOURNAL/h2-soak-interleave.log` (chi tiết từng
target trong `/tmp/opencode/h2-soak-interleave-*`).

## 3. Snapshot cache

- `TMPDIR=/home/ps/soak-tmp` dùng chung với các tiến trình khác nên thư mục `/tmp` cũ có thể còn
  `vs3-w2-bin-<pid>` từ tiến trình chưa cập nhật; **không** phải do soak này.
- Kiểm cô lập: chạy `vs3_http2` **2 lần** với `TMPDIR=/home/ps/soak-tmp/ours-only`:
  - cả hai **34+1 pass**;
  - `ls vs3-w2-bin-*` = **1** (`vs3-w2-bin-cache-21d0830e84e74548`), lần 2 tái dùng.
- Sau khi dọn snapshot cũ ngoài cache, `/home/ps/soak-tmp` còn **223 MB** (chỉ cache + temp nhỏ).

## 4. Gate / đĩa

- `./vis-dev gate --fast` (TMPDIR=`/home/ps/soak-tmp/ours-only`): **PASS — 3638 passed / 0 failed /
  4 skipped**, 135,4 s; fmt/clippy/R1..R6 OK.
- Cache sau gate vẫn **1 dir**.
- df sau dọn build test artifact: **14 G** trống (>5 G).

## 5. Kết luận

- Main `621fad0` chịu soak **2 giờ liên tục**: 7155 vòng, 0 fail, fd/RSS/threads phẳng,
  p50 14 ms / p95 83 ms; các suite chính xen kẽ đều xanh 8/8 mốc.
- Snapshot cache mới hoạt động đúng: **1 bản/build**, không rò tmpfs như lỗi cycle 31 cũ.
- **Không blocker**; không tag/push/merge.
