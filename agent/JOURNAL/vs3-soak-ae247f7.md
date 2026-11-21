<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 final QA — `ae247f7` (fresh-clone + 3× soak PASS, còn flake VM)

> **Người chạy**: W3 (QA/hardening), 2026-11-22 04:03–05:02 JST (~59 phút, time-box 2 h).
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| `main`/HEAD | **`ae247f7`** (P3 VM + flake fix; gate kỳ vọng **3571/3**) |
| Branch QA | `vs3/w3-registry`, merge `main` → **`ae247f7`** |
| Toolchain | `~/.cargo/bin/cargo` 1.100.0 |
| Disk | đầu **13G**; thấp nhất **12G** (> 5G); cuối **23G** (agent khác dọn) |

## 1. Fresh-clone `ae247f7`

- Clone `/home/ps/vis-qa-final`, checkout `CHECKOUT=ae247f74e461dd2ec3da7c72324aa2f018adaa38`.
- Target nội bộ tên **`target`**, `CARGO_INCREMENTAL=0`, `debug=0` (tiết kiệm đĩa).
- Log: `/tmp/opencode/qa-w3-ae247-clone.log`; clone 1,6G đã xoá.

| Lệnh | Kết quả | Thời gian |
|---|---|---|
| `./vis-dev gate --fast` | **PASS — 3571 passed / 3 skipped** | 157,38 s |
| `cargo test -p vis-cli --test vs3_worker_eval_vm` | **10 passed** | 13,06 s |
| `cargo test -p vis-cli --test vs3_worker_eval_parallel` | **14 passed** | 6,87 s |
| `cargo test -p vis-cli --test vs3_worker_eval_local` | **8 passed** | 2,62 s |
| `cargo test -p vis-cli --test vs3_http2` | **19 passed** | 19,96 s |
| `cargo test -p vis-cli --features async-h2 --test vs3_http2` | **34 passed / 1 ignored** | 25,77 s |
| `cargo test -p vis-cli --test vs3_default` | **22 passed** | 6,88 s |
| `cargo test -p vis-cli --test vs3_tls` | **21 passed** | 8,39 s |
| `./bin/vis-host-suite` | **54/54** + `host_posix_real` | 3,87 s |

- Tất cả số ca khớp yêu cầu; fresh-clone sạch.

## 2. 3× soak 10/10 (có 2 attempt flake trước đó)

| Lần | Tổng | Gate ×3 | Host | Kết quả | Log |
|---|---|---|---|---|---|
| attempt 1 | 267 s | gate-2 FAIL | — | **FAIL** — VM thread 2→5 | `/tmp/opencode/qa-w3-ae-soak1-soak.log` |
| attempt 2 | 556 s | gate-3 FAIL | — | **FAIL** — VM thread 2→4 | `.../ae-soak2-soak.log` |
| **run 3** | **902 s** | **3571/3** | 54/54 | **10/10 OK** | `.../ae-soak3-soak.log` |
| **run 4** | **690 s** | **3571/3** | 54/54 | **10/10 OK** | `.../ae-soak4-soak.log` |
| **run 5** | **702 s** | **3571/3** | 54/54 | **10/10 OK** | `.../ae-soak5-soak.log` |
| **3 lần PASS** | **2294 s (~38 phút)** | — | — | **30/30 vòng OK** | — |

- Flake còn lại: `vs3_worker_eval_vm::vm_khong_tao_them_thread_sau_khi_join` — attempt 1
  `rò thread: 2->5` (gate-2), attempt 2 `2->4` (gate-3). Target riêng nên không T-INFRA.
- Trong 3 lần PASS, test VM này pass **9/9 lần** (3 gate × 3 soak), nhưng flake vẫn tái diễn dưới tải.
- Soak dùng seed cố định `20261115`, host 54/54 mỗi lần.

## 3. Diff 100k

`VIS_DIFF_N=100000 VIS_DIFF_SEED=202611220436 ./bin/vis-diff`

| Chỉ số | Giá trị |
|---|---|
| Tổng | **100.000** |
| Chạy | 93.770 |
| Lỗi tĩnh | 6.230 |
| **Lệch** | **0** |
| Thời gian | 89,44 s (real 97,20 s) |
| Log | `/tmp/opencode/qa-w3-ae-diff.log` |

## 4. Đánh giá

- Fresh-clone + targeted + host + **3 lần soak 10/10** + diff đều xanh trong ~59 phút.
- Gate **3571/3**, VM 10, parallel 14, local 8, h2 19 + 34+1 ignored, default 22, TLS 21, host 54/54;
  soak **30/30 vòng** (3 lần pass); diff **lệch 0**.
- **Flake VM còn mở**: `vm_khong_tao_them_thread_sau_khi_join` đỏ 2/5 attempt dưới tải; fix P2
  (poll 500 ms) chưa áp cho VM hoặc VM cần join/poll tương tự.

## 5. Blocker / còn lại

1. **Blocker flake**: VM thread test `2->5`/`2->4` trong gate khi tải; cần W5/W1 sửa test/runtime
   tương tự P2. Không sửa code trong phiên QA.
2. 2 ô `EXPECTED_RED` msvc/wasm do #156 C toolchain.
3. Không GitHub Actions; Windows `ae247f7` chưa đo trong phiên này.
4. Tàn dư đã biết: evaluator P3 mới, async-h2 default-off, `select` VS3-2c — release notes.

## 6. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
git clone . /home/ps/vis-qa-final && cd /home/ps/vis-qa-final
git checkout ae247f7
export CARGO_TARGET_DIR=$PWD/target CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
./vis-dev gate --fast
cargo test -p vis-cli --test vs3_worker_eval_vm
cargo test -p vis-cli --test vs3_worker_eval_parallel
cargo test -p vis-cli --test vs3_worker_eval_local
cargo test -p vis-cli --test vs3_http2
cargo test -p vis-cli --features async-h2 --test vs3_http2
cargo test -p vis-cli --test vs3_default
cargo test -p vis-cli --test vs3_tls
./bin/vis-host-suite

for n in 1 2 3; do
  export CARGO_TARGET_DIR=$PWD/target/qa-w3-ae-soak$n
  ./bin/vis-soak --log /tmp/opencode/qa-w3-ae-soak$n-soak.log
  rm -rf "$CARGO_TARGET_DIR"
done

VIS_DIFF_N=100000 VIS_DIFF_SEED=202611220436 ./bin/vis-diff
```

**Kết luận**: `ae247f7` **fresh-clone + targeted + 3 lần soak + diff xanh**, gate 3571/3, diff 100k
lệch 0. **Còn flake VM thread** (2/5 attempt) là blocker cần W5/W1 xử lý; không có lỗi sản phẩm khác
trong phạm vi QA local.
