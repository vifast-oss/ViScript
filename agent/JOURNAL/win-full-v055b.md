<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Windows `--full` trên `main` `e6ca52e` (v0.5.5 + vá `vis-targets` CRLF)

> **Người đo**: subagent (phiên sạch), 2026-11-23 · time-box ~1 h.
> **Mốc**: `main` **`e6ca52e45c72f99bf05700b560f9c20397486d00`**
> (`[ST-310f] vis-targets: strip CRLF Windows (python stdout)`; version **0.5.5**;
> baseline Linux trước đó **3720/0/4**, host **54/54** chỉ fail tooling).
> `psvn` worktree **riêng** `/c/Users/ps/viscript-win-full-v055b` @ `e6ca52e`, dirty **0**.
> `TMPDIR=/c/Users/ps/tmp` (ngoài `/tmp` tmpfs), jobs **3**. Không ssh `mps`; không tag/push/merge;
> không GitHub Actions.
> **Kết quả: PASS** — full gate **3720 passed / 0 failed / 4 ignored**, host **54/54**,
> targets **18/18 + 17/1**, wall **552 s**. Không blocker.

## 0. TL;DR

| Hạng mục | Kết quả | Thời gian |
|---|---|---:|
| `bin/vis-gate-windows --full` | **exit 0** — **122 block; 3720 passed / 0 failed / 4 ignored** | **552 s** (đo 20:25:05Z → 20:34:17Z; header/KET 20:25:06Z → 20:34:16Z = **550 s**) |
| Workspace (`cargo test --workspace --no-fail-fast`) | **3719 passed / 0 failed / 4 ignored**, **121** test-result block | trong 552 s |
| host (lần chạy riêng trong full) | **54/54** (1..54, **120 lần chạy**, 0 lệch, VM **54/54**) | 3,92 s |
| targets core | **PASS=38, EXPECTED_RED=0, FAIL=0** (18 crate × 2 target + 2 crate phụ) | — |
| targets sqlite | **PASS=37 + EXPECTED_RED=1**, FAIL=0 (`vis-cli` × wasm32, nợ #156) | — |
| wasm32-wasip1 (báo cáo) | coverage **388/390** (99,5%), suite **28/28**, known-gap 2 stack-overflow interp | — |
| Dirty cuối | worktree `e6ca52e` **0** mục (untracked **0**); HEAD không đổi | — |
| Blocker | **Không** | — |

## 1. Môi trường và worktree

- `psvn`: `MINGW64_NT-10.0-26300 3.5.4-395fda67.x86_64`, **6 nhân**;
  `cargo 1.100.0 (3acc79c0c 2026-11-05)`, `rustc 1.100.0 (0ef1bc386 2026-11-09)`;
  `TMPDIR/TMP/TEMP=/c/Users/ps/tmp`; `python 3.13.3` (cần cho vá CRLF ở `vis-targets`).
- Repo chung `/c/Users/ps/viscript-moi` đang detached `8cc3588a`, sạch; đã `git fetch origin main`
  → `origin/main = a0b7beb4` (đã chứa `e6ca52e`) rồi tạo worktree riêng:

```bash
git -C /c/Users/ps/viscript-moi fetch origin main
git -C /c/Users/ps/viscript-moi worktree add --detach \
    /c/Users/ps/viscript-win-full-v055b e6ca52e
# HEAD=e6ca52e45c72f99bf05700b560f9c20397486d00 · dirty=0
```

- **jobs = 3**: `vis-remote` không chuyển env sang thân lệnh trên máy đích, và
  `vis-gate-windows` chưa có biến riêng cho jobs (xem `win-quick-gd3.md` §5.2). Lượt này đặt
  `[build] jobs = 3` trong **`/c/Users/ps/.cargo/config.toml`** (ngoài repo ⇒ không sinh mục bẩn),
  **xoá sau khi đo** (`GLOBAL_CFG_REMOVED=1`). Không dùng `.cargo/config.toml` trong worktree:
  bản Git trên `psvn` đọc `info/exclude` ở **git-common-dir** (`.../viscript-moi/.git/info/exclude`),
  không đọc `.../.git/worktrees/<tên>/info/exclude` (đã thử: `git check-ignore` không khớp ⇒
  worktree vẫn `?? .cargo/`). Đặt config ngoài repo là cách sạch, không đụng exclude dùng chung.
- Không sửa mã nguồn; không ssh `mps`.

## 2. Full gate

Lệnh đúng như yêu cầu (env trỏ worktree riêng):

```bash
VIS_REMOTE_PSVN_REPO=/c/Users/ps/viscript-win-full-v055b \
VIS_GATE_WINDOWS_REPO=/c/Users/ps/viscript-win-full-v055b \
VIS_GATE_WINDOWS_TMPDIR=/c/Users/ps/tmp \
VIS_GATE_WINDOWS_LOG=/tmp/opencode/vis-gate-windows-full-v055b.log \
bin/vis-gate-windows --full
```

- **exit 0**, `vis-remote-exit=0`, `.exit` trên máy đích = **0**; wall **552 s**
  (đo bằng `date` 20:25:05Z → 20:34:17Z; header/KET của gate 20:25:06Z → 20:34:16Z = **550 s**).
- Thân lệnh (do script sinh): `cargo test --workspace --no-fail-fast && cargo test -p vis-cli
  --test host_conformance -- --nocapture && bash bin/vis-targets`, chạy tại
  `/c/Users/ps/viscript-win-full-v055b`, `TMPDIR=/c/Users/ps/tmp`.
- Workspace: **3719 passed / 0 failed / 4 ignored** (121 test-result block);
  không có `FAILED`, không `panicked`, không `error: test failed`, **0** dòng `error`.
- host: `1..54`, **54 ca / 120 lần chạy** (interp + VM), 0 lệch, VM **54/54**;
  lần chạy riêng thêm **1 passed** (3,92 s) ⇒ tổng **3720**.
- targets: core **PASS=38 / EXPECTED_RED=0 / FAIL=0**; sqlite **PASS=37 + 1 EXPECTED_RED**, 0 FAIL;
  phủ **100% workspace (18 crate)**. `vis-cli × x86_64-pc-windows-msvc` **PASS** (nhắc “từng
  EXPECTED_RED (#156) nay xanh — cần rà soát nợ #156”), `vis-cli × wasm32-unknown-unknown` vẫn
  `EXPECTED_RED` đúng nợ #156.
- wasm32-wasip1: coverage 388/390 eligible (99,5%), suite 28/28, known-gap 2 stack-overflow interp
  (không tuyên bố full).
- 4 ignored (đều có chủ đích, không phải fail):
  `lat256_budget::measure_workloads`, `lat257_b_vm_steps::measure_vm_instruction_counts`,
  `lat257_fairness::quet_q_phan_bo_khong_chon_default` (measurement: chậm riêng, finite, seeded),
  và 1 doc-test `vis-plugin-macros`.
- **12 dòng `warning:`**, **0 `error`** — đều là cảnh báo sẵn có: `vish-edit` dead-code
  (`JobControl::next`, helper `tam/ghi/duong/chay`, unused import `JobControl` trong
  `completion_job`) và 4 helper `cfg(unix)` không dùng của `security_it` trên Windows.
  Không ảnh hưởng kết quả.

### 2.1 Đối chiếu kỳ vọng ~3720/0/4, host 54/54

- **Khớp chính xác kỳ vọng**: Windows **3720/0/4** (workspace 3719 + host 1), host **54/54**,
  targets **18/18** crate phủ + **17 PASS/1 EXPECTED_RED** ở bảng sqlite; 0 failed, exit 0.
- 4 ignored trùng đúng danh sách baseline (3 measurement + 1 doc-test) ⇒ không có test nào bị
  bỏ qua thêm trên Windows.
- Vá `bin/vis-targets` (strip CRLF từ stdout python) chạy đúng trên Windows: bảng targets in sạch,
  không còn ký tự `\r` lẫn trong kết quả.

## 3. Bằng chứng

| Log | sha256 |
|---|---|
| local `/home/ps/vislogs/vis-gate-windows-full-v055b.log` (254 035 B; bản gốc `/tmp/opencode/vis-gate-windows-full-v055b.log`, copy nguyên byte) | `2349d428a280bc4836b9049167293e056e5203c65a50ead4c966dfe7251d447e` |
| trên `psvn` `/c/Users/ps/vislogs/vis-remote-20261123T202505Z.log` (bản gốc của gate) | `8686d0800e43444fe267b86191839aacb8cabde06773e76f1c27aaf6b334ed8e` |

- sha256 bản máy đích khớp dòng `# sha256(log_tren_may_dich)=…` ở footer; `.exit` = **0**.
- Header ghi `head_full=e6ca52e45c72f99bf05700b560f9c20397486d00`, `git_untracked: 0`,
  `git_status:` rỗng; sau đo `DIRTY=0`, `UNTRACKED=0`, HEAD không đổi.
- Log nằm **ngoài repo** (cả local lẫn `psvn`); không sinh tệp untracked trong worktree.
- `C:` còn **19 G** (476 G, dùng 97%) sau build — đủ cho lượt đo, không chặn.

## 4. Kết luận / blocker

- `main` **`e6ca52e`** — mốc v0.5.5 đã vá `vis-targets` CRLF — đạt `vis-gate-windows --full`
  trên Windows: workspace **3719/0/4** + host **1** = **3720/0/4**, host **54/54**,
  targets **18/18 + 17/1**, wall **552 s**, dirty **0**.
- **Không blocker.** Không tag/push/merge; không ssh `mps`.
- Journal này nằm trên nhánh **`plan/win-full-v055b`** tạo từ `e6ca52e`; chỉ thêm tệp này,
  không merge/push/tag.
