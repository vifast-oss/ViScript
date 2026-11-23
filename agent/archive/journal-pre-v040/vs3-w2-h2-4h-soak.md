<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W2 — soak dài 4 giờ async-h2 (release, test-only)

**Trạng thái**: soak/hardening, **không sửa production/default/API/dependency**; feature
`async-h2` vẫn **default-off**; không GitHub Actions; không push/merge/tag.
**Nhánh**: `vs3/w2-tls`, merge main **`d1b2f52`**.
**Target riêng**: `$PWD/target/w2` (không dùng target chung).
**Log**: [`vs3-w2-h2-4h-soak.log`](vs3-w2-h2-4h-soak.log).

## 1. Cấu hình

```bash
CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true CARGO_TARGET_DIR=$PWD/target/w2 \
VS3_W2_SOAK_SECS=14400 VS3_W2_SOAK_CAP_EVERY=5 \
cargo test --release -p vis-cli --features async-h2 --test vs3_http2 \
  async_h2_soak_env_gated -- --nocapture
```

Mỗi vòng xen kẽ interp/VM: cold 4 task/1 connection/4 stream; redirect+cookie 1 conn/5 stream;
cap 8 MiB mỗi 5 vòng; timeout 400 ms; GOAWAY. Batch ~30 vòng ghi fd/RSS/threads, counter
connection/stream, cold p50/p95. Fail bất kỳ ⇒ dừng ngay.

## 2. Kết quả 4 giờ

**PASS** — **14.400,30 s (4 giờ)**, **21.807 vòng**, 0 fail:

```text
soak KẾT THÚC: vòng=21807 cold=21807 redirect=21807 cap=4362 timeout=21807 goaway=21807
giây=14400.2 fd=8->9 rss=3.8->6.0MB threads=6->6
conns(cold/thuong/treo/goaway)=21807/26169/21807/21807
streams(cold/thuong)=87228/113397 cold_p50=7ms cold_p95=27ms
soak: PASS
```

- Cold **1 connection/4 stream**, redirect **1/5**, cap 1/1, timeout/GOAWAY 1 conn — khớp tuyệt đối.
- **Không rò**: fd **8→9** (9 suốt 4 giờ), RSS **3,8→6,0 MB**, threads **6→6**.
- Cold p50 **7 ms**, p95 **26–39 ms** (release, target riêng).

## 3. Mẫu mỗi 10 phút

Full mẫu 10 phút nằm trong log; bảng dưới trích mỗi 30 phút:

| Thời điểm | Vòng | fd | RSS (MB) | Threads | Cold p50/p95 (ms) |
|---:|---:|---:|---:|---:|---:|
| 10m | 930 | 9 | 5,6 | 6 | 7 / 31 |
| 30m | 2.670 | 9 | 5,5 | 6 | 7 / 35 |
| 60m | 5.340 | 9 | 5,6 | 6 | 7 / 31 |
| 90m | 8.070 | 9 | 5,7 | 6 | 7 / 28 |
| 120m | 10.710 | 9 | 5,7 | 6 | 7 / 28 |
| 150m | 13.470 | 9 | 5,8 | 6 | 7 / 27 |
| 180m | 16.290 | 9 | 5,8 | 6 | 7 / 27 |
| 210m | 19.050 | 9 | 6,0 | 6 | 7 / 26 |
| 230m | 20.871 | 9 | 6,0 | 6 | 7 / 27 |

## 4. Regression cuối soak

| Cấu hình | Kết quả |
|---|---|
| feature `vs3_http2` | **34 pass + 1 ignored** (16,18 s) |
| default `vs3_http2` | **19/19** (15,62 s) |
| cold-start 1.000 vòng song song | **PASS** (20,43 s; 1 conn/4 stream mỗi vòng) |
| feature `vs3_tls` | **21/21** (6,44 s) |
| fmt / clippy default + feature / `./vis-dev check` | **OK / PASS** |

## 5. R1 / df / target

- `async_net.rs` **1490** (≤1500), `h2_async_spike.rs` 987, `vs3_http2.rs` 884; các module
  h2/nguon/host nhỏ.
- `df`: **22 G trống** (>5 G).
- Target: **`$PWD/target/w2`** (không dùng target chung).

## 6. Kết luận / blocker

- **4 giờ soak release PASS**, 21.807 vòng, không lỗi; tài nguyên phẳng, multiplex/cap/timeout/
  GOAWAY/cookie ổn định.
- Regression + check xanh; **không sửa production**, feature vẫn **default-off**.
- **Merge-eligible**. Còn mở ngoài phạm vi: cancel task giữa request (nợ chung h1+h2), async-h2
  Windows. Không push/merge/tag theo lệnh.
