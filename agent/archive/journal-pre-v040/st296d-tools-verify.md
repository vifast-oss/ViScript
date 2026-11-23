<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-296d-tools — verify thật trên `a534fd7`

> **Người kiểm**: subagent VS3-W4, 2026-11-23 · time-box 1 h.
> **Baseline**: `main` **`a534fd719e4ac09678c2f5c11595458c1c91d79c`**
> (`[ST-296d] R7: them st291-gate.log vao legacy`); `plan/w4-qa-docs`, cây sạch.
> Không tag/push/merge; không GitHub Actions. Không chạy full gate/soak (chờ ST-292).
> **Kết quả**: cả 4 tool chạy đúng; `vis-gate-windows --quick` trên psvn (worktree riêng)
> **PASS — pkg 25, h2 33, cancel 8, select 25**, exit 0. Không blocker.

## 0. TL;DR

| Tool / lệnh | Kết quả |
|---|---|
| `bin/vis-jobs --json` | JSON hợp lệ, liệt kê đúng job nền thật (`vis-soak`, `vis-diff`, `cargo test`, …) |
| `bin/vis-status` | HEAD/branch/dirty/disk/target/job/journal in đúng, ≤ 60 dòng |
| `bin/vis-gc-targets --dry-run` | free **14–16 G** > ngưỡng 10 G ⇒ `OK`, **không xoá gì** |
| `bin/vis-gate-windows --quick --dry-run` | in đúng lệnh `vis-remote --require-clean` + body quick |
| `bin/vis-gate-windows --quick` (psvn) | **exit 0**: `pkg` **25/25**, `vs3_http2` **33/33**, `vs3_http_cancel` **8/8**, `vs3_select` **25/25** |

## 1. Tool cục bộ (Linux `ps`)

- `bin/vis-jobs --json`: mỗi dòng là một object `{pid,ppid,etime,protected,cmd}`; mẫu ghi nhận
  `vis-soak`, `vis-diff`, `cargo test -p vis-vm --test diff_100k`, `cargo nextest` của các luồng khác.
  Chỉ khớp tên chương trình, không dính chuỗi trong `-c "…"`.
- `bin/vis-status`: `HEAD a534fd7`, branch `plan/w4-qa-docs`, dirty 0, disk **15 G**, target **1,8 G**,
  job nền + 3 journal mới — đúng định dạng dashboard.
- `bin/vis-gc-targets --dry-run`: `repo=… free=14G (min=10G, target=15G)`, kết luận
  `OK — đĩa trên ngưỡng, không cần dọn`; không xoá.
- `bin/vis-gate-windows --quick --dry-run`: in
  `vis-remote psvn --require-clean --log … -- bash -lc '… cargo test … vs3_http_cancel vs3_select vs3_http2 pkg'`.

## 2. Chạy thật `vis-gate-windows --quick` trên psvn

- Worktree **riêng**: `/c/Users/ps/viscript-w4gate-a534fd7` @ `a534fd71`, dirty **0**.
- Gọi tool với:
  `VIS_REMOTE_PSVN_REPO=/c/Users/ps/viscript-w4gate-a534fd7`
  `VIS_GATE_WINDOWS_REPO=/c/Users/ps/viscript-w4gate-a534fd7`
  ⇒ `--require-clean` kiểm đúng worktree sạch (không dùng repo chung `/c/Users/ps/viscript-moi`).
- Kết quả (debug, `TMPDIR=/c/Users/ps/tmp`):

| Suite | Kết quả |
|---|---|
| `pkg` | **25/25** |
| `vs3_http2` | **33/33** (async-h2 default) |
| `vs3_http_cancel` | **8/8** |
| `vs3_select` | **25/25** |

- `vis-remote-exit=0`, `[exit=0]`; không fail/skip bất thường.
- Sau chạy: worktree HEAD `a534fd7`, dirty 0; `C:` còn **114 G**.
- Log local `/tmp/vis-gate-windows-quick-a534fd7.log` sha256
  `7a2b4082d3a1fad364383a184b39fce3cb2750440b746cbf1b5970c3c47c30ab`;
  bản trên máy đích sha256 `80938563e0922f765f82dcfdd15683b0f1c7eb64ca416a89aba7eec565f959a3`.

## 3. Ghi chú

- **Không** chạy full gate/soak (theo yêu cầu, đang chờ ST-292).
- `vis-gc-targets --apply` không chạy (đĩa còn 15 G > 10 G; dry-run đã đủ).
- Không commit log >50 KB: journal này chỉ `.md`; log lớn nằm ngoài repo `/tmp`.
- Không tag/push/merge; không GitHub Actions.
- Disk: local `/` **15 G** (>10 G); psvn `C:` **114 G**.

## 4. Blocker

- **Không.**
