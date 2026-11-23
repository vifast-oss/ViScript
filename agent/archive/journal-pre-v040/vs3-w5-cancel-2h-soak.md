<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — soak cancel-before 2 giờ (test-only)

**Trạng thái**: **SOAK TEST-ONLY** — không sửa production/default/API/ABI.
**Nhánh**: `vs3/w5-spike`, đã `git merge main` → **`1edec5d`** (fix cancel-before đã merge).
**Harness**: `/tmp/opencode/cancel_soak.sh`; log `agent/JOURNAL/vs3-w5-cancel-2h-soak.log`.
**Thời lượng**: `SOAK_SECS=7200` (2 giờ) hoặc tối thiểu `EXACT_N=1000` cancel-before.
**Tải**: 4 tiến trình `yes` chạy nền; test binary debug; full P2 chạy default test-threads.

---

## 1. Cách chạy

1. Build test binaries một lần (`cargo test --no-run`).
2. **1000 lần** `vs3_worker_eval_parallel` filter `cancel_pre_run_tra_loi_ro --exact`
   (chạy trực tiếp binary, không qua cargo/lock).
3. Lặp đến 7200 s:
   - **full P2 14/14** (default threads);
   - mỗi 10 vòng: P1 8/8 + P3 10/10;
   - log mỗi 20 vòng: thời gian, RSS max (`/usr/bin/time -v`), load, free, fd shell.
4. Dừng ngay nếu bất kỳ test exit != 0.

---

## 2. Kết quả

**PASS — 0 fail / 0 flake.**

| Mục | Số đo |
|---|---:|
| Thời gian | **7.201 s** (2 giờ) |
| Cancel-before exact | **1.000/1.000** (test `cancel_pre_run_tra_loi_ro --exact`) |
| Full P2 14/14 | **2.050/2.050** vòng |
| P1 8/8 | **205/205** vòng |
| P3 10/10 | **205/205** vòng |
| Tổng lượt cancel-before | **3.050** test invocation (1.000 exact + 2.050 full) |
| Max RSS | **38.868 kB** (~38 MB; `/usr/bin/time -v`, full P2) |
| Load | **12,8–21,8** (4 CPU load nền) |
| Free RAM | 788–1.990 MB trong soak |
| fd shell | **5** ổn định |
| Threads/fd test | `muoi_vong_threads_fd_rss` xanh trong **mọi** full P2 |
| Lỗi | **0**; harness dừng ngay nếu exit != 0 |

- Log chi tiết: `agent/JOURNAL/vs3-w5-cancel-2h-soak.log` (102 dòng FULL + START/DONE).
- 1.000 exact chạy trong ~11 s đầu; sau đó 2.050 vòng full P2 dưới tải.
- Không thấy flake cancel-before sau fix `dcd66e2`; test cũ `cancel_pre_run_tra_loi_ro` xanh
  liên tục.

---

## 3. Test / gate cuối soak

| Lệnh | Kết quả |
|---|---|
| P2 `vs3_worker_eval_parallel` **×10** | **140/140 PASS** (1,56–1,92 s/vòng) |
| P1 `vs3_worker_eval_local` | **8/8 PASS** (0,39 s) |
| P3 `vs3_worker_eval_vm` | **10/10 PASS** (5,69 s) |
| `./vis-dev gate --fast` | **PASS**; nextest **3571 passed / 3 skipped** (109,783 s) |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy -p vis-cli --all-targets -- -D warnings` | PASS |

- R1 mọi tệp ≤1.500; không sửa production trong phiên soak.
- `df /home`: 19,45G cuối soak; sau xoá target riêng của worktree còn **20G**.
- Không GitHub Actions, không push/merge/tag.

---

## 4. Blocker / ghi chú

- Flake gốc `cancel_pre_run_tra_loi_ro` đã fix ở `dcd66e2` (set cờ huỷ trước enqueue);
  soak này kiểm chứng dưới tải.
- Không đổi tag `v0.3.4`; nếu cần phát hành patch, `v0.3.5`.
- Không GitHub Actions; không push/merge/tag.
