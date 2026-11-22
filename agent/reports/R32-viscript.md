# R32 — ViScript, vòng 32 (Stage 0 `v0.3.6` đủ 6/6 — tag candidate `0645700`)

> **Trạng thái: Stage 0 ĐỦ 6/6; Linux final QA XANH; ST-287 fix Windows đã merge.** Sáu lát đã có
> trên `main`: `ST-284`..`ST-289`; code cuối `6b2a216` (ST-287 fix), merge **`0645700`**.
>
> **Windows h1 cancel đã fix**: root cause là **test fixture** — listener `set_nonblocking(true)`,
> Windows `accept` **thừa hưởng nonblocking**, handler server blocking gặp `WSAEWOULDBLOCK/10053` và
> đóng connection trước safe point (không phải task huỷ chạy tiếp). Fix: server handler
> `set_nonblocking(false)`; trigger test chuyển sang **byte đã đọc trên handle `C`**; product thêm
> guard `cancelled` trước `step_frame`/cache và `AsyncNet::close` deregister **trước** khi drop socket.
> Matrix sau fix @ `6b2a2161`: suite serial **20/20**, default **10/10**, ca riêng lap **20/20**;
> workspace **3538/0/3** (355 block). **Tag candidate `0645700` — W3/W4 đang verify cuối.**
>
> Baseline báo cáo: **`0645700`** (merge ST-287 fix; code `6b2a216`). Linux final QA đo tại
> **`4c5dec6`** (fresh-clone). Tag gần nhất **`v0.3.5`** @ `03bb7b9` giữ nguyên; **`v0.3.6` CHƯA tag**.
> Không push/merge/tag từ W4; không GitHub Actions.

## 1. Phạm vi

Vòng 32 mở chương trình dài hạn hậu `v0.3.5` theo `VD-272`..`VD-279`; Giai đoạn 0 (`v0.3.6`) gồm
sáu lát: R0a/R0b/R0c refactor 0 đổi hành vi (`ST-284`..`ST-286`), hook huỷ HTTP (`ST-287`), khép G3
gói (`ST-288`: `VS0708`, `pkg gc --root`, `#142`) và đồng bộ sự thật (`ST-289`: version một nguồn,
`#111`, `#151`, docs, R32). Báo cáo **chỉ** chốt phần đã có bằng chứng trên `main`; số xanh của
từng lát không thay thế cổng Windows trước tag.

## 2. Baseline được đo

- `head` = **`a1930cb`** (merge final QA; commit sản phẩm **`4c5dec6`**; QUEUE `ST-284..ST-309`
  phủ đủ 322 hàng).
- **Linux final QA — fresh-clone `4c5dec6`** (rustc 1.100.0):
  - `./vis-dev gate --fast` **PASS — 3581 passed / 0 failed / 3 skipped** (fmt, clippy, R1..R6,
    self-test, doc-examples 24 khối: 19 xanh + 5 ngoại lệ, L0 nextest).
  - `cargo test --release --workspace --no-fail-fast` **3586 passed / 0 failed / 3 ignored**
    (355 binary); +5 so với gate là doc-test nextest không đếm.
  - P2 **14/14**, P1 **8/8**, P3 **10/10**; h2 **19/19**, feature `async-h2` **34+1**;
    `pkg` **25/25**, `vs3_registry_sig` **10/10**, `vs3_http_cancel` **5/5** + feature **8/8**;
    host **54/54**; `vis 0.3.6` (debug + release).
  - Differential 100k seed **`202611221700`** **lech=0** (100 000 tổng, 6 232 lỗi tĩnh, 93 768 chạy,
    81,5 s); các seed cũ `20261115`/`20261122` cũng `lech=0`.
  - **2× soak 10/10 = 20/20 vòng** (593 s + 602 s); mỗi soak 3 gate `3581/3`, 3 diff 100k `lech=0`,
    2 host `54/54`, 2 cache `5/5`.
  - Nguồn: `JOURNAL/final-qa-v036.md`, `JOURNAL/stage0-release-13d184d.md`,
    `JOURNAL/stage0-qa-84915aa.md`.
- **Windows (Tier 2, chưa tuyên bố hỗ trợ) — ST-287 fix @ `6b2a2161`** (`psvn`, 6 CPU):
  - **Root cause**: `accept` trên Windows thừa hưởng nonblocking từ listener; test server dùng
    blocking read/write nên gặp `10035`/`10053`, đóng socket trước turn-20. Log scheduler **không có
    `cancelreq`/`cancel:1`**, chỉ `rerr:C…10053`; task in `V:post` qua nhánh lỗi HTTP hợp lệ.
  - **Fix**: `phuc_vu_h1` `set_nonblocking(false)`; trigger test `frame_cancel_after_bytes_for_test`
    (8192 byte trên handle `C`); product guard frame `cancelled` + `AsyncNet::close` deregister trước
    drop; `cancel_task_handles`/`RST_STREAM` giữ nguyên.
  - Matrix sau fix: `vs3_http_cancel` serial **20/20**, default **10/10**, ca riêng lap **20/20**
    (trước: serial 2/10 pass, default 8/10); workspace **355 block, 3538 passed / 0 failed / 3 ignored**,
    dirty 0, rc 0. `--features async-h2` Windows **BLOCKED-ENV rc 101** (Unix-only), không tính.
  - Nguồn: `JOURNAL/st287-win-cancel-flake.md`, `JOURNAL/st287-win-cancel-final-matrix.log`,
    `JOURNAL/st287-win-cancel-workspace.log`; matrix cũ `JOURNAL/win-h1-cancel-matrix.md`.
- macOS: không có máy Darwin thật — **không tuyên bố**; wasm32: chỉ `cargo check`.

## 3. Bằng chứng Stage 0 (6/6)

| Task | Trạng thái | Commit | Bằng chứng |
|---|---|---|---|
| **ST-284** — R0a `vm.rs`/`frame.rs` | **DONE** | `83a9a10` (merge `e363a15`) | `crates/vis-vm/src/vm.rs` **1240** dòng, `crates/vis-interp/src/frame.rs` **1042** dòng (ngưỡng ≤1250); tách `vm/host.rs` (205) + `frame/worker.rs` (464) giữ nguyên API; 0 đổi ABI/cache/opcode; gate `3571/3`; diff 100k seed `20261115` **lech=0** (`JOURNAL/st284-refactor.md`) |
| **ST-285** — R0b `value`/`codegen`/`tyck`/`interp/goi` | **DONE** | `ab5d292` (merge `246b6c5`) | `value.rs` **1186**, `codegen.rs` **1045**, `tyck/expr.rs` **1064**, `interp/goi.rs` **1155** (≤1250); tách `value/show_eq.rs`, `codegen/cau.rs`, `tyck/expr/goi.rs`, `interp/goi_ham.rs`; 0 đổi golden/API/cache; gate `3576/3`; diff 100k seed `20261115` **lech=0** (`JOURNAL/st285-refactor.md`) |
| **ST-286** — R0c `async_net`/`quyen`/`sqlite` | **DONE** | `f193c57` (merge `cfba662`) | `async_net.rs` **606**, `quyen.rs` **1190**, `db/sqlite/mod.rs` **1245** dòng (≤1250); module con `async_net_{conn,nguon,host,h2}.rs`, `quyen_{path,guard,chuan,st174}.rs`, `db/sqlite/{pragma_doc,schema,tach_cau,tests*}.rs`; API công khai không đổi; gate PASS; diff 100k seed `20261122` **lech=0** (`JOURNAL/st286-tach-module.md`) |
| **ST-287** — hook huỷ giữa HTTP request h1/h2 | **DONE — Windows fix đã merge** | `03ed84c` (merge `f8f1d4b`); fix `e43ca43` + `6b2a216` (merge `0645700`) | `FrameReactorBackend::cancel_task_handles(frame_id)` + `note_task_handle`/`note_task_h2`; safe point `frame_cancel_one` đóng socket h1 / `RST_STREAM` h2, session dùng chung không đóng. **Linux** h1 **5/5**, h2 **8/8**, đỏ-trước 5/5 + 7/7, soak fd/RSS/threads phẳng; gate `3581/3`; diff 100k **lech=0**. **Windows fix**: root cause test fixture nonblocking (`accept` thừa hưởng) — server blocking + trigger byte; product guard `cancelled`/close order; serial **20/20**, default **10/10**, workspace **3538/0/3** (`JOURNAL/st287-win-cancel-flake.md`) |
| **ST-288** — `VS0708` + `pkg gc --root` + `#142` | **DONE** (Gatekeeper độc lập 2026-11-22) | `c513e7a` (merge `545a38d`) | `VS0708` vi/en/ja + `vis explain`/`--list` **54** + `API-CODES.txt` **54**; **6 ca âm** (gồm key rotation) đều nêu `VS0708` (`vs3_registry_sig` 10/10); `pkg update <tên>` **soft pin** — `update_mot_ten_*` 2/2 + unit `soft_pin` ⇒ **#142 đóng**; `pkg gc --root` lặp/lồng dedupe, không tạo `.vis` (`vs3_pkg_gc_multi` 4/4); `pkg` 25/25. **QUEUE còn `REVIEW`** — chờ A0 lật sổ (`JOURNAL/st288-goi.md`) |
| **ST-289** — version `0.3.6` + `#111`/`#151` + R32 | **DONE** (docs phát hành + QA) | `ff55056` (merge `84915aa`); QA `c69127f`/`a997e06`; docs `705afcf` | 8 crate thành viên `version.workspace = true`; `vis 0.3.6`; `#111` khoá bằng `VS0412`; `#151` 24 fixture đổi tên **thuần** (git R100%); Linux final QA ở §2; R32 cập nhật fix ST-287 Windows; **tag chưa tạo** |

**Sổ nợ sau Stage 0** (`agent/SO-NO.md` §8): **192 = 93 mở / 74 đóng / 22 chuyển VS3 / 2 không làm /
1 điều hướng** — `#142` (ST-288), `#111` và `#151` (ST-289) đã đóng; ST-287 không thêm dòng nợ mới.
Windows h1 cancel **đã fix** (test fixture + P0/P1), không còn blocker đã biết; chờ W3/W4 verify cuối.

## 4. Còn chờ — trước khi tag `v0.3.6` / mở GĐ 1

- **W3/W4 verify cuối tại `0645700`** (Windows suite 20/20 + workspace 1 vòng; Linux gate 3581/3
  đã có). Khi xanh ⇒ A0 quyết định tag `v0.3.6` tại `0645700` (tag candidate).
- Linux final QA **đã xong** (gate 3581/3, release 3586/0/3, 2× soak 20/20, diff 0).
- **ST-288** đã được Gatekeeper duyệt **DONE** nhưng QUEUE tại `head` còn `REVIEW`; A0 cần lật
  `QUEUE.md` khi chốt sổ.
- Không còn blocker Windows đã biết sau fix ST-287; nếu verify cuối đỏ, giữ known-issue thay vì
  tuyên bố Windows xanh (Tier 2).

## 5. Cổng

- `fast`: **PASS** @ **`4c5dec6`** (fresh-clone) — **3581 passed / 0 failed / 3 skipped** (Linux `ps`).
- `full`: **—** (release workspace 3586/0/3 đã chạy; chưa chạy `vis-dev gate` full).
- `nightly`: **—** (chưa chạy cho Stage 0).
- Windows @ `6b2a2161` (code của `0645700`): `vs3_http_cancel` serial **20/20**, default **10/10**,
  workspace **3538/0/3**; feature h2 **BLOCKED-ENV** (Unix-only). **Tag candidate `0645700`** —
  W3/W4 đang verify cuối.
- Kiểm báo cáo: `test/report/check_report.py --all` **PASS** (32 báo cáo, 322 hàng QUEUE phủ đủ;
  22 mục `blocked[]` gồm ST-287).

## 6. Ghi chú sổ sách

`head` là **baseline sản phẩm được đo** (`0645700`; code `6b2a216`, Linux final QA tại `4c5dec6`), không phải commit
chứa báo cáo này. `ST-284`..`ST-287`/`ST-289` ghi `status = DONE` với `queue = READY` vì QUEUE tại
`head` chưa lật trạng thái; `ST-288` ghi `DONE` (Gatekeeper) + `queue = REVIEW`. Hàng QUEUE sẽ được
A0 cập nhật cùng lúc với chốt Stage 0; báo cáo này **không** sửa `QUEUE.md`.
