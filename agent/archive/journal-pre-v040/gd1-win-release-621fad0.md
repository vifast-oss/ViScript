<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# GĐ1 Windows release verify — `621fad0`

> **Người đo**: subagent VS3-W4, 2026-11-22 · **release-mode**, time-box 1,5 h.
> **Mốc**: `psvn` worktree **riêng** `/c/Users/ps/viscript-w4rel-621fad0`
> @ **`621fad07fff88c9154990416d701a7b145631783`** (`merge ST-296: Windows verify f693972`);
> `git describe --tags --always` = `v0.3.6-34-g621fad07`, dirty **0**.
> `TMPDIR=/c/Users/ps/tmp` (ngoài `/tmp`). Không tag/push/merge; không GitHub Actions.
> **Kết quả: PASS toàn bộ** — release workspace **3588/0/3**, targeted đúng kỳ vọng.
> Không blocker.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| `cargo test --release --workspace --no-fail-fast` | **358 block; 3588 passed / 0 failed / 3 ignored** — **569 s** (đúng kỳ vọng ~3588/0/3) |
| `vs3_select` (release) | **25/25** |
| `vs3_http2` (release, async-h2 default) | **33/33** |
| `vs3_http_cancel` (release) | **8/8** |
| P2 / P1 / P3 (release) | **14/14**, **8/8**, **10/10** |
| `pkg` (release) | **25/25** |
| `vis --version` | **`vis 0.3.6`** (release + debug); JSON đúng |
| host (release) | **54/54** (`1..54`, 120 lần chạy, VM 54/54) |
| targets | core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED**, 0 FAIL |
| Release build | `vis.exe` **11 450 368 B**, sha256 `bfe6a5b8…42ba796d`; CLI/smoke xanh |
| Đĩa | `psvn` `C:` **126 G**; local `/` **9,6 G** (>5 G) |

## 1. Môi trường / mốc

- `psvn`: Windows `MINGW64_NT-10.0-26300`, 6 CPU; cargo/rustc **1.100.0** (MSVC).
- Worktree riêng `/c/Users/ps/viscript-w4rel-621fad0` (không dùng repo chung), dirty 0.
- `TMPDIR=TMP=TEMP=/c/Users/ps/tmp`.
- `./target/release/vis.exe --version` → `vis 0.3.6`; `version --json` →
  `{"name":"vis","version":"0.3.6"}`.

## 2. Release workspace (1 vòng)

| Lệnh | Kết quả | Thời gian |
|---|---|---:|
| `cargo test --release --workspace --no-fail-fast` | **358 block; 3588 passed / 0 failed / 3 ignored** | **569 s** |

- `rc=0`; không dòng `FAILED`; 3 ignored là ca đo lường có `#[ignore]`.
- 8 ca h2 **debug-only** không có trong release (không tính vào 3588) — đúng kỳ vọng.
- Log `rel-workspace.log`, sha256 `6d8015ed93753341117f3704655bd43661fe2349bb259d30de4980e7a96b9c17`.

## 3. Targeted (release)

| Suite | Kết quả | Thời gian |
|---|---|---:|
| `vs3_select` | **25/25** | 0,00 s |
| `vs3_http2` | **33/33** | 15,36 s |
| `vs3_http_cancel` | **8/8** | 0,34 s |
| `vs3_worker_eval_parallel` (P2) | **14/14** | 0,53 s |
| `vs3_worker_eval_local` (P1) | **8/8** | 0,03 s |
| `vs3_worker_eval_vm` (P3) | **10/10** | 1,00 s |
| `pkg` | **25/25** | 0,43 s |

- Tất cả `rc=0`, 0 failed/ignored.

## 4. Host / targets / release / CLI

- host release: **54/54** — `1..54`, **120 lần chạy** (interp+VM), 0 lệch, VM 54/54.
- targets: rc 0 — core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED, 0 FAIL**.
- Release: `cargo build --release -p vis-cli --bin vis` rc 0;
  `target/release/vis.exe` **11 450 368 B**,
  sha256 **`bfe6a5b806abb92acbc9ade883b70550affd414c30824ed13527aa0b42ba796d`**.
- CLI/smoke: `--version`/`version --json`/`doctor` đúng; `check` TAP `ok 1`, `run` → `42`,
  `run --version` rc 2; temp file dưới `TMPDIR` (`/c/Users/ps/tmp`).

## 5. Bằng chứng

| Log `psvn` (`/c/Users/ps/vislogs/`) | sha256 |
|---|---|
| `rel-workspace.log` | `6d8015ed…a96b9c17` |
| `rel-select.log` | `8326919f…0fa449fb` |
| `rel-h2.log` | `ff684de4…81761b4` |
| `rel-cancel.log` | `1003ffb0…2b6bd48` |
| `rel-p2.log` / `rel-p1.log` / `rel-p3.log` | `1051c2fa…a4f50f` / `74137a88…8170b2` / `be50ebee…81ab27d` |
| `rel-pkg.log` | `691db631…7100622` |
| `rel-host.log` | `712f0153…e50125ef` |
| `rel-targets.log` | `8f824cc9…889df3` |
| `rel-build.log` | `d1235ea4…d95e57` |

- Worktree `psvn` HEAD `621fad0`, dirty 0 sau đo; `C:` còn **126 G**.
- Local `/` còn **9,6 G** (>5 G; `/home/ps/tmp` đang dùng chung với các tác nhân khác — không dọn).

## 6. Kết luận / blocker

- **`621fad0` đạt release-mode trên Windows**: workspace **3588/0/3**, select **25**, h2 **33**,
  cancel **8**, P1/P2/P3 + pkg xanh, host **54/54**, targets **18/18 + 17/1**, release/CLI xanh.
- Version **0.3.6** nhất quán ở debug/release.
- **Không blocker.** Không tag/push/merge; không GitHub Actions.
