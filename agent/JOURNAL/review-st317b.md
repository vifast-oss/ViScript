<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# REVIEW ST-317b — kiểm toán độc lập tách 3 tệp sản phẩm > 1.100 LOC

> **Người kiểm**: Gatekeeper (Agent 1 độc lập), 2026-11-27 (giờ thấp điểm).
> **Đối tượng**: nhánh `plan/st317b`, HEAD **`31a2c11ba0d174655387270de22e5d8a31ac1650`**,
> nền `main` **`3e828393`**; 4 commit `f247567c` / `987c5585` / `7ae2f96e` / `31a2c11b`.
> **Worktree**: `/home/ps/viswt/st317b` — **không sửa mã sản phẩm**, không push, không fuzz/soak.
> **Cấu hình đo**: `CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0`; đĩa `/home/ps` **14 GB trống**
> (> 6 GB) trước và sau. Log ngoài repo tại `/home/ps/vislogs/review-st317b-*.log` (R7).
>
> **Kết luận: PASS** cho **lát đã giao** (3 tệp tách + thiết kế worker). **0 P1, 0 P2 lỗi mã.**
> **1 P2 phạm vi**: ST-317b theo `QUEUE.md:2237` yêu cầu **6 tệp + khử trùng lặp worker/sync↔frame**;
> lát này đạt **3/6 tệp + thiết kế**, nên **chưa đủ điều kiện đóng `DONE`** — xem §6/F3.

## 0. TL;DR

| # | Hạng mục | Kết quả | Bằng chứng chính |
|---|---|---|---|
| 1 | Phạm vi git | **PASS** | `git status --short` rỗng; 27 tệp = 22 tệp sản phẩm mới + 3 tệp gốc xoá + journal + `STATE.md`; **0** tệp ngoài phạm vi, **0** đường cấm bị chạm |
| 2 | Move nguyên văn | **PASS** | Băm từng item (fn/struct/enum/const/static/type): **43/43**, **37/37**, **28/28** khớp; chỉ 4 khác biệt cơ học (dấu phẩy cuối + rút gọn đường dẫn); `mod.rs` **0** `fn` |
| 3 | API công khai | **PASS** | `vis_pkg::pkg::run`, `pub mod resolver`, `pub use semver`; `rt::Scheduler` + **22/22** tên `pub fn` y hệt; `pat::{than_uses, arm_uses}`; không mất/đổi chữ ký |
| 4 | Test | **PASS** | `cargo test -p vis-pkg -p vis-types -p vis-runtime` **exit 0** (27 + 288 + 2 + 221 + 2 doc); `sig_it` **16/16**; số `#[test]` trong src nền = HEAD (29/288/2) |
| 5 | `gate --fast` | **PASS** | **exit 0** — `3880/3880 passed, 4 skipped` (106,7 s); fmt/clippy/R1..R9/self-test 18/DAG/doc-examples đều OK; sha256 `e9525357…6a8804` |
| 6 | `vis-diff 100k` | **PASS** | **`lech=0`** — 100.000 chương trình (93.768 chạy + 6.232 lỗi tĩnh), 101,1 s, 989/s; 7 test xanh; exit 0; sha256 `7cc69ebd…a17d3a` |
| 7 | R8 / R1 | **PASS** | R8 = **39/40** (`vis-cli/tests` **8/15**), **0 tệp test mới**; **0** tệp sản phẩm > 1.500 dòng; tệp con mới max **377 / 321 / 279** |
| 8 | DAG 5 tầng | **PASS** | `vis-dag-check`: **23 crate / 5 tầng / 0 cạnh cấm**; log sha256 `5ee0d7f4…c4af4d` |
| 9 | Rủi ro thật | **PASS** | Không mất re-export, không `pub` rò ra ngoài crate, không `#[path]` gãy, không test bỏ sót; 3 finding P3 + 1 ghi chú phạm vi P2 (§6) |

## 1. Phạm vi git

```
git log --oneline 3e828393..HEAD
31a2c11b [ST-317b] journal tách tệp + thiết kế worker (chờ 317a); STATE cập nhật
7ae2f96e [ST-317b] tách vis-types/src/tyck/pat.rs 1140 → mod 21 + 6 tệp (<322 dòng), API than_uses/arm_uses giữ nguyên
987c5585 [ST-317b] tách vis-runtime/src/rt/sched.rs 1189 → mod 22 + 8 tệp (<280 dòng), API Scheduler/trigger giữ nguyên
f247567c [ST-317b] tách vis-pkg/src/pkg.rs 1151 → mod 54 + 5 tệp (goi/khoa/lenh/manifest/nguon), API + test giữ nguyên
```

`git diff --stat -M 3e828393..HEAD`: **27 tệp, +3.911/−3.480**. Từng commit đúng nhóm:

| Commit | Tệp | Ghi chú |
|---|---|---|
| `f247567c` | 7 tệp `vis-pkg/src/pkg{,.rs}` | xoá `pkg.rs` 1.151, thêm `pkg/{mod,goi,khoa,lenh,manifest,nguon}.rs` |
| `987c5585` | 10 tệp `vis-runtime/src/rt/sched{,.rs}` | xoá `sched.rs` 1.189, thêm 9 tệp |
| `7ae2f96e` | 8 tệp `vis-types/src/tyck/pat{,.rs}` | xoá `pat.rs` 1.140, thêm 7 tệp |
| `31a2c11b` | `agent/JOURNAL/st317b-tach-tep.md` + `agent/STATE.md` | chỉ tài liệu |

**Đường cấm — 0 chạm** (kiểm bằng `git diff --name-only | grep -E`): `vis-core/src/value/**`,
`vis-lsp/**`, `vis-hir/src/stdlib/**`, `vis-diag/**`,
`vis-host-native/src/db/sqlite/mod.rs`, `vis-runtime/src/worker/proto.rs`.
4 tệp anh em `pkg/{gc,registry_http,resolver,tar_b64}.rs` **không sửa một dòng** (không có trong diff).

## 2. Move nguyên văn — kiểm bằng băm item, không chỉ `git diff -M`

`git diff -M --summary` báo 22 `create` + 3 `delete` (không phải rename) vì nội dung bị chia lại —
đúng bản chất tách tệp. Để chứng minh **không sửa logic**, tôi trích **mọi item cấp cao**
(`fn`, `struct`, `enum`, `union`, `trait`, `type`, `const`, `static`) từ tệp gốc và từ toàn bộ tệp
mới, chuẩn hoá (bỏ khoảng trắng/comment/visibility/tiền tố `super::|crate::|self::`) rồi so **multiset
băm SHA-256** của thân item:

| Tệp gốc | Item cũ | Item mới | Khớp | Lệch |
|---|---|---|---|---|
| `vis-pkg/src/pkg.rs` (1.151) | 43 | 43 | **41** | 2 (chỉ thêm `,` cuối tham số) |
| `vis-runtime/src/rt/sched.rs` (1.189) | 37 | 37 | **36** | 1 (`super::exec_state::ExecState` → `ExecState`) |
| `vis-types/src/tyck/pat.rs` (1.140) | 28 | 28 | **27** | 1 (`std::collections::BTreeMap` → `BTreeMap`) |

4 lệch đều **cơ học, 0 đổi ngữ nghĩa**:
- `lenh_add`/`lenh_update`: rustfmt xuống dòng tham số nên thêm dấu phẩy cuối (`net: &mut dyn NetHost,`).
- `Scheduler.exec`: kiểu vẫn là `ExecState` (đã `use super::exec_state::ExecState`).
- `BindTypes`: vẫn là `BTreeMap<u32, Type>` (đã import `BTreeMap`).

Kiểm bổ sung: **thuộc tính/derive khớp 100%** — pkg: `1 #[cfg(test)]` + `2 #[test]`;
sched: `1 #[derive(Debug, Default)]` + `20 #[doc(hidden)]` **đúng từng item** (so cặp attr→item);
pat: 0 attr. Không `impl` nào bị đổi đích (pkg gốc 0 `impl`; pat 2 `impl Checker` → 4 khối cùng đích
`impl<'a> Checker<'a>`; sched 2 → 8 khối cùng đích `impl Scheduler`). Không thêm `unsafe`, SPDX đủ
22/22 tệp, toàn bộ LF.

`mod.rs` chỉ khai báo + re-export: **0 định nghĩa `fn`** ở cả 3 tệp (54/22/21 dòng).

## 3. API công khai trước/sau

**`vis-pkg`** — công khai ngoài crate **không đổi**:

| Trước | Sau |
|---|---|
| `pub fn run(…) -> ExitCode` | `pub fn run` (trong `lenh.rs`, `pub use lenh::run` ở `mod.rs`) |
| `pub mod resolver`, `pub use vis_loader::semver` | y hệt (`mod.rs`) |
| mọi helper `fn`/`struct` **private** | `pub(crate)` (nội bộ crate) |

Re-export `pub(crate)` đều **có người dùng thật** (không chết): `dung` ← `gc.rs`; `cwd` ← `gc.rs`;
`doc_tat_ca`/`giai_roots`/`GoiCai` ← `registry_http.rs`; `Manifest`/`manifest_tu_gia_tri`/
`doc_manifest` ← `registry_http.rs`/`resolver.rs`; `lock_meta::*` ← `gc/registry_http/resolver`.

**`vis-runtime`** — `pub struct Scheduler` + **22/22** `pub fn` **trùng tên tuyệt đối** (diff danh sách
= rỗng): `tao spawn new diag_snapshot frame_enable_for_test frame_rearm_for_test frame_await
frame_register frame_next frame_stuck_trap frame_task_done frame_drain_events frame_final_join
frame_take_final_join_error_for_test frame_reclaim frame_cancel_request_for_test
frame_cancel_process_for_test frame_reactor_load_for_test frame_reactor_poll frame_reactor_fire
frame_idle frame_reactor_shutdown`. Chữ ký khớp từng byte sau chuẩn hoá (băm item).
`FRAME_CANCEL_{TEST,TURNS,BYTES}` giữ `pub(crate)`, re-export qua `rt.rs` — `frame_test.rs` không sửa.

**`vis-types`** — `pub(crate) pat::{than_uses, arm_uses}` re-export nguyên; 6 method `pub(crate)`
(`bind_pat`, `xet_het`, `kiem_or_bind`, `kiem_let_phu_het`, `kiem_for_phu_het`) khớp chữ ký.
`expr.rs`, `tyck/mod.rs`, `phuong_thuc.rs` không sửa và vẫn gọi `pat::…` như cũ.

Nới visibility **nội bộ** (không ra ngoài crate, clippy sạch): pkg `fn→pub(crate)`; sched
`frame_after_ready_budget|frame_fire|frame_wake_one` private→`pub(super)`,
`frame_dem_byte_va_yeu_cau_huy|wake_reactor` `pub(super)`→`pub(crate)`; pat helper→`pub(super)`.
Không có `pub fn` nào lọt trong module helper private (đã grep: 0).

## 4. Test

| Lệnh | Kết quả | Log |
|---|---|---|
| `cargo test -p vis-pkg -p vis-types -p vis-runtime` | **exit 0** — pkg lib **27**, runtime lib **288**, types lib **2** + `types_it` **221** + doc **2** | `/home/ps/vislogs/review-st317b-cargo-test.log` · `de73f0b5…af0dde` |
| `cargo test -p vis-pkg --features pkg-sig --test sig_it` | **exit 0 — 16/16** | `/home/ps/vislogs/review-st317b-sig-it.log` · `86456a20…5842e` |
| `./vis-dev gate --fast` | **exit 0 — `3880/3880 passed, 4 skipped`** (106,711 s) | `/home/ps/vislogs/review-st317b-gate.log` · `e9525357…6a8804` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **exit 0 — `lech=0`**; `tong=100000 … chay=93768 … giay=101.1 toc_do=989/s`; 7 test xanh | `/home/ps/vislogs/review-st317b-diff.log` · `7cc69ebd…a17d3a` |
| `./bin/vis-dag-check` | **exit 0 — 23 crate / 5 tầng / 0 cạnh cấm** | `/home/ps/vislogs/review-st317b-dag.log` · `5ee0d7f4…c4af4d` |
| `./vis-dev rules` (trong gate) | **OK R1..R9**, self-test **18/18** case âm, doc-examples **31 khối** OK | cùng log gate |

Gate chi tiết: `--- fmt: OK` · `--- clippy: OK` · `--- luật R1..R9: OK` · `--- DAG 5 tầng: OK` ·
`--- DAG self-test: OK` · `--- self-test case âm: OK` · `--- doc-examples: OK` · `--- L0 test: OK`.
Log DAG của tôi **trùng sha256** với log Builder (`5ee0d7f4…c4af4d`) vì nội dung tất định.

**Không mất test**: đếm `#[test]` trong `src/` ở nền vs HEAD — `vis-pkg` **29 = 29**,
`vis-runtime` **288 = 288**, `vis-types` **2 = 2**; riêng `pkg.rs` 2 test → `manifest.rs` 2 test
(nội dung băm y hệt). `vis-runtime`/`vis-types` không có unit test trong 2 tệp bị tách.

## 5. R8 / R1 / kích thước tệp

- **R8 = 39/40**: `crates/*/tests/*.rs` **38** + `vis-host-linux/tests/*.rs` **1**; `vis-cli/tests` **8/15**.
  **0 tệp test cấp 1 mới** — `diff` danh sách tệp test nền↔HEAD rỗng; không `#[path]` nào bị đụng
  (không tệp test nào di chuyển).
- **R1**: **0** tệp sản phẩm > 1.500 dòng ở HEAD. Tệp gốc → max tệp con:

| Tệp gốc | Trước | `mod.rs` | Max tệp con | Tệp con |
|---|---|---|---|---|
| `vis-pkg/src/pkg.rs` | 1.151 | 54 | **377** (`lenh`) | 170, 178, 213, 228 |
| `vis-runtime/src/rt/sched.rs` | 1.189 | 22 | **279** (`frame_reactor`) | 16, 109, 113, 120, 195, 215, 216 |
| `vis-types/src/tyck/pat.rs` | 1.140 | 21 | **321** (`gan`) | 98, 169, 177, 204, 210 |

Mọi tệp con **< 400** dòng. 4 tệp anh em `pkg/` giữ nguyên kích thước (266/431/777/446).

## 6. Rủi ro & finding

**Không tìm thấy lỗi mã P1/P2.** Các kiểm nhắm đúng rủi ro của tách tệp:
- *Mất re-export*: không — mọi `super::…` của `gc/registry_http/resolver` và `rt/frame_test.rs`
  resolve (build + toàn bộ test xanh); danh sách re-export đều có caller thật.
- *Visibility sai*: không — helper chéo tệp dùng `pub(super)`, chỉ 2 hàm cần từ `rt.rs`/`frame_test.rs`
  mới `pub(crate)`; không `pub` rò ra ngoài crate; clippy `-D warnings` OK.
- *Test cũ bị bỏ sót*: không — số `#[test]` nền = HEAD; 2 test pkg băm y hệt.
- *`#[path]` gãy*: không — 0 tệp test thay đổi.

| # | Mức | Nội dung | Tái lập |
|---|---|---|---|
| F1 | P3 | Unit test pkg đổi module path `pkg::tests` → `pkg::manifest::tests` (nội dung 2/2 y hệt). Không filter CI nào trong repo dùng path cũ (chỉ tài liệu lưu trữ nhắc). | `grep -rn 'pkg::tests' --include='*.sh' --include='*.yml' .` |
| F2 | P3 | Nới visibility **nội bộ** như §3 (private → `pub(super)`/`pub(crate)`). Không đổi API ngoài crate; cần cho gọi chéo tệp. | `git diff 3e828393..HEAD -- '*sched*' '*pkg*' '*pat*' \| grep '^[-+]pub'` |
| F3 | **P2 (phạm vi)** | **ST-317b chưa trọn**: `QUEUE.md:2237` yêu cầu 6 tệp + khử trùng lặp `vis-interp` sync↔frame + gom worker orchestration. Lát này đạt **3/6 tệp**; còn `sqlite/mod.rs` 1.130 (chờ 317a), `value/mod.rs` 1.138 + `lsp/lib.rs` 1.144 (bị khoá bởi W1/W3), worker/sync↔frame **mới là thiết kế**. Ở HEAD vẫn còn **15 tệp sản phẩm > 1.000 dòng** (max `trait_impl.rs` 1.152, `lsp/lib.rs` 1.144, `value/mod.rs` 1.138, `sqlite/mod.rs` 1.130). **Không đóng `DONE`**; giữ `READY`/một phần, mở lại sau 317a/317c. | `grep -n 'ST-317b' agent/QUEUE.md`; `git ls-files '*.rs' \| grep -v /tests/ \| xargs wc -l \| awk '$1>1000'` |
| F4 | P3 | Khác biệt cơ học: dấu phẩy cuối `lenh_add`/`lenh_update`; rút gọn đường dẫn `ExecState`, `BTreeMap`. 0 đổi hành vi (băm thân hàm sau chuẩn hoá khớp). | `python3` so băm item; xem §2 |

**Đánh giá phạm vi**: journal Builder khai đúng và minh bạch phần hoãn (§0, §5), các số liệu
`3880/3880`, `lech=0`, R8 39/40, DAG 23/5/0, kích thước tệp **đều tái lập được**. Vì vậy **PASS cho
lát đã giao**; F3 chỉ là điều kiện đóng task, không phải lỗi của mã trong lát.

## 7. Phán quyết

**PASS** — 3 tệp tách đúng là move nguyên văn (không đổi logic/hành vi/API công khai), test + gate +
diff 100k + DAG + R8/R1 đều xanh, không có thay đổi ngoài phạm vi. Điều kiện kèm theo: **không đánh
dấu `ST-317b` là `DONE`** cho tới khi hoàn tất 3 tệp còn lại + worker orchestration (F3).
