<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-296 test-harness — gate `h2_async_spike` root fixture theo debug

> **Loại**: **test-only** (không sửa sản phẩm/`tls.rs`/security). 2026-11-22.
> **Baseline**: `plan/w3-platform` merge `main` **`708c3a7`**; không push/merge/tag; không GitHub Actions.
> **Mục tiêu**: release workspace **0 failed** khi 8 ca `h2_async_spike` cần
> `VIS_TLS_TEST_ROOT_DER` — hook chỉ có ở bản `debug_assertions` (`net/tls.rs:51-53`).

## 1. Thay đổi (test-only)

`crates/vis-cli/tests/support/h2_async_spike.rs` (+37 dòng):

- Thêm helper `bo_qua_release(ten)`:
  - `cfg!(debug_assertions)` ⇒ `false` (tiếp tục chạy đủ coverage debug);
  - release ⇒ `eprintln!("bỏ qua {ten}: VIS_TLS_TEST_ROOT_DER chỉ có ở bản debug (release không tin cert fixture)")`
    và trả `true`.
- Gate **8 ca** spawn binary `vis` con với root fixture:
  `async_h2_production_bon_stream_mot_connection`, `async_h2_production_timeout_va_goaway`,
  `async_h2_production_cookie_redirect`, `async_h2_production_cap_body`,
  `async_h2_production_bon_stream_cold`, `async_h2_cold_multiplex_20_vong`,
  `async_h2_production_redirect`, `async_h2_soak_env_gated`.
- Cách này khớp `vs3_registry_https.rs` (test TLS/registry debug-only cũng bỏ qua release có lý do).
  **Không** sửa `net/tls.rs`, không mở root test ở release, không đổi product/security.
- Debug vẫn đủ 35 ca (34 pass + 1 ignored); release 8 ca trả về sớm có log lý do, không fail.

## 2. Kết quả

| Lệnh | Kết quả |
|---|---|
| `cargo test --release --workspace --no-fail-fast` | **PASS — 3643 passed / 0 failed / 4 ignored** (358 binary, wall 2:10) |
| `cargo test -p vis-cli --test vs3_http2` (release) | **34 passed / 1 ignored** (8 ca gate early-return) |
| `./vis-dev gate --fast` | **PASS — 3638 passed (1 slow) / 4 skipped** (wall 2:44) |
| `cargo test -p vis-cli --test vs3_http2` (debug) | **34 passed / 1 ignored** (giữ coverage 34+1) |
| `cargo fmt --all -- --check` | **OK** |

Log:
- `/tmp/opencode/release-h2-harness-release2.log` (release workspace, 0 failed)
- `/tmp/opencode/release-h2-harness-gate2.log` (gate 3638/4)

Không có thay đổi sản phẩm; không đụng `tls.rs`/security. Đĩa cuối **21 G** (> 10 G).

## 3. Blocker

Không còn blocker release-harness: release workspace **0 failed**, gate **3638/4**, debug h2 **34+1**
giữ nguyên. Không tag/push/merge; không GitHub Actions.
