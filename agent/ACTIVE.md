# agent/ACTIVE.md — Bảng điều khiển tác chiến hiện hành (`VD-289` / `VD-295`, 2026-11-27)

> **Quyết định mới nhất**: `agent/JOURNAL/quyet-dinh-a0-2026-11-27-vd295.md` · `agent/DECISIONS.md` (`VD-295`) · Kiến trúc 5 tầng DAG: `VD-289`/`VD-290` · Gói Sóng 4: `VD-291` · Sóng 5: `VD-292`/`VD-293` · Sóng 6: `VD-294`/`VD-295`.

## 1. CHẾ ĐỘ HIỆN HÀNH — PHÁT TRIỂN KIẾN TRÚC TỐI ƯU (`VD-289` / `VD-295`)

- **TIẾP TỤC TẠM DỪNG VÔ THỜI HẠN** kế hoạch phát hành cũ (`ST-309` = `BLOCKED`, **không** soak 24h/4h, **không** fuzz 8h, **không** 3× full lặp lại, **không** `R37` JSON, **không** tag mới).
- **SÓNG 6 — HOÀN TẤT 3/3 (`ST-317a`/`ST-317b`/`ST-317c` `DONE`)**:
  - **`ST-317a` (W1)** — `VD-294`: nén `Value` **64 → 32 B** (`Record(Rc<VisRecord>)`, `Enum(Rc<VisEnum>)`, `Cap(Box<CapVal>)`), CoW `Record` ghi tại chỗ qua `ptr_rc`, singleton `Rc<VisMap>` cho `{}`; gate **3886/3886**, conformance **568/568**, `size_of::<Value>() == 32`, VBC1 rỗng, 0 `unsafe`.
  - **`ST-317b` (W2)** — **`VD-295`** (`main` @ `4e03a19c`): tách `vis-pkg/pkg.rs` 1151→max 377, `rt/sched.rs` 1189→max 279, `tyck/pat.rs` 1140→max 321; `vis-lsp/lib.rs` 1056→**236**; `value/mod.rs` 1415→**748** (+`lit` 403, `ten` 291); `sqlite/mod.rs` 1132→**282** (+6 tệp); **worker orchestration một nguồn** `vis-runtime/src/worker/orchestrate.rs` **135** dòng (`ap_dung_outcome`/`flush_proxy`/`pump`/`ap_va_flush`, `codes::CROSS`), `exec.rs` 461→368, `vm/worker.rs` 586→485, **10 test W1..W10**; dedup `opt_depth_ty`/`loi_arity`. Audit `e1fe1733` + `ef3e6928` + `4e03a19c` **PASS**; gate **3906/3906 (5 ignored)** ≡ 3901/3901+4 skip, diff 100k `lech=0` (56,5 s), clippy 0 warning.
  - **`ST-317c` (W3)** — `VD-294`: StdId registry **167** hàm một nguồn, R10 `API-CODES.txt` ↔ `SPEC.md` `#75`, `AnalysisHost` `#81`/`#66`; facility gate **3885/3885**, `lsp_it` **158/158**, `API-STD.txt` không đổi.
- **Cổng tích hợp cuối `main` `4e03a19c`** (log `/home/ps/vislogs/final-main-20261126T1747Z.log`, sha256 `b0044754…f75dd`): `GATE_EXIT=0` (L0 **3901/3901**, 4 skipped), `DIFF_EXIT=0` (100.000, `lech=0`, 61,9 s), `CLIPPY_EXIT=0`, `RULES_EXIT=0`; **R8 39/40**, `vis-cli/tests` **8/15**.
- **TRỤC HIỆN HÀNH — CHỈ CÒN `ST-316c` (W4) `BLOCKED`**: phần `ps` **đã xong** — WASI **395×2** (368 `.out` + 27 `.trap`), `vis-wasm-sem` **390/390**, `vis-wasm-suite` **28/28** @`29a7b254`; chain 0-LLM `/home/ps/vislogs/peak1-2026-11-27-chain.sh` (18 bước, `--dry-run` exit 0, **chưa chạy**). Phần `mps`/`psvn` vẫn `BLOCKED` (`mps` @`311d2288`, `psvn` @`fe3446d2`), **không** tuyên bố 3-OS khi chưa có log. Không mở task mã mới khi chưa có quyết định A0.
- **Nợ mới từ audit `ST-317b` (`VD-295`)**: **`#203`** (`mở`, W2 frame W=1 lệch interp↔VM: stdout `0` vs `1`, exit 0) & **`#204`** (`mở`, non-frame `run_workers_parallel*`/`_vm*` W≥2 nuốt lỗi task không await, cả 2 engine), giao **VS3/correctness**. Sổ nợ: **205 = 85 mở + 101 đóng + 16 VS3 + 2 không làm + 1 điều hướng**.

## 2. Bảng Trạng thái Sóng 4–6 (`ST-315a..ST-317c`)

| Sóng | Lát | Luồng | Trạng thái | Mục tiêu & KPI nghiệm thu |
|---|---|---|---|---|
| **4** | **`ST-315a..d`** | **W1..W4** | **`DONE`** | APPROVED `VD-292` (`200a4c86`/`7014fb7b`): `budget_charge` O(1), `Rc<VisMap>` CoW 45/42 ms, `#197` + pipe cap 8 MiB, `VS0304` + Tarjan lặp + parity 66/66; Windows **3.779/0/4**, R-A..R-Win2 PASS |
| **5** | **`ST-316a`** | **W1+W2** | **`DONE`** | APPROVED `VD-293` (`882b7ffa`): `value_ops` một nguồn + true O(1) CoW `Map`/`List` + `Value::List(Rc<Vec>)` + F1/F2/F4/F5 + `bin/vis-bench`; **writebench ~240×**, **listbench 13–21×**, diff 100k `lech=0` |
| **5** | **`ST-316b`** | **W3** | **`DONE`** | APPROVED `VD-293` (`da5590e3`, audit `089b9f4c`): `#199` 25k `const` 1,604 → **0,196 s**, `#48`/`#7`/`#186`/`#145`/`#47`; Gatekeeper **PASS 6/6** |
| **5** | **`ST-316d`** | **W4** | **`DONE`** | APPROVED `VD-293` (`ecdb0660`): DAG cấm `vis-host-native → vis-runtime` (+self-test), `#198` (`worker_leak_it.rs`, 0 binary mới), `host_parity` 4 tệp < 500 dòng, tồn dư test Sóng 4 |
| **5** | **`ST-316c`** | **W4** | `BLOCKED` | *(chờ `mps`/`psvn`)* **`ps` đã xong**: WASI **395×2** (368 `.out` + 27 `.trap`), sem **390/390**, suite **28/28** @`29a7b254`; chain 0-LLM `peak1-2026-11-27-chain.sh` (dry-run exit 0, chưa chạy); `mps` @`311d2288`, `psvn` @`fe3446d2` vẫn lệch |
| **6** | **`ST-317a`** | **W1** | **`DONE`** | APPROVED `VD-294` (merge `426b83a3`): Value **64 → 32 B**, `Record` CoW in-place, singleton `{}`; `size_of::<Value>() == 32`, conformance **568/568**, gate **3886/3886**, diff 100k `lech=0` (60,1 s), 0 `unsafe`, VBC1 rỗng |
| **6** | **`ST-317b`** | **W2** | **`DONE`** | APPROVED **`VD-295`** (`main` @ `4e03a19c`): P1+P2a+P2b + audit `e1fe1733`/`ef3e6928`/`4e03a19c` PASS; **worker một nguồn** `orchestrate.rs` 135, `value/mod.rs` **748**, `sqlite/mod.rs` **282**, `vis-lsp/lib.rs` **236**; gate **3906/3906 (5 ignored)**, diff 100k `lech=0`, clippy 0, W **10/10**, R8 **39/40** |
| **6** | **`ST-317c`** | **W3** | **`DONE`** | APPROVED `VD-294` (merge `aecb266c`): StdId **167**, R10 `#75`, `AnalysisHost` `#81`/`#66`; gate **3885/3885**, diff 0, `lsp_it` **158/158**, `API-STD.txt` không đổi; flake P2 → `#202` |

## 3. Cổng Kiểm tra Gọn (Chế độ Phát triển — KHÔNG soak/fuzz nặng)

- **Cuối mỗi lát**: `./vis-dev gate --fast` PASS + `VIS_DIFF_N=100000 ./bin/vis-diff` (`lech=0` cho lát chạm compiler/runtime/VM) + số đo release có sha256.
- **Cuối Sóng 6**: 1× `cargo clippy --workspace --all-targets -- -D warnings` trên Linux + Windows (`psvn`, và `mps` khi rảnh) + 1× `bin/vis-targets --wasm` — Linux đã xanh (`CLIPPY_EXIT=0`); Windows/macOS bù ở `ST-316c` khi máy rảnh.
- **Luật cứng**: 0 `unsafe` (R2), R1 ≤ 1.500 (mục tiêu mềm < 1.000 đã đạt ở lane W2), R5 (`SO-NO`), R7 (log ≤ 50 KB), **R8 ĐANG CHẠM 39/40 — CẤM TẠO TỆP `.rs` CẤP 1 MỚI** (mọi test mới vào `tests/<domain>/` + `#[path]`, hoặc `worker_leak_it.rs` cho test cách ly), R9, R10 (`API-CODES.txt` ↔ `SPEC.md`), không `.github/workflows/`.
- **Quản trị nhánh (`AGENTS.md` §6)**: merge xong xóa worktree + nhánh local (`-d`) + `origin` trong 24h; ≤ 8 worktree; `CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0`, đĩa trống > 10 GB.

## 4. Lịch Tác chiến theo Khung giờ DeepSeek (`VD-287`, chuẩn `AGENTS.md` §5)

| Khung giờ | UTC | JST (UTC+9) | ICT (UTC+7) | Tác chiến ưu tiên (hậu Sóng 6) |
|---|---|---|---|---|
| **Cao điểm 1 & 2** *(2×)* | `01:00–04:00` · `06:00–10:00` | `10:00–13:00` · `15:00–19:00` | `08:00–11:00` · `13:00–17:00` | **0-LLM**: chạy chain `/home/ps/vislogs/peak1-2026-11-27-chain.sh` khi `mps`/`psvn` rảnh (18 bước, dry-run exit 0) — phục vụ `ST-316c`; kiểm `vis-targets --wasm`, Windows full, `vis-diff 100k` |
| **Thấp điểm trưa** *(0,5×)* | `04:00–06:00` | `13:00–15:00` | `11:00–13:00` | Đọc log cao điểm, vá lỗi ngắn, chuẩn bị hồ sơ Go/No-Go cho `ST-316c` |
| **Thấp điểm chính & Cuối tuần** *(0,5×)* | `10:00–01:00` (+ T7–CN) | `19:00–10:00` | `17:00–08:00` | Tổng hợp kết quả chain → đóng `ST-316c` nếu đủ log 3-OS; nếu chưa, giữ `BLOCKED` và chỉ xử nợ `#203`/`#204` khi A0 mở VS3 |

## 5. Vận hành Cao điểm 2 — 2026-11-26 (Đã xong nền 0-LLM)

- **`ps`**: chuỗi 0-LLM **HOÀN TẤT 09:00:12Z, mọi bước exit 0** — `cargo clean` → `gate --fast` PASS → clippy PASS → `vis-diff 100k` ×3 seed `lech=0` (≈1730 chương trình/s) → fuzz parser 130' OK (fmt 475 tệp bất động) → `fuzz_mutate` 25' `vi_pham=0` (31,5 M ca); log `/home/ps/vislogs/peak2-20261126T0622Z.log` (`sha256 fd32dbb5…`), `HEAD=9ff0659e dirty=0`.
- **`psvn`**: `7014fb7b → fe3446d2` (docs-only, ff) và chạy `vis-gate-windows --full` từ 06:35Z; log `/home/ps/vislogs/win-full-20261126T0636Z.log`. **KẾT QUẢ: PASS 3.779/0/4, exit 0 @06:40Z** (`sha256 e46b5831…`); `vis-targets` core `47/0`, sqlite `46+1/0`. `C:` còn 14 GB — nếu lượt sau đỏ do đĩa thì ghi `BLOCKED-ENV`, không mở task mã.
- **`mps`**: **không dùng** (đang thuộc dự án khác); cổng macOS giữ nguyên ở `ST-316c`.
