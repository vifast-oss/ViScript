<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# R36 — ViScript vòng 36 (khung `v0.5.0`)

> **Trạng thái: KHUNG — CHƯA CHỐT.** Báo cáo này được mở tại baseline **`700bb66`** (VD-284,
> hậu tag `v0.4.5`) để chuẩn bị cho mốc **`v0.5.0`** (GĐ 3: Pha 2 ngôn ngữ + CLI Developer +
> ViShell). Chưa có `R36-viscript.json`; số liệu QA 3-OS/WASI sẽ được điền sau khi các lát
> `ST-301`..`ST-305` merge và chạy đủ cổng. Không dùng khung này để tuyên bố phát hành.

## 1. Phạm vi

`v0.5.0` gồm: `ST-301` (`T??` + lambda param annotation + `#155`), `ST-302` (`async` method/cmd
`#13` + `derives Json` `#8`), `ST-303` (`vis new`/`vis doc`/`vis repl`), `ST-304` (ViShell MVP +
`ST-304b` completion/job control), `ST-305` (LSP hoàn thiện + extension VS Code mỏng). Nguồn quyết
định: `DECISIONS.md` `VD-278`, `VD-283`, `VD-284`; hàng việc: `QUEUE.md` `ST-301`..`ST-305`.

## 2. Baseline

- `main` **`700bb66`** (VD-284); tag gần nhất **`v0.4.5`**.
- Linux gate tại baseline: **3660/4** (theo QUEUE/QA gần nhất); Windows `v0.4.5` đã verify
  `3611/0/3` + soak 4h/985 vòng (xem `JOURNAL/st298-win-f4cb136.md`, `JOURNAL/final-win-*`).
- macOS arm64: Tier 1 mới được mở theo `VD-284`; **QA 3-OS sẽ chạy ở `ST-300m1..m3`** và ghi
  bằng chứng riêng — khung này **không** tuyên bố trước.

## 3. Phần đã có trong nhánh ST-305 (chưa merge)

- `crates/vis-lsp`: hover generic (hàm/phương thức in `[T]`) + completion `std.` (module).
- `editors/vscode/`: `package.json`, TextMate grammar, `language-configuration.json`,
  `extension.js` (client JSON-RPC `vis lsp --stdio`), `README.md`; không build/publish.
- Gate nhánh: điền sau khi chạy `./vis-dev gate --fast` (kỳ vọng **3671/4**, tăng theo ca mới).

## 4. QA 3-OS / WASI — sẽ điền sau

- **Linux**: gate + release workspace + `vis test test/sem` + diff 100k.
- **Windows**: `bin/vis-gate-windows --full` 3× + soak 4h; host 54/54; targets.
- **macOS arm64**: `bin/vis-gate-macos --full` 3× + soak 4h (ST-300m3; chưa chạy trong nhánh này).
- **WASI**: `vis-wasm-sem` ≥ 388/390 eligible + suite 28/28; ghi known-gap stack-overflow interp.
- Mọi số đo phải kèm log ngoài repo + sha256; không dùng placeholder trong `main`.

## 5. Cổng

- `fast`: **—** (khung; sẽ chốt khi `v0.5.0`).
- `full`: **—**.
- `nightly`: **—**.
- `report-check`: chạy sau khi hợp nhất `R36-viscript.json` do W3b/A0 chốt (không tự tạo JSON ở
  nhánh W4).

## 6. Ghi chú sổ sách

`head` của báo cáo cuối sẽ là commit R36 (không phải commit chứa khung này). Không tag/push/merge
từ W4; macOS ngoài phạm vi cho tới khi `ST-300m1..m3` hoàn tất. Các mục `[SOAK_*]`/số QA chỉ được
điền khi có bằng chứng máy; khung này không thay thế `R36-viscript.json`.
