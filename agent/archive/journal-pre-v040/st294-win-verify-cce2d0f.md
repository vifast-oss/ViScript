<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-294 — verify merge `cce2d0f` trên Windows (`psvn`)

> **Người đo**: subagent VS3-W2, 2026-11-22. **Mốc**: `cce2d0f` merge ST-294 (async-h2 Windows +
> default non-wasm). `psvn` `/c/Users/ps/viscript-moi` detached `cce2d0f`, **dirty 0**;
> `git describe --tags --always` = `v0.3.6-9-gcce2d0fb`. Target riêng
> `/c/Users/ps/vt-st294-verify`. Không tag/push/merge; không GitHub Actions.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| HEAD / phiên bản | **`cce2d0fb`**; `vis 0.3.6` (debug + release) |
| Workspace | **355 block; 3563 passed / 0 failed / 3 ignored** (rc 0, dirty 0) |
| `vs3_http2` | **33/33** (Windows; h2 async default-on) |
| `vs3_http_cancel` | **8/8** |
| `vs3_default` / `vs3_tls` / `net` | **22/22**, **22/22**, **9/9** |
| Check | default + `--no-default-features` + `--features async-h2`: **PASS, 0 warning** |
| Host | **54/54** |
| Targets | core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED** |
| Release | `vis.exe` **11 257 344 B**, sha256 `812b9b81…38f63f4`; CLI/TAP/smoke xanh |
| Blocker | **không** |

## 1. Môi trường

- Windows `MINGW64_NT-10.0-26300`, 6 CPU; cargo/rustc **1.100.0** (MSVC).
- `git fetch origin main` → checkout `--detach cce2d0f`; `git status --porcelain` **rỗng**
  trước/sau đo.
- `CARGO_TARGET_DIR=/c/Users/ps/vt-st294-verify`.

## 2. Workspace (1 vòng)

| Lệnh | Kết quả |
|---|---|
| `cargo test --workspace --no-fail-fast` | **356 block; 3563 passed / 0 failed / 3 ignored**, rc 0 |
| `cargo build -p vis-cli --bin vis` + `vis --version` | `vis 0.3.6` |

Kỳ vọng **3563/0/3** đạt; +25 so mốc 3538 trước ST-294 là các ca h2 async nay chạy mặc định
(Windows `vs3_http2` 33 thay 19, `vs3_http_cancel` 8 thay 5).

## 3. Targeted

| Suite | Kết quả |
|---|---|
| `vs3_http2` | **33/33** (gồm cold-start 4 stream/1 conn, cold multiplex 20 vòng, timeout/GOAWAY/cancel h2) |
| `vs3_http_cancel` | **8/8** |
| `vs3_default` | **22/22** |
| `vs3_tls` | **22/22** (Windows có thêm 1 ca `#[cfg(windows)]` so Linux 21) |
| `net` | **9/9** |

## 4. Check / host / targets

- `cargo check -p vis-cli`: **PASS**, `warning:` **0**.
- `cargo check -p vis-cli --no-default-features`: **PASS**, `warning:` **0**.
- `cargo check -p vis-cli --features async-h2`: **PASS** (rc 0; async-h2 không còn expected-red).
- `bin/vis-host-suite`: **54/54 ca xanh** (`MINGW64_NT-10.0-26300 x86_64`;
  `host_posix_real` bỏ qua trên Windows).
- `bin/vis-targets`: core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED** (`#156`: libsqlite3-sys /
  ring cần C toolchain cho wasm32); script exit 0 đúng kỳ vọng.

## 5. Release / CLI

- `cargo build --release -p vis-cli --bin vis`: rc 0.
- `target/release/vis.exe` — **11 257 344 B**;
  sha256 `812b9b81bc77778660d0e4bb525a55496fa6c91d54385a10688fce08d38f63f4`.
- `vis --version` → `vis 0.3.6`; `vis doctor` → `os windows`, `arch x86_64`, `sqlite on`,
  `features sqlite,tls,pkg-sig,http2`.
- Smoke: `vis run` file `print(42)` → `42`, rc 0; `vis check` TAP `1..1`, `ok 1`, rc 0;
  `vis run --version` → **rc 2** (đúng hành vi CLI, không phải lỗi).
- `git status` sau đo: **dirty 0**.

## 6. Kết luận

- Merge `cce2d0f` **xác nhận xanh trên Windows**: async-h2 default non-wasm chạy thật (không còn
  `E0433 net::h2_async`), workspace đúng kỳ vọng, host/targets/release/CLI xanh.
- **Không blocker**. Không tag/push/merge trong phiên này.
- Ghi chú tồn từ trước ST-294: soak 2h trên tag `v0.3.6` fail cycle 31 `http2-h2` rc101 (chưa
  triage; không rerun theo chỉ đạo) — cần đưa vào soak/QA GĐ1.
