# agent/ACTIVE.md — Bảng điều khiển tác chiến hiện hành (`VD-289`, 2026-11-24)

> **Toàn văn quyết định & sơ đồ Kiến trúc Đích 5 tầng DAG**: `agent/JOURNAL/quyet-dinh-a0-2026-11-24-vd289.md` · `agent/DECISIONS.md` (`VD-289`).

## 1. CHẾ ĐỘ HIỆN HÀNH — PHÁT TRIỂN KIẾN TRÚC TỐI ƯU (`VD-289`)

- **HỦY / TẠM DỪNG VÔ THỜI HẠN** toàn bộ kế hoạch phát hành cũ (`ST-309` → `BLOCKED`, **không** soak 24h/4h, **không** fuzz 8h, **không** 3× full lặp lại, **không** `R37` JSON, **không** tag `v0.6.0`/`v1.0.0-rc1`).
- **ĐƯỢC PHÉP DỪNG** các tiến trình soak/fuzz dài ngày của `H4` đang chạy nền (nếu có) để giải phóng máy cho tái kiến trúc.
- **Mục tiêu duy nhất hiện tại**: đưa ViScript về **Kiến trúc Đích 5 Tầng DAG** (`vis-hir`, `vis-core` mỏng, `vis-http`, `vis-runtime` tách khỏi `vis-interp`, `vis-vm` 0 dep `vis-interp`/`vis-types`, `vis-pkg` 0 dep `vis-host-native`, `vis-host-linux` vào workspace, stdlib 1 nguồn sự thật `StdId` + chia miền, `vis-conformance` dữ liệu).
- **Nền đã xanh**: `ST-311a..f` DONE (`#190`/`#194` đóng, `#195` đã vá mã), `./vis-dev rules` R1..R9 PASS, 0 `unsafe` (R2).
- **Tiến độ 2026-11-24 (Sóng 0–1 xong, chờ A0 duyệt mở Sóng 2)**: kho sạch (2 nhánh local, 1 worktree); `ST-312a`/`ST-312b`/`ST-312c`/`ST-312d` = `REVIEW` với commit `65ea2a0`/`02e881e`/`cd007f9`/`4e6f810`. Kết quả: 9 shim `vis_interp→vis_core` đã xóa; `vis-host-linux` vào workspace (21 crate); `crates/vis-hir` tách xong (`vis-core`/`vis-bytecode`/`vis-vm` **0 dep** `vis-types`); `crates/vis-http` tách xong (`vis-core` 0 HTTP protocol; `vis-pkg` 0 dep `vis-host-native`, tiêm `NetHost` + mock test); đóng `#196`.

## 2. Chương trình Tái kiến trúc 4 Sóng (`ST-312a..ST-313d` — Chạy song song `W1..W4`)

| Sóng | Lát | Luồng | Trạng thái | Mục tiêu & KPI |
|---|---|---|---|---|
| **0** | Dọn nhánh (`VD-288` §4) | **W4** | `DONE` | Dọn Đợt 1–3 (83 nhánh → `main` + `st309-qa`; worktree ≤ 8) + `bin/vis-gc-targets` |
| **0** | **`ST-312a`** | **W1** | `REVIEW` | Xóa 9 shim re-export `vis_interp → vis_core`; chuyển `Abort`/`Trap`/`R`/`RunOutput`/`FrameConfig`/`FrameOutput` về `vis-core` |
| **0** | **`ST-312b`** | **W4** | `REVIEW` | Đưa `vis-host-linux` vào workspace chính (1 `Cargo.lock`, `version.workspace = true`); gộp `clippy -D warnings` vào `bin/vis-gate-windows --full`; vá `#196` (`parse_duration`) |
| **1** | **`ST-312c`** | **W3** | `REVIEW` | Tách **`crates/vis-hir`** (dữ liệu HIR/Type/`TypeTable`/bảng `stdlib`) khỏi `vis-types` → `vis-core`/`vis-bytecode`/`vis-vm` **0 dep `vis-types`** |
| **1** | **`ST-312d`** | **W2** | `REVIEW` | Tách **`crates/vis-http`** (sans-IO HTTP/1.1 + cookie + async state machine) khỏi `vis-core`/`vis-interp`; cắt `vis-pkg → vis-host-native` (tiêm `NetHost` + mock test) |
| **2** | **`ST-313a`** | **W1+W3** | `BLOCKED` | Tách **`crates/vis-runtime`** khỏi `vis-interp` (`WorkerRunner` trait); cắt **`vis-vm → vis-interp`**; giữ `vis-interp` ~8,6k; chia `vm_frame.rs` < 1.000 LOC |
| **2** | **`ST-313b`** | **W2+W4** | `BLOCKED` | 1 nguồn sự thật stdlib (**`StdId`** O(1) + test song ánh 1–1 `FUNCS` ↔ dispatch) + tổ chức `stdlib/` theo miền (xóa hết 20 tệp `_a.._e`) |
| **3** | **`ST-313c`** | **W1+W2** | `BLOCKED` | Dời test API thư viện khỏi `vis-cli/tests/` về crate chủ + dựng **`crates/vis-conformance`** (dữ liệu `.vis`/`.out`/`.err` chạy tự động `interp ↔ VM` + WASI) |
| **3** | **`ST-313d`** | **W3+W4** | `BLOCKED` | Chuyển `lang_it`/`stdlib_it` sang `test/conformance/` (0 mất assertion), cập nhật R8 nếu cần, đánh giá đóng `#193` (WASI interp trên `vis-interp` mỏng) |

## 3. Cổng Kiểm tra Gọn (Chế độ Phát triển — KHÔNG soak/fuzz nặng)

- **Cuối mỗi lát**: `./vis-dev gate --fast` PASS + `VIS_DIFF_N=100000 ./bin/vis-diff` (`lech=0` cho lát chạm compiler/runtime/VM).
- **Cuối mỗi Sóng**: 1× `cargo clippy --workspace --all-targets -- -D warnings` trên 3 OS (`ps`/`psvn`/`mps`) + 1× `bin/vis-targets` + 1× WASI smoke (`vis-wasm-suite` / `vis-wasm-sem`).
- **Luật cứng giữ nguyên**: 0 `unsafe` (R2), R1 ≤ 1.500 (mục tiêu mềm ≤ 1.150), R5 (`SO-NO`), R7 (log ≤ 50 KB), R8/R9, không `.github/workflows/`.
- **Quản trị nhánh (`AGENTS.md` §6)**: merge xong xóa worktree + nhánh local (`-d`) + nhánh `origin` trong 24h; giữ **≤ 8 worktree** hoạt động; `CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0`, đĩa trống > 10 GB.

## 4. Lịch Tác chiến theo Khung giờ DeepSeek (`VD-287`, chuẩn `AGENTS.md` §5)

| Khung giờ | UTC | JST (UTC+9) | ICT (UTC+7) | Tác chiến ưu tiên (Tái kiến trúc `ST-312a..ST-313d`) |
|---|---|---|---|---|
| **Cao điểm 1 & 2** *(2×)* | `01:00–04:00` · `06:00–10:00` | `10:00–13:00` · `15:00–19:00` | `08:00–11:00` · `13:00–17:00` | **0-LLM**: chạy kiểm Sóng (`vis-diff 100k`, `clippy` 3 OS, `vis-targets`, WASI smoke, `vis-gc-targets` + dọn nhánh) hoặc nghỉ gọi LLM |
| **Thấp điểm trưa** *(0,5×)* | `04:00–06:00` | `13:00–15:00` | `11:00–13:00` | Đọc kết quả cổng Sóng, vá lỗi build/import ngắn, merge nhánh gọn, dọn worktree |
| **Thấp điểm chính & Cuối tuần** *(0,5×)* | `10:00–01:00` (+ T7–CN) | `19:00–10:00` | `17:00–08:00` | **Phát triển & Tái kiến trúc song song `W1..W4`**: chạy Sóng 0–1 (`ST-312a..d`) → Sóng 2 (`ST-313a..b`) → Sóng 3 (`ST-313c..d`) |
