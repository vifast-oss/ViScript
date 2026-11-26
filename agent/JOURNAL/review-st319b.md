<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# REVIEW ST-319b — Gatekeeper (Agent 1) độc lập

- **Ngày**: 2026-11-27 (giờ thấp điểm UTC) · **Vai**: Gatekeeper (Agent 1), **không sửa mã**
- **Worktree**: `/home/ps/viswt/st319b` · **Nhánh**: `plan/st319b` · **HEAD**: `c6fa22c7`
- **Nền**: `main` `d97773de` · **main lúc audit**: `151f2029` (`origin/main`) và `9d3bb30f` (local, `[ST-316c]`)
- **Kết luận**: **PASS** — 0 P1; 3 ghi nhận P2 không chặn (§6)

## 1. Phạm vi

| Kiểm | Kết quả |
|---|---|
| `git status --porcelain` trước audit | **sạch** (sau đó chỉ `touch` mtime `frame.rs` để ép rustdoc chạy lại; nội dung byte-identical — `git hash-object` = `git rev-parse HEAD:…` = `a15946cd…`) |
| `git log --oneline d97773de..HEAD` | **1 commit**: `c6fa22c7 [ST-319b] fix rustdoc vis-interp ([Interp])` |
| `git diff --stat d97773de..HEAD` | **1 tệp, +1 / −1** — chỉ `crates/vis-interp/src/frame.rs` |
| Dòng thay đổi | đúng **1 dòng**, cả hai vế đều là `//!` (dòng 5): `[`Interp`]` → `` `Interp` `` |
| `pub`/`fn`/`struct`/`enum`/`impl`/`use` trong diff | **0** — không đổi code, visibility, API |
| `git diff --check` | exit **0** |
| Journal/STATE của Builder trên nhánh | **không có** (nhánh chỉ 1 tệp); chấp nhận theo brief "nếu có" |
| Ngoài phạm vi | **0** |

Cây làm việc sau toàn bộ audit: `git status --porcelain` rỗng; blob `frame.rs` trên đĩa trùng blob trong commit.

## 2. Căn cứ lỗi (kiểm chứng độc lập)

| # | Tệp:dòng | Trước | Sau | Kiểm chứng |
|---|---|---|---|---|
| 1 | `frame.rs:5` | `[`Interp`]` | `` `Interp` `` | `lib.rs:68` khai `pub mod frame` (module **công khai**); `interp.rs:39` khai `pub(crate) struct Interp<'a>` (**không** công khai), không có `pub use Interp` nào ở `lib.rs` (chỉ dùng nội bộ `interp::Interp`) ⇒ link từ tài liệu công khai tới item `pub(crate)` là `private_intra_doc_links`, bị `-D warnings` biến thành lỗi. Bản `frame.rs:26` có `use crate::interp::{…, Interp, …}` chỉ là import private, không mở đường link công khai. |

Chữ hiển thị **không đổi**: cả hai dạng đều render chữ `Interp` (bản cũ có thêm `<a>` chết). Dòng 10 đã dùng code span `` `Interp::run_program` `` từ trước — cùng lý do, nay nhất quán.

## 3. Cổng máy

Lưu ý tái lập (giống ST-318i): `cargo` trên `PATH` hệ thống là **1.93.1**, bị `rust-version = 1.100.0`
từ chối (`error: rustc 1.93.1 is not supported`). Repo ghim `rust-toolchain.toml` = **1.100.0**; phải đặt
`PATH=/home/ps/.cargo/bin:$PATH` (rustup 1.100.0) như `bin/_vis-toolchain.sh` yêu cầu.
Lượt chạy đầu với cargo hệ thống đã **fail 101** đúng vì lý do này (không phải do nhánh).

| Cổng | Kết quả | Log · sha256 |
|---|---|---|
| `RUSTDOCFLAGS="-D warnings" cargo doc -p vis-interp --no-deps` (1.100.0, lần đầu) | **exit 0** nhưng **cache** (`Finished … 0.07s`) — không đủ tin cậy | `review-st319b-doc.log` (bị ghi đè ở lượt sau) |
| Cùng lệnh sau `touch crates/vis-interp/src/frame.rs` → rustdoc chạy lại thật (`Documenting vis-interp v0.6.0`, 0.40s) | **exit 0**, **0 warning/error** | `/home/ps/vislogs/review-st319b-doc.log` · `490dea1e…5927c2c6f` |
| `cargo test -p vis-interp` | **exit 0** — **5/5** unit + **55/55** tích hợp + **2/2** doc-test, 0 failed | `st319b-test.log` (Builder) · `d83fbb31…fdd96f87` |
| `./vis-dev gate --fast` (`TMPDIR=/var/tmp`, đĩa) | **PASS** — fmt OK, clippy OK, R1..R10 OK, DAG OK, toolchain OK, release OK, doc-examples OK (31 khối: 26 xanh + 5 ngoại lệ đỏ đúng), readme-examples OK, **3912/3912** nextest (4 skipped), **L0 OK**, `gate --fast: PASS` | `/home/ps/vislogs/review-st319b-gate.log` · `57c0ce51…6474afc` |
| R8 trần test binary | `crates/vis-cli/tests` = **8** ≤ 15; tổng `.rs` cấp 1 (`crates/*/tests` + `vis-host-linux/tests`) = **39** ≤ 40 | §1 |

## 4. Merge-tree

| Lệnh | Tree | Exit | Xung đột |
|---|---|---|---|
| `git merge-tree --write-tree main HEAD` (main `9d3bb30f`) | `f6314128…` | **0** | **0** |
| `git merge-tree --write-tree 151f2029 HEAD` | `987dbf55…` | **0** | **0** |

Khác dự kiến "chỉ `STATE.md`": nhánh **không** chạm `STATE.md` nên **không có xung đột nào**.
Giao hai tập tệp thay đổi kể từ `d97773de` là **rỗng** (`comm -12` không dòng nào):

- nhánh: `crates/vis-interp/src/frame.rs`;
- main: `agent/{ACTIVE,DECISIONS,DECISIONS-INDEX,QUEUE,ROADMAP,STATE}.md`, `agent/JOURNAL/{quyet-dinh-a0-2026-11-27-vd300,st316c-go-no-go-draft}.md`, `bin/vis-targets`.

Kiểm cây hợp nhất: `git diff --stat main f6314128…` = **đúng 1 tệp** `crates/vis-interp/src/frame.rs`
(+1/−1); `git diff --name-only HEAD f6314128…` = đúng 9 tệp phía main. Tức merge chỉ áp thêm 1 dòng doc.

## 5. Ghi nhận không chặn (P2)

- **P2-1 (vận hành)**: `cargo`/`rustc` mặc định trên `PATH` (1.93.1) **không** dựng được workspace
  (`rust-version = 1.100.0`). Mọi cổng phải chạy qua `PATH=/home/ps/.cargo/bin:$PATH`; nên đưa vào
  checklist Gatekeeper để tránh kết luận FAIL oan.
- **P2-2 (tái lập)**: lượt `cargo doc` đầu tiên trả **exit 0 trong 0.07s** (fingerprint cache của Builder).
  Chỉ sau `touch` nguồn mới chứng minh được rustdoc thật sự xanh; audit tương lai nên luôn ép chạy lại.
- **P2-3 (biên bản)**: nhánh không kèm journal Builder, nên **không có** bản ghi lỗi gốc `private_intra_doc_links`
  trong repo; căn cứ lỗi ở §2 được dựng lại độc lập từ mã (`pub mod frame` + `pub(crate) struct Interp`),
  không dựa vào lời kể. Đề nghị Builder ghi journal cho các lát rustdoc sau.

## 6. Ràng buộc

`jobs=1` (`CARGO_BUILD_JOBS=1`, `cargo-nextest` do `vis-dev` điều phối) · **không** `cargo clean` ·
đĩa `/home/ps` còn **16 GB** (> 6 GB, ngưỡng dừng) · `TMPDIR=/var/tmp` (đĩa, không dùng tmpfs `/tmp` 7,2 G) ·
**không push** · **không sửa mã** (chỉ `touch` mtime `frame.rs`; blob trên đĩa = blob trong `HEAD`).
