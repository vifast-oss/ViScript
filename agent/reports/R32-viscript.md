# R32 — ViScript, vòng 32 (Stage 0 `v0.3.6` đủ 6/6 — **CHƯA tag**: blocker Windows h1 cancel)

> **Trạng thái: Stage 0 ĐỦ 6/6; Linux final QA XANH; `v0.3.6` CHƯA tag.** Sáu lát đã có trên `main`:
> `ST-284`..`ST-289` (commit sản phẩm cuối `4c5dec6`, merge final QA **`a1930cb`**).
>
> **Blocker trước tag**: trên Windows (`psvn`, Tier 2), `vs3_http_cancel::cancel_http_h1_lap_nhieu_vong_khong_ro`
> **flake ~21 %** (3/14 lần chạy suite, kể cả `--test-threads=1`; ca riêng ×10 10/10): task bị huỷ vẫn chạy
> tiếp và in `V:post` sau `WSAECONNABORTED 10053`. Đây là **hành vi sản phẩm** của ST-287, không chỉ test.
> Workspace Windows **lần 2 đạt 3538/0/3** (đúng kỳ vọng), nhưng lần 1 đỏ 5 ca `vs3_http_cancel` (chạy chồng
> 10 phút cuối soak `f80f774`) — cổng Windows chưa tất định. **Chờ fix ST-287 rồi verify lại trước khi tag.**
>
> Baseline báo cáo: **`a1930cb`**. Linux final QA đo tại **`4c5dec6`** (fresh-clone).
> Tag gần nhất **`v0.3.5`** @ `03bb7b9` giữ nguyên; **`v0.3.6` CHƯA tag**.
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
- **Windows (Tier 2, chưa tuyên bố hỗ trợ) — final @ `a1930cb`** (`psvn`, 6 CPU):
  - Workspace **lần 2**: **3538 passed / 0 failed / 3 ignored** (463 s) — đúng kỳ vọng (+5 ca ST-287).
  - Workspace **lần 1**: **3533 passed / 5 failed / 3 ignored** (528 s) — 5/5 `vs3_http_cancel` h1
    đỏ, chạy chồng 10 phút cuối soak `f80f774` (nghi nhiễu tải/port, nhưng vẫn là cổng chưa tất định).
  - **`vs3_http_cancel` default flake**: 3/14 lần suite đỏ (~21 %), kể cả `--test-threads=1`;
    ca riêng ×10 **10/10**; lỗi `task bị huỷ phải dừng: … V:post` sau `10053` (WSAECONNABORTED).
  - `--features async-h2` trên Windows **BLOCKED-ENV rc 101** (`E0433 net::h2_async`) — feature
    Unix-only, **không đo được 8/8**.
  - Targeted còn lại: P2 14/14, P1 8/8, P3 10/10; h2 19, `pkg` 25, sig 10, default 22, TLS 22,
    net 9; host 54/54; targets 18/18 + 17/1; release `vis.exe` **11 228 672 B**,
    sha256 `fa363043…5d3b08b`; CLI/smoke xanh.
  - Nguồn: `JOURNAL/final-win-v036.md`, log `/c/Users/ps/vislogs/final-*.log`.
- macOS: không có máy Darwin thật — **không tuyên bố**; wasm32: chỉ `cargo check`.

## 3. Bằng chứng Stage 0 (6/6)

| Task | Trạng thái | Commit | Bằng chứng |
|---|---|---|---|
| **ST-284** — R0a `vm.rs`/`frame.rs` | **DONE** | `83a9a10` (merge `e363a15`) | `crates/vis-vm/src/vm.rs` **1240** dòng, `crates/vis-interp/src/frame.rs` **1042** dòng (ngưỡng ≤1250); tách `vm/host.rs` (205) + `frame/worker.rs` (464) giữ nguyên API; 0 đổi ABI/cache/opcode; gate `3571/3`; diff 100k seed `20261115` **lech=0** (`JOURNAL/st284-refactor.md`) |
| **ST-285** — R0b `value`/`codegen`/`tyck`/`interp/goi` | **DONE** | `ab5d292` (merge `246b6c5`) | `value.rs` **1186**, `codegen.rs` **1045**, `tyck/expr.rs` **1064**, `interp/goi.rs` **1155** (≤1250); tách `value/show_eq.rs`, `codegen/cau.rs`, `tyck/expr/goi.rs`, `interp/goi_ham.rs`; 0 đổi golden/API/cache; gate `3576/3`; diff 100k seed `20261115` **lech=0** (`JOURNAL/st285-refactor.md`) |
| **ST-286** — R0c `async_net`/`quyen`/`sqlite` | **DONE** | `f193c57` (merge `cfba662`) | `async_net.rs` **606**, `quyen.rs` **1190**, `db/sqlite/mod.rs` **1245** dòng (≤1250); module con `async_net_{conn,nguon,host,h2}.rs`, `quyen_{path,guard,chuan,st174}.rs`, `db/sqlite/{pragma_doc,schema,tach_cau,tests*}.rs`; API công khai không đổi; gate PASS; diff 100k seed `20261122` **lech=0** (`JOURNAL/st286-tach-module.md`) |
| **ST-287** — hook huỷ giữa HTTP request h1/h2 | **DONE — có BLOCKER Windows** | `03ed84c` (merge `f8f1d4b`) | `FrameReactorBackend::cancel_task_handles(frame_id)` + `note_task_handle`/`note_task_h2`; safe point `frame_cancel_one` đóng socket h1 / `RST_STREAM` h2 (session dùng chung không đóng); **Linux** h1 **5/5**, h2 **8/8**, đỏ-trước 5/5 + 7/7, soak 1 giờ fd/RSS/threads phẳng; gate `3581/3`; diff 100k **lech=0**. **Windows: `cancel_http_h1_lap_nhieu_vong_khong_ro` flake ~21 % (3/14 suite; `V:post` sau 10053)** — chờ fix (`JOURNAL/st287-http-cancel.md`, `JOURNAL/st287-soak-1h.md`, `JOURNAL/final-win-v036.md`) |
| **ST-288** — `VS0708` + `pkg gc --root` + `#142` | **DONE** (Gatekeeper độc lập 2026-11-22) | `c513e7a` (merge `545a38d`) | `VS0708` vi/en/ja + `vis explain`/`--list` **54** + `API-CODES.txt` **54**; **6 ca âm** (gồm key rotation) đều nêu `VS0708` (`vs3_registry_sig` 10/10); `pkg update <tên>` **soft pin** — `update_mot_ten_*` 2/2 + unit `soft_pin` ⇒ **#142 đóng**; `pkg gc --root` lặp/lồng dedupe, không tạo `.vis` (`vs3_pkg_gc_multi` 4/4); `pkg` 25/25. **QUEUE còn `REVIEW`** — chờ A0 lật sổ (`JOURNAL/st288-goi.md`) |
| **ST-289** — version `0.3.6` + `#111`/`#151` + R32 | **DONE** (docs phát hành + QA) | `ff55056` (merge `84915aa`); QA `c69127f`/`a997e06`; docs `705afcf` | 8 crate thành viên `version.workspace = true`; `vis 0.3.6`; `#111` khoá bằng `VS0412`; `#151` 24 fixture đổi tên **thuần** (git R100%); Linux final QA ở §2; R32 cập nhật blocker Windows; **tag chưa tạo** |

**Sổ nợ sau Stage 0** (`agent/SO-NO.md` §8): **192 = 93 mở / 74 đóng / 22 chuyển VS3 / 2 không làm /
1 điều hướng** — `#142` (ST-288), `#111` và `#151` (ST-289) đã đóng; ST-287 không thêm dòng nợ mới
(blocker Windows theo dõi ở R32 `blocked[]` + QUEUE, chưa cấp số nợ).

## 4. Còn chờ — trước khi tag `v0.3.6` / mở GĐ 1

- **Blocker bắt buộc**: fix/triage `vs3_http_cancel::cancel_http_h1_lap_nhieu_vong_khong_ro` trên
  Windows (thứ tự safe-point vs nhánh lỗi socket `10053` trong `frame_cancel_one`/`AsyncNet`), rồi
  chạy lại `vs3_http_cancel` **×10** + workspace **1 vòng** trên `psvn`. Đây là **ST-287 follow-up**,
  không sửa code trong báo cáo này.
- Linux final QA **đã xong** (gate 3581/3, release 3586/0/3, 2× soak 20/20, diff 0).
- **ST-288** đã được Gatekeeper duyệt **DONE** nhưng QUEUE tại `head` còn `REVIEW`; A0 cần lật
  `QUEUE.md` khi chốt sổ.
- Nếu A0 chấp nhận **waiver Tier 2**: chỉ tag sau khi (a) ghi known-issue h1 cancel flake trong
  release notes, (b) **không** tuyên bố Windows xanh, (c) giữ Linux final QA 3581/3 làm bằng chứng chính.

## 5. Cổng

- `fast`: **PASS** @ **`4c5dec6`** (fresh-clone) — **3581 passed / 0 failed / 3 skipped** (Linux `ps`).
- `full`: **—** (release workspace 3586/0/3 đã chạy; chưa chạy `vis-dev gate` full).
- `nightly`: **—** (chưa chạy cho Stage 0).
- Windows: workspace lần 2 **3538/0/3** nhưng `vs3_http_cancel` h1 **flake ~21 %**; feature h2
  **BLOCKED-ENV** (Unix-only) ⇒ **chưa đạt cổng tag**.
- Kiểm báo cáo: `test/report/check_report.py --all` **PASS** (32 báo cáo, 322 hàng QUEUE phủ đủ;
  22 mục `blocked[]` gồm ST-287).

## 6. Ghi chú sổ sách

`head` là **baseline sản phẩm được đo** (`a1930cb`; Linux final QA tại `4c5dec6`), không phải commit
chứa báo cáo này. `ST-284`..`ST-287`/`ST-289` ghi `status = DONE` với `queue = READY` vì QUEUE tại
`head` chưa lật trạng thái; `ST-288` ghi `DONE` (Gatekeeper) + `queue = REVIEW`. Hàng QUEUE sẽ được
A0 cập nhật cùng lúc với chốt Stage 0; báo cáo này **không** sửa `QUEUE.md`.
