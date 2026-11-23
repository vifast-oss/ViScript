<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# macOS `mps` — verify `v0.4.5` lần đầu (2026-11-23)

> **Máy**: macOS **27.0.1** (Darwin 27.0.0) **arm64**, Rust/Cargo **1.100.0**, Node **26.12.1**,
> Python **3.9.6**, `cargo-nextest 0.9.152`. **Không** phải macOS giả/clone Linux.
> Clone riêng `/Users/mps/dvchd/ViScript-mac-v045` @ tag **`v0.4.5` (`3daff62`)**; nhánh fix
> **`fix/macos-v045-tests`** (`7242c11` → `a66d880` → `d775b91`); không tag/push main từ mps.
> `CARGO_TARGET_DIR=$PWD/target`, `CARGO_BUILD_JOBS=3`, `TMPDIR=/Users/mps/tmp`.

## 1. Kết quả sau fix (nhánh `d775b91`)

| Hạng mục | Kết quả |
|---|---|
| `./vis-dev check` | **PASS** (fmt/clippy/R1..R7/report-check/doc-examples) |
| `./vis-dev gate --fast` | **PASS — 3631 passed / 0 failed / 3 skipped** (79,2 s) |
| `cargo test --workspace --no-fail-fast` | **3636 passed / 0 failed / 3 ignored** |
| `./bin/vis-host-suite` | **54/54**, `host_posix_real` PASS (Darwin arm64) |
| `./bin/vis-wasm-suite` | **28/28**, runtime `node:wasi` |
| `./bin/vis-wasm-sem` | **388/390 eligible = 99,5%**; 2 ca stack-overflow interp đã biết (`ham/de-quy-1000`, `ham/de-quy-vo-ham`), VM pass |
| `vis --version` | **0.4.5** |

Log (mps `/Users/mps/vislogs/`, sha256):
- `mac-fix-check.log` `8db0f36a…c24943`
- `mac-fix-gate.log` `703ef345…164cfd`
- `mac-fix-workspace.log` `6dcceb08…6daed`
- `mac-fix-host.log` `10c68a44…6deecb`
- `mac-fix-wasm2.log` `b0b05c15…69b869`
- `mac-sem.log` `87dd26c4…23fb0a`

## 2. Lỗi lần đầu và phân loại

1. **Clippy/gate đỏ** `st295_sandbox_proc.rs`: `TC`/`tam`/`chay` chỉ dùng trong test
   `#[cfg(target_os = "linux")]` ⇒ macOS báo dead-code (`-D warnings`).
   **Fix**: `#![cfg(target_os = "linux")]` cho cả tệp (test Landlock chỉ Linux).
2. **Workspace 2 ca đỏ**: `vd270_fs_canonicalize::dich_khong_utf8_la_err` và
   `vd270_fs_current_dir::cwd_khong_utf8_la_err_khong_trap` tạo tên/cwd chứa byte `0xFF`;
   APFS không biểu diễn tên không UTF-8 ⇒ `expect` panic. **Fix test**: skip mềm có lý do khi
   nền không tạo được tên; Linux vẫn chạy nguyên.
3. **WASM `BLOCKED-ENV`**: thiếu target `wasm32-wasip1`; đã `rustup target add`.
4. **Harness WASM tìm sai target**: `bin/vis-wasm-suite`/`vis-wasm-sem` dùng cứng
   `vis-host-wasm/target` trong khi build theo `CARGO_TARGET_DIR`. **Fix**: tôn trọng
   `${CARGO_TARGET_DIR:-…}`; `vis-wasm-sem` thêm `from __future__ import annotations` để chạy
   Python 3.9 của macOS.

## 3. Xác nhận trên Linux

- Nhánh fix: `./vis-dev gate --fast` **PASS 3660/4** @ `7242c11` (trước patch harness Python).
- Sau merge vào main (`faac029`): gate **PASS 3671/4**.

## 4. Kết luận

macOS arm64 **chạy thật** toàn bộ gate/workspace/host/WASI ở `v0.4.5` sau các fix test/tooling.
**Chưa tuyên bố hỗ trợ macOS** (A0 quyết định tier/chính sách); bằng chứng này đủ để mở đánh giá
macOS Tier chính thức. Không có lỗi sản phẩm macOS-specific trong đợt đo này.
