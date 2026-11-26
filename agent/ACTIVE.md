# agent/ACTIVE.md — Bảng điều khiển tác chiến hiện hành (`VD-289` / `VD-292`, 2026-11-26)

> **Quyết định mới nhất**: `agent/JOURNAL/quyet-dinh-a0-2026-11-26-vd292.md` · `agent/DECISIONS.md` (`VD-292`) · Kiến trúc 5 tầng DAG: `VD-289`/`VD-290` · Gói Sóng 4: `VD-291`.

## 1. CHẾ ĐỘ HIỆN HÀNH — PHÁT TRIỂN KIẾN TRÚC TỐI ƯU (`VD-289` / `VD-292`)

- **TIẾP TỤC TẠM DỪNG VÔ THỜI HẠN** kế hoạch phát hành cũ (`ST-309` = `BLOCKED`, **không** soak 24h/4h, **không** fuzz 8h, **không** 3× full lặp lại, **không** `R37` JSON, **không** tag mới).
- **ĐÃ DUYỆT `DONE` (`VD-292`, 2026-11-26, nền `200a4c86` / mã `7014fb7b`)**: đóng trọn **Sóng 4 (`ST-315a..ST-315d` DONE)** — VM `budget_charge` O(1) + flush TLS 1 lần (`opt-bench` VM 610→344 ms); `Value::Map(Rc<VisMap>)` CoW + chỉ mục băm lười ngưỡng 8 (`opt-bench` **45/42 ms**, `mapbench N=316` ~27×, Q1–Q3 giữ); đóng **`#197`** Landlock `Truncate` tệp đơn `(write)` (19/19) + W3 M2 pipe cap 8 MiB/RAII/UTF-8 (plugin-host 30/30, 0 `unsafe`); mã tĩnh **`VS0304`** (57/57 `API-CODES.txt`) + Tarjan lặp (100k cạnh, SCC > 8 có trần) + parity **66/66** hàm host (12 test). Windows full **`3.779/0/4 @7014fb7b`** & **`@fe3446d2`** (`host_parity` 11/11, host 54/54, `vis-targets` 47/0 + 46+1, clippy PASS); Gatekeeper R-A/R-B/R-C/R-D/R-Parity/R-Docs2/R-Win2 đều **PASS**.
- **Phán quyết A0 (`VD-292`) & Sổ nợ**: chốt **F1 `ST-315b`** (`res.headers: map[str, str]` last-wins không phá API, cookie jar không đổi, đồng bộ `SPEC.md` §8.3); phê chuẩn **F-R1** mức hàm; giữ `API-CODES.txt` đóng băng (`VS0204`); **đăng ký `#198` (`mở` — flake `stress_dai` trong `lib_api_it`) & `#199` (`mở` — lowering `O(N²)` trong `lower/co_so.rs`)** (tổng `SO-NO`: 200 = 80 mở + 101 đóng + 16 VS3 + 2 không làm + 1 điều hướng).
- **TRỤC HIỆN HÀNH — SÓNG 5 KIẾN TRÚC SẠCH, CoW ĐÚNG NGHĨA & ĐÓNG NỢ (`ST-316a`, `ST-316b`, `ST-316d` `READY`; `ST-316c` `BLOCKED`)**:
  - **Phát hiện kiến trúc cốt lõi của A0**: `vm_helpers.rs` ↔ `gan.rs` trùng lặp logic `Value`; `vm_helpers::set(&base)` + `LoadLocal`/`doc_o` luôn giữ `Rc::strong_count ≥ 2` khi ghi `m[k]=v` ⇒ `ST-316a` hợp nhất `vis_runtime::value_ops` + chuyển giao `Rc::ptr_eq` cho `Map` & `List` CoW ghi **tại chỗ O(1)** (0 đổi VBC1).
  - **Sóng 6 hoạch định sẵn (`ST-317a..ST-317c`, `BLOCKED`)**: nén `Value` 64→32 B + `Record` CoW + singleton `{}`; khử trùng lặp `vis-interp` sync↔frame & worker + mọi tệp sản phẩm `< 1.000` LOC; điểm mở rộng một nguồn (`StdId`, `SPEC.md` sync `#75`, `vis-lsp` `AnalysisHost` `#81`/`#66`).

## 2. Bảng Trạng thái Sóng 4–6 (`ST-315a..ST-317c`)

| Sóng | Lát | Luồng | Trạng thái | Mục tiêu & KPI nghiệm thu |
|---|---|---|---|---|
| **4** | **`ST-315a..d`** | **W1..W4** | **`DONE`** | APPROVED `VD-292` (`200a4c86`/`7014fb7b`): `budget_charge` O(1), `Rc<VisMap>` CoW 45/42 ms, `#197` + pipe cap 8 MiB, `VS0304` + Tarjan lặp + parity 66/66; Windows **3.779/0/4**, R-A..R-Win2 PASS |
| **5** | **`ST-316a`** | **W1+W2** | **`READY`** | **Hợp nhất `vis_runtime::value_ops` + `Value::List(Rc<Vec<Value>>)` CoW + True O(1) In-Place CoW (`Map`/`List` qua `Rc::ptr_eq`) + F1/F2/F4/F5 `ST-315b` + `bin/vis-bench`**. KPI: `writebench` ≥ 20×, `listbench` ≥ 10×, `size_of::<Value>() == 64`, `vis-diff 100k` + `stdlib_diff` `lech=0` |
| **5** | **`ST-316b`** | **W3** | **`READY`** | **Đóng `#199` lowering `O(N²) → O(N)` (`HashMap` nội bộ `Lowerer`) + `#47` (`vis-diag` i18n) + `#48` (`is_pub` HIR/LSP) + `#7` (`co_float_sau` đệ quy) + `#186` + `#145`**. KPI: 25k `const` **< 1 s**, 100k **< 5 s**, `vis-diff 100k lech=0` |
| **5** | **`ST-316d`** | **W4** | **`READY`** | **Vệ sinh DAG (`vis-host-native` `vis-runtime` → dev-deps + `CAM_NGANG`) + đóng `#198` (`stress_dai` → `worker_leak_it.rs`, 0 binary R8 mới) + tách `host_parity.rs` (1.457→<500 LOC/tệp) + tồn dư test Sóng 4 (`MAX+1`, truncate `cap[Dir]` chỉ-đọc, 2 SCC rời)** |
| **5** | **`ST-316c`** | **W4** | `BLOCKED` | *(chờ `ST-316a/b/d` & `mps` rảnh)* Bù 3-OS macOS `mps` (`clippy` + `vis-gate-macos --full`), `vis-targets --wasm`, đối chuẩn `vis-bench` 3-OS & hồ sơ Go/No-Go (Sóng 6 vs mở lại `ST-309`) |
| **6** | **`ST-317a`** | **W1** | `BLOCKED` | *(chờ `ST-316a`)* Nén `Value` **64 B → 32 B** (`Box`/`Rc` cho `Cap`/`Record`/`Enum`), CoW `Record` & singleton `{}` (đóng `ST-315b` F6) |
| **6** | **`ST-317b`** | **W2** | `BLOCKED` | *(chờ `ST-316a`)* Khử trùng lặp `vis-interp` sync↔frame & worker orchestration về `vis-runtime` + đưa 6 tệp sản phẩm > 1.100 LOC về **< 1.000 LOC** |
| **6** | **`ST-317c`** | **W3** | `BLOCKED` | *(chờ `ST-316b`)* Điểm mở rộng một nguồn: bảng khai báo `StdId`, kiểm máy `API-CODES.txt`↔`SPEC.md` (`#75`), tách `vis-lsp` khỏi `thread_local!` (`#81`/`#66`) sang `AnalysisHost` |

## 3. Cổng Kiểm tra Gọn (Chế độ Phát triển — KHÔNG soak/fuzz nặng)

- **Cuối mỗi lát**: `./vis-dev gate --fast` PASS + `VIS_DIFF_N=100000 ./bin/vis-diff` (`lech=0` cho lát chạm compiler/runtime/VM) + số đo release có sha256 (`ST-316a`/`ST-316b`).
- **Cuối Sóng 5**: 1× `cargo clippy --workspace --all-targets -- -D warnings` trên Linux + Windows (`psvn`, và `mps` khi rảnh — 0 warning unused dep sau `ST-316d`) + 1× `bin/vis-targets --wasm`.
- **Luật cứng**: 0 `unsafe` (R2), R1 ≤ 1.500 (mục tiêu mềm < 1.000 ở Sóng 6), R5 (`SO-NO`), R7 (log ≤ 50 KB), **R8 ĐANG CHẠM 39/40 — CẤM TẠO TỆP `.rs` CẤP 1 MỚI** (mọi test mới vào `tests/<domain>/` + `#[path]`, hoặc `worker_leak_it.rs` cho test cách ly), R9, không `.github/workflows/`.
- **Quản trị nhánh (`AGENTS.md` §6)**: merge xong xóa worktree + nhánh local (`-d`) + `origin` trong 24h; ≤ 8 worktree; `CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0`, đĩa trống > 10 GB.

## 4. Lịch Tác chiến theo Khung giờ DeepSeek (`VD-287`, chuẩn `AGENTS.md` §5)

| Khung giờ | UTC | JST (UTC+9) | ICT (UTC+7) | Tác chiến ưu tiên (Sóng 5 `ST-316a/b/d`) |
|---|---|---|---|---|
| **Cao điểm 1 & 2** *(2×)* | `01:00–04:00` · `06:00–10:00` | `10:00–13:00` · `15:00–19:00` | `08:00–11:00` · `13:00–17:00` | **0-LLM**: giữ chuỗi chạy nền `peak2-2026-11-26.md`, kiểm cổng (`vis-diff 100k`, `stdlib_diff`, release `vis-bench`, Windows full, `vis-targets --wasm`) |
| **Thấp điểm trưa** *(0,5×)* | `04:00–06:00` | `13:00–15:00` | `11:00–13:00` | Hoàn tất nhanh `ST-316d` (0,5–1 ngày) & rà log cao điểm |
| **Thấp điểm chính & Cuối tuần** *(0,5×)* | `10:00–01:00` (+ T7–CN) | `19:00–10:00` | `17:00–08:00` | Triển khai song song `ST-316a` (W1+W2), `ST-316b` (W3), `ST-316d` (W4) — 0 xung đột tệp — rồi kiểm độc lập & nộp `REVIEW` |

## 5. Vận hành Cao điểm 2 — 2026-11-26 (Đang chạy nền 0-LLM)

- **`ps`**: chuỗi 0-LLM **HOÀN TẤT 09:00:12Z, mọi bước exit 0** — `cargo clean` → `gate --fast` PASS → clippy PASS → `vis-diff 100k` ×3 seed `lech=0` (≈1730 chương trình/s) → fuzz parser 130' OK (fmt 475 tệp bất động) → `fuzz_mutate` 25' `vi_pham=0` (31,5 M ca); log `/home/ps/vislogs/peak2-20261126T0622Z.log` (`sha256 fd32dbb5…`), `HEAD=9ff0659e dirty=0`.
- **`psvn`**: `7014fb7b → fe3446d2` (docs-only, ff) và chạy `vis-gate-windows --full` từ 06:35Z; log `/home/ps/vislogs/win-full-20261126T0636Z.log`. **KẾT QUẢ: PASS 3.779/0/4, exit 0 @06:40Z** (`sha256 e46b5831…`); `vis-targets` core `47/0`, sqlite `46+1/0`. `C:` còn 14 GB — nếu lượt sau đỏ do đĩa thì ghi `BLOCKED-ENV`, không mở task mã.
- **`mps`**: **không dùng** (đang thuộc dự án khác); cổng macOS giữ nguyên ở `ST-316c`.
