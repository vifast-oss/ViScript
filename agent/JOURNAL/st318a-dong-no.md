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
