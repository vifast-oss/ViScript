<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Bộ kiểm trong `test/`

| Thư mục / tệp | Nội dung |
|---|---|
| `std/` | ví dụ + kết quả mong đợi cho thư viện chuẩn (`.vis` + `.out`) |
| `doc-examples/` | khối ví dụ sinh từ `docs/ViScript-thiet-ke.md` (kiểm bằng `vis-dev doc-examples`) |
| `report/` | `check_report.py` + fixture cho báo cáo đợt (`agent/reports/R##-viscript.json`) |
| `fixtures/` | `run_fixtures.sh` — ca âm cho luật cấu trúc R1–R4 |
| `check/`, `cmd/`, `expr/`, `fmt/`, `host/`, `modules/`, `pkg/`, `sem/` | fixture theo từng mảng chức năng |
| `doc_excluded.rs` | danh sách khối ví dụ **không** chạy (được `include!` từ test) |

Chạy toàn bộ: `./vis-dev gate --fast` (hoặc `cargo test --workspace`).
