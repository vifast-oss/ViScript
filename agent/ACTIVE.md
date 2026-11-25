# agent/ACTIVE.md — Bảng điều khiển tác chiến hiện hành (`VD-289` / `VD-290`, 2026-11-25)

> **Quyết định mới nhất**: `agent/JOURNAL/quyet-dinh-a0-2026-11-25-vd290.md` · `agent/DECISIONS.md` (`VD-290`) · Kiến trúc Đích 5 tầng DAG: `agent/JOURNAL/quyet-dinh-a0-2026-11-24-vd289.md`.

## 1. CHẾ ĐỘ HIỆN HÀNH — PHÁT TRIỂN KIẾN TRÚC TỐI ƯU (`VD-289` / `VD-290`)

- **TIẾP TỤC TẠM DỪNG VÔ THỜI HẠN** kế hoạch phát hành cũ (`ST-309` = `BLOCKED`, **không** soak 24h/4h, **không** fuzz 8h, **không** 3× full lặp lại, **không** `R37` JSON, **không** tag mới).
- **ĐÃ DUYỆT `DONE` (`VD-290`, 2026-11-25)**: toàn bộ 4 Sóng tái kiến trúc 5 tầng DAG (**`ST-312a..ST-313d`**, code freeze **`90fd24de`**) — 23 crate, `./bin/vis-dag-check` PASS, `./vis-dev rules` R1..R9 PASS, 0 `unsafe` (R2), `vis-interp` **8.792 LOC**, `vm_frame.rs` **952 LOC**, `StdId` 167 hàm + `stdlib/` chia miền (0 tệp `_a.._e`), `vis-conformance` **420 ca** + **384 test API** dời khỏi `vis-cli/tests`, `worker_leak_it.rs` tách process riêng (fallback **3.809/0**, Windows full @`2b86efee` **3.729/0** + targeted @`90fd24de` PASS), đóng **`#196`** & **`#193`** (`vis-wasm-sem` **390/390**, `vis-wasm-suite` **28/28**).
- **Quyết định đã chốt ở `VD-290`**: chấp thuận đóng `#193` (stack 12 MiB + Node 4096 KiB kèm caveat); chốt luật **R8** vào `AGENTS.md` §6.3 (ca mới vào `tests/<domain>/` + `#[path]`); giữ nguyên 8 binary CLI ở `vis-cli/tests/`; đồng bộ `docs/ViScript-thiet-ke.md` & `docs/SPEC.md`.
- **Lát đang `REVIEW` — `ST-314a` (commit `311d2288`)**: đã vá lỗi `NODE_DRIVER` ở `bin/vis-wasm-conformance` (chạy thật **299 ca × 2 engine, 0 hỏng**, self-check cờ lạ `exit 2`), khai tường minh `"crates/vis-hir"` ở `Cargo.toml` (**23** member) + R8 quét `vis-host-linux` (**39** binary ≤ 40), bỏ wildcard `_ =>` của `StdId` (**167/167**, `path`/`fs_net` nhận `StdId`), và vá đỏ `DOC_BLOCKS` do tài liệu lệch 1 dòng. Cổng: `gate --fast` **3804/3804 PASS**, `vis-diff 100k lech=0`, `vis-targets` PASS, clippy Linux + Windows `psvn` PASS. Bổ sung khung cao điểm 0-LLM: Windows full **3.729/0** (90 binary) + host conformance 54/120 + targets 23 crate; **8× `vis-diff 100k` lech=0**; **fuzz parser 115 phút 959.027.870 ca/0 vi phạm**. macOS `mps` **bỏ qua theo chỉ đạo người dùng** (đang dùng dự án khác) — giữ nợ bằng chứng macOS. Chi tiết `JOURNAL/st314a-hau-kiem.md`.

## 2. Trạng thái Chương trình Tái kiến trúc (`ST-312a..ST-314a`)

| Sóng | Lát | Luồng | Trạng thái | Mục tiêu & KPI |
|---|---|---|---|---|
| **0** | Dọn nhánh (`VD-288` §4) | **W4** | `DONE` | Đã dọn Đợt 1–3 (`main` + `plan/st309-qa`, 1 worktree, `vis-gc-targets` 23 GB trống) |
| **0** | **`ST-312a`** | **W1** | `DONE` | APPROVED `VD-290` (`65ea2a0`): xoá 9 shim `vis_interp → vis_core`; chuẩn hoá kiểu lỗi/config về `vis-core` |
| **0** | **`ST-312b`** | **W4** | `DONE` | APPROVED `VD-290` (`02e881e`): `vis-host-linux` vào workspace; clippy vào `vis-gate-windows --full`; đóng `#196` |
| **1** | **`ST-312c`** | **W3** | `DONE` | APPROVED `VD-290` (`cd007f9`): tách `crates/vis-hir`; `vis-core`/`vis-bytecode`/`vis-vm` **0 dep `vis-types`** |
| **1** | **`ST-312d`** | **W2** | `DONE` | APPROVED `VD-290` (`4e6f810`): tách `crates/vis-http` sans-IO; cắt `vis-pkg → vis-host-native` (`NetHost` + mock) |
| **2** | **`ST-313a`** | **W1+W3** | `DONE` | APPROVED `VD-290` (`aa816e4`): tách `crates/vis-runtime`; cắt `vis-vm → vis-interp`; `vis-interp` 8.792 LOC; `vm_frame.rs` 952 LOC |
| **2** | **`ST-313b`** | **W2+W4** | `DONE` | APPROVED `VD-290` (`c04fee9`+`8855bc3c`): `StdId` 167 hàm + test song ánh; `stdlib/` chia miền (0 tệp `_a.._e`) |
| **3** | **`ST-313c`** | **W1+W2** | `DONE` | APPROVED `VD-290` (`64855d7`..`90fd24de`): `crates/vis-conformance` (420 ca); 384 test API dời khỏi `vis-cli/tests`; `worker_leak_it.rs` (3.809/0); `vis-dag-check` |
| **3** | **`ST-313d`** | **W3+W4** | `DONE` | APPROVED `VD-290` (`47245b0`..`157c73b`): dữ liệu hoá `vd283`/`vd284`; R1..R9 PASS; đóng `#193` (`vis-wasm-sem` 390/390) |
| **Hậu-kiểm** | **`ST-314a`** | **W1/W4** | **`REVIEW`** | `311d2288`: (1) `vis-wasm-conformance` chạy thật **299 × 2 engine, 0 hỏng** + self-check cờ lạ `exit 2`; (2) `"crates/vis-hir"` tường minh (**23** member) + R8 `vis-host-linux` (**39** ≤ 40); (3) `StdId` **167/167**, bỏ `_ =>` & string dispatch ở `path`/`fs_net`; (4) clippy Linux + Windows PASS, macOS bỏ qua theo chỉ đạo; vá `DOC_BLOCKS` 1481→1480/1528→1527 |

## 3. Cổng Kiểm tra Gọn (Chế độ Phát triển — KHÔNG soak/fuzz nặng)

- **Cuối mỗi lát**: `./vis-dev gate --fast` PASS + `VIS_DIFF_N=100000 ./bin/vis-diff` (`lech=0` cho lát chạm compiler/runtime/VM).
- **Cuối đợt / `ST-314a`**: 1× `cargo clippy --workspace --all-targets -- -D warnings` trên 3 OS (`ps`/`psvn`/`mps` khi rảnh) + 1× `bin/vis-targets` (gồm `vis-dag-check`) + 1× WASI smoke (`vis-wasm-suite` / `vis-wasm-sem` / `vis-wasm-conformance` thật 2 engine).
- **Luật cứng giữ nguyên**: 0 `unsafe` (R2), R1 ≤ 1.500 (mục tiêu mềm ≤ 1.150), R5 (`SO-NO`), R7 (log ≤ 50 KB), **R8 (`AGENTS.md` §6.3: ca mới vào `tests/<domain>/` + `#[path]`, trần ≤ 40 toàn workspace)**, R9, không `.github/workflows/`.
- **Quản trị nhánh (`AGENTS.md` §6)**: merge xong xóa worktree + nhánh local (`-d`) + nhánh `origin` trong 24h; giữ **≤ 8 worktree** hoạt động; `CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0`, đĩa trống > 10 GB.

## 4. Lịch Tác chiến theo Khung giờ DeepSeek (`VD-287`, chuẩn `AGENTS.md` §5)

| Khung giờ | UTC | JST (UTC+9) | ICT (UTC+7) | Tác chiến ưu tiên (`ST-314a` & kế tiếp) |
|---|---|---|---|---|
| **Cao điểm 1 & 2** *(2×)* | `01:00–04:00` · `06:00–10:00` | `10:00–13:00` · `15:00–19:00` | `08:00–11:00` · `13:00–17:00` | **0-LLM**: chạy kiểm cổng (`vis-diff 100k`, `clippy` 3 OS, `vis-targets`, WASI smoke, `vis-gc-targets`) hoặc nghỉ gọi LLM |
| **Thấp điểm trưa** *(0,5×)* | `04:00–06:00` | `13:00–15:00` | `11:00–13:00` | Triển khai gọn `ST-314a` (mục 1–3), chạy cổng Linux + Windows, chuẩn bị `REVIEW` |
| **Thấp điểm chính & Cuối tuần** *(0,5×)* | `10:00–01:00` (+ T7–CN) | `19:00–10:00` | `17:00–08:00` | Hoàn tất `ST-314a` (+ clippy macOS `mps` khi rảnh) và mở trục kiến trúc kế tiếp theo chỉ đạo A0 |
