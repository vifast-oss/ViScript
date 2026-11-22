<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-303 — `vis new` / `vis doc` / `vis repl` (VD-283 làn 4)

> **Người làm**: W3, 2026-11-23 · nhánh **`plan/w3-st303-cli-dev`** từ main **`9af3976`**.
> Không merge/push/tag trước `v0.4.5`; không GitHub Actions; không đụng `QUEUE`/`SO-NO`.
> `CARGO_BUILD_JOBS=3`, `CARGO_PROFILE_DEV_DEBUG=0`, `TMPDIR=/home/ps/tmp`, target riêng.

## 1. Lệnh mới

- **`vis new <dir> [--template bin|lib]`** (`crates/vis-cli/src/cli/new.rs`):
  tạo `main.vis` (+ `lib.vis` cho `lib`), `test/sem/smoke.vis` + `.out`, `README.md`;
  từ chối thư mục đã tồn tại và template lạ (exit 2). Dự án sinh ra chạy:
  `vis check main.vis`, `vis test`, `vis run main.vis`.
- **`vis doc <path> [--format md|json]`** (`cli/doc.rs`):
  parse CST `vis-syntax`, lấy token `DocComment` (`##`) gắn với item cấp tệp kế tiếp;
  xuất `# path` + `## <signature>` (md) hoặc JSON `{path,items:[{line,signature,doc}]}`.
  Không regex trên text; parse lỗi ⇒ exit 2.
- **`vis repl`** (`cli/repl.rs`): `vis-loader` (`load_full_stdin`) + `vis-types::check` +
  `vis-interp::run`; commit từng lượt sạch, in phần stdout mới; lượt lỗi in stderr và **rollback**
  (không mất state). Lệnh phiên: `:help`, `:type <expr>`, `:reset`, `:quit`.
- `main.rs` chỉ thêm **3 match arm + 1 dòng usage** (4 dòng thêm / 1 dòng sửa — ≤20); module khai
  báo tại `cli/mod.rs`.

## 2. Test mới

`crates/vis-cli/tests/cli_it/st303_cli_dev.rs` (8 ca, thêm vào binary `cli_it`):
1. `new_bin_du_an_chay_check_test_run`
2. `new_lib_du_an_chay_check_test_run`
3. `new_tu_choi_thu_muc_ton_tai`
4. `new_tu_choi_template_la`
5. `doc_md_va_json_bang_cst`
6. `doc_tu_choi_file_khong_ton_tai`
7. `repl_giu_trang_thai_va_rollback`
8. `repl_help_va_quit`

`cargo test -p vis-cli --test cli_it st303_cli_dev` → **8/8 pass**.

## 3. Cổng

| Lệnh | Kết quả |
|---|---|
| `./vis-dev gate --fast` | **PASS — 3668 passed / 4 skipped** (94 binary; wall **190,97 s**) |
| `cargo check -p vis-cli --no-default-features --tests` | **PASS** |
| fmt / clippy / R1..R7 | **PASS** |
| 0 `unsafe` | Có (`#![forbid(unsafe_code)]` của `vis-cli`; không thêm `unsafe`) |

Log gate ngoài repo: `/home/ps/vislogs/st303-gate.log`
sha256 `e004ced9fb39b3863d64da52ff54f0ceedae59bc2b5f0c21496b4aa59d6d3581`.

## 4. Còn lại

- **ST-304** (`crates/vish-edit`) **chưa mở** — ngoài thời gian 2,5 h của lát này; không phải
  blocker ST-303.
- Không merge/push/tag; chờ `v0.4.5` như VD-283 §2.
