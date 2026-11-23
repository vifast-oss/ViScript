<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Final Windows QA — `v0.4.0` candidate `e47b3ca` (ST-292 merged)

> **Người đo**: subagent VS3-W4, 2026-11-23 · **QA cuối Windows trước tag `v0.4.0`**.
> **Mốc**: `main` **`e47b3ca90567b4ea2dc561eabc1436aa01c250e3`**
> (`[ST-292] R7: dua st292-gate.log ra /home/ps/vislogs`; Linux gate 3656/4).
> `psvn` worktree **riêng** `/c/Users/ps/viscript-v040-e47b3ca` @ `e47b3ca`, dirty **0**.
> `TMPDIR=/c/Users/ps/tmp` (ngoài `/tmp`). Không tag/push/merge; không GitHub Actions.
> **Kết quả: PASS toàn bộ** — full gate debug **3607/0/3**, release workspace **3606/0/3**,
> worker-frame + budget CLI đúng parity Linux. **Đề xuất: TAG `v0.4.0` @ `e47b3ca`.**

## 0. TL;DR

| Hạng mục | Kết quả | Thời gian |
|---|---|---:|
| `bin/vis-gate-windows --full` (debug) | **exit 0** — **361 block; 3607 passed / 0 failed / 3 ignored** | **700 s** |
| host (trong full) | **54/54** (1..54, 120 lần chạy, VM 54/54) | 3,8 s |
| targets | core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED** | — |
| `cargo test --release --workspace --no-fail-fast` | **360 block; 3606 passed / 0 failed / 3 ignored** | **534 s** |
| `vs3_worker_frame` / `_vm` (release) | **12/12** / **6/6** | 0,04 / 0,02 s |
| P2 / P1 / P3 (release) | **14/14**, **8/8**, **10/10** | — |
| select / h2 / cancel (release) | **25/25**, **33/33**, **8/8** | — |
| Budget CLI | `--workers 4 --max-budget 5000` (loop) **rc 101 + VS0510**, giống `W=1`; `--workers 0` **rc 2**; `--workers 4` **rc 0/42** | — |
| Release | `vis.exe` **11 530 240 B**, sha256 `2eb7a314…50da7f`; `vis 0.3.6` | — |
| Blocker | **Không** | — |

## 1. Full gate Windows (debug + host + targets)

Lệnh (env trỏ worktree riêng):

```bash
VIS_REMOTE_PSVN_REPO=/c/Users/ps/viscript-v040-e47b3ca \
VIS_GATE_WINDOWS_REPO=/c/Users/ps/viscript-v040-e47b3ca \
VIS_GATE_WINDOWS_TMPDIR=/c/Users/ps/tmp \
bin/vis-gate-windows --full
```

- **exit 0**, **700 s**; `--require-clean` thoả (dirty 0).
- Workspace: **361 block; 3607 passed / 0 failed / 3 ignored**; không dòng `FAILED`.
- host: **54/54**, 120 lần chạy interp+VM, 0 lệch, VM 54/54.
- targets: core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED**, 0 FAIL.
- Log local `/tmp/vis-gate-windows-full-e47b3ca.log` sha256
  `ae7dafa39397df6682a077d33722bd3a82b2fef7bee8ba5d796250ed65ae013f`.

## 2. Release workspace (1 vòng)

| Lệnh | Kết quả | Thời gian |
|---|---|---:|
| `cargo test --release --workspace --no-fail-fast` | **360 block; 3606 passed / 0 failed / 3 ignored** | **534 s** |

- `rc=0`; 1 test ít hơn debug (ca debug-only), đúng kỳ vọng ~3607 → **3606 release**.
- 3 ignored là ca đo lường `#[ignore]`.

## 3. Targeted (release)

| Suite | Kết quả |
|---|---|
| `vs3_worker_frame` | **12/12** |
| `vs3_worker_frame_vm` | **6/6** |
| `vs3_worker_eval_parallel` (P2) | **14/14** |
| `vs3_worker_eval_local` (P1) | **8/8** |
| `vs3_worker_eval_vm` (P3) | **10/10** |
| `vs3_select` | **25/25** |
| `vs3_http2` | **33/33** |
| `vs3_http_cancel` | **8/8** |

- Tất cả rc 0, 0 failed/ignored.

## 4. CLI budget/worker (`run --workers …`)

Chương trình ngắn `print(42)`; loop `while i < 1_000_000`:

| Lệnh | Windows release | Linux debug (parity) |
|---|---|---|
| `run --workers 4 --max-budget 5000 a.vis` | **rc 0**, `42` | **rc 0**, `42` |
| `run --workers 4 --max-budget 5000 loop.vis` | **rc 101**, `VS0510: vượt ngân sách ngữ nghĩa (E_EXPR)` | **rc 101**, cùng `VS0510` |
| `run --workers 1 --max-budget 5000 loop.vis` | **rc 101**, cùng `VS0510` | **rc 101**, cùng `VS0510` |
| `run --workers 0 a.vis` | **rc 2**, ``--workers 0` không hợp lệ (phải >= 1)` | **rc 2**, cùng thông điệp |
| `run --workers 4 a.vis` | **rc 0**, `42` | **rc 0**, `42` |

- **Parity W=4 ↔ W=1** cho ngân sách; `--workers 4` không đổi hành vi chương trình ngắn.
- Chương trình ngắn **không** vượt 5000 (đúng ngân sách, không phải lỗi); loop vượt ⇒ `VS0510` exit 101.

## 5. Release build / CLI / smoke

- `cargo build --release -p vis-cli --bin vis` rc 0;
  `target/release/vis.exe` **11 530 240 B**,
  sha256 **`2eb7a3148731d92fb4bf83dfa364463ab7f6edfd53c195b47016f540f550da7f`**.
- `vis 0.3.6`; `version --json` đúng.
- Smoke: `check a.vis` TAP `1..1 ok 1` rc 0; `run a.vis` → `42`; `--workers 4` → `42`.

## 6. Bằng chứng

| Log `psvn` (`/c/Users/ps/vislogs/`) | sha256 |
|---|---|
| `v040-rel-workspace.log` | `e522319b…4b91593` |
| `v040-wf.log` / `v040-wfvm.log` | `ab34ffd6…f311b8` / `a9a9c3bd…8ed56f5` |
| `v040-p2.log` / `v040-p1.log` / `v040-p3.log` | `aeb006e2…26fddce4` / `e97eaf01…5b165c8e` / `caff93a7…b9b57b12` |
| `v040-select.log` / `v040-h2.log` / `v040-cancel.log` | `02bd1ff0…303c28d4` / `ef01e4dd…f047670` / `ec3c5f59…206b822` |
| `v040-release-build.log` | `6b1c8cbe…b46e30b5` |
| full log local | `ae7dafa3…65ae013f` |

- Worktree HEAD `e47b3ca`, dirty **0** sau đo; `psvn` `C:` còn **94 G**.
- Local `/` còn **12 G** (>10 G).

## 7. Đề xuất tag

**TAG `v0.4.0` @ `e47b3ca`** — không còn blocker Windows trong phạm vi QA cuối:

- Full gate debug **3607/0/3**, release **3606/0/3**, host **54/54**, targets **18/18 + 17/1**.
- `ST-292`: `vs3_worker_frame` **12/12**, `_vm` **6/6**; `--workers 4 --max-budget 5000` trap
  **`VS0510` exit 101** parity `W=1`; `--workers 0` rc 2; `--workers 4` chạy đúng.
- Version **0.3.6** (release hiện tại); release/CLI/smoke xanh.
- Không tag/push/merge trong phiên này; không GitHub Actions. Ghi chú nguyên trạng: Windows Tier 2
  (chưa tuyên bố hỗ trợ), macOS ngoài phạm vi.
