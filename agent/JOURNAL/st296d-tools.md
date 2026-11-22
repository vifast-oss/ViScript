<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-296d-tools — Luật R7, nextest, bộ công cụ vận hành & điều tiết build

> **Người làm**: subagent VS3-W4, 2026-11-23 · **VD-282(2)(3)** · time-box 2 h.
> **Baseline**: `main` **`02af66c`** (`merge ST-296: Windows soak 2h 580 vong`); làm trên
> `plan/w4-qa-docs`. `CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0 TMPDIR=/home/ps/tmp`.
> Không tag/push/merge; không GitHub Actions.
> **Kết quả**: gate **3638/4 PASS** (fmt/clippy/R1..R7), self-test fixtures **15/15**; R7 bắt log
> >50 KB mới và miễn legacy; 4 script mới chạy `bash -n` + dry-run/self-test OK. Không blocker.

## 1. R7 — cấm log bằng chứng > 50 KB trong `agent/JOURNAL/`

- `bin/vis-dev`: thêm `rule_journal_logs` (R7) + gọi trong `run_rules`; text cổng đổi thành
  `luật R1..R7`.
  - Chỉ xét `agent/JOURNAL/*.log`, `*.log.*`, `*.txt` ở **maxdepth 1**, bỏ qua `*.license`.
  - Tệp > **50 KB** (find `-size +50k`) phải có trong `test/gate/r7-legacy-logs.txt`; nếu không ⇒ R7 đỏ.
  - Không có `agent/JOURNAL/` (cây self-test R1..R4) ⇒ bỏ qua; có thư mục nhưng thiếu danh sách ⇒ đỏ.
- `test/gate/r7-legacy-logs.txt`: **173** tệp lịch sử đã >50 KB tại thời điểm đóng băng; kèm
  `r7-legacy-logs.txt.license` (R3 sidecar). Không thêm mục mới sau hôm nay.
- Ca âm/khẳng định trong `test/fixtures/run_fixtures.sh` (nâng `1..13` → `1..15`):
  - **14**: `agent/JOURNAL/moi.log` 52 000 B + danh sách rỗng ⇒ đỏ **R7**;
  - **15**: cùng tệp nằm trong danh sách legacy ⇒ **xanh** (miễn lịch sử).
- `bash test/fixtures/run_fixtures.sh` ⇒ **PASS 15/15**.

## 2. `.config/nextest.toml`

- `[profile.default] retries = 0` — giữ luật **không che flake** (VD-282). Có SPDX.

## 3. Bộ công cụ vận hành

| Script | Việc | Đã kiểm |
|---|---|---|
| `bin/vis-gc-targets` | canh đĩa **≥10 GB**, nhắm **≥15 GB**; xoá `target/` của worktree **khác** đã merge hoặc cũ hơn `--age-days`; bỏ qua worktree đang có tiến trình; mặc định **dry-run**, cần `--apply` | `--dry-run`: free 12–13 G, **SKIP** mọi worktree đang dùng; không xoá gì |
| `bin/vis-jobs` | liệt kê job nền (cargo/rustc/nextest/vis-dev/vis-soak/vis-remote/vis-gate-windows/vis-diff); `--check` exit 1 nếu còn job chưa bảo vệ; `--protect <pid>` miễn; `--json` | liệt kê đúng job thật (không dính chuỗi trong `-c "…"`); `--check` rc 1 khi các luồng khác đang chạy (đúng) |
| `bin/vis-status` | dashboard ≤60 dòng: HEAD/describe/branch/dirty, đĩa, `target/`, job nền, 3 journal mới | chạy OK |
| `bin/vis-gate-windows --quick\|--full` | wrapper `vis-remote psvn --require-clean`; quick = cancel/select/h2/pkg, full = workspace + host + targets; `--dry-run` in lệnh | `--help`, `--quick --dry-run`, `--full --dry-run` OK (chưa chạy remote trong phiên) |

- Tất cả có SPDX `#`, `bash -n` OK; `bin/README.md` cập nhật bảng công cụ.
- `vis-gc-targets` **không chạy `--apply`** trong phiên (an toàn); chỉ dry-run + kiểm điều kiện.

## 4. Tự điều tiết build trong `bin/vis-dev`

- `CARGO_PROFILE_DEV_DEBUG=0` nếu người dùng chưa đặt (giảm `target/`, tăng tốc gate).
- `CARGO_BUILD_JOBS = max(2, nproc / active_jobs)` nếu chưa đặt; `active_jobs` = số
  cargo/rustc/nextest đang chạy tại thời điểm gọi.
- `RUSTC_WRAPPER=sccache` nếu có `sccache` trên PATH và người dùng chưa đặt.
- Không đè biến người dùng; phiên này chạy với `CARGO_BUILD_JOBS=3` như yêu cầu.

## 5. Cổng

| Bước | Kết quả |
|---|---|
| `./vis-dev rules` | **OK** (R1..R7) |
| `test/fixtures/run_fixtures.sh` | **PASS 15/15** (R7 ca 14/15) |
| `./vis-dev gate --fast` | **PASS — 3638 passed / 0 failed / 4 skipped**; fmt/clippy OK |
| Disk | local `/` còn **13 G** (>10 G) |

- Không commit log >50 KB mới: `agent/JOURNAL/*` hiện chỉ có legacy đã đóng băng; journal này
  không kèm `.log`.
- Không tag/push/merge; không GitHub Actions.

## 6. Blocker

- **Không.** Chưa chạy `vis-gate-windows` thật (remote) và `vis-gc-targets --apply` (xoá) trong
  phiên — đúng phạm vi tooling/self-test; sẽ dùng ở ST-296 chính thức.
