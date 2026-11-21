<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — release-mode Windows @ `3aec977`

> **Người đo**: subagent VS3-W4, 2026-11-22.
> **Mốc**: `main` **`3aec977`** (P3 + journals; code = `d9e33f3`; Linux gate **3571/3**).
> `psvn` worktree **detached đúng `3aec977`**, dirty 0.
> **Kết quả**: release workspace **3528 / 0 / 3** (353 block, 531 s); release targeted 3/3 vòng;
> P3 10/10, P2 14/14, P1 8/8; h2 sync release 19/19; worker release 14/14; host 54/54;
> targets 18/18 + 17/1; release/CLI OK. Không tag/push/merge; không GitHub Actions.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| `cargo test --release --workspace --no-fail-fast` | **353 block; 3528 passed / 0 failed / 3 ignored** — 531 s |
| Release targeted ×3 | **22/22 + 22/22 + 9/9** mỗi vòng (9–10 s) |
| `vs3_worker_eval_vm` release | **10/10** |
| `vs3_worker_eval_parallel` release | **14/14** |
| `vs3_worker_eval_local` release | **8/8** |
| `vs3_http2` sync release | **19/19** |
| `vs3_worker` release | **14/14** |
| host / targets | **54/54**; core **18/18**, sqlite **17 + 1 EXPECTED_RED** |
| Release artifact | `vis.exe` **11 205 120 B**, sha256 `929d081a…2bbf2` |
| `--features async-h2` | **rc 101 — EXPECTED** (Unix-only) |
| Debug-only skip | **Không có** — release workspace cùng 3528 pass / 3 ignored như debug |
| Blocker | **Không có** |

## 1. Môi trường

- `psvn`: Windows `MINGW64_NT-10.0-26300`, 6 CPU; cargo/rustc **1.100.0** (MSVC native).
- Worktree `/c/Users/ps/viscript-w4i` detached **`3aec977a`**, dirty 0.

## 2. Release workspace (1 vòng)

| Lệnh | Kết quả | Thời gian |
|---|---|---:|
| `cargo test --release --workspace --no-fail-fast` | **353 block; 3528 passed / 0 failed / 3 ignored** | 531 s |

- Số pass/ignored **giống debug** (3528/3); không phát hiện ca chỉ chạy debug bị bỏ ở release.
- Không `FAILED`/`panicked`.

## 3. Release targeted (3 vòng)

`cargo test --release -p vis-cli --test vs3_default --test vs3_tls --test net`

| Vòng | net | vs3_default | vs3_tls | Thời gian |
|---|---:|---:|---:|---:|
| R1 | 9/9 | 22/22 | 22/22 | 10 s |
| R2 | 9/9 | 22/22 | 22/22 | 9 s |
| R3 | 9/9 | 22/22 | 22/22 | 9 s |

- **0 fail, 0 flake**; `10053` không tái hiện. `vs3_tls` release nhanh hơn debug (4,3 s vs 6,4 s).

## 4. Release suite P1/P2/P3 + h2 + worker

| Suite (release) | Kết quả |
|---|---|
| `vs3_worker_eval_vm` | **10/10** (0,96 s) |
| `vs3_worker_eval_parallel` | **14/14** (0,51 s) |
| `vs3_worker_eval_local` | **8/8** (0,03 s) |
| `vs3_http2` (sync h2) | **19/19** (15,26 s) |
| `vs3_worker` | **14/14** (30,66 s) |

## 5. Host / targets / release / CLI

- host **54/54**; targets core **18/18**, sqlite **17 PASS + 1 EXPECTED_RED** (#156).
- `target/release/vis.exe`: **11 205 120 B**,
  sha256 `929d081a05d13442626b4319bf8e457a9dd04e4ac790555304fde0a32702bbf2`.
- `vis --version` → `vis 0.1.0`; `doctor` → `os windows`, `arch x86_64`, `sqlite on`,
  `features sqlite,tls,pkg-sig,http2`.
- Smoke: `check` → TAP `1..1 ok 1`; `run` → `42`; `run --version` → rc 2.

## 6. `--features async-h2` trên Windows — EXPECTED RED

`cargo check -p vis-cli --features async-h2` → **rc 101** (`h2_async.rs:15` `cfg(…, unix)`).
Async-h2 vẫn **Unix-only/default-off**; không sửa, không tuyên bố hỗ trợ h2 async Windows.

## 7. Log / bằng chứng

| Log `psvn` | sha256 |
|---|---|
| `/c/Users/ps/vislogs/w4i-release-ws.log` | `55230da880d1c143ae480ebe0a70d3678218586e214682783ae2e1a60666fbd9` |
| `/c/Users/ps/vislogs/w4i-targeted.log` | `1b3f9e2c4ced38fd285d0d693cfb6c2458ef2d5f6baea93d2434f4ae3abd850b` |
| `/c/Users/ps/vislogs/w4i-release-cli.log` | `08adb06481ef4911e9682202cb37f06db5c7b7944515ee1f61eaa03fb3c23142` |

- Worktree `psvn` đã xoá sau đo; repo chính @ `73af17bd`, dirty 0; `C:` còn **179G**.
- Local `/` còn **12G** (>5G).

## 8. Kết luận

- Commit **`3aec977`** đạt **release mode** trên Windows: workspace 3528/0/3, 3/3 targeted,
  P3/P2/P1 + h2 sync + worker xanh, host/targets xanh, release/CLI đạt.
- Không có ca debug-only bị skip ở release (cùng số pass).
- Async-h2 vẫn **Unix-only/default-off**.
- **Không blocker.**
