# Báo cáo sáng 2026-11-23 — ViScript (chốt)

> **Trạng thái**: báo cáo chốt; số liệu đã xác nhận bằng máy.
> **Tag**: `v0.4.5 @ 3daff62`; `v0.4.1 @ 90844bc`; `v0.4.0 @ c2accdf` (annotated, pushed).
> **Main**: `0039837` (sau merge ST-300b/301/303); tag `v0.4.5` @ `3daff62`.
> Docs-only; không GitHub Actions; macOS ngoài phạm vi.

## 1. Giai đoạn 1 — `v0.4.0`

- Linux final QA @ `e47b3ca`: gate **3656/4**; release **3661/0/4**; **2× soak 10/10**
  (531 s + 642 s; mỗi soak 3× gate 3656/4, 3× diff 100k `lech=0`, host 54/54, cache 5/5).
- Diff ngoài soak seed `202611230230` **0 lệch** (100.000; 69,2 s).
- Windows @ `e47b3ca`: full **3607/0/3**; release **3606/0/3**.
- Đã đóng GĐ1: ST-290..295 + ST-296d + ST-299a; tag `v0.4.0 @ c2accdf`.

## 2. Giai đoạn 1.5 — `v0.4.1`

- **ST-296a** `53990c0`: `vis-cli` **280 → 22** binary; gate **352 → 92** binary;
  gate **3656/4**; wall **304 → 210,77 s**.
- **ST-296b** `067e1a5`/merge `1b27cb8`: tách **`vis-loader` + `vis-lsp`**; Windows post-merge
  **3607/0/3**; lsp 27/27, loader 29/29, targets core 18/18.
- **ST-296c** `ab8d393`: tách `stdlib`/`lower`/`sqlite tests` theo R1; gate 3656/4, diff 0.
- **ST-296d** `a1730c5`: **63 tệp** R100 vào `agent/archive/`, `DECISIONS-INDEX` **271 VD**;
  `agent/` rời **77 → 19**; report-check 33/329.
- QA `v0.4.1`: Linux @ `5d7791c` gate **3656/4**, release **3661/0/4**, soak **10/10** 530 s;
  Windows @ `5d7791c`+`11e8120` full **3607/0/3**, release **3606/0/3**; lock
  `vis-host-wasm` 0.4.1 sạch.
- Tag `v0.4.1 @ 90844bc`.

## 2b. Giai đoạn 2 — `v0.4.5` @ `3daff62`

- **ST-298**: Windows **3× full 3611/0/3**; **soak 4h 985 vòng, fail 0, flake 1**,
  free RAM 8204–8766 MB; proc/threads 0/0 sau mỗi vòng; fd/handle không đo được sau soak (NO_PROC; không còn tiến trình vis/cargo/rustc); dirty cuối 0; **Windows Tier 1** công bố (caveat `FsGuard`, uỷ quyền `VD-283`).
- **ST-299a/b**: `WasiHost` + `vis-loader` đa tệp; native 29/29, wasm 28/28.
- **ST-300**: `vis-wasm-sem` **388/390 = 99,5%** (VM 100%); nợ `#193` 2 stack interp.
- **Version 0.4.5**, cache `d92f31c2…`, R35 md+json; fix `bin/vis-dev` fuzz theo `CARGO_TARGET_DIR`.

## 3. Sau tag — ST-297 / ST-299b / ST-300

- **ST-297** merged `f4cb136`: gate **3660/4** (4 ca mới UNC/mixed/percent + `--grant` Windows);
  Windows **3× full 3611/0/3**, host 54/54, targets 18/18 + 17/1; Linux **10×** `vis-lsp`
  29/29, `path_impl` 2/2 (`8b03077`); **#149 đóng** (SO-NO §8: 84 mở / 87 đóng / 192 dòng).
- **ST-299b** merged `3b8049c`: `vis-loader` vào `vis-wasm-run` (đa tệp + `use`);
  native **29/29**, wasm **28/28**.
- **ST-300 harness** `3581c9a`: `test/sem` WASI trên oracle native — **388/390 = 99,5%**
  eligible; 2 fail còn lại là **stack overflow interp WASI** (VM pass), đã ghi rõ.
- **P2b-3/4 NO-GO** `16fba3a`: bench **4,0×**, không đạt ≤1,5×, không đổi production.
- **Sau tag `v0.4.5`**: merge **ST-300b** `770fdf7` → **ST-301** `2bc6be7` → **ST-303** `a114e38`; gate **3671/4**.
- Flake fix `044b289`: poll 2 s sau join (test-only; 0/60 đỏ dưới tải).

## 4. ST-298 — soak 4h (PASS)

- Trạng thái: **PASS** trên `f4cb136`; 3× `vis-gate-windows --full` **xanh 3611/0/3**;
  soak 4h: `985` vòng, fail `0`, fd/RSS `free RAM 8204–8766 MB; proc/threads 0/0 sau mỗi vòng; fd/handle không đo được sau soak (NO_PROC; không còn tiến trình vis/cargo/rustc); dirty cuối 0`.
- **Windows Tier 1 đã công bố** (caveat `FsGuard`). Điều kiện công bố: soak 4h đủ vòng, `0` = 0,
  fd/RSS phẳng (`free RAM 8204–8766 MB; proc/threads 0/0 sau mỗi vòng; fd/handle không đo được sau soak (NO_PROC; không còn tiến trình vis/cargo/rustc); dirty cuối 0`), 3× full xanh, host 54/54, targets 18/18 + 17/1.

## 5. A0 cần quyết

1. Merge theo thứ tự **ST-300b → ST-301 → ST-303** (nhánh đã rebase trên 0.4.5, gate 3660/3663/3668).
2. **ST-302** (Pha 2b) + **ST-304** (ViShell) + **ST-305** (LSP/VS Code) hướng `v0.5.0`.
3. Chốt **R36**/`v0.5.0`; R34/R35 JSON do A0 rà soát theo `VD-142`.
4. Duy trì caveat Windows **Tier 1**: `FsGuard` chỉ kiểm path, không file identity/Landlock.

## 6. Rủi ro

- **2 stack overflow wasm** (interp `de-quy-1000`, `de-quy-vo-han`); VM pass — cần task tăng
  stack/tail-call nếu muốn 390/390.
- **P2b ratio 4,0×** chưa đạt ≤1,5× — NO-GO, không đổi production; ghi nợ hiệu năng.
- **LSP timing dưới tải** — đã có flake fix `044b289`, tiếp tục theo dõi trong soak.

## 7. Kết

- Đĩa: `17 G`; **R7 xanh** (không log >50 KB mới trong repo); **0 unsafe** trong mã ViScript.
- Không GitHub Actions; tag `v0.4.5` trên commit R35; macOS ngoài phạm vi.
