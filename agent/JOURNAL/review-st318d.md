<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Review ST-318d — Gatekeeper (Agent 1) độc lập

**Worktree**: `/home/ps/viswt/st318d` · nhánh `plan/st318d` · HEAD `3f800ca1` · nền `main` `2b5ccb51`
(main hiện `2c7dbef6`). Không sửa mã, không push.
**Kết luận**: **PASS** — 0 finding P1; **1 P2** (6 rustdoc intra-doc link mới hỏng ở `vis-diag`,
không chặn cổng hiện hành); **1 P3** (journal ghi "0 đổi thứ tự" quá rộng).

## 0. Phạm vi

- `git status --short`: **sạch** trước review (`3f800ca1`); sau review chỉ thêm tệp này.
- `git log --oneline 2b5ccb51..HEAD`: `1d8fbcbd`, `7be2eabc`, `3f800ca1` — đúng 3 commit.
- `git diff --stat 2b5ccb51..HEAD`: **13 tệp, +2029/−1861**.
- Đúng phạm vi: `crates/vis-hir/src/hir.rs` + `hir/{item,stmt,pat,expr,lit}.rs`;
  `crates/vis-diag/src/lib.rs` + `{codes,books,tra_cuu,explain,render}.rs`;
  `agent/JOURNAL/st318d-tach-hir-diag.md`. **Không** chạm crate khác, không chạm
  `vis-hir/src/lib.rs`, không chạm `hir/tests.rs`.
- `git diff --check 2b5ccb51..HEAD`: exit 0.

## 1. Move nguyên văn — PASS

### 1a. `vis-hir/src/hir.rs` 1088 → 39 + 5 module

| Tệp | Dòng | Bằng chứng |
|---|---:|---|
| `hir.rs` | 39 | doc + `mod` private + `pub use` + `#[cfg(test)] use` |
| `hir/item.rs` | 261 | body là **dải liên tục** của bản gốc |
| `hir/stmt.rs` | 194 | idem |
| `hir/pat.rs` | 156 | idem |
| `hir/expr.rs` | 447 | **byte-identical** với `2b5ccb51:hir.rs` dòng 604–1038; cùng sha256 `8981d0f9…` |
| `hir/lit.rs` | 53 | idem |

- Script so khớp có thứ tự (bỏ blank, `//!`, `use`, `mod`, `#[cfg(test)]`): bản cũ **1019** dòng
  "significant", bản mới **1023**; **0 dòng chỉ-có-ở-cũ**, chỉ **4 dòng chèn** (1 comment giải thích
  + 3 dòng nối của `pub use item::{…}`).
- Năm module mới lần lượt là các dải liên tục `old[14..244]`, `[245..419]`, `[420..565]`,
  `[566..989]`, `[990..1032]` (chỉ số dòng significant) — không mất/di chuyển lộn xộn dòng nào.
- `hir/tests.rs` **không đổi**: sha256 `3a10c19c…` giống hệt `2b5ccb51`; vẫn chạy (10 test
  `vis-hir`, gồm 3 test HIR).

### 1b. `vis-diag/src/lib.rs` 1070 → 298 + 5 module

| Tệp | Dòng | Bằng chứng |
|---|---:|---|
| `lib.rs` | 298 | doc + `mod` private + `pub use` + nguyên khối `tests` cũ |
| `codes.rs` | 64 | dải liên tục cũ (significant `[5..59]`) |
| `books.rs` | **587** | body dòng 7–587 **byte-identical** với `lib.rs` cũ dòng 68–648; sha256 `a7fe4c45…` — **không cắt dòng** |
| `tra_cuu.rs` | 37 | dải liên tục cũ (significant `[666..694]`) |
| `explain.rs` | 55 | ghép 2 dải liên tục: `[631..665]` + `[727..737]` |
| `render.rs` | 72 | ghép 2 dải liên tục: `[695..726]` + `[738..768]` |

- Multiset dòng "significant" cũ: **1024** dòng, tất cả còn đủ; bản mới 1027 dòng, 3 dòng thêm chỉ là
  phần nối của `pub use books::{…}`.
- **Toàn bộ 312 string literal** (thông điệp chẩn đoán, help vi/en/ja) **giống hệt** bản cũ —
  `Counter` hai bên không có phần tử lệch.
- Khối `#[cfg(test)] mod tests` của `lib.rs` **byte-identical** với bản cũ; sha256 `471324da…`.

### 1c. Chữ ký / item công khai

- `vis-hir`: grep `pub (fn|struct|enum|const|use|static|type)` cũ **46**, mới **51** = 46 cũ
  (multiset giống hệt, **0 old-only**) + 5 dòng `pub use` mới.
- `vis-diag`: cũ **27**, mới **32** = 27 cũ (multiset giống hệt, **0 old-only**) + 5 dòng `pub use`.
- Không mất/đổi tên/chữ ký item công khai nào.

## 2. API cũ còn dùng được — PASS

- `vis-hir/src/hir.rs` re-export đủ 28 tên: `Hir`, `HItem`, `HExpr`, `HStmt`, `HPat`, `Lit`,
  `BinOp`, `UnOp`, `AssignOp`, `HBody`, `InterpPart`, …; `vis_diag/src/lib.rs` re-export đủ 24 tên
  gốc (`CODES`…, `lookup`, `explain_help`, `render`, `Diag`, `Severity`, …).
- Kiểm **độc lập ngoài repo** (`/tmp/aud318d/api-check`): crate thử `use vis_hir::hir::{…}`,
  `use vis_hir::{…}`, `use vis_diag::{…}` gọi cả `Diag::new`, `DiagLine`, `render`, `lookup`,
  `next_block`… → `cargo +1.100.0 check` **exit 0**.
- `cargo test -p vis-hir -p vis-diag`: **exit 0** — `vis-diag` lib **11/11**,
  `vis-diag/tests/messages.rs` **12/12**, `vis-hir` **10/10** (3 HIR), doc-test 0.
- Không đổi mã/thông điệp chẩn đoán (mục 1b).

## 3. Cổng độc lập — PASS

| Cổng | Kết quả | Log / sha256 |
|---|---|---|
| `cargo fmt --all --check` | exit **0** | `/home/ps/vislogs/review-st318d-fmt.log` |
| `cargo test -p vis-hir -p vis-diag` | exit **0** — 11 + 12 + 10 pass | `review-st318d-test.log` `48bfd72e…` |
| `./vis-dev gate --fast` | **PASS** — **3901/3901**, 4 skipped | `review-st318d-gate.log` `319f9746…` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **lech=0** (100 000; 141,7 s) | `review-st318d-diff.log` `9867468f…` |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit **0** | `review-st318d-clippy.log` `8ab828a9…` |
| `./bin/vis-dag-check` | **OK** — 23 crate / 5 tầng, 0 cạnh cấm | `review-st318d-dag.log` `5ee0d7f4…` |
| R8 | **39/40** tệp test cấp 1; `vis-cli/tests` **8/15** | — |
| Kích thước tệp | mọi tệp ST-318d ≤ **587**; toàn workspace **0 tệp >1500** | — |

- Ghi chú R8/kích thước: 17 tệp **có sẵn** trong 1000–1500 dòng (max `types_it/stdlib_ty.rs` 1275)
  không thuộc ST-318d; các tệp mới đều <1000.
- Môi trường: `cargo` mặc định `/usr/bin` là 1.93.1 (không đủ MSRV 1.100). Mọi cổng trên chạy bằng
  `$HOME/.cargo/bin` (rustup 1.100.0, khớp `rust-toolchain.toml`); `vis-dev` tự chọn đúng bản.
  Đây là khác biệt môi trường, không phải lỗi ST-318d.

## 4. Hợp nhất với `main`

- `git merge-tree --write-tree main HEAD`: **exit 0**, tree `ca5c5b13…`, **không conflict**.
- `main` (`2c7dbef6`) chỉ thêm tài liệu `agent/*` (ACTIVE/DECISIONS/QUEUE/ROADMAP/STATE/JOURNAL);
  không overlap mã với nhánh. Không có conflict `STATE.md`/journal như dự kiến.

## 5. Finding

### P2 — 6 rustdoc intra-doc link mới hỏng ở `vis-diag` (không chặn cổng)

- Tái lập:
  `RUSTDOCFLAGS="-D warnings" cargo +1.100.0 doc -p vis-diag --no-deps` → **exit 101**, 6 lỗi:
  - `books.rs:569`, `books.rs:581`: `next_block` (giờ ở `tra_cuu.rs`);
  - `render.rs:16`: `lookup`, `CODES`;
  - `tra_cuu.rs:11`: `CODES`, `SYNTAX_CODES`.
- Đối chứng nền `2b5ccb51` (giải nén ra `/tmp/aud318d/base`): cùng lệnh **exit 0**, **0 warning**.
  Bản hiện tại chạy **không** `-D warnings` vẫn exit 0 nhưng phát 6 warning.
- Ảnh hưởng: chỉ tài liệu/rustdoc; build, test, clippy, gate, runtime **không** bị ảnh hưởng.
  Cách sửa gợi ý (không thực hiện): dùng link đầy đủ `[`super::tra_cuu::next_block`]` /
  `[`super::books::CODES`]` hoặc import có chủ đích cho rustdoc.
- `vis-hir` chỉ có 4 warning **có sẵn** ở `stdlib.rs`/`stdlib/ty.rs`/`symbols.rs`, **không** phát sinh
  từ `hir/*` (đã đối chiếu log).
- Log: `review-st318d-doc.log` `6ae876c2…`; nền `review-st318d-doc-base.log` `8e76f2bc…`.

### P3 — câu "0 đổi thứ tự" trong journal quá rộng

- Trong `vis-diag`, thứ tự **top-level** đổi do gom nhóm: `severity_label` rời khỏi giữa
  `DiagLine`/`render` vào `explain.rs`; `tra_cuu` đứng trước `explain`; `render` đứng trước
  `severity_label`.
- Từng khối vẫn **verbatim** và không có static/khởi tạo phụ thuộc thứ tự ⇒ **không** đổi hành vi.
  Nên sửa câu journal thành "0 đổi thứ tự **trong từng khối**; nhóm `vis-diag` xếp lại".
- Không phải lỗi mã; không cần chặn merge.

## 6. Việc đã kiểm / chưa kiểm

- Đã kiểm: phạm vi, move từng khối, chữ ký public, API cũ, test, fmt, gate, diff 100k, clippy,
  dag, R8, kích thước, merge-tree, rustdoc.
- Chưa kiểm: soak 4h/24h, 3-OS (theo `VD-289` chế độ Phát triển Kiến trúc — không bắt buộc).
- Không sửa mã, không push, không `cargo clean`; đĩa còn **9,2 GB** (>6 GB).

## 7. Addendum F1 — vá P2 rustdoc (`2ae1eb2e`)

**Kết luận addendum: PASS** — P2 rustdoc đã đóng; P3 câu chữ journal vẫn chỉ là ghi chú.

- `git log --oneline accf421e..2ae1eb2e`: đúng **1** commit `[ST-318d] fix rustdoc links sau tach module`.
- `git diff --stat accf421e..2ae1eb2e`: **4 tệp, +29/−5** = 4 dòng `///` (books.rs **2**,
  render.rs **1**, tra_cuu.rs **1**) + journal (`+25/−1`). **0 dòng code.**
- Kiểm 4 dòng doc: tất cả là `///`; bỏ phần target `(crate::…)` khỏi link thì **4/4 dòng chữ hiển thị
  giống hệt** bản trước; chỉ thêm đích rustdoc, không đổi thông điệp/tên link.
- `RUSTDOCFLAGS="-D warnings" cargo doc -p vis-diag --no-deps`: **exit 0, 0 warning**
  (`/home/ps/vislogs/review-st318d-rustdoc.log`, sha256 `b55d8a6b…`).
- `cargo test -p vis-diag -p vis-hir`: **exit 0** — **11 + 12 + 10** pass
  (`review-st318d-test-f1.log`, `660d3b80…`).
- `./vis-dev gate --fast`: **PASS — 3901/3901**, 4 skipped (`review-st318d-gate-f1.log`,
  `fbd1b951…`).
- `git merge-tree --write-tree main HEAD` (main `baa2dbd3`, VD-297): **exit 0**, tree
  `daa896e5…`, **0 conflict**. Nhánh không chạm `agent/STATE.md` nên dự kiến conflict
  `STATE.md` **không xảy ra**; `main` chỉ thêm docs/STATE + `tests/misc_api_it/host_parity/net_http.rs`,
  không overlap `vis-diag`/`vis-hir`.
- `cargo doc -p vis-hir` vẫn **4 lỗi có sẵn từ nền** (không phải của 318d):
  `stdlib.rs:4` (`registry` private), `stdlib.rs:5` (`ty` private), `stdlib/ty.rs:64`
  (`BUILTIN_FUNCS`), `symbols.rs:32` (`BUILTIN_VALUE_NAMES`). Nền `2b5ccb51` cho **cùng 4 lỗi**
  (`review-st318d-rustdoc-hir-base.log`, `1c1b3bc2…`); các tệp này **không** bị ST-318d chạm.
  Không tính FAIL cho 318d — đề xuất mở **nợ riêng** (task doc-link `vis-hir`) nếu muốn
  `-D warnings` xanh toàn workspace.
- Log addendum: `review-st318d-rustdoc.log` `b55d8a6b…`, `review-st318d-test-f1.log` `660d3b80…`,
  `review-st318d-gate-f1.log` `fbd1b951…`, `review-st318d-rustdoc-hir.log` `6b5b686f…`.
- Không sửa mã, không push, không `cargo clean`; đĩa còn **12 GB** (>6 GB).
