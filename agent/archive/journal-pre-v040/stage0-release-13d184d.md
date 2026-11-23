<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Stage 0 Release QA — `13d184d` (`v0.3.6`): workspace release + targeted + 2× soak

> **Người chạy**: W3 (Gatekeeper QA), 2026-11-22 14:50–15:30 JST (~40 phút; time-box 2 h).
> **Phạm vi**: **chỉ đo**. Không sửa code/docs chung, không push/merge/tag, không GitHub Actions.
> **Nguồn**: `main` = **`13d184d`** (`merge ST-289 QA: fresh-clone 84915aa + diff 100k`;
> R32/QA docs đã merge).

## 0. Mốc đo

| Mục | Giá trị |
|---|---|
| Commit đo | **`13d184dbcdb09debfeffa389d9b0aeb51005d854`** |
| Phiên bản | **0.3.6** (`release/vis --version` → `vis 0.3.6`) |
| Nhánh QA | `plan/w3-platform` — merge `main` **fast-forward** lên `13d184d`; không sửa gì thêm |
| Toolchain | `~/.cargo/bin/cargo` **1.100.0**, `rustc 1.100.0 (0ef1bc386 2026-11-09)` |
| Máy | Linux x86_64, 12 CPU; load đầu phiên soak1 **48,91 / 25,63 / 24,10** (worktree khác build), cuối **3,91 / 9,18 / 14,44** |
| Đĩa | đầu **15 G**; sau release target **12 G**; cuối **12 G** (> 5 G) |
| Target | `target/rel-workspace` (release), `target/soak1`, `target/soak2` — **riêng**, không dùng target `main`; `CARGO_INCREMENTAL=0` |

## 1. Release workspace — **1 vòng**

```bash
export CARGO_TARGET_DIR=$PWD/target/rel-workspace CARGO_INCREMENTAL=0
cargo test --release --workspace --no-fail-fast
```

| Chỉ số | Giá trị |
|---|---|
| Kết quả | **PASS** (`rc=0`, không `FAILED`/`failures:`) |
| Binary test | **354** |
| **passed** | **3581** |
| **failed** | **0** |
| **ignored** | **3** |
| **blocked** | **0** |
| Thời gian | wall **5:49,41** (349 s), max RSS ~892 MB |
| Log | `/tmp/opencode/stage0-release-workspace.log` |

**So với debug 3576/3**: release **3581 passed / 0 failed / 3 ignored**. Chênh **+5 passed** đúng bằng
5 doc-test (`vis_interp` 2 + `vis_syntax` 1 + `vis_types` 2) — `cargo-nextest` của gate debug **không
đếm doc-test**, còn `cargo test --workspace` có. Không có test nào đổi trạng thái passed/skipped.

## 2. Release targeted

Cùng target release, `rc=0` toàn bộ:

| Suite | Lệnh (`--release -p vis-cli`) | Kết quả | Thời gian |
|---|---|---|---|
| **P2** | `--test vs3_worker_eval_parallel` | **14/14** | 0,30 s |
| **P1** | `--test vs3_worker_eval_local` | **8/8** | 0,55 s |
| **P3** | `--test vs3_worker_eval_vm` | **10/10** | 0,51 s |
| h2 mặc định | `--test vs3_http2` | **19/19** | 15,61 s |
| `pkg` | `--test pkg` | **25/25** | 0,07 s |
| phiên bản | `./target/rel-workspace/release/vis --version` | **`vis 0.3.6`** | — |

Log: `/tmp/opencode/stage0-release-targeted.log`.

## 3. Hai lần soak 10/10 trên `13d184d`

Chạy `./bin/vis-soak` (3× `gate --fast`, 3× diff 100k seed `20261115`, 2× host, 2× cache), mỗi lần
target **riêng**, `CARGO_INCREMENTAL=0`, `debug=0`:

| Lần | Target | Tổng | 10 vòng | Gate | Diff 100k | Host | Cache | Log |
|---|---|---|---|---|---|---|---|---|
| **1** | `target/soak1` | **919 s** (wall 15:19,36) | **10/10 OK** | 3× **3576/3** | 3× **lech=0** | 2× **54/54** | 2× **5/5** | `/tmp/opencode/stage0-release-13d184d-soak1.log` |
| **2** | `target/soak2` | **752 s** (wall 12:31,89) | **10/10 OK** | 3× **3576/3** | 3× **lech=0** | 2× **54/54** | 2× **5/5** | `/tmp/opencode/stage0-release-13d184d-soak2.log` |

Chi tiết theo vòng:

| Vòng | Soak 1 | Soak 2 |
|---|---|---|
| gate-1 | 250 s OK | 210 s OK |
| gate-2 | 124 s OK | 156 s OK |
| diff-1 | 93 s OK (`tong=100000 chay=93770 lech=0`) | 79 s OK (`tong=100000 chay=93770 lech=0`) |
| diff-2 | 96 s OK (lech=0) | 79 s OK (lech=0) |
| gate-3 | 213 s OK | 125 s OK |
| diff-3 | 114 s OK (lech=0) | 79 s OK (lech=0) |
| host-1 | 6 s OK (54/54 + `host_posix_real`) | 4 s OK (54/54) |
| host-2 | 1 s OK (54/54) | 1 s OK (54/54) |
| cache-1 | 11 s OK (5/5) | 10 s OK (5/5) |
| cache-2 | 11 s OK (5/5) | 9 s OK (5/5) |
| **Tổng** | **919 s — 10/10 OK** | **752 s — 10/10 OK** |

- Cả 6 lần gate trong 2 soak đều **3576 passed, 3 skipped** (soak1 gate-1 có `1 slow`, không fail).
- Cả 6 lần diff đều **`tong=100000`, `lech=0`**, seed `20261115` (soak dùng seed cố định của script).
- Không có vòng đỏ, không retry, không T-INFRA dù soak1 khởi động dưới load ~49.

## 4. Đánh giá

- Release workspace **3581 passed / 0 failed / 3 ignored** — tương đương debug **3576/3** (+5 doc-test).
- Release targeted đủ **P2 14, P1 8, P3 10, h2 19, pkg 25**; `vis --version` = **0.3.6**.
- **2 soak × 10/10 vòng OK** (919 s + 752 s = **1671 s**), mỗi soak 3 gate 3576/3, 3 diff 100k lệch 0,
  2 host 54/54, 2 cache 5/5.
- Không thấy flake/T-INFRA; không suite đỏ.

## 5. Blocker / ghi chú

1. **Không có blocker**; không có test failed/blocked.
2. Windows (`psvn`) và macOS **không đo** trong phiên này (ngoài phạm vi release QA local; macOS ngoài
   phạm vi dự án).
3. Không sửa code/docs chung; chỉ thêm journal này. Không tag/push/merge.
4. Đĩa cuối **12 G** (> 5 G); target release 1,2 G + soak 1,5 G × 2. Có thể xoá
   `target/{rel-workspace,soak1,soak2}` bằng `rm -rf target` nếu cần giải phóng thêm (log đã giữ).
5. Log bằng chứng nằm ở `/tmp/opencode/stage0-release-*.log` và `...-soak{1,2}.log`.

## 6. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_INCREMENTAL=0
cd /home/ps/.local/share/opencode/worktree/cd45e5/vs3-w3-registry

CARGO_TARGET_DIR=$PWD/target/rel-workspace cargo test --release --workspace --no-fail-fast
for t in vs3_worker_eval_parallel vs3_worker_eval_local vs3_worker_eval_vm vs3_http2 pkg; do
  CARGO_TARGET_DIR=$PWD/target/rel-workspace cargo test --release -p vis-cli --test "$t"
done
./target/rel-workspace/release/vis --version   # vis 0.3.6

for n in 1 2; do
  CARGO_TARGET_DIR=$PWD/target/soak$n CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
    ./bin/vis-soak --log /tmp/opencode/stage0-release-13d184d-soak$n.log
done
```
