<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — verify tag `v0.3.5` (`03bb7b9`) trên Windows

> **Người đo**: subagent VS3-W4, 2026-11-22.
> **Tag**: **`v0.3.5`** → commit **`03bb7b929133497aa56b5008b5d9bcbc8a704968`**
> (`merge VS3 W3ae: QA candidate v0.3.5 7e72afd`; cancel-before fix + net timeout bound).
> `psvn` worktree **detached đúng tag**, `git describe --tags --exact-match` = `v0.3.5`, dirty 0.
> Không tag/push/merge; không GitHub Actions.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| HEAD / tag | **`03bb7b9291…`** = `v0.3.5` (describe exact) |
| Workspace | **353 block; 3528 passed / 0 failed / 3 ignored** — 592 s |
| P2 ×10 | **10/10 lần 14/14**, `cancel_pre_run_tra_loi_ro` ok mỗi lần |
| P1 / P3 | **8/8**; P3 **10/10** debug + **10/10** release |
| `net` | **9/9**, gồm `tcp_het_gio_doc` (biên 3 s) |
| Targeted ×5 | **22/22 + 22/22 + 9/9** mỗi vòng |
| host / targets | **54/54**; core **18/18**, sqlite **17 + 1 EXPECTED_RED** |
| Release | `vis.exe` **11 205 120 B**, sha256 `099f1be9…8363d6` |
| CLI | `--version` `vis 0.1.0`; doctor 4 feature; check TAP ok; run `42`; run --version rc 2 |
| async-h2 Windows | **rc 101 — EXPECTED** (Unix-only) |
| Blocker | **Không có** |

## 1. Tag / môi trường

- `psvn`: Windows `MINGW64_NT-10.0-26300`, 6 CPU; cargo/rustc 1.100.0 (MSVC native).
- `git fetch origin --tags` nhận `v0.3.5`; `v0.3.5^{commit}` = `03bb7b92…`;
  worktree `/c/Users/ps/viscript-w4s` detached đúng tag, dirty 0.
- `git describe --tags --exact-match` = **`v0.3.5`**.

## 2. Workspace (1 vòng)

| Lệnh | Kết quả | Thời gian |
|---|---|---:|
| `cargo test --workspace --no-fail-fast` | **353 block; 3528 passed / 0 failed / 3 ignored** | 592 s |

- Không `FAILED`/`panicked`.

## 3. P2 10 lần (cancel-before)

| Lần | Kết quả | `cancel_pre_run_tra_loi_ro` |
|---|---|---|
| 1 | 14/14 | ok |
| 2 | 14/14 | ok |
| 3 | 14/14 | ok |
| 4 | 14/14 | ok |
| 5 | 14/14 | ok |
| 6 | 14/14 | ok |
| 7 | 14/14 | ok |
| 8 | 14/14 | ok |
| 9 | 14/14 | ok |
| 10 | 14/14 | ok |

- P1 **8/8**; P3 debug **10/10**; P3 release **10/10** (build 89 s).
- `net` **9/9**, `tcp_het_gio_doc` pass (biên 3 s từ `20b0987`).

## 4. Targeted 5 vòng

| Vòng | net | vs3_default | vs3_tls | Thời gian |
|---|---:|---:|---:|---:|
| R1 | 9/9 | 22/22 | 22/22 | 15 s (build) |
| R2 | 9/9 | 22/22 | 22/22 | 12 s |
| R3 | 9/9 | 22/22 | 22/22 | 12 s |
| R4 | 9/9 | 22/22 | 22/22 | 12 s |
| R5 | 9/9 | 22/22 | 22/22 | 12 s |

- **0 fail, 0 flake**; `10053` không tái hiện.

## 5. Bối cảnh: soak candidate 1 giờ

- Trước tag, soak **386 vòng / 3602 s** trên candidate `e672df7` (cùng cancel fix + net bound):
  **0 fail/flake**, 77 lần default/tls/net, host 54/54, targets 18/18 + 17/1.
- Free memory giảm 7,73 GB → 6,02 GB trong 1h (theo dõi; `vis_proc=0` 386/386, các ca thread/fd/RSS
  xanh) — cần quan sát ở soak 4h trên `v0.3.5`.
- Journal: `agent/JOURNAL/vs3-w4-candidate-1h.md` + `.log`.

## 6. Host / targets / release / CLI

- host **54/54**; targets core **18/18**, sqlite **17 PASS + 1 EXPECTED_RED** (#156).
- `target/release/vis.exe`: **11 205 120 B**,
  sha256 `099f1be90ce1fc6d33d0fd99f5a9456d7a635be3944ac84200bcfba9788363d6`.
- `vis --version` → `vis 0.1.0`; `doctor` → `os windows`, `arch x86_64`, `sqlite on`,
  `features sqlite,tls,pkg-sig,http2`.
- Smoke: `check` → TAP `1..1 ok 1`; `run` → `42`; `run --version` → rc 2.

## 7. `--features async-h2` trên Windows

`cargo check -p vis-cli --features async-h2` → **rc 101** (`h2_async.rs:15` `cfg(…, unix)`) —
**EXPECTED**: async-h2 vẫn Unix-only/default-off.

## 8. Log / bằng chứng

| Log `psvn` | sha256 |
|---|---|
| `/c/Users/ps/vislogs/w4s-targeted.log` | `3e428b8a652ff72e1c678c81113d838cb86585cbfb7488066a0d06e6738a3d85` |
| `/c/Users/ps/vislogs/w4s-workspace.log` | `e495294cc6e6d2e398a13528134865b51fc39ad25a603141eb1f5f282417b462` |
| `/c/Users/ps/vislogs/w4s-release.log` | `e0c00d283f26b2d3ba89e50fb60110ff75ff3c40f313876bc7dbb21a8c3faf1c` |
| `/c/Users/ps/vislogs/w4r-soak.log` (candidate 1h) | `f08bb9a6fe044fff2436ba6b536dfc51fcaf584934c1fa18a6ba08cfc30ce2d3` |

- Worktree `psvn` đã xoá sau đo; repo chính @ `73af17bd`, dirty 0; `C:` còn **171G**.
- Local `/` còn **19G** (>5G).

## 9. Kết luận

- **Tag `v0.3.5` (`03bb7b9`)** đạt trên Windows: workspace 3528/0/3, P2 10/10 (pre-cancel xanh),
  P1/P3 xanh, 5/5 targeted, host/targets xanh, release/CLI đạt; 0 flake.
- Candidate 1h soak 386 vòng cũng xanh; async-h2 vẫn **Unix-only/default-off**.
- **Không blocker**; theo dõi free-memory trend khi soak dài.
