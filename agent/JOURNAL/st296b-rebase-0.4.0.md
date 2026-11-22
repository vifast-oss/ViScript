<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-296b — rebase lên `main ceac954` (v0.4.0 tag candidate)

> **Người làm**: subagent VS3-W2, 2026-11-23. **Nhánh**: `rebase/st296b` trong worktree
> `plan-w2-st296b-rebase`, tạo từ `plan/w3-st296b-crates` @ `c71e65a`.
> **Đích rebase**: `ceac954` (`[ST-296] final QA Linux e47b3ca: gate 3656/4...`).
> Không push/merge/tag; không GitHub Actions. `CARGO_BUILD_JOBS=3`, `TMPDIR=/home/ps/tmp`,
> target riêng `target/w3b`.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Rebase | **thành công, không xung đột thủ công** |
| Commit journal `c71e65a` | **bỏ** — patch-id trùng `ceac954` (đã có `JOURNAL/final-qa-e47b3ca.md`) |
| Commit ST-297 `30a7f98` | **git tự skip** — đã có trên main |
| Commit ST-296b | replay thành **`002c076`** (+ lock/journal trong commit rebase) |
| Version/cache | workspace `0.4.0`; `vis-loader`/`vis-lsp` kế thừa `version.workspace = true`; `Cargo.lock` chỉ đổi 2 dòng 0.3.6→0.4.0 |
| Gate | **3656 passed / 0 failed / 4 skipped** |
| R1..R7 / fmt / clippy | **PASS** (R7 self-test 15/15) |
| `cargo check -p vis-cli --no-default-features --tests` | **PASS** |
| `bin/vis-targets` core | **18 PASS / 0 EXPECTED_RED / 0 FAIL** |
| Blocker | **không** |

## 1. Cách rebase

- `git worktree add -b rebase/st296b … plan/w3-st296b-crates`.
- `git reset --hard 30a7f98` (bỏ `c71e65a`; patch-id `ab9d7f0f…` trùng `ceac954`).
- `git rebase ceac954`:
  - `30a7f98` (ST-297 plan) **skipped** vì main đã có;
  - `7c4c205` replay sạch, không conflict → `002c076`.
- Cập nhật `Cargo.lock` + `vis-host-wasm/Cargo.lock` cho version `0.4.0` của các crate thành viên
  (`vis-loader`, `vis-lsp`, `vis-diag`, `vis-syntax`, `vis-bytecode`, `vis-interp`, …).
  Không đổi dependency/feature/cache key.

## 2. Kiểm

| Lệnh | Kết quả |
|---|---|
| `cargo check -p vis-cli --no-default-features --tests` | **PASS**, 0 warning |
| `cargo fmt --all --check` | **PASS** |
| `./vis-dev gate --fast` | **PASS — 3656 passed / 0 failed / 4 skipped**; fmt/clippy/R1..R7 OK |
| `bin/vis-targets` core | **18/18**; sqlite 16 PASS + 2 EXPECTED_RED (wasm/msvc C toolchain #156) |

Log: `JOURNAL/st296b-gate.log`, `JOURNAL/st296b-targets.log`.

## 3. Kết luận

- ST-296b đã rebase sạch lên `ceac954` với version `0.4.0`, không xung đột logic, gate đúng
  **3656/4**, targets core 18/18.
- **Chưa merge vào main** — chờ A0 tag `v0.4.0` xong; không tag/push/merge.

## Log lớn ngoài repo (R7)

Đã chuyển `st296b-gate.log`/`st296b-targets.log` sang `/home/ps/vislogs/st296b/`; sha256:
```
c55eab3d6b95a1480c6fc243509316bbbe90dc6b9439314b78117b2809b1c881  st296b-gate.log
7debefd1a86aeff25d13a5407b2e5a537739658c77331fd713543c74d6dbef38  st296b-targets.log
```
