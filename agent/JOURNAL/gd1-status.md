<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# GĐ1 status @ `3fbf31a` — ACTIVE/VS3-TIEN-DO + 1 soak

> **Người chạy**: W3 (Gatekeeper QA), 2026-11-22 22:02–22:22 JST (~20 phút; time-box 1 h).
> **Phạm vi**: docs-only + 1 soak. Không sửa code, **không sửa `agent/QUEUE.md`** (A0 owns),
> không push/merge/tag, không GitHub Actions.
> **Baseline**: `plan/w3-platform` merge `main` **`3fbf31a`**.

## 1. Cập nhật tài liệu

- `agent/ACTIVE.md` (**31 dòng**, ≤100):
  - `ST-290` **DONE** `a48d5b87`; `ST-294` **DONE** `cce2d0fb`.
  - `ST-293` **REVIEW** `6d6bfe9` (code `82064e9`) — 25 ca, gate **3632/4**, diff 100k seed
    `202611222100` **0 lệch**.
  - `ST-295` **REVIEW** `cd76da5` (code `ba4023f`) — race ghi **100/100 → 0/100**, Landlock
    **18/18**, gate **3613/4**, diff 0, license `st295-license.log`.
  - `ST-291` **WIP**; `ST-292` **BLOCKED**, nhắc **bắt buộc** nối `gioi_han_phien()` vào
    `SharedBudget` khi `--workers > 1` (`VD-281`) + test `--workers 4 --max-budget 5000` trap
    `VS0510`.
  - `ST-296` **QA/WIP**: release h2 harness fixed `3896bc6` (release **3643/0/4**, gate
    **3638/4**); 2 soak **20/20** @ `6d6bfe9`; chờ `ST-291/292/293/295`.
- `agent/VS3-TIEN-DO.md`: cập nhật đoạn trạng thái GĐ1 trong §2c + thêm **§2d** bảng GĐ1 với
  hash/bằng chứng từng task và ghi chú `ST-292` về `gioi_han_phien()`.
- **Không** đụng `agent/QUEUE.md`, không sửa code.

## 2. Soak trên `3fbf31a`

1× `./bin/vis-soak`, target riêng `target/soak-gd1`, `TMPDIR=/home/ps/tmp`,
`CARGO_INCREMENTAL=0`, `debug=0`:

| Vòng | Thời gian | Kết quả |
|---|---:|---|
| gate-1 | 163 s | OK — **3638/4** |
| gate-2 | 122 s | OK — **3638/4** |
| diff-1 | 152 s | OK — `tong=100000 lech=0` |
| diff-2 | 99 s | OK — `lech=0` |
| gate-3 | 176 s | OK — **3638/4** |
| diff-3 | 152 s | OK — `lech=0` |
| host-1 | 19 s | OK — **54/54** |
| host-2 | 2 s | OK — **54/54** |
| cache-1 | 38 s | OK — **5/5** |
| cache-2 | 16 s | OK — **5/5** |
| **TỔNG** | **939 s** | **10/10 vòng OK** |

Load trước **21,97 / 27,18 / 19,78** (worktree khác build song song), sau **23,74 / 22,95 / 19,76**;
không fail, không retry/T-INFRA. Log `/tmp/opencode/gd1-status-soak.log`.

## 3. Cổng kiểm

`./vis-dev check` → **PASS** (fmt, clippy, R1..R6, self-test, doc-examples 24 khối,
readme-examples 20/20, report-check **32 báo cáo / 327 hàng QUEUE**). Log
`/tmp/opencode/gd1-status-check.log`.

## 4. Blocker / ghi chú

- Không có blocker mới từ soak/check. Trạng thái GĐ1 đúng như tài liệu: **ST-291 WIP**, **ST-292
  BLOCKED** (chờ ST-291 + `gioi_han_phien()`), **ST-293/295 REVIEW**, **ST-296 QA/WIP**.
- Release h2 harness **đã fix** (`3896bc6`) — release **3643/0/4**; không còn blocker release.
- Đĩa cuối **14 G** (> 10 G); log soak check giữ ngoài repo. Không tag/push/merge.
