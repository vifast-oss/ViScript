<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-296a — rebase lên `ceac954` (sau tag `v0.4.0`)

> **Người làm**: W3, 2026-11-23 · nhánh `plan/w4-st296a-tests` · **không** push/merge/tag.
> **Commit gốc**: `f961b83` (base `8cc3588`) → **rebase** lên **`ceac954`** (main có ST-292 +
> version `0.4.0` + final QA) → commit **`5765a39`** (parent `ceac954`).

## 1. Xung đột / cách xử lý

- **0 xung đột conflict thực tế** khi `git rebase ceac954`: Git theo rename đưa đúng bản `main`
  của `st289_version.rs` (**`BAN = "0.4.0"`**) và `vd270_test_slowest.rs` vào path mới
  `crates/vis-cli/tests/cli_it/`; so sha256 khớp `ceac954`.
- `vs3_worker_frame.rs` / `vs3_worker_frame_vm.rs` mới của ST-292 **giữ standalone**, không gom,
  sha256 khớp `ceac954`.
- `crates/vis-cli/Cargo.toml`: giữ 4 entry gộp (`sqlite_it`, `tls_it`, `http2_it`, `sig_it`);
  worker-frame auto-discover standalone; không xung đột.

## 2. Sau rebase

| Hạng mục | Kết quả |
|---|---|
| `./vis-dev gate --fast` | **PASS — 3656 passed / 4 skipped** (92 binary; wall **210,77 s**) |
| `nextest` binary `vis-cli` | **24** = 12 nhóm + 3 standalone + 7 `vs3_worker*` + 2 bin |
| fmt / clippy / R1..R7 | **PASS** |
| ST-292 | 18 ca mới (`vs3_worker_frame` 12 + `_vm` 6) có mặt, standalone |

Không sửa code sản phẩm sau rebase; chỉ journal này. Đĩa cuối **15 G** (> 10 G).
