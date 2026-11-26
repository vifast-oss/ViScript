# agent/ACTIVE.md — Bảng điều khiển tác chiến hiện hành (`VD-289` / `VD-294`, 2026-11-27)

> **Quyết định mới nhất**: `agent/JOURNAL/quyet-dinh-a0-2026-11-27-vd294.md` · `agent/DECISIONS.md` (`VD-294`) · Kiến trúc 5 tầng DAG: `VD-289`/`VD-290` · Gói Sóng 4: `VD-291` · Sóng 5: `VD-292`/`VD-293`.

## 1. CHẾ ĐỘ HIỆN HÀNH — PHÁT TRIỂN KIẾN TRÚC TỐI ƯU (`VD-289` / `VD-294`)

- **TIẾP TỤC TẠM DỪNG VÔ THỜI HẠN** kế hoạch phát hành cũ (`ST-309` = `BLOCKED`, **không** soak 24h/4h, **không** fuzz 8h, **không** 3× full lặp lại, **không** `R37` JSON, **không** tag mới).
- **ĐÃ DUYỆT `DONE` (`VD-293`, 2026-11-26, tích hợp `review/song5` @ `882b7ffa`)**: đóng trọn **Sóng 5 (`ST-316a`, `ST-316b`, `ST-316d` DONE)** — `vis_runtime::value_ops` một nguồn + true O(1) in-place CoW `Rc::ptr_eq` cho `Map`/`List` (0 đổi VBC1); release `vis-bench`: **writebench ~240×**, **listbench 13–21×**; đóng **`#199`** (25k `const` 1,604 → 0,196 s), **`#48`**, **`#7`**, **`#186`**, **`#145`**, **`#47`**; DAG cấm `vis-host-native → vis-runtime` (+self-test, 4 cạnh cấm), đóng **`#198`** (0 binary R8 mới), tách `host_parity.rs` 1.457 → 4 tệp < 500 dòng. Tích hợp: gate **3.880/3.880 (4 skip)**, clippy `--all-targets -D warnings` **exit 0**, `vis-diff 100k lech=0` (60,2 s), DAG **23 crate/5 tầng**, **R8 39/40**.
- **ĐÃ DUYỆT `DONE` (`VD-294`, 2026-11-27, `main` @ `426b83a3`)**: **`ST-317a`** (nén `Value` **64 → 32 B**, `Record` CoW in-place qua `ptr_rc`, singleton `{}`; gate **3886/3886**, diff 100k `lech=0` 60,1 s, conformance **568/568**, `size_of::<Value>() == 32`, VBC1 rỗng, 0 `unsafe`) và **`ST-317c`** (StdId registry **167**, R10 `API-CODES.txt`↔`SPEC.md` `#75`, `AnalysisHost` `#81`/`#66`; gate **3885/3885**, diff 0, `lsp_it` **158/158**, `API-STD.txt` không đổi). Flake P2 `host_parity::net_http` (stdout chứa ms thật) đăng ký nợ **`#202`** (VS3/test hygiene).
- **Đăng ký nợ từ audit `ST-316b` (`VD-293`)**: **`#200`** (`mở`, P2 — `fn main(a: int, b: int)`: interp `VS0501`/exit 101 vs VM exit 0) & **`#201`** (`mở`, P3 — `fn main(a: int)` gán `argv` vào tham số `int` không kiểm kiểu), giao **VS3/correctness**. Sổ nợ hiện tại (`VD-294`): **203 = 83 mở + 101 đóng + 16 VS3 + 2 không làm + 1 điều hướng**.
- **TRỤC HIỆN HÀNH — SÓNG 6 (`ST-317a`/`ST-317c` `DONE`; `ST-317b` `READY` P2b đang chạy; `ST-316c` giữ `BLOCKED` vì `mps`)**:
  - **`ST-317b` (W2)**: P1 (`pkg.rs`, `rt/sched.rs`, `tyck/pat.rs`) + P2a (`vis-lsp/lib.rs`) đã `DONE`/merged; **P2b đang thi công** — `value/mod.rs`, `sqlite/mod.rs`, worker orchestration + khử trùng lặp `vis-interp` sync↔frame; chưa DONE/REVIEW.
  - **`ST-316c` (W4)**: `ps` **đã xong** — WASI **395×2** (368 `.out` + 27 `.trap`), sem **390/390**, suite **28/28**; chain 0-LLM `/home/ps/vislogs/peak1-2026-11-27-chain.sh` sẵn sàng (dry-run exit 0, chưa chạy). Phần `mps`/`psvn` vẫn `BLOCKED` (`mps` @`311d2288`, `psvn` @`fe3446d2`), **không** tuyên bố 3-OS khi chưa có log.

## 2. Bảng Trạng thái Sóng 4–6 (`ST-315a..ST-317c`)

| Sóng | Lát | Luồng | Trạng thái | Mục tiêu & KPI nghiệm thu |
|---|---|---|---|---|
| **4** | **`ST-315a..d`** | **W1..W4** | **`DONE`** | APPROVED `VD-292` (`200a4c86`/`7014fb7b`): `budget_charge` O(1), `Rc<VisMap>` CoW 45/42 ms, `#197` + pipe cap 8 MiB, `VS0304` + Tarjan lặp + parity 66/66; Windows **3.779/0/4**, R-A..R-Win2 PASS |
| **5** | **`ST-316a`** | **W1+W2** | **`DONE`** | APPROVED `VD-293` (`882b7ffa`): `value_ops` một nguồn + true O(1) CoW `Map`/`List` + `Value::List(Rc<Vec>)` + F1/F2/F4/F5 + `bin/vis-bench`; **writebench ~240×**, **listbench 13–21×**, `size_of::<Value>() == 64`, diff 100k `lech=0` |
| **5** | **`ST-316b`** | **W3** | **`DONE`** | APPROVED `VD-293` (`da5590e3`, audit `089b9f4c`): `#199` 25k `const` 1,604 → **0,196 s**, `#48`/`#7`/`#186`/`#145`/`#47`; Gatekeeper **PASS 6/6** |
| **5** | **`ST-316d`** | **W4** | **`DONE`** | APPROVED `VD-293` (`ecdb0660`): DAG cấm `vis-host-native → vis-runtime` (+self-test), `#198` (`worker_leak_it.rs`, 0 binary mới), `host_parity` 4 tệp < 500 dòng, tồn dư test Sóng 4 |
| **5** | **`ST-316c`** | **W4** | `BLOCKED` | *(chờ `mps`/`psvn`)* **`ps` đã xong**: WASI **395×2** (368 `.out` + 27 `.trap`), sem **390/390**, suite **28/28** @`29a7b254`; chain 0-LLM `peak1-2026-11-27-chain.sh` (dry-run exit 0, chưa chạy); `mps` @`311d2288`, `psvn` @`fe3446d2` vẫn lệch |
| **6** | **`ST-317a`** | **W1** | **`DONE`** | APPROVED `VD-294` (merge `426b83a3`): Value **64 → 32 B**, `Record` CoW in-place, singleton `{}`; `size_of::<Value>() == 32`, conformance **568/568**, gate **3886/3886**, diff 100k `lech=0` (60,1 s), 0 `unsafe`, VBC1 rỗng |
| **6** | **`ST-317b`** | **W2** | **`READY` (P2b đang chạy)** | P1 (`pkg.rs`/`rt/sched.rs`/`tyck/pat.rs`, audit `e1fe1733` PASS, merge `f8f155d0`) + P2a (`29a7b254`, tách `vis-lsp/lib.rs`, `lsp_it` 158/158) DONE/merged; P2b: `value/mod.rs`, `sqlite/mod.rs`, worker + khử trùng lặp `vis-interp` sync↔frame |
| **6** | **`ST-317c`** | **W3** | **`DONE`** | APPROVED `VD-294` (merge `aecb266c`): StdId **167**, R10 `#75`, `AnalysisHost` `#81`/`#66`; gate **3885/3885**, diff 0, `lsp_it` **158/158**, `API-STD.txt` không đổi; flake P2 → `#202` |

## 3. Cổng Kiểm tra Gọn (Chế độ Phát triển — KHÔNG soak/fuzz nặng)

- **Cuối mỗi lát**: `./vis-dev gate --fast` PASS + `VIS_DIFF_N=100000 ./bin/vis-diff` (`lech=0` cho lát chạm compiler/runtime/VM) + số đo release có sha256 (`ST-316a`/`ST-316b`).
- **Cuối Sóng 6**: 1× `cargo clippy --workspace --all-targets -- -D warnings` trên Linux + Windows (`psvn`, và `mps` khi rảnh — 0 warning unused dep sau `ST-316d`) + 1× `bin/vis-targets --wasm`.
- **Luật cứng**: 0 `unsafe` (R2), R1 ≤ 1.500 (mục tiêu mềm < 1.000 ở Sóng 6), R5 (`SO-NO`), R7 (log ≤ 50 KB), **R8 ĐANG CHẠM 39/40 — CẤM TẠO TỆP `.rs` CẤP 1 MỚI** (mọi test mới vào `tests/<domain>/` + `#[path]`, hoặc `worker_leak_it.rs` cho test cách ly), R9, không `.github/workflows/`.
- **Quản trị nhánh (`AGENTS.md` §6)**: merge xong xóa worktree + nhánh local (`-d`) + `origin` trong 24h; ≤ 8 worktree; `CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0`, đĩa trống > 10 GB.

## 4. Lịch Tác chiến theo Khung giờ DeepSeek (`VD-287`, chuẩn `AGENTS.md` §5)

| Khung giờ | UTC | JST (UTC+9) | ICT (UTC+7) | Tác chiến ưu tiên (Sóng 6 `ST-317a/b/c`) |
|---|---|---|---|---|
| **Cao điểm 1 & 2** *(2×)* | `01:00–04:00` · `06:00–10:00` | `10:00–13:00` · `15:00–19:00` | `08:00–11:00` · `13:00–17:00` | **0-LLM**: giữ chuỗi chạy nền `peak2-2026-11-26.md`, kiểm cổng (`vis-diff 100k`, `stdlib_diff`, release `vis-bench`, Windows full, `vis-targets --wasm`) |
| **Thấp điểm trưa** *(0,5×)* | `04:00–06:00` | `13:00–15:00` | `11:00–13:00` | Đọc log cao điểm, vá lỗi ngắn / refactor gọn, chuẩn bị commit sạch cho Sóng 6 |
| **Thấp điểm chính & Cuối tuần** *(0,5×)* | `10:00–01:00` (+ T7–CN) | `19:00–10:00` | `17:00–08:00` | Hoàn tất `ST-317b` P2b (`value/mod.rs`, `sqlite/mod.rs`, worker, `vis-interp` dedup) → `REVIEW`; kiểm độc lập `ST-317a`/`ST-317c` (đã `DONE`); phần `ps` của `ST-316c` đã xong — chạy chain 0-LLM khi `mps`/`psvn` rảnh |

## 5. Vận hành Cao điểm 2 — 2026-11-26 (Đang chạy nền 0-LLM)

- **`ps`**: chuỗi 0-LLM **HOÀN TẤT 09:00:12Z, mọi bước exit 0** — `cargo clean` → `gate --fast` PASS → clippy PASS → `vis-diff 100k` ×3 seed `lech=0` (≈1730 chương trình/s) → fuzz parser 130' OK (fmt 475 tệp bất động) → `fuzz_mutate` 25' `vi_pham=0` (31,5 M ca); log `/home/ps/vislogs/peak2-20261126T0622Z.log` (`sha256 fd32dbb5…`), `HEAD=9ff0659e dirty=0`.
- **`psvn`**: `7014fb7b → fe3446d2` (docs-only, ff) và chạy `vis-gate-windows --full` từ 06:35Z; log `/home/ps/vislogs/win-full-20261126T0636Z.log`. **KẾT QUẢ: PASS 3.779/0/4, exit 0 @06:40Z** (`sha256 e46b5831…`); `vis-targets` core `47/0`, sqlite `46+1/0`. `C:` còn 14 GB — nếu lượt sau đỏ do đĩa thì ghi `BLOCKED-ENV`, không mở task mã.
- **`mps`**: **không dùng** (đang thuộc dự án khác); cổng macOS giữ nguyên ở `ST-316c`.
