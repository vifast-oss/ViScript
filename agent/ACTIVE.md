# agent/ACTIVE.md — Bảng điều khiển tác chiến hiện hành (`VD-285`, cập nhật 2026-11-24)

> **Toàn văn quyết định & kế hoạch refactor**: `agent/JOURNAL/quyet-dinh-a0-2026-11-23-vd285.md` · `agent/DECISIONS.md` (`VD-285`).

## 1. Trạng thái Thực tế tại `7930e90` (`APPROVED VD-285`)

- **GĐ 2 (`v0.4.5` @ `3daff62`)**: 5/5 **DONE** — **Windows Tier 1** (`ST-298`), WASI Preview 1 (`ST-299a/b`, `ST-300` 388/390 = 99,5%), `R35` PASS.
- **Đợt tính năng GĐ 3 & GĐ 4 đã `APPROVED DONE` trên `main`**:
  - macOS Core & Bench: `ST-300m1` (`c3b72907`, đóng **`#137`**) · `ST-300m2` (`5741d234`, `mps` 0,73× `ps`, đóng **`#147`/`#152`**).
  - Ngôn ngữ Pha 2b: `ST-302` + `ST-302b` (`b79ae3ae`, `6bc4ac35`, đóng **`#8`/`#13`**).
  - ViShell & ViTerm: `ST-304` (`8e73e81f`) · `ST-304b` (`1eb2768c`) · `ST-308` (`a9eef01f`).
  - LSP + VS Code + bump `0.5.0`: `ST-305` code (`374b4e8e`, `75e5a53e`, `7b79769b`) — **chưa tag `v0.5.0`**.
  - Plugin tĩnh & động WASI: `ST-306` (`6093aef0`) · `ST-307` (`ab6beb77`/`b5aa40b1`, 0 `unsafe`).
  - `ST-309` (2/3 DONE): `docs/SPEC.md` (`51e937db`/`f9e7503d`, 1.181 dòng, 31 doc-example PASS) + kiểm toán bảo mật (`59028879`, vá F1/F2, fuzz 1,5M = 0 panic, mở **`#194`**).
- **macOS arm64 Tier 1 (đo xong, chờ merge docs)**: `ST-300m3` final6 @ `7930e90` — 3× `bin/vis-gate-macos --full` PASS (220/216/210 s; nextest **3.745/3 skip/0 fail**, doc-test **5/0/1**, host-wasm 29/29, WASI 28/28), soak 4 h **1.878 vòng / 0 lỗi sản phẩm**, `vis-wasm-sem` **388/390**, dirty 0; docs trên `plan/w4-st300m3-final6` (**chưa merge/push/tag**).
- **Sổ nợ `SO-NO.md` §8**: **195 dòng = 82 mở / 94 đóng / 16 VS3 / 2 không làm / 1 điều hướng** (R5 xanh).

## 2. Trình tự 3 Pha Thực thi (KHÔNG trộn lẫn!)

### Pha A — Khóa gọn `v0.5.0` (ĐANG CHẠY — 0 đổi kiến trúc trước tag!)
- **W4 (`mps` + `ps` + `psvn`)**: **`ST-300m3` DONE (đo)** — 3× `bin/vis-gate-macos --full` PASS (220/216/210 s), soak 4 h **1.878 vòng / 0 lỗi sản phẩm**, `vis-wasm-sem` 388/390, dirty 0; docs Tier 1 trên `plan/w4-st300m3-final6` (**chưa merge/push/tag**). Còn lại: `ST-305` R36 + tag `v0.5.0`.
- Hoàn tất **`ST-305`**: làm mới `agent/reports/R36-viscript.{md,json,json.license}` trên `HEAD` chốt (đủ bằng chứng 3-OS Tier 1), xác minh `check_report.py --n 36` + `./vis-dev gate --fast` PASS → gắn & push tag **`v0.5.0`**.

### Pha B — Đợt Refactor Kiến trúc 4 Tầng & Tối ưu Kho **`v0.5.5`** (`ST-310a..ST-310f`, ~3–4 ngày)

> **Chỉ mở merge vào `main` SAU khi tag `v0.5.0` đã push** (có thể tạo nhánh chuẩn bị trước nếu luồng rảnh; `ST-300m3` đã đo xong trên `mps`).

| Luồng | Lát việc | Thứ tự | Phạm vi & KPI cứng (`VD-285`) |
|---|---|---|---|
| **W1** | **`ST-310a`** | Bước 1 (mở đường) | Tạo **`crates/vis-core`** (`value/`, `json/` 1 bộ `json_esc` đóng **`#112`**, `host/` traits, `reactor/` traits, `http_client`); gỡ dep `vis-interp` khỏi `vis-lsp`/`vis-loader`/`vis-host-native`/`vis-pkg`; chia `interp.rs` (< 900 LOC), `rt.rs` (< 800 LOC), gom `builtin/` & `json/`; diff 100k = 0, `vis-host-wasm` xanh. |
| **W3** | **`ST-310b`** | Sau `ST-310a` | Chốt **`vish-term`** làm tầng Terminal/Render duy nhất (bỏ trùng lặp 4 nơi); nối `vish-edit` → `vish-term`; nối `vis-cli` → `vish-edit` + `vish-term` (`vis repl` TTY mặc định dùng `vish-edit`, non-TTY giữ pipe sạch; hỗ trợ `--format viterm`); **0 crate mồ côi**, 0 `unsafe`. |
| **W2** | **`ST-310c`** → **`ST-310d`** | Sau `ST-310a` | **310c**: tách `vis-plugin-abi` mỏng (~350 LOC, **0 dep nội bộ**, `wasm32` < 1 s) vs **`crates/vis-plugin-host`** (`wasi/{protocol,runner,sandbox}.rs`, `static_reg.rs`, max file < 650 LOC); **xoá `cap_fs.rs`** trùng lặp, dùng duy nhất `vis_host_native::quyen::mo_goc`/`mo_tep_ghi`. **310d**: gỡ 7 dep OS/TLS/SQLite trực tiếp thừa khỏi `crates/vis-cli/Cargo.toml` (`[dependencies]` chỉ feature-forwarding), 12 feature matrix xanh. |
| **W4** | **`ST-310e`** → **`ST-310f`** | Sau `310c/d` (`310f` sau `310b/e`) | **310e**: gom `vis-cli/tests/` **27 → ≤ 12 binary** (`vd283/284` → `lang_it`, 6 `vs3_worker*` → `worker_it.rs`, soak → `examples/`; chuyển suite về `vis-lsp`, `vis-pkg`, `vis-loader`, `vis-plugin-host`, `vis-host-native`); thêm cổng trong `bin/vis-dev` (≤ 15 binary + cấm bản sao `openat2`). **310f**: gom thư mục con `vis-lsp/src/` & `vis-host-native/src/`, `bin/vis-targets` phủ **100% workspace**, dọn 3 script khỏi `agent/JOURNAL/`, archive journal tiền-`v0.4.0` & `CHANGELOG.md` tiền-`v0.1.0` (`VD-142`), bump & tag **`v0.5.5`**. |

### Pha C — Tổng Nghiệm thu Cuối `ST-309` & Tag `v0.6.0` → `v1.0.0-rc1`
Chạy trên nền kiến trúc sạch **`v0.5.5`**: fresh-clone 3-OS Tier 1 (`ps`/`psvn`/`mps`) + WASI (`vis-wasm-suite`/`vis-wasm-sem`) + diff 100k ×5 seed + **soak 24h** + **fuzz parser 8h** + **fuzz decoder/verifier 100M ca** + `R37-viscript.{md,json,json.license}` → tag **`v0.6.0`** & **`v1.0.0-rc1`**.

## 3. Rào An toàn Bắt buộc
- **KHÔNG** kill tiến trình nền đang sống (soak/cổng đang chạy trên `mps`, `ps`, `psvn`); **KHÔNG** `git reset --hard` worktree bận; dùng `bin/vis-lock` khi đo độc quyền.
- **CPU & Đĩa**: `CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0` (`TMPDIR=/home/ps/tmp` trên `ps`, `TMPDIR=/Users/mps/tmp` trên `mps`); giữ đĩa trống > 10 GB (`bin/vis-gc-targets`).
- **Luật cứng**: **0 `unsafe`** (`R2`), R1 ≤ 1.500 dòng (mục tiêu mềm sau `v0.5.5` ≤ 1.250), **R5 `SO-NO` §8 == §4**, **R7 không commit `.log`/`.txt` > 50 KB** trong `agent/JOURNAL/`, **không** `.github/workflows/`.
