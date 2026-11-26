<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-318f — R1 mềm `vis-syntax`: tách `parse/mod.rs`, `lex/mod.rs`, `parse/cmd.rs`

- **Ngày**: 2026-11-26/27 (giờ thấp điểm, UTC) · **Vai**: Builder (Agent 2) dòng ViScript
- **Worktree**: `/home/ps/viswt/st318f` · **Nhánh**: `plan/st318f` · **Nền**: `main` `baa2dbd3` (`VD-297`)
- **Phạm vi**: **chỉ** `crates/vis-syntax/**` + journal này. Không chạm crate khác, không sửa
  `SPEC`/`QUEUE`/`DECISIONS`/`ACTIVE`/`STATE`.

## 0. TL;DR

| Tệp gốc | Trước | Sau | Tệp mới (dòng) |
|---|---:|---:|---|
| `crates/vis-syntax/src/lex/mod.rs` | 1033 | **134** | `scan.rs` 330, `script.rs` 344, `quoted.rs` 107, `cmd.rs` 149 |
| `crates/vis-syntax/src/parse/mod.rs` | 1049 | **69** | `co_so.rs` 256, `kieu.rs` 96, `pattern.rs` 153, `lenh.rs` 401; `khai_bao.rs` 434→559 |
| `crates/vis-syntax/src/parse/cmd.rs` | 1007 | **39** (`cmd/mod.rs`) | `phan_loai.rs` 200, `file.rs` 217, `dong.rs` 155, `stage.rs` 143, `lenh_let.rs` 71, `nhom.rs` 96, `tu.rs` 95, `closure.rs` 89 |

- **0 đổi token/AST/CST/hành vi/API**: chỉ **move nguyên văn**; khác biệt duy nhất là thêm
  `pub(super)` cho method dùng chéo tệp, import tường minh ở đầu mỗi tệp mới, và doc module.
- **0 `unsafe` mới** (grep diff = 0). R8 giữ **39/40** (không tạo tệp test cấp 1 nào).
- Tệp lớn nhất trong lane sau tách: `parse/lenh.rs` **401** (< 700); `expr.rs` 768 (không chạm).
- **Cổng xanh**: `cargo test -p vis-syntax` (70 unit + 1 doctest + 29/3 lib_api), `gate --fast`
  PASS, `VIS_DIFF_N=100000` **`lech=0`**, `clippy --workspace --all-targets -D warnings` sạch,
  `vis-dev rules` OK, `cargo fmt --check` OK.

## 1. Commit

| Commit | Nội dung |
|---|---|
| `e3f966f9` | `[ST-318f]` tách `lex/mod.rs` 1033 → 134 (`scan`/`script`/`quoted`/`cmd`) |
| `eb0180e0` | `[ST-318f]` tách `parse/mod.rs` 1049 → 69 (`co_so`/`khai_bao`/`kieu`/`pattern`/`lenh`) |
| `4269b668` | `[ST-318f]` tách `parse/cmd.rs` 1007 → `parse/cmd/` (8 submodule, `mod.rs` 39) |
| (journal) | `[ST-318f]` nhật ký + bằng chứng |

## 2. Cách tách (giữ nguyên văn)

- Dùng script cắt **đúng khoảng dòng** của tệp gốc (`git show baa2dbd3:<tệp>`), dán vào tệp mới
  trong `impl` riêng; **không** gõ lại, **không** đổi tên biến/hàm/thứ tự nhánh.
- Mỗi tệp mới thêm `// SPDX` + doc module + import tường minh; method dùng chéo tệp được đánh
  `pub(super)` (riêng `parse_type`/`parse_pattern` vẫn `pub(crate)`; `cmd_file` vẫn `pub`).
- `parse/mod.rs` còn docs + `mod` + `pub use api::*` + `ParseOutput` + `Parser` + `PErr` + `PResult`.
  `lex/mod.rs` còn docs + `mod` + re-export + `Mode`/`Diagnostic`/`LexOutput`/`normalize_*`/`lex`/
  `lex_file` + `struct Lexer`. `parse/cmd/mod.rs` còn docs + `mod` + `pub use` 8 hàm công khai.
- **Kiểm chứng**: so từng khối đã cắt với body trong tệp mới (chuẩn hoá `pub(super) `):
  `co_so` 8.019 B, `kieu` 3.416 B, `pattern` 5.293 B, `lenh` 15.184 B, khối `khai_bao` 127 dòng;
  `scan` 10.050 B, `script` 10.785 B, `quoted` 3.490 B, `cmd` 4.335 B; `cmd/*` 8.441/5.723/4.726/
  2.479/3.451/3.438/2.905 B + `phan_loai` 7.497 B — **khớp 100%** (chỉ khác tiền tố visibility).

## 3. Cổng (log tại `/home/ps/vislogs/st318f-*.log`)

| Cổng | Kết quả |
|---|---|
| `cargo fmt --all -- --check` | OK |
| `cargo test -p vis-syntax` | **70** unit + **1** doctest + lib_api 29/3, 0 fail |
| `./vis-dev gate --fast` | **PASS** (L0 OK, `GATE_EXIT=0`) |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | `tong=100000 … **lech=0**` (91,2 s; seed 3523158054) |
| `cargo clippy --workspace --all-targets -- -D warnings` | sạch (exit 0) |
| `./vis-dev rules` | **OK** (R1..R10) |
| `unsafe` mới | 0 |

## 4. Việc còn lại

- Agent 1 (Gatekeeper) audit độc lập; A0 duyệt `DONE` rồi hợp nhất `plan/st318f`.
- Dọn worktree/nhánh trong 24 h sau merge (`VD-288`); `agent/STATE.md` để tầng tích hợp cập nhật
  cùng lúc nhiều lát song song (`main` đang có `ST-318a..e/g` chạy).
- Không phát sinh nợ mới; không đổi `API-CODES.txt`/`API-STD.txt`/VBC1.
