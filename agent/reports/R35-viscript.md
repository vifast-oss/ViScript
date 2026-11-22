# R35 — ViScript, vòng 35 (Giai đoạn 2 `v0.4.5` — **CHỐT**)

> **Trạng thái: R35 CHỐT phần harness/QA; `v0.4.5` CHƯA TAG.**
> Tag trước: **`v0.4.0` @ `c2accdf`**, **`v0.4.1` @ `90844bc`**. Baseline tài liệu/JSON:
> **`9af3976`** (sẽ cập nhật head khi A0 chốt soak + release). Không merge/push/tag từ nhánh soạn.

## 1. Phạm vi

Giai đoạn 2 — **G4b Platforms (`v0.4.5`)**: đóng `#149` path-split đa nền (ST-297), xác nhận
Windows Tier 1 (ST-298), `WasiHost` lõi (ST-299a), `vis-loader` đa tệp trên WASI (ST-299b), và
harness `test/sem` WASI + row `vis-targets` (ST-300). Version/tag `v0.4.5` **chưa** thực hiện;
`R35` này chốt bằng chứng trước khi W4 tag.

## 2. Bằng chứng Linux / Windows / WASI

| Hạng mục | Kết quả |
|---|---|
| **ST-297** `f4cb136` | gate **3660/4** (4 ca mới UNC/mixed/percent + `--grant` Windows); Windows **3× full 3611/0/3**, host 54/54, targets 18/18 + 17/1; Linux **10×** `vis-lsp` 29/29, `path_impl` 2/2 (`8b03077`); **`#149` đóng** |
| **ST-298** `f4cb136` | **3× `vis-gate-windows --full` PASS 3611/0/3** (host 54/54, targets 18/18 + 17/1, dirty 0); **soak 4h đang chạy** — `[SOAK_ROUNDS]` vòng, `[SOAK_FAIL]` fail, fd/RSS `[SOAK_FD_RSS]`; **Windows Tier 1 CHƯA công bố** |
| **ST-299a** `5cc1903`/`060a744` | `WasiHost` `wasm32-wasip1` + `vis-wasm-run`; wasm **26/26** (13 ca × 2 đường) |
| **ST-299b** `3b8049c`/`aab5ffa` | `vis-loader` đa tệp + `use` trên WASI; native **29/29**, wasm **28/28** |
| **ST-300** `3581c9a` + `8286caf` + `b3a9742` | `bin/vis-wasm-sem`: **388/390 eligible = 99,5%** (VM **390/390**; 2 stack-overflow interp known — nợ **`#193`**); suite cũ **28/28**, native **29/29**; row **`wasm32-wasip1 (runtime)`** trong `vis-targets` |
| **Final Linux soak** `45122b7`/`84e8eb6` | **10/10** (759 s); 3× diff 100k `lech=0`, host **54/54**, cache **5/5**, wasm **28/28** |

- **#149** đã đóng (SO-NO: `f4cb136`, nhánh `4cad7a0`); **#193** mở (2 stack overflow interp
  WASI, VM pass — đề xuất task tăng stack/tail-call).
- **P2b-3/4 NO-GO** `16fba3a` (bench 4,0×, không đổi production); flake fix `044b289`.

## 3. Cổng / release

| Cổng | Kết quả |
|---|---|
| Linux `./vis-dev gate --fast` @ `f4cb136` | **PASS — 3660 passed / 4 skipped** |
| Linux final soak @ `84e8eb6` | **10/10** (759 s), 3× diff 0, host 54/54, cache 5/5 |
| Windows `--full` @ `f4cb136` | **3× PASS 3611/0/3**; host 54/54; targets 18/18 + 17/1 |
| WASI `vis-wasm-sem` | **388/390 = 99,5%**; suite cũ 28/28; native 29/29 |
| Release Linux/Windows `v0.4.5` | **chưa chạy** — W4 thực hiện trước tag (điều kiện `VD-283`) |
| Tag | **`v0.4.0` @ `c2accdf`**, **`v0.4.1` @ `90844bc`**; **`v0.4.5` CHƯA TAG** |

## 4. JSON & sổ

- `agent/reports/R35-viscript.json`: **330 task** (đúng **330 hàng QUEUE@`9af3976`**, gồm
  `ST-300b` mới), **10 blocked** (8 BLOCKED + ST-298/ST-300 REVIEW), `round=35`, `head=9af3976`.
  Không placeholder `[SOAK_*]` trong JSON; các số soak điền ở tài liệu md khi A0 chốt.
- Sinh lại bằng `python3 agent/JOURNAL/gen-r35-json.py` sau khi đổi `HEAD_SHA`.
- **Không** sửa `QUEUE.md`/`SO-NO.md`/`DECISIONS.md`; `report-check --all` PASS.

## 5. Điều kiện công bố Windows Tier 1 / tag `v0.4.5`

- **Tier 1**: soak 4h đủ vòng, `[SOAK_FAIL]` = 0, fd/RSS phẳng (`[SOAK_FD_RSS]`), 3× full
  xanh, host 54/54, targets 18/18 + 17/1, dirty 0. **Chưa công bố** ở báo cáo này.
- **Tag `v0.4.5`**: sau final QA/release Linux+Windows, cập nhật head JSON (`gen-r35-json.py`),
  điền `[SOAK_ROUNDS]`/`[SOAK_FAIL]`/`[SOAK_FD_RSS]`, `check_report.py --all` PASS, rồi tag theo
  uỷ quyền `VD-283`. Tag trước **`v0.4.1` @ `90844bc`**.

## 6. Rủi ro & ghi chú

- **2 stack overflow interp WASI** (`de-quy-1000`, `de-quy-vo-han`) — VM pass; nợ **`#193`**.
- **P2b ratio 4,0×** — NO-GO, giữ production; ghi nợ hiệu năng.
- **LSP timing dưới tải** — flake fix `044b289` (test-only), tiếp tục theo dõi.
- R7 xanh, **0 unsafe** trong mã ViScript; không GitHub Actions. Head JSON tạm `9af3976` sẽ được
  A0 cập nhật khi chốt — đây là điểm cần sửa duy nhất trước tag.
