<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Final Windows — chốt tag `v0.3.6` @ `0645700` (ST-287 fix)

> **Người đo**: subagent VS3-W4, 2026-11-22 · **verify chốt tag** · chỉ đo, không sửa code.
> **Mốc**: `06457003ae3e38bfa682d4ec953157606130849e` (`merge ST-287: fix Windows h1 cancel
> (blocking accept + cancel guard)`; Linux gate 3581/3). `psvn` `/c/Users/ps/viscript-moi`
> detached `0645700`, **dirty 0**; `git describe --tags --always` = `v0.3.5-50-g06457003`.
> **Kết luận: ĐỦ ĐIỀU KIỆN TAG `v0.3.6` @ `0645700`** — workspace **3538/0/3**, cancel
> **40/40** (10 default + 10 serialized + 20 ca riêng), toàn bộ targeted/host/targets/release xanh.
> Không tag/push/merge trong phiên này; không GitHub Actions.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| HEAD / describe | `0645700…`; `v0.3.5-50-g06457003`; dirty **0** |
| `vis --version` | **`vis 0.3.6`** (debug + release); `version --json` đúng; doctor 0.3.6 |
| Workspace 1 vòng | **355 block; 3538 passed / 0 failed / 3 ignored** — **457 s** (đúng kỳ vọng) |
| `vs3_http_cancel` default ×10 | **10/10 lần 5/5** (0,25–0,26 s/lần) |
| `vs3_http_cancel --test-threads=1` ×10 | **10/10 lần 5/5** (0,65–0,71 s/lần) |
| ca `lap_nhieu_vong` riêng ×20 | **20/20** (0,25 s/lần) |
| P2 / P1 / P3 | **14/14**, **8/8**, **10/10** |
| h2 / pkg / default / TLS / net | **19/19**, **25/25**, **22/22**, **22/22**, **9/9** |
| host / targets | **54/54** (120 lần chạy, VM 54/54); core **18/18**, sqlite **17 + 1 EXPECTED_RED** |
| Release | `vis.exe` **11 257 344 B**, sha256 `812b9b81…d38f63f4`; CLI/smoke xanh |
| `cargo check --features async-h2` | **rc 101 EXPECTED** (Unix-only) |
| Blocker | **Không** — đủ điều kiện tag |

## 1. Môi trường / mốc

- `psvn`: Windows `MINGW64_NT-10.0-26300`, 6 CPU; cargo/rustc **1.100.0** (MSVC).
- `git fetch origin main` → checkout `--detach 0645700`; dirty 0 trước/sau đo; prebuild 18 s.
- `C:` còn **168 G**; local `/` còn **12 G** (>5 G).
- Bối cảnh: `agent/JOURNAL/win-h1-cancel-matrix.md` đo flake tại `a1930cb` (default 8/10,
  serialized 2/10, ca riêng 46/50). Commit `0645700` merge fix ST-287 (blocking accept + cancel guard).

## 2. Workspace (1 vòng)

| Lệnh | Kết quả | Thời gian |
|---|---|---:|
| `cargo test --workspace --no-fail-fast` | **355 block; 3538 passed / 0 failed / 3 ignored** | **457 s** |

- `rc=0`, không dòng `FAILED`; 3 ignored là ca đo lường có `#[ignore]` như baseline.
- `./target/debug/vis.exe --version` → `vis 0.3.6`; `version --json` → `{"name":"vis","version":"0.3.6"}`.

## 3. `vs3_http_cancel` — hết flake

| Pha | Cách chạy | Kết quả |
|---|---|---|
| **C** | full suite default ×10 | **10/10 pass** (mỗi lần 5/5; 0,25–0,26 s) |
| **D** | full suite `--test-threads=1` ×10 | **10/10 pass** (mỗi lần 5/5; 0,65–0,71 s) |
| **E** | `cancel_http_h1_lap_nhieu_vong_khong_ro` ×20 | **20/20 pass** |

- **Tổng 40/40 lần chạy xanh**, không có `10053`/`V:post` sau huỷ. So với trước fix (`a1930cb`):
  default 2/10 fail, serialized 8/10 fail, ca riêng 4/50 fail ⇒ fix xử lý đúng.
- Thời gian mỗi suite giảm còn ~0,25 s (trước ~0,86–1,0 s) — phù hợp thay đổi blocking accept.

## 4. Targeted

| Suite | Kết quả |
|---|---|
| `vs3_worker_eval_parallel` (P2) | **14/14** |
| `vs3_worker_eval_local` (P1) | **8/8** |
| `vs3_worker_eval_vm` (P3) | **10/10** |
| `vs3_http2` | **19/19** |
| `pkg` | **25/25** |
| `vs3_default` | **22/22** |
| `vs3_tls` | **22/22** (Windows có thêm 1 ca `#[cfg(windows)]`; Linux 21) |
| `net` | **9/9** |

## 5. Host / targets / release / async-h2

- host: **54/54** (`1..54`, 120 lần chạy interp+VM, 0 lệch; VM 54/54).
- targets: rc 0 — core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED, 0 FAIL**.
- `cargo check --features async-h2`: **rc 101 EXPECTED** — `E0433: cannot find h2_async in net`
  (async-h2 Unix-only; không phải lỗi của candidate).
- Release: `cargo build --release -p vis-cli --bin vis` rc 0 (31 s);
  `target/release/vis.exe` **11 257 344 B**,
  sha256 **`812b9b81bc77778660d0e4bb525a55496fa6c91d54385a10688fce08d38f63f4`**.
- CLI release: `vis 0.3.6`; `version --json` đúng; `doctor` (`os windows`, `arch x86_64`,
  `sqlite on`, `features sqlite,tls,pkg-sig,http2`); smoke `check` TAP `ok 1`, `run` → `42`,
  `run --version` rc 2.

## 6. Bằng chứng

| Log `psvn` (`/c/Users/ps/vislogs/`) | sha256 |
|---|---|
| `fix-workspace.log` | `d98d909ec80cee83f71b75fa53ea770e97a4accb3cac392cf09dc81ded33226e` |
| `fix-cancel-summary.log` (40 lần) | `b4a7d46b7b76d0e608e72bb1b48f1aa88f8bfd11f617b74fc2d9fc0d3acaee85` |
| `fix-host.log` | `c43a77e0142f530730b3869da44190f64790766a9230a9019dec14dd8b8dbec5` |
| `fix-targets.log` | `8f824cc9e20cd9f217aa8a5c27ca77a0eb712a276814b6e0690001c0bd889df3` |
| `fix-release.log` | `34b6ed8125c11f93da81bc0753cd49d92ebdc1c3762dbf350d9e604d8b7fd3a5` |
| `fix-async-h2.log` | `2001b6ade8ef5b8ab66267f22c742ff6a7616ea3169edb58557764524ed98793` |
| `fix-p2.log` / `fix-p3.log` / `fix-h2.log` / `fix-pkg.log` | `5de8f8e2…681f927` / `a98244ae…ea05aac` / `d8952a2e…22ce7c8e` / `fcdebb9c…1c0796ca` |

- Log 40 lần cancel chép vào repo: `agent/JOURNAL/final-win-v036fix.log`.
- `psvn` HEAD `0645700`, dirty 0 sau đo; `C:` còn 168 G; local `/` còn 12 G.

## 7. Đề xuất tag

**TAG `v0.3.6` @ `0645700`.** Không còn blocker Windows trong phạm vi yêu cầu:

- Workspace đúng **3538/0/3**; cancel h1 **40/40** sau fix (trước fix flake 20–80 %).
- Toàn bộ targeted, host, targets, release/CLI/smoke xanh; version **0.3.6** nhất quán.
- `async-h2` vẫn **Unix-only/default-off** (rc 101 expected) — đúng thiết kế, không chặn tag.
- Lưu ý nguyên trạng: Windows vẫn là **Tier 2, chưa tuyên bố hỗ trợ**; tag dựa trên bằng chứng
  Linux final QA 3581/3 + Windows verify này. Không tag/push/merge trong phiên này.
