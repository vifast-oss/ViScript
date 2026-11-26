<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-318a — Đóng nợ `#203` (frame W=1 lệch interp↔VM) + `#204` (non-frame nuốt lỗi task)

- **Ngày**: 2026-11-27 (giờ thấp điểm) · **Vai**: Builder (Agent 2) · **Nhánh**: `plan/st318a`
- **Nền**: `main` @ `2b5ccb51` (`VD-295`, Sóng 6 `DONE`)
- **Worktree**: `/home/ps/viswt/st318a` — không chạm repo chính `/home/ps/ViScript`.
- **Căn cứ**: `VD-295` §3 (đăng ký `#203`/`#204`), `JOURNAL/st317b-tach-tep.md` §2b.7,
  `JOURNAL/review-st317b-p2b.md` F1.3.

## 0. TL;DR

| Nợ | Trước | Sau | Hướng chọn |
|---|---|---|---|
| **`#203`** | `vis run w2.vis` → stdout **`0`**, exit 0 (interp frame W=1); `vis run --vm w2.vis` → **`1`**, exit 0 | cả hai **`1`**, exit **0** (W=1 frame), `--workers 2` vẫn `VS0509` | **Sửa interp frame** cho biến cấp tệp là ô **chung** giữa frame; **không** đổi VM |
| **`#204`** | non-frame `run_workers_parallel*`/`run_workers_parallel_vm*` W=2/4: exit 0, diagnostics rỗng khi task lỗi không ai await | cả hai engine: exit **1**, `VS0506` (message `tác vụ \`hong\` lỗi: …`), output sạch giữ nguyên | **`ap_va_flush` dùng chung** trả trap `kq` đã cache (`or` sau trap dirty/cancel) |

## 1. `#203` — hướng sửa và lý do

**Chọn: interp frame W=1 phải in `1` (không phải VM về `0`).**

1. **SPEC §5.4**: `--workers N`, `N = 1` "giữ đường tuần tự cũ"; snapshot + `VS0509` chỉ áp cho
   `N > 1`. Đường tuần tự của interp non-frame dùng **một `Env` chung** (`Interp.env`), nên task
   ghi `var` top-level thấy ngay ở `main`.
2. **Parity + test đã khoá**: `w2_ghi_cheo_vs0509_ca_hai_engine` (ST-317b W2) đã assert
   **W=1 non-frame cả hai engine** in `1\n`; VM frame W=1 cũng in `1`. Chỉ interp frame lệch.
3. **Nguyên nhân**: `run_frame_driver` tạo `globals` rồi mỗi frame giữ **`Env` clone**; đọc ưu
   tiên env của frame nên bản clone cũ che giá trị mới. Ghi trong task chỉ vào clone của task,
   không bao giờ tới main.

**Sửa (`crates/vis-interp/src/frame/worker.rs`)**:

- `it.env` (globals, do `khoi_tao_bien_cap_tep_frame` gắn) là **nguồn chung** cho các `sym` đã
  có trong đó; tham số/`let` trong thân (sym riêng) vẫn ở `Env` của frame.
- `nap_bien_cap_tep`: mỗi step của một frame nạp giá trị top-level mới nhất vào **mọi** khung
  gọi đang treo của frame (task/main thấy ghi của nhau khi resume).
- `buoc_frame_dong_bo`: bản sao `step_frame_owned` (frame.rs) có **đồng bộ sau từng work-item**
  cho ba đường ghi được biến cấp tệp (`ApplyAssignVar`, `ApplyAssignPath`, writeback
  `ApplyMethod`); nhờ vậy `tang(); print(g)` trong **cùng một step** cũng đúng, kể cả khi lệnh
  ghi nằm trong hàm được gọi (env tạm bị pop). Mọi nhánh khác giữ nguyên hành vi bản gốc.
- Không có `main`: thân cấp tệp chạy **trực tiếp** trên `globals` (thay vì clone) — cùng lớp
  lỗi, cùng cách sửa; chương trình có `main` không đổi cách khởi tạo.
- Không đụng `frame.rs`/`expr.rs`/`proc_expr.rs` (ngoài lane); phần sao lặp loop có ghi chú rõ
  để lần sau gộp khi mở lane.

## 2. `#204` — surface `VS0506` ở `final_join` non-frame

`vis_runtime::worker::ap_va_flush` (một nguồn cho interp `rt::exec::final_join` và VM
`vm/worker::final_join`):

- `ap_dung_outcome` đã cache `Err(trap)` vào `task.kq`; nay trap đó được trả kèm khi flush.
- **Thứ tự giữ nguyên**: trap `dirty` (`VS0509`) / `cancelled` (`VS0506 "tác vụ bị huỷ"`) của
  `flush_proxy` **thắng**; chỉ khi flush sạch mới dùng lỗi `kq`.
- Proxy đã flush (đã `await`) không bị báo lại (`flush_proxy` trả `None`).
- Message/mã/span lấy **nguyên** trap `loi_task` (VS0506, `tác vụ \`ten\` lỗi: <mã gốc>`, span
  lúc tạo task) ⇒ byte-identical với đường frame.

## 3. Test khoá (thêm vào `st317b_worker_orchestrate.rs`, không tệp `.rs` cấp 1 mới)

| Test | Phủ |
|---|---|
| `w11_frame_w1_parity_ghi_bien_cap_tep` | `#203`: frame W=1 interp ↔ VM trên `GHI_CHEO` — stdout `1\n`, exit 0, codes/value khớp |
| `w11_frame_w1_ghi_qua_ham_duoc_goi` | `#203`: ghi trong hàm được gọi + đọc lại cùng step của `main` — hai engine `1\n` |
| `w12_non_frame_loi_task_khong_await_w2_w4` | `#204`: `run_workers_parallel*` + `run_workers_parallel_vm*` W=2/4 — exit 1, stdout `sau\n`, `VS0506` chứa `hong`, **message bằng frame W=1** |

Không `#[ignore]`; 13/13 test của tệp PASS.

## 4. Cổng

| Cổng | Kết quả |
|---|---|
| `cargo fmt --all` | sạch |
| `cargo test -p vis-conformance --test lib_api_it st317b_worker_orchestrate` | **13/13 PASS** |
| `cargo test ... vd284_spawn_method / vs3_worker_frame_lib / vs3_worker_frame_vm / vs3_worker_eval_parallel / vs3_worker_eval_local / vs3_worker_eval_vm` | PASS (12/10/7/13/8/10) |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `./vis-dev gate --fast` | **PASS** — nextest **3904/3904**, 4 skipped (117,7 s); `check` gồm R1..R10 OK, DAG 23/5/0; log `/home/ps/vislogs/st318a-gate.log` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** — 100.000 chương trình (93.768 chạy), 65,7 s, 1.521/s; log `/home/ps/vislogs/st318a-diff.log` |

## 5. Việc còn lại / ghi chú

- **Đính chính lane (`VD-296`)**: `crates/vis-runtime/src/rt/exec.rs` không tồn tại — `final_join`
  non-frame nằm ở `crates/vis-interp/src/rt/exec.rs`. Lát này **không phải sửa** tệp đó vì cả hai
  engine gọi chung `vis_runtime::worker::ap_va_flush`; sửa một nơi là đủ cho interp + VM.
- `frame/worker.rs` **652** dòng, `orchestrate.rs` **149**, test **501** — đều < 1.500 (R1).
- 0 `unsafe`; không tạo tệp `.rs` cấp 1 (R8 giữ 39/40).
- Loop sao lặp `buoc_frame_dong_bo` là điểm nợ kỹ thuật đã ghi: gộp vào `frame.rs` khi lane mở
  (cần callback/hook giữa các work-item), tránh drift nếu `step_frame_owned` đổi.
- Ghi chú hạ tầng: `cargo test` thường (một process, chạy song song trong binary) có thể làm ca
  `vs3_worker_eval_vm::vm_khong_tao_them_thread_sau_khi_join` đỏ do **đếm thread toàn process**;
  chạy riêng hoặc qua `nextest` (mỗi ca một process — cổng chuẩn) PASS. Không phải regression.

## 6. F1 — vá audit sau `7d754309` (P2-A/P2-B/P3)

### P2-A — `nap_bien_cap_tep` quét toàn bộ globals mỗi turn (perf)

- **Nguyên nhân**: frame W=1 clone **mọi** `globals` (gồm cả `const`) rồi insert lại vào mỗi env mỗi
  turn; chương trình 20k `const` + 2k `await` ⇒ 2k × 20k clone → ~13×.
- **Vá**: chỉ nạp **`frame.mutable`** (`var` cấp tệp — `const`/`let` bất biến, không bao giờ đổi;
  sym cục bộ không có trong globals bị lọc). Không đổi semantics `#203` (ghi `var` top-level
  trong task/closure vẫn thấy; `await` lồng vẫn đúng — test W11/W12 giữ nguyên).
- **Đo lại đúng kịch bản** (20.000 `const` + 2.000 `await`, debug, cùng máy, `vis run`):

| Binary | Lần đo (s) | Nhận xét |
|---|---|---|
| `main` `2b5ccb51` | 1,475 / 1,483 / 1,532 / 1,575 / 1,475 | median **1,507** |
| HEAD trước F1 (`7d754309`) | 19,394 / 20,173 / 20,190 | median **20,17** — ~**13,4×** main (khớp audit ~20,4 s) |
| HEAD sau F1 | 1,961 / 1,963 (cửa sổ tải thấp) | ~**1,3×** main; dưới tải nặng min 2,249 vs main min 2,555 **cùng lượt** ⇒ không còn overhead đo được |

- Đối chứng tách thành phần (binary C = loop gốc `step_frame_owned`, B = loop sao không sync):
  min cùng lượt dưới tải: main 2,555 / C 2,374 / B 2,326 / F1 2,249 s — F1 ≤ đối chứng.

### P2-B — `final_join` trả trap đầu làm mất stdout task sau

- **Vá** (`vis-interp/src/rt/exec.rs`, `vis-vm/src/vm/worker.rs`): vòng outcome **gom** trap đầu
  (`Option<Trap>`, `if trap.is_none()`), **flush hết** output mọi task theo spawn order rồi mới
  trả trap. Message/mã/span giữ nguyên (`VS0506` của task lỗi đầu tiên).
- **Repro** (`hong` lỗi trước + `ok` in `b` sau):
  - Trước F1 (W=2): `sau` — test mới **đỏ** `left "sau\n" / right "sau\nb\n"`.
  - Sau F1: `vis run` / `--vm` / `--workers 2/4` / `--vm --workers 2/4` đều stdout
    **`sau\nb`**, exit **101**, `VS0506: tác vụ \`hong\` lỗi: VS0502: lấy dư cho 0 (%)`.

### P3

- Comment `buoc_frame_dong_bo` thêm cảnh báo ⚠️ **giữ khớp từng nhánh** với
  `frame.rs::step_frame_owned`.
- Thêm test **không `main`**: `w11_frame_w1_no_main_ghi_bien_cap_tep` — task ghi `var` top-level,
  lệnh cấp tệp đọc lại; interp ↔ VM đều `1\n`, exit 0.

### Test khoá mới

| Test | Phủ |
|---|---|
| `w13_final_join_flush_het_output_truoc_trap` | P2-B: frame W=1 + non-frame W=2/4 hai engine — stdout `sau\nb\n`, exit 1, `VS0506` `hong` |
| `w11_frame_w1_no_main_ghi_bien_cap_tep` | P3: no-main parity frame W=1 |

`st317b_worker_orchestrate`: **15/15 PASS**.

### Cổng lại sau F1

| Cổng | Kết quả | Log |
|---|---|---|
| `cargo fmt --all` | sạch | — |
| `st317b_worker_orchestrate` | **15/15 PASS** | `/home/ps/vislogs/st318a-fix-test-st317b.log` |
| module worker liên quan | 12/10/7/13/8/10 PASS | — |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 | `/home/ps/vislogs/st318a-fix-clippy.log` · `0cb19ec3…91b1` |
| `./vis-dev gate --fast` | **PASS** — nextest **3906/3906**, 4 skipped (77,9 s) | `/home/ps/vislogs/st318a-fix-gate2.log` · `38bad8e5…31de` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** — 100.000 chương trình (93.768 chạy), 102,4 s | `/home/ps/vislogs/st318a-fix-diff.log` |

- Lần gate 1 (`st318a-fix-gate.log`): 3 ca `vis-host-native` đỏ vì `Os { code: 122, kind:
  QuotaExceeded }` khi `/tmp` (tmpfs) đầy 6,1/7,2 GB do cache binary `vs3-w2-bin-cache-*` của
  nhiều tiến trình; chạy lại 2 ca `quyen*` với `TMPDIR` trên đĩa ⇒ **PASS**. Gate lại đặt
  `TMPDIR=/home/ps/vislogs/tmp` (đĩa còn > 6 GB).

## 7. F2 — dirty-set cho `nap` (P2-C var-heavy)

- **Nguyên nhân P2-C**: F1 vẫn quét **toàn bộ `frame.mutable`** mỗi turn; chương trình khai báo
  nhiều `var` top-level (dù không ghi) làm `nap` clone O(#var) mỗi turn ⇒ 20k `var` + 2k await
  chậm ~20×, 5k var ~15×, 1k var + 10k await ~7×.
- **F2**: `DongBo` (trong `frame/worker.rs`) — sổ **dirty-set** `sym → seq` chỉ đánh dấu sym **thực
  sự bị ghi** (`DongBo::ghi` sau `ApplyAssignVar`/`ApplyAssignPath`/writeback `ApplyMethod`, kể cả
  no-main nơi env chính là globals); mỗi frame nhớ `frame_seq` và `nap` chỉ chép các sym có
  `seq > frame_seq` vào env của nó. `prune` dọn mục mà mọi frame sống đã thấy (chỉ khi sổ ≥ 64).
  Việc ghi vẫn cập nhật ngay mọi call-frame của frame đang chạy (đọc cùng step đúng như F1).
- Semantics `#203` giữ nguyên: 5 ca `w2`, `w2nested`, `w2two`, `w2read`, `w2nomain` interp = VM
  (`1/1/2/5/1`), khoá bằng test mới `w15` (2 task cộng dồn = `2`) và `w16` (main ghi trước await
  = `5`) cho **frame W=1 cả interp + VM**.
- **P2-C frame VM W≥2**: thêm `w17` dùng `vis_vm::run_async_workers_with_full_hosts_caps`
  (production `vis run --vm --workers N`) — task lỗi không await ⇒ exit 1, `VS0506` `hong`,
  stdout `sau\n`; W=2/4 PASS.
- Thêm `w14` var-heavy nhỏ (200 `var`, 3 sym bị ghi): `1/17/201/100`, parity interp ↔ VM.

### Đo lại (median 5 lần, main / F1 / **F2**, cùng lượt; binary debug cùng máy)

| Kịch bản | main (ms) | F1 (ms) | **F2 (ms)** | F2/main |
|---|---|---|---|---|
| 20k `var` + 2k await | 2.688 | **54.400** (1 lần; ~20×) | **2.775** | **~1,03×** |
| 5k `var` + 1k await | 422 | **6.332** (~15×) | **442** | **~1,05×** |
| 1k `var` + 10k await | 1.721 | **12.162** (~7×) | **2.065** | **~1,20×** |
| 20k `var` + 0 await (đối chứng) | 635 | 601 | **625** | ~0,98× |
| 20k `const` + 2k await | 2.389 | 2.371 | **2.403** | ~1,01× |

(Máy có tải nền ~10–13 load; số tuyệt đối cao hơn lúc rảnh, nhưng main/F1/F2 đo xen kẽ cùng
lượt nên tỉ lệ là đối chứng. Log: `/home/ps/vislogs/st318a-fix2-perf2.log`.)

### Cổng lại sau F2

| Cổng | Kết quả | Log |
|---|---|---|
| `cargo fmt --all` | sạch | — |
| `st317b_worker_orchestrate` | **19/19 PASS** (`w14..w17` mới) | `st318a-fix2-clippy.log` liên quan; test chạy trong gate |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 | `/home/ps/vislogs/st318a-fix2-clippy.log` |
| `./vis-dev gate --fast` (`TMPDIR` trên đĩa) | **PASS** — nextest **3910/3910**, 4 skipped (80,6 s) | `/home/ps/vislogs/st318a-fix2-gate3.log` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** — 100.000 chương trình (93.768 chạy), 89,2 s | `/home/ps/vislogs/st318a-fix2-diff.log` |

- Hai lần gate đầu đỏ **cùng một ca** `host_parity::net_http::http_bon_ham_parity` do timer lệch
  **1 ms** (`78 ms` vs `79 ms`, dưới tải nền); chạy riêng PASS, gate lại khi tải hạ PASS toàn bộ.
  Không liên quan mã `ST-318a` (lane `host_parity` bị cấm chạm).
- `w17` dùng API VM frame W≥2 có sẵn (`run_async_workers_with_full_hosts_caps`) — không phải
  bỏ qua ca nào.
- `bin/vis-bench --runs 2 --bin target/debug/vis`: **PASS** (stdout sha256 khớp, 5 chương trình ×
  2 engine; log `/home/ps/vislogs/st318a-fix2-visbench.log`).

## 8. F3 — prune dirty khi `ds` rỗng (+ method chỉ đọc) — P2-1/P3-1/P3-2

- **P2-1**: `DongBo::nap` khi `dirty` khác rỗng nhưng **mọi `seq ≤ frame_seq`** (frame đã thấy hết,
  vd chính nó vừa ghi 20k global) vẫn duyệt toàn bộ `dirty` rồi `return` ở nhánh `ds.is_empty()`
  **mà không `prune`** ⇒ `dirty` không co, O(#sym × #turn).
- **Vá**: `nap` không còn `return` sớm — sau khi cập nhật `frame_seq[id] = self.seq` **luôn** gọi
  `self.prune()`. `prune` giữ `s > min(frame_seq)` (mục mà **mọi** frame sống đã thấy bị bỏ; frame
  đứng sau vẫn còn mục cần). `prune` vẫn tự no-op khi sổ < 64 mục.
- **P3-1**: `sym_ghi_cua` chỉ trả `Some(base_var)` cho `ApplyMethod` khi `name == "push"` —
  `method_writeback` (proc.rs) chỉ ghi ngược với `push`; method chỉ đọc (`len`/`get`/`keys`/
  `first`/…) không còn đánh dirty.
- **P3-2**: thêm nhánh `Work::RedispatchMethod { base_var: Some(sym), .. }` cùng điều kiện
  `push` — nhánh resume sau `Pending` **có** gọi `method_writeback` nên an toàn và cần thiết.

### Đo lại write-heavy (`20k var` + main gán 20k biến = 1 + 2.000 await task rỗng)

Median 5 lần, xen kẽ, binary debug cùng máy (`/home/ps/vislogs/st318a-fix3-perf-write.log`):

| Binary | Lần đo (ms) | Median |
|---|---|---|
| `main` `2b5ccb51` | 1810 / 2538 / 2892 / 3427 / 3521 | 2892 |
| F2 (`fcae15df`) | 3410 / 4069 / 5718 / 5253 / 4992 | 4992 |
| **F3** | 2269 / 2565 / 3785 / 3507 / 2835 | **2835** (~0,98× main, ~1,76× → ~1,0× so F2) |

### Đối chứng không hồi quy (median 3 lần, F2 → F3)

| Kịch bản | F2 (ms) | F3 (ms) |
|---|---|---|
| 20k `var` + 2k await | 2861 | 2893 |
| 20k `const` + 2k await | 3010 | 2647 |
| 5k `var` + 1k await | 534 | 559 |

### Test

- Thêm `w18_frame_w1_main_ghi_nhieu_var_roi_await` (100 `var`, main ghi hết rồi 50 await task
  rỗng): `1\n100\n`, parity interp ↔ VM. `st317b_worker_orchestrate`: **20/20 PASS**.

### Cổng lại sau F3

| Cổng | Kết quả | Log |
|---|---|---|
| `cargo fmt --all` | sạch | — |
| `st317b_worker_orchestrate` | **20/20 PASS** (`w18` mới) | — |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 | `/home/ps/vislogs/st318a-fix3-clippy.log` |
| `./vis-dev gate --fast` (`TMPDIR` trên đĩa) | **PASS** — nextest **3911/3911**, 4 skipped (91,1 s) | `/home/ps/vislogs/st318a-fix3-gate.log` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** — 100.000 chương trình (93.768 chạy), 159,2 s (có fuzz nền) | `/home/ps/vislogs/st318a-fix3-diff.log` |
