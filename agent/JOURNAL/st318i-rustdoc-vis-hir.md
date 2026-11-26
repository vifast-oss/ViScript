<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-318i — đóng nợ rustdoc `vis-hir` (4 intra-doc link)

- **Ngày**: 2026-11-27 (giờ thấp điểm) · **Vai**: Builder (Agent 2) · **Nhánh**: `plan/st318i` · **Nền**: `main` @ `bfe165e2`
- **Worktree**: `/home/ps/viswt/st318i` — không chạm repo chính `/home/ps/ViScript`.
- **Tính chất**: **chỉ doc comment/link**; 0 đổi code, hành vi, API, visibility. Không thêm `pub(crate)`.
- **Nợ nguồn**: audit `ST-318d` (`accf421e`) phát hiện `cargo doc -p vis-hir` exit 101 với 4 lỗi có sẵn từ nền.

## 1. Bốn link trước/sau

| Tệp:dòng | Trước | Sau | Lý do |
|---|---|---|---|
| `crates/vis-hir/src/stdlib.rs:4` | `[`registry`]` | `` `registry` `` | `registry` là module **private**; không có đường dẫn công khai để trỏ |
| `crates/vis-hir/src/stdlib.rs:5` | `[`ty`]` | `` `ty` `` | `ty` là module **private** |
| `crates/vis-hir/src/stdlib/ty.rs:64` | `[`BUILTIN_FUNCS`]` | `[`crate::stdlib::BUILTIN_FUNCS`]` | Item public thật qua re-export `pub use builtin::BUILTIN_FUNCS` |
| `crates/vis-hir/src/symbols.rs:32` | `[`BUILTIN_VALUE_NAMES`]` | `` `vis_types::lower::BUILTIN_VALUE_NAMES` `` | Không tồn tại trong `vis-hir`; item thật ở crate `vis-types` (không phải dependency của `vis-hir`) nên không thể link |

Hai lỗi đầu là `rustdoc::private-intra-doc-links`, hai lỗi sau là `rustdoc::broken-intra-doc-links`.
Không đổi visibility nào; `vis_hir::stdlib::{StdId, FUNCS, …}` và `vis_hir::symbols::*` giữ nguyên.

## 2. Cổng

| Cổng | Kết quả | Log / sha256 |
|---|---|---|
| `cargo fmt --all` | OK, không diff | — |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p vis-hir --no-deps` | **exit 0, 0 warning** | `/home/ps/vislogs/st318i-doc.log` `24477ec2…d8c8f` |
| `cargo test -p vis-hir` | **PASS** 10/10 unit + 0 doctest | `/home/ps/vislogs/st318i-test.log` `8d86f37a…7cf937` |
| `./vis-dev gate --fast` (lần 1) | **FAIL** — 3902/3902 nextest PASS, 2 doctest `vis-interp` lỗi **link** `ld: signal 7 [Bus error]` do `/tmp` tmpfs 86% | `/home/ps/vislogs/st318i-gate.log` `e04096fd…d4adc` |
| `./vis-dev gate --fast` (lần 2, `TMPDIR` trên đĩa) | **PASS** — L0 OK, `gate --fast: PASS` | `/home/ps/vislogs/st318i-gate2.log` `c9024365…a987` |

Lần 1 không phải lỗi sản phẩm: toàn bộ 3902 test nextest đã PASS, chỉ 2 doctest của crate khác
(`vis-interp`) không link được vì `/tmp` đầy bởi phiên khác. Lần 2 đặt
`TMPDIR=/home/ps/vistmp/st318i` (đĩa) ⇒ PASS trọn cổng.

## 3. Commit

- Commit sản phẩm + journal: `[ST-318i] fix rustdoc vis-hir (4 link)`.

## 4. Việc còn lại

- Không. `cargo doc -p vis-hir` nay xanh; chờ Agent 0/Gatekeeper nghiệm thu.
