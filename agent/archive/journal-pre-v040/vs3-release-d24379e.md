<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 release-mode Linux + 2× soak — `d24379e` / tag v0.3.4

> **Người chạy**: W3 (QA/release), 2026-11-22 10:51–11:34 JST (~43 phút, time-box 1,5 h).
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| `main`/HEAD | **`d24379e`** (tag v0.3.4 code + flake journal) |
| Debug gate | **3571 passed / 3 skipped** |
| Branch QA | `vs3/w3-registry`, fast-forward `d24379e` |
| Toolchain | `~/.cargo/bin/cargo` 1.100.0 |
| Disk | đầu **22G**; thấp nhất **12G** (> 5G); cuối **21G** |

## 1. Release workspace

`cargo test --release --workspace --no-fail-fast` (target riêng `target/qa-w3-rel-d243`,
`CARGO_INCREMENTAL=0`):

| Chỉ số | Giá trị |
|---|---|
| **Passed** | **3576** |
| Failed | **0** |
| Ignored | **3** |
| Test binary / result block | **345 / 353** |
| Thời gian | **425,34 s** |
| Log | `/tmp/opencode/qa-w3-rel-d243-workspace.log` |

- So với debug **3571/3**: release +5 pass (5 doctest/target), 0 fail.
- **Ca debug-only** (tự skip ở release, `if !cfg!(debug_assertions)`): **9 chỗ** trong test —
  `vs3_tls` (6), `vs3_registry_https` (2), `vs3_http2` (1); cần hook root chỉ có ở binary debug.
  Không phải fail.
- Không có `test result: FAILED`, không panic.

## 2. Release targeted

| Test | Kết quả | Thời gian |
|---|---|---|
| `vs3_worker_eval_vm` | **10 passed** | 2,10 s |
| `vs3_worker_eval_parallel` (P2) | **14 passed** | 1,44 s |
| `vs3_worker_eval_local` (P1) | **8 passed** | 0,35 s |
| `vs3_http2` | **19 passed** | 15,76 s |
| `vs3_worker` | **14 passed** | 11,38 s |

- Log: `/tmp/opencode/qa-w3-rel-d243-targeted.log`.
- Tất cả dùng binary release; không test đỏ.

## 3. 2× soak 10/10

Mỗi lần target riêng `target/qa-w3-d243-soakN`, debug=0/incremental=0; xoá sau mỗi lần.

| Lần | Tổng | Gate ×3 | Host | Kết quả | Log |
|---|---|---|---|---|---|
| 1 | **970 s** | 3571/3 | 54/54 | 10/10 OK | `/tmp/opencode/qa-w3-d243-soak1-soak.log` |
| 2 | **1072 s** | 3571/3 | 54/54 | 10/10 OK | `.../qa-w3-d243-soak2-soak.log` |
| **Tổng** | **2042 s (~34 phút)** | — | — | **20/20 vòng OK** | — |

- Cả 2 flake đã từng gặp đều xanh trong **6/6 gate**:
  `cancel_pre_run_tra_loi_ro` PASS (0,014–0,484 s),
  `vm_khong_tao_them_thread_sau_khi_join` PASS (0,083–0,153 s).
- Không T-INFRA; target riêng.

## 4. Đánh giá

- **Release-mode Linux xanh**: workspace **3576/0/3**, targeted 10/14/8/19/14, host 54/54, 2× soak
  **20/20 vòng**, gate 3571/3.
- Release nhanh hơn debug đáng kể (425 s so với ~2 phút/gate debug + build; số đo chỉ tham khảo).
- Không cần tag mới; v0.3.4 đã có.

## 5. Blocker / còn lại

1. Không có blocker local.
2. 2 ô `EXPECTED_RED` msvc/wasm do #156 (C toolchain) — không đổi nhãn PASS.
3. Không GitHub Actions; Windows release-mode `d24379e` chưa đo (W4 verify v0.3.4 debug 3469/0).
4. 9 ca debug-only tự skip ở release (danh sách trên) — cần binary debug nếu muốn chạy.
5. Tàn dư đã biết: evaluator P3, async-h2 default-off, `select` VS3-2c — release notes.

## 6. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR=$PWD/target/qa-w3-rel-d243
export CARGO_INCREMENTAL=0

cargo test --release --workspace --no-fail-fast
for t in vs3_worker_eval_vm vs3_worker_eval_parallel vs3_worker_eval_local vs3_http2 vs3_worker; do
  cargo test --release -p vis-cli --test "$t"
done

for n in 1 2; do
  export CARGO_TARGET_DIR=$PWD/target/qa-w3-d243-soak$n
  export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
  ./bin/vis-soak --log /tmp/opencode/qa-w3-d243-soak$n-soak.log
  rm -rf "$CARGO_TARGET_DIR"
done
```

**Kết luận**: `d24379e` **release-mode Linux PASS** — workspace **3576/0/3**, targeted
10/14/8/19/14, 2× soak **20/20 vòng**, gate 3571/3, không flake/T-INFRA. Không có blocker local.
