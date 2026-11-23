<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Windows `--full` trên `main` `f9e7503` (v0.5.0 + security hotfix + `SPEC.md`)

> **Người đo**: subagent (phiên sạch), 2026-11-23 · time-box 1,5 h.
> **Mốc**: `main` **`f9e7503dd3af56f3f7f6ef22b115b497edad7d3b`**
> (`[ST-309] SPEC: R6 wording + doc fixtures skip SPEC`; đã gồm merge **security hotfix cap/plugin**
> và **`SPEC.md`**; version **0.5.0**; Linux gate **3754/4**).
> `psvn` worktree **riêng** `/c/Users/ps/viscript-win-full-f9e7503` @ `f9e7503`, dirty **0**.
> `TMPDIR=/c/Users/ps/tmp` (ngoài `/tmp` tmpfs), jobs **3**. Không ssh `mps`; không tag/push/merge;
> không GitHub Actions.
> **Kết quả: PASS** — full gate **3701 passed / 0 failed / 4 ignored**, host **54/54**,
> targets **18/18 + 17/1**, wall **550 s**. Không blocker.

## 0. TL;DR

| Hạng mục | Kết quả | Thời gian |
|---|---|---:|
| `bin/vis-gate-windows --full` | **exit 0** — **128 block; 3701 passed / 0 failed / 4 ignored** | **550 s** (gate 10:49:22Z → 10:58:29Z = 547 s) |
| Workspace (`cargo test --workspace --no-fail-fast`) | **3700 passed / 0 failed / 4 ignored**, **127** test-result block | trong 550 s |
| host (lần chạy riêng trong full) | **54/54** (1..54, **120 lần chạy**, 0 lệch, VM **54/54**) | 3,86 s |
| targets core | **18/18 PASS** (8 crate × 2 target + 2 crate phụ) | — |
| targets sqlite | **17 PASS + 1 EXPECTED_RED**, 0 FAIL (`vis-cli` × wasm32, nợ #156) | — |
| wasm32-wasip1 (báo cáo) | coverage **388/390** (99,5%), suite **28/28**, known-gap 2 stack-overflow interp | — |
| Dirty cuối | worktree `f9e7503` **0** mục; HEAD không đổi | — |
| Blocker | **Không** | — |

## 1. Môi trường và worktree

- `psvn`: `MINGW64_NT-10.0-26300`, **6 nhân**; `cargo 1.100.0 (3acc79c0c 2026-11-05)`,
  `rustc 1.100.0 (0ef1bc386 2026-11-09)`; `TMPDIR/TMP/TEMP=/c/Users/ps/tmp`.
- Repo chung `/c/Users/ps/viscript-moi` đang **detached `8cc3588a`**, sạch; đã `git fetch origin main`
  → `origin/main = f9e7503d` rồi tạo worktree riêng:

```bash
git -C /c/Users/ps/viscript-moi fetch origin main
git -C /c/Users/ps/viscript-moi worktree add --detach \
    /c/Users/ps/viscript-win-full-f9e7503 f9e7503
# HEAD=f9e7503dd3af56f3f7f6ef22b115b497edad7d3b · dirty=0
```

- **jobs = 3**: `vis-remote` không chuyển env sang thân lệnh trên máy đích, và
  `vis-gate-windows` chưa có biến riêng cho jobs (xem `win-quick-gd3.md` §5.2). Lượt này áp bằng
  `.cargo/config.toml` **tạm trong worktree** (`[build] jobs = 3`), đồng thời thêm đúng đường dẫn
  `.cargo/config.toml` vào `.git/info/exclude` để `git status --porcelain` vẫn **rỗng** (cùng cơ chế
  “tệp tạm ngoài dấu vết” như `win-full-f2d3e73.md` dùng `~/.cargo/config.toml`, nhưng phạm vi hẹp
  hơn: chỉ worktree đo). Sau gate đã **xoá** tệp + dòng exclude (`CONFIG_REMOVED`, `EXCLUDE_HAS_CONFIG: 0`).
- Không sửa mã nguồn; không ssh `mps`.

## 2. Full gate

Lệnh đúng như yêu cầu (env trỏ worktree riêng):

```bash
VIS_REMOTE_PSVN_REPO=/c/Users/ps/viscript-win-full-f9e7503 \
VIS_GATE_WINDOWS_REPO=/c/Users/ps/viscript-win-full-f9e7503 \
VIS_GATE_WINDOWS_TMPDIR=/c/Users/ps/tmp \
VIS_GATE_WINDOWS_LOG=/tmp/opencode/vis-gate-windows-full-f9e7503.log \
bin/vis-gate-windows --full
```

- **exit 0**, `vis-remote-exit=0`, `.exit` trên máy đích = **0**; wall **550 s** (đo bằng `date`
  10:49:20Z → 10:58:30Z; header/KET của gate 10:49:22Z → 10:58:29Z = **547 s**).
- Thân lệnh (do script sinh): `cargo test --workspace --no-fail-fast && cargo test -p vis-cli
  --test host_conformance -- --nocapture && bash bin/vis-targets`, chạy tại
  `/c/Users/ps/viscript-win-full-f9e7503`, `TMPDIR=/c/Users/ps/tmp`.
- Workspace: **3700 passed / 0 failed / 4 ignored** (127 test-result block);
  không có `FAILED`, không `panicked`, không `error: test failed`.
- host: `1..54`, **54 ca / 120 lần chạy** (interp + VM), 0 lệch, VM **54/54**;
  lần chạy riêng thêm **1 passed** (3,86 s) ⇒ tổng **3701**.
- targets: core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED**, 0 FAIL;
  wasm32-wasip1: coverage 388/390, suite 28/28, known-gap 2 stack-overflow interp (không tuyên bố full).
- 4 ignored (đều có chủ đích, không phải fail):
  `lat256_budget::measure_workloads`, `lat257_b_vm_steps::measure_vm_instruction_counts`,
  `lat257_fairness::quet_q_phan_bo_khong_chon_default`, và 1 doc-test `vis-plugin-macros`.
- Có **10 dòng `warning:`**, **0 `error`** (đáng chú ý: `security_it` sinh 4 warning helper
  `cfg(unix)` không dùng trên Windows; `vish-edit/tests/completion_job.rs` unused `JobControl`).
  Không ảnh hưởng kết quả.

### 2.1 Đối chiếu kỳ vọng ~3705 và Linux gate 3754/4

- Windows **3701/0/4** so với kỳ vọng **~3705/0/4**: lệch **−4** (~0,1%), **không phải hồi quy**
  (0 failed, exit 0). Số học khớp chính xác với các test mới từ baseline Windows trước
  `f2d3e73` (**3699/0/4**) và Linux gate hiện tại **3754/4**:
  - `f2d3e73..f9e7503` thêm **7** test: **2** chạy được trên Windows
    (`vis-bytecode/tests/st309_fuzz_verify.rs::decoder_verifier_khong_panic`,
    `vis-plugin-abi/src/wasi.rs::moi_lan_goi_thu_muc_driver_rieng`) và **5** chỉ Unix/Linux
    (`security_it/st309.rs` 2, `parity.rs` 1, `vs3_plugin.rs` 1, `quyen_st295.rs` 1 —
    đều `#[cfg(unix)]`/`cfg(all(test, unix))`).
  - Linux **3747 → 3754 = +7** (đủ 7 test); Windows **3699 → 3701 = +2** (đúng 2 test đa nền).
  - 4 ignored không đổi ⇒ không có test nào bị bỏ qua thêm trên Windows.
- Ghi nhận nhỏ (không chặn): ở bảng sqlite, ô `vis-cli × x86_64-pc-windows-msvc` **PASS**
  (script in nhắc “từng EXPECTED_RED nay xanh — cần rà soát nợ #156”); ô
  `vis-cli × wasm32-unknown-unknown` vẫn `EXPECTED_RED` đúng nợ #156 (thiếu C toolchain).

## 3. Bằng chứng

| Log | sha256 |
|---|---|
| local `/home/ps/vislogs/vis-gate-windows-full-f9e7503.log` (250 120 B; bản gốc `/tmp/opencode/…`, đã copy nguyên byte) | `cae0602f508ba65e5a07799c33f0c4371a443dfb0621ac59aee4ffe6ca8e85c5` |
| local stdout `/home/ps/vislogs/vis-gate-windows-full-f9e7503.stdout` (247 872 B) | `b4f874785e6b9cd3bd7dc50443f0163af63b94e2ca373822f07fd976f02dca93` |
| trên `psvn` `/c/Users/ps/vislogs/vis-remote-20261123T104920Z.log` (247 083 B) | `d508b11e7cf43026d8ab98b990ccf8890e4503c3d0040adc35cc096c857cdbf1` |

- sha256 bản máy đích khớp dòng `# sha256(log_tren_may_dich)=…` ở footer; `.exit` = **0**.
- Log nằm **ngoài repo** (cả local lẫn `psvn`); không sinh tệp untracked trong worktree.
- Worktree sau đo: `HEAD=f9e7503dd3af56f3f7f6ef22b115b497edad7d3b`, `DIRTY=0`;
  config jobs tạm đã xoá; `C:` còn **36 G** (476 G, dùng 93%).

## 4. Kết luận / blocker

- `main` **`f9e7503`** — mốc v0.5.0 đã gồm security hotfix cap/plugin và `SPEC.md` — đạt
  `vis-gate-windows --full` trên Windows: workspace **3700/0/4** + host **1** = **3701/0/4**,
  host **54/54**, targets **18/18 + 17/1**, wall **550 s**.
- **Không blocker.** Không tag/push/merge; không ssh `mps`.
- Journal này nằm trên nhánh **`plan/win-full-f9e7503`** tạo từ `f9e7503`; chỉ thêm tệp này,
  không merge/push/tag.
