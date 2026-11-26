<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# REVIEW ST-318f — Gatekeeper (Agent 1) độc lập

- **Ngày**: 2026-11-27 (giờ thấp điểm UTC) · **Vai**: Gatekeeper (Agent 1), không sửa mã
- **Worktree**: `/home/ps/viswt/st318f` · **Nhánh**: `plan/st318f` · **HEAD**: `916f7b02`
- **Nền**: `main` `baa2dbd3` · **main lúc audit**: `bc8fe68c` (mới hơn `c02050c0`)
- **Kết luận**: **PASS** — không có P1; 2 ghi nhận P2 không chặn (xem §7)

## 1. Phạm vi

| Kiểm | Kết quả |
|---|---|
| `git status --porcelain` trước audit | sạch |
| `git log --oneline baa2dbd3..HEAD` | 4 commit, tất cả `[ST-318f]`, merge-base đúng `baa2dbd3` |
| `git diff --stat` | 24 tệp, **+3157 / −2908** |
| Ngoài `crates/vis-syntax/**` + `agent/JOURNAL/` | **0 tệp** (`OUT_OF_SCOPE_COUNT=0`) |
| `git diff --check` | exit 0 (không lỗi khoảng trắng) |
| `unsafe` mới trong diff | **0** |

Commit: `e3f966f9` (lex), `eb0180e0` (parse), `4269b668` (cmd), `916f7b02` (journal).

## 2. LOC sau tách (khớp báo cáo Builder)

| Tệp gốc | Trước | Sau | Tệp mới |
|---|---:|---:|---|
| `lex/mod.rs` | 1033 | **134** | `scan` 330, `script` 344, `quoted` 107, `cmd` 149 |
| `parse/mod.rs` | 1049 | **69** | `co_so` 256, `kieu` 96, `pattern` 153, `lenh` 401; `khai_bao` 434→**559** |
| `parse/cmd.rs` | 1007 | **cmd/mod.rs 39** | `phan_loai` 200, `file` 217, `dong` 155, `stage` 143, `lenh_let` 71, `nhom` 96, `tu` 95, `closure` 89 |

Tệp lớn nhất **mới tạo**: `parse/khai_bao.rs` 559 (< 700 mục tiêu, < 1500 trần).
`parse/expr.rs` 769 dòng nhưng **không thuộc phạm vi tách** (chỉ đổi 2 dòng `use`), xem P2-1.

## 3. Kiểm move nguyên văn — bằng chứng máy

Phương pháp: bóc **thân hàm** (khớp ngoặc, tính cả chuỗi/comment) và **item kiểu** (struct/enum/
const/type) từ bản `baa2dbd3` và từ các tệp mới, so **multiset byte-for-byte**; sau đó so thứ tự
xuất hiện và tập item `pub`.

| Nhóm | Hàm cũ | Hàm mới | Thiếu | Thừa |
|---|---:|---:|---:|---:|
| `lex/mod.rs` → `scan`/`script`/`quoted`/`cmd` | 46 | 46 | **0** | **0** |
| `parse/mod.rs` → `co_so`/`kieu`/`pattern`/`lenh` | 63 | 58 (+5 sang `khai_bao`) | **0** | **0** |
| `parse/cmd.rs` → `cmd/*` (8 submodule) | 29 | 29 | **0** | **0** |

- 5 hàm chuyển sang `khai_bao.rs`: `file` (17 dòng), `at_item_start` (11), `item` (59),
  `use_decl` (6), `dotted_path` (16) — **khớp byte 100%** với bản cũ. Đây là toàn bộ phần tăng
  `khai_bao` 434→559 (`diff` +127/−2: 5 hàm + `impl`/import/doc; 58 hàm còn lại không đổi).
- Item kiểu: `lex` 6/6, `cmd` 2/2 (gồm `enum KieuThan`, `const KHAI`), `parse` `type PResult`
  — tất cả khớp; không có `trait`/`static`/`macro_rules!` nào bị bỏ.
- Thuộc tính: `lex` 4/4 (`3 derive` + `#[cfg(test)] mod tests`), `parse` 2/2
  (`derive ParseOutput`, `derive KieuThan`) — không mất `#[cfg]`/`#[derive]`.
- Thứ tự: mỗi tệp mới là **dãy con liên tiếp** theo thứ tự hàm cũ; ngoại lệ duy nhất là
  `script.rs` đặt 2 hàm tự do `scan_size_suffix`/`scan_duration_suffix` ở cuối tệp (bản cũ cũng
  ở cuối tệp, thân khớp byte; không ảnh hưởng hành vi).
- Tập `pub` (đường dẫn công khai): **không có item `pub` nào của bản cũ bị mất**; chỉ thêm 1
  `pub use phan_loai::{…}` trong `parse/cmd/mod.rs` tái xuất đúng 8 hàm vốn ở `parse::cmd::*`.
- `parse/api.rs`, `parse/expr.rs`: diff **chỉ** ở dòng `use` (đổi `use super::*` → import tường
  minh); không đổi mã.
- `parse/mod.rs`, `lex/mod.rs`: khối `Mode`/`Diagnostic`/`LexOutput`/`normalize_*`/`lex`/
  `lex_file`/`struct Lexer`/`ParseOutput`/`Parser`/`PErr`/`PResult` giữ nguyên văn (đối chiếu
  `diff` phần đầu tệp: chỉ thêm `mod` + doc + `pub use`).

⇒ Không đổi token/span/thứ tự chẩn đoán/nhánh scanner/nhánh parser. Các thay đổi văn bản còn lại
chỉ là: thêm `pub(super)` cho method dùng chéo tệp (module con vẫn **private**), import tường
minh, doc module, và tách `impl` theo tệp.

## 4. Frontend + cổng độc lập

| Cổng | Kết quả | Bằng chứng |
|---|---|---|
| `cargo fmt --all --check` | **OK** (exit 0) | chạy trực tiếp |
| `cargo test -p vis-syntax --jobs 2` | **lib 29 + doc_fixtures 3 + syntax_it 70 + doctest 1**, 0 fail | `/home/ps/vislogs/review-st318f-test.log` (`sha256 6a1da0e6…`) |
| `./vis-dev gate --fast` (jobs=2) | **PASS**, `GATE_EXIT=0`; L0 **3902 passed / 4 skipped**; fmt/clippy/R1..R10/DAG/toolchain/release/fixtures/report/doc-examples/readme đều OK | `/home/ps/vislogs/review-st318f-gate.log` (`sha256 77328d71…`) |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | `tong=100000 … **lech=0**`, seed `3523158054`, 153,1 s (máy đang chạy gate song song; Builder 91,2 s) | `/home/ps/vislogs/review-st318f-diff100k.log` (`sha256 11df7ca6…`) |
| `cargo clippy --workspace --all-targets -- -D warnings` | **sạch**, exit 0 (42 s) | `/home/ps/vislogs/review-st318f-clippy.log` (`sha256 7fa24bd9…`) |
| R8 | **39/40** (38 `crates/*/tests` + 1 `vis-host-linux/tests`); `vis-cli/tests` **8/15** | đếm trực tiếp + `luật R1..R10: OK` |
| Trần dòng | tệp mới lớn nhất **559**; không tệp nào > 1500 | `wc -l` |

Test **không bị bỏ**: `crates/vis-syntax/tests/**` và `src/lex/tests.rs` không có trong diff;
`#[test]` của `syntax_it` vẫn 70, khớp Builder.

## 5. API

- `agent/API-VBC1.txt` **không đổi** (`git diff --quiet` + sha256
  `64282aee3645be729438ff8791f485444bf1472be40545667808994311e3ba6f`); `API-CODES.txt`,
  `API-STD.txt` cũng không đổi.
- Test đóng băng `vis-conformance::lib_api_it api_dong_bang::api_vbc1_khop_code` **PASS**
  (gate log dòng 2036); `api_std_khop_code`, `api_codes_khop_code` PASS.
- Tập item `pub` của `vis_syntax` trước/sau: **không mất đường dẫn nào**; `parse::cmd::*` 8 hàm
  vẫn công khai qua re-export; `parse_type`/`parse_pattern` vẫn `pub(crate)`, `cmd_file` vẫn
  `pub` (method trên `Parser`, định vị qua kiểu nên đường dẫn không đổi).

## 6. Hợp nhất

- `git merge-tree --write-tree main HEAD` → **exit 0**, tree `95608cda191e…`; **0 tệp chồng lấn**
  giữa `main` (`bc8fe68c`) và nhánh (so `git diff --name-only` hai phía từ merge-base).
- Nhánh **không chạm** `agent/STATE.md` ⇒ không có xung đột `STATE.md` như dự kiến; tầng tích hợp
  tự cập nhật `STATE.md` khi merge nhiều lát song song.

## 7. Rủi ro / ghi nhận

- **P1**: không có.
- **P2-1 (không chặn, có sẵn)**: `parse/expr.rs` **769 dòng** > mục tiêu 700. Tệp này chỉ đổi
  2 dòng `use` trong lát này; không phải hồi quy. Đề xuất tách `expr` ở lát R1 mềm sau.
- **P2-2 (thấp)**: **110** method được nâng `pub(super)` để dùng chéo tệp (bản cũ: 0). Đã đối chiếu: module
  chứa chúng đều **private** (`mod scan/script/quoted/cmd`, `mod co_so/kieu/pattern/lenh`,
  `cmd/*`), tập item `pub` bare không đổi ⇒ không nới API công khai. Cần giữ luật này cho các lát
  tách sau (không đổi `pub(super)` thành `pub`).
- **P2-3 (quan sát)**: `script.rs` không giữ thứ tự tuyến tính tuyệt đối vì 2 hàm tự do
  `scan_*_suffix` nằm cuối tệp; thân khớp byte nên không ảnh hưởng.
- Không phát hiện: `CoSo`/`Kieu`/`Pattern` visibility sai, scanner đổi nhánh, parser đổi thứ tự,
  test bị bỏ.

## 8. Tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"   # rustup 1.100.0, không dùng cargo hệ thống 1.93.1
cd /home/ps/viswt/st318f
git status --porcelain
git diff --stat baa2dbd3..HEAD
cargo fmt --all --check
cargo test -p vis-syntax --jobs 2
VIS_DIFF_N=100000 ./bin/vis-diff
cargo clippy --workspace --all-targets -- -D warnings
./vis-dev gate --fast
git merge-tree --write-tree main HEAD
```

Ràng buộc đã giữ: `jobs=2`, không `cargo clean`, đĩa trống 9,9 GB (> 6 GB), không push.
