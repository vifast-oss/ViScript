# R33 — ViScript, vòng 33 (khung Giai đoạn 1 `v0.4.0` — **CHƯA CHỐT**)

> **Trạng thái: KHUNG BÁO CÁO — R33 CHƯA CHỐT; `v0.4.0` CHƯA TAG.**
> Baseline sản phẩm hiện tại: **`e47b3ca`** (merge **ST-292**; Linux gate **3656/4**).
> **Final QA Linux và Windows đang chạy** — chưa có kết quả cuối, chưa dùng để chốt R33.
> Tag gần nhất **`v0.3.6`** @ `8fa2c09`. Không push/merge/tag từ W3; không GitHub Actions.

## 1. Phạm vi

Vòng 33 mở **Giai đoạn 1 — G4a Core (`v0.4.0`)** theo `VD-280`/`VD-281`: ngân sách mặc định,
worker pool/frame, `select`, `async-h2` Windows/default, đóng TOCTOU ghi Unix, và QA/release.
Báo cáo này là **khung** để W4/A0 chốt. `ST-290`, `ST-291`, `ST-292`, `ST-294`, `ST-295` đã vào
`main`; `ST-293`/`ST-295` còn **REVIEW** theo sổ; **final QA Linux/Windows đang chạy**, nên R33
**chưa phải báo cáo cuối vòng** và `v0.4.0` **chưa tag**.

## 2. Bằng chứng Giai đoạn 1

| Task | Trạng thái | Commit / bằng chứng |
|---|---|---|
| **ST-290** — ngân sách `40M` + `VS0509`/`VS0510` + cache `VFEE` | **DONE** (A0 `VD-281`) | `3627f3bc` (merge `a48d5b8`); workspace **3594/0/3**, diff 0; `JOURNAL/st290-budget-default.md` |
| **ST-291** — P2b-1/2 lazy/batched submit + compact `Vec` proxy | **DONE** (merge `main`) | `269ff71` (merge `8cc3588`); gate **3638/4**, diff 100k **0 lệch**, Windows verify **3588/0/3**; `JOURNAL/st291-p2b.md` |
| **ST-292** — P4a/P4b frame + worker pump, `VS0509`/`VS0510`, CLI `--workers` | **DONE** (merge `main`) | `eca4e82` (merge `e47b3ca`); gate Linux **3656/4**, diff 100k **0 lệch**; 12+6 ca frame; `JOURNAL/st292-p4-frame-worker.md` |
| **ST-293** — `select_recv*`/`select_send*` | **REVIEW** | `82064e9` (merge `6d6bfe9`); **25 ca**, gate **3632/4**, diff 100k seed `202611222100` **0 lệch**; fairness/waiter test; `JOURNAL/st293-select.md` |
| **ST-294** — `async-h2` Windows + default non-wasm | **DONE** (A0 `VD-281`) | `39cc81cc` (merge `e35dcf6`); Windows verify `05187a6`: workspace **3563/0/3**, h2 **33/33**; Linux gate **3607/4** |
| **ST-295** — `rustix` `openat2`/`O_NOFOLLOW` + Landlock `#125` | **REVIEW** | `ba4023f` (merge `cd76da5`); race ghi **100/100 → 0/100**, Landlock **18/18**, gate **3613/4**, diff 0; license `st295-license.log` |
| **ST-296d** — luật R7, nextest `retries=0`, bộ công cụ vận hành | **DONE** (merge `main`) | `b36f8a5` + `4b2484f`; gate **3638/4**, fixtures 15/15, Windows quick **pkg 25 / h2 33 / cancel 8 / select 25**; `JOURNAL/st296d-tools*.md` |
| **ST-299a** — `WasiHost` lõi + `vis-wasm-run` (`wasm32-wasip1`) | **DONE** (merge `main`) | `5cc1903` (merge `060a744`); native **27/27**, wasm **26/26** trên wasmtime 51 + Node 26; binary 3.430.132 B; `JOURNAL/st299a-wasi-core.md` |

- **Timing fix** `6c7e188` (merge ST-296): `vd270_test_slowest` chịu tải, không `#[ignore]`;
  sửa ca chậm/ngưỡng + warmup/median; không đổi mã sản phẩm.
- **Nhánh READY, CHƯA MERGE** (A0 chưa quyết): `plan/w4-st296a-tests` @ **`f961b83`** — gom 272
  tệp test thành 12 binary + common + 3 standalone, gate **3638/4**, 350 → 90 binary, wall −28%;
  `plan/w3-st296b-crates` @ **`7c4c205`** — tách `vis-loader` + `vis-lsp`, gom
  `workspace.dependencies`, gate **3638/4**, targets core **18/18**, diff 100k **0 lệch**.
- **Linux gate** @ `90cfc39`: `./vis-dev gate --fast` **PASS — 3638 passed / 4 skipped**.
- **Linux gate** @ `e47b3ca`: **PASS — 3656 passed / 4 skipped** (ST-292).
- **Release Linux** @ `5cb2dbd` (mã tương đương `90cfc39`): `cargo test --release --workspace` —
  **3643 passed / 0 failed / 4 ignored**; targeted `vs3_select` 25, P2/P1/P3 14/8/10,
  `vs3_http2` 34+1, `vs3_http_cancel` 8/8, `pkg` 25, host 54/54, `vis 0.3.6`.
- **Soak**: 2× `bin/vis-soak` **10/10 = 20/20 vòng** @ `5cb2dbd` (830 s + 816 s); mỗi lần 3 gate
  3638/4, 3 diff 100k `lech=0`, host 54/54, cache 5/5. Thêm 1×10/10 @ `3fbf31a` (939 s).
- **Diff 100k**: seed **`202611222300`** **lech=0** (100.000; 6.231 lỗi tĩnh; 93.769 chạy; 94,9 s).
- **Windows**: release `621fad0` **3588/0/3** (`JOURNAL/...`, W4); `#191` nextest `psvn` @
  `f6939721`: **3583/3583 passed, 3 skipped, 0 fail/flake** (`JOURNAL/st296-pre-qa.md`).

## 3. Debt audit (`VD-281` §2.6)

Nguồn: **`JOURNAL/st296-debt-audit.md`** (commit `d442b1f`). Đề xuất **đóng**:
`#16` (`d060f5a`, chan0 13/13), `#17` (`8da533b`+`82064e9`, timeout 17/17 + select 25/25),
`#21` (`a5e3040`+`b7385e9`, TLS 21/21 + keepalive 8/8), `#26` (`9efa420`, LSP module-aware),
`#84` (`096ea2c`, cookie/auth 11/11), `#191` (nextest 3583/0/3). **Cần A0** chốt phạm vi:
`#19` (cap0 chỉ frame/async), `#31` (entry+GC xong, lock vẫn `main_dir`), `#27` (incremental xong,
EOF/một luồng khác spec), `#98` (publish tệp phụ chưa mở). **Không** sửa `SO-NO.md`/`QUEUE.md`.

## 4. Việc còn mở trước khi chốt R33/tag `v0.4.0`

- **Final QA Linux/Windows đang chạy** (fresh-clone, release, soak, Windows) — **chưa có kết quả
  cuối**; không dùng số liệu trung gian để tuyên bố `v0.4.0`.
- **ST-293/ST-295** — còn **REVIEW** trong sổ; chờ A0 duyệt/lật sổ.
- **Hai nhánh READY chưa merge**: `plan/w4-st296a-tests` @ `f961b83` (gom test) và
  `plan/w3-st296b-crates` @ `7c4c205` (tách crate) — cần A0 quyết merge/hoãn.
- **ST-296 QA/WIP** — `CHANGELOG`, `ROADMAP`, `ACTIVE`, `VS3-TIEN-DO`, R33/JSON cuối và tag
  `v0.4.0`; **không** sửa `SO-NO`/`QUEUE` trong báo cáo này.
- **Ràng buộc R7**: log bằng chứng >50 KB phải ở ngoài repo (kèm sha256) hoặc trong danh sách
  legacy đóng băng; không thêm mục mới.

## 5. Cổng

- `fast`: **PASS** @ `e47b3ca` — **3656 passed / 4 skipped** (Linux `ps`); gate trước @ `90cfc39`
  **3638/4**.
- `release`: **PASS** @ `5cb2dbd` — **3643 passed / 0 failed / 4 ignored** (bản trước ST-292).
- `nightly`: **—** (chưa chạy lại cho vòng 33; W4 đang giữ lịch).
- **Final QA Linux/Windows**: **đang chạy**, chưa có kết quả cuối.
- `#191` Windows nextest: **3583/0/3** @ `f6939721`; release Windows `621fad0` **3588/0/3**.
- `./vis-dev check`: PASS (fmt, clippy, R1..R6, report-check 33/329, doc-examples 24 khối); R7 đã
  bật trong `vis-dev` từ ST-296d.

## 6. Ghi chú sổ sách

`head` của JSON R33 vẫn là baseline **`90cfc39`** (không tạo/cập nhật JSON mới trong lượt này);
báo cáo Markdown này phản ánh trạng thái sau **`e47b3ca`**. R33 là **khung chưa chốt**; trạng thái
task lấy đúng `QUEUE.md`/`SO-NO.md` — tệp này **không sửa** hai sổ đó. **Final QA Linux/Windows
đang chạy**; `v0.4.0` **chưa tag**; tag hiện hành vẫn **`v0.3.6` @ `8fa2c09`**.
