# R32 — ViScript, vòng 32 (Stage 0 `v0.3.6`, cập nhật 2026-11-22)

> **Trạng thái: CHƯA ĐÓNG R32.** Stage 0 mới có **4/6** lát trên `main` (`ST-284`, `ST-286`,
> `ST-288`, `ST-289`); **`ST-285` (R0b) và `ST-287` (hook huỷ HTTP) còn WIP — chưa merge**.
> Báo cáo ghi bằng chứng đã có tại `84915aa`; số nghiệm thu cuối vòng (full gate, nightly,
> fresh-clone, `psvn`) **chưa chạy** và sẽ được chốt khi Stage 0 đủ 6/6.
>
> Baseline `head`: **`84915aa`** (merge Stage 0: `0.3.6` + `#111`/`#151` + khung R32).
> Tag gần nhất **`v0.3.5`** @ `03bb7b9` giữ nguyên; **chưa tag `v0.3.6`**.
> Không push/merge/tag từ W4; không GitHub Actions.

## 1. Phạm vi

Vòng 32 mở chương trình dài hạn hậu `v0.3.5` theo `VD-272`..`VD-279`; Giai đoạn 0 (`v0.3.6`) gồm
sáu lát: R0a/R0b/R0c refactor 0 đổi hành vi (`ST-284`..`ST-286`), hook huỷ HTTP (`ST-287`), khép G3
gói (`ST-288`: `VS0708`, `pkg gc --root`, `#142`) và đồng bộ sự thật (`ST-289`: version một nguồn,
`#111`, `#151`, docs, R32). Báo cáo **chỉ** chốt phần đã có bằng chứng trên `main`; không dùng số
xanh của 4 lát để tuyên bố Stage 0 hoàn tất.

## 2. Baseline được đo

- `head` = `84915aa` (tổ tiên HEAD; QUEUE `ST-284..ST-309` phủ đủ 322 hàng).
- Linux (`ps`): `./vis-dev gate --fast` **PASS — 3576 passed / 0 failed / 3 skipped** (fmt, clippy,
  R1..R6, self-test, doc-examples 24 khối: 19 xanh + 5 ngoại lệ, L0 nextest; log phiên
  `/tmp/st289-stage0-gate.log`).
- Differential 100k: seed `20261115` (`ST-284`) **lech=0** (158,4 s, 93 770 ca chạy);
  seed `20261122` (`ST-286`) **lech=0** (172,8 s, 93 770 ca chạy) —
  `JOURNAL/st284-diff-100k.log`, `JOURNAL/st286-diff-100k.log`.
- Windows: bằng chứng hiện hành vẫn là tag `v0.3.5` @ `03bb7b9` — workspace **3528/0/3**
  (debug + release), P2 10/10, P3 10/10; **Tier 2**, chưa tuyên bố hỗ trợ. Stage 0 **chưa** verify
  lại `psvn`.
- macOS: không có máy Darwin thật — **không tuyên bố**; wasm32: chỉ `cargo check`.

## 3. Bằng chứng Stage 0 (4/6)

| Task | Trạng thái | Commit | Bằng chứng |
|---|---|---|---|
| **ST-284** — R0a `vm.rs`/`frame.rs` | **DONE** | `83a9a10` (merge `e363a15`) | `crates/vis-vm/src/vm.rs` **1240** dòng, `crates/vis-interp/src/frame.rs` **1042** dòng (ngưỡng ≤1250); tách `vm/host.rs` (205) + `frame/worker.rs` (464) giữ nguyên API; 0 đổi ABI/cache/opcode; gate `3571/3`; diff 100k seed `20261115` **lech=0** (`JOURNAL/st284-refactor.md`) |
| **ST-286** — R0c `async_net`/`quyen`/`sqlite` | **DONE** | `f193c57` (merge `cfba662`) | `async_net.rs` **606**, `quyen.rs` **1190**, `db/sqlite/mod.rs` **1245** dòng (≤1250); module con `async_net_{conn,nguon,host,h2}.rs`, `quyen_{path,guard,chuan,st174}.rs`, `db/sqlite/{pragma_doc,schema,tach_cau,tests*}.rs`; API công khai không đổi; gate PASS; diff 100k seed `20261122` **lech=0** (`JOURNAL/st286-tach-module.md`) |
| **ST-288** — `VS0708` + `pkg gc --root` + `#142` | **REVIEW** (chờ Gatekeeper) | `c513e7a` (merge `545a38d`) | `VS0708` vi/en/ja (54 mã, 8 mã module) cho chữ ký/khoá registry; `pkg update <tên>` dùng **soft pin** thay ghim cứng ⇒ **#142 đóng**; `gc --root` lặp/lồng dedupe, không tạo `.vis`; `pkg` 25/25, `vs3_registry_sig` 10/10, `vs3_pkg_gc_multi` 4/4 (`JOURNAL/st288-goi.md`) |
| **ST-289** — version `0.3.6` + `#111`/`#151` + R32 | **DONE** | `ff55056` (merge `84915aa`) | 8 crate thành viên `version.workspace = true`; `vis 0.3.6`; `#111` khoá bằng `VS0412` (`cap_khong_lo.rs` 1b); `#151` 24 fixture đổi tên **thuần** (git R100%); docs/SO-NO/HOST chỉ sửa phần đã kiểm; khung R32 |

**Sổ nợ sau Stage 0** (`agent/SO-NO.md` §8): **192 = 93 mở / 74 đóng / 22 chuyển VS3 / 2 không làm /
1 điều hướng** — `#142` (ST-288), `#111` và `#151` (ST-289) đã đóng; không thêm dòng nợ mới.

## 4. Còn chờ — điều kiện đóng R32

- **ST-285** (R0b: `value.rs`, `codegen.rs`, `tyck/expr.rs`, `interp/goi.rs`) — **WIP, chưa merge**;
  cần xác nhận mọi tệp lõi ≤1.250 dòng, 0 đổi golden/API/cache, `gate --fast` PASS.
- **ST-287** (hook huỷ giữa HTTP request h1/h2) — **WIP, chưa merge**; cần `vs3_http_cancel.rs`
  đỏ-trước, không rò fd/không treo, parity interp↔VM trên `main`.
- **ST-288** cần Gatekeeper chuyển `REVIEW → DONE`.
- Khi đủ 6/6: chốt R32 đầy đủ — full gate + nightly, fresh-clone Linux, verify `psvn`, rà `SO-NO.md`
  kèm commit hash, cập nhật `coverage`/`sim`/`perf`, rồi mới mở Giai đoạn 1 (`ST-290..`).

## 5. Cổng

- `fast`: **PASS** @ `84915aa` — **3576 passed / 0 failed / 3 skipped** (Linux `ps`, rustc 1.100.0,
  `CARGO_INCREMENTAL=0`).
- `full`: **—** (chưa chạy cho Stage 0).
- `nightly`: **—** (chưa chạy cho Stage 0).
- Kiểm báo cáo: `test/report/check_report.py --all` **PASS** (32 báo cáo, 322 hàng QUEUE phủ đủ;
  21 hàng `BLOCKED/REVIEW` có câu hỏi).

## 6. Ghi chú sổ sách

`head` là **baseline sản phẩm được đo** (`84915aa`), không phải commit chứa báo cáo này.
`ST-284/ST-286/ST-289` ghi `status = DONE` với `queue = READY` vì QUEUE tại `head` chưa lật trạng
thái; `ST-288` giữ `REVIEW` đúng QUEUE. Hàng QUEUE sẽ được A0 cập nhật cùng lúc với chốt R32; báo
cáo này **không** sửa `QUEUE.md`. Các hàng `ST-212/ST-241/ST-243/ST-248` còn `READY` trong QUEUE là
**hàng lịch sử**, không thuộc Stage 0. Không dùng số xanh của 4 lát để tuyên bố Stage 0 hoàn tất hay
mở Giai đoạn 1.
