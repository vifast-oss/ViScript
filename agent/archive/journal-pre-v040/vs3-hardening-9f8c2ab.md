<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 hardening — `9f8c2ab` (fresh-clone + 2× soak + diff)

> **Người chạy**: W3 (QA/hardening), 2026-11-22 02:38–03:14 JST (~36 phút, time-box 1,5 h).
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| `main`/HEAD | **`9f8c2ab`** (sau evaluator P2 `0b4ad7e`; Linux gate kỳ vọng **3561/3**) |
| Branch QA | `vs3/w3-registry`, merge `main` → **`9f8c2ab`** |
| Toolchain | `~/.cargo/bin/cargo` 1.100.0 |
| Disk | đầu **15G**; thấp nhất **13G** (> 5G); cuối **23G** (agent khác dọn) |

## 1. Fresh-clone `9f8c2ab`

- Clone `/home/ps/vis-qa-main9f8`, checkout `CHECKOUT=9f8c2ab148601c7079dc5b93a4946d52a99771a1`.
- Target nội bộ tên **`target`**, `CARGO_INCREMENTAL=0`, `debug=0` (tiết kiệm đĩa).
- Log: `/tmp/opencode/qa-w3-9f8-clone.log`; clone 1,5G đã xoá.

| Lệnh | Kết quả | Thời gian |
|---|---|---|
| `./vis-dev gate --fast` | **PASS — 3561 passed / 3 skipped** | 149,94 s |
| `cargo test -p vis-cli --test vs3_worker_eval_parallel` | **14 passed** | 5,12 s |
| `cargo test -p vis-cli --test vs3_worker_eval_local` | **8 passed** | 0,48 s |
| `cargo test -p vis-cli --test vs3_http2` | **19 passed** | 16,25 s |
| `cargo test -p vis-cli --features async-h2 --test vs3_http2` | **34 passed / 1 ignored** | 20,08 s |
| `cargo test -p vis-cli --test vs3_default` | **22 passed** | 2,56 s |
| `./bin/vis-host-suite` | **54/54** + `host_posix_real` | 1,35 s |

- Tất cả số ca khớp yêu cầu; không test đỏ trong các lệnh targeted.

## 2. 2× soak 10/10

### 2.1 Lần 1 (attempt) — FAIL gate-2 do flake test

- `target/qa-w3-9f8-soak1`, log `/tmp/opencode/qa-w3-9f8-soak1-soak.log`.
- gate-1 OK (150 s); **gate-2 FAIL sau 116 s**:
  `vs3_worker_eval_parallel::muoi_vong_threads_fd_rss_khong_ro` — `rò thread: 2 -> 4`.
- Dừng theo script (không retry trong cùng lần), **không** phải T-INFRA phần cứng nhưng là flake
  đo thread khi gate chạy song song; test targeted chạy riêng đã 14/14 PASS.
- Ghi nhận blocker flake (xem §5).

### 2.2 Hai lần chạy lại — PASS 10/10

| Lần | Tổng | Gate ×3 | Host | Kết quả | Log |
|---|---|---|---|---|---|
| 1 (retry) | **796 s** | 3561/3 | 54/54 | 10/10 OK | `/tmp/opencode/qa-w3-9f8-soak1b-soak.log` |
| 2 | **733 s** | 3561/3 | 54/54 | 10/10 OK | `/tmp/opencode/qa-w3-9f8-soak2-soak.log` |
| **Tổng** | **1529 s (~25,5 phút)** | — | — | **20/20 vòng OK** | — |

- Mỗi lần target riêng, debug=0/incremental=0, xoá sau chạy.
- Không T-INFRA; gate/host/diff ổn định.
- Wrapper: `/tmp/opencode/qa-w3-9f8-soak1b.log`, `...soak2.log`.

## 3. Diff 100k

`VIS_DIFF_N=100000 VIS_DIFF_SEED=202611220312 ./bin/vis-diff`

| Chỉ số | Giá trị |
|---|---|
| Tổng | **100.000** |
| Chạy | 93.770 |
| Lỗi tĩnh | 6.230 |
| **Lệch** | **0** |
| Thời gian | 78,59 s (real 86,02 s) |
| Log | `/tmp/opencode/qa-w3-9f8-diff.log` |

## 4. Đánh giá

- Fresh-clone + targeted + host + **2× soak** + diff xanh trong ~36 phút.
- Gate **3561/3**; worker eval **14+8**, h2 **19 + 34+1 ignored**, `vs3_default` **22**, host 54/54;
  soak **20/20 vòng**; diff **lệch 0**.
- **Một flake** ở lần soak đầu: `muoi_vong_threads_fd_rss_khong_ro` thread 2→4. Cần theo dõi; không
  tái diễn trong 2 lần soak sau.

## 5. Blocker / còn lại

1. **Flake gate**: `vs3_worker_eval_parallel::muoi_vong_threads_fd_rss_khong_ro` có thể đỏ khi chạy
   nextest song song (`rò thread: 2 -> 4`); targeted `--test-threads=1` 14/14 pass. Đề xuất W5/W1
   siết đo thread hoặc test chạy serial; **chưa sửa code** (ngoài phạm vi QA).
2. 2 ô `EXPECTED_RED` msvc/wasm do #156 C toolchain — không đổi nhãn PASS.
3. Không GitHub Actions; Windows `9f8c2ab` chưa đo trong phiên này.
4. Evaluator P2 mới; worker/HTTP async vẫn cần release notes; tàn dư đã biết.

## 6. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
git clone . /home/ps/vis-qa-main9f8 && cd /home/ps/vis-qa-main9f8
git checkout 9f8c2ab
export CARGO_TARGET_DIR=$PWD/target CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
./vis-dev gate --fast
cargo test -p vis-cli --test vs3_worker_eval_parallel
cargo test -p vis-cli --test vs3_worker_eval_local
cargo test -p vis-cli --test vs3_http2
cargo test -p vis-cli --features async-h2 --test vs3_http2
cargo test -p vis-cli --test vs3_default
./bin/vis-host-suite

for n in 1 2; do
  export CARGO_TARGET_DIR=$PWD/target/qa-w3-9f8-soak$n
  ./bin/vis-soak --log /tmp/opencode/qa-w3-9f8-soak$n-soak.log
  rm -rf "$CARGO_TARGET_DIR"
done

VIS_DIFF_N=100000 VIS_DIFF_SEED=202611220312 ./bin/vis-diff
```

**Kết luận**: `9f8c2ab` **xanh** cho fresh-clone/targeted/host/2× soak/diff; gate 3561/3, soak
20/20 vòng, diff 100k lệch 0. **Một flake thread-leak** ở soak đầu là blocker cần theo dõi, không
tái diễn ở 2 lần sau.
