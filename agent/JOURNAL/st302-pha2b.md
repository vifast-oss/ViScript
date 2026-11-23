<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-302 (Pha 2b) — `async` trong `impl`/`cmd` (#13) + `derives Json` (#8)

**Nhánh**: `plan/w1-st302-pha2b` từ `main` **`700bb66`** (VD-284). Không merge/push/tag;
không ssh mps. Target riêng `.../target/vs3-w1-st302`; 0 `unsafe`; R1/R7 giữ.

## 1. `#13` — `async fn` trong `impl` + `async cmd`

- Parser `impl` nhận `async` trước `fn`; tyck cho `async` trên method `impl`/`cmd` (trait method
  async vẫn từ chối), gọi method async trả `Task[ret]`; interp/VM dùng đường `HFn.is_async` sẵn có.
- Test `crates/vis-cli/tests/vd284_async_impl.rs` **3/3**: `self`, `var self`, `async cmd` +
  `await`; parity interp↔VM.
- `--flag` riêng của `cmd` giữ nguyên (không đụng parser `cmd`).
- **Chưa làm**: `spawn obj.m()` (spawn method) — `spawn` vẫn chỉ nhận tên hàm cấp tệp.

## 2. `#8` — `derives Json` sinh mã thật

- Lower sinh 2 method thật cho `type`/`enum`:
  - `to_json(self) -> Json`
  - `from_json(j: Json) -> Result[Type, Error]` (gọi tĩnh `Type.from_json(j)`).
- Descriptor kiểu do lower sinh (record/enum lồng, `T?`, list/map/Result, số/chuỗi/bool/path/char/
  size/duration/percent); runtime `crates/vis-interp/src/json_derive.rs` chuyển `Value ↔ Json`.
- Từ chối **tĩnh** `cap`/`Chan`/`Task`/`fn` bằng **`VS0412`** (kể cả khi lower permissive giữ
  `{error}` — kiểm thêm văn bản khai báo).
- 15 ca `crates/vis-cli/tests/vd284_derives_json.rs` (record/optional/list/map/nested/enum
  unit/payload/multi/float/bool-char-path/error/thiếu trường/list record) + 3 ca từ chối.
- `CHEAT-SHEET-ViScript.md` thêm mục `derives Json`; cache `PHIEN_BAN` thêm nhãn
  `+st302-derives-json-async-impl-2026-11-23` (bytecode cùng nguồn đổi nghĩa) + ghim lại khoá
  `88613f2f…` trong `vis-loader/src/cache.rs`.
- DOC_BLOCKS cập nhật 2 khối có `derives Json` (item/khai báo tăng do method sinh).

## 3. Kiểm chứng

- `./vis-dev gate --fast` ⇒ **PASS — 3676 passed / 4 skipped** (3671 + 5 test function mới);
  fmt/clippy/R1..R7/self-test/build/doc-examples/readme OK.
  Log `/home/ps/vislogs/st302-gate.log` sha256 `60d7a984…0adfe565`.
- `VIS_DIFF_N=100000` seed `20261115` ⇒ **lech=0** (84,6 s;
  `/home/ps/vislogs/st302-diff-100k.log` sha256 `aec94e9c…c97ed94`).
- `agent/SO-NO.md`: #8/#13 `đóng ở ST-302`; §8 **83 mở / 91 đóng / 17 VS3 / 2 / 1 = 194**.

## 4. Chưa làm / blocker

- `spawn obj.m()` (method async qua `spawn`) — chưa mở; `await obj.m()` đã chạy.
- Windows `vis-gate-windows --quick` / macOS: **chưa chạy** (m2 độc quyền; để sau).
- Không merge.
