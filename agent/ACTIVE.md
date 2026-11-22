<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ACTIVE — Bảng điều khiển nhanh cho Agent 2 (≤ 100 dòng)

> **Cập nhật**: 2026-11-23 (`VD-282` @ `a1127f5`) · **Tag mới nhất**: `v0.3.6` (`8fa2c09`) · **Giai đoạn đang mở**: **GĐ 1 (`v0.4.0`) + Zero-Idle GĐ 1.5/2**
> **Kế hoạch đêm tới 09:00 sáng 2026-11-23**: xem chi tiết tại `agent/JOURNAL/ke-hoach-dem-2026-11-22-vd282.md`.
> **Khởi động 30 giây**: Đọc `AGENTS.md` → tệp này → phần cuối `agent/QUEUE.md` (`ST-290`..`ST-309`). Chỉ mở `agent/DECISIONS.md` đúng mục `VD-###` được dẫn chiếu.

## 1. Trạng thái hiện hành & Phân làn Zero-Idle (`VD-282`)

| Lát | Luồng | Trạng thái | Ghi chú thực thi & Quy tắc merge |
|---|---|---|---|
| **ST-290** | W3 | **DONE** (`a48d5b87`) | Budget `40M`, `VS0509`/`VS0510` (56 mã), cache `VFEE`; 3.594/0/3, diff 0 |
| **ST-294** | W2 | **DONE** (`cce2d0fb`) | `async-h2` Windows + default-on non-wasm + fix soak `9c0ffd12`; Windows h2 33/33 ×2 |
| **ST-293** | W3 | **DONE** (`6d6bfe9c`) | APPROVED `VD-282`: 4 `select_*` tại `rt/select.rs` + `tyck/expr/kenh.rs`; 25/25 Linux+Win, diff 0 |
| **ST-295** | W2 | **DONE** (`cd76da56`) | APPROVED `VD-282`: `rustix =1.1.6` `openat2`/`O_NOFOLLOW`, race ghi 0/100, Landlock 18/18; đóng `#109b`/`#125`/`#140` |
| **ST-291** | **W1** | **REVIEW** (`269ff71`/merge `8cc3588`) | `P2b-1/2` lazy/batched submit + compact `Vec` proxy; batch 10k **29,1 ms** (interp) / **28,6–29,7 ms** (VM). Tỉ lệ **≤1,5× W=1 chưa đạt** (~3,3–4,0×; trần dựng Interp/Vm mỗi task — cần P2b-3/4, ngoài ST-291) |
| **ST-292** | **W1** | **WIP** | `P4a..c` + `--workers <N>` + `VS0509`; **bắt buộc nối `gioi_han_phien()` vào `SharedBudget`** khi `N>1` (`VD-281`), `--unlimited-budget` ⇒ `None`; test `--workers 4 --max-budget 5000` trap `VS0510` như `W=1` |
| **ST-296** | W4 | **BLOCKED (chờ ST-292)** | Pre-QA xanh (release 3.643/0/4, Win 3.588/0/3, `#191` nextest 3.583/0/3); **R33 chưa chốt**, chưa commit `R33-viscript.json`; chốt sau ST-292 → `R33` + tag `v0.4.0` |
| **ST-296d-tools** | **W4** | **DONE** (`b36f8a5` + `4b2484f`) | **Luật R7** (173 legacy log đóng băng) + `.config/nextest.toml` (`retries=0`) + `bin/vis-{gc-targets,jobs,status,gate-windows}` + auto `CARGO_BUILD_JOBS`/`sccache`; quick Windows `--quick` PASS (pkg 25, h2 33, cancel 8, select 25) |
| **ST-296a** | **W4** | **WIP (nhánh riêng)** | `plan/w4-st296a-tests` **chưa merge** (đầu nhánh hiện `8cc3588`); gom test binary + `tests/common/mod.rs`, trừ `vs3_worker*`; **chỉ merge sau tag `v0.4.0`** |
| **ST-296b** | **W3** | **WIP (nhánh riêng)** | `plan/w3-st296b-crates` **chưa merge** (đầu nhánh `30a7f98`, chưa có commit ST-296b); tách `vis-loader` + `vis-lsp` + `[workspace.dependencies]`; **chỉ merge sau tag `v0.4.0`** |
| **ST-299a** | **W2** | **DONE** (`5cc1903`/merge `060a744`) | `WasiHost` `wasm32-wasip1` + `vis-wasm-run`; **wasm 26/26** (13 ca × 2 đường). `wasmtime` CLI **BLOCKED-ENV**; smoke dùng Python/Node WASI |

## 2. Sổ nợ & Đường dài (`v0.4.1 → v1.0.0-rc1`)

- **`SO-NO.md` sau `VD-282`**: **85 mở / 86 đóng / 18 chuyển VS3 / 2 không làm / 1 điều hướng = 192** (vừa đóng 12 nợ: `#16`, `#17`, `#19`, `#21`, `#26`, `#27`, `#84`, `#98`, `#109b`, `#125`, `#140`, `#191`; giữ `#31` mở thu hẹp).
- **GĐ 1.5 (`v0.4.1`)**: Merge `ST-296a`/`ST-296b` + W1 làm `ST-296c` (stdlib + tách 3 tệp R1: `db/sqlite/tests.rs` 1.495, `lower/expr.rs` 1.456, `lower.rs` 1.446) + `ST-296d` (`agent/archive/`, `DECISIONS-INDEX.md`, tag `v0.4.1`).
- **GĐ 2 (`v0.4.5`, `ST-297..300`)**: Đóng `#149`, Windows Tier 1 (`ST-298`), nối `vis-loader` vào WASI (`ST-299b`) + `bin/vis-wasm-suite` (`ST-300`).
- **GĐ 3 (`v0.5.0`, `ST-301..305`)**: `T??` + Lambda annot + `#155`, `async` method/`cmd` (`#13`), `derives Json` (`#8`), `vis new/doc/repl`, ViShell (`ST-304` + `ST-304b`), VS Code extension.
- **GĐ 4 (`v0.6.0 → v1.0.0-rc1`, `ST-306..309`)**: Plugin tĩnh Native Rust + động WASI/WASM (0 `unsafe`), `Terminal`/ViTerm, `docs/SPEC.md` + kiểm toán bảo mật + soak 24h.

## 3. Rào an toàn bắt buộc khi chạy xuyên đêm

- **KHÔNG** kill tiến trình nền đang sống (`W1`/fuzz/soak/`psvn`); **KHÔNG** `git reset --hard` worktree đang bận; đợi lát hiện tại hoàn tất rồi kéo `origin/main`.
- **CPU & Đĩa (`ps`)**: Giới hạn `CARGO_BUILD_JOBS` (`≤ nproc / số luồng đang chạy`, ví dụ `4` mỗi luồng khi chạy 3–4 luồng), đặt `CARGO_PROFILE_DEV_DEBUG=0`, `TMPDIR=/home/ps/tmp` (KHÔNG `/tmp`, KHÔNG `target/`), giữ trống đĩa > 10 GB.
- **Luật cứng**: `0 unsafe` (R2); R1 ≤ 1.500 dòng; **R7 không commit `.log`/`.txt` > 50 KB trong `agent/JOURNAL/`** (lưu log ngoài repo + trích tóm tắt `.md` & `sha256`); **không** commit `R##-viscript.json` nháp; **không** `.github/workflows/`; macOS ngoài phạm vi.
- **Cổng**: test đỏ-trước; `./vis-dev gate --fast` trước khi báo `REVIEW`; giữ `main` sạch trước tag `v0.4.0` (chỉ merge `ST-291`, `ST-292`, `ST-296d-tools`, `ST-299a`, `ST-296`).
