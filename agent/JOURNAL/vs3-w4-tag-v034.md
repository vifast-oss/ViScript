<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — verify tag `v0.3.4` (`8fa1f96`) trên Windows

> **Người đo**: subagent VS3-W4, 2026-11-22.
> **Tag**: **`v0.3.4`** → annotated `dec5192…`, commit **`8fa1f96720b5cb37613226e7713beb50527b3f71`**
> (`merge VS3 W5ae: evaluator 2h soak 168M task`; code = `52850ee` + docs).
> `psvn` worktree **detached đúng `v0.3.4`**, `git describe --tags --exact-match` = `v0.3.4`,
> dirty 0. Không tag/push/merge; không GitHub Actions.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| HEAD verify | **`8fa1f96720b5cb37613226e7713beb50527b3f71`** = tag `v0.3.4` |
| `cargo test --workspace --no-fail-fast` | **353 block; 3528 passed / 0 failed / 3 ignored** — 526 s |
| `vs3_default` + `vs3_tls` + `net` ×5 | **22/22 + 22/22 + 9/9** mỗi vòng; 0 flake |
| P3 `vs3_worker_eval_vm` debug / release | **10/10** / **10/10** |
| P2 `vs3_worker_eval_parallel` | **14/14** |
| P1 `vs3_worker_eval_local` | **8/8** |
| sync h2 `vs3_http2` | **19/19** |
| `vs3_worker` | **14/14** |
| host / targets | **54/54**; core **18/18**, sqlite **17 + 1 EXPECTED_RED** |
| Release | `vis.exe` **11 205 120 B**, sha256 `a9368472…1717e7` |
| CLI | `--version` `vis 0.1.0`; doctor 4 feature; check TAP ok; run `42`; run --version rc 2 |
| async-h2 Windows | **rc 101 — EXPECTED** (Unix-only) |
| Blocker | **Không có** |

## 1. Môi trường / tag

- `psvn`: Windows `MINGW64_NT-10.0-26300`, 6 CPU; cargo/rustc **1.100.0** (MSVC native).
- `git fetch origin --tags`: nhận `v0.3.4`; `git rev-parse v0.3.4` = `dec51929…` (annotated),
  `v0.3.4^{commit}` = **`8fa1f967…`**; worktree `/c/Users/ps/viscript-w4m` detached đúng tag,
  dirty 0.
- `git describe --tags --exact-match` trong worktree = **`v0.3.4`**.

## 2. Workspace (1 vòng)

| Lệnh | Kết quả | Thời gian |
|---|---|---:|
| `cargo test --workspace --no-fail-fast` | **353 block; 3528 passed / 0 failed / 3 ignored** | 526 s |

- Đúng kỳ vọng; không `FAILED`/`panicked`.

## 3. 5 vòng targeted

| Vòng | net | vs3_default | vs3_tls | Thời gian |
|---|---:|---:|---:|---:|
| R1 | 9/9 | 22/22 | 22/22 | 15 s (build) |
| R2 | 9/9 | 22/22 | 22/22 | 12 s |
| R3 | 9/9 | 22/22 | 22/22 | 12 s |
| R4 | 9/9 | 22/22 | 22/22 | 12 s |
| R5 | 9/9 | 22/22 | 22/22 | 12 s |

- **0 fail, 0 flake; `10053` không tái hiện.**

## 4. P3 / P2 / P1 / h2 / worker

| Suite | Kết quả |
|---|---|
| `vs3_worker_eval_vm` debug | **10/10** (2,72 s) |
| `vs3_worker_eval_vm` release | **10/10** (1,12 s; build 68 s) |
| `vs3_worker_eval_parallel` | **14/14** (1,58 s) |
| `vs3_worker_eval_local` | **8/8** (0,22 s) |
| `vs3_http2` sync | **19/19** (15,35 s) |
| `vs3_worker` | **14/14** (24,74 s) |

## 5. Host / targets / release / CLI

- host **54/54**; targets core **18/18**, sqlite **17 PASS + 1 EXPECTED_RED** (#156).
- `target/release/vis.exe`: **11 205 120 B**,
  sha256 `a9368472708ce40688c8d8d43b92fc90e17a1ca97700b44ac707dd85717717e7`.
- `vis --version` → `vis 0.1.0`; `doctor` → `os windows`, `arch x86_64`, `sqlite on`,
  `features sqlite,tls,pkg-sig,http2`.
- Smoke: `check` → TAP `1..1 ok 1`; `run` → `42`; `run --version` → rc 2.

## 6. `--features async-h2` trên Windows — EXPECTED RED

`cargo check -p vis-cli --features async-h2` → **rc 101** (`h2_async.rs:15` `cfg(…, unix)`).
Async-h2 vẫn **Unix-only/default-off**; không sửa, không tuyên bố hỗ trợ h2 async Windows.

## 7. Log / bằng chứng

| Log `psvn` | sha256 |
|---|---|
| `/c/Users/ps/vislogs/w4m-targeted.log` | `f1ea88753754c7804604e628b8d38a687afcd8776a503bf8814b108792a9f89f` |
| `/c/Users/ps/vislogs/w4m-workspace.log` | `b4b82e8b58a3ff90ea7a8f802f88896dc81fac7379e1cb7a222c6dd126f740e4` |
| `/c/Users/ps/vislogs/w4m-release.log` | `546be60a54cde5cba640263aa343e33ad2c70304ee585cebba44a3cfe477d9b7` |

- Worktree `psvn` đã xoá sau đo; repo chính @ `73af17bd`, dirty 0; `C:` còn **174G**.
- Local `/` còn **21G** (>5G).

## 8. Kết luận

- **Tag `v0.3.4` (`8fa1f96`)** đạt trên Windows: workspace 3528/0/3, 5/5 vòng targeted,
  P3/P2/P1 + h2 sync + worker xanh, host/targets xanh, release/CLI đạt; 0 flake.
- Windows async net/TLS/HTTP-1.1 + evaluator P1/P2/P3 đạt phạm vi đo; async-h2 vẫn **Unix-only**.
- **Không blocker.**
