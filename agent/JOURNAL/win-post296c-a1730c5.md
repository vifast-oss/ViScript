<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Windows post-merge ST-296c — `a1730c5`

> **Người đo**: subagent VS3-W4, 2026-11-23 · time-box 1 h.
> **Mốc**: `main` **`a1730c53b953e5bd40969db5982d3121989d4643`**
> (`merge ST-296d: archive + DECISIONS-INDEX`; đã gồm v0.4.0 + ST-296a/b/c/d).
> `psvn` worktree **riêng** `/c/Users/ps/viscript-post296c-a1730c5` @ `a1730c5`, dirty **0**;
> không chạy song song job psvn khác. `TMPDIR=/c/Users/ps/tmp`.
> Không tag/push/merge; không GitHub Actions.
> **Kết quả: PASS** — full gate **3607/0/3**, host 54/54, targets đạt; release targeted xanh;
> `vis-lsp` **27/27**, `vis-loader` **29/29**. Không blocker.

## 0. TL;DR

| Hạng mục | Kết quả | Thời gian |
|---|---|---:|
| `bin/vis-gate-windows --full` | **exit 0** — **105 block; 3607 passed / 0 failed / 3 ignored** | **480 s** |
| host (trong full) | **54/54** (1..54, 120 lần chạy, VM 54/54) | 3,8 s |
| targets | core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED** | — |
| Release targeted | wf **12/12**, wf_vm **6/6**, P2 **14/14**, P1 **8/8**, P3 **10/10**, select **25/25**, h2 **33/33**, cancel **8/8** | ~115 s |
| `cargo test -p vis-lsp` | **27/27** | 0,06 s |
| `cargo test -p vis-loader` | **29/29** | 4,22 s |
| Release build/CLI | `vis.exe` **11 718 144 B**, sha256 `afe89928…c331835`; `vis 0.4.0` | 0 s |

## 1. Full gate

Lệnh (env trỏ worktree riêng):

```bash
VIS_REMOTE_PSVN_REPO=/c/Users/ps/viscript-post296c-a1730c5 \
VIS_GATE_WINDOWS_REPO=/c/Users/ps/viscript-post296c-a1730c5 \
VIS_GATE_WINDOWS_TMPDIR=/c/Users/ps/tmp \
bin/vis-gate-windows --full
```

- **exit 0**, **480 s**; **105 test binary** (ST-296a) — **3607/0/3**, không dòng `FAILED`.
- host: **54/54**, 120 lần chạy, 0 lệch, VM 54/54.
- targets: core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED**, 0 FAIL.
- Log local `/tmp/vis-gate-windows-full-a1730c5.log` sha256
  `b91a758240ec71a23a74f76b003cb92024d9af076b045c81ae525c7a04ac0856`.

## 2. Release targeted (binary nhóm sau ST-296a)

| Mục tiêu | Lệnh | Kết quả |
|---|---|---|
| Worker frame | `--test vs3_worker_frame` | **12/12** |
| Worker frame VM | `--test vs3_worker_frame_vm` | **6/6** |
| P2 / P1 / P3 | `--test vs3_worker_eval_parallel/local/vm` | **14/14**, **8/8**, **10/10** |
| `select` | `--test async_it vs3_select::` | **25/25** |
| h2 | `--test http2_it` | **33/33** |
| cancel | `--test net_pkg_it cancel_http_` | **8/8** |

- Tất cả release, rc 0, 0 failed.

## 3. `vis-lsp` / `vis-loader`

- `cargo test -p vis-lsp` → **27 passed / 0 failed** (0,06 s; doc-test 0).
- `cargo test -p vis-loader` → **29 passed / 0 failed** (4,22 s; doc-test 0).
- Cả hai crate tách (ST-296b) vẫn chạy test bình thường trên Windows sau ST-296c.

## 4. Release build / CLI

- `cargo build --release -p vis-cli --bin vis` rc 0 (artifact sẵn);
  `target/release/vis.exe` **11 718 144 B**,
  sha256 **`afe89928dac533131594d3b6fcce284cf0a2cf4c13342f214e6015195c331835`**.
- `vis --version` → **`vis 0.4.0`**; `version --json` đúng.
- `check` TAP `1..1 ok 1`; `run` → `42`.

## 5. Bằng chứng

| Log `psvn` (`/c/Users/ps/vislogs/`) | sha256 |
|---|---|
| `post296c-wf.log` / `post296c-wfvm.log` | `8450ca82…6b3c910` / `94ed3853…23d09b9` |
| `post296c-p2.log` / `post296c-p1.log` / `post296c-p3.log` | `a6acacda…8787010e` / `b66fea7b…0fd0b719` / `4fcd4dc1…4e40eb2f` |
| `post296c-select.log` | `fd37b124…dae6380` |
| `post296c-h2.log` | `f8abe600…0366e452` |
| `post296c-cancel.log` | `7adcec89…f5920251` |
| `post296c-lsp.log` / `post296c-loader.log` | `f9cbeb4c…fea83ae` / `1d2ea52c…0b78ca04` |
| `post296c-build.log` | `92e06e48…f551bc9ec1` |
| full log local | `b91a7582…7a04ac0856` |

- Worktree HEAD `a1730c5`, dirty **0**; `psvn` `C:` còn **81 G**; local `/` còn **15 G** (>10 G).

## 6. Blocker

- **Không.** ST-296c (stdlib/lower/sqlite + 68→17 module) ổn định trên Windows.
