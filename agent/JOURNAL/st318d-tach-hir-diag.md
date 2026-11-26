<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-318d — R1 mềm `vis-hir` + `vis-diag` (tách tệp > 1.000 LOC)

- **Ngày**: 2026-11-27 (giờ thấp điểm) · **Vai**: Builder (Agent 2) · **Nhánh**: `plan/st318d` · **Nền**: `main` @ `2b5ccb51`
- **Worktree**: `/home/ps/viswt/st318d` — không chạm repo chính `/home/ps/ViScript`.
- **Tính chất**: **move nguyên văn**, 0 đổi hành vi/API, 0 đổi mã/thông điệp chẩn đoán, 0 đổi thứ tự.

## 1. Bảng tệp trước/sau (`wc -l`)

### 1a. `crates/vis-hir/src/hir.rs` (1088) → entry + `hir/*`

| Tệp | Dòng | Trách nhiệm |
|---|---|---|
| `hir.rs` (entry) | **39** | doc + `mod` + `pub use`; `#[cfg(test)] use` `Span`/`Type` cho `hir/tests.rs` (`use super::*`) |
| `hir/item.rs` | **261** | `Hir`, `HItem` + `HFn`/`FnKind`/`HParam`/`HSig`/`HField`/`HRec`/`HEnum`/`HVariant`/`HAlias`/`HConst`/`HUse`/`HTrait`/`HImpl` |
| `hir/stmt.rs` | **194** | `HStmt`, `HElse`, `AssignOp`, `HArm`, `HCond` |
| `hir/pat.rs` | **156** | `HPat` + `else_binds_err`/`span`/`sexp` |
| `hir/expr.rs` | **447** | `HExpr`, `InterpPart`, `tagged_items`, `HBody`, `BinOp`, `UnOp` |
| `hir/lit.rs` | **53** | `Lit` (+`Display`), `LitKind` |
| `hir/tests.rs` | **137** | **không đổi**, chạy nguyên chỗ |

### 1b. `crates/vis-diag/src/lib.rs` (1070) → entry + 5 submodule

| Tệp | Dòng | Trách nhiệm |
|---|---|---|
| `lib.rs` (entry) | **298** | doc + `mod` + `pub use` + `tests` (giữ nguyên) |
| `codes.rs` | **64** | `code()`, `Severity`, `Code`, `Lang` (+`parse`/`message`) |
| `books.rs` | **587** | `CODES`..`MODULE_CODES`, `ma_truong`, `BOOKS`, `BLOCKS_RESERVED`, `all_codes` |
| `tra_cuu.rs` | **37** | `lookup`, `lookup_str`, `next_block` |
| `explain.rs` | **55** | `ExplainHelp`, `explain_help`, `severity_label` |
| `render.rs` | **72** | `Diag`, `DiagLine`, `render` |

Mọi tệp ≤ **587** (trần 1.500, mục tiêu < 1.000). `mod` con là **private** + `pub use` tường minh
⇒ không phát sinh đường dẫn công khai mới; `vis_hir::hir::*` và `vis_diag::*` giữ nguyên.

## 2. Kiểm chứng "nguyên văn"

Cắt theo dải dòng gốc rồi `diff` từng khối với tệp gốc: **11/11 khối VERBATIM** (`item`, `stmt`,
`pat`, `expr`, `lit`; `codes`, `books`, `tra_cuu`, `explain`, `severity_label`, `render`).
Chỉ thêm header SPDX/doc + `use` cần thiết cho module mới, và 1 dòng doc `//!` ghi chú ST-318d ở
entry; **không** sửa dòng code/thông điệp nào.

## 3. Cổng

| Cổng | Kết quả | Log |
|---|---|---|
| `cargo fmt --all` | OK | — |
| `cargo test -p vis-hir -p vis-diag` | vis-hir **10/10**, vis-diag lib **11/11**, `tests/messages.rs` **12/12** | — |
| `./vis-dev gate --fast` | **PASS** — L0 `3901/3901` (4 skipped), 66 binary | `/home/ps/vislogs/st318d-gate-fast.log` (`c3471a1c…`) |
| `./vis-dev rules` | **OK** (R1..R9) | — |
| `cargo clippy --workspace --all-targets -- -D warnings` | **exit 0** | `/home/ps/vislogs/st318d-clippy.log` (`1b06c206…`) |
| R8 | **39/40**, `vis-cli/tests` **8/15** — không tạo tệp test cấp 1 | — |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** (100.000; 100,0 s) | `/home/ps/vislogs/st318d-visdiff.log` (`d0765152…`) |

## 4. Commit

- `1d8fbcbd` — `[ST-318d] vis-hir: tach hir.rs 1088 dong ...`
- `7be2eabc` — `[ST-318d] vis-diag: tach lib.rs 1070 dong ...`
- commit journal này (thứ 3).

## 5. Việc còn lại

- Không. Chờ Agent 0/Gatekeeper nghiệm thu; nếu cần có thể chạy lại `./vis-dev check` độc lập.
