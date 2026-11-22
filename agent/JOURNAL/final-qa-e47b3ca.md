<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Final QA Linux — `v0.4.0` @ `e47b3ca` (ST-292 đã merge)

> **Người chạy**: W3 (Gatekeeper), 2026-11-23 · **nhánh tạm** `qa/final-e47b3ca` trong
> **fresh clone** `/home/ps/vis-qa-v040` @ **`e47b3ca90567b4ea2dc561eabc1436aa01c250e3`**.
> **Không** merge code ST-296b; **không** sửa code; **không** push/merge/tag; không GitHub Actions.
> `CARGO_BUILD_JOBS=3`, `CARGO_PROFILE_DEV_DEBUG=0`, `TMPDIR=/home/ps/tmp`, target trong clone.

## 0. Kết luận

| Hạng mục | Kết quả |
|---|---|
| `./vis-dev gate --fast` | **PASS — 3656 passed / 4 skipped** (352 binary; wall **195,84 s**) |
| `cargo test --release --workspace --no-fail-fast` | **PASS — 3661 passed / 0 failed / 4 ignored** (wall **445,18 s**) |
| Soak 10/10 lần 1 | **PASS — 10/10**, 531 s |
| Soak 10/10 lần 2 | **PASS — 10/10**, 642 s |
| Diff 100k seed mới `202611230230` | **lech=0** (100.000; 6.233 lỗi tĩnh; 93.767 chạy; 69,2 s) |
| Worker frame / budget / P1-P2-P3 | **PASS** (chi tiết §2) |
| **Đề xuất tag `v0.4.0`** | **CÓ — đủ điều kiện tag** (Linux + Windows; chi tiết §5) |

**Không blocker Linux.** Không có lỗi/flake trong gate, release, 2 soak, diff và targeted.

## 1. Gate + release

- `./vis-dev gate --fast` @ `e47b3ca`:
  `Starting 3656 tests across 352 binaries (4 tests skipped)`;
  `Summary [72.398 s] 3656 tests run: 3656 passed, 4 skipped`; wall **195,84 s**;
  fmt/clippy/R1..R7/report-check OK.
- `cargo test --release --workspace --no-fail-fast`:
  tổng **passed=3661 / failed=0 / ignored=4** (wall **445,18 s**).
  8 ca `h2_async_spike` là **debug-only** (release không chạy theo thiết kế).

## 2. Targeted

| Nhóm | Lệnh | Kết quả |
|---|---|---|
| Worker frame | `cargo test -p vis-cli --test vs3_worker_frame` | **12/12** |
| Worker frame VM | `--test vs3_worker_frame_vm` | **6/6** |
| P1 local | `--test vs3_worker_eval_local` | **8/8** |
| P2 parallel | `--test vs3_worker_eval_parallel` | **14/14** |
| P3 VM | `--test vs3_worker_eval_vm` | **10/10** |
| HTTP/2 | `--test vs3_http2` | **34 passed / 0 failed / 1 ignored** |
| HTTP cancel | `--test vs3_http_cancel` | **8/8** |
| Select | `--test vs3_select` | **25/25** |
| Pkg | `--test pkg` | **25/25** |
| Host suite | `./bin/vis-host-suite` | **54/54** |
| Version | `./target/debug/vis --version` | **`vis 0.3.6`** |

**Budget CLI** — chương trình `async fn dem(5000)`:
`vis run --workers 4 --max-budget 5000 …` ⇒ **exit 101**; stderr
`error VS0506: tác vụ 'dem' lỗi: VS0510: vượt ngân sách worker` (đủ **VS0506 + VS0510**).

## 3. Soak 2 × 10/10

Cùng `e47b3ca`, target clone, `VIS_SOAK_TMPDIR=/home/ps/tmp`:

| Lần | Wall | Gate | Diff 100k | Host | Cache | Tổng |
|---|---:|---|---|---|---|---|
| 1 (`v040-soak-1.log`) | 531 s | 3× **3656/4** | 3× `lech=0` seed `20261115` | 2× **54/54** | 2× **5/5** | **10/10** |
| 2 (`v040-soak-2.log`) | 642 s | 3× **3656/4** | 3× `lech=0` seed `20261115` | 2× **54/54** | 2× **5/5** | **10/10** |

- Log: `/home/ps/vislogs/v040-soak-1.log`, `/home/ps/vislogs/v040-soak-2.log`.
- Diff ngoài soak, seed **mới** `202611230230`:
  `DIFF tong=100000 loi_tinh=6233 chay=93767 lech=0 giay=69.2`.

## 4. Dấu vết

- Fresh clone: `/home/ps/vis-qa-v040` (branch `qa/final-e47b3ca` từ `e47b3ca`).
- Log chính: `/tmp/opencode/v040-gate.log`, `/tmp/opencode/v040-release.log`,
  `/tmp/opencode/v040-targeted.log`, `/tmp/opencode/v040-host.log`,
  `/tmp/opencode/v040-budget.err`, `/tmp/opencode/v040-diff-new.log`.
- Target clone: **3,0 G** (debug; release đã dọn sau khi đo); đĩa còn **16 G** (> 10 G).
- Không sửa mã; commit này chỉ thêm journal.

## 5. Đề xuất tag `v0.4.0`

**CÓ — đủ điều kiện tag `v0.4.0`.** Linux: gate **3656/4**, release **3661/0/4**, 2 soak
**10/10**, diff 100k mới **lech=0**, worker frame/budget/P1-P3, h2/cancel/select/pkg/host đều xanh;
version 0.3.6.

- Windows final `e47b3ca`: **3607/0/3** (full gate; nguồn `agent/JOURNAL/final-win-e47b3ca.md`).
- Windows release 0.4.0 @ `8696edc`: **version/cache/worker PASS** (nguồn
  `agent/JOURNAL/win-v040-version-8696edc.md`).

**Kết luận: đủ điều kiện tag** — không còn blocker Linux/Windows trong phạm vi QA cuối.

