<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ACTIVE — Bảng điều khiển nhanh cho Agent 2 (≤ 100 dòng)

> **Cập nhật**: 2026-11-22 (`VD-281`; QA GĐ1 `3fbf31a`) · **Tag mới nhất**: `v0.3.6` (`8fa2c09`) · **Giai đoạn đang mở**: **Giai đoạn 1 (`v0.4.0` — G4a Core)**
> **Khởi động 30 giây**: Đọc `AGENTS.md` → tệp này → phần cuối `agent/QUEUE.md` (`ST-290`..`ST-309`). Chỉ mở `agent/DECISIONS.md` đúng mục `VD-###` được dẫn chiếu.

## 1. Trạng thái Giai đoạn 1 (`v0.4.0`)

| Lát | Luồng | Trạng thái | Ghi chú thực thi |
|---|---|---|---|
| **ST-290** | W3 | **DONE** (`a48d5b87`) | Budget mặc định `40M`, `VS0509`/`VS0510` (56 mã), cache `VFEE`; 3.594/0/3, diff 0 |
| **ST-294** | W2 | **DONE** (`cce2d0fb`) | `async-h2` Windows + default-on non-wasm + 0 warning non-TLS; Linux 3.607/4, Windows 3.563/0/3, h2 33/33 |
| **ST-291** | W1 | **WIP** | `P2b-1/2` lazy/batched submit + compact `Vec` proxy (`rt/exec.rs`, `rt/worker.rs`) — chưa có commit chốt |
| **ST-293** | W3 | **REVIEW** (`6d6bfe9`, code `82064e9`) | 4 `select_*` ở `rt/select.rs` + `tyck/expr/kenh.rs`; **25 ca**, gate **3632/4**, diff 100k seed `202611222100` **0 lệch** (`JOURNAL/st293-select.md`) |
| **ST-295** | W2 | **REVIEW** (`cd76da5`, code `ba4023f`) | `rustix` `openat2`/`O_NOFOLLOW`; race ghi **100/100 → 0/100**, Landlock **18/18**, gate **3613/4**, diff 0; license `st295-license.log` (`JOURNAL/st295-baove.md`) |
| **ST-292** | W1 | **BLOCKED** | Chờ `ST-291`; **bắt buộc** nối `gioi_han_phien()` vào `SharedBudget` khi `--workers > 1` (`VD-281`); test `--workers 4 --max-budget 5000` trap `VS0510` giống W=1 |
| **ST-296** | W4 | **QA/WIP** | Partial QA GĐ1: release h2 harness fixed `3896bc6` (release **3643/0/4**, gate **3638/4**); 2 soak **20/20** @ `6d6bfe9`; chờ `ST-291/292/293/295` chốt để `R33`/release |

## 2. Chặng tiếp theo (đã chốt `VD-272`..`VD-281`, đang `BLOCKED` chờ mốc trước)

- **GĐ 1.5 (`v0.4.1`, `ST-296a`..`ST-296d`)**: Gom 278 test binary `vis-cli` → 12–15 + `tests/common/mod.rs`; tách `vis-loader` & `vis-lsp`; gom 68 file stdlib theo miền; dọn `agent/archive/`, tạo `bin/vis-{gc-targets,jobs,status,gate-windows}`.
- **GĐ 2 (`v0.4.5`, `ST-297`..`ST-300`)**: Đóng `#149`, nâng **Windows Tier 1**, `vis-host-wasm` WASI + `wasmtime` runner `bin/vis-wasm-suite`.
- **GĐ 3 (`v0.5.0`, `ST-301`..`ST-305`)**: `T??` + Lambda annot + `#155`, `async` method/`cmd` (`#13`), `derives Json` (`#8`), `vis new/doc/repl`, ViShell (`ST-304` + `ST-304b`), VS Code extension.
- **GĐ 4 (`v0.6.0` → `v1.0.0-rc1`, `ST-306`..`ST-309`)**: Plugin tĩnh Native Rust (`ST-306`) + Plugin động WASI/WASM (`ST-307`, 0 `unsafe`), `Terminal`/ViTerm (`ST-308`), `docs/SPEC.md` + kiểm toán bảo mật + soak 24h (`ST-309`).

## 3. Rào an toàn bắt buộc khi đang chạy song song

- **KHÔNG** kill tiến trình nền đang sống (fuzz/soak/psvn); **KHÔNG** `git reset --hard` worktree đang bận.
- **Đĩa & `/tmp`**: `/tmp` trên `ps` là tmpfs 7,2 GB dễ đầy ⇒ đặt `TMPDIR` trên `/home` khi soak, dùng `CARGO_PROFILE_DEV_DEBUG=0`, dọn `target/` worktree đã merge để luôn giữ trống > 10 GB.
- **Luật cứng**: `0 unsafe` (R2); R1 ≤ 1.500 dòng/tệp (không nhồi tệp > 1.400 dòng: `interp.rs` 1.458, `vis-types/src/stdlib.rs` 1.456, `lower/expr.rs` 1.456, `lower.rs` 1.446); **không** `.github/workflows/`; macOS ngoài phạm vi.
- **Cổng**: test đỏ-trước mỗi lát; `./vis-dev gate --fast` + `./bin/vis-host-suite` + `./bin/vis-targets` mỗi mốc 4 giờ trên `main`; commit `[ST-00]` tách riêng `[ST-###]` (`VD-279`).
