<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W2 — soak cuối async-h2 (release 30 phút) + cap 10k

**Trạng thái**: soak/verify test-only, **không sửa production**; feature `async-h2` vẫn
**default-off**; không đổi API/default/dependency; không GitHub Actions; không push/merge/tag.
**Nhánh**: `vs3/w2-tls`, merge main **`94b1b91`**.
**Target riêng**: `$PWD/target/w2` (không dùng target chung).
**Log**: [`vs3-w2-h2-final-soak.log`](vs3-w2-h2-final-soak.log).

## 1. Cấu hình soak

`async_h2_soak_env_gated` (test-only, env-gated), release + `debug-assertions=true` (chỉ để
`VIS_TLS_TEST_ROOT_DER` hoạt động với cert fixture):

```bash
CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true CARGO_TARGET_DIR=$PWD/target/w2 \
VS3_W2_SOAK_SECS=1800 VS3_W2_SOAK_CAP_EVERY=5 \
cargo test --release -p vis-cli --features async-h2 --test vs3_http2 \
  async_h2_soak_env_gated -- --nocapture
```

Mỗi vòng xen kẽ interp/VM:
1. Cold-start 4 task cùng authority (1 connection/4 stream, server chỉ trả khi đủ 4).
2. Redirect + Set-Cookie (4 request tuần tự, 5 stream; cookie gửi lại).
3. Cap 8 MiB + 1 mỗi 5 vòng.
4. Timeout 400 ms.
5. GOAWAY trước accept.

Batch 30 vòng: fd (`/proc/self/fd`), RSS (`VmRSS`), threads (`Threads`), số vòng từng scenario,
connection/stream server-side, cold p50/p95. Dừng ngay nếu fail.

## 2. Kết quả soak 30 phút

**PASS** — 1800,60 s, **2.811 vòng**, dừng đúng hạn (không fail):

```text
soak KẾT THÚC: vòng=2811 cold=2811 redirect=2811 cap=563 timeout=2811 goaway=2811
giây=1800.5 fd=8->9 rss=4.2->5.9MB threads=6->6
conns(cold/thuong/treo/goaway)=2811/3374/2811/2811
streams(cold/thuong)=11244/14618 cold_p50=6ms cold_p95=29ms
soak: PASS
```

- Mỗi vòng đúng **1 connection cold + 4 stream**; redirect **1 conn + 5 stream**; cap 1/1; timeout
  và GOAWAY 1 connection (server-side counter khớp tuyệt đối).
- **Không rò**: fd **8→9** (hằng 9 suốt soak), RSS **4,2→5,9 MB**, threads **6→6**.
- Cold p50 **6 ms**, p95 **29 ms** (release, target riêng).

## 3. Cap 10.000 vòng (target sạch)

**PASS** — 10.000/10.000 vòng, **1947,80 s** (~5,2 vòng/s), 0 fail, trên
`CARGO_TARGET_DIR=$PWD/target/w2` (release + debug-assertions). Không còn VS0501/h1 fallback;
cap trả lỗi 8 MiB đúng mỗi vòng.

## 4. Regression

| Cấu hình | Kết quả |
|---|---|
| feature `vs3_http2` song song ×2 | **34 pass + 1 ignored** (16,04 s / 16,12 s) |
| default `vs3_http2` | **19/19** |
| feature `vs3_tls` | **21/21** |
| feature `vs3_default async_http` | **9/9** |
| cold-start 1.000 vòng song song | **PASS** (33,30 s; mỗi vòng 1 connection/4 stream) |
| fmt / clippy default + feature / `./vis-dev check` | **OK / PASS** |

## 5. R1 / df / target

- `async_net.rs` **1490** (≤1500), `async_net_h2.rs` 176, `async_net_nguon.rs` 312,
  `async_net_host.rs` 111, `h2_async_spike.rs` 987, `vs3_http2.rs` 884.
- `df`: **13 G trống** (>5 G).
- Target: **`$PWD/target/w2`** (không dùng target chung).

## 6. Kết luận / blocker

- **Soak release 30 phút PASS** (2.811 vòng; fd/RSS/threads phẳng; connection/stream khớp).
- **Cap 10.000 vòng PASS** trên target sạch; cold 1.000 vòng song song PASS.
- Toàn bộ regression xanh; feature vẫn **default-off**, không đổi production/default/API/dependency.
- **Đủ điều kiện merge**; không push/merge/tag theo lệnh. Còn mở (không chặn merge theo phạm vi
  soak): cancel task giữa request (nợ chung h1+h2), async-h2 Windows, và nên soak release định kỳ
  sau này.
