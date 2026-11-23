<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Bộ kiểm trong `test/`

| Thư mục / tệp | Nội dung |
|---|---|
| `std/` | ví dụ + kết quả mong đợi cho thư viện chuẩn (`.vis` + `.out`) |
| `doc-examples/` | khối ví dụ sinh từ `docs/ViScript-thiet-ke.md` (kiểm bằng `vis-dev doc-examples`); tên fixture là **slug ổn định** `doc-<tài-liệu>-<tiêu-đề>-<số-thứ-tự>.<ext>` (#151/ST-289), không theo số dòng |
| `report/` | `check_report.py` + `gen_report_json.py` (sinh `R35`/`R36` từ `QUEUE@head`) + fixture cho báo cáo đợt (`agent/reports/R##-viscript.json`) |
| `wasm/` | `st300-sem-native-baseline.sh` — tái lập baseline interp↔VM native cho `test/sem` (ST-300) |
| `fixtures/` | `run_fixtures.sh` — ca âm cho luật cấu trúc R1–R4 |
| `release/` | `run.sh` — canh artifact phát hành chỉ có `vis` (không `test-child`) |
| `toolchain/` | `run.sh` — pin `rust-version`/rust-toolchain/hạ tầng khớp `Cargo.toml` |
| `check/`, `cmd/`, `expr/`, `fmt/`, `host/`, `modules/`, `pkg/`, `sem/` | fixture theo từng mảng chức năng |
| `cheatsheet/` | doc-test của `CHEAT-SHEET-ViScript.md` — mỗi `.vis` là một chương trình hoàn chỉnh từ ô ✔ (chạy bởi `crates/vis-cli/tests/cheatsheet.rs`) |
| `doc_excluded.rs` | danh sách khối ví dụ **không** chạy (được `include!` từ test) |

Chạy toàn bộ: `./vis-dev gate --fast` (hoặc `cargo test --workspace`).

**Core không SQLite** (`--no-default-features`): dùng `./vis-dev core-test`. Lệnh này build
`vis`/`test-child` core vào **target riêng** (`target/core-test`, đổi bằng `VIS_CORE_TARGET_DIR`)
rồi mới chạy test, nên không giẫm lên `target/debug/vis` của gate default. Chạy
`cargo test -p vis-cli --no-default-features` **một mình** vẫn cần build core trước
(`cargo build -p vis-cli --no-default-features --bin vis --bin test-child`) — nếu không, nó có
thể dùng binary `vis` còn sót từ cấu hình default.

**Host POSIX thật** (Unix): `crates/vis-cli/tests/host_posix_real.rs` chạy `/bin/echo`,
`/usr/bin/grep`, `/bin/sh -c "exit 3"`, `/usr/bin/printf` thật trên cả interp và VM. Nó nằm
trong `cargo test --workspace`, và `bin/vis-host-suite` gọi tường minh trên Unix (Windows bỏ qua,
dùng `test-child`).
