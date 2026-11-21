<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W2 — soak/verify async-h2 (30 phút, test-only)

**Trạng thái**: soak/verify, **không sửa production**. Feature `async-h2` **default-off**; không
đổi API sync/default/dependency; không push/merge/tag.
**Nhánh**: `vs3/w2-tls`, merge main **`32cb511`**.
**Log soak**: [`vs3-w2-h2-soak.log`](vs3-w2-h2-soak.log).

## 1. Harness (test-only)

Thêm vào `crates/vis-cli/tests/support/h2_async_spike.rs`:

- Test `async_h2_soak_env_gated` — **env-gated**:
  - `VS3_W2_SOAK_SECS` (mặc định **5 s** để gate ngắn; soak thật **1800 s**).
  - `VS3_W2_SOAK_CAP_EVERY` (mặc định 5 — chạy cap 8 MiB mỗi 5 vòng).
- Mỗi vòng xen kẽ **interp/VM** (`i % 2`), gồm:
  1. **Cold-start**: 4 `spawn` `http.get_timeout` cùng authority, không warm-up; server `Cho4`
     chỉ trả sau khi nhận đủ **4 stream/1 connection**; assert `conns +1`, `streams +4`, output
     đúng 4 path; ghi wall time.
  2. **Redirect + Set-Cookie**: 4 request tuần tự (`/redir-cookie`, `/echo-cookie`,
     `/set-cookie`, `/echo-cookie`), assert output `sid=xyz/sid=xyz/set/sid=abc` và
     `conns +1`, `streams +5` (1 redirect = 2 stream).
  3. **Cap 8 MiB**: `GET /big` (8 MiB + 1) mỗi 5 vòng; assert `Err` cap, `conns +1`, `streams +1`.
  4. **Timeout 400 ms**: server treo; assert "thời gian", `conns +1`.
  5. **GOAWAY**: server GOAWAY trước accept; assert GOAWAY/HTTP2, `conns +1`.
- Mỗi **30 vòng** (hoặc 30 s) ghi một dòng batch: thời gian, fd (`/proc/self/fd`), RSS
  (`VmRSS`), threads (`Threads`), số vòng từng scenario, connection/stream server-side,
  cold p50/p95.
- Dừng ngay khi bất kỳ assert nào fail (không retry, không che lỗi).
- Cuối: assert tổng `conns/streams` khớp số vòng; `fd sau ≤ fd trước + 8`;
  `threads sau ≤ trước + 2`; `RSS tăng ≤ 256 MB` để bắt rò.

## 2. Kết quả soak 30 phút

**Soak DỪNG ở vòng 800 / 719,64 s** (12 phút, chưa đủ 30 phút) vì **FAIL** — đúng luật
"dừng ngay nếu fail":

```text
soak FAIL: cap vòng 800 vm=false: code=101 out="" err=...
error VS0501: vòng lặp scheduler quá giới hạn (200000 turn)
```

- 799 vòng trước đó **PASS** đủ 5 scenario; batch cuối (vòng 780 / 705 s):
  `fd=9`, `rss=9.9MB`, `threads=6`, cold=780, redirect=780, cap=156, timeout=780, goaway=780,
  `conns(cold/thuong)=780/936`, `streams=3120/4056`, `cold p50=13ms p95=63ms`.
- **Không rò** trong 780 vòng đầu: fd **9→9**, RSS **9.9 MB ổn định**, threads **6→6**;
  connection/stream khớp tuyệt đối từng vòng (cold 1 conn/4 stream; redirect 1 conn/5 stream;
  cap 1/1; timeout/GOAWAY 1 conn).
- Lỗi chỉ xuất hiện ở **cap 8 MiB** và không tất định (xem §3): có lần VS0501, có lần
  `"phản hồi HTTP không đủ header (kết nối đóng sớm)"` (đường h1) — tức race trong xử lý cap
  async-h2, không phải rò tài nguyên.

## 3. 1000 vòng cold-start ngắn

- `VS3_W2_COLD_ROUNDS=1000 ... async_h2_cold_multiplex_20_vong`: **1000/1000 PASS**,
  tổng **12,69 s** (~**12,7 ms/vòng**, gồm spawn process interp/VM xen kẽ).
- Cộng với 780 vòng cold trong soak: **1780 mẫu cold-start**, 0 fail.
- p50/p95 cold trong soak: **p50 13 ms, p95 63–80 ms** (debug, máy có tải ngoài).
- Mỗi vòng đúng **1 connection + 4 stream** server-side.

**Chẩn đoán riêng cap** (`async_h2_cap_loop_env_gated`, test-only):

| Lần | Số vòng đặt | Fail tại | Lỗi |
|---|---:|---:|---|
| 1 | 300 | **128** | `phản hồi HTTP không đủ header (kết nối đóng sớm)` |
| 2 | 200 | **2** | `VS0501: vòng lặp scheduler quá giới hạn (200000 turn)` |
| 3 | 200 | **20** | `VS0501: vòng lặp scheduler quá giới hạn (200000 turn)` |

⇒ **3/3 lần fail** trong 2–128 request cap liên tiếp. Đây là **blocker production**, không phải
flaky của harness (mỗi request là một process `vis run` mới).

## 4. Regression sau soak

| Cấu hình | Kết quả |
|---|---|
| feature `async-h2` `vs3_http2` | **34 pass, 0 fail, 1 ignored** (soak/cap diagnostic mặc định tắt) |
| default `vs3_http2` | **19/19** |
| feature `vs3_tls` | **21/21** |
| feature `vs3_default async_http` | **9/9** |
| `fmt` / clippy default+feature / `./vis-dev check` | **OK / PASS** |

Lưu ý: `async_h2_soak_env_gated` mặc định 5 s và **không chạm cap** (`CAP_EVERY=50`);
`async_h2_cap_loop_env_gated` mặc định **0 vòng** — chỉ chạy khi đặt env, để gate không flaky
bởi bug cap đang mở.

## 5. R1 / df

- `vs3_http2.rs` 884 dòng; `tests/support/h2_async_spike.rs` 955; `async_net.rs` 1477;
  `rt.rs` 1246 — **mọi tệp ≤ 1500**.
- `df`: **13 G trống** (> 5 G).

## 6. Khuyến nghị bật `async-h2` default-on

**KHÔNG bật `async-h2` default-on ở thời điểm này.**

Lý do chặn:
1. **Cap 8 MiB không ổn định**: 3/3 lần chạy lặp fail (VS0501 hot-spin hoặc rơi về lỗi h1),
   trong khi soak 30 phút dừng ở vòng 800. Đây là đường production (`http.get` body lớn) nên
   không thể default-on.
2. **Cancel task giữa request** vẫn là nợ chung h1+h2.
3. Chưa soak **release** 30 phút; soak hiện là debug và đã fail.

Điều kiện tối thiểu trước khi xem lại:
- Root-cause + fix hot-spin cap (nghi vấn: `ReactorOp::ReadWrite` đăng ký READ|WRITE trong khi
  h2 chỉ cần một chiều ⇒ epoll WRITABLE level-trigger quay frame tới 200k turn; thử
  interest theo `H2Async::interest()` thay vì luôn cả hai — **chưa sửa production** theo yêu cầu).
- Cap loop **≥ 10.000 vòng** + soak 30 phút release xanh.
- Hook cancel safe-point cho h1+h2.
- Quyết định Windows (feature vẫn `#![cfg(unix)]`).

## 7. Còn mở / blocker

- Cancel task giữa request: nợ chung h1+h2 (scheduler cancel test-only, `DANG_CHAY` chưa có
  safe-point cleanup) — **điều kiện chặn default-on** nếu chấp nhận rò session khi huỷ task.
- Windows: `async-h2` vẫn `#![cfg(unix)]`.
- Multi-waiter cùng một reactor key: không dùng (mỗi frame `H<n>#<frame>`); nếu bỏ `#frame` cần
  fix scheduler ~80–120 dòng.
- Soak chạy **debug** build; chưa soak release 30 phút (nên chạy trước khi default-on).
