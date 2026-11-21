<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W2 — soak dài 2 giờ async-h2 (release, test-only)

**Trạng thái**: soak/hardening, **không sửa production/default/API/dependency**; feature
`async-h2` vẫn **default-off**; không GitHub Actions; không push/merge/tag.
**Nhánh**: `vs3/w2-tls`, merge main **`523e8d2`**.
**Target riêng**: `$PWD/target/w2` (không dùng target chung).
**Log**: [`vs3-w2-h2-2h-soak.log`](vs3-w2-h2-2h-soak.log).

## 1. Cấu hình

```bash
CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true CARGO_TARGET_DIR=$PWD/target/w2 \
VS3_W2_SOAK_SECS=7200 VS3_W2_SOAK_CAP_EVERY=5 \
cargo test --release -p vis-cli --features async-h2 --test vs3_http2 \
  async_h2_soak_env_gated -- --nocapture
```

Mỗi vòng xen kẽ interp/VM:
- Cold-start 4 task cùng authority: 1 connection/4 stream, server chỉ trả khi đủ 4.
- Redirect + Set-Cookie: 4 request tuần tự, 1 connection/5 stream, cookie gửi lại.
- Cap 8 MiB + 1 mỗi 5 vòng.
- Timeout 400 ms.
- GOAWAY trước accept.

Batch ~30 vòng: fd `/proc/self/fd`, RSS `VmRSS`, threads `Threads`, số vòng scenario,
connection/stream server-side, cold p50/p95. Fail bất kỳ ⇒ test dừng ngay.

## 2. Kết quả 2 giờ

**PASS** — **7200,37 s**, **11.052 vòng**, 0 fail (dừng đúng hạn):

```text
soak KẾT THÚC: vòng=11052 cold=11052 redirect=11052 cap=2211 timeout=11052 goaway=11052
giây=7200.3 fd=8->9 rss=4.1->5.9MB threads=6->6
conns(cold/thuong/treo/goaway)=11052/13263/11052/11052
streams(cold/thuong)=44208/57471 cold_p50=7ms cold_p95=27ms
soak: PASS
```

- Mỗi vòng: cold **1 connection/4 stream**, redirect **1/5**, cap 1/1, timeout/GOAWAY 1 connection;
  counter server-side khớp tuyệt đối.
- **Không rò**: fd **8→9** (giữ 9 suốt 2 giờ), RSS **4,1→5,9 MB**, threads **6→6**.
- Cold p50 **7 ms**, p95 **27–48 ms** (release, target riêng).

## 3. Mẫu mỗi 10 phút

| Thời điểm | Vòng | fd | RSS (MB) | Threads | Cold p50/p95 (ms) |
|---:|---:|---:|---:|---:|---:|
| 10m (601 s) | 900 | 9 | 7,1 | 6 | 7 / 33 |
| 20m (1200 s) | 1.680 | 9 | 5,7 | 6 | 7 / 48 |
| 30m (1824 s) | 2.667 | 9 | 5,7 | 6 | 7 / 47 |
| 40m (2415 s) | 3.630 | 9 | 5,7 | 6 | 7 / 33 |
| 50m (3008 s) | 4.530 | 9 | 5,8 | 6 | 7 / 33 |
| 60m (3622 s) | 5.490 | 9 | 5,8 | 6 | 7 / 31 |
| 70m (4210 s) | 6.420 | 9 | 5,8 | 6 | 7 / 29 |
| 80m (4821 s) | 7.380 | 9 | 5,8 | 6 | 7 / 28 |
| 90m (5404 s) | 8.250 | 9 | 5,9 | 6 | 7 / 28 |
| 100m (6014 s) | 9.210 | 9 | 5,9 | 6 | 7 / 28 |
| 110m (6617 s) | 10.170 | 9 | 5,8 | 6 | 7 / 27 |

## 4. Regression cuối soak

| Cấu hình | Kết quả |
|---|---|
| feature `vs3_http2` | **34 pass + 1 ignored** (16,00 s) |
| default `vs3_http2` | **19/19** (15,52 s) |
| cold-start 1.000 vòng song song | **PASS** (17,02 s; 1 connection/4 stream mỗi vòng) |
| feature `vs3_tls` | **21/21** (6,22 s) |
| fmt / clippy default + feature / `./vis-dev check` | **OK / PASS** |

## 5. R1 / df / target

- `async_net.rs` **1490** (≤1500), `tests/support/h2_async_spike.rs` 987, `vs3_http2.rs` 884;
  các module h2/nguon/host đều nhỏ.
- `df`: **22 G trống** (>5 G).
- Target: **`$PWD/target/w2`** (không dùng target chung).

## 6. Kết luận / blocker

- **2 giờ soak release PASS**, không lỗi, tài nguyên phẳng, multiplex/cap/timeout/GOAWAY/cookie
  ổn định qua 11.052 vòng.
- Toàn bộ regression và check xanh; **không sửa production**, feature vẫn **default-off**.
- **Merge-eligible**. Blocker còn mở ngoài phạm vi soak: cancel task giữa request (nợ chung h1+h2)
  và async-h2 trên Windows. Không push/merge/tag theo lệnh.
