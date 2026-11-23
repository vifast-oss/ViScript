<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-257 · Stream D — KẾT QUẢ quota thử nghiệm + quét Q (PHASE 2)

**Branch**: `work/vd257-d-fairness` (rebase lên `fd4ba32` = A/B/C đã merge).
**Trạng thái**: mã + test xong, commit riêng, KHÔNG merge/push. **Đường MUST-NOT-BE-DEFAULT**:
mọi charge/quota bật qua test-hook nội bộ; không CLI flag, không API công, không VERSION/cache,
không đổi runtime mặc định. Leader chạy gate một làn sau merge.

**Trung thực**: B2 **chỉ được xử lý trên đường frame test**, KHÔNG sửa `vis run`/`vis run --vm`
(run-to-completion ở `rt.rs:1413` vẫn vậy). **Không chọn Q mặc định** — dưới đây là đường cong đo
được, A0 tự quyết. Không tuyên bố "fairness đã xong" ở sản phẩm.

---

## 1. Prerequisite — dãy sự kiện vào đường frame (một định nghĩa, không instrument lần hai)

| Việc | Bằng chứng |
|---|---|
| Frame path phát sự kiện khi bật hook | `lat257_fairness.rs::frame_path_co_su_kien_va_khong_double_charge` — trace có `E_STMT/E_EXPR/E_ASSIGN/E_BUILTIN`; hook tắt ⇒ trace rỗng (`f2` của A vẫn pass) |
| Dùng chung định nghĩa | Mọi điểm charge gọi `crate::rt::fee::frame_charge` → `Interp::charge` → `fee::charge` (`FeeKind`, `standard_weight`, trap `VS0501-BUDGET`). Không định nghĩa sự kiện thứ hai |
| Thứ tự `f(g(1))` | `frame_path_giu_thu_tu_f_g` assert dãy `E_CALL(f) → E_CALL(g) → E_EXPR(1)` — khớp `interp.rs:735` / `vd252 §3` |
| Budget check trước side-effect | Charge đặt tại nút Call TRƯỚC đối số, tại `Bind/SetVar/ApplyAssign*` TRƯỚC ghi, tại `LoopStep` mỗi lần thử guard. `ngan_sach_*` tests trap sát ranh giới |
| Resume không charge lại / không lặp side-effect | `resume_khong_charge_lai_va_khong_lap_side_effect` — suspend/resume thật qua nguồn giả; mỗi `(pc,kind,site)` xuất hiện đúng 1 lần, stdout `40\n40\n` |
| Trace frame I↔V byte-equal (FLAT) | `hai_engine_cung_trace_su_kien_voi_quota` — cùng dãy 8 sự kiện trên cả hai engine |

**Charge sites đã thêm** (đều mirror `interp.rs`): `frame/expr.rs` — `eval_work` entry (E_EXPR,
trừ Call/Spawn/Await), Call (E_CALL/E_BUILTIN + E_EXPR callee), Spawn (E_SPAWN), Await (E_AWAIT),
`exec_work` entry (E_STMT); `frame/proc.rs` — `Bind/ApplyLetElse/SetVar/ApplyAssignVar/Field/Index`
(E_ASSIGN), `LoopStep` (E_LOOP). `Work::Bind/ApplyLetElse/SetVar` được thêm field `span` để `site`
đúng. Resume (`Redispatch/RedispatchValue/RedispatchMethod`, `ApplyAwait`) **không** charge lại.

---

## 2. Quota riêng — reset mỗi turn, yield đuôi ready, KHÔNG trap

- `frame/fee.rs::Budget`: thêm `quota_enabled/quota_limit/quota_remaining/quota_hit/quota_yields`.
  `charge()` trừ **cùng `w`** vào cả `remaining` (budget) và `quota_remaining`; quota vượt 0 chỉ đặt
  cờ, **không trap, không chẩn đoán**. Budget vẫn trap `VS0501-BUDGET` y hệt.
- Reset: interp `frame.rs` trước `step_frame` mỗi turn; VM `vm_frame.rs` trước `Th::exec` mỗi turn.
- Yield: interp tại `step_frame_owned` (sau `frame.done()`, trước pop work tiếp) — `IFrame` đã chứa
  đủ `calls/work/vals/loops/restore`, được `step_frame` cất lại nguyên vẹn. VM tại cuối thân loop
  `Th::exec` (sau `pc += 1` + side-effect, D6) — `Th` đủ trạng thái. Cả hai phát
  `Abort::Pending{task:None,wait:None}` ⇒ nhánh driver có sẵn `frame.ready.push_back(current)` —
  **đuôi hàng đợi, không trap**, ghi log `quota:<frame>`.
- VM: field quota trên `Vm`, hook thread-local `frame_budget_enable_for_test(..., quota)` trong
  `vm_frame.rs` — **không đổi chữ ký** `run_frame_for_test`/`FrameConfig`, không API công.

| Nghiệm thu | Kết quả |
|---|---|
| `quota_rieng_khong_phai_ngan_sach` | budget 40M / Q=300 ⇒ exit 0, in đủ `CPU_DONE 50000`, có `quota:` log, **không** `VS0501-BUDGET` |
| `ngan_sach_van_trap_khong_bi_quota_che` | budget 100 / Q=1M ⇒ exit 1, đúng `VS0501-BUDGET`, 0 yield |
| `ngan_sach_khong_reset_qua_nhieu_yield` | budget 400k (< ~450k cost) / Q=500 ⇒ **vẫn trap** sau >0 yield ⇒ quota reset KHÔNG làm reset/né budget |
| `hai_engine_cung_hanh_vi_voi_quota` | cùng stdout/exit/snapshot, số yield lệch **±1 turn** (VM bỏ sót yield ở instruction cuối vì `Th::exec` trả sớm khi hết frame) — ghi nhận, không tuyên bố byte-equal ở mọi workload |

---

## 3. Quét Q — số liệu thật, KHÔNG chọn default

Chạy: `cargo test -p vis-cli --test lat257_fairness -- --ignored --nocapture` (debug, 7 giá trị ×
7 lần/bản, bỏ warmup, seed `0x5644_3235_37 ^ Q*131 ^ r`, finite ~10 s). Workload B2 mixed:
CPU 50k vòng spawn **trước**, task I/O (`net.read` reactor giả) spawn sau, task ngắn spawn cuối.

| Q (cost units) | yields (median) | startup ms | `io_lat_cost` | `short_lat_cost` | thứ tự |
|---:|---:|---:|---:|---:|---|
| control (tắt) | 0 | 0.95 | 450,033 | 450,045 | **IO_START SAU CPU_DONE** (B2 tái hiện) |
| 1,000,000 | 0 | 0.57 | 450,033 | 450,045 | IO trước CPU (vì Q > tổng cost ⇒ thực tế không yield) |
| 200,000 | 2 | 0.57 | 200,020 | 200,032 | IO trước CPU |
| 50,000 | 10 | 0.56 | 50,020 | 50,032 | IO trước CPU |
| 10,000 | 52 | 0.54 | 10,020 | 10,032 | IO trước CPU |
| 2,000 | 262 | 0.57 | 2,020 | 2,032 | IO trước CPU |
| 500 | 1,050 | 0.52 | 520 | 532 | IO trước CPU |

**Đọc đường cong (không phải khuyến nghị sản phẩm):**
- `*_lat_cost` = tổng trọng số sự kiện chạy **trước** `print(IO_START)` / `print(SHORT_DONE)` —
  latency **monotonic, tất định** (đơn vị chi phí chuẩn hoá, không dùng wall-clock 1 ms resolution
  ở debug). Giảm **đơn điệu** từ ~450k (control) xuống ~520 khi Q giảm — đúng kỳ vọng chống đói.
- Startup (parse/lower/tyck, đo riêng ngoài workload) ~0.5–0.95 ms; **không** trộn vào latency.
- Điểm gãy: **Q = 1,000,000 cho yields = 0** (lớn hơn tổng cost ~450k) ⇒ không có hiệu ứng fairness
  dù bật hook. Q quá nhỏ (500) latency thấp nhất nhưng **1,050 yield** ⇒ overhead chuyển turn tăng
  mạnh. A0 cần chọn dựa trên đường cong này + đo release, **stream D không ấn số**.
- Control tái hiện đúng B2 (CPU_DONE và IO_START cùng chạy xong CPU trước) — chứng minh phép đo
  có đối chứng thật, không phải artifact của instrumentation.

---

## 4. Vòng đời đa sóng (`diag_snapshot()`, không chỉ RSS)

`vong_doi_da_song_voi_quota`: 5 wave giống hệt (mỗi wave spawn+await 5 task, quota Q=200), so
`DiagSnapshot` sau mỗi wave. Kết quả: **cả 5 wave đều `{live_tasks:0, frames:0, waiters:0,
ready:0, regs:0, timers:0}`** — đúng baseline sau `frame_drain_events + frame_final_join +
frame_reactor_shutdown`. Không có tăng đơn điệu; quota không làm rò rỉ `waiters/regs/timers`.

- **Baseline** = snapshot ngay sau enable/trước turn đầu (với fixture sạch = các trường trên); về 0
  khi mọi handle đã thả (`Rc::strong_count == 1` ⇒ `frame_reclaim` thu hồi).
- **Retained có chủ đích** (không tính leak): `Value::Task` còn binding, kênh trong `Scheduler.kenh`,
  nguồn giả `external`, reg reactor còn handle chủ, `strong_count > 1`. Giữ cố định qua mọi wave.
- **Tăng bất thường** (fail test): counter nào đó tăng qua wave, `ready > 0` sau quiescence,
  `regs/timers > 0` sau shutdown, hoặc snapshot wave k ≠ baseline dù retention set không đổi.

---

## 5. Validation (targeted, resource discipline)

| Lệnh | Kết quả |
|---|---|
| `cargo test -p vis-cli --test lat257_fairness` | **10 passed / 0 failed / 1 ignored** |
| `cargo test -p vis-cli --test lat257_fairness -- --ignored --nocapture` | 1 passed (quét Q, ~10 s) |
| `cargo test -p vis-cli --test lat257_a_coverage --test lat257_b_budget --test lat257_c_lifecycle --test lat256_budget` | A 13/13, B 5/5, C 11/11, VD-256 8/8 (+1 ignored) — **không regression** |
| `cargo check -p vis-interp -p vis-vm -p vis-cli` | sạch |
| `cargo fmt --all -- --check` | sạch |

Không full gate, không full suite, không benchmark nặng, không `cargo clean`, không đụng shared
log/fixture.

---

## 6. Gaps / trung thực

1. **B2 chưa "fixed" ở sản phẩm** — chỉ path frame test; `vis run` vẫn run-to-completion. Đây là
   cơ chế thử nghiệm để A0 đo/quyết, không phải thay đổi ngữ nghĩa async công bố.
2. **Không có Q mặc định** — bảng §3 là dữ liệu, không phải lựa chọn. Chưa đo release (chỉ debug).
3. **Số yield I↔V lệch ±1** ở workload loop (VM bỏ sót yield cuối) — hành vi (stdout/exit/snapshot)
   đồng nhất, trace FLAT byte-equal; không tuyên bố trace byte-equal mọi workload.
4. Phạm vi event parity tiếp tục kế thừa SUPPORTED DOMAIN của stream B (`match/with/try/?`, field/index
   đầy đủ, release measurement chưa phủ) — stream D không mở rộng thêm phạm vi đó.
5. Không đổi `rt.rs`/`stdlib.rs`/`net.rs`/`vis-bytecode/**`/`cache.rs`/manifest/lock/DECISIONS/QUEUE/
   test hay journal của stream khác. File sửa đúng 6 + 1 test mới (xem diff).

---

## 7. File PHASE 2 đã sửa

`crates/vis-interp/src/frame/{fee.rs,frame.rs,expr.rs,proc.rs}` (fee/quota core, reset+yield, charge sites),
`crates/vis-vm/src/{vm.rs,vm_frame.rs}` (quota fields/yield, hook+charge), và test mới
`crates/vis-cli/tests/lat257_fairness.rs`. Report này. **Không** commit source ở PHASE 1; PHASE 2
commit chỉ các tệp trên + report.
