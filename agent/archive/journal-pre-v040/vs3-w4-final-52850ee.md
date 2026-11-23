<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — verify cuối trước tag trên Windows (`52850ee`)

> **Người đo**: subagent VS3-W4, 2026-11-22.
> **Mốc**: `main` **`52850ee`** (fix test VM thread `eb99c19`; Linux gate 3571/3).
> `psvn` worktree **detached đúng `52850ee`**, dirty 0.
> **Kết quả**: workspace **3528 / 0 / 3**; P3 10/10 debug+release; P2 14/14; P1 8/8;
> 5/5 vòng targeted; host 54/54; targets 18/18 + 17/1; release/CLI đạt.
> Không tag/push/merge; không GitHub Actions.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| `cargo test --workspace --no-fail-fast` | **353 block; 3528 passed / 0 failed / 3 ignored** — 524 s |
| `vs3_worker_eval_vm` debug | **10/10** |
| `vs3_worker_eval_vm` release | **10/10** |
| `vs3_worker_eval_parallel` | **14/14** |
| `vs3_worker_eval_local` | **8/8** |
| `vs3_default` + `vs3_tls` + `net` ×5 | **22/22 + 22/22 + 9/9** mỗi vòng; 0 flake |
| host / targets | **54/54**; core **18/18**, sqlite **17 + 1 EXPECTED_RED** |
| Release | `vis.exe` **11 205 120 B**, sha256 `560df71e…03ba7` |
| CLI | `--version` `vis 0.1.0`; doctor 4 feature; check TAP ok; run `42`; run --version rc 2 |
| async-h2 Windows | **rc 101 — EXPECTED** (Unix-only) |
| Blocker | **Không có** |

## 1. Môi trường

- `psvn`: Windows `MINGW64_NT-10.0-26300`, 6 CPU; cargo/rustc **1.100.0** (MSVC native).
- Worktree `/c/Users/ps/viscript-w4k` detached **`52850eea`**, dirty 0.

## 2. Workspace (1 vòng)

| Lệnh | Kết quả | Thời gian |
|---|---|---:|
| `cargo test --workspace --no-fail-fast` | **353 block; 3528 passed / 0 failed / 3 ignored** | 524 s |

- Đúng kỳ vọng; không `FAILED`/`panicked`.

## 3. P3 / P2 / P1

| Suite | Kết quả |
|---|---|
| `vs3_worker_eval_vm` debug | **10/10** (2,86 s) |
| `vs3_worker_eval_vm` release | **10/10** (1,11 s; build 68 s) |
| `vs3_worker_eval_parallel` | **14/14** (1,56 s) |
| `vs3_worker_eval_local` | **8/8** (0,24 s) |

## 4. 5 vòng targeted

| Vòng | net | vs3_default | vs3_tls | Thời gian |
|---|---:|---:|---:|---:|
| R1 | 9/9 | 22/22 | 22/22 | 16 s (build) |
| R2 | 9/9 | 22/22 | 22/22 | 11 s |
| R3 | 9/9 | 22/22 | 22/22 | 12 s |
| R4 | 9/9 | 22/22 | 22/22 | 12 s |
| R5 | 9/9 | 22/22 | 22/22 | 12 s |

- **0 fail, 0 flake; `10053` không tái hiện**; fd-leak + invariant host_poll xanh mỗi vòng.

## 5. Host / targets / release / CLI

- host **54/54**; targets core **18/18**, sqlite **17 PASS + 1 EXPECTED_RED** (#156).
- `target/release/vis.exe`: **11 205 120 B**,
  sha256 `560df71ec27b9f506619e48c9cc44d2b206c0d631f4f0beac8ee1040e0d03ba7`.
- `vis --version` → `vis 0.1.0`; `doctor` → `os windows`, `arch x86_64`, `sqlite on`,
  `features sqlite,tls,pkg-sig,http2`.
- Smoke: `check` → TAP `1..1 ok 1`; `run` → `42`; `run --version` → rc 2.

## 6. `--features async-h2` trên Windows — EXPECTED RED

`cargo check -p vis-cli --features async-h2` → **rc 101** (`h2_async.rs:15` `cfg(…, unix)`).
Async-h2 vẫn **Unix-only/default-off**; không sửa, không tuyên bố hỗ trợ h2 async Windows.

## 7. Log / bằng chứng

| Log `psvn` | sha256 |
|---|---|
| `/c/Users/ps/vislogs/w4k-targeted.log` | `e74177728d4522450cd476636e836f9eebd5d9caaa2293ea5b14248a14848cce` |
| `/c/Users/ps/vislogs/w4k-workspace.log` | `486efeca91ee7447dbf5c6f19ac055e48519edadfd89928a1fd1ebd80445f37b` |
| `/c/Users/ps/vislogs/w4k-release.log` | `ab586d36de972b9e1ae8f50b475ea8d025cc725ed8786260935b90cb928cb837` |

- Worktree `psvn` đã xoá sau đo; repo chính @ `73af17bd`, dirty 0; `C:` còn **174G**.
- Local `/` còn **22G** (>5G).

## 8. Kết luận

- Commit **`52850ee`** sẵn sàng tag trên Windows: workspace 3528/0/3, P3/P2/P1 xanh,
  5/5 vòng targeted, host/targets xanh, release/CLI đạt; 0 flake.
- Windows async net/TLS/HTTP-1.1 + evaluator P1/P2/P3 đạt phạm vi đo; async-h2 vẫn **Unix-only**.
- **Không blocker.**
