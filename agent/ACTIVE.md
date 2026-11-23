<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ACTIVE — Bảng điều khiển nhanh cho Agent 2 (≤ 100 dòng)

> **Cập nhật**: 2026-11-23 (`VD-284` @ `bcea91e4`) · **Tags đã phê chuẩn**: **`v0.4.5`** (`3daff62`, Windows Tier 1 + R35), `v0.4.1` (`90844bc`), `v0.4.0` (`c2accdf`), `v0.3.6` (`8fa2c09`)
> **Giai đoạn đang mở**: **GĐ 3 (`v0.5.0`) — Pha 2 Ngôn ngữ + ViShell MVP + macOS arm64 Tier 1 (3-OS Tier 1 + WASI!)**
> **Toàn văn quyết định & kế hoạch macOS**: `agent/JOURNAL/quyet-dinh-a0-2026-11-23-vd284.md` · `agent/DECISIONS.md` (`VD-284`).

## 1. Mốc đã phê chuẩn (`APPROVED VD-284`)

- **GĐ 2 (`v0.4.5` @ `3daff62`)**: 5/5 **DONE** — **Windows Tier 1** (`ST-298` 3× full 3.611/0/3 + soak 4h **985 vòng / 0 fail**), WASI Preview 1 (`ST-299a/b` + `ST-300` **388/390 = 99,5%**, VM 100%), `R35-viscript.{md,json}` PASS.
- **GĐ 3 đã DONE (3/7)**: `ST-300b` (`770fdf7b`, tách `vis-host-native` + `vis-pkg`, `vis-cli` còn 4,8k LOC, `main.rs` 225 dòng), `ST-301` (`2bc6be74`, đóng `#192`/`#155`, fuzz 383M/0), `ST-303` (`a114e389`, `vis new`/`doc`/`repl`, gate Linux **3.671/4**).
- **macOS `mps` verify lần đầu** (`bcea91e4`): macOS 27.0.1 arm64 — `gate --fast` **3.631/0/3 (79,2 s)**, workspace **3.636/0/3**, host **54/54**, WASI **28/28**, `vis-wasm-sem` **388/390**. Gỡ tạm đình chỉ macOS (`VD-284`).

## 2. Phân làn 4 Luồng Đợt kế tiếp (`v0.5.0` — 3-OS Tier 1)

| Luồng | Lát việc | Trạng thái | Phạm vi & Yêu cầu trọng yếu (`VD-284`) |
|---|---|---|---|
| **W2** *(macOS Core)* | **`ST-300m1`** → **`ST-300m2`** | **READY** | **Ưu tiên vá 2 lỗi Darwin errno thật**: (1) `quyen_guard.rs:77` `Some(40)` → `rustix::io::Errno::LOOP \| XDEV` (Darwin `ELOOP=62`) + test trả đúng `VS0604`; (2) `async_net_nguon.rs:17` `115` → `Errno::INPROGRESS \| ALREADY \| AGAIN` (Darwin `36`/`37`) + mở `lat256_net`/`lat3_real` `cfg(unix)`; (3) chạy `quyen_st174`/`quyen_st295` 0/100 trên APFS `mps` đóng **`#137`**; (4) tạo `bin/vis-gate-macos --quick\|--full`, test `bin/vis-release` arm64, thêm dòng `aarch64-apple-darwin` vào `bin/vis-targets`. Xong thì chạy **`ST-300m2`** (`vis-lock` trên `mps` rảnh đóng **`#147`** & **`#152`**). |
| **W1** *(Pha 2b)* | **`ST-302`** | **READY** | `async fn` trong `impl` (`self`/`var self`) + `async cmd` (đóng **`#13`**, giữ `--flag` riêng cho `cmd`); `derives Json` codegen thật `.to_json()`/`from_json()` cho `type`/`enum` (`cap`/`Chan`/`fn` ⇒ `VS0412`, đóng **`#8`**); ≥ 15 ca interp↔VM, cập nhật `CHEAT-SHEET-ViScript.md`. |
| **W3** *(ViShell)* | **`ST-304`** → **`ST-304b`** | **READY** | Tạo `crates/vish-edit`: soạn dòng đa nền **Linux + Windows + macOS**, CST coloring, lịch sử mờ, Ctrl-C exit 130, ấm < 50 ms (`crossterm` hoặc `rustix::termios` duyệt có điều kiện, **0 `unsafe` R2**, bắt buộc `JOURNAL/st304-license.log`). Sau đó **`ST-304b`**: tab completion theo kiểu + job control POSIX (Linux + macOS) + PTY test. |
| **W4** *(macOS Tier 1 & Release)* | **`ST-300m3`** → **`ST-305`** | **BLOCKED** *(chờ `ST-300m1`)* | Sau `ST-300m1` (không chạy chồng lúc `ST-300m2` đo máy rảnh): **3× `bin/vis-gate-macos --full` PASS** + **soak 4h trên `mps`** (0 fail, RSS/fd phẳng) + `vis-wasm-sem` ≥ 388/390 ⇒ công bố **macOS arm64 Tier 1** (`HOST-BAO-DAM.md` & `ROADMAP.md`, caveat: không kernel Landlock). Sau đó chốt **`ST-305`** (`vis-lsp` + `editors/vscode/` + QA 3-OS Tier 1 + `R36-viscript.{md,json}` + tag **`v0.5.0`**). |

## 3. Sổ nợ & Chặng tiếp (`v0.6.0 → v1.0.0-rc1`)

- **`SO-NO.md`**: **194 dòng = 85 mở / 89 đóng / 17 VS3 / 2 không làm / 1 điều hướng** (mục tiêu đợt này đóng tiếp `#137`, `#147`, `#152`, `#8`, `#13`).
- **GĐ 4 (`v0.6.0 → v1.0.0-rc1`)**: `ST-306` (Plugin tĩnh Native Rust, 0 `unsafe`), `ST-307` (Plugin động WASI/WASM, 0 `unsafe`), `ST-308` (`Terminal`/ViTerm), `ST-309` (`docs/SPEC.md` + kiểm toán bảo mật 3-OS + soak 24h).

## 4. Rào an toàn bắt buộc

- **KHÔNG** kill tiến trình nền đang sống (`ps`/`psvn`/`mps`); **KHÔNG** `git reset --hard` worktree bận; không chạy soak trên `mps` cùng lúc `ST-300m2` đang đo benchmark máy rảnh (`bin/vis-lock`).
- **CPU & Đĩa**: `CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0` (`TMPDIR=/home/ps/tmp` trên `ps`, `TMPDIR=/Users/mps/tmp` trên `mps`); giữ đĩa trống > 10 GB (`bin/vis-gc-targets`).
- **Luật cứng**: **0 `unsafe`** (R2 — không bọc `sandbox_init` deprecated của Apple!); R1 ≤ 1.500 dòng; **R7 không commit `.log`/`.txt` > 50 KB** trong `agent/JOURNAL/`; **không** `.github/workflows/`; **3 nền chính thức**: Linux (`ps`), Windows (`psvn`), macOS arm64 (`mps`).
