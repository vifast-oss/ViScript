# ST-310e — Tái phân bổ & gom `vis-cli/tests/` (27 → 8 binary) + cổng chống phình

> **Nguồn**: `main` **`e9c62cb`**; nhánh **`plan/st310e-tests`**. **Local only** — không merge,
> không push, không tag trước `v0.5.0`. Không ssh `mps`/`psvn`.
>
> **Trạng thái phụ thuộc**: `ST-310a`/`ST-310c`/`ST-310d` **chưa chạy** trên `e9c62cb`, nên tên
> crate `vis-plugin-host` chưa tồn tại (xem §4 blocker). Phần còn lại của ST-310e độc lập và
> đã chạy xanh trên cây hiện tại.

## 1. Kết quả đo

| Hạng mục | Trước | Sau |
|---|---|---|
| Binary integration-test cấp 1 của `vis-cli/tests/` | **27** | **8** |
| Tổng test toàn workspace (`gate --fast`) | **3754 passed / 4 skipped** | **3754 passed / 4 skipped** |
| Test bị mất | — | **0** (đối chiếu tên/số ca theo từng suite) |
| Luật `bin/vis-dev` | R1..R7 | **R1..R9** (thêm R8, R9) |

`vis-cli/tests/` còn 8 binary: `async_it`, `cli_it`, `host_conformance`, `host_it`, `lang_it`,
`net_pkg_it`, `stdlib_it`, `worker_it`.

## 2. Việc đã làm

### 2.1. Gộp trong `vis-cli`

- 4 binary `vd283_lambda_annot.rs`, `vd284_async_impl.rs`, `vd284_derives_json.rs`,
  `vd284_spawn_method.rs` → `git mv` vào `tests/lang_it/`, thêm 4 `mod` trong `lang_it.rs`
  (giữ nguyên `#[path]`, không copy test).
- 6 binary `vs3_worker.rs`, `vs3_worker_eval_{local,parallel,vm}.rs`,
  `vs3_worker_frame{,_vm}.rs` → `tests/worker_it/` + wrapper `worker_it.rs`.
- `vs3_worker_soak.rs` → `examples/vs3_worker_soak.rs`; thân soak thành hàm `pub`, `main()` gọi
  hàm đó. `worker_it.rs` include lại chính tệp example và giữ **một ca gate**
  `soak_mixed_shared_copy_move_w1_w4` — nhờ vậy tổng test không đổi (baseline 3754).
- Cập nhật lệnh soak trong doc-comment: `cargo run --release -p vis-cli --example vs3_worker_soak`.

### 2.2. Chuyển suite về crate chủ

| Suite | Đích | Cách xử lý |
|---|---|---|
| `lsp_it` (16 tệp) | `vis-lsp/tests/` | giữ wrapper + `#[path]`; `env!("CARGO_BIN_EXE_vis")` → `crate::common::vis_bin()` |
| `sig_it` + `tls_it/vs3_registry_https` | `vis-pkg/tests/sig_it/` | gộp vào một binary; `vis_cli::pkg_sig` → `vis_pkg::pkg_sig`; giữ `cfg(feature = "pkg-sig")` |
| `cache_khong_che` | `vis-loader/tests/` | `common::vis_bin()`; `bin/vis-soak` đổi `-p vis-loader` |
| `vs3_plugin` (CLI) | `vis-plugin-abi/tests/vs3_plugin_cli.rs` | **tạm** tại crate plugin hiện có; xem §4 |
| `sqlite_it` (20 tệp) | `vis-host-native/tests/` | `required-features = ["sqlite"]` |
| `tls_it/vs3_tls` | `vis-host-native/tests/` | `required-features = ["tls"]`; fixture `tests/fixtures/vs3_tls` chuyển theo |
| `http2_it` + `support/h2_async_spike` | `vis-host-native/tests/` | `required-features = ["http2"]`; `vis_cli::net` → `vis_host_native::net` |
| `security_it` (12 tệp) | `vis-host-native/tests/` | `test-child` qua `common::test_child_bin()` / `TcBin` |
| `host_posix_real` | `vis-host-native/tests/` | `bin/vis-host-suite` đổi `-p vis-host-native` |

- Thêm `test/support/vis_test_support.rs`: tìm `vis`/`test-child` trong target workspace
  (`VIS_BIN`/`VIS_TEST_CHILD` → `CARGO_BIN_EXE_*` → `target/{debug,release}` kể cả target triple
  và `CARGO_TARGET_DIR`), cùng `TcBin` giữ nguyên mọi cách dùng `TC` cũ.
- Mỗi crate nhận helper `tests/common/mod.rs` trỏ tới `test/support/vis_test_support.rs`.
- `[[test]]` Cargo khai đúng ở `vis-host-native` (sqlite/tls/http2/security/host_posix_real) và
  `vis-pkg` (`sig_it`: `pkg-sig` + `tls`). `vis-cli` bỏ 4 khai báo đã chuyển.
- Fixture TLS được giữ ở cả `vis-host-native/tests/fixtures/` và `vis-pkg/tests/fixtures/`
  (vis-cli vẫn cần cho `net_pkg_it`); không đổi nội dung.
- `bin/vis-gate-windows` quick: `http2_it` chạy `-p vis-host-native --features http2`;
  `test/README.md` cập nhật đường dẫn + hướng dẫn chạy riêng package.

### 2.3. Cổng mới trong `bin/vis-dev`

- **R8**: đếm tệp `.rs` cấp 1 trong `crates/vis-cli/tests/`; **> 15 ⇒ đỏ**. KPI ST-310e là ≤ 12,
  ngưỡng cổng 15 theo `VD-285`.
- **R9**: quét toàn bộ `crates/**/*.rs` (bỏ chú thích `//`), cấm `openat2` ngoài
  `crates/vis-host-native/**`; danh sách ngoại lệ đóng băng
  `test/gate/r9-openat2-ngoai-le.txt` (hiện 1 mục `cap_fs.rs`, chờ ST-310c).
- `test/fixtures/run_fixtures.sh`: thêm ca âm 16 (R8) & 17 (R9) — **17/17 PASS**.
- Header/`do_check` đổi nhãn `R1..R7` → `R1..R9`.

## 3. Cổng chạy

| Bước | Kết quả |
|---|---|
| `cargo fmt --all -- --check` | **PASS** (trong `vis-dev check`) |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** |
| `./vis-dev rules` | **PASS** — R1..R9 |
| `bash test/fixtures/run_fixtures.sh` | **PASS** — 17/17 |
| `./vis-dev gate --fast` | **PASS** — **3754 passed / 0 failed / 4 skipped** (79,1 s); check + L0 + doc-test xanh |
| Log | `/home/ps/tmp/st310e-final-gate.log` — sha256 `998c98b9…e31c77` |

## 4. Blocker / nợ chuyển tiếp

1. **`vis-plugin-host` chưa tồn tại** — `ST-310c` chưa chạy trên `e9c62cb`. Suite CLI
   `vs3_plugin` đã rời `vis-cli` nhưng tạm nằm ở `crates/vis-plugin-abi/tests/vs3_plugin_cli.rs`
   (crate plugin hiện có, đang chứa `wasi.rs` host). Khi `ST-310c` tạo `crates/vis-plugin-host`,
   `git mv` tệp này sang `crates/vis-plugin-host/tests/` và giữ `required-features`/helper tương ứng.
2. **`openat2` còn 1 bản sao hợp lệ tạm thời**: `crates/vis-plugin-abi/src/cap_fs.rs`. R9 dùng
   allowlist đóng băng để không đỏ oan; **ST-310c xoá `cap_fs.rs` thì làm rỗng allowlist** (khi đó
   R9 chặn tuyệt đối mọi bản sao ngoài `vis-host-native`).
3. **`ST-310a`/`ST-310d` chưa chạy**: chưa có `vis-core`; `vis-cli/Cargo.toml` vẫn còn 7 dep
   OS/TLS/SQLite trực tiếp. Việc chuyển test không phụ thuộc hai task này; nhánh này chỉ chứa
   ST-310e và sẽ rebase sau khi Pha B mở.
4. Chưa merge/push/tag — đúng chỉ đạo trước `v0.5.0`.
