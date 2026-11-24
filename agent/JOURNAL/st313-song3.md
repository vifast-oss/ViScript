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
- **Đã di dời thật 2 tệp bảng `CASES`**:
  - `lang_it/vd283_lambda_annot.rs` → 22 ca + `.fmt` (kiểm format bất động) — **xóa tệp gốc**;
  - `lang_it/vd284_derives_json.rs` → 15 ca (2 ca `Err(` dùng `.out_prefix`) + 3 ca lỗi tĩnh
    (`.err = VS0412`) — **xóa tệp gốc**.
  - Tổng corpus: **420 ca**; 0 assertion mất (parity/stdout/codes/fmt/static đều được tái lập).
- **Di dời 27 tệp test API thư viện** khỏi `vis-cli/tests` sang
  `vis-conformance/tests/lib_api_it.rs` (một binary gộp, giữ trần R8):
  - `frame_it/`: `vd260_frame3`, `vd265_frame`, `vd284_async_impl`, `vd284_spawn_method` (41 test);
  - `worker_api_it/`: 5 tệp worker pool (53 test);
  - `async_api_it/`: 22 tệp frame/async (212 test, 3 ignored).
  - Thêm 3 tệp API (`api_dong_bang`, `so_ma`, `vs3_coverage_matrix`) → **323 test**.
  - Tổng **323 test** chuyển nguyên assertion; `vis-cli/tests` giảm **30 tệp**.
- Các tệp còn lại giữ nguyên (đều dùng CLI `Command`/TAP) — chờ A0 duyệt chiến lược giảm trùng lặp.
- `#193`: **vẫn mở** — `vis-wasm-sem` 388/390, 2 ca `ham/de-quy-*` tràn stack trên WASI;
  chưa có bằng chứng đóng.

## Cổng (Sóng 3)

| Cổng | Kết quả |
|---|---|
| `./vis-dev gate --fast` | PASS (gồm conformance + 3804 test) |
| `VIS_DIFF_N=100000` | `lech=0` (55,8 s, 1791/s) |
| `bin/vis-targets` | 100% **23 crate** · PASS=47/45 · FAIL=0 |
| R1..R9 | PASS |
