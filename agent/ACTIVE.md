# agent/ACTIVE.md — Bảng điều khiển tác chiến hiện hành (`VD-286`, 2026-11-23/24)

> **Toàn văn quyết định & kế hoạch đợt 2**: `agent/JOURNAL/quyet-dinh-a0-2026-11-23-vd286.md` · `agent/DECISIONS.md` (`VD-286`).

## 1. Mốc đã Phê chuẩn (`APPROVED VD-286`)
- **H = `0.6.0` (ST-311a–d, chưa tag)**: `ST-311b` metadata fd đóng `#194` (race 0/100k Linux+mac); `ST-311d` chia 10 tệp >1.150 + `builtin/`; gate Linux **3782/4**; `#195` đã vá mã, còn watchdog 2h.


- **`v0.5.0` @ `c06d7c6f`** (`ST-300m3` + `ST-305`): **DONE** — **macOS arm64 Tier 1** (3× full 220/216/210 s, soak 4h **1.878 vòng / 0 fail**, WASI sem 388/390) + `R36-viscript.{md,json,json.license}` PASS.
- **`v0.5.5` @ `3d85bd2c`** (`ST-310a..ST-310f`): **6/6 DONE** — kiến trúc 4 tầng (`vis-core`, `vis-plugin-host`, `vis-plugin-abi` 0 dep, `vish-term` duy nhất, `vis-cli` 0 dep trực tiếp & 8 test binary, R8/R9, `vis-targets` 38/38 phủ 18/18 crate), đóng **`#112`**, full QA 3-OS PASS (Linux 3.773/4 + release 3.778/0/5 + diff 5×100k=0, Windows 3.720/0/4, macOS 3.764/0/3 + WASI 28/28 & 388/390).
- **Phase C (`ST-309`) đã chạy trên `v0.5.5`**: verifier **100M/0** (35,7 s), parser fuzz 2h **861,37M ca / 0**, diff 6×100k = 0, soak 10/10 = 628 s; `R37` skeleton đã lập.
- **Sổ nợ `SO-NO.md` §8**: **196 dòng = 81 mở / 96 đóng / 16 VS3 / 2 không làm / 1 điều hướng** (đã đóng **`#190`**, vá mã **`#195`** tại `ST-311a`; R5 xanh).

## 2. Đợt Hoàn thiện Bảo mật & Kiến trúc Tiền-`v0.6.0` (`ST-311a..ST-311d` — PHẢI XONG TRƯỚC KHI CHỐT `H` = `0.6.0`)

| Luồng | Lát việc | Trạng thái | Phạm vi & KPI cứng (`VD-286`) |
|---|---|---|---|
| **W1** | **`ST-311a`** | **DONE** (`525f5054`) | Đã vá mã **`#195`** (`Frame::loop_ends` O(1) + `MAX_ITER_DEPTH = 1.024` + `Vm::deadline` `VS0501-TIME` + watchdog `fuzz_mutate.rs`: seed `0x5EED_2026_1121` **61.111 ca / max 4,55 s / 0 vi phạm**), đóng **`#190`** (`pop_frame` trap), tách `vm.rs` (634) + `vm/exec.rs` (714). `#195` giữ mở trên sổ tới khi watchdog 2h trên `H` xong ở Phase C. |
| **W2** | **`ST-311b`** | `READY` | **Đóng `#194` (metadata FS theo `fd`) + chia `quyen/mod.rs`**: chuyển `size`/`is_*`/`exists`/`read_link`/`canonicalize`/`list`/`walk` trên Unix sang `rustix::fs::{openat, statat, readlinkat}` (0 `unsafe`, giữ R9); ca đua 100.000 vòng `rename`/`symlink` đạt **0 rò** trên cả `ps` và `mps`; cập nhật `HOST-BAO-DAM.md` (giữ caveat Windows); chia `quyen/mod.rs` (< 950 LOC). |
| **W3** | **`ST-311c`** | **DONE** (`a320e027`) | Đã hợp nhất 1 nguồn `test/fixtures/vs3_tls/`; chuyển `vs3_http_cancel.rs` → `vis-host-native/tests/http2_it/` & gỡ 7 dev-dep thừa khỏi `vis-cli/Cargo.toml`; gom 56 binary rời giảm tổng workspace **78 → 37 binary** (−52,6%); thêm `crates/vis-core/tests/core_it.rs`; nâng `R8` (`bin/vis-dev` ≤ 40 binary toàn workspace). |
| **W4** | **`ST-311d`** | `READY` | **Chia 10 tệp > 1.150 LOC + dọn `vis-interp/src/` + thử `#193`**: chuyển 6 `builtin_*.rs` vào `vis-interp/src/builtin/`; chia 10 tệp sản phẩm > 1.150 LOC (`tyck/mod.rs`, `codegen/than.rs`, `lex/mod.rs`, `vistest.rs`, `sqlite/mod.rs`, `hir.rs`, `run_cmd.rs`, `parse/mod.rs`, `http_client/mod.rs`, `lower/trait_impl.rs`) về ≤ 1.150 LOC; thử `-zstack-size` WASI cho `#193` (không chặn). |

> **ADR `VD-286`**: **KHÔNG** tách `vis-runtime` khỏi `vis-interp` trước `v1.0.0-rc1` (hoãn sang `v1.x`).

## 3. Pha C Cuối (`ST-309` → Tag `v0.6.0` & `v1.0.0-rc1`)

Sau khi `ST-311a..ST-311d` merge xanh và `#194`/`#195` đóng:
1. Bump **`0.6.0`** (`Cargo.toml`, 2 `Cargo.lock`, `PHIEN_BAN`, khoá cache) → khóa commit `H` đồng nhất trên `ps`, `psvn`, `mps`.
2. Chạy đủ ma trận trên `H`: Linux fresh (`check`/`gate`/`release`/`host`/`diff 5×100k`/`WASI`), Windows **3× `--full` + soak 4h**, macOS **3× `--full` + soak 4h**, **soak Linux 24h**, **fuzz parser 8h**, **verifier 100M**, **VM watchdog 2h** (`0 ca > 300 s`).
3. Xuất **`agent/reports/R37-viscript.{md,json,json.license}`** (`check_report.py --all` PASS), trình A0 duyệt tag **`v0.6.0`** + **`v1.0.0-rc1`** trên cùng commit `D`.

## 4. Rào An toàn Bắt buộc
- **KHÔNG** kill tiến trình nền đang sống (`ps`/`psvn`/`mps`); **KHÔNG** `git reset --hard` worktree bận; dùng `bin/vis-lock` khi đo độc quyền.
- **CPU & Đĩa**: `CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0`, đĩa trống > 10 GB (`bin/vis-gc-targets`).
- **Luật cứng**: **0 `unsafe`** (R2), R1 ≤ 1.500 (mục tiêu mềm ≤ 1.150), R5 (`SO-NO` §8 == §4), R7 (không `.log`/`.txt` > 50 KB trong `agent/JOURNAL/`), R8/R9, không `.github/workflows/`.

## 5. Lịch Tác chiến theo Khung giờ DeepSeek (`VD-287`, chuẩn `AGENTS.md` §5)

| Khung giờ | UTC | JST (UTC+9) | ICT (UTC+7) | Tác chiến ưu tiên cho đợt hiện tại (`ST-311b/d` → `v0.6.0`) |
|---|---|---|---|---|
| **Cao điểm 1** *(3h, 2×)* | `01:00–04:00` | `10:00–13:00` | `08:00–11:00` | **0-LLM**: chạy `vis-gate-windows/macos --full` ×3 (xác nhận `ST-311a..d` / `H`), VM watchdog **2h** (`VBC_FUZZ_PHUT=120`), fuzz parser **3h**, `vis-diff` 5×100k, verifier 100M |
| **Thấp điểm trưa** *(2h, 0,5×)* | `04:00–06:00` | `13:00–15:00` | `11:00–13:00` | Thu log + sha256 Khung 1, vá lỗi ngắn/chốt PR `ST-311b`/`ST-311d`, chuẩn bị & bấm nút chạy soak 4h cho Khung 2 lúc `05:55 UTC` |
| **Cao điểm 2** *(4h, 2×)* | `06:00–10:00` | `15:00–19:00` | `13:00–17:00` | **0-LLM**: **soak 4h** trên `psvn` (Windows) & `mps` (macOS), **fuzz parser 4h** trên `ps`, duy trì **soak 24h** nền trên `ps` |
| **Thấp điểm chính** *(15h, 0,5×)* & **Cuối tuần** | `10:00–01:00` (+ T7–CN) | `19:00–10:00` | `17:00–08:00` | **Phát triển & Refactor**: triển khai **`ST-311b`** (`#194` `statat`/`openat`/`readlinkat` + test đua 100k) & **`ST-311d`** (chia 10 tệp > 1.150 LOC + `builtin/` + thử `#193`), bump `0.6.0`, viết `R37` md/json |

