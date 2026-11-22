# R33 — ViScript, vòng 33 (Giai đoạn 1 `v0.4.0` — **CHỐT**)

> **Trạng thái: R33 CHỐT.** Tag **`v0.4.0` trên commit R33 này** (W4/A0 đặt sau khi
> `./vis-dev check` xanh); tag trước vẫn **`v0.3.6` @ `8fa2c09`**.
> Baseline sản phẩm: **`e47b3ca`** (Linux + Windows final QA); version bump **`8696edc`**
> (Linux gate 3656/4); Windows release **`8696edc`**. Không push/merge/tag từ W3; không GitHub Actions.

## 1. Phạm vi

Vòng 33 đóng **Giai đoạn 1 — G4a Core (`v0.4.0`)** theo `VD-280`/`VD-281`: ngân sách mặc định,
worker pool/frame, `select`, `async-h2` Windows/default, đóng TOCTOU ghi Unix, và QA/release.
`ST-290`..`ST-295`, `ST-296d`, `ST-299a` đã vào `main`; final QA Linux + Windows đã xong trên
`e47b3ca`; release `v0.4.0` đã verify trên Windows @ `8696edc`. R33 này là **báo cáo chốt**;
tag `v0.4.0` đặt trên chính commit tài liệu này.

## 2. Bằng chứng Giai đoạn 1

| Task | Trạng thái | Commit / bằng chứng |
|---|---|---|
| **ST-290** — ngân sách `40M` + `VS0509`/`VS0510` + cache `VFEE` | **DONE** (A0 `VD-281`) | `3627f3bc` (merge `a48d5b8`); workspace **3594/0/3**, diff 0; `JOURNAL/st290-budget-default.md` |
| **ST-291** — P2b-1/2 lazy/batched submit + compact `Vec` proxy | **DONE** | `269ff71` (merge `8cc3588`); gate **3638/4**, diff 100k **0 lệch**, Windows verify **3588/0/3**; `JOURNAL/st291-p2b.md` |
| **ST-292** — P4a/P4b frame + worker pump, `VS0509`/`VS0510`, CLI `--workers` | **DONE** | `eca4e82` (merge `e47b3ca`); gate Linux **3656/4**, diff 100k **0 lệch**; 12+6 ca frame; budget CLI `--workers 4 --max-budget 5000` → **101 + VS0506/VS0510**; `JOURNAL/st292-p4-frame-worker.md` |
| **ST-293** — `select_recv*`/`select_send*` | **DONE** (A0 `VD-282`) | `82064e9` (merge `6d6bfe9`); **25/25** Linux + Windows; diff **0 lệch**; fairness/waiter; `JOURNAL/st293-select.md` |
| **ST-294** — `async-h2` Windows + default non-wasm | **DONE** (A0 `VD-281`) | `39cc81cc` (merge `e35dcf6`); Windows h2 **33/33**; Linux gate **3607/4** |
| **ST-295** — `rustix` `openat2`/`O_NOFOLLOW` + Landlock `#125` | **DONE** (A0 `VD-282`) | `ba4023f` (merge `cd76da5`); race ghi **100/100 → 0/100**; Landlock **18/18**; gate **3613/4**, diff 0 |
| **ST-296d** — luật R7, nextest `retries=0`, bộ công cụ vận hành | **DONE** | `b36f8a5` + `4b2484f`; gate **3638/4**, fixtures 15/15; Windows quick **pkg 25 / h2 33 / cancel 8 / select 25** |
| **ST-299a** — `WasiHost` lõi + `vis-wasm-run` (`wasm32-wasip1`) | **DONE** | `5cc1903` (merge `060a744`); native **27/27**, wasm **26/26** trên wasmtime 51 + Node 26; binary **3.430.132 B** |

- **Timing fix** `6c7e188`: `vd270_test_slowest` chịu tải, không `#[ignore]`, giữ kiểm regression.
- **Version bump / release** `8696edc` (Linux gate **3656/4**): workspace **0.4.0**, `PHIEN_BAN`
  `+v040-release-2026-11-23`, khoá cache **`22572d0b3267c93ddecc223a7522cc69767b3a5c1f6b46f8bff04d95d3803a8b`**,
  `assert_ne` khoá 0.3.6 xanh.

### 2.1 Linux final QA @ `e47b3ca`

- Gate **3656 passed / 4 skipped** (352 binary, wall **195,84 s**); release **3661/0/4** (445,18 s).
- **2× soak `bin/vis-soak` 10/10** (531 s + 642 s): mỗi soak **3× gate 3656/4**, **3× diff 100k
  seed `20261115` `lech=0`**, **2× host 54/54**, **2× cache 5/5**.
- Diff ngoài soak seed **`202611230230`**: **`lech=0`** (100.000 chương trình; 69,2 s).
- Targeted: worker_frame **12/12**, frame_vm **6/6**, P1 **8/8**, P2 **14/14**, P3 **10/10**,
  h2 **34+1**, cancel **8/8**, select **25/25**, pkg **25/25**, host **54/54**.
- Budget CLI: `--workers 4 --max-budget 5000` → **101 + VS0506/VS0510** (parity W=1).

### 2.2 Windows final QA @ `e47b3ca`

- Full **3607/0/3** (361 block, 700 s); release **3606/0/3** (534 s).
- worker_frame **12/12**, frame_vm **6/6**, P1 **8**, P2 **14**, P3 **10**, select **25**,
  h2 **33**, cancel **8**, host **54/54**, targets **18/18 + 17/1**.

### 2.3 Windows release `0.4.0` @ `8696edc`

- `vis.exe` **11.530.240 B**, sha256 **`3813a1b3e7aa2f8608aaf0a168e2dffbc0d904554bf0a096fee5927a821cb065`**;
  `vis 0.4.0`.
- `st289_version` **2/2**; `cache::tests` **3/3** (khoá mới `22572d0b…3a8b`, `assert_ne` khoá cũ xanh);
  worker_frame **12/12 + 6/6**; budget parity 101/VS0510; `--workers 0` rc **2**.

## 3. Debt audit (`VD-281` §2.6)

Nguồn: **`JOURNAL/st296-debt-audit.md`** (commit `d442b1f`). Đề xuất **đóng**:
`#16` (`d060f5a`, chan0 13/13), `#17` (`8da533b`+`82064e9`, timeout 17/17 + select 25/25),
`#21` (`a5e3040`+`b7385e9`, TLS 21/21 + keepalive 8/8), `#26` (`9efa420`, LSP module-aware),
`#84` (`096ea2c`, cookie/auth 11/11), `#191` (nextest 3583/0/3). **Cần A0** chốt phạm vi:
`#19` (cap0 chỉ frame/async), `#31` (entry+GC xong, lock vẫn `main_dir`), `#27` (incremental xong,
EOF/một luồng khác spec), `#98` (publish tệp phụ chưa mở). Báo cáo **không** sửa
`SO-NO.md`/`QUEUE.md`.

## 4. Sau R33 / tag `v0.4.0`

- Tag **`v0.4.0` trên commit R33 này** (sau `./vis-dev check` xanh); tag trước **`v0.3.6` @
  `8fa2c09`**.
- **Hai nhánh READY chưa merge** (merge **sau** tag): `plan/w4-st296a-tests` @ `f961b83` (gom
  272 tệp test → 12 binary + common + 3 standalone, gate 3638/4, 350→90 binary, wall −28%) và
  `plan/w3-st296b-crates` @ `7c4c205` (tách `vis-loader`/`vis-lsp`, gom `workspace.dependencies`,
  gate 3638/4, targets core 18/18, diff 0).
- **JSON R33 giữ nguyên baseline `90cfc39`** — không tạo/cập nhật `R33-viscript.json` trong lượt này.
- Không sửa `SO-NO.md`/`QUEUE.md`/mã sản phẩm.

## 5. Cổng

| Cổng | Kết quả |
|---|---|
| Linux gate `./vis-dev gate --fast` @ `e47b3ca` | **PASS — 3656 passed / 4 skipped** (352 binary, 195,84 s) |
| Linux release workspace @ `e47b3ca` | **3661 passed / 0 failed / 4 ignored** (445,18 s) |
| Linux soak | **2× 10/10** (531 s + 642 s), gate/diff/host/cache đủ |
| Linux diff ngoài soak seed `202611230230` | **0 lệch** (100.000; 69,2 s) |
| Linux targeted | worker_frame 12/12, frame_vm 6/6, P1 8/8, P2 14/14, P3 10/10, h2 34+1, cancel 8/8, select 25/25, pkg 25/25, host 54/54 |
| Windows full/release @ `e47b3ca` | **3607/0/3** (700 s) / **3606/0/3** (534 s) |
| Windows release `0.4.0` @ `8696edc` | `vis.exe` 11.530.240 B, sha256 `3813a1b3…cb065`, `vis 0.4.0`; version 2/2, cache 3/3, frame 12/12+6/6, budget 101/VS0510 |
| `./vis-dev check` | **PASS** (fmt/clippy/R1..R7/report-check/doc-examples) |

## 6. Ghi chú sổ sách

R33 **CHỐT**; tag **`v0.4.0` trên commit R33 này**; tag trước **`v0.3.6` @ `8fa2c09`**.
Báo cáo Markdown này phản ánh `main` @ `ab17f1a` (sau `8696edc`/`e47b3ca`); JSON R33 **giữ
nguyên baseline `90cfc39`**, không tạo/cập nhật JSON. Trạng thái task lấy đúng
`QUEUE.md`/`SO-NO.md` — tệp này **không sửa** hai sổ đó. Hai nhánh `ST-296a`/`ST-296b` **READY,
chưa merge**, sẽ xét sau tag.
