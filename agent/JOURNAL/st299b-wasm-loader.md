<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-299b — nối `vis-loader` vào `vis-wasm-run` (đa tệp + `use`) 

> **Người làm**: subagent VS3-W2, 2026-11-23. **Nhánh** `plan/w2-st299b-wasm-loader` từ
> **`f4cb136`** (v0.4.1 + ST-297). Không push/merge/tag; chờ review/QA sau ST-298.
> `CARGO_BUILD_JOBS=3`, `TMPDIR=/home/ps/tmp`.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| `vis-loader` trong `vis-wasm-run` | **có** — `chay_tep` dùng `modules::load_full`, `use ./x` đa tệp |
| 6 crate lõi | `vis-syntax`, `vis-types`, `vis-bytecode`, `vis-interp`, `vis-vm`, **`vis-loader`** (+ `vis-diag`) |
| Native tests | **29/29** (`vis-host-wasm`), gồm đa tệp relative `use` interp/VM + thiếu tệp rõ |
| WASM suite | **28/28** lượt (14 ca × interp/VM, thêm `da_tep`) — runtime **node:wasi** |
| `cargo check --manifest-path vis-host-wasm --target wasm32-wasip1` | **PASS** |
| `cargo test -p vis-loader` (root) | **29/29** |
| `cargo check --workspace` root | **PASS** |
| Gate Linux | **3660 passed / 0 failed / 4 skipped** |
| fmt / clippy / R1..R7 | **PASS** |
| Diff 100k | **lech=0** (seed `20261122`, 70,4 s) |
| Blocker | **không** |

## 1. Thay đổi

- `vis-host-wasm/Cargo.toml`: thêm `vis-loader = { path = "../crates/vis-loader" }` (workspace
  `vis-host-wasm` vẫn tách/exclude khỏi root).
- `src/runner.rs`:
  - tách `chay_hir` (frontend đã xong → host WASI → interp/VM) dùng chung;
  - `chay_tep(tep, engine, args, preopens)` gọi `vis_loader::modules::load_full` (đa tệp,
    chẩn đoán mang `rel` tên tệp, lỗi thiếu tệp ⇒ exit 2 rõ);
  - `chay` một nguồn/stdin giữ nguyên API cho test cũ.
- `src/bin/vis-wasm-run.rs`: tệp chính đi `chay_tep`; `--stdin` vẫn `chay`; usage ghi đa tệp.
- `bin/vis-wasm-suite`: thêm ca `da_tep` (`use ./util`, `pub fn cong`) ×2 engine.
- `tests/wasi_fs.rs`: thêm 2 test native đa tệp (interp/VM) + thiếu module; README cập nhật 29/28.
- Không đổi API/ABI/cache root; `vis-host-wasm` vẫn `exclude`; `WasiHost` vẫn từ chối
  net/proc/sqlite, FS preopen/clock/stdout; cả hai crate `#![forbid(unsafe_code)]`.

## 2. Native

- `cargo test --manifest-path vis-host-wasm/Cargo.toml`: **17 unit + 12 integration = 29/29**.
- Ca mới: `da_tep_relative_use_ca_hai_engine` (main `use ./util`, `cong(4,6)` ⇒ `10`),
  `da_tep_thieu_tep_bao_ro` (exit 2, stderr nêu tệp thiếu).

## 3. WASM

- `cargo check --manifest-path vis-host-wasm/Cargo.toml --target wasm32-wasip1`: **PASS**
  (vis-loader biên dịch cho WASI; không thêm dependency ngoài crate lõi).
- `bin/vis-wasm-suite --build --runtime node`: build wasm **3 648 118 B**; **28/28** lượt
  (14 ca × interp/VM) — gồm `da_tep` cả hai engine. Không có wasmtime CLI/Python, dùng Node
  built-in `node:wasi` (không tải binary lạ).
- `WasiHost` giữ nguyên: FS preopen, clock, stdout/stderr; net/proc/sqlite vẫn `CHAN`.

## 4. Root / cổng

- `cargo test -p vis-loader`: **29/29**; `cargo check --workspace`: PASS.
- `./vis-dev gate --fast`: **PASS — 3660 passed / 0 failed / 4 skipped** (72,9 s).
- fmt root + `vis-host-wasm`; clippy `vis-host-wasm --all-targets -D warnings`: PASS.
- Diff 100k seed `20261122`: `lech=0` (93 770 ca chạy, 70,4 s).
- Log ngoài repo (R7):
  - `/home/ps/vislogs/st299b-gate.log` — sha256 `1eb23dff82bbd6f500b56c40f1d6654871697e7ddafce4013da808bed33cbf9f`.
  - `/home/ps/vislogs/st299b-wasm-suite.log` — sha256 `1b33f4cf5e0671c2f2a07a5d03389c1975b11afa9a57857f93de06e17d1dc0d7`.
  - `/home/ps/vislogs/st299b-diff100k.log` — sha256 `e79e7349ac272cdda7be0a5a9487b0ebd7ca31f6522d4d3311d689fe393dc920`.

## 5. Kết luận

- ST-299b đạt: `vis-wasm-run` chạy **đa tệp + `use`** trên `wasm32-wasip1` (Node WASI) và
  native, giữ capability và 0 unsafe; root không đổi API/ABI/cache.
- **Không blocker**; **chưa merge** — chờ review/QA sau ST-298.
