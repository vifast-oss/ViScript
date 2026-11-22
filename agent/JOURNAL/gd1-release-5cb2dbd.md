<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Release QA GĐ1 `v0.4.0` — `5cb2dbd`

> **Người chạy**: W3 (Gatekeeper QA), 2026-11-22 22:22–23:05 JST (~43 phút; time-box 2 h).
> **Phạm vi**: **chỉ đo**. Không sửa code, không push/merge/tag, không GitHub Actions.
> **Mốc**: `plan/w3-platform` merge `main` **`5cb2dbd`** (ST-290/293/294/295 + h2 cache fix +
> release harness); gate kỳ vọng **3638/4**.

## 0. Mốc đo

| Mục | Giá trị |
|---|---|
| Commit đo | **`5cb2dbd5c403fac03a11cef863b326ec620d9b9c`** |
| Phiên bản | **0.3.6** (`release/vis --version`, `soak2/debug/vis --version`) |
| Target | release: `target/release-gd1`; soak: `target/soak1`, `target/soak2` — riêng |
| TMPDIR | **`/home/ps/tmp`** |
| Toolchain | cargo/rustc **1.100.0**; Linux x86_64 12 CPU |
| Đĩa | cuối **33 G** (> 10 G) |

## 1. Release workspace — **3643 / 0 / 4**

```bash
cargo test --release --workspace --no-fail-fast
```

| Chỉ số | Giá trị |
|---|---|
| Kết quả | **PASS** (`rc=0`) |
| Binary test | **358** |
| **passed** | **3643** |
| **failed** | **0** |
| **ignored** | **4** |
| Thời gian | wall **5:32,65** |
| Log | `/tmp/opencode/gd1-release-5cb2dbd-workspace.log` |

8 ca `h2_async_spike` debug-only **bỏ qua có lý do** ở release (test-harness `3896bc6`); release
không còn đỏ. So với gate **3638/4**: +5 doc-test (nextest không đếm).

## 2. Release targeted

Cùng target `target/release-gd1`, `rc=0` toàn bộ:

| Suite | Kết quả |
|---|---|
| `vs3_select` | **25/25** |
| P2 `vs3_worker_eval_parallel` | **14/14** |
| P1 `vs3_worker_eval_local` | **8/8** |
| P3 `vs3_worker_eval_vm` | **10/10** |
| `vs3_http2` | **34 passed / 1 ignored** |
| `vs3_http_cancel` | **8/8** |
| `pkg` | **25/25** |
| `release/vis --version` | **`vis 0.3.6`** |

Log: `/tmp/opencode/gd1-release-5cb2dbd-targeted.log`.

## 3. Hai soak 10/10 trên `5cb2dbd`

Mỗi lần: 3× `gate --fast`, 3× diff 100k seed `20261115`, 2× host, 2× cache; target riêng,
`TMPDIR=/home/ps/tmp`, `debug=0/incremental=0`:

| Lần | Tổng | Kết quả | Gate | Diff 100k | Host | Cache | Log |
|---|---:|---|---|---|---|---|---|
| 1 | **830 s** (13:50) | **10/10 OK** | 3× **3638/4** | 3× **lech=0** | 2× **54/54** | 2× **5/5** | `/tmp/opencode/gd1-release-5cb2dbd-soak1.log` |
| 2 | **816 s** (13:36) | **10/10 OK** | 3× **3638/4** | 3× **lech=0** | 2× **54/54** | 2× **5/5** | `/tmp/opencode/gd1-release-5cb2dbd-soak2.log` |

Tổng **20/20 vòng**; không vòng đỏ/retry/T-INFRA; load 4,8–12,7.

## 4. Differential 100k — seed mới

```bash
VIS_DIFF_N=100000 VIS_DIFF_SEED=202611222300 ./bin/vis-diff
```

| Chỉ số | Giá trị |
|---|---|
| Seed | **202611222300** (mới) |
| Tổng | **100.000** |
| Lỗi tĩnh | 6.231 |
| Chạy | 93.769 |
| **Lệch** | **0** |
| Thời gian | 94,9 s test / **103 s wall** |
| Log | `/tmp/opencode/gd1-release-5cb2dbd-diff.log` |

## 5. Đánh giá / blocker

- Release workspace **3643/0/4** đúng kỳ vọng; release targeted đủ; 2 soak **20/20**; diff **0**.
- Không có blocker trong phạm vi release QA này. `ST-291` vẫn WIP và `ST-292` BLOCKED
  (`gioi_han_phien()`); `ST-293/295` vẫn REVIEW — đây là blocker **phạm vi GĐ1**, không phải lỗi
  release QA.
- Không sửa code; không tag/push/merge; không GitHub Actions. Đĩa cuối **33 G** (> 10 G).
