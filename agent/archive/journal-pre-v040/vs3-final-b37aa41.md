<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 QA đóng gói — `b37aa41` (fresh-clone + 2× soak + diff)

> **Người chạy**: W3 (QA/release), 2026-11-22 05:43–06:13 JST (~30 phút, time-box 1,5 h).
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| `main`/HEAD | **`b37aa41`** (code = `ca17dde` + journals; VM flake closed) |
| Gate kỳ vọng | **3571/3** |
| Branch QA | `vs3/w3-registry`, merge `main` → **`b37aa41`** |
| Toolchain | `~/.cargo/bin/cargo` 1.100.0 |
| Disk | đầu **23G**; thấp nhất **21G** (> 5G); cuối **23G** |

## 1. Fresh-clone `b37aa41`

- Clone `/home/ps/vis-qa-b37`, checkout `CHECKOUT=b37aa41c54fc331057ffb0c10f1447f7d7326a2f`.
- Target nội bộ tên **`target`**, `CARGO_INCREMENTAL=0`, `debug=0` (tiết kiệm đĩa).
- Log: `/tmp/opencode/qa-w3-b37-clone.log`; clone 1,6G đã xoá.

| Lệnh | Kết quả | Thời gian |
|---|---|---|
| `./vis-dev gate --fast` | **PASS — 3571 passed / 3 skipped** | 141,85 s |
| `cargo test -p vis-cli --test vs3_worker_eval_vm` | **10 passed** | 4,28 s |
| `cargo test -p vis-cli --test vs3_worker_eval_parallel` | **14 passed** | 1,47 s |
| `cargo test -p vis-cli --test vs3_worker_eval_local` | **8 passed** | 0,31 s |
| `cargo test -p vis-cli --test vs3_http2` | **19 passed** | 16,01 s |
| `cargo test -p vis-cli --features async-h2 --test vs3_http2` | **34 passed / 1 ignored** | 20,57 s |
| `cargo test -p vis-cli --test vs3_default` | **22 passed** | 2,51 s |
| `cargo test -p vis-cli --test vs3_tls` | **21 passed** | 6,76 s |
| `./bin/vis-host-suite` | **54/54** + `host_posix_real` | 1,14 s |

- Tất cả số ca khớp yêu cầu; fresh-clone sạch, không test đỏ.

## 2. 2× soak 10/10

Mỗi lần target riêng `target/qa-w3-b37-soakN`, debug=0/incremental=0, log mới; xoá target sau
từng lần.

| Lần | Tổng | Gate ×3 | Host | Kết quả | Log |
|---|---|---|---|---|---|
| 1 | **731 s** | 3571/3 | 54/54 | 10/10 OK | `/tmp/opencode/qa-w3-b37-soak1-soak.log` |
| 2 | **726 s** | 3571/3 | 54/54 | 10/10 OK | `.../qa-w3-b37-soak2-soak.log` |
| **Tổng** | **1457 s (~24,3 phút)** | — | — | **20/20 vòng OK** | — |

- Mỗi lần: gate ×3, diff ×3 (seed cố định `20261115`), host ×2, cache ×2.
- **VM thread test** `vm_khong_tao_them_thread_sau_khi_join`: **6/6 gate PASS**, 0,070–0,132 s;
  không còn `2 -> 4/5`.
- Không T-INFRA; target riêng.
- Wrapper: `/tmp/opencode/qa-w3-b37-soakN.log`.

## 3. Diff 100k

`VIS_DIFF_N=100000 VIS_DIFF_SEED=202611220611 ./bin/vis-diff`

| Chỉ số | Giá trị |
|---|---|
| Tổng | **100.000** |
| Chạy | 93.767 |
| Lỗi tĩnh | 6.233 |
| **Lệch** | **0** |
| Thời gian | 95,25 s (real 106,11 s) |
| Log | `/tmp/opencode/qa-w3-b37-diff.log` |

## 4. Đánh giá

- **Fresh-clone + targeted + host + 2× soak + diff đều xanh** trong ~30 phút.
- Gate **3571/3**, VM 10, P2 14, P1 8, h2 19 + 34+1 ignored, default 22, TLS 21, host 54/54.
- Soak **20/20 vòng**, VM thread test **6/6 gate PASS**; diff 100k **lệch 0**.
- Không cần tag mới; `v0.3.3` đã tag; đây là QA đóng gói hardening.

## 5. Blocker / còn lại

1. Không có blocker local; flake VM đã đóng (6/6 gate + 20/20 lần trước đó).
2. 2 ô `EXPECTED_RED` msvc/wasm do #156 C toolchain — không đổi nhãn PASS.
3. Không GitHub Actions; Windows `b37aa41` chưa đo trong phiên này (W4 đã verify v0.3.3 3469/0).
4. Tàn dư đã biết: evaluator P3 mới, async-h2 default-off, `select` VS3-2c — release notes.

## 6. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
git clone . /home/ps/vis-qa-b37 && cd /home/ps/vis-qa-b37
git checkout b37aa41
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

for n in 1 2; do
  export CARGO_TARGET_DIR=$PWD/target/qa-w3-b37-soak$n
  ./bin/vis-soak --log /tmp/opencode/qa-w3-b37-soak$n-soak.log
  rm -rf "$CARGO_TARGET_DIR"
done

VIS_DIFF_N=100000 VIS_DIFF_SEED=202611220611 ./bin/vis-diff
```

**Kết luận**: `b37aa41` **xanh toàn bộ** — fresh-clone gate 3571/3, VM/P2/P1/h2/default/TLS/host
khớp yêu cầu, 2× soak **20/20 vòng OK** (1457 s), VM thread test 6/6 gate PASS, diff 100k
**lệch 0**. Không có blocker kỹ thuật local.
