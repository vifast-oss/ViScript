# ST-313c/d — Sóng 3 (2026-11-24/25)

- **Mốc**: `c829b6a` → `dfe6f186`. Chế độ VD-289; chỉ `ps` + `psvn` (không dùng `mps`).
- **Mục tiêu**: harness dữ liệu `interp ↔ VM`, dời test API thư viện khỏi `vis-cli`,
  chuyển hành vi sang `test/conformance/`, rà R1..R9, đóng `#193`.

## ST-313c — `crates/vis-conformance`

- Crate mới chạy thẳng **API thư viện** (`vis_syntax` → `vis_types` → `vis_interp` +
  `vis_bytecode`/`vis_vm`), không cần binary `vis`, không `Command`.
- Định dạng ca:
  - `<tên>.vis` — nguồn (bắt buộc);
  - `<tên>.out` — stdout chính xác (thiếu ⇒ rỗng); `.out_prefix` — chỉ cần tiền tố;
  - `<tên>.err` — mã lỗi tĩnh mong đợi (parse/lower/tyck), mỗi dòng một mã;
  - `<tên>.exit` — mã thoát (thiếu ⇒ 0);
  - `<tên>.fmt` — bật kiểm `vis_fmt` bất động.
- `use std.<mod>` được quét như loader CLI (`std_mods`).
- Test `tests/corpus.rs` chạy toàn bộ corpus, báo danh sách ca đỏ.

## ST-313d — dữ liệu hóa hành vi + dời test API

- Trích tự động từ `vis-cli/tests/{lang_it,stdlib_it}`:
  - **297** ca `dung(tag, src, stdout)` → `.vis` + `.out`;
  - **118** ca `tu_choi(tag, src, mã)` → `.vis` + `.err`;
  - **5** ca bảng `CASES` (gồm 2 `.out_prefix`) → **420 ca** tổng, tất cả xanh.
- **Di dời thật**:
  - `vd283_lambda_annot` (22 ca + `.fmt`) và `vd284_derives_json` (15 ca + 3 lỗi tĩnh)
    → xoá tệp gốc, 0 assertion mất;
  - 27 tệp frame/worker/async/API → `vis-conformance/tests/lib_api_it.rs`
    (**336 test**: 333 pass + 3 ignored);
  - `vs3_keepalive` (8 test) → `vis-host-native/tests/net_it.rs`;
  - tách 10 test API khỏi `vs3_worker_frame` (giữ 2 test CLI).
- Dọn dẹp: xoá `mod common;` chết ở 6 aggregator + `vis-cli/tests/common/`;
  sửa doc rot; gộp `vis-fmt/moi → fmt`, `vish-edit/completion_job → editor`.
- **R8**: tổng **38** binary (vis-cli 8/15, còn 2 slot tổng).

## `#193` — ĐÓNG

- WASI cần **hai** trần stack:
  - `-zstack-size` module: **12 MiB** đủ cho 2.000 khung ngôn ngữ (8 MiB vẫn tràn);
  - `--stack-size=4096` cho V8 `node:wasi` (mặc định ~1 MiB chặn trước).
- Đã mặc định trong `bin/vis-wasm-suite` và `bin/vis-wasm-sem`
  (override `VIS_WASM_STACK_MB` / `VIS_WASM_NODE_STACK_KB`).
- Đo lại: **sem 390/390**, **suite 28/28**.

## Cổng (Sóng 3)

| Cổng | Kết quả |
|---|---|
| `./vis-dev gate --fast` | **PASS** (3800 test) |
| `cargo test --workspace --no-fail-fast` (fallback libtest) | **3805 pass / 0 fail** |
| Windows (`psvn`): conformance | corpus 420 ca OK; `lib_api_it` 312 pass + 3 ignored (21 ca `cfg(unix)`) |
| `VIS_DIFF_N=100000` | `lech=0` (55,8 s, 1791/s) |
| `bin/vis-targets` | 100% **23 crate** · PASS=47/45 · FAIL=0 · `vis-dag-check` OK |
| clippy `--workspace --all-targets -D warnings` | Linux PASS · Windows (`psvn`) PASS · **macOS bỏ qua** (`mps` bận) |
| `vis-wasm-suite --build` | **28/28** |
| `vis-wasm-sem` | **390/390** |
| conformance corpus | **420/420** |
| R1..R9 | PASS |
| R8 | **38** binary (trần 40) |

## Ghi chú vận hành

- Một lần `gate --fast` dưới tải có 2 ca `vis-cli::vis_test` đỏ; chạy lại **PASS**
  ⇒ phân loại **T-INFRA** (flake tài nguyên), không sửa ngưỡng/assertion.
- `#193` đóng bằng cấu hình harness, không đổi ngữ nghĩa ngôn ngữ; vẫn giữ caveat
  “WASI runtime có phạm vi, không full host”.
- Các tệp `lang_it`/`stdlib_it` còn lại kiểm hành vi CLI (TAP/`check`/stderr) — giữ ở
  `vis-cli`; kế hoạch giảm trùng lặp nằm trong audit gửi A0.
