# R32 — ViScript, vòng 32 (Stage 0 `v0.3.6`, cập nhật 2026-11-22)

> **Trạng thái: CHƯA ĐÓNG R32.** Stage 0 đã có **5/6** lát trên `main` (`ST-284`, `ST-285`,
> `ST-286`, `ST-288`, `ST-289`); **chỉ còn `ST-287` (hook huỷ HTTP) WIP — chưa merge**.
> Báo cáo ghi bằng chứng đã có tại `eaf22a2`; số nghiệm thu cuối vòng (full gate, nightly,
> fresh-clone) **chưa chạy** và sẽ được chốt khi Stage 0 đủ 6/6.
>
> Windows: đã verify Stage 0 tại **`0d057e5`** — workspace **3533/0/3**, `vis 0.3.6` (xem §2);
> `eaf22a2` chỉ thêm ST-285 (R0b) + báo cáo, **chưa verify lại Windows**.
>
> Baseline `head`: **`eaf22a2`** (merge ST-289: Windows verify + ST-285).
> Tag gần nhất **`v0.3.5`** @ `03bb7b9` giữ nguyên; **chưa tag `v0.3.6`**.
> Không push/merge/tag từ W4; không GitHub Actions.

## 1. Phạm vi

Vòng 32 mở chương trình dài hạn hậu `v0.3.5` theo `VD-272`..`VD-279`; Giai đoạn 0 (`v0.3.6`) gồm
sáu lát: R0a/R0b/R0c refactor 0 đổi hành vi (`ST-284`..`ST-286`), hook huỷ HTTP (`ST-287`), khép G3
gói (`ST-288`: `VS0708`, `pkg gc --root`, `#142`) và đồng bộ sự thật (`ST-289`: version một nguồn,
`#111`, `#151`, docs, R32). Báo cáo **chỉ** chốt phần đã có bằng chứng trên `main`; không dùng số
xanh của 5 lát để tuyên bố Stage 0 hoàn tất.

## 2. Baseline được đo

- `head` = `eaf22a2` (tổ tiên HEAD; QUEUE `ST-284..ST-309` phủ đủ 322 hàng).
- Linux (`ps`): `./vis-dev gate --fast` **PASS — 3576 passed / 0 failed / 3 skipped** (fmt, clippy,
  R1..R6, self-test, doc-examples 24 khối: 19 xanh + 5 ngoại lệ, L0 nextest).
- Differential 100k: seed `20261115` (`ST-284`, lặp lại `ST-285`) **lech=0** (158,4 s / 200,5 s,
  93 770 ca chạy); seed `20261122` (`ST-286`) **lech=0** (172,8 s, 93 770 ca chạy) —
  `JOURNAL/st284-diff-100k.log`, `JOURNAL/st285-diff-100k.log`, `JOURNAL/st286-diff-100k.log`.
- Windows (Tier 2, **chưa tuyên bố hỗ trợ**): verify Stage 0 @ **`0d057e5`** — `psvn` cây sạch,
  `cargo test --workspace --no-fail-fast` **354 block, 3533 passed / 0 failed / 3 ignored** (534 s);
  `vis 0.3.6`; P2 14/14, P1 8/8, P3 10/10 debug + release; h2 19, `pkg` 25, sig 10, default 22,
  tls 22, net 9; host 54/54; targets 18/18 + 17/1; release 11 211 776 B
  (`JOURNAL/stage0-win-0d057e5.md`).
- macOS: không có máy Darwin thật — **không tuyên bố**; wasm32: chỉ `cargo check`.

## 3. Bằng chứng Stage 0 (5/6)

| Task | Trạng thái | Commit | Bằng chứng |
|---|---|---|---|
| **ST-284** — R0a `vm.rs`/`frame.rs` | **DONE** | `83a9a10` (merge `e363a15`) | `crates/vis-vm/src/vm.rs` **1240** dòng, `crates/vis-interp/src/frame.rs` **1042** dòng (ngưỡng ≤1250); tách `vm/host.rs` (205) + `frame/worker.rs` (464) giữ nguyên API; 0 đổi ABI/cache/opcode; gate `3571/3`; diff 100k seed `20261115` **lech=0** (`JOURNAL/st284-refactor.md`) |
| **ST-285** — R0b `value`/`codegen`/`tyck`/`interp/goi` | **DONE** | `ab5d292` (merge `246b6c5`) | `value.rs` **1186**, `codegen.rs` **1045**, `tyck/expr.rs` **1064**, `interp/goi.rs` **1155** (≤1250); tách `value/show_eq.rs`, `codegen/cau.rs`, `tyck/expr/goi.rs`, `interp/goi_ham.rs`; 0 đổi golden/API/cache; gate `3576/3`; diff 100k seed `20261115` **lech=0** (`JOURNAL/st285-refactor.md`) |
| **ST-286** — R0c `async_net`/`quyen`/`sqlite` | **DONE** | `f193c57` (merge `cfba662`) | `async_net.rs` **606**, `quyen.rs` **1190**, `db/sqlite/mod.rs` **1245** dòng (≤1250); module con `async_net_{conn,nguon,host,h2}.rs`, `quyen_{path,guard,chuan,st174}.rs`, `db/sqlite/{pragma_doc,schema,tach_cau,tests*}.rs`; API công khai không đổi; gate PASS; diff 100k seed `20261122` **lech=0** (`JOURNAL/st286-tach-module.md`) |
| **ST-288** — `VS0708` + `pkg gc --root` + `#142` | **REVIEW** (chờ Gatekeeper) | `c513e7a` (merge `545a38d`) | `VS0708` vi/en/ja (54 mã, 8 mã module) cho chữ ký/khoá registry; `pkg update <tên>` dùng **soft pin** thay ghim cứng ⇒ **#142 đóng**; `gc --root` lặp/lồng dedupe, không tạo `.vis`; `pkg` 25/25, `vs3_registry_sig` 10/10, `vs3_pkg_gc_multi` 4/4 (`JOURNAL/st288-goi.md`) |
| **ST-289** — version `0.3.6` + `#111`/`#151` + R32 | **DONE** | `ff55056` (merge `84915aa`) | 8 crate thành viên `version.workspace = true`; `vis 0.3.6`; `#111` khoá bằng `VS0412` (`cap_khong_lo.rs` 1b); `#151` 24 fixture đổi tên **thuần** (git R100%); docs/SO-NO/HOST chỉ sửa phần đã kiểm; khung R32 |

**Sổ nợ sau Stage 0** (`agent/SO-NO.md` §8): **192 = 93 mở / 74 đóng / 22 chuyển VS3 / 2 không làm /
1 điều hướng** — `#142` (ST-288), `#111` và `#151` (ST-289) đã đóng; không thêm dòng nợ mới.

## 4. Còn chờ — điều kiện đóng R32

- **ST-287** (hook huỷ giữa HTTP request h1/h2) — **WIP, chưa merge**; cần `vs3_http_cancel.rs`
  đỏ-trước, không rò fd/không treo, parity interp↔VM trên `main`.
- **ST-288** cần Gatekeeper chuyển `REVIEW → DONE`.
- Khi đủ 6/6: chốt R32 đầy đủ — full gate + nightly, fresh-clone Linux, verify `psvn` tại đúng
  commit chốt, rà `SO-NO.md` kèm commit hash, cập nhật `coverage`/`sim`/`perf`, rồi mới mở
  Giai đoạn 1 (`ST-290..`).

## 5. Cổng

- `fast`: **PASS** @ `eaf22a2` — **3576 passed / 0 failed / 3 skipped** (Linux `ps`, rustc 1.100.0).
- Windows: **PASS** @ `0d057e5` — workspace **3533/0/3**, version **0.3.6** (debug + release),
  async-h2 vẫn `EXPECTED_RED` (Unix-only).
- `full`: **—** (chưa chạy cho Stage 0).
- `nightly`: **—** (chưa chạy cho Stage 0).
- Kiểm báo cáo: `test/report/check_report.py --all` **PASS** (32 báo cáo, 322 hàng QUEUE phủ đủ;
  21 hàng `BLOCKED/REVIEW` có câu hỏi).

## 6. Ghi chú sổ sách

`head` là **baseline sản phẩm được đo** (`eaf22a2`), không phải commit chứa báo cáo này.
`ST-284/ST-285/ST-286/ST-289` ghi `status = DONE` với `queue = READY` vì QUEUE tại `head` chưa lật
trạng thái; `ST-288` giữ `REVIEW` đúng QUEUE; `ST-287` giữ `READY` (WIP). Hàng QUEUE sẽ được A0 cập nhật cùng lúc với chốt R32; báo
cáo này **không** sửa `QUEUE.md`. Các hàng `ST-212/ST-241/ST-243/ST-248` còn `READY` trong QUEUE là
**hàng lịch sử**, không thuộc Stage 0. Không dùng số xanh của 4 lát để tuyên bố Stage 0 hoàn tất hay
mở Giai đoạn 1.
