<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Final Windows QA — `v0.4.1` candidate `5d7791c`

> **Người đo**: subagent VS3-W4, 2026-11-23 · final QA trước tag `v0.4.1` · time-box 1,5 h.
> **Mốc**: `main` **`5d7791c26b68b2713a7489e560a21d6a577a37c1`**
> (`[ST-296b] R7: ghi sha256 log da chuyen`; đã gồm v0.4.1 + ST-296a/b/c/d).
> `psvn` worktree **riêng** `/c/Users/ps/viscript-v041-5d7791c` @ `5d7791c`, **dirty 0 lúc đầu**;
> `TMPDIR=/c/Users/ps/tmp`; không job psvn khác. Không tag/push/merge; không GitHub Actions.
> **Kết quả chức năng: PASS toàn bộ.** **Phát hiện chặn tag (nhỏ nhưng thật):**
> `vis-host-wasm/Cargo.lock` còn ghim **0.4.0**; `bin/vis-targets` trong full gate tự cập nhật
> 0.4.0 → **0.4.1** ⇒ worktree sạch **thành bẩn** sau cổng. **Đề xuất: chưa tag `v0.4.1`**
> cho tới khi commit lock này (và re-run cổng xác nhận dirty 0).

## 0. TL;DR

| Hạng mục | Kết quả | Thời gian |
|---|---|---:|
| `bin/vis-gate-windows --full` | **exit 0** — **105 block; 3607 passed / 0 failed / 3 ignored** | **478 s** |
| host (trong full) | **54/54** (1..54, 120 lần chạy, VM 54/54) | 3,8 s |
| targets | core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED** | — |
| `cargo test --release --workspace --no-fail-fast` | **104 block; 3606 passed / 0 failed / 3 ignored** | **384 s** |
| Version / cache | **`vis 0.4.1`**; `st289_version` **2/2**; `cache::tests` **3/3**, khoá mới **`42f53893…4d7c0f`**, `assert_ne` khoá 0.4.0 xanh | — |
| Release targeted | wf **12/12**, wf_vm **6/6**, P2 **14/14**, P1 **8/8**, P3 **10/10**, select **25/25**, h2 **33/33**, cancel **8/8** | ~35 s |
| `vis-lsp` / `vis-loader` | **27/27** / **29/29** | — |
| Budget CLI | loop `--workers 4 --max-budget 5000` → **101 + VS0510**; `--workers 0` → **2**; `--workers 4` → **42** | — |
| Release build | `vis.exe` **11 718 144 B**, sha256 `a3d375fd…24ec3d`; check/run OK | — |
| **Lock hygiene** | **FAIL** — `vis-host-wasm/Cargo.lock` bị cargo sửa 0.4.0 → 0.4.1 khi chạy `vis-targets` | — |

## 1. Full gate

Lệnh (worktree riêng):

```bash
VIS_REMOTE_PSVN_REPO=/c/Users/ps/viscript-v041-5d7791c \
VIS_GATE_WINDOWS_REPO=/c/Users/ps/viscript-v041-5d7791c \
VIS_GATE_WINDOWS_TMPDIR=/c/Users/ps/tmp \
bin/vis-gate-windows --full
```

- **exit 0**, **478 s**; **105 block; 3607 passed / 0 failed / 3 ignored**.
- host **54/54** (120 lần chạy, VM 54/54); targets **18/18 + 17/1**.
- Log local `/tmp/vis-gate-windows-full-5d7791c.log` sha256
  `9daf52237edd91fa5240565cc024cad6030bc90c135f84c2e96c846109ff93c7`.

## 2. Release workspace (1 vòng)

| Lệnh | Kết quả | Thời gian |
|---|---|---:|
| `cargo test --release --workspace --no-fail-fast` | **104 block; 3606 passed / 0 failed / 3 ignored** | **384 s** |

- Đúng kỳ vọng 3606/0/3 (1 ca debug-only); 3 ignored là ca đo lường.

## 3. Version / cache

- `vis --version` → **`vis 0.4.1`**; `version --json` đúng.
- `st289_version` (trong `cli_it`) → **2/2 pass** (`BAN = "0.4.1"`, quét mọi crate thành viên).
- `cache::tests` (đã chuyển sang **`vis-loader`**) → **3/3 pass**:
  - khoá mới ghim **`42f5389364ae126f8b9dea4c3bf107c983c3b92d7b1743a5293d547f3d4d7c0f`**;
  - `assert_ne` khoá 0.4.0 **`22572d0b…3a8b`** và 0.4.0 cũ hơn **`e5373caf…026d`** xanh;
  - các assert `VFEE`/`fee::VERSION = 4`/`WEIGHT_TABLE_VERSION = 1`/`INSTRUMENTED_VERSION = 3`
    trong cùng test xanh.

## 4. Targeted (release) + lsp/loader

| Suite | Kết quả |
|---|---|
| `vs3_worker_frame` | **12/12** |
| `vs3_worker_frame_vm` | **6/6** |
| P2 / P1 / P3 | **14/14**, **8/8**, **10/10** |
| select (`async_it vs3_select::`) | **25/25** |
| h2 (`http2_it`) | **33/33** |
| cancel (`net_pkg_it cancel_http_`) | **8/8** |
| `cargo test -p vis-lsp` | **27/27** |
| `cargo test -p vis-loader` | **29/29** |

## 5. CLI budget / release

| Lệnh | Kết quả |
|---|---|
| `run --workers 4 --max-budget 5000` loop | **rc 101**, `VS0510: vượt ngân sách ngữ nghĩa (E_EXPR)` |
| `run --workers 0` | **rc 2**, `--workers 0` không hợp lệ (phải >= 1) |
| `run --workers 4` | **rc 0**, `42` |

- Release: `cargo build --release -p vis-cli --bin vis` rc 0;
  `vis.exe` **11 718 144 B**, sha256
  **`a3d375fd87b9fc040cab386c53ef62821ca2e1e8dcf6901ea2b43b5b1824ec3d`**.
- `vis 0.4.1`; `check` TAP `ok 1`, `run` → `42`.

## 6. Phát hiện chặn tag — lock `vis-host-wasm` còn 0.4.0

- Worktree **sạch lúc đầu** (`dirty=0`); sau full gate (bước `bash bin/vis-targets`) trở thành
  **dirty 1**:
  ```
  M vis-host-wasm/Cargo.lock
  ```
- Diff: 6 package nội bộ `vis-*` trong lock riêng của `vis-host-wasm` bị cargo nâng
  **0.4.0 → 0.4.1** (khớp bump workspace), tức bản commit `5d7791c` **chưa commit lock này**.
- Hệ quả: fresh checkout + chạy cổng chính thức ⇒ cây bẩn; release không tái lập sạch.
  Đây là lỗi **đóng gói/tài liệu**, không phải lỗi sản phẩm (mọi test chức năng xanh).
- **Cần**: commit `vis-host-wasm/Cargo.lock` (0.4.1) vào main; chạy lại `bin/vis-targets`/full gate
  xác nhận `git status` vẫn sạch, rồi mới tag.

## 7. Bằng chứng

| Log `psvn` (`/c/Users/ps/vislogs/`) | sha256 |
|---|---|
| `v041-rel-workspace.log` | `f525a115…9882d012` |
| `v041-st289.log` | `3206b6e2…dc58bc6f8` |
| `v041-cache.log` | `292aa4d8…d775e975` |
| `v041-wf.log` / `v041-wfvm.log` | `63c470cb…41938e785` / `dee31209…eeda313` |
| `v041-p2.log` / `v041-p1.log` / `v041-p3.log` | `5a8ebcad…dfe47809` / `b0a3778c…c48caf3b9` / `5b2a0110…1dfa47c1` |
| `v041-select.log` / `v041-h2.log` / `v041-cancel.log` | `64a27f56…10696c8` / `11e665f9…ce0e7925` / `0df1c662…dd9c7744` |
| `v041-lsp.log` / `v041-loader.log` | `5a70ace6…42e20d782` / `713093d6…39f427a1` |
| `v041-build.log` | `d1235ea4…d95e57` |
| full log local | `9daf5223…09ff93c7` |

- Worktree HEAD `5d7791c`, **dirty 1** (`vis-host-wasm/Cargo.lock`) sau cổng; `psvn` `C:` **74 G**;
  local `/` **20 G** (>10 G).

## 8. Đề xuất tag

**CHƯA tag `v0.4.1` @ `5d7791c`** cho tới khi commit `vis-host-wasm/Cargo.lock` (0.4.1) và xác nhận
cổng không làm bẩn cây. Toàn bộ kiểm chức năng đã đạt: full **3607/0/3**, release **3606/0/3**,
version/cache **0.4.1/42f53893…**, worker-frame + budget + lsp/loader + release/CLI xanh.
Sau khi vá lock: tag `v0.4.1` an toàn (không cần chạy lại soak; chỉ cần `vis-targets`/full gate
xác nhận sạch).
