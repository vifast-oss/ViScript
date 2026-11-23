<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Công cụ trong `bin/`

| Tệp | Việc | Ai dùng |
|---|---|---|
| `vis` | chạy chương trình `.vis` (shim gọi `crates/vis-cli`) | mọi người |
| `vis-dev` | cổng phát triển: `check`, `gate --fast`, `core-test`, `nightly`, `doc-examples`, `rules` | người đóng góp |
| `vis-release` | build + stage artifact phát hành **chỉ** `vis` (không `test-child`) | phát hành |
| `vis-diff` | so sánh kết quả hai lần chạy (differential) | kiểm thử |
| `vis-soak` | chạy lặp nhiều vòng để tìm lỗi chập chờn | kiểm thử |
| `vis-lock` | giữ khoá `~/.vifast-measure.lock` khi đo thời gian (macOS không có `flock`) | đo hiệu năng |
| `vis-remote` | chạy lệnh trên máy `mps`/`psvn` qua ssh (máy chủ đạo `ps` chạy tại chỗ), log ngoài repo, đọc mã thoát từ tệp | đo đa nền |
| `vis-linux` | chạy cổng trong container Linux | đo đa nền |
| `vis-targets` | kiểm crate `vis-host-*` theo nền (gồm dòng `wasm32-wasip1 (runtime)` và `aarch64-apple-darwin (native)`) | kiểm thử |
| `vis-host-suite` | bộ kiểm host (Linux/Windows) | kiểm thử |
| `vis-wasm-sem` | chạy bộ ngữ nghĩa `test/sem` trên `wasm32-wasip1` bằng runtime WASI (Node/wasmtime) | kiểm thử |
| `vis-wasm-suite` | smoke suite `vis-host-wasm` trên `wasm32-wasip1` (preopen/quyền/đồng hồ/argv) | kiểm thử |
| `vis-gc-targets` | canh đĩa ≥10/15 GB, dọn `target/` worktree đã merge/cũ (mặc định dry-run) | vận hành |
| `vis-jobs` | liệt kê/kiểm tiến trình nền; `--protect <pid>` khi cố ý giữ | vận hành |
| `vis-status` | bảng điều khiển nhanh HEAD/đĩa/target/job/journal | vận hành |
| `vis-gate-windows` | `--quick`/`--full` chạy cổng Windows qua `vis-remote` | kiểm thử |
| `vis-gate-macos` | `--quick`/`--full`/`--dry-run`/`--self-test` chạy cổng macOS arm64 qua `vis-remote mps` (HEAD guard + `vis-lock`, Python 3.9+) | kiểm thử |
| `_vis-toolchain.sh` | helper nội bộ: chọn cargo/rustup và biến môi trường cho các script khác | nội bộ |

Quy ước: mọi script có header SPDX `#`, log mang mã task (`ST-##`), không ghi đè log cũ.

**Phát hành (VD-259)**: `crates/vis-cli` có bin test-support `test-child`; Cargo không cho ẩn bin
khỏi `cargo build --release`/`cargo install` mà vẫn giữ `CARGO_BIN_EXE_test-child`. Vì vậy **không**
phát hành bằng `cargo build --release` trần. Dùng `./bin/vis-release` (hoặc tường minh
`cargo build --release -p vis-cli --bin vis`); `test/release/run.sh` canh metadata `release-bins`
và chứng minh staging không chứa `test-child`.
