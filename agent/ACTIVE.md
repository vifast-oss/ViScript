<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ACTIVE — Bảng điều khiển nhanh cho Agent 2 (≤ 100 dòng)

> **Cập nhật**: 2026-11-23 (**ST-296d**) · **Tag `v0.4.0` @ `c2accdf`** (commit R33); tag trước **`v0.3.6` @ `8fa2c09`** · **Giai đoạn đang mở**: GĐ 1.5 (`v0.4.1`)
> **GĐ 1.5**: `ST-296a` **đã merge** (`53990c0`), `ST-296b` **đã merge** (`067e1a5`); `ST-296c` nhánh `plan/w1-st296c-stdlib` @ `28738fe`; `ST-297` nhánh `plan/w2-st297-paths`; `ST-296d` nhánh `plan/w3-st296d-archive` (archive/INDEX, DECISIONS-INDEX, ACTIVE).
> **Final QA `v0.4.0`**: Linux @ `e47b3ca` gate **3656/4**, release **3661/0/4**, 2× soak **10/10**; Windows @ `e47b3ca` full **3607/0/3**, release **3606/0/3**; Windows release `0.4.0` @ `8696edc` (`vis.exe` 11.530.240 B).
> **Kế hoạch đêm tới 09:00 sáng 2026-11-23**: xem chi tiết tại `agent/JOURNAL/ke-hoach-dem-2026-11-22-vd282.md`.
> **Khởi động 30 giây**: Đọc `AGENTS.md` → tệp này → phần cuối `agent/QUEUE.md` (`ST-290`..`ST-309`). Chỉ mở `agent/DECISIONS.md` đúng mục `VD-###` được dẫn chiếu.

## 1. Trạng thái hiện hành & Phân làn Zero-Idle (`VD-282`)

| Lát | Luồng | Trạng thái | Ghi chú thực thi & Quy tắc merge |
|---|---|---|---|
| **ST-290** | W3 | **DONE** (`a48d5b87`) | Budget `40M`, `VS0509`/`VS0510` (56 mã), cache `VFEE`; 3.594/0/3, diff 0 |
| **ST-294** | W2 | **DONE** (`cce2d0fb`) | `async-h2` Windows + default-on non-wasm + fix soak `9c0ffd12`; Windows h2 33/33 ×2 |
| **ST-293** | W3 | **DONE** (`6d6bfe9c`) | APPROVED `VD-282`: 4 `select_*` tại `rt/select.rs` + `tyck/expr/kenh.rs`; 25/25 Linux+Win, diff 0 |
| **ST-295** | W2 | **DONE** (`cd76da56`) | APPROVED `VD-282`: `rustix =1.1.6` `openat2`/`O_NOFOLLOW`, race ghi 0/100, Landlock 18/18; đóng `#109b`/`#125`/`#140` |
| **ST-291** | **W1** | **DONE** (`269ff71`/merge `8cc3588`) | `P2b-1/2` lazy/batched submit + compact `Vec` proxy; batch 10k **29,1 ms** (interp) / **28,6–29,7 ms** (VM). Tỉ lệ **≤1,5× W=1 chưa đạt** (~3,3–4,0×; trần dựng Interp/Vm mỗi task — cần P2b-3/4, ngoài ST-291) |
| **ST-292** | **W1** | **DONE** (`eca4e82`/merge `e47b3ca`) | `P4a..c` + `--workers <N>` + `VS0509`; **bắt buộc nối `gioi_han_phien()` vào `SharedBudget`** khi `N>1` (`VD-281`), `--unlimited-budget` ⇒ `None`; test `--workers 4 --max-budget 5000` trap `VS0510` như `W=1` |
| **ST-296** | W4 | **DONE** | R33 **CHỐT**; version **0.4.0** @ `8696edc`, cache re-pin, final QA Linux/Windows xong; tag **`v0.4.0` @ `c2accdf`**; R33 JSON giữ baseline `90cfc39` |
| **ST-296d-tools** | **W4** | **DONE** (`b36f8a5` + `4b2484f`) | **Luật R7** (173 legacy log đóng băng) + `.config/nextest.toml` (`retries=0`) + `bin/vis-{gc-targets,jobs,status,gate-windows}` + auto `CARGO_BUILD_JOBS`/`sccache`; quick Windows `--quick` PASS (pkg 25, h2 33, cancel 8, select 25) |
| **ST-296a** | **W4** | **DONE — đã merge** (`53990c0`) | `plan/w4-st296a-tests` @ `e04d051`; gom 272 tệp test → 12 binary + common + 3 standalone; rebase `ceac954`, gate **3656/4**, 350→92 binary |
| **ST-296b** | **W3** | **DONE — đã merge** (`067e1a5`) | `plan/w3-st296b-crates`; tách `vis-loader` + `vis-lsp` + `[workspace.dependencies]`, rebase `v0.4.0` gate **3656/4**, targets core 18/18, diff 0 |
| **ST-296c** | **W1** | **READY (nhánh riêng, chưa merge)** | `plan/w1-st296c-stdlib` @ `28738fe`; tách `stdlib`/`lower`/`sqlite tests` + gom 68 module interp; chỉ merge ở `v0.4.1` |
| **ST-297** | **W2** | **READY (nhánh riêng, chưa merge)** | `plan/w2-st297-paths`; kế hoạch `PATH-SPLIT` (#149): LSP URI/UNC, `--grant` Windows, test mixed-slash; chỉ merge sau `v0.4.1` |
| **ST-296d** | **W3** | **WIP (nhánh riêng)** | `plan/w3-st296d-archive`; quy hoạch `agent/archive/`, `DECISIONS-INDEX.md`, `ACTIVE.md`, đo trước/sau; chưa tag `v0.4.1` |
| **ST-299a** | **W2** | **DONE** (`5cc1903`/merge `060a744`) | `WasiHost` `wasm32-wasip1` + `vis-wasm-run`; **wasm 26/26** (13 ca × 2 đường). `wasmtime` CLI **BLOCKED-ENV**; smoke dùng Python/Node WASI |

## 2. Sổ nợ & Đường dài (`v0.4.1 → v1.0.0-rc1`)

- **`SO-NO.md` sau `VD-282`**: **85 mở / 86 đóng / 18 chuyển VS3 / 2 không làm / 1 điều hướng = 192** (vừa đóng 12 nợ: `#16`, `#17`, `#19`, `#21`, `#26`, `#27`, `#84`, `#98`, `#109b`, `#125`, `#140`, `#191`; giữ `#31` mở thu hẹp).
- **GĐ 1.5 (`v0.4.1`)**: `ST-296a`/`ST-296b` **đã merge**; `ST-296c` (`plan/w1-st296c-stdlib` @ `28738fe`) + `ST-297` (`plan/w2-st297-paths`) đang nhánh; `ST-296d` (`agent/archive/`, `DECISIONS-INDEX.md`, tag `v0.4.1`) đang nhánh. Chưa bump `0.4.1`, chưa xuất R34.
- **GĐ 2 (`v0.4.5`, `ST-297..300`)**: Đóng `#149`, Windows Tier 1 (`ST-298`), nối `vis-loader` vào WASI (`ST-299b`) + `bin/vis-wasm-suite` (`ST-300`).
- **GĐ 3 (`v0.5.0`, `ST-301..305`)**: `T??` + Lambda annot + `#155`, `async` method/`cmd` (`#13`), `derives Json` (`#8`), `vis new/doc/repl`, ViShell (`ST-304` + `ST-304b`), VS Code extension.
- **GĐ 4 (`v0.6.0 → v1.0.0-rc1`, `ST-306..309`)**: Plugin tĩnh Native Rust + động WASI/WASM (0 `unsafe`), `Terminal`/ViTerm, `docs/SPEC.md` + kiểm toán bảo mật + soak 24h.

## 3. Rào an toàn bắt buộc khi chạy xuyên đêm

- **KHÔNG** kill tiến trình nền đang sống (`W1`/fuzz/soak/`psvn`); **KHÔNG** `git reset --hard` worktree đang bận; đợi lát hiện tại hoàn tất rồi kéo `origin/main`.
- **CPU & Đĩa (`ps`)**: Giới hạn `CARGO_BUILD_JOBS` (`≤ nproc / số luồng đang chạy`, ví dụ `4` mỗi luồng khi chạy 3–4 luồng), đặt `CARGO_PROFILE_DEV_DEBUG=0`, `TMPDIR=/home/ps/tmp` (KHÔNG `/tmp`, KHÔNG `target/`), giữ trống đĩa > 10 GB.
- **Luật cứng**: `0 unsafe` (R2); R1 ≤ 1.500 dòng; **R7 không commit `.log`/`.txt` > 50 KB trong `agent/JOURNAL/`** (lưu log ngoài repo + trích tóm tắt `.md` & `sha256`); **không** commit `R##-viscript.json` nháp; **không** `.github/workflows/`; macOS ngoài phạm vi.
- **Cổng**: test đỏ-trước; `./vis-dev gate --fast` trước khi báo `REVIEW`; tag `v0.4.0` @ `c2accdf` (sau `./vis-dev check`); `ST-296a`/`ST-296b` **đã merge**; `ST-296c`/`ST-297` chỉ merge ở `v0.4.1`.
