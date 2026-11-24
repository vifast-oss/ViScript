# ST-313c/d — Sóng 3 (2026-11-24/25)

- **Mốc**: `c829b6a` → commit ST-313c. Chế độ VD-289; chỉ `ps`.
- **Mục tiêu**: harness dữ liệu `interp ↔ VM`, chuyển hành vi `lang_it`/`stdlib_it` sang
  `test/conformance/`, rà R1..R9, đánh giá `#193`.

## ST-313c — `crates/vis-conformance`

- Crate mới chạy thẳng **API thư viện** (`vis_syntax` → `vis_types` → `vis_interp` +
  `vis_bytecode`/`vis_vm`), không cần binary `vis`, không `Command`.
- Định dạng ca:
  - `<tên>.vis` — nguồn (bắt buộc);
  - `<tên>.out` — stdout chính xác (thiếu ⇒ rỗng);
  - `<tên>.err` — mã lỗi tĩnh mong đợi (mỗi dòng một mã) ⇒ ca **lỗi tĩnh**
    (parse/lower/tyck phải chứa lần lượt các mã);
  - `<tên>.exit` — mã thoát (thiếu ⇒ 0).
- `use std.<mod>` được quét như loader CLI (`std_mods`).
- Test `tests/corpus.rs` chạy toàn bộ corpus, báo danh sách ca đỏ.

## ST-313d — dữ liệu hóa hành vi

- Trích tự động từ `vis-cli/tests/{lang_it,stdlib_it}`:
  - **297** ca `dung(tag, src, stdout)` → `.vis` + `.out`;
  - **118** ca `tu_choi(tag, src, mã)` → `.vis` + `.err`.
- Tổng **415 ca**, tất cả xanh, interp ↔ VM byte-identical.
- SPDX R3: thêm header cho toàn bộ `.vis`.
- **Không xóa tệp gốc** (0 assertion mất); việc giảm trùng lặp chờ A0 duyệt.
- `#193`: **vẫn mở** — `vis-wasm-sem` 388/390, 2 ca `ham/de-quy-*` tràn stack trên WASI;
  chưa có bằng chứng đóng.

## Cổng (Sóng 3)

| Cổng | Kết quả |
|---|---|
| `./vis-dev gate --fast` | PASS (gồm conformance + 3804 test) |
| `VIS_DIFF_N=100000` | `lech=0` (55,8 s, 1791/s) |
| `bin/vis-targets` | 100% **23 crate** · PASS=47/45 · FAIL=0 |
| R1..R9 | PASS |
