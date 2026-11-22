<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Verify tag `v0.3.6` trên Windows — `8fa2c09`

> **Người đo**: subagent VS3-W4, 2026-11-22 · **verify tag** (time-box 45 phút).
> **Tag**: **`v0.3.6`** (annotated) → **`e27a5ed144a150242e12b6b440dcc91c41bc4e44`** →
> commit **`8fa2c097bcdf2eb1eace583ac7fae016a90647b6`** (`merge ST-289: final Linux v0.3.6 0645700`;
> code `0645700` đã được W4 verify trước đó).
> `psvn` `/c/Users/ps/viscript-moi` **detached đúng tag**, `git describe --tags --exact-match`
> = **`v0.3.6`**, **dirty 0**.
> **Kết quả: tag `v0.3.6` đạt trên Windows** — workspace **3538/0/3**, cancel **10/10 lần 5/5**
> (5 default + 5 serial), host/targets/release xanh. Không tag/push/merge; không GitHub Actions.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Tag / HEAD | `v0.3.6` → annotated `e27a5ed…` → commit `8fa2c09…`; describe exact `v0.3.6`; dirty 0 |
| `vis --version` | **`vis 0.3.6`** (debug + release); `version --json` đúng; doctor 0.3.6 |
| Workspace 1 vòng | **355 block; 3538 passed / 0 failed / 3 ignored** — **415 s** |
| `vs3_http_cancel` default ×5 | **5/5 lần 5/5** (0,23–0,25 s/lần) |
| `vs3_http_cancel --test-threads=1` ×5 | **5/5 lần 5/5** (0,64–0,67 s/lần) |
| P2 / P1 / P3 | **14/14**, **8/8**, **10/10** |
| host / targets | **54/54** (120 lần chạy, VM 54/54); core **18/18**, sqlite **17 + 1 EXPECTED_RED** |
| Release | `vis.exe` **11 257 344 B**, sha256 `812b9b81…d38f63f4`; CLI/smoke xanh |
| `cargo check --features async-h2` | **rc 101 EXPECTED** (Unix-only) |
| Blocker | **Không** |

## 1. Tag / môi trường

- `git fetch origin main --tags` trên `psvn` nhận tag mới; `git checkout --detach v0.3.6`.
- `git rev-parse v0.3.6^{commit}` = **`8fa2c097bcdf2eb1eace583ac7fae016a90647b6`**;
  `git describe --tags --exact-match` = **`v0.3.6`**; dirty **0** trước và sau đo.
- `psvn`: Windows `MINGW64_NT-10.0-26300`, 6 CPU; cargo/rustc **1.100.0** (MSVC);
  `C:` còn **168 G**.

## 2. Workspace (1 vòng)

| Lệnh | Kết quả | Thời gian |
|---|---|---:|
| `cargo test --workspace --no-fail-fast` | **355 block; 3538 passed / 0 failed / 3 ignored** | **415 s** |

- `rc=0`, không dòng `FAILED`; 3 ignored là ca đo lường có `#[ignore]` như baseline.
- `./target/debug/vis.exe --version` → `vis 0.3.6`; `version --json` →
  `{"name":"vis","version":"0.3.6"}`.

## 3. `vs3_http_cancel`

| Pha | Cách chạy | Kết quả |
|---|---|---|
| **C** | full suite default ×5 | **5/5 lần 5/5** (0,23–0,25 s/lần) |
| **D** | full suite `--test-threads=1` ×5 | **5/5 lần 5/5** (0,64–0,67 s/lần) |

- **Tổng 10/10 lần chạy xanh**; không còn `10053`/`V:post` sau huỷ — khớp kết quả fix `0645700`
  (trước fix: default 2/10 fail, serialized 8/10 fail).
- Log 10 lần: `agent/JOURNAL/tag-v036-win.log`.

## 4. Targeted

| Suite | Kết quả |
|---|---|
| `vs3_worker_eval_parallel` (P2) | **14/14** |
| `vs3_worker_eval_local` (P1) | **8/8** |
| `vs3_worker_eval_vm` (P3) | **10/10** |

## 5. Host / targets / release / async-h2

- host: **54/54** (`1..54`, 120 lần chạy interp+VM, 0 lệch; VM 54/54).
- targets: rc 0 — core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED, 0 FAIL**.
- `cargo check --features async-h2`: **rc 101 EXPECTED** (`E0433: cannot find h2_async in net`,
  async-h2 Unix-only/default-off — không phải lỗi của tag).
- Release: `cargo build --release -p vis-cli --bin vis` rc 0 (artifact tái dùng, 0 s);
  `target/release/vis.exe` **11 257 344 B**,
  sha256 **`812b9b81bc77778660d0e4bb525a55496fa6c91d54385a10688fce08d38f63f4`**.
- CLI release: `vis 0.3.6`; `version --json` đúng; `doctor` (`os windows`, `arch x86_64`,
  `sqlite on`, `features sqlite,tls,pkg-sig,http2`); smoke `check` TAP `ok 1`, `run` → `42`,
  `run --version` rc 2.

## 6. Bằng chứng

| Log `psvn` (`/c/Users/ps/vislogs/`) | sha256 |
|---|---|
| `tag-workspace.log` | `35c87aad4e041beea7e7b0087e63cb0f029ba937a2d046fb5c0257b4634228bf` |
| `tag-cancel-summary.log` (10 lần) | `d4c71f3dd572d0ea11c5ed5032e18f21efa8fa8cda79ba225c8d950cc0cc5255` |
| `tag-host.log` | `c3801e1e02406a57b69d01c4eef0e02f34036735bf3899b606fb59b556ca2afa` |
| `tag-targets.log` | `8f824cc9e20cd9f217aa8a5c27ca77a0eb712a276814b6e0690001c0bd889df3` |
| `tag-release.log` | `6b1c8cbe0bd9cb3b12baa322a9182419af04da4050d3affc7c9798b1b46e30b5` |
| `tag-async-h2.log` | `e041f0f0e70a58474df7274661dd530750958ea8fff13605b4afe4890e16c7a8` |
| `tag-p2.log` / `tag-p1.log` / `tag-p3.log` | `40bdef72…da069` / `ac950019…474d3` / `0d4fdd87…146d4` |

- `psvn` HEAD `8fa2c09`, dirty 0 sau đo; `C:` còn 168 G; local `/` còn 12 G (>5 G).

## 7. Kết luận

- **Tag `v0.3.6` (`8fa2c09`, annotated `e27a5ed`)** đạt trên Windows: workspace **3538/0/3**,
  cancel **10/10 lần 5/5** (default + serial), P1/P2/P3 xanh, host **54/54**, targets
  **18/18 + 17/1**, release/CLI/smoke xanh, version **0.3.6** nhất quán.
- `async-h2` vẫn **Unix-only/default-off** (rc 101 expected) — đúng thiết kế.
- **Không blocker.** Nguyên trạng: Windows **Tier 2, chưa tuyên bố hỗ trợ**; không tag/push/merge
  trong phiên này.
