<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 soak 3×10/10 + VM 20 lần — `ca17dde`

> **Người chạy**: W3 (QA/hardening), 2026-11-22 05:06–05:43 JST (~37 phút, time-box 1,5 h).
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| `main`/HEAD | **`ca17dde`** — fix VM thread flake test-only (`eb99c19`, poll 500 ms) |
| Gate kỳ vọng | **3571/3** |
| Branch QA | `vs3/w3-registry`, merge `main` → **`ca17dde`** |
| Toolchain | `~/.cargo/bin/cargo` 1.100.0 |
| Disk | đầu **23G**; cuối **23G** (> 5G) |

## 1. 3× soak 10/10

Mỗi lần target riêng `target/qa-w3-ca-soakN`, debug=0/incremental=0, log mới; xoá target sau
từng lần.

| Lần | Tổng | Gate ×3 | Host | Kết quả | Log |
|---|---|---|---|---|---|
| 1 | **676 s** | 3571/3 | 54/54 | 10/10 OK | `/tmp/opencode/qa-w3-ca-soak1-soak.log` |
| 2 | **704 s** | 3571/3 | 54/54 | 10/10 OK | `.../qa-w3-ca-soak2-soak.log` |
| 3 | **721 s** | 3571/3 | 54/54 | 10/10 OK | `.../qa-w3-ca-soak3-soak.log` |
| **Tổng** | **2101 s (~35 phút)** | — | — | **30/30 vòng OK** | — |

- Mỗi lần: gate ×3, diff ×3 (seed cố định `20261115`), host ×2, cache ×2.
- **VM thread test**: `vm_khong_tao_them_thread_sau_khi_join` xuất hiện **9/9 lần** (3 gate × 3 soak)
  đều **PASS**, thời gian 0,051–0,128 s; không còn `2 -> 4/5`.
- Không T-INFRA; target riêng.
- Wrapper: `/tmp/opencode/qa-w3-ca-soakN.log`.

## 2. VM test 20 lần dưới tải

`cargo test -q -p vis-cli --test vs3_worker_eval_vm` lặp **20 lần** (target riêng
`target/qa-w3-ca-vm`, debug=0/incremental=0; load đầu 4,77/7,08/8,26):

| Chỉ số | Giá trị |
|---|---|
| Số lần | **20** |
| Pass | **20** |
| Fail | **0** |
| Thời gian | ~64 s (05:41:29–05:42:53) |
| Log | `/tmp/opencode/qa-w3-ca-vm20.log` |

- Không lần nào FAILED, không `rò thread`; fix poll 500 ms có hiệu lực cho VM.
- Mỗi lần 10 test VM (tổng 200 test-case chạy).

## 3. Đánh giá

- **3× soak 10/10 PASS**, gate **3571/3**, host 54/54, VM thread test xanh **9/9**.
- **VM 20/20 PASS** dưới tải; xác nhận fix test-only `eb99c19`.
- Không cần tag mới; đây là hardening sau v0.3.3.

## 4. Blocker / còn lại

1. Không có blocker local; flake VM đã đóng ở mức test (9 gate + 20 lần).
2. 2 ô `EXPECTED_RED` msvc/wasm do #156 C toolchain — không đổi nhãn PASS.
3. Không GitHub Actions; Windows `ca17dde` chưa đo trong phiên này.
4. Tàn dư đã biết: evaluator P3 mới, async-h2 default-off, `select` VS3-2c — release notes.

## 5. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0

for n in 1 2 3; do
  export CARGO_TARGET_DIR=$PWD/target/qa-w3-ca-soak$n
  ./bin/vis-soak --log /tmp/opencode/qa-w3-ca-soak$n-soak.log
  rm -rf "$CARGO_TARGET_DIR"
done

export CARGO_TARGET_DIR=$PWD/target/qa-w3-ca-vm
for n in $(seq 1 20); do
  cargo test -q -p vis-cli --test vs3_worker_eval_vm
done
```

**Kết luận**: `ca17dde` **30/30 vòng soak OK** (2101 s), gate 3571/3, host 54/54; VM **20/20 lần
PASS** dưới tải; **không còn flake VM thread**. Không có blocker kỹ thuật local.
