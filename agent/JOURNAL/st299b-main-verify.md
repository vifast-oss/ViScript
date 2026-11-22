<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-299b — xác minh trên main `16fba3a` (WASI + `vis-loader`)

> **Người chạy**: W3, 2026-11-23 · nhánh **`plan/w3-st299b-verify`** từ **`16fba3a`**
> (`ST-299b` đã merge). Không push/merge/tag; không sửa code; không chạy gate/soak/diff
> (W4 đang soak Windows).

## 1. `bin/vis-wasm-suite` — 28/28

- Runtime: **Node `v26.5.0`** (`node:wasi`, `/usr/local/bin/node`); `wasmtime` CLI và Python
  `wasmtime` không có trên máy, script tự chọn Node.
- **28/28 ca xanh**, gồm `da_tep` (interp + VM): `ok 1..28`, `# tổng: 28 ca, 0 hỏng`.
- Artifact: `vis-host-wasm/target/wasm32-wasip1/release/vis-wasm-run.wasm`
  **3.646.721 byte**, sha256 **`4445698c5adac5e6775240725a12e7236c4d27ba6faac871349f909745a3f617`**.
- Log: `/home/ps/vislogs/st299b-wasm-suite.log`
  sha256 `28a0947c0ae1bd9020c2c00945424e8d4cf4cf5efcaff7bee8bd91805c963964`.

## 2. `vis-host-wasm` native + wasm32-wasip1

| Lệnh | Kết quả |
|---|---|
| `cargo test` trong `vis-host-wasm` | **29/29** (17 + 12; 0 failed) |
| `cargo check --target wasm32-wasip1` | **PASS** (7,57 s) |

- Native log: `/home/ps/vislogs/st299b-native.log`
  sha256 `b2550dc6e9b15551447785f2f219355d4932ac8e2d5a4e78c5f739f0b456acdf`.
- Check log: `/home/ps/vislogs/st299b-check-wasi.log`
  sha256 `209619abbdcb73bf224c874a0d9bfd5c92a2de0d4a433ead9eda8129479bc1c7`.

## 3. Tách workspace / unsafe

- `Cargo.toml` gốc vẫn `exclude = ["vis-host-linux", "vis-host-wasm"]`; `vis-host-wasm` có
  `[workspace]` riêng + `Cargo.lock` riêng.
- `vis-host-wasm/src/lib.rs` có `#![forbid(unsafe_code)]`; grep `unsafe` chỉ còn **chú thích**
  (“không cần unsafe”), **0 `unsafe` thực thi**. Không đụng root workspace.

## 4. Blocker

Không. Không sửa code, không push/merge/tag.
