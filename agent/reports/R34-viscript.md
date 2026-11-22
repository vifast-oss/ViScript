# R34 — ViScript, vòng 34 (Giai đoạn 1.5 `v0.4.1` — **CHỐT**)

> **Trạng thái: R34 CHỐT.** Tag **`v0.4.1` trên commit R34 này** (final QA Linux/Windows và `./vis-dev check` đã xanh). Tag trước: **`v0.4.0` @ `c2accdf`**; `v0.3.6` @ `8fa2c09`.
> Baseline sản phẩm: **`11e8120`** (`5d7791c` + lock `vis-host-wasm` 0.4.1; merge đủ `ST-296a/b/c/d`).
> Không GitHub Actions.

## 1. Phạm vi

Vòng 34 đóng **Giai đoạn 1.5 (`v0.4.1`)** theo `VD-281`: tối ưu repo & DevEx sau `v0.4.0` —
gom test binary, tách `vis-loader`/`vis-lsp`, tách các tệp sát trần R1, dọn `agent/` và lập
`DECISIONS-INDEX`. Phạm vi bản phát hành **docs/version/lock-only** trên nền `5d7791c`/`11e8120`; `ST-296a/b/c/d`
đã merge `main`; version **0.4.1** + cache re-pin; **tag `v0.4.1` đặt trên commit R34 này** sau
final QA Linux/Windows. **ST-297** (nhánh READY) và **ST-298** (BLOCKED) **không** nằm trong bản này.

## 2. Bằng chứng Giai đoạn 1.5

| Task | Trạng thái | Commit / số đo |
|---|---|---|
| **ST-296a** — gom test binary | **DONE — merged** | `53990c0`: `vis-cli` **280 → 22** binary; gate **352 → 92** binary; gate **3656/4**; wall **304 → 210,77 s** |
| **ST-296b** — tách `vis-loader` + `vis-lsp` | **DONE — merged** | `067e1a5`/merge `1b27cb8`: gate **3656/4**; Windows post-merge **3607/0/3**; `lsp` **27/27**, `loader` **29/29**, targets core **18/18** |
| **ST-296c** — tách tệp sát R1 | **DONE — merged** | `ab8d393`: `vis-types/stdlib.rs` **1456 → 23 + 11 module**; sqlite tests **1495 → 68 + 3**; `lower/expr.rs` **1456 → 702 + 2**; `lower.rs` **1446 → 420 + 2**; interp stdlib **68 → 17 tệp**; gate **3656/4**, diff 100k **0 lệch** |
| **ST-296d** — archive + index | **DONE — merged** | `a1730c5`: **63 tệp R100** vào `agent/archive/`; `DECISIONS-INDEX` **271 VD**; `agent/` rời **77 → 19**; report-check **PASS 33/329** |

- **Version/cache/lock (`5d7791c` + `11e8120`)**: workspace **0.4.1**; `vis-host-wasm/Cargo.lock`
  ghim **0.4.1** (final QA Windows phát hiện lệch 0.4.0 → đã sửa, `bin/vis-targets` không còn làm bẩn cây); `PHIEN_BAN`
  `+v041-release-2026-11-23`; khoá cache mới **`42f5389364ae126f8b9dea4c3bf107c983c3b92d7b1743a5293d547f3d4d7c0f`**;
  `assert_ne` khoá 0.4.0 **`22572d0b…3a8b`** xanh; `VERSION` VBC1/ABI không đổi.
- **Tag**: **`v0.4.0` @ `c2accdf`**; **`v0.4.1` — tag trên commit R34 này**.
- **Nhánh chưa merge**: **ST-297 READY** `plan/w2-st297-paths` @ `4cad7a0` (gate **3660/4**, 4 ca
  mới) chờ **sau** `v0.4.1`; **ST-298 BLOCKED**.

## 3. Cổng

| Cổng | Kết quả |
|---|---|
| `./vis-dev gate --fast` @ `a1730c5` | **PASS — 3656 passed / 4 skipped**; gate **92** binary; wall **210,77 s** (từ 304 s) |
| **Final Linux @ `5d7791c`** (fresh clone) | gate **3656/4**; release **3661/0/4** (445 s); **soak 10/10 = 530 s** (3× gate 3656/4, 3× diff 100k `lech=0`, host 54/54, cache 5/5); `vis 0.4.1` |
| **Final Windows @ `5d7791c` + lock `11e8120`** | full **3607/0/3** (478 s); release **3606/0/3** (384 s); worker_frame 12/12+6/6, P1/P2/P3 8/14/10, select 25, h2 33, cancel 8, lsp 27, loader 29; `vis 0.4.1`; khoá cache `42f53893…7c0f` |
| `report-check --all` | **PASS — 33 báo cáo, 329 hàng QUEUE** |
| fmt / clippy / R1..R7 | sạch (`vis-dev check` PASS) |
| Windows post-merge ST-296b | **3607/0/3**; lsp 27/27, loader 29/29, targets core 18/18 |
| ST-296c diff 100k | **0 lệch** |
| `vis --version` | **`vis 0.4.1`** |
| ST-297 (ngoài bản này) | branch `4cad7a0`, gate **3660/4**, 4 ca path-split mới |

## 4. Tài liệu & phạm vi

- `CHANGELOG.md` thêm `## v0.4.1` (ST-296a/b/c/d + số đo + tag `v0.4.0 @ c2accdf`).
- `agent/ROADMAP.md`/`agent/ACTIVE.md`: `v0.4.0` đã tag `c2accdf`; ST-296a/b/c/d **DONE/merged**;
  ST-297 **READY** `plan/w2-st297-paths` @ `4cad7a0` (gate 3660/4, 4 ca mới) chờ sau `v0.4.1`;
  **ST-298 BLOCKED**.
- **Không sửa** `agent/QUEUE.md`, `agent/SO-NO.md`, `agent/DECISIONS.md`, **R33 JSON**.
- **Không tạo/cập nhật `R34-viscript.json`** trong lượt này: R33 JSON giữ baseline `90cfc39`;
  JSON R34 do **A0** chốt sau final QA. Báo cáo này là Markdown đầy đủ, không phải draft.

## 5. Việc còn lại trước tag `v0.4.1`

- **Final QA Linux/Windows** trên `5d7791c` (+ lock `11e8120`): fresh-clone gate **3656/4**,
  release **3661/0/4**, soak **10/10** 530 s; Windows full **3607/0/3**, release **3606/0/3**;
  blocker lock `vis-host-wasm` 0.4.0 đã sửa và xác nhận `vis-targets` sạch. **Tag trên commit này.**
- Tag **`v0.4.1` trên commit R34 này** (check/gate đã xanh).
- Sau tag: xét merge **ST-297** (`plan/w2-st297-paths` @ `4cad7a0`) và xử lý **ST-298 BLOCKED**.
- Không mở rộng phạm vi sang code/API mới trong bản docs/version-only này.

## 6. Ghi chú sổ sách

`head` baseline sản phẩm là **`11e8120`**; R34 Markdown phản ánh cây sau merge GĐ 1.5 + lock fix. R33 JSON
**giữ nguyên baseline `90cfc39`**, không sửa; JSON R34 **không được tạo** trong lượt này (A0 chốt
sau final QA). Trạng thái task lấy đúng `QUEUE.md`/`SO-NO.md` — báo cáo **không sửa** hai sổ đó.
Tag **`v0.4.0` @ `c2accdf`**; **`v0.4.1` trên commit R34**; tag cũ hơn `v0.3.6` @ `8fa2c09`.
