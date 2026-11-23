<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ACTIVE — Bảng điều khiển nhanh cho Agent 2 (≤ 100 dòng)

> **Cập nhật**: 2026-11-23 (`main` **`f2d3e73`**; nhánh prep `plan/v050-prep`) · **Tags đã phê chuẩn**: **`v0.4.5`** (`3daff62`, Windows Tier 1 + R35), `v0.4.1` (`90844bc`), `v0.4.0` (`c2accdf`), `v0.3.6` (`8fa2c09`) · **`v0.5.0` CHƯA tag**
> **Giai đoạn đang mở**: **GĐ 3 (`v0.5.0`) — Pha 2 Ngôn ngữ + ViShell MVP + macOS arm64 Tier 1 (3-OS Tier 1 + WASI!)**
> **Toàn văn quyết định & kế hoạch macOS**: `agent/JOURNAL/quyet-dinh-a0-2026-11-23-vd284.md` · `agent/DECISIONS.md` (`VD-284`).

## 1. Mốc đã phê chuẩn (`APPROVED VD-284`)

- **GĐ 2 (`v0.4.5` @ `3daff62`)**: 5/5 **DONE** — **Windows Tier 1** (`ST-298` 3× full 3.611/0/3 + soak 4h **985 vòng / 0 fail**), WASI Preview 1 (`ST-299a/b` + `ST-300` **388/390 = 99,5%**, VM 100%), `R35-viscript.{md,json}` PASS.
- **GĐ 3 đã tích hợp vào `main` `f2d3e73`** (trừ `ST-300m3`): `ST-300b` (`770fdf7b`), `ST-301` (`2bc6be74`), `ST-303` (`a114e389`), `ST-300m1` (`c3b7290`), `ST-300m2` (`5741d23`: đóng **`#147`**/**`#152`**), `ST-302` (`b79ae3a`: `#13` + `#8`), `ST-302b` (`6bc4ac3`: spawn method), `ST-304` (`8e73e81`: `vish-edit`), `ST-304b` (`1eb2768`: completion + job control), `ST-305` (`374b4e8`: LSP + VS Code), `ST-306` (`6093aef`: plugin tĩnh), `ST-307` (`ab6beb7`/`b5aa40b`: plugin WASI), `ST-308` (`a9eef01`: `vish-term`); workspace **16 crate**.
- **Gate Linux `main` `f2d3e73`**: **3.747/4** (nextest, version 0.5.0 sau prep); fmt/clippy/R1..R7/doc-examples PASS.
- **Version**: nhánh `plan/v050-prep` bump workspace **0.5.0** + `PHIEN_BAN` `+v050-release-2026-11-23` (khoá `fc20a811…`), lock root + `vis-host-wasm` đồng bộ; **chưa merge, chưa tag**.
- **macOS `mps`**: verify `v0.4.5` **3.631/0/3** (gate), workspace **3.636/0/3**, host **54/54**, WASI **28/28**, `vis-wasm-sem` **388/390**; **Tier 1 CHƯA công bố** — `ST-300m3` (**3× `--full` + soak 4 h**) **đang chạy / chờ kết quả**.

## 2. Phân làn 4 Luồng Đợt kế tiếp (`v0.5.0` — 3-OS Tier 1)

| Luồng | Lát việc | Trạng thái | Phạm vi & Yêu cầu trọng yếu (`VD-284`) |
|---|---|---|---|
| **W2** *(macOS Core)* | **`ST-300m1`** ✅ → **`ST-300m2`** ✅ → **`ST-300m3`** | **m1/m2 DONE/merged** (`c3b7290`, `5741d23`) · **m3 ĐANG CHẠY** | **m1**: errno Darwin, `VS0604` symlink APFS, `bin/vis-gate-macos`, release/targets arm64, đóng **`#137`**. **m2**: soak 10 vòng (`mps` 379,7 s / `ps` 517,1 s = **0,73×**), A/B 24 cặp (interp **1,0385** / VM **1,0626**) — đóng **`#147`**/**`#152`**. **m3**: 3× `bin/vis-gate-macos --full` PASS + soak 4 h (0 fail, RSS/fd phẳng) + `vis-wasm-sem` ≥ 388/390 ⇒ mới công bố **macOS arm64 Tier 1** (`HOST-BAO-DAM.md` & `ROADMAP.md`). |
| **W1** *(Pha 2b)* | **`ST-302`** ✅ · **`ST-302b`** ✅ | **DONE/merged** (`b79ae3a`, `6bc4ac3`) | `async fn` trong `impl` + `async cmd` (đóng **`#13`**); `derives Json` codegen thật (đóng **`#8`**); `spawn obj.m()` cho method async; 0 `unsafe`. |
| **W3** *(ViShell)* | **`ST-304`** ✅ · **`ST-304b`** ✅ · **`ST-308`** ✅ | **DONE/merged** (`8e73e81`, `1eb2768`, `a9eef01`) | `vish-edit` raw mode đa nền, CST coloring, history mờ, Ctrl-C 130; completion theo kiểu + job control POSIX; `vish-term` (`Terminal`, `ViTermFramed`) — **0 `unsafe` R2**. |
| **W4** *(macOS Tier 1 & Release)* | **`ST-305`** ✅ · **`ST-306`** ✅ · **`ST-307`** ✅ → **`ST-300m3`** | **DONE/merged** · **m3 chờ kết quả** | LSP hover/completion + `editors/vscode/`; plugin tĩnh `VIS_PLUGIN_ABI_V1` + plugin động WASI (sandbox `cap`, 3 lớp ABI). **Tag `v0.5.0` chỉ sau** khi m3 xanh + QA 3-OS + `R36-viscript.{md,json}` chốt. **Hiện chưa tag.** |

## 3. Sổ nợ & Chặng tiếp (`v0.6.0 → v1.0.0-rc1`)

- **`SO-NO.md`**: số hiện hành ở §8 là **nguồn duy nhất**; đợt này đã đóng **`#147`**/**`#152`** (`ST-300m2`) và **`#8`**/**`#13`** (`ST-302`).
- **GĐ 4 (`v0.6.0 → v1.0.0-rc1`)**: `ST-309` (`docs/SPEC.md` + kiểm toán bảo mật 3-OS + soak 24h); các mốc `ST-306`/`ST-307`/`ST-308` đã xong sớm ở GĐ 3.

## 4. Rào an toàn bắt buộc

- **KHÔNG** kill tiến trình nền đang sống (`ps`/`psvn`/`mps`); **KHÔNG** `git reset --hard` worktree bận; không chạy soak/gate nặng trên `mps` khi `ST-300m3` đang đo (`bin/vis-lock`).
- **CPU & Đĩa**: `CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0` (`TMPDIR=/home/ps/tmp` trên `ps`, `TMPDIR=/Users/mps/tmp` trên `mps`); giữ đĩa trống > 10 GB (`bin/vis-gc-targets`).
- **Luật cứng**: **0 `unsafe`** (R2 — không bọc `sandbox_init` deprecated của Apple!); R1 ≤ 1.500 dòng; **R7 không commit `.log`/`.txt` > 50 KB** trong `agent/JOURNAL/`; **không** `.github/workflows/`; **3 nền chính thức**: Linux (`ps`), Windows (`psvn`), macOS arm64 (`mps`).
