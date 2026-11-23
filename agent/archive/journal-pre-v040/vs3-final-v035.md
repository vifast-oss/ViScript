<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 QA đóng gói candidate `v0.3.5` — `7e72afd` (fresh-clone + 1000 cancel + h2×3 + 2× soak + diff)

> **Người chạy**: W3 (QA/release), 2026-11-22 12:35–13:20 JST (~45 phút, time-box 2 h).
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| `main`/HEAD | **`7e72afd`** — cancel fix + net test bound + h2 snapshot `OnceLock` (hết ETXTBSY) |
| Commit đo | **`7e72afd7256264137926cf71dd4617261dc12a25`** |
| Candidate | **`v0.3.5`** (chưa tag trong phiên này) |
| Branch QA | `vs3/w3-registry`, merge `main` → **`7e72afd`** |
| Gate kỳ vọng | **3571/3** |
| Toolchain | `~/.cargo/bin/cargo` 1.100.0 |
| Disk | đầu **20G**; thấp nhất **19G** (> 5G); cuối **21G**; clone 1,6G đã xoá |

## 1. Fresh-clone `7e72afd`

- Clone `/home/ps/vis-qa-v035`, checkout `CHECKOUT=7e72afd7256264137926cf71dd4617261dc12a25`.
- Target nội bộ tên **`target`**, `CARGO_INCREMENTAL=0`, `debug=0` (tiết kiệm đĩa).
- Log: `/tmp/opencode/qa-w3-v035-clone.log`; clone 1,6G đã xoá sau đo.

| Lệnh | Kết quả | Thời gian |
|---|---|---|
| `./vis-dev gate --fast` | **PASS — 3571 passed (1 slow) / 3 skipped** | 215,29 s (test 105,11 s) |
| `vs3_worker_eval_parallel` (P2) | **14 passed** | 7,05 s (test 2,19 s) |
| `vs3_worker_eval_local` (P1) | **8 passed** | 0,67 s (0,30 s) |
| `vs3_worker_eval_vm` (P3) | **10 passed** | 5,61 s (5,25 s) |
| `vs3_http2` default | **19 passed** | 16,71 s (15,62 s) |
| `vs3_default` | **22 passed** | 2,69 s (2,00 s) |
| `./bin/vis-host-suite` | **54/54** + `host_posix_real` | 1,84 s |

### Feature h2 ×3 (xác nhận hết ETXTBSY)

`cargo test -p vis-cli --features async-h2 --test vs3_http2` lặp **3 lần** dưới tải
(load 14,40–15,53):

| Lần | Kết quả | Thời gian |
|---|---|---|
| 1 | **34 passed / 0 failed / 1 ignored** | 16,16 s |
| 2 | **34 passed / 0 failed / 1 ignored** | 16,08 s |
| 3 | **34 passed / 0 failed / 1 ignored** | 15,91 s |

- **3/3 xanh**, không còn `Text file busy`/ETXTBSY; log
  `/tmp/opencode/qa-w3-v035-h2x3.log`. Fix `snapshot_vis()` OnceLock có hiệu lực.

## 2. Cancel-before 1.000 lần

`cargo test -q -p vis-cli --test vs3_worker_eval_parallel cancel_pre_run_tra_loi_ro -- --exact`
lặp **1.000 lần** trong clone (load đầu 15,04/16,92/16,74):

| Chỉ số | Giá trị |
|---|---|
| Số lần | **1.000** |
| Pass | **1.000** |
| Fail | **0** |
| Thời gian | 12:40:43–12:42:47 JST (~124 s) |
| Log | `/tmp/opencode/qa-w3-v035-cancel1000.log` (chi tiết: `...-runs.log`) |

## 3. 2× soak 10/10

Mỗi lần target riêng `target/qa-w3-v035-soakN`, debug=0/incremental=0; xoá sau mỗi lần.

| Lần | Tổng | Gate ×3 | Host | Kết quả | Log |
|---|---|---|---|---|---|
| 1 | **1090 s** | 3571/3 (×3) | 54/54 | 10/10 OK | `/tmp/opencode/qa-w3-v035-soak1-soak.log` |
| 2 | **991 s** | 3571/3 (×3) | 54/54 | 10/10 OK | `.../qa-w3-v035-soak2-soak.log` |
| **Tổng** | **2081 s (~34,7 phút)** | — | — | **20/20 vòng OK** | — |

- **Cancel** `cancel_pre_run_tra_loi_ro`: **6/6 gate PASS** (0,016–0,026 s).
- **VM thread** `vm_khong_tao_them_thread_sau_khi_join`: **6/6 gate PASS** (0,071–0,131 s);
  không `rò thread`, không `2 -> 4/5`.
- **Net**: **78 PASS/lần soak, 0 FAIL** (gồm `net::tests::*`, `net::h2::tests::*`,
  `lat256_net`, `net_server`); `t8_write_timeout_is_bounded_and_cleans_up` **7/7 lần**
  (gate clone + 6 gate soak) PASS, 0,67–0,78 s.
- Không T-INFRA; target riêng.

## 4. Diff 100k

`VIS_DIFF_N=100000 VIS_DIFF_SEED=202611221318 ./bin/vis-diff`

| Chỉ số | Giá trị |
|---|---|
| Tổng | **100.000** |
| Chạy | 93.768 |
| Lỗi tĩnh | 6.232 |
| **Lệch** | **0** |
| Thời gian | 103,95 s (real 112,07 s) |
| Log | `/tmp/opencode/qa-w3-v035-diff.log` |

## 5. Đánh giá

- Fresh-clone gate **3571/3**, P2 14, P1 8, P3 10, h2 default 19, **h2 feature 34+1 ×3**,
  default 22, host 54/54 — khớp yêu cầu.
- **Cancel 1.000/1.000 PASS**; **2× soak 20/20 vòng**; cancel 6/6, VM 6/6, net 78/78 mỗi soak;
  diff 100k **lệch 0**.
- Không còn ETXTBSY; không flake/T-INFRA trong phiên. `7e72afd` đủ điều kiện làm **candidate v0.3.5**
  (việc tag/push do người có thẩm quyền quyết định, phiên này không làm).

## 6. Blocker / còn lại

1. Không có blocker local; không flake.
2. 2 ô `EXPECTED_RED` msvc/wasm do #156 C toolchain — không đổi nhãn PASS.
3. Không GitHub Actions; Windows `7e72afd` chưa đo phiên này.
4. Tàn dư đã biết: evaluator P3, async-h2 default-off, `select` VS3-2c — release notes.
5. Chưa tag `v0.3.5` (đúng phạm vi: không tag/push/merge).

## 7. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
git clone . /home/ps/vis-qa-v035 && cd /home/ps/vis-qa-v035
git checkout 7e72afd
export CARGO_TARGET_DIR=$PWD/target CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
./vis-dev gate --fast
cargo test -p vis-cli --test vs3_worker_eval_parallel
cargo test -p vis-cli --test vs3_worker_eval_local
cargo test -p vis-cli --test vs3_worker_eval_vm
cargo test -p vis-cli --test vs3_http2
cargo test -p vis-cli --test vs3_default
./bin/vis-host-suite
for n in 1 2 3; do
  cargo test -p vis-cli --features async-h2 --test vs3_http2
done
for n in $(seq 1 1000); do
  cargo test -q -p vis-cli --test vs3_worker_eval_parallel cancel_pre_run_tra_loi_ro -- --exact || break
done

for n in 1 2; do
  export CARGO_TARGET_DIR=$PWD/target/qa-w3-v035-soak$n
  ./bin/vis-soak --log /tmp/opencode/qa-w3-v035-soak$n-soak.log
  rm -rf "$CARGO_TARGET_DIR"
done

VIS_DIFF_N=100000 VIS_DIFF_SEED=202611221318 ./bin/vis-diff
```

**Kết luận**: `7e72afd` **xanh toàn bộ** — fresh-clone 3571/3, h2 feature **34+1 ×3** (hết ETXTBSY),
cancel **1000/1000**, 2× soak **20/20 vòng** (2081 s), cancel/VM 6/6 + net 78/78 mỗi soak,
diff 100k **lệch 0**. Không có blocker local; sẵn sàng làm candidate `v0.3.5`.
