<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-296c — tách stdlib / lower / sqlite tests + gom 68 module interp stdlib

**Trạng thái**: DONE. **Nhánh**: `plan/w1-st296c-stdlib` từ `main` **`ab17f1a`**. Không push/merge/tag.
**Phạm vi**: 0 đổi API/hành vi, 0 `unsafe`, không đổi default/cache/ABI. Target riêng
`CARGO_TARGET_DIR=/home/ps/ViScript/target/vs3-w1-st296c`; `CARGO_BUILD_JOBS=3`,
`CARGO_PROFILE_DEV_DEBUG=0`, `TMPDIR=/home/ps/tmp`.

## 1. Số dòng trước → sau

| Tệp gốc | Trước | Sau |
|---|---:|---|
| `crates/vis-types/src/stdlib.rs` | 1.456 | **23** + 11 module con (13–251 dòng; tổng 1.289) |
| `crates/vis-cli/src/db/sqlite/tests.rs` | 1.495 | **68** + 3 module (279 / 528 / 637) |
| `crates/vis-types/src/lower/expr.rs` | 1.456 | **702** + 2 module (234 / 550) |
| `crates/vis-types/src/lower.rs` | 1.446 | **420** + 2 module (764 / 285) |
| `crates/vis-interp/src/stdlib/` | **68 tệp** (9.542 dòng) | **17 tệp** (9.753 dòng) theo miền |

Cả 3 tệp cận trần R1 nay **< 900** dòng. Mọi tệp `.rs` sửa/thêm ≤ 1.500 (R1 PASS).

## 2. Cách tách (nguyên văn, không đổi logic)

- `vis-types/stdlib`: `ty` (Ty + `sang_type` + `ten_bien`), `dinh_nghia` (StdFn/MODULES),
  `funcs_*` (6 nhóm theo tiền tố `str/list+map/fs+path/net+http/sqlite/time+json`), `builtin`,
  `bang` (ghép mảng const bằng macro `noi_bang!` — vẫn `FUNCS: [StdFn; 167]`), `api`
  (`sig_of`/`sig_builtin`/…). `stdlib.rs` chỉ còn `mod` + `pub use` ⇒ API y nguyên.
- `sqlite/tests.rs`: giữ helper dùng chung ở cha; 54 ca chia 3 module con `tests/sqlite_{a,b,c}`;
  **không đổi tên ca**; helper `db_gioi_han` chuyển `pub(super)` cho module em dùng.
- `lower.rs`: `new/run/finish` + khung tên/khai báo builtin → `lower/co_so.rs`; phân giải kiểu →
  `lower/kieu.rs`; struct/API công khai ở `lower.rs`.
- `lower/expr.rs`: literal chuỗi/nội suy → `expr/chuoi.rs`; gọi hàm/đối số/dispatch kiểu →
  `expr/goi.rs`; phần `lower_expr` chính + if/lambda + helper ở `expr.rs`.
- `vis-interp/stdlib`: 68 tệp nhỏ gom thành 17 module miền (`helper`, `list_a/b`, `path_a/b/c`,
  `fs_net`, `sqlite_a..e`, `str_a..e`); mỗi tệp gốc thành `pub(crate) mod <tên_gốc>` trong module
  miền, re-export tại `stdlib.rs` ⇒ đường dẫn `stdlib::<tên_gốc>::…` **không đổi**. Chỉ đổi
  `pub(super)` → `pub(crate)` (nội bộ) và `super::` → `super::super::` ở phần thân (giữ nguyên
  `super::` trong `mod tests`).

## 3. Kiểm chứng

- `cargo check --workspace` xanh; `cargo test -p vis-types` 62 ca stdlib + 17/1/29/6/12/3/2 xanh;
  `cargo test -p vis-interp --lib` **303/303**; `cargo test -p vis-cli --bin vis db::sqlite`
  **87/87**.
- `./vis-dev gate --fast` ⇒ **PASS**: **3656 passed / 4 skipped**, fmt/clippy/R1..R7/self-test/
  build/doc-examples OK. Log đầy đủ **ngoài repo** (R7/VD-282):
  `/home/ps/vislogs/st296c-gate.log` sha256 `18e16706…ecdd` (vòng trước rebase) và
  `/home/ps/vislogs/st296c-rebase-gate.log` sha256 `b5622329…c4474` (vòng sau rebase).
- `VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff` ⇒ **lech=0** (140,5 s;
  `st296c-diff-100k.log`).
- Đĩa còn **27 G** (> 10 G).

## 3b. Rebase lên `main` `1b27cb8` (ST-296a + ST-296b)

- `git rebase main` **không xung đột**; commit rebase `28738fe` (nội dung y nguyên 47b7c85).
- `cargo check --workspace` xanh, gồm `vis-loader` và `vis-lsp` mới của ST-296b.
- Gate **3656/4** PASS; `api_dong_bang` (API-STD ↔ FUNCS) xanh trong L0.
- R7 mới của ST-296b bắt log gate 338 KB còn trong repo ⇒ đã chuyển ra
  `/home/ps/vislogs/st296c-gate.log` (sha256 `18e16706…`), chỉ giữ log diff 264 B + tóm tắt này.
- Một lần gate đỏ do flake thread-count `vs3_worker::spawn_shared_stress_rounds_read_only`
  (2→4) dưới tải cao; chạy riêng xanh, gate lại xanh — không phải hồi quy ST-296c.

## 4. Ghi chú / còn lại

- Một số module `vis-types/stdlib/funcs_*` nhỏ hơn 300 dòng do bảng chữ ký chỉ ~1.287 dòng sau
  khi bỏ comment giữa các entry (comment chỉ là tài liệu, không phải hành vi); vẫn giữ 11 module
  đối xứng theo miền. Không gộp thêm để tránh file > 400 dòng vô ích.
- Không đổi `API-STD.txt`/`FUNCS` (167)/thứ tự hàm; test `api_dong_bang` xanh trong gate.
- Không còn phần bắt buộc nào dang dở; P4c/Windows không thuộc ST-296c.
