<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# REVIEW ST-318a — kiểm toán độc lập đóng `#203` + `#204` (frame W=1 & non-frame final join)

> **Người kiểm**: Gatekeeper (Agent 1 độc lập), 2026-11-27 (giờ thấp điểm).
> **Đối tượng**: nhánh `plan/st318a`, HEAD **`0482e5ce391a5adb6b5b85b3a818645a6a8db056`**,
> nền **`2b5ccb512522c80ac0170a92e68168820dc24729`**; 3 commit `4bd432ec` / `4826b68b` / `0482e5ce`.
> **`main` đối chiếu**: lúc giao `2c7dbef6`; trong phiên `main` đã tiến tới **`baa2dbd3`**
> (`[ST-00] VD-297`), `2c7dbef6` vẫn là tổ tiên. Kiểm merge-tree với **cả hai**.
> **Worktree**: `/home/ps/viswt/st318a` — **không sửa mã**, không push, không `cargo clean`.
> **Cấu hình đo**: cargo **1.100.0** (`$HOME/.cargo/bin/cargo`; `vis-dev` tự chọn), `CARGO_BUILD_JOBS=2`;
> đĩa `/home` **9,2 GB → 15 GB** trống (luôn > 6 GB). Log ngoài repo tại `/home/ps/vislogs/` (R7).
>
> **Kết luận: PASS** cho đúng phạm vi `#203`/`#204`: **0 P1**, **2 P2 mới** (perf `nap_bien_cap_tep`
> và mất stdout sau trap đầu ở `final_join`), **2 P3**. Hai P2 **không** làm sai tiêu chí đóng nợ
> nhưng là regression so với `main`; A0 cần hoặc cho vá trong cùng lane, hoặc đăng ký nợ mới trước
> khi hợp nhất. Chi tiết + repro ở §6.

## 0. TL;DR

| # | Hạng mục | Kết quả | Bằng chứng chính |
|---|---|---|---|
| 1 | Phạm vi git | **PASS** | `git status` rỗng; 3 commit; **5 tệp** = 2 mã + 1 test + journal + `STATE.md`; 0 chạm `vis-types`/`vis-hir`/`vis-diag`/`vis-core`/`host_parity` |
| 2 | `#203` frame W=1 | **PASS** | `vis run` / `--vm` / `--workers 1` đều **`1\n`**, exit **0**; `main` cũ interp **`0`**; `--workers 2` **`VS0509`**, exit **101** |
| 3 | `#204` non-frame W=2/4 | **PASS** | `w12_*` thật: exit **1**, `VS0506` chứa `hong`, message **byte-identical** frame W=1, stdout `sau\n`; cả interp + VM |
| 4 | Test/khoá | **PASS** | `st317b_worker_orchestrate` **13/13**; 6 module worker liên quan **60/60**; 0 `#[ignore]` |
| 5 | Cổng độc lập | **PASS** | gate **3904/3904** (4 skip, 109,2 s); diff 100k **`lech=0`** (70,0 s); clippy exit 0 |
| 6 | Hợp nhất | **PASS (dự kiến)** | merge-tree chỉ xung đột `agent/STATE.md`, 0 tệp khác (cả `2c7dbef6` lẫn `baa2dbd3`) |
| 7 | Rủi ro/lỗi thật | **PASS có finding** | 2 P2 mới (§6.1 perf 10–14×; §6.2 mất output task sau trap đầu) + 2 P3 |

## 1. Phạm vi git

```
0482e5ce [ST-318a] journal: lane dinh chinh VD-296 + ghi chu copy loop/thread test
4826b68b [ST-318a] journal + STATE: so lieu cong (#203/#204), gate 3904/3904, diff 100k lech=0
4bd432ec [ST-318a] dong #203 (frame W=1 bien cap tep) + #204 (non-frame final_join surface VS0506)
```

`git diff --stat 2b5ccb51..HEAD` — **5 tệp, +341/−4**:

| Tệp | Thay đổi | Loại |
|---|---|---|
| `crates/vis-interp/src/frame/worker.rs` | +155/−1 | mã |
| `crates/vis-runtime/src/worker/orchestrate.rs` | +18/−1 | mã |
| `crates/vis-conformance/tests/async_api_it/st317b_worker_orchestrate.rs` | +82 | test |
| `agent/JOURNAL/st318a-dong-no.md` | +89 | journal Builder |
| `agent/STATE.md` | +1 | trạng thái |

`git status --short` **rỗng** trước và sau phiên; `git diff --check` sạch. Không tệp `.rs` cấp 1
mới, 0 `unsafe` mới, 0 `#[ignore]`. Diff chỉ **thêm** test, không sửa/xoá assert cũ.

## 2. `#203` — frame W=1, biến cấp tệp là ô chung

### 2.1 Hướng sửa khớp SPEC §5.4

`docs/SPEC.md:638-641`: `N = 1` "giữ đường tuần tự cũ"; snapshot + `VS0509` chỉ cho `N > 1`.
`main` cũ: interp frame `vis run w2.vis` in **`0`**, VM in **`1`** — lệch parity. Sửa **interp** để
W=1 dùng chung ô top-level (không hạ VM về `0`) là đúng hướng.

Cơ chế tại HEAD (`frame/worker.rs`):

- `nap_bien_cap_tep` (dòng **519**) đầu mỗi step: nạp giá trị `it.env` (globals) vào **mọi khung
  gọi đang treo** của frame, chỉ với `sym` đã có trong env.
- `dong_bo_bien_cap_tep` (dòng **563**) sau từng work-item ghi (`ApplyAssignVar`, `ApplyAssignPath`,
  `ApplyMethod { base_var }`): ghi về globals + mọi khung đang treo, **kể cả khi item trap**
  (side-effect trước trap vẫn thấy được).
- `buoc_frame_dong_bo` (dòng **592**) là bản sao loop `step_frame_owned` (`frame.rs:768`) có chèn
  đồng bộ; nhánh quota/Pending, pop khung, merge closure, hoàn nguyên tham số giữ nguyên.
- Không có `main`: `it.env = globals.clone()` và thân cấp tệp chạy **trực tiếp** trên globals
  (`frame/worker.rs:103`) — cùng lớp lỗi, sửa cùng cách.

Kiểm semantics (không chỉ stdout): chương trình đọc/ghi global qua hàm, qua `await` lồng trong hàm,
closure/env tạm — HEAD khớp **VM** từng dòng, còn `main` cũ lệch:

```
# /tmp/opencode/st318a/sem_probe.vis (var g=0, h=10; task ghi g+=1,h+=2; main đọc trước/giữa/sau)
HEAD interp : a 10 / b 10 / task 13 / c 13 / d 16   exit 0
HEAD --vm   : a 10 / b 10 / task 13 / c 13 / d 16   exit 0
MAIN interp : a 10 / b 10 / task 10 / c 10 / d 10   exit 0   (lỗi cũ)
```

### 2.2 Chạy thật `w2.vis` (CLI production)

```
vis run w2.vis              -> 1\n  exit 0
vis run --vm w2.vis         -> 1\n  exit 0
vis run --workers 1 w2.vis  -> 1\n  exit 0
vis run --workers 2 w2.vis  -> VS0509: ghi chéo worker (W>1): biến s14 (task `tang`)  exit 101
```

Đối chiếu `main` cũ: `vis run` → `0`, `--vm` → `1` (đúng mô tả `#203`). `w11_*` chạy thật:
`w11_frame_w1_parity_ghi_bien_cap_tep` + `w11_frame_w1_ghi_qua_ham_duoc_goi` PASS trong cụm 13/13.

Không-main (`let t = spawn tang(); await t; print(g)`): HEAD `1` exit 0; `main` cũ
`VS0501 biến g chưa định nghĩa trong frame test` exit 101. Đây là cải thiện cùng lớp, **chưa có
test khoá** (P3-B).

## 3. `#204` — non-frame `final_join` surface `VS0506`

`ap_va_flush` (`orchestrate.rs:93-110`): sau khi áp outcome, lấy `Err` đã cache trong `p.task.kq`
(do `ap_dung_outcome`/`loi_task` tạo) rồi `loi.or(loi_ket)`:

- **thứ tự**: trap `dirty` (`VS0509`) / `cancelled` (`VS0506 "tác vụ bị huỷ"`) từ `flush_proxy`
  **thắng**; chỉ khi flush sạch mới dùng lỗi `kq` — đúng yêu cầu;
- **không báo lại**: `flush_proxy` trả `None` nếu `p.flushed` (đã `await`) → `?` thoát, trap cũ
  không lặp;
- **message/span**: `loi_task` dùng `task.ten` + `task.span` (span lúc tạo), message
  `tác vụ \`ten\` lỗi: <mã gốc>: <msg>`; worker interp/VM đã format `Err` đúng dạng này
  (`vis-vm/src/vm/worker.rs:442`) nên **byte-identical** với frame W=1.

`w12_non_frame_loi_task_khong_await_w2_w4` chạy thật (interp `run_workers_parallel` + VM
`run_workers_parallel_vm`, W=2/4): exit **1**, stdout **`sau\n`**, diagnostic `VS0506` chứa `hong`,
`message` **bằng** frame W=1. CLI async `--workers 2 /tmp/opencode/st318a/nonframe.vis` cho cùng
kết quả (`sau`, `VS0506`, exit 101).

Giữ thứ tự stdout: task in `truoc` **trước khi trap**, main in `sau` → cả W=1 lẫn W=2/4 in
`truoc` rồi `sau` theo đúng thứ tự đã xảy ra (output của chính task lỗi vẫn được ghi trước trap):

```
HEAD --workers 2 print_err.vis -> "sau\ntruoc\n" + VS0506, exit 101
```

## 4. Cổng độc lập (log ngoài repo)

| Cổng | Kết quả | Log / sha256 |
|---|---|---|
| `st317b_worker_orchestrate` | **13/13** pass, 0 ignore | (trực tiếp) |
| `vs3_worker_frame_lib` / `frame_vm` / `eval_parallel` / `eval_local` / `eval_vm` / `vd284_spawn_method` | **10 / 7 / 13 / 8 / 10 / 12** = **60/60** | `--test-threads=1` |
| `./vis-dev gate --fast` | **PASS** — nextest **3904/3904** (4 skip, 1 slow, 109,2 s); fmt/clippy/R1..R10/DAG 23-5-0/self-test/doc-examples OK; R8 **39/40** (`vis-cli` 8/15) | `/home/ps/vislogs/review-st318a-gate.log` — `f838cfe1…20e2` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** — 100.000 sinh, **93.768 chạy**, **70,0 s**, 1.428/s, seed `3523158054` | `/home/ps/vislogs/review-st318a-diff.log` — `645865eb…ea151` |
| `cargo clippy --workspace --all-targets -- -D warnings` | **exit 0**, 35,7 s | `/home/ps/vislogs/review-st318a-clippy.log` — `fc583dcc…a37fd` |

## 5. Hợp nhất

```
git merge-tree --write-tree main HEAD       # main = baa2dbd3
b9a8ff8467f991e049c03535f8d6e3d34f55d027
CONFLICT (content): agent/STATE.md   (stage 1/2/3 — đúng dự kiến, không tệp khác)

git merge-tree --write-tree 2c7dbef6 HEAD   # nền lúc giao
e1cab1e01c0785fcc16408c14162d60b2ace4a6f
CONFLICT (content): agent/STATE.md   (không tệp khác)
```

Chỉ `agent/STATE.md` xung đột nội dung — A0 xử lý khi hợp nhất; mã/test không xung đột.

## 6. Finding

### P2-A — `nap_bien_cap_tep` làm chậm 10–14× chương trình nhiều biến top-level + nhiều `await`

**Repro** (sinh ngoài repo, `/tmp/opencode/st318a/many_const20k.vis`): 20.000 `const C_i: int = i`,
`main` vòng 2.000 lần `await spawn dummy()`, in `C_19999`. Cùng máy, cùng chế độ debug, xen kẽ:

| Chương trình | HEAD `0482e5ce` | `main` `fb725479` | Tỉ lệ |
|---|---|---|---|
| 5.000 const + 1.000 await | 2,51 s | 0,26 s | **9,7×** |
| 20.000 const + 2.000 await (3 lần, median) | **20,4 s** | **1,5 s** | **13,6×** |
| 0 const + 2.000 await | 0,05 s | 0,05 s | 1,0× |
| 20.000 const + 0 await | 0,62 s | 0,63 s | ~1,0× |

**Nguyên nhân**: `nap_bien_cap_tep` (worker.rs:519) **clone toàn bộ** `globals` và **insert lại
từng khoá** vào mọi env của frame ở **mỗi turn**, dù không có ghi nào; `globals` chứa cả `const`
(bất biến). Chi phí O(số biến top-level × số turn × độ sâu call), chỉ xảy ra khi có `main`
(main-env là clone; không-main thì `ptr_eq` bỏ qua). Các chương trình 5k/20k const không hề ghi
global — đây là overhead thuần.

**Đề xuất**: chỉ đồng bộ **sym thực sự đổi** (dirty set đẩy từ `dong_bo_bien_cap_tep`, hoặc chỉ
giao `frame.mutable ∩ globals` + `let` top-level một lần), thay vì quét toàn bộ globals mỗi turn;
thêm benchmark/regression nhỏ (5k const + 1k await, ngưỡng thời gian tương đối). **P2** (không sai
kết quả, nhưng ảnh hưởng workload const lớn — repo đã có benchmark 25k const).

### P2-B — `final_join` dừng ở trap đầu ⇒ mất stdout của task **sau** task lỗi (regression so với `main`)

**Repro** `/tmp/opencode/st318a/two.vis`:

```vis
async fn hong() -> int:
    let a = 10
    let b = 0
    a % b

async fn ok():
    print("b")

fn main():
    let t1 = spawn hong()
    let t2 = spawn ok()
    print("sau")
```

```
HEAD --workers 2 / --workers 4 / --vm --workers 2 : "sau\n"      + VS0506, exit 101
main --workers 2 / --workers 4 / --vm --workers 2 : "sau\nb\n"   + VS0506, exit 101
HEAD --workers 1                                  : "sau\nb\n"   + VS0506, exit 101
```

`ap_va_flush` nay trả trap lỗi `kq`; cả `exec.rs:355-359` và `vm/worker.rs:372-376` **return Err
ngay** trong vòng outcome theo spawn order, nên output của các outcome sau chưa được `push_str`.
Trước đây lỗi `kq` không được `ap_va_flush` trả (đó là `#204`), nên vòng flush chạy hết rồi
`frame_final_join` mới trap. Hệ quả: W=1 in `b`, W≥2 mất `b` — phá parity stdout trên đường lỗi;
non-frame dùng **cùng** vòng lặp nên cùng cơ chế (w12 không phủ ca task sau có output).

**Đề xuất**: gom trap đầu tiên, **tiếp tục** flush/ghi output các outcome còn lại, trả trap sau
vòng lặp (hoặc tách “lỗi” khỏi “output” trong `ap_va_flush`); thêm test khoá: task lỗi trước +
task in `b` sau, W=1/2/4 × interp/VM, assert `sau\nb\n` + VS0506. **P2** (mất output người dùng,
regression xác định; chương trình vẫn exit 1).

### P3-A — `buoc_frame_dong_bo` sao lặp loop `step_frame_owned`

Đã so khớp từng dòng: ngoài phần chèn đồng bộ + comment, loop **trùng khít** bản gốc (kể cả nhánh
quota, pop khung, merge closure/hoàn nguyên tham số) — **chưa drift**. Rủi ro bảo trì: sửa
`frame.rs:768` mà quên bản sao. Đề xuất comment trỏ ngược ở `frame.rs` + gộp bằng hook/callback khi
mở lane. **P3** (đã ghi nợ trong journal Builder).

### P3-B — ghi chú

- Nhánh **không-main** đổi hành vi (từ `VS0501` sang chạy đúng) nhưng **không có test khoá**;
  nên thêm 1 ca frame parity no-main.
- `vs3_worker_eval_vm::vm_khong_tao_them_thread_sau_khi_join` đếm thread **toàn process**; chạy
  riêng module (kể cả song song) **10/10** và nextest (mỗi ca một process) xanh. Diff ST-318a
  **không tạo thread** ⇒ flake khi chạy cả binary là **có sẵn**, không phải regression.

## 7. Đề nghị A0

1. **Chấp nhận PASS** cho `#203`/`#204` (đúng tiêu chí, bằng chứng đầy đủ, không P1).
2. **Trước khi hợp nhất**: nên cho Builder vá **P2-B** (vá nhỏ, rủi ro thấp, giữ output + parity
   W=1) và **P2-A** (chỉ đồng bộ dirty/mutable thay vì quét toàn bộ globals). Nếu chưa mở lane,
   đăng ký 2 nợ mới (ngoài `#204`) với repro + ngưỡng trong §6, và ghi rõ `#203`/`#204` chỉ đóng
   **kỹ thuật**, chưa đóng nợ hiệu năng/mất output.
3. Cập nhật `SO-NO.md`/`QUEUE.md`/`STATE.md` theo quy trình; không cần sửa mã trong worktree này.

---

## 8. Addendum F1 (`640180e0`) — kiểm toán vá P2-A/P2-B

> **Người kiểm**: Gatekeeper (Agent 1), 2026-11-27 (giờ thấp điểm), tiếp audit `7d754309`.
> **Đối tượng**: HEAD **`640180e0`** (code `0a493f92`, journal `640180e0`), nền audit trước
> `7d754309`; nhánh `plan/st318a`. **Không sửa mã**, không push, không `cargo clean`.
> **`main` đối chiếu**: **`9045dd78`** (`[REVIEW] merge ST-318e`). Binary `main` dùng đo là
> `/home/ps/ViScript/target/debug/vis` (dựng @ `fb725479`; các commit tới `9045dd78` chỉ tách
> module/move nguyên văn, không đổi runtime).
> **Hạ tầng**: `CARGO_BUILD_JOBS=2`; **`TMPDIR=/home/ps/vislogs/tmp`** cho mọi cargo/gate/diff
> (bài học `QuotaExceeded` tmpfs ở gate Builder); đĩa `/home` **18 → 19 GB** trống (> 6 GB).
>
> **Kết luận: PASS** — **0 P1**; **P2-A đạt đúng kịch bản 20k `const` + 2k `await`** (~1,0× `main`,
> không còn 20 s), **P2-B fixed** (stdout `sau\nb` + trap đầu, mọi W/engine), test **15/15**.
> **1 P2-C mới/còn lại**: `nap` vẫn quét **toàn bộ `frame.mutable`** mỗi turn ⇒ chương trình nhiều
> `var` top-level + nhiều `await` vẫn chậm 13–33×; cần dirty-set hoặc đăng ký nợ trước hợp nhất.

### 8.1 Phạm vi

```
640180e0 [ST-318a] F1 journal: so do perf truoc/sau + gate 3906/3906 + diff 100k lech=0
0a493f92 [ST-318a] F1: perf nap bien (chi var) + flush het stdout truoc trap final_join
```

`git diff --stat 7d754309..640180e0` — **5 tệp, +163/−12**: `frame/worker.rs` +23/−2,
`rt/exec.rs` +13/−2, `vm/worker.rs` +13/−2, test W +64, journal Builder +62.
`git status --short` **rỗng** trước/sau phiên (journal review này là ngoại lệ duy nhất, được commit).
**`crates/vis-runtime/src/worker/orchestrate.rs` KHÔNG đổi** (`git diff --name-only` kiểm exact);
không chạm `vis-types`/`vis-hir`/`vis-diag`/`vis-core`/`host_parity`; 0 tệp `.rs` cấp 1 mới,
0 `unsafe`, `git diff --check` sạch.

### 8.2 P2-A — `nap_bien_cap_tep` chỉ quét `frame.mutable`

Code (`frame/worker.rs:518-540`): `ds = frame.mutable` (sym `var`), `filter_map` chỉ giữ sym có
trong `globals` (loại sym local), clone **chỉ các giá trị đó**; `ds` rỗng ⇒ return sớm. Không còn
clone toàn bộ `globals` (const/let bất biến bị bỏ). Semantics `#203` giữ nguyên (5 ca §8.4).

**Đo lại đúng kịch bản** `perf20k.vis` = 20.000 `const Ci: int = i` + 2.000 `s = s + await spawn tang(i)`,
debug, `vis run --workers 1`, xen kẽ main/F1 cùng lượt (load ~2–3):

| Binary | 3 lần (ms) | Median | Tỉ lệ |
|---|---|---|---|
| `main` `fb725479` | 1.975 / 2.217 / 1.678 | **1.975 ms** | 1,00× |
| HEAD **trước F1** (`7d754309`; audit đầu + log Builder) | 20.379 (audit, 3 lần) / 20.173 (Builder) | **~20,3 s** | **~10,3×** |
| HEAD **sau F1** (`640180e0`) | 1.658 / 2.411 / 1.968 | **1.968 ms** | **0,996×** |

Đối chứng thêm trên `many_const20k.vis` (20k const + 2k await dạng vòng lặp, 10 lượt xen kẽ):
main median **2.264 ms**, F1 median **2.510 ms** (1,11× do 5 lượt đầu tải cao); 5 lượt cuối
cùng cửa sổ: main **2.143 ms** vs F1 **2.146 ms** = **1,001×**. Kết luận: **đạt kỳ vọng ≤ ~2×**,
không còn 20 s. Log: `/home/ps/vislogs/review-st318a-f1-perf.log`.

### 8.3 P2-C — `nap` vẫn O(số `var` × số turn): var-heavy còn 13–33×

`frame.mutable` chứa **mọi** `var` (kể cả var không bao giờ đổi); mỗi turn vẫn duyệt + clone
**toàn bộ** các var có trong `globals` rồi insert lại. Chương trình nhiều `var` top-level + nhiều
`await` vì thế vẫn rất chậm (cùng máy, debug, `vis run --workers 1`, load ~2–3):

| Chương trình | `main` | F1 `640180e0` | Tỉ lệ |
|---|---|---|---|
| 20.000 `var` + 2.000 await (`many_var20k.vis`) | 1.347 / 1.849 ms | 44.946 / 45.085 ms | **~24–33×** |
| 5.000 `var` + 1.000 await (`many_var5k.vis`) | 385 ms | 7.325 ms | **19×** |
| 1.000 `var` + 10.000 await (`many_var1k_10kawait.vis`) | 1.161 ms | 15.297 ms | **13×** |
| 20.000 `var` + 0 await (đối chứng) | 705 ms | 682 ms | ~1,0× |

Đối chứng 0-await chứng minh overhead nằm ở **mỗi turn `nap`**, không phải khởi tạo/parse. Đây là
hệ quả trực tiếp của thiết kế "đồng bộ cả bộ mutable mỗi turn"; repro + số đo ở log perf.
**Đề xuất**: dirty-set các sym vừa ghi (`dong_bo_bien_cap_tep` đẩy vào set, `nap` chỉ áp set rồi
clear) hoặc version counter; thêm regression nhỏ (1k var + 10k await) vào bộ đo. **P2** (không sai
kết quả; nếu A0 coi hiệu năng var-heavy là chặn merge thì phải vá tiếp).

### 8.4 Semantics `#203` — 5 ca Builder nêu, interp ↔ VM

Chạy thật `/tmp/opencode/w2*.vis` (binary HEAD), cả `vis run` và `vis run --vm`:

| Ca | File | Kỳ vọng | interp | VM |
|---|---|---|---|---|
| `w2` cơ bản | `w2.vis` | `1`, exit 0 | **1** | **1** |
| ghi qua hàm | `w2nested.vis` | `1`, exit 0 | **1** | **1** |
| 2 task cộng dồn | `w2two.vis` | `2`, exit 0 | **2** | **2** |
| main ghi trước `await` | `w2read.vis` | `5`, exit 0 | **5** | **5** |
| không `main` | `w2nomain.vis` | `1`, exit 0 | **1** | **1** |

Thêm `w2loop.vis` (vòng lặp top-level, `tong=14`): interp/VM đều **14**. Test khoá:
`w11_frame_w1_no_main_ghi_bien_cap_tep` mới PASS. Log: `review-st318a-f1-semantics.log`.

### 8.5 P2-B — `final_join` giữ trap đầu + flush hết output

Code (`rt/exec.rs:347-368`, `vm/worker.rs:365-386`): `let mut trap = None`; mỗi outcome vẫn
`push_str(output)`; `if trap.is_none() { trap = loi; }`; sau vòng lặp mới `Err(trap)`. Không đổi
message/mã/span; outcome đã flush (`flushed`) vẫn bỏ qua qua `ap_va_flush → flush_proxy` trả `None`.

Repro `hong` lỗi trước + `ok` in `b` sau (`/tmp/opencode/st318a/two.vis`) — **tất cả** đều
stdout **`sau\nb`**, exit **101**, `VS0506: tác vụ \`hong\` lỗi: VS0502: lấy dư cho 0`:

| Engine | W=1 | W=2 | W=4 |
|---|---|---|---|
| interp (`vis run`) | `sau\nb` | `sau\nb` | `sau\nb` |
| VM (`vis run --vm`) | `sau\nb` | `sau\nb` | `sau\nb` |

`w13_final_join_flush_het_output_truoc_trap` chạy thật PASS: frame W=1 + non-frame W=2/4 cả hai
engine, assert `sau\nb\n`, exit 1, `VS0506` `hong`. (Trước F1 W=2 = `sau\n` — test này đỏ đúng
như Builder ghi.)

### 8.6 Cổng lại (log ngoài repo)

| Cổng | Kết quả | Log / sha256 |
|---|---|---|
| `st317b_worker_orchestrate` | **15/15** pass, 0 ignore | `review-st318a-f1-test.log` — `cfd95f92…5da99` |
| `./vis-dev gate --fast` (`TMPDIR` đĩa) | **PASS** — nextest **3906/3906** (4 skip, 66,9 s); fmt/clippy/R1..R10/DAG 23-5-0/self-test/doc-examples OK; R8 39/40 | `review-st318a-f1-gate.log` — `a1464d55…a84a9` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** — 100.000 sinh, **93.768 chạy**, **85,7 s**, seed `3523158054` | `review-st318a-f1-diff.log` — `c247eb6b…cb300` |
| `cargo clippy --workspace --all-targets -- -D warnings` | **exit 0**, 5,19 s | `review-st318a-f1-clippy.log` — `271025c2…01212` |

Gate lần này **không** gặp `QuotaExceeded` nhờ `TMPDIR=/home/ps/vislogs/tmp` (ghi rõ theo yêu cầu).
File sau F1: `frame/worker.rs` **663**, `rt/exec.rs` **375**, `vm/worker.rs` **492**, test **565**
dòng — đều < 1.500 (R1); R8 vẫn 39/40 (0 tệp cấp 1 mới).

### 8.7 Hợp nhất

```
git merge-tree --write-tree main HEAD     # main = 9045dd78
c211a39575e5e3140759eb1378ad573d4df1e123
CONFLICT (content): agent/STATE.md   (stage 1/2/3)
```

**Chỉ** `agent/STATE.md`; 0 tệp mã/test xung đột.

### 8.8 Kết luận addendum

- **PASS** cho phạm vi F1: P2-B fixed hoàn toàn; P2-A đạt đúng kịch bản được giao (20k const +
  2k await: **1,968 s** vs main **1,975 s**, ~1,0×; trước F1 ~20,3 s); semantics 5 ca + 15/15 test;
  gate/diff/clippy sạch; merge-tree chỉ `STATE.md`.
- **P2-C phải được xử lý**: var-heavy (1k–20k `var` + 1k–10k await) còn **13–33×**; nếu chưa vá
  ngay, A0 đăng ký nợ mới kèm repro `many_var1k_10kawait.vis` (13×) / `many_var20k.vis` (24–33×)
  và ngưỡng đo. Không có P1, không sai ngữ nghĩa `#203`/`#204`.
