# R33 — ViScript, vòng 33 (khung Giai đoạn 1 `v0.4.0` — **CHƯA CHỐT**)

> **Trạng thái: KHUNG BÁO CÁO — R33 CHƯA CHỐT; `v0.4.0` CHƯA TAG.**
> Baseline `head`: **`90cfc39446f499745489714727cf6987e6ee0794`** (ST-290/293/294/295
> + release h2 harness + debt audit đã merge `main`). Tag gần nhất **`v0.3.6`** @ `8fa2c09`.
> Không push/merge/tag từ W3; không GitHub Actions.

## 1. Phạm vi

Vòng 33 mở **Giai đoạn 1 — G4a Core (`v0.4.0`)** theo `VD-280`/`VD-281`: ngân sách mặc định,
worker pool/frame, `select`, `async-h2` Windows/default, đóng TOCTOU ghi Unix, và QA/release.
Báo cáo này là **khung** để W4/A0 chốt; các mục `ST-291`/`ST-292` còn **WIP/BLOCKED**, `R33`
**chưa phải báo cáo cuối vòng**, `v0.4.0` **chưa tag**.

## 2. Bằng chứng Giai đoạn 1

| Task | Trạng thái | Commit / bằng chứng |
|---|---|---|
| **ST-290** — ngân sách `40M` + `VS0509`/`VS0510` + cache `VFEE` | **DONE** (A0 `VD-281`) | `3627f3bc` (merge `a48d5b8`); workspace **3594/0/3**, diff 0; `JOURNAL/st290-budget-default.md` |
| **ST-293** — `select_recv*`/`select_send*` | **REVIEW** | `82064e9` (merge `6d6bfe9`); **25 ca**, gate **3632/4**, diff 100k seed `202611222100` **0 lệch**; fairness/waiter test; `JOURNAL/st293-select.md` |
| **ST-294** — `async-h2` Windows + default non-wasm | **DONE** (A0 `VD-281`) | `39cc81cc` (merge `e35dcf6`); Windows verify `05187a6`: workspace **3563/0/3**, h2 **33/33**; Linux gate **3607/4** |
| **ST-295** — `rustix` `openat2`/`O_NOFOLLOW` + Landlock `#125` | **REVIEW** | `ba4023f` (merge `cd76da5`); race ghi **100/100 → 0/100**, Landlock **18/18**, gate **3613/4**, diff 0; license `st295-license.log` |

- **Linux gate** @ `90cfc39`: `./vis-dev gate --fast` **PASS — 3638 passed / 4 skipped**.
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

- **ST-291 WIP** — `P2b-1/2` lazy/batched submit + compact `Vec` proxy; chưa có commit chốt.
- **ST-292 BLOCKED** — chờ `ST-291`; **bắt buộc** nối `gioi_han_phien()` vào `SharedBudget` khi
  `--workers > 1` (`VD-281`) + test `--workers 4 --max-budget 5000` trap `VS0510` giống W=1.
- **ST-293/ST-295 REVIEW** — chờ A0 duyệt/lật sổ.
- **ST-296 QA/WIP** — release h2 harness fixed `3896bc6` (3643/0/4); còn nightly/fuzz/#191 chốt,
  `R33`, `CHANGELOG`, `ROADMAP`, `ACTIVE`, và tag `v0.4.0`.

## 5. Cổng

- `fast`: **PASS** @ `90cfc39` — **3638 passed / 4 skipped** (Linux `ps`).
- `release`: **PASS** @ `5cb2dbd` — **3643 passed / 0 failed / 4 ignored**.
- `nightly`: **—** (chưa chạy lại cho vòng 33; W4 đang giữ lịch).
- `#191` Windows nextest: **3583/0/3** @ `f6939721`; release Windows `621fad0` **3588/0/3**.
- `./vis-dev check`: PASS (fmt, clippy, R1..R6, report-check 32/327, doc-examples 24 khối).

## 6. Ghi chú sổ sách

`head` là baseline sản phẩm được đo (`90cfc39`), không phải commit chứa báo cáo này. R33 là
**khung chưa chốt**; mọi trạng thái task lấy đúng `QUEUE.md` tại `head` (ST-291 `READY` nhưng
thực tế WIP, ST-296 `BLOCKED` nhưng đang QA). `v0.4.0` **chưa tag**; tag hiện hành vẫn
**`v0.3.6` @ `8fa2c09`**. Báo cáo không sửa `QUEUE.md`/`SO-NO.md`.
