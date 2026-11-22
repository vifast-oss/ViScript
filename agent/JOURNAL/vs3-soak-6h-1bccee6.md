<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 soak dài 6 h — `1bccee6` (DỪNG ở run 2 do flake mới)

> **Người chạy**: W3 (QA/hardening), 2026-11-22 10:28–10:50 JST.
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**
> **Quy tắc**: dừng ngay khi fail. Đã dừng ở run 2; **không đủ** 4 h/20 lần.

## 0. Mốc

| Mục | Giá trị |
|---|---|
| `main`/HEAD | **`1bccee6`** (20× soak đã merge; code = tag v0.3.4; gate 3571/3) |
| Branch QA | `vs3/w3-registry`, merge `main` → **`1bccee6`** |
| Target | mỗi lần riêng `target/qa-w3-6h-N`; xoá sau mỗi lần |
| Env | `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0` |
| Disk | **22G** ổn định (> 5G) |

## 1. Kết quả

| Run | Tổng | Gate ×3 | Host | Kết quả | Log |
|---|---|---|---|---|---|
| 1 | **765 s** | 3571/3 | 54/54 | **10/10 OK** | `/tmp/opencode/qa-w3-6h-soak1.log` |
| 2 | 572 s (dừng) | gate-3 FAIL | — | **FAIL** | `/tmp/opencode/qa-w3-6h-soak2.log` |

- Run 1: đủ 10 vòng; gate **3571/3** ×3; host 54/54 + POSIX.
- Run 2: gate-1/2 OK; **gate-3 FAIL** sau 109 s:
  `vs3_worker_eval_parallel::cancel_pre_run_tra_loi_ro` — `out.exit = 0`, kỳ vọng **1**
  (`task bị huỷ phải lỗi`).
- Dừng ngay theo quy tắc; không chạy tiếp 18 lần còn lại.

## 2. Tổng kết

| Chỉ số | Giá trị |
|---|---|
| Số run hoàn tất | **1** |
| Số vòng hoàn tất | **10/200** |
| Gate pass | 5/6 gate (run1 ×3, run2 ×2) |
| Flake | **1** — `cancel_pre_run_tra_loi_ro` (cancel-before-run trả 0 thay vì 1) |
| T-INFRA | 0 |
| Tổng thời gian | ~22 phút (10:28–10:50), không đạt 4 h/20 lần |
| Disk | 22G → 22G, không <5G |

## 3. Đánh giá

- Long-soak **không đạt mục tiêu** vì flake mới ở evaluator P3: `cancel_pre_run_tra_loi_ro`.
- Run 1 sạch; run 2 hỏng ở gate-3 dưới tải (load ~9,9), test khác với VM flake đã đóng.
- Không kết luận ổn định 4 h/20 lần; cần fix flake cancellation trước khi chạy lại.

## 4. Blocker / còn lại

1. **Blocker**: `vs3_worker_eval_parallel::cancel_pre_run_tra_loi_ro` flake (exit 0 thay vì 1) —
   cần W5/W1 kiểm tra race cancel-before-run; **chưa sửa code** trong QA.
2. Chưa đủ **4 h/20 lần** (mới 1 run + 0,5 run); 6 h/30 lần càng chưa.
3. 2 ô `EXPECTED_RED` msvc/wasm do #156.
4. Không GitHub Actions; Windows `1bccee6` chưa đo.
5. Tàn dư đã biết: evaluator P3 mới, async-h2 default-off, `select` VS3-2c — release notes.

## 5. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
for n in $(seq 1 20); do
  export CARGO_TARGET_DIR=$PWD/target/qa-w3-6h-$n
  ./bin/vis-soak --log /tmp/opencode/qa-w3-6h-soak$n.log
  rc=$?
  rm -rf "$CARGO_TARGET_DIR"
  [ $rc -eq 0 ] || { echo "STOP at $n"; break; }
done
```

**Kết luận**: `1bccee6` **run 1 10/10 PASS** (765 s), nhưng **run 2 FAIL** ở
`cancel_pre_run_tra_loi_ro` ⇒ dừng theo quy tắc. Chưa đạt 4 h/20 lần; cần fix flake cancellation
trước khi chạy lại.
