<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# REVIEW ST-318i — Gatekeeper (Agent 1) độc lập

- **Ngày**: 2026-11-27 (giờ thấp điểm UTC) · **Vai**: Gatekeeper (Agent 1), **không sửa mã**
- **Worktree**: `/home/ps/viswt/st318i` · **Nhánh**: `plan/st318i` · **HEAD**: `5a59a6e4`
- **Nền**: `main` `bfe165e2` · **main lúc audit**: `9045dd78` (`[REVIEW] merge ST-318e`)
- **Kết luận**: **PASS** — 0 P1; 1 ghi nhận P2 không chặn (§6)

## 1. Phạm vi

| Kiểm | Kết quả |
|---|---|
| `git status --porcelain` trước audit | **sạch** (sau đó chỉ `touch` mtime 3 tệp nguồn để ép rustdoc chạy lại; nội dung không đổi, cây vẫn sạch) |
| `git log --oneline bfe165e2..HEAD` | **1 commit**: `5a59a6e4 [ST-318i] fix rustdoc vis-hir (4 link)` |
| `git diff --stat bfe165e2..HEAD` | **5 tệp, +51 / −4** (3 tệp `.rs` +5/−4; journal +41; `STATE.md` +5) |
| Diff `crates/` | **3 tệp, +5/−4**; mọi dòng `+`/`−` đều là `///` hoặc `//!` |
| `pub`/`fn`/`struct`/`enum`/`impl` trong diff | **0** (không đổi code, visibility, API) |
| `git diff --check` | exit **0** |
| Ngoài phạm vi | **0** — chỉ `crates/vis-hir/src/{stdlib.rs,stdlib/ty.rs,symbols.rs}` + `agent/JOURNAL/st318i-rustdoc-vis-hir.md` + `agent/STATE.md` |

## 2. Bốn intra-doc link — trước/sau và kiểm chứng

| # | Tệp:dòng | Trước | Sau | Kiểm chứng độc lập |
|---|---|---|---|---|
| 1 | `stdlib.rs:4` | `[`registry`]` | `` `registry` `` | `stdlib.rs:11` là `mod registry;` **private**; không có `pub use` nào lộ đường dẫn ⇒ `private-intra-doc-links` là đúng |
| 2 | `stdlib.rs:5` | `[`ty`]` | `` `ty` `` | `stdlib.rs:13` là `mod ty;` **private**; `Ty` chỉ re-export qua `pub use ty::{…}` ⇒ đúng |
| 3 | `stdlib/ty.rs:64` | `[`BUILTIN_FUNCS`]` | `[`crate::stdlib::BUILTIN_FUNCS`]` | `stdlib.rs:17` có `pub use builtin::BUILTIN_FUNCS;`, `builtin.rs:7` `pub const` ⇒ link hợp lệ; `ty.rs` **không** import tên trần nên bản cũ hỏng thật |
| 4 | `symbols.rs:32` | `[`BUILTIN_VALUE_NAMES`]` | `` `vis_types::lower::BUILTIN_VALUE_NAMES` `` | Trong `vis-hir` **không** có item này (grep 0); item thật ở `crates/vis-types/src/lower.rs:58`, mà `vis-hir/Cargo.toml:3` ghi rõ **không** kéo `vis-types` ⇒ không thể link, code span là lựa chọn đúng |

Nền lỗi khớp biên bản Gatekeeper `ST-318d` (`agent/JOURNAL/review-st318d.md:155-160`): đúng 4 dòng này,
và nền `2b5ccb51` cho cùng 4 lỗi. Fix **không** đổi visibility (không thêm `pub(crate)`).

Hai dòng `stdlib.rs:4,5` mất hyperlink nhưng chữ hiển thị giữ nguyên; đây là hệ quả bắt buộc vì
module private không có đích công khai.

## 3. Cổng máy

Lưu ý tái lập: `cargo` trên `PATH` hệ thống là **1.93.1** và bị `rust-version = 1.100.0` từ chối
(`error: rustc 1.93.1 is not supported`). Repo ghim `rust-toolchain.toml` = **1.100.0**; phải đặt
`PATH=/home/ps/.cargo/bin:$PATH` (rustup 1.100.0) như `bin/_vis-toolchain.sh` yêu cầu.

| Cổng | Kết quả | Log · sha256 |
|---|---|---|
| `RUSTDOCFLAGS="-D warnings" cargo doc -p vis-hir --no-deps` (lần đầu, cache) | **exit 0**, 0 warning | `review-st318i-doc.log` |
| Cùng lệnh sau `touch` 3 tệp nguồn (rustdoc chạy lại thật, `Documenting vis-hir v0.6.0`) | **exit 0**, **0 warning/error** | `/home/ps/vislogs/review-st318i-doc2.log` · `c478c7bd…f766c61cc9` |
| `cargo test -p vis-hir` | **exit 0** — **10/10** unit, 0 doctest | `review-st318i-test.log` · `40a3f8df…0e7c70e2` |
| `./vis-dev gate --fast` (`TMPDIR=/home/ps/vislogs/tmp`) | **PASS** — fmt OK, clippy OK, R1..R10 OK, toolchain PASS, release PASS, fixtures PASS, **3902/3902** nextest (`Summary 3902 tests run: 3902 passed, 4 skipped`), **L0 OK**, `gate --fast: PASS` | `/home/ps/vislogs/review-st318i-gate.log` · `3131e455…58c74c13f` |
| R8 trần test binary | `crates/vis-cli/tests` = **8** ≤ 15; tổng `.rs` cấp 1 (`crates/*/tests` + `vis-host-linux/tests`) = **39** ≤ 40 | §1 `ls | wc -l` |

Đối chiếu log Builder: `st318i-gate2.log` cũng `3902/3902` + `PASS`; `st318i-doc.log` exit 0;
`/tmp` hiện **86%** (tmpfs 7,2 G, còn 1,1 G) — xác nhận giải thích "lần 1 Bus error khi link
doctest `vis-interp` vì `/tmp` đầy" là hợp lý, không phải lỗi sản phẩm.

## 4. Merge-tree

`git merge-tree --write-tree main HEAD` (main `9045dd78`) → tree `4e193fc3…`, exit **1**,
**đúng 1 xung đột**: `agent/STATE.md` (3 stage `3803e255` / `87bb3814` / `3ea21111`).
Không xung đột nào ở `crates/` hay `agent/JOURNAL/`. Khớp dự kiến; phần `STATE.md` cần resolve tay
khi merge (thêm bullet `ST-318i`, không đụng bullet khác).

## 5. Chữ hiển thị sau khi render (kiểm HTML thật)

| Vị trí | HTML render | Kết luận |
|---|---|---|
| `stdlib/index.html` | `metadata module sống trong <code>registry</code>; …` — **không** còn `<a>` tới `registry`/`ty` | chữ nguyên, chỉ mất link chết |
| `stdlib/enum.Ty.html` | `<a href="constant.BUILTIN_FUNCS.html" title="constant vis_hir::stdlib::BUILTIN_FUNCS"><code>crate::stdlib::BUILTIN_FUNCS</code></a>` | link **sống**, đích đúng |
| `symbols/enum.SymKind.html` | `Tên giá trị dựng sẵn (<code>vis_types::lower::BUILTIN_VALUE_NAMES</code>).` | không link, chữ đúng item thật |

Không có link mới nào hỏng: `-D warnings` xanh trên lượt rustdoc chạy lại thật (đã `touch` nguồn).
Không đổi nghĩa: bản `ty.rs` chỉ nới dòng (thêm 1 dòng do wrap), nội dung câu giữ nguyên.

## 6. Ghi nhận không chặn (P2)

- **P2-1 (thẩm mỹ)**: `ty.rs:64` dùng dạng `[`crate::stdlib::BUILTIN_FUNCS`]` nên **chữ hiển thị**
  đổi từ `BUILTIN_FUNCS` thành `crate::stdlib::BUILTIN_FUNCS` (dài hơn). Nếu muốn giữ chữ ngắn thì
  dùng `[`BUILTIN_FUNCS`](crate::stdlib::BUILTIN_FUNCS)`. Đúng chỉ dẫn task, nghĩa không đổi ⇒ không chặn.
- **P2-2 (tái lập)**: máy nào để `cargo` hệ thống (1.93.1) trước `~/.cargo/bin` sẽ thấy `cargo doc`
  fail vì `rust-version`, **không** phải do nhánh này. Nên ghi vào checklist Gatekeeper.

## 7. Ràng buộc

`jobs=1` (không chạy cargo song song) · **không** `cargo clean` · đĩa `/home/ps` còn **17 GB** (> 6 GB)
· **không push** · **không sửa mã** (chỉ `touch` mtime 3 tệp nguồn, nội dung byte-identical, `git status` sạch).
