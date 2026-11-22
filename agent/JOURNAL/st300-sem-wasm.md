<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-300 — harness `test/sem` trên WASI (interp + VM) so oracle native

> **Người làm**: subagent VS3-W2, 2026-11-23. **Nhánh** `plan/w2-st300-wasm-suite` từ
> **`16fba3a`** (v0.4.1 + ST-299b). Không push/merge/tag; chờ review/QA.
> `CARGO_BUILD_JOBS=3`, `TMPDIR=/home/ps/tmp`.
> **Oracle native W1**: `/home/ps/vislogs/sem-native-baseline.tsv`, sha256
> `482b35c276d76fca9200770f78b3cba891535b09b65dbce0fc52bce6379a308d` (475/475, parity
> interp↔VM). Không chạy lại native.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Tổng `test/sem` | **525** = **475 `.vis`** + 50 `.vish` |
| Eligible WASI | **390/475 `.vis`** |
| Loại trừ | **135** = 38 `sqlite` + 26 `async` + 15 `kenh` + 6 `cap` + 50 `.vish` |
| WASM pass | **388/390 — coverage 99,5%** (mục tiêu ≥85%) |
| Fail còn lại | **2** ca đệ quy sâu: interp WASI stack overflow (VM pass) |
| Runtime | **Node `node:wasi`** (không có wasmtime CLI/Python) |
| Suite cũ | **28/28** |
| `vis-wasm-run` wasm32-wasip1 check | **PASS** |
| Native `vis-host-wasm` | **29/29** |
| Gate root | **3660 passed / 0 failed / 4 skipped** |
| fmt/clippy/R1..R7 | **PASS** |
| Blocker | **không** (2 fail là giới hạn stack interp, đã ghi rõ) |

## 1. Phân loại `test/sem`

- **Chạy được trên WASI (eligible)**: ngôn ngữ + stdlib lõi, không cần host ngoài; **390 tệp**.
- **Loại trừ có lý do**:
  - `sqlite/` **38**: feature/host SQLite không có trên WASI.
  - `async/` **26** + `kenh/` **15**: async/spawn/chan chưa có backend scheduler WASI.
  - `cap/` **6**: cap/host quyền ngoài chương trình thuần WASI.
  - `.vish` **50**: command/library (không phải entrypoint script; `vish` cần proc/nền).
  - Quét thêm `use std.net/proc/sqlite/fs`, `use ./`, `pkg.`, `chan[`, `spawn`, `await`, `cap[`:
    không còn tệp eligible nào vướng (0 hit ngoài nhóm trên).
- Không loại trừ vì “không có entrypoint” hay timeout: oracle W1 xác nhận 475/475 chạy native.

## 2. Harness mới `bin/vis-wasm-sem`

- Mode riêng (không đổi `bin/vis-wasm-suite` cũ):
  - đọc oracle TSV W1 (path/category/interp_rc/interp_sha/vm_rc/vm_sha);
  - mỗi tệp eligible chạy `vis-wasm-run --interp` và `--vm` trên Node WASI;
  - so **rc + sha256(stdout)** với oracle tương ứng, in bảng
    `file, category, native_rc, wasm_interp_rc, wasm_vm_rc, stdout_sha256, status`;
  - in lý do EXCLUDE; summary coverage; `--require-target` để exit 1 nếu <85%.
- Runtime auto: wasmtime CLI → Python wasmtime → Node; log ghi runtime đã chọn.
- Không chạy lại native (theo W1 oracle); oracle sha được kiểm và in.

## 3. Kết quả WASM

- `# sem total=525 eligible=390 excluded=135 runtime=node
  oracle_sha256=482b35c2…a308d`
- **pass=388, fail=2, coverage=99,5%**.
- Fail:
  1. `test/sem/ham/de-quy-1000.vis` — interp wasm `RuntimeError: memory access out of bounds`
     (stack), VM **pass** rc0/hash khớp.
  2. `test/sem/ham/de-quy-vo-han.vis` — cùng stack overflow interp; VM **pass** rc101/hash khớp.
  - Không sửa/che: đây là giới hạn độ sâu stack của interp trên `wasm32-wasip1`; coverage vẫn
    vượt ngưỡng. Đề xuất task riêng nếu muốn tăng stack/tail-call.

## 4. Giữ cổng cũ

- `bin/vis-wasm-suite --runtime node`: **28/28** (14 ca × interp/VM).
- `cargo test --manifest-path vis-host-wasm/Cargo.toml`: **17 unit + 12 integration = 29/29**.
- `cargo check --manifest-path vis-host-wasm/Cargo.toml --target wasm32-wasip1`: **PASS**.
- `./vis-dev gate --fast`: **PASS — 3660 passed / 0 failed / 4 skipped** (108 s); fmt/clippy/R1..R7.
- Root API/ABI/cache không đổi; chỉ thêm script harness.

## 5. Log ngoài repo (R7)

- `/home/ps/vislogs/st300-sem.log` (59 KB) — sha256
  `0d4feb6e4b5b3509994e528e0c9b3b85e1a3808dbb955a6f9dffc56fa55a79fa`.
- `/home/ps/vislogs/st300-gate.log` (365 KB) — sha256
  `026e90cf48d1020ea9dbb40deffe3e4b5761cf09a98d49ed82988accc11554c4`.
- `/home/ps/vislogs/st300-suite28.log` — sha256
  `5bf98b79ceb1afc34aa0890b95e68a7783016f64fcc963561ba2db0076e0c089`.

## 6. Kết luận

- Đạt mục tiêu harness: **388/390 eligible (99,5%)** trên WASI interp+VM, so byte/exit với oracle
  native; loại trừ 135 tệp có lý do, không che fail.
- **Không blocker**; **không merge/push/tag**.
