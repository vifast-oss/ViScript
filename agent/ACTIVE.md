# agent/ACTIVE.md — Bảng điều khiển tác chiến hiện hành (`VD-288`, 2026-11-24)

> **Toàn văn quyết định & kịch bản Phase C / dọn nhánh**: `agent/JOURNAL/quyet-dinh-a0-2026-11-24-vd288.md` · `agent/DECISIONS.md` (`VD-288`).

## 1. Mốc đã Phê chuẩn & Đóng băng (`APPROVED VD-288`)

- **Mốc mã đóng băng Phase C (`H_code`)**: **`H4 = db67eab`** (`0.6.0`, **CHƯA tag**). Commit `[ST-00]` `VD-288` là **docs-only** ⇒ **KHÔNG restart** soak/fuzz/full-gate đang chạy hoặc đã đo trên `H4 = db67eab`.
- **`ST-311a..f` — 6/6 DONE**:
  - `ST-311a` (`525f505`): vá mã `#195` (`loop_ends` O(1) + `MAX_ITER_DEPTH=1024` + `deadline` + watchdog), đóng `#190`, tách `vm/exec.rs`.
  - `ST-311b` (`61bf4e1`/`ad961bc`): đóng **`#194`** (metadata FS Unix theo `fd`, đua 100k **0 rò** Linux+macOS, `quyen/mod.rs` 1.197→675), gate Linux **3.782/4**.
  - `ST-311c` (`a320e02`): 1 fixture TLS, gỡ 7 dev-dep `vis-cli`, binary integration **78→37**, R8 ≤ 40.
  - `ST-311d` (`7383e58`/`415d205` + `478aa9f`): `builtin/`, chia 10 tệp >1.150 LOC, thử `-zstack-size` WASI (`#193` giữ mở non-blocking), bump **`0.6.0`** + cache key mới.
  - `ST-311e` (`d76e257`, `76d2121`): vá 3 lỗi clippy Windows-only; `ST-311f` (`db67eab`): vá race soak `lat0_suspend`.
- **Tags đã chốt**: `v0.5.5` @ `3d85bd2c` (4 tầng DAG, 3-OS PASS) · `v0.5.0` @ `c06d7c6f` (macOS arm64 Tier 1 + `R36`) · `v0.4.5` @ `3daff62` (Windows Tier 1 + WASI).
- **Sổ nợ `SO-NO.md` §8**: **197 dòng = 81 mở / 97 đóng / 16 VS3 / 2 không làm / 1 điều hướng** (R5 xanh; `#194`/`#190` đóng; `#195` chờ watchdog 2h trên `H4`; thêm `#196` `parse_duration` non-blocking hậu-RC1).

## 2. Pha C Cuối (`ST-309` READY trên `H4 = db67eab` → Tag `v0.6.0` & `v1.0.0-rc1`)

Chạy **song song 3 máy vật lý**, **tuần tự trong từng máy**, log lưu tại `/home/ps/vislogs/r37-h4/` kèm `sha256sum`:

| Máy | Chuỗi lệnh tự trị 0-LLM trên `H4 = db67eab` | Tiêu chí PASS |
|---|---|---|
| **`ps`** (Linux fresh clone) | (1) `check` + `gate --fast` + `cargo test --release` + `vis-host-suite` + WASI (`28/28` & `≥388/390`) + verifier **100M** + `vis-diff` **5×100k** → (2) **Watchdog 2h** (`VBC_FUZZ_PHUT=120`) → (3) **Fuzz parser 8h** (`--seconds 28800`) → (4) **Soak 24h** (`vis-soak --append` ≥ 86.400 s) | 0 fail, `lech=0`, verifier 100M/0, watchdog **0 ca > 300 s** (đóng `#195`), parser 8h/0, soak 24h RSS/fd phẳng, dirty 0 |
| **`psvn`** (Windows sạch) | `cargo clippy --workspace --all-targets -- -D warnings` → **3× `bin/vis-gate-windows --full`** → **soak 4h** | 0 warning, 3× full 0 fail (host 54/54, targets 38/38+37/1), soak 4h 0 fail sản phẩm, dirty 0 |
| **`mps`** (macOS arm64 sạch) | **3× `bin/vis-gate-macos --full`** + `bin/vis-wasm-sem --require-target` → **soak 4h** | 3× full 0 fail (host-wasm 29/29, WASI 28/28, sem ≥ 388/390), soak 4h 0 fail sản phẩm, dirty 0 |

Sau khi đủ bằng chứng trên `H4`: đóng `#195` trong `SO-NO.md` → điền `R37-viscript.md` + thêm `--round 37` trong `test/report/gen_report_json.py` sinh `R37-viscript.json` + `.license` → `check_report.py --all` PASS → A0 duyệt tag **`v0.6.0`** + **`v1.0.0-rc1`** trên cùng commit `D`.

## 3. Dọn dẹp Nhánh `plan/*` & Worktree (`VD-288` §4 — ĐÃ DUYỆT 3 ĐỢT)

- **Đợt 1 (XÓA NGAY)**: 23 nhánh đã merge + không worktree (`git branch -d` + `git push origin --delete` + `git fetch --prune`; gồm `plan/st311b-fd-metadata`, `plan/st311d-split` nếu rảnh).
- **Đợt 2 (XÓA KHI RẢNH)**: 43 nhánh đã merge + có worktree — chỉ gỡ worktree không còn PID (`ps`/`lsof` trống) & `status` sạch (`git worktree remove` không `--force`, sau đó `git branch -d`, xóa `origin`, `git worktree prune`, `bin/vis-gc-targets`).
- **Đợt 3 (17 nhánh chưa merge)**:
  - **GIỮ tới sau RC1**: `origin/plan/st309-qa` (`623f3ba`, đối chứng `#195` trong `R36`/`R37`; KHÔNG merge).
  - **DUYỆT XÓA**: `plan/win-full-f9e7503` (diff=0 với `main`), `plan/phasec-qa-plan` (thay bởi `r37-phasec.md`), `plan/win-full-c4337ce`, `plan/win-quick-v055b` (sao `.md` vào `/home/ps/vislogs/archive-journals/` trước), `fix/macos-v045-tests` (đã vào `main`).
  - **12 nhánh local-only**: rà `git cherry -v main` + `git diff --name-only main...<branch>` — chỉ docs/journal mốc đã tag ⇒ sao `.md` độc nhất rồi `-D`; có diff mã/test ⇒ **DỪNG báo A0**.

## 4. Rào An toàn & Quy tắc Thường trực (`AGENTS.md` §5–§6)

- **Cấm** sửa mã/test/script (`crates/`, `bin/`, `Cargo.*`) trước tag `v0.6.0`/`v1.0.0-rc1`; **cấm** kill tiến trình nền đang sống; **cấm** `git reset --hard` worktree bận; dùng `bin/vis-lock` khi đo độc quyền.
- **CPU & Đĩa**: `CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0`, đĩa trống > 10 GB (`bin/vis-gc-targets`); trần **≤ 8 worktree** hoạt động, dọn nhánh đã merge trong 24h.
- **Luật cứng**: **0 `unsafe`** (R2), R1 ≤ 1.500 (mềm ≤ 1.150), R5 (`SO-NO`), R7 (log ≤ 50 KB), R8 (≤ 40 binary), R9 (`openat2` duy nhất), không `.github/workflows/`.
- **Hậu-RC1** (không làm trước tag): gộp `clippy -D warnings` vào `bin/vis-gate-windows --full`, vá `#196` (`parse_duration`), xác nhận `#193` trên `wasmtime`/trampoline, ADR `vis-runtime` (`v1.x`).

## 5. Lịch Tác chiến theo Khung giờ DeepSeek (`VD-287`, chuẩn `AGENTS.md` §5)

| Khung giờ | UTC | JST (UTC+9) | ICT (UTC+7) | Tác chiến ưu tiên (`H4 = db67eab` → `v0.6.0`/`v1.0.0-rc1`) |
|---|---|---|---|---|
| **Cao điểm 1** *(3h, 2×)* | `01:00–04:00` | `10:00–13:00` | `08:00–11:00` | **0-LLM**: 3× `vis-gate-windows`/`macos --full` trên `H4`, VM watchdog **2h** (`VBC_FUZZ_PHUT=120`), verifier 100M, `vis-diff` 5×100k |
| **Thấp điểm trưa** *(2h, 0,5×)* | `04:00–06:00` | `13:00–15:00` | `11:00–13:00` | Đọc đuôi log + `sha256`, dọn nhánh/worktree theo Đợt 1–3 (`VD-288`), kích hoạt chuỗi job Cao điểm 2 lúc `05:55 UTC` |
| **Cao điểm 2** *(4h, 2×)* | `06:00–10:00` | `15:00–19:00` | `13:00–17:00` | **0-LLM**: **soak 4h** `psvn` & `mps`, **fuzz parser 8h** (`ps`), nối tiếp **soak 24h** nền trên `ps` |
| **Thấp điểm chính** *(15h, 0,5×)* & **Cuối tuần** | `10:00–01:00` (+ T7–CN) | `19:00–10:00` | `17:00–08:00` | Thu log + `sha256`, hoàn thiện `R37-viscript.{md,json,json.license}`, đóng `#195`, chạy `check_report.py --all`, trình A0 duyệt tag `v0.6.0` & `v1.0.0-rc1` |
