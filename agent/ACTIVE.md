<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ACTIVE — Bảng điều khiển nhanh cho Agent 2 (≤ 100 dòng)

> **Cập nhật**: 2026-11-23 (`VD-283` @ `b3a97428`) · **Tags đã phê chuẩn**: **`v0.4.1`** (`90844bc`), **`v0.4.0`** (`c2accdf`), `v0.3.6` (`8fa2c09`)
> **Giai đoạn đang mở**: **Chốt cuối GĐ 2 (`v0.4.5`, W4 soak 4h)** + **Zero-Idle GĐ 3 (`v0.5.0`: W1/W2/W3 chạy song song trên nhánh riêng!)**
> **Toàn văn quyết định & kế hoạch GĐ 2→3→4**: `agent/JOURNAL/quyet-dinh-a0-2026-11-23-vd283.md` · `agent/DECISIONS.md` (`VD-283`).

## 1. Mốc đã phê chuẩn (`APPROVED VD-283`)

- **GĐ 1 (`v0.4.0` @ `c2accdf`)**: 7/7 **DONE** (`ST-290`..`ST-296`; `P2b-3/4 NO-GO` chốt; phần tồn đọng `P4c` tách nợ **`#192`**).
- **GĐ 1.5 (`v0.4.1` @ `90844bc`)**: 5/5 **DONE** (`ST-296d-tools` Luật R7 + tools, `ST-296a` 280→22 test binary & gate 304→108 s, `ST-296b` tách `vis-loader` + `vis-lsp`, `ST-296c` gom stdlib + 3 tệp R1, `ST-296d` 63 tệp archive + `DECISIONS-INDEX`).
- **GĐ 2 đã DONE**: `ST-297` (`f4cb136`, `#149` đóng, Win 3× 3.611/0/3), `ST-299a` (`060a744`), `ST-299b` (`3b8049c`, wasm 28/28), và phần harness `ST-300` (`3581c9a`+`8286caf`+`b3a9742`: `vis-wasm-sem` **388/390 = 99,5%**, VM **100%**, nightly fuzz 13,56M/0, soak 10/10; nợ **`#193`** ghi 2 ca stack interp).

## 2. Phân làn Thực thi Ngay lúc này (`VD-283` — 0 Luồng Rảnh)

| Luồng | Lát việc | Trạng thái | Nhánh & Quy tắc Merge |
|---|---|---|---|
| **W4** *(Đường găng `v0.4.5`)* | **`ST-298` + `ST-300` Release `v0.4.5`** | **RUNNING (soak 4h `psvn` — KHÔNG NGẮT!)** | Khi soak 4h xong (0 lỗi sản phẩm, fd/RSS phẳng): tuyên bố **Windows Tier 1** (`ROADMAP` + `HOST-BAO-DAM`), vá `bin/vis-dev` tôn trọng `CARGO_TARGET_DIR` cho `vis-parse-fuzz`, bump `0.4.5` + `vis-host-wasm/Cargo.lock` + cache, xuất đủ **`R35-viscript.{md,json,json.license}`** (`check_report --all` PASS, 0 `[SOAK_*]`), **tag & push `v0.4.5`** (đã uỷ quyền `VD-283` §3). |
| **W2** *(Kiến trúc mỏng)* | **`ST-300b`** (`vis-host-native` + `vis-pkg` + `main.rs` < 900 + tách `lsp.rs`) | **READY (Zero-Idle)** | Chạy ngay trên **`plan/w2-st300b-host-pkg`** (0 đổi hành vi). **Rebase & merge đầu tiên ngay SAU tag `v0.4.5`**! |
| **W1** *(Pha 2 Ngôn ngữ)* | **`ST-301`** (`#192` + `T??` + Lambda annot + `#155`) → **`ST-302`** (`async` impl/cmd `#13` + `derives Json` `#8`) | **READY (Zero-Idle)** | Chạy ngay trên **`plan/w1-st301-pha2a`** (áp hợp đồng văn phạm `VD-283` §4, chỉ chạm core crates, 0 đụng `vis-cli/src/`). **Rebase & merge SAU `v0.4.5` + `ST-300b`**, rồi tiếp nối `ST-302`. |
| **W3** *(CLI Dev & ViShell)* | **`ST-303`** (`vis new`/`doc`/`repl`) → **`ST-304`** (`crates/vish-edit`) | **READY (Zero-Idle)** | Chạy ngay trên **`plan/w3-st303-cli-dev`** (viết mới `cli/{new,doc,repl}.rs`, chạm `main.rs` ≤ 20 dòng). **Rebase & merge SAU `v0.4.5` + `ST-300b`**, rồi mở `crates/vish-edit` (`crossterm`/`rustix::termios` duyệt có điều kiện, log `st304-license.log`). |

## 3. Sổ nợ & Chặng tiếp (`v0.5.0 → v1.0.0-rc1`)

- **`SO-NO.md` (`VD-283`)**: **194 dòng = 86 mở / 87 đóng / 18 chuyển VS3 / 2 không làm / 1 điều hướng** (`#149` đóng; thêm `#192` & `#193`).
- **GĐ 3 (`v0.5.0`)**: `ST-300b` + `ST-301` (`#192`, `#155`) + `ST-302` (`#8`, `#13`) + `ST-303` (`new`/`doc`/`repl`) + `ST-304`/`ST-304b` (`vish-edit` + tab/job/PTY) + `ST-305` (`vis-lsp` + `editors/vscode/` + `R36` + tag `v0.5.0`).
- **GĐ 4 (`v0.6.0 → v1.0.0-rc1`)**: `ST-306` (Plugin tĩnh Native Rust, 0 `unsafe`), `ST-307` (Plugin động WASI/WASM, 0 `unsafe`), `ST-308` (`Terminal`/ViTerm), `ST-309` (`docs/SPEC.md` + kiểm toán bảo mật + soak 24h).

## 4. Rào an toàn bắt buộc

- **KHÔNG** kill soak 4h Windows trên `psvn` hay tiến trình nền đang sống; **KHÔNG** `git reset --hard` worktree bận; đợi lát hiện tại hoàn tất rồi `git fetch origin main`.
- **CPU & Đĩa**: `CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0 TMPDIR=/home/ps/tmp` (+ `sccache` nếu có); giữ đĩa trống > 10 GB (`bin/vis-gc-targets`).
- **Luật cứng**: `0 unsafe` (R2); R1 ≤ 1.500 dòng; **R7 không commit `.log`/`.txt` > 50 KB** trong `agent/JOURNAL/` (log dài ở `/home/ps/vislogs/` + `sha256`); **không** `.github/workflows/`; macOS ngoài phạm vi; **không** trộn nhánh GĐ 3 vào `main` trước tag `v0.4.5`.
