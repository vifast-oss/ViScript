# Báo cáo sáng 2026-11-23 — ViScript (khung)

> **Trạng thái**: khung báo cáo sáng; số liệu đã xác nhận, chỗ cần đo điền sau.
> **Tag**: `v0.4.0 @ c2accdf`; **`v0.4.1 @ 90844bc`** (annotated, pushed).
> **Main**: `3581c9a` + các merge sau (nếu HEAD đã tiến, dùng HEAD thực tế khi chốt).
> Docs-only; không merge/push/tag trong lượt này; không GitHub Actions.

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

## 3. Sau tag — ST-297 / ST-299b / ST-300

- **ST-297** merged `f4cb136`: gate **3660/4** (4 ca mới UNC/mixed/percent + `--grant` Windows);
  Windows **3× full 3611/0/3**, host 54/54, targets 18/18 + 17/1; Linux **10×** `vis-lsp`
  29/29, `path_impl` 2/2 (`8b03077`); **#149 đóng** (SO-NO §8: 84 mở / 87 đóng / 192 dòng).
- **ST-299b** merged `3b8049c`: `vis-loader` vào `vis-wasm-run` (đa tệp + `use`);
  native **29/29**, wasm **28/28**.
- **ST-300 harness** `3581c9a`: `test/sem` WASI trên oracle native — **388/390 = 99,5%**
  eligible; 2 fail còn lại là **stack overflow interp WASI** (VM pass), đã ghi rõ.
- **P2b-3/4 NO-GO** `16fba3a`: bench **4,0×**, không đạt ≤1,5×, không đổi production.
- Flake fix `044b289`: poll 2 s sau join (test-only; 0/60 đỏ dưới tải).

## 4. ST-298 — soak 4h (chỗ trống)

- Trạng thái: **đang chạy** trên `f4cb136`; 3× `vis-gate-windows --full` **đã xanh 3611/0/3**;
  soak 4h: `[SOAK_ROUNDS]` vòng, fail `[SOAK_FAIL]`, fd/RSS `[SOAK_FD_RSS]`.
- **Windows Tier 1 CHƯA công bố.** Điều kiện công bố: soak 4h đủ vòng, `[SOAK_FAIL]` = 0,
  fd/RSS phẳng (`[SOAK_FD_RSS]`), 3× full xanh, host 54/54, targets 18/18 + 17/1.

## 5. A0 cần quyết

1. Cập nhật `QUEUE.md`/JSON báo cáo (R33 JSON giữ baseline `90cfc39`; R34 JSON A0 chốt).
2. Công bố **Windows Tier 1** sau khi soak 4h đạt điều kiện §4.
3. Phạm vi release **`v0.4.5`** (ST-300/harness WASI + ST-298) và mốc tag.
4. Mở nhánh **ST-301** (Pha 2 ngôn ngữ) sau khi chốt Tier 1/release.

## 6. Rủi ro

- **2 stack overflow wasm** (interp `de-quy-1000`, `de-quy-vo-han`); VM pass — cần task tăng
  stack/tail-call nếu muốn 390/390.
- **P2b ratio 4,0×** chưa đạt ≤1,5× — NO-GO, không đổi production; ghi nợ hiệu năng.
- **LSP timing dưới tải** — đã có flake fix `044b289`, tiếp tục theo dõi trong soak.

## 7. Kết

- Đĩa: `[DISK]`; **R7 xanh** (không log >50 KB mới trong repo); **0 unsafe** trong mã ViScript.
- Không GitHub Actions; không merge/push/tag trong lượt soạn khung.
