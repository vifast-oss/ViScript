<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Windows post-merge — `v0.4.0` + ST-296a/ST-296b @ `1b27cb8`

> **Người đo**: subagent VS3-W4, 2026-11-23 · time-box 1,5 h.
> **Mốc**: `main` **`1b27cb8029361e0f1e1c30eaf4025ed9de5cb0cb`**
> (`[ST-296b] R7: dua log lon ra /home/ps/vislogs`; đã gồm `v0.4.0`, ST-296a gom test binary,
> ST-296b tách `vis-loader`/`vis-lsp`; Linux gate 3656/4).
> `psvn` worktree **riêng** `/c/Users/ps/viscript-postmerge-1b27cb8` @ `1b27cb8`, dirty **0**.
> `TMPDIR=/c/Users/ps/tmp`. Không tag/push/merge; không GitHub Actions.
> **Kết quả: PASS** — full gate **3607/0/3**, host 54/54, targets đạt; release targeted xanh;
> `vis-lsp` **27/27**, `vis-loader` **29/29**. Không blocker.

## 0. TL;DR

| Hạng mục | Kết quả | Thời gian |
|---|---|---:|
| `bin/vis-gate-windows --full` | **exit 0** — **105 block; 3607 passed / 0 failed / 3 ignored** | **478 s** |
| host (trong full) | **54/54** (1..54, 120 lần chạy, VM 54/54) | 3,8 s |
| targets | core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED** | — |
| Release targeted | wf **12/12**, wf_vm **6/6**, P2 **14/14**, P1 **8/8**, P3 **10/10**, select **25/25**, h2 **33/33**, cancel **8/8** | ~80 s |
| `cargo test -p vis-lsp` | **27/27** | 0,06 s |
| `cargo test -p vis-loader` | **29/29** | 3,87 s |
| Release build/CLI | `vis.exe` **11 719 168 B**, sha256 `d19ff42a…59aa43`; `vis 0.4.0`; check/run OK | 0 s |

## 1. Full gate (debug + host + targets)

Lệnh (env trỏ worktree riêng):

```bash
VIS_REMOTE_PSVN_REPO=/c/Users/ps/viscript-postmerge-1b27cb8 \
VIS_GATE_WINDOWS_REPO=/c/Users/ps/viscript-postmerge-1b27cb8 \
VIS_GATE_WINDOWS_TMPDIR=/c/Users/ps/tmp \
bin/vis-gate-windows --full
```

- **exit 0**, **478 s** (nhanh hơn nhờ ST-296a gom binary: **105 block** thay vì 361).
- Workspace: **3607 passed / 0 failed / 3 ignored**; không dòng `FAILED`.
- host: **54/54**, 120 lần chạy, 0 lệch, VM 54/54.
- targets: core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED**, 0 FAIL.
- Log local `/tmp/vis-gate-windows-full-1b27cb8.log` sha256
  `074004ba406523b882c6781eee4c8aac41122d789142b73cc4da37e213e9cc96`.

## 2. Release targeted — tên binary đã đổi do ST-296a

ST-296a gom test binary, nên các tên cũ `--test vs3_select`, `--test vs3_http2`,
`--test vs3_http_cancel` **không còn**; dùng binary nhóm:

| Mục tiêu | Lệnh | Kết quả |
|---|---|---|
| Worker frame | `--test vs3_worker_frame` | **12/12** |
| Worker frame VM | `--test vs3_worker_frame_vm` | **6/6** |
| P2 / P1 / P3 | `--test vs3_worker_eval_parallel/local/vm` | **14/14**, **8/8**, **10/10** |
| `select` | `--test async_it vs3_select::` | **25/25** |
| h2 | `--test http2_it` | **33/33** |
| cancel | `--test net_pkg_it cancel_http_` | **8/8** |

- Tất cả release, rc 0, 0 failed.

## 3. `vis-lsp` / `vis-loader` trên Windows

- `cargo test -p vis-lsp` → **27 passed / 0 failed** (0,06 s; doc-test 0).
- `cargo test -p vis-loader` → **29 passed / 0 failed** (3,87 s; doc-test 0).
- Hai crate tách mới (ST-296b) biên dịch/chạy test trên Windows bình thường, 0 lỗi.

## 4. Release build / CLI

- `cargo build --release -p vis-cli --bin vis` rc 0 (artifact sẵn);
  `target/release/vis.exe` **11 719 168 B**,
  sha256 **`d19ff42a80c1b0ddbb37c6448844a3b87430c27cafe037fcbc2178914359aa43`**.
- `vis --version` → **`vis 0.4.0`**; `version --json` đúng.
- Smoke: `check` TAP `1..1 ok 1`, rc 0; `run` → `42`.

## 5. Bằng chứng

| Log `psvn` (`/c/Users/ps/vislogs/`) | sha256 |
|---|---|
| `pm-wf.log` / `pm-wfvm.log` | `81db18e5…0bbcb51` / `2e29a8ae…a1d7b72` |
| `pm-p2.log` / `pm-p1.log` / `pm-p3.log` | `620ffdfc…7867cd4` / `93374e82…7e3576` / `784ab432…ed5b35` |
| `pm-select25.log` | `ae0d8c8e…e4ba498` |
| `pm-cancel_group.log` | `746e6673…58a424f` |
| `pm-h2_group.log` | `d9edf1b3…96b4014` |
| `pm-lsp.log` / `pm-loader.log` | `a0f3650a…7b62e2d` / `715dd9bd…a2a390` |
| `pm-build.log` | `92e06e48…f551bc9ec1` |
| full log local | `074004ba…13e9cc96` |

- Worktree HEAD `1b27cb8`, dirty **0**; `psvn` `C:` còn **87 G**; local `/` còn **18 G** (>10 G).

## 6. Blocker

- **Không.** Hậu merge `v0.4.0` + ST-296a/296b ổn định trên Windows.
