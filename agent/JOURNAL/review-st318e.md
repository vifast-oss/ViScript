<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# REVIEW ST-318e — Gatekeeper (Agent 1) độc lập

- **Ngày**: 2026-11-27 · **Vai**: Gatekeeper · **Chế độ**: chỉ đọc/kiểm chứng, **không sửa mã**.
- **Nhánh**: `plan/st318e` @ `14e19e83` · **Nền**: `main` @ `baa2dbd3` (`origin/main` cùng mốc) ·
  **`main` hiện tại (local)**: `bc8fe68c` (chứa `c02050c0` + ST-316c).
- **Worktree**: `/home/ps/viswt/st318e` (không chạm repo chính, không push).
- **Kết luận**: **PASS** — move nguyên văn (0 dòng logic đổi), API công khai/VBC1 không đổi,
  toàn bộ cổng độc lập xanh, không P1. Ghi nhận 3 P2 (nới `pub(super)`, rustfmt reflow chữ ký,
  glob `super::*`) và 3 P3 hành chính.

## 1. Vệ sinh & phạm vi

| Kiểm | Kết quả |
|---|---|
| `git status --porcelain` | **rỗng** (sạch) |
| `git log --oneline baa2dbd3..HEAD` | **4 commit** `[ST-318e]` (`22e81371`, `70f8ab29`, `832bbfee`, `14e19e83`) |
| `git diff --stat baa2dbd3..HEAD` (tại `14e19e83`) | **8 tệp, +1.571 / −1.489** |
| Phạm vi | `crates/vis-bytecode/src/codegen{,.rs,/expr.rs}` + `crates/vis-cli/src/vistest{,.rs,/{run,ca,tien_trinh,tests}.rs}` + `agent/JOURNAL/st318e-tach-r1-mem.md` |
| Ngoài phạm vi | **0** — không crate khác, không `Cargo.toml`/`Cargo.lock`, không `main.rs`/`lib.rs`, không SPEC/QUEUE/DECISIONS/ACTIVE, không `API-*.txt` |
| Commit `[ST-00]` lẫn trong nhánh | **0** (cả 4 commit đều `[ST-318e]`) |

`git diff --check` sạch (0 lỗi whitespace). 4 tệp mới đều có SPDX dòng 1.

## 2. Move thật (không đổi logic/thứ tự emit/span)

### 2.1 `codegen.rs` → `codegen/expr.rs`

- Khối `impl<'g> Emit<'g> { … }` cũ: `baa2dbd3:crates/vis-bytecode/src/codegen.rs` dòng
  **438–1060** = **623 dòng** (gồm `impl`, chú thích, `fn expr` 618 dòng, `}`).
- Tệp mới `codegen/expr.rs` dòng **7–628** = **623 dòng**; 6 dòng đầu là SPDX + 2 doc + `use super::*`.
- Sau khi thay đúng **một** token hiển thị `fn expr` → `pub(super) fn expr` (bắt buộc để sibling
  gọi được) và bỏ **một** dòng trống cuối khối, **SHA-256 hai khối trùng tuyệt đối**:
  `118d81071890af2b131b249ebb81945f89d95281a73a2c2797093256c72cae32`.
- Toàn tệp: `codegen.rs` chỉ **+1** `mod expr;` và **−623** dòng khối cũ (numstat `1/623`);
  `expr.rs` **+628**. Không đổi một dòng nào khác trong `codegen.rs`.
- Hệ quả: thân `expr` (618 dòng) **byte-identical** ⇒ thứ tự `push`/`charge`/`net_stack`, mọi
  `span`, mọi nhánh `match` giữ nguyên; 7 entry `pub fn compile*`, `Gen`, `Emit`, `bin_tag`,
  `un_tag`, `pub use crate::cerr::CodegenError` vẫn ở `codegen.rs`.

### 2.2 `vistest.rs` → `vistest/{run,ca,tien_trinh}.rs`

Đối chiếu **từng item** (khớp tên + so khối sau khi bỏ `pub(super)`/gộp whitespace):
**28 item cũ → 0 item mất**. Các item chuyển tệp:

| Tệp mới | Item | Dòng cũ | Dòng mới | Kết quả |
|---|---|---|---:|---:|---|
| `run.rs` | `usage` | 64–76 | 10–22 | **byte-identical** |
| `run.rs` | `pub fn run` | 613–1055 | 24–465 | **byte-identical** (442 dòng thân) |
| `ca.rs` | `Kq` + `impl Kq` | 349–373 | 12–36 | nguyên văn, chỉ `pub(super)` field/method cần thiết |
| `ca.rs` | `chay_mot_lan` | 382–474 | 45–137 | **byte-identical** |
| `ca.rs` | `CauHinhChay` | 478–486 | 141–149 | nguyên văn + `pub(super)` field |
| `ca.rs` | `chay_tuan_tu` | 490–523 | 153–189 | **thân byte-identical**; chữ ký reflow |
| `ca.rs` | `chay_song_song` | 531–589 | 197–255 | **thân byte-identical**; chữ ký reflow |
| `tien_trinh.rs` | `path_voi_muc_dau` | 189–197 | 14–22 | nguyên văn + `pub(super)` |
| `tien_trinh.rs` | `dat_path_tro_giup` | 200–207 | 25–32 | nguyên văn + `pub(super)` |
| `tien_trinh.rs` | `chay_theo_han` | 211–224 | 36–52 | **thân byte-identical**; chữ ký reflow |
| `tien_trinh.rs` | `chay` | 228–245 | 56–73 | nguyên văn + `pub(super)` |
| `tien_trinh.rs` | `chan_doan` | 249–277 | 77–105 | nguyên văn + `pub(super)` |
| `tien_trinh.rs` | `HAN_GOM_OUTPUT` | 281 | 109 | **byte-identical** |
| `tien_trinh.rs` | `doc_nen` | 285–296 | 113–124 | **byte-identical** |
| `tien_trinh.rs` | `chay_co_han` | 303–346 | 131–174 | nguyên văn + `pub(super)` |

Kiểm dòng toàn cục (multiset, bỏ whitespace/`pub(super)`): **chỉ 3 dòng cũ không còn** —
`use std::process::{Command, ExitCode};` (đã chuyển sang tệp con) và **2 dòng chữ ký** bị rustfmt
gập dòng (`chay_theo_han`, `chay_tuan_tu`); **0 dòng logic mất**. 31 dòng "mới" còn lại đều là
header `//!`/`SPDX`, `use`, `mod`, `pub use run::run;`, và các dòng chữ ký reflow — không có logic mới.

**Thứ tự**: trong mỗi tệp mới, thứ tự item giữ đúng thứ tự cũ (`usage` trước `run`; `Kq` →
`chay_mot_lan` → `CauHinhChay` → `chay_tuan_tu` → `chay_song_song`; `path_voi_muc_dau` → … →
`chay_co_han`). Không đổi thứ tự emit/span.

**Test**: hai tệp gốc (`codegen.rs`, `vistest.rs`) có **0** `#[test]`; `tests.rs` giữ nguyên **6**
`#[test]` (diff chỉ +3 dòng `use`), `mod tests;` vẫn ở `vistest.rs:201`. Không mất ca test nào.

## 3. LOC & API

### 3.1 LOC (`wc -l` tại HEAD)

| Tệp | Trước | Sau |
|---|---:|---:|
| `crates/vis-bytecode/src/codegen.rs` | 1.069 | **447** |
| `crates/vis-bytecode/src/codegen/expr.rs` (mới) | — | **628** |
| `crates/vis-cli/src/vistest.rs` | 1.061 | **201** |
| `crates/vis-cli/src/vistest/run.rs` (mới) | — | **465** |
| `crates/vis-cli/src/vistest/ca.rs` (mới) | — | **255** |
| `crates/vis-cli/src/vistest/tien_trinh.rs` (mới) | — | **174** |

Mọi tệp trong phạm vi **< 700** (mục tiêu R1 mềm); trần cứng 1.500: OK. Chênh lệch chỉ là
header module/`use` và 1 dòng `mod expr;`.

### 3.2 API công khai — **không đổi**

- `vis-bytecode`: **7 → 7** `pub fn compile*`; `pub use crate::cerr::CodegenError` giữ nguyên;
  `pub fn bin_tag`/`un_tag` vẫn ở `codegen.rs`; không thêm `pub` nào.
- `vis-cli`: `pub fn run` chuyển sang `run.rs` nhưng `vistest.rs:67` vẫn `pub use run::run;`;
  `main.rs:201` `vistest::run(&args[1..])` **0 diff**; `vis test` giữ nguyên entry/usage.
- Thay đổi hiển thị **nội bộ** (không phải API): `expr` + `Kq`/field + `CauHinhChay`/field +
  7 helper `tien_trinh`/`ca` từ private → `pub(super)` (bắt buộc khi sang module con).
- `agent/API-VBC1.txt`, `API-CODES.txt`, `API-STD.txt`: diff **rỗng**, SHA-256 trước/sau trùng
  (`64282aee…`, `4f9a595a…`, `b1801a8c…`).

## 4. Hành vi & cổng độc lập (chạy lại từ đầu trên HEAD `14e19e83`)

| Cổng | Kết quả | Log + sha256 |
|---|---|---|
| `cargo test -j 2 -p vis-bytecode -p vis-cli` | **exit 0** — tổng **1.919 passed, 0 failed** (bộ builder nêu: bytecode `103+1`, cli `523+254+496+125+287+3`; cộng unit/doc phụ `3+4+16+103+1`) | `vislogs/review-st318e-test.log` · `66d69c37…ff761` |
| `cargo clippy -j 2 --workspace --all-targets -- -D warnings` | **exit 0** (42,11 s) | `vislogs/review-st318e-clippy.log` · `e4ab9dde…ae66b` |
| `./vis-dev gate --fast` | **PASS** — nextest **3902/3902** (4 skipped, 83,5 s), luật R1..R10 OK, fmt/clippy OK | `vislogs/review-st318e-gate.log` · `0ceff1cc…5b00dd` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** — tong=100000, chay=93.768, loi_tinh=6.232, 124,3 s, seed `3523158054` | `vislogs/review-st318e-diff.log` · `28757456…82a81` |
| `cargo fmt --all --check` | **exit 0** | `vislogs/review-st318e-fmt.log` · `3de4ed90…f5fa2` |
| R1 (≤1.500; mục tiêu <700) | OK — max trong phạm vi **628** (`expr.rs`) | `wc -l` |
| R8 (15/40) | **39/40**, `crates/vis-cli/tests` **8/15** — không thêm binary cấp 1 | đếm độc lập + gate log |

Môi trường: toolchain ghim **1.100.0** qua `rust-toolchain.toml` (PATH cần `~/.cargo/bin`);
`CARGO_BUILD_JOBS=2`; **không** `cargo clean`; đĩa trống lúc bắt đầu **~10 GB** (> 6 GB), thấp nhất
trong lúc gate **7,6 GB** (vẫn > 6 GB).

## 5. Hợp nhất

- `git merge-tree --write-tree main HEAD` (`main` = `bc8fe68c`) → tại HEAD builder `14e19e83`:
  tree `131c14b3cb07150337a6abc40f9d29b561e9fbec`, **exit 0, không xung đột**; sau commit review:
  tree `4f8cc4cbb253d5d448e4bac2db33466a1d1f66ef`, vẫn **exit 0**.
- `git merge-tree --write-tree c02050c0 HEAD` → tree `b250cf9c8ca6d2383fec74960c36ab5b643426ca`,
  **exit 0**. `origin/main` = `baa2dbd3` (đúng nền) nên merge với `origin/main` là fast-forward.
- Nhánh **không** chạm `agent/STATE.md` hay bất kỳ tệp nào `main` đã đổi ⇒ **không conflict**
  (dự đoán `STATE.md` không xảy ra vì `STATE.md` chỉ đổi phía `main`).

## 6. Rủi ro & finding

**P1 — không có.** Không đổi logic/byte (`expr` trùng SHA; vistest 0 dòng logic mất), không mất
`pub use`/entry `compile*`/`vis test`, không mất test, không đổi VBC1/golden.

**P2 (thấp, không chặn)**
1. **Nới `pub(super)`**: `expr` (1), `ca` (14 — 2 struct + 10 field + 2 hàm), `tien_trinh`
   (6 hàm). Đây là mức tối thiểu để sibling/parent dùng; không thành API crate. Tái lập:
   `grep -c 'pub(super)' crates/vis-bytecode/src/codegen/expr.rs crates/vis-cli/src/vistest/*.rs`.
2. **rustfmt reflow 2 chữ ký** (`chay_theo_han`, `chay_tuan_tu`) + xoá 1 `use` thừa; **thân hàm
   byte-identical**, không đổi hành vi. Tái lập: multiset dòng ở §2.2.
3. **Glob `super::*` giữa các module `vistest`** (`ca`/`run`/`ten` dùng chéo `Kq`,
   `splitmix64`…). Biên dịch sạch; chỉ là phụ thuộc ngầm, không phải lỗi.

**P3 (hành chính)**
1. Journal builder ghi `expr` "622 dòng" (thực tế thân `fn expr` **618**, cả khối impl **623**)
   và commit `70f8ab29` ghi `vistest 1061 → 202` (HEAD thực tế **201** sau `832bbfee`). Chỉ lệch mô tả.
2. Nhánh không cập nhật `agent/STATE.md` (trong phạm vi cho phép, không bắt buộc; merge sạch).
3. `origin/main` vẫn ở `baa2dbd3` trong khi `main` local đã tới `bc8fe68c` — chỉ là trạng thái
   đồng bộ local/remote, không liên quan ST-318e.

## 7. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"        # toolchain ghim 1.100.0
cd /home/ps/viswt/st318e

git status --porcelain && git log --oneline baa2dbd3..HEAD
git diff --stat baa2dbd3..HEAD
git diff baa2dbd3..HEAD -- agent/API-VBC1.txt agent/API-CODES.txt agent/API-STD.txt

CARGO_BUILD_JOBS=2 cargo test -j 2 -p vis-bytecode -p vis-cli
CARGO_BUILD_JOBS=2 cargo clippy -j 2 --workspace --all-targets -- -D warnings
cargo fmt --all --check
./vis-dev gate --fast
VIS_DIFF_N=100000 ./bin/vis-diff

git merge-tree --write-tree main HEAD
```

## 8. Chứng cứ commit

- HEAD review: `14e19e83` · base `baa2dbd3` · `main` local `bc8fe68c`.
- 4 commit `[ST-318e]`: `22e81371` (codegen), `70f8ab29` (vistest), `832bbfee` (bỏ import thừa),
  `14e19e83` (journal builder).
- **Kết luận: PASS**, đề nghị A0 `DONE` cho ST-318e (không có điều kiện P1).
