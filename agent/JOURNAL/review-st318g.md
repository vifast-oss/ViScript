<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# `ST-318g` — REVIEW (Gatekeeper/Agent 1) — **PASS**

- **Ngày**: 2026-11-27 (giờ thấp điểm) · **Vai**: Gatekeeper (Agent 1) độc lập · **Không sửa mã**
- **Nhánh**: `plan/st318g` · **HEAD nghiệm thu**: `c194dd83` · **Nền**: `main` `baa2dbd3`
- **Worktree**: `/home/ps/viswt/st318g` · `main` lúc review: `bc8fe68c` (sau `c02050c0`)
- **Kết luận**: **PASS** — move nguyên văn, API công khai giữ, toàn bộ cổng độc lập xanh.

## 1. Trạng thái git

```
git status --porcelain   -> (rỗng, sạch)
git log --oneline baa2dbd3..HEAD
c194dd83 [ST-318g] journal: ket qua tach R1 vis-loader + vis-runtime (gate/diff/clippy xanh)
4f614921 [ST-318g] R1 mem vis-runtime: tach builtin/mod.rs 1006 -> {mod,host,toan_tu,goi}.rs
71efb3f3 [ST-318g] R1 mem vis-loader: tach modules.rs 1043 -> modules/{mod,kieu,duong_dan,state,nap}.rs
```

`git diff --stat baa2dbd3..HEAD`: 10 tệp, `+1499 / -1386`. Danh sách đúng phạm vi:
`crates/vis-loader/src/modules{,/mod,/kieu,/duong_dan,/state,/nap}.rs`,
`crates/vis-runtime/src/builtin/{mod,host,toan_tu,goi}.rs`, `agent/JOURNAL/st318g-tach-r1-loader-runtime.md`.
**Không** chạm `crates/vis-interp/src/frame.rs` (0 tệp ngoài phạm vi; không sửa
SPEC/QUEUE/DECISIONS/ACTIVE). `git diff --check` sạch; diff không thêm `unsafe` nào.

## 2. LOC sau khi tách

| Trước | Dòng | Sau | Dòng |
|---|---:|---|---:|
| `vis-loader/src/modules.rs` | 1043 | `modules/mod.rs` | 81 |
| | | `modules/kieu.rs` | 68 |
| | | `modules/duong_dan.rs` | 212 |
| | | `modules/state.rs` | 155 |
| | | `modules/nap.rs` | 555 |
| | | **tổng** | **1071** (+28: header/`mod`/`pub use` + helper) |
| `vis-runtime/src/builtin/mod.rs` | 1006 | `builtin/mod.rs` | 128 |
| | | `builtin/host.rs` | 117 |
| | | `builtin/toan_tu.rs` | 287 |
| | | `builtin/goi.rs` | 509 |
| | | **tổng** | **1041** (+35: header/`mod`/`pub use`) |

Tệp lớn nhất sinh mới **555** (`modules/nap.rs`) ≤ 1.500 (R1) và ≤ trần mục tiêu 700.

## 3. Kiểm chứng "move nguyên văn" (đối chiếu `baa2dbd3:path`)

Script đối chiếu dòng-theo-dòng (`difflib`, không dựa vào rename detection của git) trên cả hai cây:

- `vis-loader`: **100% dòng gốc được phủ** bởi 5 tệp mới (1043/1043). Khác biệt **duy nhất**:
  - 8 dòng khai báo helper `fn` → `pub(super) fn` (`chuan_hoa`, `rel_cua`, `chua_cai`, `tach_pkg`,
    `chon_tep_pkg`, `doc_tep_goi`, `ung_vien`, `span_cua`, `load_full_voi`) — bắt buộc để mod con gọi được;
  - `doc_tep_phu` private → `pub(super)` (nới **nội bộ**, không đổi hành vi);
  - thêm `voi_trang_thai` (`TRANG_THAI.with(|c| f(&mut c.borrow_mut()))`) và 4 call-site
    `load_full*` đổi sang `state::voi_trang_thai(...)` / `nap::load_full_voi(...)` — **tương đương
    từng ký tự ngữ nghĩa** với `TRANG_THAI.with(...)` cũ;
  - `mod`/`pub use`/doc header mới. Toàn bộ thân hàm, chú thích, `#[allow]`, thứ tự item giữ nguyên.
- `vis-runtime/builtin`: **100% dòng gốc được phủ** (1006/1006). Khác biệt **duy nhất**:
  - 3 call-site đổi đường dẫn module: `super::value::value_builtin`,
    `super::assert::assert_builtin`, `super::panic::panic_builtin` (trước là sibling trong `mod.rs`);
  - `mod`/`pub use`/doc header mới. Thân `Host`, `bin`/`arith`/`un`, `call`, `KieuKhoa`,
    `sap_xep_theo_khoa`, `trap` **byte-identical** với bản gốc.

Không phát hiện đổi logic, đổi chữ, đảo thứ tự item hay mất nhánh `match`.

## 4. API công khai (đối chiếu tên + kiểm bằng `cargo doc`)

- `vis_loader::modules`: đủ **15** item công khai như bản gốc —
  `FileUnit`, `Loaded`, `LoadError`, `LoadState` (+3 method), `load`, `load_with`, `load_full`,
  `load_full_voi_text`, `load_full_voi_text_trang_thai`, `load_full_stdin`, `dump_deps`, `vi_tri`,
  `dat_noi_dung_mo`, `xoa_noi_dung_mo`, `xoa_cache_phu` (3 hàm free giữ nguyên chữ ký, uỷ quyền
  sang `LoadState`). `cargo doc` sinh đủ trang: `struct.{FileUnit,LoadState,Loaded}.html`,
  `enum.LoadError.html`, 11 `fn.*.html`.
- `vis_runtime::builtin`: đủ `Host` (trait, thân nguyên văn), `call`, `bin`, `un`, `SIGS`, `sig`,
  `builtin_name`, `BuiltinSig`, `pub use method::*`, `pub mod {assert,list,panic}`.
  `cargo doc` sinh `trait.Host.html`, `fn.{call,bin,un,sig,builtin_name}.html`,
  `struct.BuiltinSig.html`, `constant.SIGS.html`.
- `struct Nap<'a>` **vẫn private** (`struct Nap<'a>`, chỉ `impl Nap<'_>` nội bộ `nap.rs`).
- `mod` không treo: `mod goi/host/method/toan_tu` + `mod duong_dan/kieu/nap/state` đều có tệp và
  được `lib.rs` gắn qua `pub mod modules` / `pub mod builtin`.

## 5. Cổng độc lập (chạy lại, không dùng log Builder)

| Cổng | Lệnh | Kết quả |
|---|---|---|
| Test crate | `cargo test -j 1 -p vis-loader -p vis-runtime` | **PASS** 24 + 5 + 290, 0 failed |
| Overlay | `cargo test -p vis-loader tests_overlay` | **PASS** `modules::state::tests_overlay::buffer_mo_thang_dia_va_close_tra_lai_dia` |
| fmt | `cargo fmt --all --check` | **exit 0** (log rỗng) |
| Gate | `./vis-dev gate --fast` | **PASS** — fmt/clippy/R1..R10/DAG/build/doc-examples/L0 **3902/3902** (4 skipped) |
| Diff | `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** — `tong=100000 chay=93768 lech=0 giay=181.4` |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | **exit 0** (53,1 s) |

Log: `/home/ps/vislogs/review-st318g-{test,fmt,gate,diff,clippy,doc}.log`.
Lưu ý tái lập: `cargo` trên `PATH` là 1.93.1 (< MSRV 1.100.0) — phải dùng
`PATH="$HOME/.cargo/bin:$PATH"` hoặc `./vis-dev` (tự chọn `~/.cargo/bin/cargo` 1.100.0).

## 6. R8 và kích thước

- `crates/vis-cli/tests` cấp 1: **8/15**.
- Tổng `crates/*/tests/*.rs` cấp 1 + `vis-host-linux/tests`: **38 + 1 = 39/40** (`bin/vis-dev` R8
  đếm đúng công thức; không có tệp test cấp 1 mới).
- Mọi tệp `vis-loader/src` + `vis-runtime/src` ≤ 1.500; max **976** (`vis-runtime/src/stdlib.rs`,
  có từ trước); max tệp mới **555**.

## 7. Hợp nhất với `main`

```
git merge-tree --write-tree main HEAD
98fca882f26f03f68e59f9fd3e6baa23841ae934      (exit 0, KHÔNG conflict)
```

`main` hiện `bc8fe68c` (đã đi thêm `c02050c0` + `ST-316c`), thay đổi chỉ ở
`vis-diag`/`vis-hir`/`vis-types` + journal. Giao với `baa2dbd3..HEAD` = **rỗng** ⇒ không thể
xung đột văn bản. **Không** có conflict `agent/STATE.md` như dự kiến (main không sửa `STATE.md`);
cây hợp nhất giữ nguyên `agent/STATE.md` của `main`.

## 8. Rủi ro / finding

- **P1**: không.
- **P2**: không.
- **P3 (ghi nhận, không chặn)**:
  1. `crates/vis-runtime/src/builtin/mod.rs` — re-export `pub(crate) use vis_core::host::{…}` tách
     thành `DbHost` (mod.rs) + `{DbHost, FsHost, KetQua, NetHost}` (host.rs); **`MucFs` không còn
     đường dẫn `crate::builtin::MucFs`**. Đây là surface `pub(crate)`; đã grep toàn workspace: 0
     nơi dùng `crate::builtin::MucFs` (nơi dùng thật đều `vis_core::host::MucFs` hoặc
     `crate::builtin::DbHost` — vẫn còn). Build/clippy xanh ⇒ không ảnh hưởng.
  2. `modules/state.rs`: `doc_tep_phu` nới `private → pub(super)` không cần thiết (test helper cùng
     module vẫn truy cập được); chỉ là nới nội bộ, 0 rò API.
  3. Journal Builder liệt kê `sap_xep_theo_khoa`/`KieuKhoa` trong mục "API công khai" — thực tế là
     `pub(crate)` (không đổi so với gốc). Sai sót câu chữ, không ảnh hưởng mã.

## 9. Tái lập nhanh

```bash
cd /home/ps/viswt/st318g
git status --porcelain                       # rỗng
export PATH="$HOME/.cargo/bin:$PATH"         # cargo 1.100.0 (PATH mặc định 1.93.1 quá cũ)
cargo test -j 1 -p vis-loader -p vis-runtime # 24 + 5 + 290
cargo fmt --all --check
./vis-dev gate --fast                        # 3902/3902
VIS_DIFF_N=100000 ./bin/vis-diff             # lech=0
cargo clippy --workspace --all-targets -- -D warnings
git merge-tree --write-tree main HEAD        # exit 0, không conflict
```
