<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 hardening — release-mode `14310f4`

> **Người chạy**: W3 (QA/hardening), 2026-11-21 20:16–20:22 JST.
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| `main`/HEAD | **`14310f4`** (merge W3m 3× soak; v0.3.2 tagged) |
| Code | v0.3.2 `5c13741`; không đổi trong phiên |
| Target | `/home/ps/ViScript/target/qa-w3-release` (riêng, release profile) |
| Env | `CARGO_INCREMENTAL=0` (release mặc định `debug=0`) |
| Toolchain | `~/.cargo/bin/cargo` 1.100.0 |
| Disk | đầu **19G**; trong/cuối **18G** (> 5G) |

## 1. Release workspace test

`cargo test --release --workspace --no-fail-fast`:

| Chỉ số | Giá trị |
|---|---|
| **Passed** | **3533** |
| Failed | **0** |
| Ignored | **3** |
| Test binary | **341** |
| Thời gian | **367,72 s** |
| Log | `/tmp/opencode/qa-w3-release-mode.log` |

- Debug gate cùng cây: **3528/3**; release có thêm 5 doctest/target tổng nên 3533 pass.
- Không có `test result: FAILED`, không panic, exit 0.
- **Ca tự bỏ qua trong release**: 9 chỗ `if !cfg!(debug_assertions)` trong test (registry HTTPS/TLS/
  async HTTPS dùng hook root chỉ có ở debug) — chủ động return, **không** phải fail. Không tìm thấy
  ca nào **chỉ chạy** ở release (`cfg(not(debug_assertions))` chỉ để bỏ qua, không có nhánh riêng).

## 2. Host suite trên binary release

`bin/vis-host-suite` **không có cờ `--release`** (script gọi `cargo test` debug). Đã chạy **hai test
host tương đương** bằng binary release:

| Lệnh | Kết quả | Thời gian | Log |
|---|---|---|---|
| `cargo test --release -p vis-cli --test host_conformance -- --nocapture` | **54/54 ok** | 0,60 s | `/tmp/opencode/qa-w3-release-host.log` |
| `cargo test --release -p vis-cli --test host_posix_real -- --nocapture` | **PASS** | 0,24 s | cùng log |

- 54 dòng `ok N - …`, không `not ok`.
- `host_posix_real` PASS (Linux x86_64).

## 3. Đánh giá

- **Release-mode xanh**: workspace **3533 passed / 0 failed / 3 ignored**, host 54/54 + POSIX.
- Không thấy khác biệt hành vi release vs debug; 9 ca skip release là **có chủ đích** (hook root).
- Không cần sửa code; đây là bằng chứng hardening sau `v0.3.2`.

## 4. Blocker / còn lại

1. Không có blocker local.
2. `bin/vis-host-suite` không hỗ trợ `--release`; muốn một lệnh host release chuẩn cần thêm cờ (việc
   script, không sửa trong phiên này).
3. Windows psvn release-mode chưa đo; Windows v0.3.2 đã verify debug 3462/0 theo tag.
4. Không GitHub Actions; local only. 2 ô `EXPECTED_RED` msvc/wasm #156 không liên quan release Linux.
5. Tàn dư đã biết: worker evaluator, `select` VS3-2c, async HTTP Unix — release notes.

## 5. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR=/home/ps/ViScript/target/qa-w3-release
export CARGO_INCREMENTAL=0

cargo test --release --workspace --no-fail-fast
cargo test --release -p vis-cli --test host_conformance -- --nocapture
cargo test --release -p vis-cli --test host_posix_real -- --nocapture
```

**Kết luận**: release-mode `14310f4` **PASS** — workspace **3533/0/3**, host **54/54** + POSIX.
Không có blocker kỹ thuật local; Windows release-mode và việc thêm cờ `--release` cho host suite là
việc còn lại.
