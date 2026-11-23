<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-302 (Pha 2b) — PARTIAL: `async fn` trong `impl` + `async cmd` (#13)

**Trạng thái**: **DỪNG THEO CHỈ ĐẠO GIỜ CAO ĐIỂM** — commit partial, không gate, không merge.
**Nhánh**: `plan/w1-st302-pha2b` từ `main` **`700bb66`** (VD-284). Không push/tag, không ssh mps.

## 1. Đã xong (code + kiểm tay)

- Parser `crates/vis-syntax/src/parse/khai_bao.rs`: `impl` nhận `async` trước `fn`
  (`mods` như cấp tệp).
- Tyck `crates/vis-types/src/tyck/mod.rs`: cho `async` trên `impl` method và `cmd`;
  trait method async vẫn từ chối (`ASYNC_SAI_CHO`).
- Tyck `crates/vis-types/src/tyck/tuy_chon.rs`: gọi method async trả `Task[ret]` (không
  chạy thân, không báo lỗi cũ "chỉ hàm cấp tệp").
- Interp/VM: đường gọi method qua `sym` sẵn có (`HFn.is_async`) đã trả `Task` — không cần
  sửa thêm; `await obj.m()` chạy đúng.
- Kiểm tay (binary debug, chưa gate):
  - `type C` + `impl C: async fn lay(self) -> int` + `await c.lay()` ⇒ interp `7`, VM `7`,
    exit 0 cả hai.
  - `async cmd chao(ten: str)` + `await chao("an")` ⇒ interp/VM `chao an`, exit 0.

## 2. Chưa làm

- **`derives Json` codegen thật** (`#8`): chưa sinh `.to_json()`/`Type.from_json`; chưa có
  test ≥15 ca; chưa cập nhật `CHEAT-SHEET-ViScript.md`.
- `spawn obj.m()` (method async qua `spawn`) chưa mở — hợp đồng ST-302 cần; hiện `spawn`
  vẫn chỉ nhận tên hàm cấp tệp.
- Chưa thêm bộ test ≥15 ca; chưa chạy `gate --fast`, fmt/clippy, diff 100k.
- Windows `vis-gate-windows --quick` / macOS: **không chạy** (theo chỉ đạo, không ssh mps).

## 3. Bằng chứng / rủi ro

- Code mới compile qua `cargo build -p vis-cli --bin vis` (đã chạy trước khi dừng).
- Không đổi ABI/`Value`/cache; 0 `unsafe`.
- Rủi ro còn lại: `async cmd` gọi không `await` (bỏ Task) cần test/tyck; method async
  trong `spawn`; derive Json chưa có.
