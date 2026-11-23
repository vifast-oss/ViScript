<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 QA đóng gói — `1edec5d` (fresh-clone + 1000 cancel + 2× soak + diff)

> **Người chạy**: W3 (QA/release), 2026-11-22 11:34–12:25 JST (~51 phút, time-box 2 h).
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| `main` | **`7d48b36`** (merge release QA `d24379e`; code cancel fix = **`1edec5d`**) |
| Commit đo | **`1edec5dc6140876b13cd1540234be8dec6647f34`** — `merge VS3 W5ai: fix cancel-before race (pre-cancel flag)` |
| Branch QA | `vs3/w3-registry`, merge `main` → **`7d48b36`** |
| Code | `git diff --quiet 1edec5d 7d48b36 -- crates/` → **giống hệt** (chỉ journal khác) |
| Gate kỳ vọng | **3571/3** |
| Toolchain | `~/.cargo/bin/cargo` 1.100.0 |
| Disk | đầu **22G**; thấp nhất **20G** (> 5G); cuối **22G**; clone 1,6G đã xoá |

## 1. Fresh-clone `1edec5d`

- Clone `/home/ps/vis-qa-1edec5d`, checkout `CHECKOUT=1edec5dc6140876b13cd1540234be8dec6647f34`.
- Target nội bộ tên **`target`**, `CARGO_INCREMENTAL=0`, `debug=0` (tiết kiệm đĩa).
- Log: `/tmp/opencode/qa-w3-1ede-clone.log`; clone 1,6G đã xoá sau đo.

| Lệnh | Kết quả | Thời gian |
|---|---|---|
| `./vis-dev gate --fast` | **PASS — 3571 passed (1 slow) / 3 skipped** | 212,79 s (test 101,55 s) |
| `vs3_worker_eval_parallel` (P2) | **14 passed** | 6,83 s (test 2,25 s) |
| `vs3_worker_eval_local` (P1) | **8 passed** | 0,84 s (0,42 s) |
| `vs3_worker_eval_vm` (P3) | **10 passed** | 5,57 s (5,15 s) |
| `vs3_http2` default | **19 passed** | 16,77 s (15,55 s) |
| `vs3_http2 --features async-h2` | **34 passed / 1 ignored** (sau flake lần đầu) | 22,74 s (16,01 s) |
| `vs3_default` | **22 passed** | 2,85 s (2,08 s) |
| `./bin/vis-host-suite` | **54/54** + `host_posix_real` | 1,71 s |

### Flake feature h2 (test helper, không phải sản phẩm)

- Lần chạy đầu `--features async-h2`: **33 passed / 1 failed / 1 ignored**;
  `h2_async_spike::async_h2_soak_env_gated` panic tại
  `crates/vis-cli/tests/support/h2_async_spike.rs:425`:
  `Os { code: 26, kind: ExecutableFileBusy, message: "Text file busy" }`.
- **3 lần retry liền sau: 34 passed / 0 failed / 1 ignored** (16,06–16,30 s), log
  `/tmp/opencode/qa-w3-1ede-h2retry.log`.
- Kết luận: race copy/exec binary snapshot của test helper dưới tải (ETXTBSY), **không tái diễn**;
  không phải lỗi runtime `1edec5d`. Cần theo dõi nếu lặp lại.

## 2. Cancel-before 1.000 lần

`cargo test -q -p vis-cli --test vs3_worker_eval_parallel cancel_pre_run_tra_loi_ro -- --exact`
lặp **1.000 lần** trong clone (load đầu 15,54/15,91/15,26):

| Chỉ số | Giá trị |
|---|---|
| Số lần | **1.000** |
| Pass | **1.000** |
| Fail | **0** |
| Thời gian | 11:39:13–11:41:20 JST (~127 s) |
| Log | `/tmp/opencode/qa-w3-1ede-cancel1000.log` (chi tiết: `...-runs.log`) |

- Không lần nào `out.exit = 0` sai; fix pre-cancel flag ổn định 1.000/1.000 dưới tải cao.

## 3. 2× soak 10/10

Soak chạy trên HEAD `7d48b36`; `crates/` **giống hệt** `1edec5d` (đã kiểm `git diff`).
Mỗi lần target riêng `target/qa-w3-1ede-soakN`, debug=0/incremental=0; xoá sau mỗi lần.

| Lần | Tổng | Gate ×3 | Host | Kết quả | Log |
|---|---|---|---|---|---|
| 1 | **1087 s** | 3571/3 (×3) | 54/54 | 10/10 OK | `/tmp/opencode/qa-w3-1ede-soak1-soak.log` |
| 2 | **1204 s** | 3571/3 (×3) | 54/54 | 10/10 OK | `.../qa-w3-1ede-soak2-soak.log` |
| **Tổng** | **2291 s (~38,2 phút)** | — | — | **20/20 vòng OK** | — |

- **Cancel** `cancel_pre_run_tra_loi_ro`: **6/6 gate PASS** (0,014–0,036 s).
- **VM thread** `vm_khong_tao_them_thread_sau_khi_join`: **6/6 gate PASS** (0,066–0,116 s);
  không `rò thread`, không `2 -> 4/5`.
- Không T-INFRA; target riêng, load cuối soak 2 tới 22,40.

## 4. Diff 100k

`VIS_DIFF_N=100000 VIS_DIFF_SEED=202611221222 ./bin/vis-diff`

| Chỉ số | Giá trị |
|---|---|
| Tổng | **100.000** |
| Chạy | 93.771 |
| Lỗi tĩnh | 6.229 |
| **Lệch** | **0** |
| Thời gian | 165,64 s (real 178,24 s; tải cao) |
| Log | `/tmp/opencode/qa-w3-1ede-diff.log` |

## 5. Đánh giá

- Fresh-clone gate **3571/3**, P2 14, P1 8, P3 10, h2 default 19, h2 feature 34+1, default 22,
  host 54/54 — khớp yêu cầu.
- **Cancel 1.000/1.000 PASS**; **2× soak 20/20 vòng**, cancel 6/6 + VM 6/6 gate PASS;
  diff 100k **lệch 0**.
- Không có lỗi sản phẩm; chỉ 1 flake test-helper h2 (ETXTBSY) lần đầu, 3 retry + 6 gate sau đều xanh.

## 6. Blocker / còn lại

1. **Theo dõi**: `h2_async_spike::async_h2_soak_env_gated` ETXTBSY 1/4 lần chạy feature h2 —
   test helper copy/exec (`h2_async_spike.rs:425`), không phải runtime; chưa sửa code trong QA.
2. 2 ô `EXPECTED_RED` msvc/wasm do #156 C toolchain — không đổi nhãn PASS.
3. Không GitHub Actions; Windows `1edec5d` chưa đo phiên này.
4. Tàn dư đã biết: evaluator P3, async-h2 default-off, `select` VS3-2c — release notes.
5. Không có blocker local khác.

## 7. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
git clone . /home/ps/vis-qa-1edec5d && cd /home/ps/vis-qa-1edec5d
git checkout 1edec5d
export CARGO_TARGET_DIR=$PWD/target CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
./vis-dev gate --fast
cargo test -p vis-cli --test vs3_worker_eval_parallel
cargo test -p vis-cli --test vs3_worker_eval_local
cargo test -p vis-cli --test vs3_worker_eval_vm
cargo test -p vis-cli --test vs3_http2
cargo test -p vis-cli --features async-h2 --test vs3_http2
cargo test -p vis-cli --test vs3_default
./bin/vis-host-suite
for n in $(seq 1 1000); do
  cargo test -q -p vis-cli --test vs3_worker_eval_parallel cancel_pre_run_tra_loi_ro -- --exact || break
done

for n in 1 2; do
  export CARGO_TARGET_DIR=$PWD/target/qa-w3-1ede-soak$n
  ./bin/vis-soak --log /tmp/opencode/qa-w3-1ede-soak$n-soak.log
  rm -rf "$CARGO_TARGET_DIR"
done

VIS_DIFF_N=100000 VIS_DIFF_SEED=202611221222 ./bin/vis-diff
```

**Kết luận**: `1edec5d` **xanh toàn bộ** — fresh-clone 3571/3, cancel **1000/1000**, 2× soak
**20/20 vòng** (2291 s), cancel 6/6 + VM 6/6 gate PASS, diff 100k **lệch 0**. Chỉ còn flake
test-helper h2 ETXTBSY (1 lần, 3 retry xanh) cần theo dõi; không có blocker sản phẩm local.
