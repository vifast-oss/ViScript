# R34 — ViScript, vòng 34 (Giai đoạn 1.5 `v0.4.1` — **CHỐT**)

> **Trạng thái: R34 CHỐT.** Tag **`v0.4.1` @ `90844bc`** (annotated, đã push);
> tag trước **`v0.4.0` @ `c2accdf`**, `v0.3.6` @ `8fa2c09`.
> Baseline phát hành: **`90844bc`**; sau tag `main` đã merge **ST-297** + flake fix tại **`f4cb136`**, rồi **ST-299b/P2b-3/4** tại **`16fba3a`**, **ST-300** tại **`3581c9a`/`8286caf`**.
> Không push/merge/tag từ nhánh tài liệu này; không GitHub Actions.

## 1. Phạm vi

Vòng 34 đóng **Giai đoạn 1.5 (`v0.4.1`)** theo `VD-281`: tối ưu repo & DevEx sau `v0.4.0` —
gom test binary, tách `vis-loader`/`vis-lsp`, tách các tệp sát trần R1, dọn `agent/` và lập
`DECISIONS-INDEX`. Bản phát hành **docs/version-only** trên nền `a1730c5`; `ST-296a/b/c/d` đã
merge `main`; version **0.4.1** + cache re-pin. **Tag `v0.4.1` @ `90844bc`**.
Sau tag: **ST-297 đã merge** (`f4cb136`, **#149 đóng**), flake fix **`044b289`**, **ST-299b**
(`3b8049c`) và **P2b-3/4 NO-GO** (`16fba3a`); **ST-298 đang chạy** (chưa tuyên bố Windows Tier 1).

## 2. Bằng chứng Giai đoạn 1.5

| Task | Trạng thái | Commit / số đo |
|---|---|---|
| **ST-296a** — gom test binary | **DONE — merged** | `53990c0`: `vis-cli` **280 → 22** binary; gate **352 → 92** binary; gate **3656/4**; wall **304 → 210,77 s** |
| **ST-296b** — tách `vis-loader` + `vis-lsp` | **DONE — merged** | `067e1a5`/merge `1b27cb8`: gate **3656/4**; Windows post-merge **3607/0/3**; `lsp` **27/27**, `loader` **29/29**, targets core **18/18** |
| **ST-296c** — tách tệp sát R1 | **DONE — merged** | `ab8d393`: `vis-types/stdlib.rs` **1456 → 23 + 11 module**; sqlite tests **1495 → 68 + 3**; `lower/expr.rs` **1456 → 702 + 2**; `lower.rs` **1446 → 420 + 2**; interp stdlib **68 → 17 tệp**; gate **3656/4**, diff 100k **0 lệch** |
| **ST-296d** — archive + index | **DONE — merged** | `a1730c5`: **63 tệp R100** vào `agent/archive/`; `DECISIONS-INDEX` **271 VD**; `agent/` rời **77 → 19**; report-check **PASS 33/329** |
| **ST-297** — LSP URI/UNC + `--grant` Windows (`#149`) | **DONE — merged; #149 đóng** | `f4cb136` (nhánh `4cad7a0`): gate **3660/4** (4 ca mới UNC/mixed/percent + `--grant` Windows); Windows **3× full 3611/0/3**, host 54/54, targets 18/18+17/1; Linux **10×** `vis-lsp` **29/29**, `path_impl` **2/2** (`8b03077`); **#149 code/test đã xong** |
| **ST-299b** — `vis-loader` vào `vis-wasm-run` | **DONE — merged** | `3b8049c`: nạp **đa tệp + `use`** trong WASI; native **29/29**, wasm **28/28** |
| **ST-300** — harness `test/sem` trên WASI | **DONE — merged** | `3581c9a` + `8286caf`: `bin/vis-wasm-sem` **388/390 eligible = 99,5%** (2 stack-overflow interp, VM pass); suite cũ **28/28**, native **29/29**, row **`wasm32-wasip1 (runtime)`** trong `vis-targets` |
| **P2b-3/4** — completion slot / runner reuse | **NO-GO** | `16fba3a`: bench tỉ lệ **4,0×**, không đạt ≤1,5×; **không đổi production** |
| **Flake fix** — thread-count sau join | **DONE — merged** | `044b289`: `vs3_worker::spawn_shared_stress_rounds_read_only` poll **2 s** sau join (T-INFRA; **0/60** đỏ dưới tải; test-only) |

- **Version/cache**: workspace **0.4.1**; `PHIEN_BAN` `+v041-release-2026-11-23`; khoá cache
  **`42f5389364ae126f8b9dea4c3bf107c983c3b92d7b1743a5293d547f3d4d7c0f`**; `assert_ne` khoá 0.4.0
  **`22572d0b…3a8b`** xanh; `VERSION` VBC1/ABI không đổi.
- **Tag**: **`v0.4.1` @ `90844bc`** (annotated, pushed); **`v0.4.0` @ `c2accdf`**.
- **ST-298**: **3× `vis-gate-windows --full` PASS 3611/0/3** @ `f4cb136` (host 54/54, targets
  18/18 + 17/1, dirty 0); **soak 4h đang chạy** — `985` vòng, `0` fail,
  fd/RSS `free RAM 8204–8766 MB; proc/threads 0/0 sau mỗi vòng; fd/handle không đo được sau soak (NO_PROC; không còn tiến trình vis/cargo/rustc); dirty cuối 0`; **Windows Tier 1 CHƯA công bố**.
- **Final Linux soak** `45122b7`/`84e8eb6`: **10/10** (759 s), 3× diff 100k `lech=0`, host
  54/54, cache 5/5, wasm **28/28**.

## 3. Cổng

| Cổng | Kết quả |
|---|---|
| `./vis-dev gate --fast` @ `a1730c5` | **PASS — 3656 passed / 4 skipped**; gate **92** binary; wall **210,77 s** (từ 304 s) |
| ST-297 gate @ `f4cb136` | **PASS — 3660/4** (4 ca mới); Windows **3× full 3611/0/3**, host 54/54, targets 18/18+17/1; Linux **10×** `vis-lsp` **29/29**, `path_impl` **2/2** (`8b03077`) |
| ST-299b @ `3b8049c` | native **29/29**, wasm **28/28**, `vis-loader` đa tệp + `use` |
| ST-300 @ `3581c9a`+`8286caf` | `vis-wasm-sem` **388/390 = 99,5%**; 2 stack-overflow interp (VM pass); suite cũ **28/28**, native **29/29**; targets có row wasm runtime |
| P2b-3/4 @ `16fba3a` | **NO-GO**: bench 4,0×, không đổi production |
| `report-check --all` | **PASS — 33 báo cáo, 329 hàng QUEUE** |
| fmt / clippy / R1..R7 | sạch (`vis-dev check` PASS) |
| Windows post-merge ST-296b | **3607/0/3**; lsp 27/27, loader 29/29, targets core 18/18 |
| ST-296c diff 100k | **0 lệch** |
| `vis --version` | **`vis 0.4.1`** |
| ST-298 | **3× full PASS 3611/0/3** @ `f4cb136`; soak 4h `985`/`0`/`free RAM 8204–8766 MB; proc/threads 0/0 sau mỗi vòng; fd/handle không đo được sau soak (NO_PROC; không còn tiến trình vis/cargo/rustc); dirty cuối 0`; chưa công bố Tier 1 |

## 4. Tài liệu & phạm vi

- `CHANGELOG.md` mục `v0.4.1` — ST-296a/b/c/d, tag `v0.4.1 @ 90844bc`; ghi ST-297 merged +
  flake fix + ST-298 đang chạy.
- `agent/ROADMAP.md`/`agent/ACTIVE.md`: `v0.4.0 @ c2accdf`; `v0.4.1 @ 90844bc`; ST-296a/b/c/d
  **DONE/merged**; ST-297 **DONE/merged** `f4cb136` (gate 3660/4; `#149` đóng); ST-298
  **RUNNING**, chưa Tier 1.
- **Không sửa** `agent/QUEUE.md`, `agent/SO-NO.md`, `agent/DECISIONS.md`, **R33/R34 JSON**.
- **Không tạo/cập nhật `R34-viscript.json`** trong lượt này: R33 JSON giữ baseline `90cfc39`;
  JSON R34 do **A0** chốt. Báo cáo này là Markdown đầy đủ, không phải draft.

## 5. Sau tag `v0.4.1`

- **ST-297** đã merge `f4cb136`: **#149 code/test đã xong** — Windows 3× full **3611/0/3**, Linux
  10× `vis-lsp` 29/29 (`8b03077`); **đóng `#149` trong `SO-NO`** ở lượt tài liệu này.
- **ST-299b** đã merge `3b8049c`: `vis-loader` đa tệp trong `vis-wasm-run`, native 29/29, wasm 28/28.
- **P2b-3/4 NO-GO** `16fba3a`: bench 4,0×, không đổi production.
- **ST-298**: **3× full PASS 3611/0/3** @ `f4cb136` (host 54/54, targets 18/18 + 17/1, dirty 0);
  **soak 4h đang chạy**: `985` vòng, fail `0`, fd/RSS `free RAM 8204–8766 MB; proc/threads 0/0 sau mỗi vòng; fd/handle không đo được sau soak (NO_PROC; không còn tiến trình vis/cargo/rustc); dirty cuối 0`;
  **chưa** tuyên bố Windows Tier 1 — điều kiện: soak đủ vòng, fail 0, fd/RSS phẳng.
- **ST-300** đã merge (`3581c9a` + `8286caf`); final Linux soak `45122b7`/`84e8eb6` 10/10 759 s.
- Flake thread-count đã xử lý test-only (`044b289`): poll 2 s sau join, 0/60 đỏ dưới tải.
- Không mở rộng phạm vi sang code/API mới trong bản docs/version-only này.

## 6. Ghi chú sổ sách

Baseline phát hành **`90844bc`** (tag `v0.4.1`); `main` sau tag **`16fba3a`** (ST-297, flake fix, ST-299b, P2b-3/4 NO-GO).
R33 JSON **giữ nguyên baseline `90cfc39`**, không sửa; JSON R34 **không được tạo** trong lượt này
(A0 chốt). Trạng thái task lấy đúng `QUEUE.md`/`SO-NO.md` — báo cáo **không sửa** hai sổ đó.
Tag **`v0.4.0` @ `c2accdf`**; **`v0.4.1` @ `90844bc`**; tag cũ hơn `v0.3.6` @ `8fa2c09`.
