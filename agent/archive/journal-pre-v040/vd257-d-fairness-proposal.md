<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-257 · Stream D — đề xuất CPU-FAIRNESS QUOTA (PHASE 1: phân tích + đo lường)

**Trạng thái**: **đề xuất, KHÔNG phải hiện thực**. PHASE 1 chỉ đọc mã + lập kế hoạch — **không sửa
bất kỳ tệp nguồn nào**, không chạy gate/suite/benchmark (VD-257 §5). **B2 CHƯA được sửa**: đường
mặc định `vis run` / `vis run --vm` vẫn run-to-completion, `CPU_DONE == IO_START` như đo trong
`BAO-CAO-SAN-SANG-THUC-TE-2026-11-20.md` §3.2 nhóm 3. **Không chọn default Q** — A0 dặn Q phải đo.

**Phạm vi cơ sở (đọc ở HEAD `fc297ba`)**: `agent/BAO-CAO-SAN-SANG-THUC-TE-2026-11-20.md` §1.2,
§2, §3.2/3.3; `agent/JOURNAL/vd252-charging-final.md` §1, §3; `agent/JOURNAL/vd256-budget-report.md`
§1, §2, §7, §8; `crates/vis-interp/src/rt.rs`, `crates/vis-interp/src/frame/**`,
`crates/vis-vm/src/vm_frame.rs`, `crates/vis-vm/src/vm.rs`, `crates/vis-cli/tests/lat*.rs`.

---

## 0. Tóm tắt cho A0 (3 dòng)

1. **Phát hiện chặn**: đường **frame test hiện không phát sự kiện ngữ nghĩa nào** ở interp
   (`frame/**` không gọi `charge`); VM chỉ phát khi `budget_fee` được bật (`vm.rs:741`). Muốn "quota
   từ cùng sự kiện" thì trước tiên phải nối event stream vào driver frame — đây là việc của stream
   A/C, không phải của D.
2. **Thiết kế**: một lần `charge()` vừa trừ `remaining` (budget toàn chương trình, hết ⇒ trap
   `VS0501-BUDGET`) vừa trừ `quota_remaining` (hạn ngạch **riêng**, reset mỗi lượt, hết ⇒ **yield
   về đuôi `ready`**, không trap). Yield đặt đúng ranh giới work-item/instruction — trạng thái frame
   đã lưu xong, sự kiện đã tính phí đúng một lần, không re-charge/không lặp side-effect.
3. **P2 sửa 6 tệp nguồn** (3 interp + 2 VM + 1 test mới), **tất cả nằm trong ownership của A/C** ⇒
   chỉ rebase sau khi A/C merge. Danh sách đầy đủ ở §5.

---

## 1. Phát hiện nền tảng (có bằng chứng file:dòng)

| Khẳng định | Bằng chứng |
|---|---|
| `charge` của interp **chỉ** nằm ở đường trực tiếp | `grep '\.charge('` trong `vis-interp/src` ⇒ chỉ `interp.rs:232,244,303,337,352,376,427,482,506,533,628,751,1004,1025`; **không có** trong `frame.rs`, `frame/proc.rs`, `frame/expr.rs` |
| Test parity VD-256 dùng đường trực tiếp, không dùng frame | `lat256_budget.rs:60-63` gọi `vis_interp::run(...)` (dòng 61), rồi `ifee::take_trace_for_test()` |
| VM frame **có** event hook nhưng mặc định tắt | `vm.rs:741` `vm.budget_charge(fi, pc, span)?` nằm trong `Th::exec`; `vm_frame.rs:74` trả `Ok(())` ngay khi `budget_fee == None`; `run_frame_for_test` (`vm_frame.rs:134-336`) **không** gọi `budget_enable_for_test` |
| Ranh giới continuation của interp frame đã có sẵn | `frame.rs:1031-1034` (`step_frame` remove→`step_frame_owned`→reinsert `IFrame`); `frame.rs:1042-1044` pop `Work` rồi `process_work`?; `proc.rs:26-28, 717-724, 753-760` tự push continuation về `work` **trước** khi trả `Abort::Pending` |
| Ranh giới continuation của VM frame đã có sẵn | `vm.rs:739-752`: charge tại `pc`, sau đó `pc += 1` (D6), rồi mới thi hành `ins`; `vm_frame.rs:119-125` khôi phục args + lùi `pc` khi `CallBuiltin` phát `Pending` |
| **Yield về đuôi `ready` đã được code sẵn** | `frame.rs:875-879` và `vm_frame.rs:255-259`: `Abort::Pending` với `task:None, wait:None` ⇒ `w = None` ⇒ `frame.ready.push_back(current)` — không cần cơ chế hàng đợi mới |
| Snapshot vòng đời đã có hook | `rt.rs:688-705` `diag_snapshot()` = `{live_tasks, frames, waiters, ready, regs, timers}`; `frame.rs:949`, `vm_frame.rs:353` lấy ở cuối chạy; `lat1_frame.rs:217-235` (`c7_reclaim_verified_by_diag_snapshot`) đã assert về baseline |

**Hệ quả**: "no second instrumentation" không có nghĩa là "không thêm call-site" — mà là **không
định nghĩa bộ đếm thứ hai chạy bằng đơn vị khác** (không đếm vòng `while` hay `vm.steps` như cũ).
Ta dùng đúng `FeeKind` + `standard_weight()` + `fee::charge()` / `budget_charge()` của VD-256, chỉ
thêm call-site trong `frame/expr.rs` để đường frame phát **cùng dãy sự kiện** với compiler/direct
path. Parity của dãy đó phải được test chứng minh (§2, invariant I6).

---

## 2. DELIVERABLE 1 — thiết kế quota thử nghiệm (điểm chèn chính xác)

### 2.1 Mô hình counter

```
fee::charge(kind, site)              // MỘT sự kiện, MỘT trọng số w = standard_weight(kind)
  ├─ remaining        -= w           // budget toàn chương trình (VD-256). <=0 ⇒ VS0501-BUDGET trap
  └─ quota_remaining  -= w           // MỚI: hạn ngạch lượt. reset = Q ở đầu mỗi turn.
                                      // <=0 ⇒ set cờ quota_hit, KHÔNG trap, KHÔNG báo chẩn đoán
```

- Hai counter nằm **cùng struct** `Budget` (`frame/fee.rs:13-22`) / cùng `Vm` (`vm.rs:150-160`); cùng
  một lần tính `w`; **không** có `charge_quota()` riêng.
- `quota_hit` là cờ **một chiều trong turn**, được đọc tại ranh giới (§2.2) rồi xoá. Nó là *tín hiệu
  lập lịch*, không phải lỗi: không `Trap`, không `VS05xx`, không làm `exit = 1`.
- Khi instrumentation tắt (`quota_enabled == false`, mặc định): mọi helper là no-op, cờ không bao giờ
  set ⇒ hành vi mặc định byte-for-byte như cũ.
- Reset **per scheduling turn**, không per task, không per chương trình: đặt lại `quota_remaining = Q`
  ngay trước mỗi lần driver gọi `step_frame` / `Th::exec` (điểm R1/R2 bên dưới). Task đơn độc bị cắt
  thành nhiều turn Q sự kiện ⇒ vẫn tiến triển, nhưng có overhead chuyển turn (sweep §3 phải đo nó).

### 2.2 Điểm chèn cụ thể

Ký hiệu: **I** = interp, **V** = VM; **R** = reset, **Y** = yield check, **C** = charge call-site.

#### Interp

| ID | Tệp:dòng hiện tại | Việc cần thêm (PHASE 2) |
|---|---|---|
| I-C1 | `crates/vis-interp/src/frame/fee.rs:13-22` (`struct Budget`), `:74-92` (`charge`) | Thêm `quota_enabled: bool`, `quota_limit: u64`, `quota_remaining: i64`, `quota_hit: bool`, `quota_yields: u64`; trong `charge()` trừ thêm `quota_remaining` bằng **cùng `w`**, set `quota_hit` khi vượt 0 (vẫn charge trọn sự kiện theo VD-252 §3: nguyên tử, không rollback). Thêm test-hook `#[doc(hidden)] quota_enable_for_test(Q)`, `quota_reset_for_test()`, `take_quota_yield_for_test()`. |
| I-R1 | `crates/vis-interp/src/frame.rs:804-805` (trước `step_frame(...)`) | `it.sched...; fee::quota_reset_for_test()` — reset đầu mỗi turn kể cả turn 0. |
| I-Y1 | `crates/vis-interp/src/frame.rs:1037-1045` (`step_frame_owned`, ngay sau `loop {` ở 1038, **trước** `let w = frame.cur().work.pop()` ở 1042) | Nếu `fee::take_quota_yield_for_test()` ⇒ **không pop work** (hoặc push lại `w` nếu đã pop), ghi log `quota:{frame_id}`, trả `Err(Abort::Pending(Pending { task: None, wait: None, span }))`. `IFrame` đã được `step_frame` (1031-1034) cất lại **nguyên vẹn** (`calls/work/vals/loops/restore/env`) — đây là continuation point hợp lệ. |
| I-C2 | `crates/vis-interp/src/frame/expr.rs`: `eval_work` ở `:7` (E_EXPR), `HExpr::Call` `:84` (E_CALL), `HExpr::Spawn` `:131` (E_SPAWN), `HExpr::Await` `:166` (E_AWAIT), `exec_work` ở `:340` (E_STMT), `HStmt::Let` `:342` / `Assign` `:375` (E_ASSIGN), `For` `:457` / `While` `:471` / `Loop` `:479` (E_LOOP), builtin dispatch `classify_callee`/`Callee::Builtin` `:600,646` (E_BUILTIN) | Gọi `it.charge(kind, span)` theo đúng **8 quy tắc VD-252 / bảng映射 vd256 §2**, đối chiếu trực tiếp với `interp.rs:232,244,303,337,352,376,427,482,506,533,628,751,1004,1025`. Charge ở đầu execution của work-item tương ứng; **không** charge trong `ApplyAwait`/`Apply*`/`Bind` (là continuation tiêu thụ giá trị ⇒ resume sẽ chạy lại chúng, charge ở đó ⇒ double-charge). |
| I-config | `crates/vis-interp/src/frame.rs:33-52` (`FrameConfig`) | Thêm `#[doc(hidden)] quota_cost: Option<u64>` (None = tắt). Không đổi CLI/ngôn ngữ. |

**Không cần sửa `rt.rs`** theo thiết kế được chọn: tín hiệu yield dùng lại `Pending { task: None,
wait: None }` — nhánh xử lý đã có ở `frame.rs:875-879`. Nếu A0 muốn log phân biệt bền vững hơn,
dùng cờ `quota_yields` trong `fee.rs` + dòng log do I-Y1 ghi, **không** thêm field vào `Pending`
(để giảm diện chạm vào tệp đang bị stream A/C sửa).

#### VM

| ID | Tệp:dòng hiện tại | Việc cần thêm (PHASE 2) |
|---|---|---|
| V-C1 | `crates/vis-vm/src/vm_frame.rs:73-99` (`budget_charge`) | Chia `charge` thành một hàm trả `(w, quota_hit)` dùng chung; trừ `quota_remaining` bằng cùng `w`; **không** tạo trap cho quota (budget vẫn trap `VS0501-BUDGET` như `:85-94`). |
| V-field | `crates/vis-vm/src/vm.rs:150-160` (`struct Vm`), `:170-181` (`Vm::new`) | Thêm `quota_enabled/quota_limit/quota_remaining/quota_hit/quota_yields`, khởi tạo None/0. |
| V-Y1 | `crates/vis-vm/src/vm.rs:708-...` (`Th::exec`), **cuối thân `loop`** (sau `match ins` ở `:752`, trước vòng lặp kế) | Nếu cờ `quota_hit` ⇒ xoá cờ, tăng `quota_yields`, log `quota:{frame}`, `return Err(Abort::Pending(Pending{task:None,wait:None,span}))`. Đặt **sau** side-effect và **sau** `pc += 1` (`:742`) ⇒ resume ở lệnh kế, lệnh vừa charge **không chạy lại, không charge lại** (D6). Không đặt ngay sau `budget_charge` ở `:741` — lúc đó `ins` chưa thi hành ⇒ mất side-effect. |
| V-R1 | `crates/vis-vm/src/vm_frame.rs:191-193` (trước `th.exec(&mut vm)`) | Reset quota mỗi turn (đối trọng I-R1). |
| V-enable | `crates/vis-vm/src/vm_frame.rs:134-175` (`run_frame_for_test`) | Nhận `FrameConfig.quota_cost` + fee table (`compile_with_fee`), gọi `vm.budget_enable_for_test(fee, budget_limit)` rồi bật quota; không bật khi None. |
| V-driver | `crates/vis-vm/src/vm_frame.rs:194-263` (nhánh `Err(Abort::Pending(p))`) | Nhánh `w = None` ở `:257-259` đã push `current` vào đuôi — chỉ cần ghi log/counter; `Th` đã được cất lại ở `:261`. Không đổi final-join (`:327-332`) hay reactor shutdown (`:334`). |

### 2.3 Bất biến phải check (invariant checks)

| # | Bất biến | Cách check trong P2 |
|---|---|---|
| I1 | Mặc định không đổi | `vis run`/`vis run --vm` + `frame.quota_cost = None` ⇒ `quota_yields = 0`, stdout/diag/log giữ nguyên; test kiểu `default_khong_doi` (vd256) mở rộng thêm. |
| I2 | Budget ≠ quota | Một chương trình: đặt `BUDGET` lớn, `Q` nhỏ ⇒ nhiều `quota:` log, **không** có `VS0501-BUDGET`; ngược lại budget nhỏ ⇒ trap, dù quota còn. Hai short code riêng (`VS0501-BUDGET` vs không code cho quota). |
| I3 | Reset đúng turn | Trong 2 turn liên tiếp, số event mỗi turn ≤ Q+w-1 (w tối đa 4); log có dãy `quota:a`, `quota:b`, `quota:a`, … theo thứ tự ready. |
| I4 | Tail, không trap | Sau quota-yield, `frame.ready.back() == current`; không có diagnostic mới; `exit` chỉ phụ thuộc trap thật. So sánh log `wake:` vs `quota:` để táchI/O-wake và quota-yield. |
| I5 | Không re-charge / không lặp side-effect | Chạy `print("x")` + vòng lặp ngắn với Q cực nhỏ; đếm `E_BUILTIN(print)` bằng `take_trace_for_test()` phải **bằng** chạy Q=∞; stdout in mỗi dòng đúng một lần. VM: `budget_trace_for_test()` so dãy khi Q=∞ và Q nhỏ phải giống hệt. |
| I6 | Cùng một event stream | Với cùng nguồn: `compiler fee.trace()` == `interp direct trace` == `interp frame trace` == `VM frame budget_trace()` (so **dãy**, không phải tổng — `lat256_budget.rs:73-90` là mẫu). Đây là bằng chứng "no second instrumentation". |
| I7 | Suspension an toàn | Quota-yield xảy ra ở work-item boundary / sau `pc += 1`; `IFrame`/`Th` reinsert trước khi đổi `current`; không có `"frame hiện tại"` panic (`frame.rs:1031`, `vm_frame.rs:192`). |
| I8 | Guard không nuốt | Số turn `ceil(events/Q) < 200_000` (`frame.rs:796`, `vm_frame.rs:183`) cho mọi workload sweep — nếu vi phạm, tăng Q hoặc giảm N, **không** tăng guard. |
| I9 | Lifecycle về baseline | Sau mỗi wave: `diag_snapshot()` == baseline (§4), `quota` không làm tăng `waiters`/`regs`/`timers`. |

---

## 3. DELIVERABLE 2 — kế hoạch sweep Q (CHƯA chọn default)

### 3.1 Nguyên tắc

- Q là **số đơn vị chi phí chuẩn hoá** (`standard_weight`: Stmt/Expr/Assign/Loop = 1, Builtin/Await
  = 2, Call/Spawn = 4 — bảng v1 **chưa duyệt**, VD-256 D2). Vì trọng số chưa ratified, mọi số Q
  dưới đây là **đơn vị đo tạm**, không phải khuyến nghị sản phẩm.
- **Không** lấy 10^7, không lấy BUDGET 20,000,000 làm Q (VD-252/256 cấm kế thừa).
- Mỗi điểm đo là **trung vị + min/max + p90** trên các lần lặp có seed cố định; báo cả dữ liệu thô
  vào `/tmp/vd257-d/sweep.csv`. Không chạy ở PHASE 1.

### 3.2 Ma trận

| Trục | Giá trị | Lý do |
|---|---|---|
| Q (cost units) | `{500, 2_000, 10_000, 50_000, 200_000, 1_000_000, None(vô hạn = control)}` | 6 mức + control; vô hạn tái hiện B2 (starvation), làm điểm so sánh. `{500…}` đủ nhỏ để cắt task CPU 2M vòng (≈18M event, vd256 §7) thành < 200k turn. |
| N CPU (vòng lặp) | `{100_000, 500_000}` (tuỳ chọn thêm 2_000_000 nếu thời gian cho phép) | 500k là workload B2 gốc; 100k là điểm crossover; 2M là ca parity đang MỞ — **không** dùng để tuyên bố parity. |
| Engine | interp frame + VM frame | parity hành vi, không chỉ một engine. |
| Repeats | **7 lần/ô**, bỏ lần 1 (warmup), lấy 6; seed = `0x5644_3235_37 + Q*131 + N` (hàm tầm thường, không RNG ngoài) | "finite, seeded analysis"; đủ để thấy xu hướng, không ảo thống kê. |
| Tổng (P2) | 7×2×2×7 = **196 lần chạy frame** (mỗi lần < vài trăm ms ở debug) | Chỉ chạy khi A0 cho phép, không trong lúc agent khác build. |

### 3.3 Workload (chạy qua `run_frame_for_test`, không qua `vis run`)

```
use std.time
use std.net
async fn cpu(n: int) -> ():                 # spawn TRƯỚC
    var i = 0
    while i < n: i = i + 1
    let e = time.monotonic_ms()
    print("CPU_DONE {e}")
async fn io() -> ():                        # spawn SAU cpu — mô phỏng B2
    let s = time.monotonic_ms()
    print("IO_START {s}")
    match net.read("C1", 16, 2000):         # Lát 2 reactor giả: register ⇒ Pending ⇒ yield
        Ok(v): print("IO_OK")
        Err(e): print("IO_ERR")
    let e = time.monotonic_ms()
    print("IO_DONE {e}")
async fn short():                           # task ngắn, spawn CUỐI — không được đói
    let s = time.monotonic_ms()
    print("SHORT_START {s}")
    let e = time.monotonic_ms()
    print("SHORT_DONE {e}")
let c = spawn cpu(N)
let d = spawn io()
let q = spawn short()
```

- I/O được mô phỏng **tất định** bằng `FrameConfig.reactors` (`ReactorEvent { seq, key, gen,
  outcome: Ready(...), deadline_ms: Some(200) }`) — `net.read` đăng ký rồi trả `Abort::Pending`,
  scheduler chạy `short`, rồi fire event theo `seq`. Không socket thật, không sleep-poll (đúng thiết
  kế Lát 2 `rt.rs:168-175`).
- Một biến thể B1 dùng `timer_real_wait: true` để đo độ trễ 200 ms thật; **không** trộn vào ma trận
  fairness chính (real wait làm nhiễu số liệu).

### 3.4 Tách khởi động khỏi workload & đo latency monotonic

1. **Compile once**: parse/lower/tyck/`compile_with_fee` một lần, giữ `Hir`/`Symbols`/`Module`/`FeeTable`
   dùng lại cho cả 6 lần lặp. Không tính parse/compile vào latency.
2. **Warm-up 1 lần** với chương trình rỗng + cùng config ⇒ loại chi phí `frame_enable_for_test`,
   `frame_reactor_load_for_test`, khởi tạo `Scheduler`.
3. **Đồng hồ**: chỉ dùng `time.monotonic_ms()` **bên trong chương trình** (`stdlib.rs:437-448`);
   không dùng `/usr/bin/time` wall (VD-256 §3 ghi rõ wall gồm startup 0.05–0.2 s).
4. **Các metric ghi mỗi lần** (ghi nguyên văn vào CSV):
   - `cpu_span   = CPU_DONE - t_spawn` (thời gian task CPU chiếm runner)
   - `io_start   = IO_START  - t_spawn` (khi I/O bắt đầu — **số chính của B2**)
   - `short_lat  = SHORT_DONE - SHORT_START` (latency task ngắn; phải bị chặn bởi ~Q × w, không bị
     kéo dài bởi N CPU)
   - `starve_gap = CPU_DONE - IO_START` (B2 hiện tại ≈ 0 ms, tức I/O chỉ bắt đầu **sau** CPU xong;
     sau quota mong đợi **âm** hoặc nhỏ hơn nhiều)
   - `quota_yields` (từ `take_quota_yield_for_test()`), `events_total`, `diag_snapshot()` cuối.
5. **Tính chất monotonic cần kiểm chứng, không giả định**: khi Q giảm từ ∞ ⇒ `short_lat` và
   `io_start` phải **giảm không tăng** (`monotonic non-increasing`); khi Q quá nhỏ (< ~2×w) overhead
   chuyển turn có thể làm **tăng** lại — ghi rõ điểm gãy này thay vì ép một default. Dùng median của
   6 lần; nếu median tăng ở Q nhỏ, báo "overhead regime", không tự loại bỏ.
6. **Tiêu chí đọc kết quả (không phải ngưỡng sản phẩm)**: Q được coi là "có tín hiệu fairness" nếu
   `short_lat(Q) < 0.5 × short_lat(None)` **và** `starve_gap(Q) < starve_gap(None)` trên cả hai engine.
   A0 sẽ chọn giá trị từ đồ thị; stream D **không** chọn.

---

## 4. DELIVERABLE 3 — đo vòng đời đa wave (`diag_snapshot()`)

### 4.1 Định nghĩa baseline

`baseline = diag_snapshot()` ngay sau `frame_enable_for_test` + `frame_reactor_load_for_test` +
dựng frame `main`, **trước** `step_frame`/`Th::exec` đầu tiên. Với chương trình sạch (không giữ
handle, không kênh để lâu): `baseline = { live_tasks: 0..k, frames: 1, waiters: 0, ready: 0,
regs: 0, timers: 0 }`. Sau khi chạy xong (`frame_drain_events` + `frame_final_join` +
`frame_reactor_shutdown`, `frame.rs:942-949` / `vm_frame.rs:326-335`) phải về
`Z = { live_tasks: 0, frames: 0, waiters: 0, ready: 0, regs: 0, timers: 0 }` — đúng ca
`c7_reclaim_verified_by_diag_snapshot` (`lat1_frame.rs:217-235`).

### 4.2 Cách lấy nhiều mẫu mà không sửa nguồn

Dùng mẫu **driver tay** đã có trong `lat3_real.rs:610-697`: tạo `Scheduler`, gọi
`frame_enable_for_test`, `frame_register`, `frame_next`, `frame_reactor_fire`, `frame_reactor_shutdown`,
và gọi `s.diag_snapshot()` **sau mỗi wave**. Không cần hook mới, không đụng `frame.rs`. Với quota,
mỗi wave gồm: spawn 3 task (CPU ngắn + I/O reactor + short) → chạy tới quiescence → drain →
final-join → shutdown → sample. Lặp **5 wave** với cùng seed, lấy 5 snapshot.

### 4.3 Phân biệt giữ lại có ý đồ vs tăng trưởng bất thường

| Loại | Dấu hiệu | Xử lý |
|---|---|---|
| **Giữ lại có ý đồ** | `Value::Task` vẫn còn binding trong `main`; `Scheduler.kenh` giữ kênh; nguồn giả `external`; reg reactor có handle còn chủ; `Rc::strong_count(t) > 1` ⇒ `frame_reclaim` (`rt.rs:651-686`) bỏ qua. | Liệt kê **tập retention cố định** trong fixture, giữ nguyên qua mọi wave; baseline của wave k = baseline + tập này. Không tính là leak. |
| **Tăng bất thường** | Bất kỳ counter nào trong `{live_tasks, frames, waiters, ready, regs, timers}` tăng đơn điệu qua wave (slope > 0); `ready > 0` sau quiescence; `regs/timers > 0` sau `frame_reactor_shutdown`; snapshot wave k ≠ baseline dù retention set không đổi; số dòng `reclaim:` giảm dần. | Fail test, báo kênh cụ thể (task/frame/waiter/reg), không "đợi GC" (repo **không có GC** — `rt.rs:10`). |

Ghi log `snapshot[0..5]` + `frame.log` (`wake:`, `quota:`, `reclaim:`) vào `/tmp/vd257-d/lifecycle.csv`.
Kiểm tra thêm: bật quota (Q nhỏ) vs tắt (Q=∞) ⇒ **cùng** chuỗi snapshot; nếu quota làm `waiters`/
`regs` tăng, đó là bug yield (frame bị register nhầm), không phải "fairness".

---

## 5. DELIVERABLE 4 — danh sách tệp PHASE 2 & trình tự bàn giao

### 5.1 Tệp sẽ sửa (PHASE 2 — hiện **KHÔNG** đụng vào)

| # | Tệp | Việc | Ownership |
|---|---|---|---|
| 1 | `crates/vis-interp/src/frame/fee.rs` | counter quota + helpers trong `Budget`/`charge` (I-C1) | **A/C** (`frame/**`) |
| 2 | `crates/vis-interp/src/frame.rs` | `FrameConfig.quota_cost`, reset (I-R1), yield boundary (I-Y1), log/telemetry | **A/C** (`frame.rs`) |
| 3 | `crates/vis-interp/src/frame/expr.rs` | 14 call-site `charge()`镜像 `interp.rs` (I-C2) — prerequisite cho "same events" | **A/C** (`frame/**`) |
| 4 | `crates/vis-vm/src/vm.rs` | field quota + yield check cuối `Th::exec` (V-field, V-Y1) | cần xác nhận với A/C (Vm core) |
| 5 | `crates/vis-vm/src/vm_frame.rs` | `budget_charge` chia sẻ counter, enable/reset/driver log (V-C1, V-R1, V-enable, V-driver) | **C** |
| 6 | `crates/vis-cli/tests/lat257_fairness.rs` (**mới**) | sweep §3 + lifecycle §4 + parity I6 | Stream D (P2) |

**Không sửa**: `rt.rs` (dùng lại `Pending{task:None,wait:None}`), `stdlib.rs`, `net.rs`,
`interp.rs`, `builtin.rs`, `vis-bytecode/**`, `Cargo.toml/.lock`, `DECISIONS.md`, `QUEUE.md`,
fixture/golden, cache/VERSION. Không thêm opcode/API ngôn ngữ/CLI flag.

### 5.2 Trình tự đề xuất cho leader

1. A0 merge streams **A** và **C** trước (chúng đang sửa `frame/**`, `rt.rs`, `vm_frame.rs`, `stdlib.rs`).
2. Rebase nhánh D lên HEAD sau merge; resolve chỉ các vùng chèn §2.2 (không đụng logic mới của A/C).
3. Chạy **targeted** trước: `cargo test -p vis-cli --test lat1_frame --test lat256_budget --test lat257_fairness`
   + parity I6. Chỉ chạy `./vis-dev gate --fast` khi A0 bật (VD-257 §5: không full gate lúc agent khác build).
4. Chạy sweep §3 với resource discipline; gửi **số liệu + đồ thị** cho A0; A0 mới quyết Q (hoặc quyết
   không đưa quota vào sản phẩm). D không tự ấn default và không tự đổi tiêu chí B2.

---

## 6. Trung thực — giới hạn của đề xuất này

- B2 **chưa được fix**: quota chỉ là test-hook trên **đường frame**, không chạm
  `await_task` run-to-completion (`rt.rs:1413`) của `vis run`/`vis run --vm`. Muốn sửa B2 thật là quyết
  định phạm vi sản phẩm của A0 (Giống §5 nhóm 2/3 của báo cáo readiness).
- **Chưa có số liệu Q nào được đo ở PHASE 1** (đúng §5 VD-257). Mọi con số §3 là *kế hoạch thử*, không
  phải kết quả. Không benchmark, không build, không gate.
- Trọng số VD-256 là bảng v1 chưa duyệt ⇒ Q tính bằng đơn vị tạm. Nếu A0 đổi trọng số, phải chạy lại
  sweep.
- Rủi ro guard 200k (`frame.rs:796`, `vm_frame.rs:183`) khi Q nhỏ + N lớn — đã đưa vào I8; không tăng
  guard để "cho chạy".
- Quota-yield dùng chung `Pending{task:None,wait:None}` nên **khác I/O-wake chỉ qua log** `quota:`
  và counter; nếu A0 muốn tách hẳn về type, đó là thay đổi `Abort`/`Pending` trong `rt.rs` (đụng
  ownership A/C) — để A0 quyết.
- `frame_reclaim` chỉ thu hồi khi `Rc::strong_count == 1` (`rt.rs:658`) ⇒ baseline về 0 chỉ đúng khi
  fixture thả handle; mọi retention phải được khai báo (§4.3).

---

## 7. Trạng thái commit

- **Không commit nguồn** (PHASE 1). Tệp duy nhất được ghi trong repo: `agent/JOURNAL/vd257-d-fairness-proposal.md`.
- Scratch/log (nếu có) chỉ nằm ở `/tmp/vd257-d/`, không đụng shared repo hay temp của stream khác.
- Tôi để tệp này **đã commit** trên `work/vd257-d-fairness` với message `[VD-257][D] fairness analysis`
  (chỉ journal, không file nguồn). Nếu leader muốn rebase, đây là commit duy nhất của stream D.
