<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-257 · Stream B — báo cáo TƯƠNG THÍCH + BẰNG CHỨNG NGÂN SÁCH / FUEL

**Branch**: `work/vd257-b-budget` (base `fc297ba`). **Trạng thái**: mã + test xong, commit trên branch,
KHÔNG merge/push. **Đường MUST-NOT-BE-DEFAULT**: mọi charge chỉ chạy qua test-hook VD-256; bảng
trọng số v1 vẫn **thử nghiệm**; không bật budget/fuel mới làm chính sách mặc định.

**Validation (resource discipline)**: chỉ test đích (`cargo test -p <crate> --test <file>`),
`cargo fmt --all -- --check` sạch; **không** full gate/suite/benchmark, **không** `cargo clean`.
Measurement chạy riêng `--ignored`, debug, finite; ghi duration ở §5.

---

## 0. Tóm tắt

1. **PART 1** — bốn trục phiên bản được tách rõ trong code. Hàng rào tương thích đặt ở **module
   format version** (trục 1): artifact có trailer `VFEE` mang version **3**; runtime cũ (so `2`)
   từ chối ngay ở `decode`, kể cả đường **direct load** không qua cache. Cửa nạp có budget **chỉ**
   nhận `3` + trailer hợp lệ ⇒ artifact cũ (`2`) bị từ chối rõ, không bị coi là "đã charge".
   Cache key đưa **cả bốn** trục; hash ghim cập nhật. `3` được chọn sau khi check `git log`:
   `VERSION` lõi chỉ từng là **1 → 2**; `3` từng là *sentinel test*, **không phải bản phát hành**.
2. **PART 2** — đo VM instruction counts (`RunOutput.steps`). Vòng 2M có chi phí ngữ nghĩa
   ~18M (**dưới** budget đề xuất 40M) nhưng VM chạm fuel **10^7 lệnh** ở 10,000,001 ⇒ **fuel chặn
   trước budget**. Đây là root cause thật (không phải tên `MAX_STEPS`/`SAFETY_FUEL`). Tỷ lệ tối đa
   `steps/cost ≈ 1.0` (vòng lặp) ⇒ **fuel phải ≥ budget** nếu muốn budget là giới hạn đầu tiên.
3. **Bổ sung stream A** — `await` trong thân closure trên VM frame path: nguyên nhân KHÔNG phải
   codegen (đúng như nghi ngờ) mà là `vm.rs::goi_ham_bytecode` tạo `Th` **lồng nhau** không
   suspend/resume được. Đã sửa trong `vm.rs` (thuộc ownership B) — giờ cả hai engine in `42\n`.

---

## 1. Bốn trục phiên bản — định nghĩa và vị trí mã

| Trục | Tên / hằng | Giá trị | Vai trò | Vị trí |
|---|---|---|---|---|
| 1. Module format (cơ sở) | `vis_bytecode::VERSION` | **2** | VBC1 thường; artifact cache/default. Không đổi ⇒ ST-246 giữ nguyên. | `format.rs:44` |
| 1b. Module format (instrumented) | `vis_bytecode::INSTRUMENTED_VERSION` | **3** | Artifact có trailer VFEE, **bắt buộc** ngữ nghĩa budget mới. `3` chưa từng phát hành. | `format.rs:60`, export `lib.rs:30` |
| 2. Event-table | `fee::VERSION` = `fee::EVENT_TABLE_VERSION` | **4** | Cấu trúc `pc/kind/site` trong trailer `VFEE`. Độc lập với trục 1. | `fee_shared.rs:8,14` |
| 3. Weight-table | `fee::WEIGHT_TABLE_VERSION` | **1** | Bảng trọng số v1 (thử nghiệm). Không đọc trọng số từ artifact (D5). | `fee_shared.rs:20` |
| 4. Cache key | `PHIEN_BAN` + các số trên + bảng thẻ + mode + nguồn | — | Tooling cũ tự vô hiệu. | `cache.rs:26-35, 90-101` |

**Check `git log`** (`git log --all -p -- crates/vis-bytecode/src/format.rs | grep 'pub const VERSION'`):
chỉ có `VERSION = 1` (ST-19) rồi `VERSION = 2` (ST-246). `3` xuất hiện **duy nhất** làm sentinel
trong `tests/decode_neg.rs` + `tests/verify.rs` — không phải artifact đã phát hành. Vì vậy `3` là
giá trị **thực sự chưa dùng** cho module format, và sentinel test được đổi sang **99** (một số
không phải version hợp lệ nào). Không chọn số "để khỏi sửa test" — ngược lại, đã **sửa** test.

### 1.1 Thay đổi mã chính xác

| Tệp | Thay đổi |
|---|---|
| `format.rs` | thêm `INSTRUMENTED_VERSION = 3` + doc 4 trục. |
| `lib.rs` | export `INSTRUMENTED_VERSION`. |
| `decode.rs` | tách `decode_for_version(bytes, expected)` (`pub(crate)`); `decode` gọi với `VERSION=2`. Fee-load gọi với `3`. |
| `verify.rs` | tách `verify_for_version(m, expected)`; `verify` gọi với `2`. Phần kiểm còn lại **dùng chung**, không copy. |
| `fee_shared.rs` | thêm alias `EVENT_TABLE_VERSION`, docs phân biệt 4 trục; giữ `VERSION=4`, `WEIGHT_TABLE_VERSION=1`. |
| `fee.rs` | `encode_with_fee` đóng dấu **module version 3** (clone module, đổi field); `load_verified_with_fee` dùng `decode_for_version(3)` + `verify_for_version(3)` + bắt buộc trailer; `decode_fee` kiểm cả event-table (4) lẫn weight-table (1); docs cập nhật. |
| `cache.rs` | `PHIEN_BAN` += `+vd257-b-four-version-axes-2026-11-20`; khoá thêm `WEIGHT_TABLE_VERSION` + `INSTRUMENTED_VERSION` (cùng `VERSION` + `fee::VERSION` có sẵn). Hash ghim: `cca587…e0f20` → **`98a0b983…550e8`**; thêm `assert_ne!` với hash cũ. |
| Test sentinel | `decode_neg.rs`, `verify.rs`: `3` → `99` (kèm assert `INSTRUMENTED_VERSION == 3`); `fee_side_table.rs`: so sánh core bytes bỏ qua 2 byte version, assert version 3. |

**Quyết định có chủ ý**: giữ `VERSION = 2` làm **base module format** và chỉ đóng dấu `3` cho
artifact có trailer. Lý do: (a) `st246_cache_compat.rs:109,123,146` (ngoài ownership B) assert `2`
— giữ nguyên giúp suite của leader **không đỏ**; (b) đây là đúng ngữ nghĩa "module format có
instrumentation" là một **giá trị mới** của cùng trục 1, không phải đổi giá trị base. Nếu A0 muốn
nhảy **toàn bộ** `2 → 3`, đó là patch riêng cần sửa `st246_cache_compat.rs` (stream/leader khác).

---

## 2. PART 1 — bằng chứng tương thích (test `lat257_b_compat.rs`, 9/9 pass)

### 2.1 Old-runtime REJECT artifact mới (không lờ trailer)
`runtime_cu_tu_choi_artifact_moi_khong_lo_trailer` mô phỏng đúng runtime cũ (`decode`/`verify` so `2`,
không biết `VFEE`):
- artifact = `encode_with_fee(...)` → field version = `3`;
- `decode_runtime_cu` → `Err(3)`; `verify_runtime_cu` → `Err(3)`;
- sanity: runtime cũ vẫn nhận artifact `2` ⇒ đây là phép so sánh thật, không nhánh chết.

### 2.2 New-runtime xử lý artifact cũ với chính sách rõ
`runtime_moi_tu_choi_artifact_cu_voi_chinh_sach_ro_rang`:
- artifact `2`, không trailer → `decode_fee = Err(Absent)`; `load_verified_with_fee = Err(Decode(BadVersion(2)))`;
  **không** coi là "đã instrumented / đã charge", **không** rơi vào đường mặc định im lặng.
- artifact `3` nhưng **thiếu trailer** → `Err(Verify(FeeDecode(...)))` (artifact hỏng, không phải zero-work).

### 2.3 Cache + DIRECT LOAD đều được bảo vệ
`duong_direct_load_cung_bi_bao_ve_khong_chi_cache`:
- `decode(v3)` → `Err(BadVersion(3))`; `load_verified(v3)` → `Err(Decode(BadVersion(3)))`;
  `verify(Module{version:3})` → `Err(BadVersion(3))`. Nghĩa là **mọi** cửa nạp mặc định (dùng cho cả
  cache lẫn `vis run file.vbc`) từ chối artifact đòi budget — không cần biết cache key.
- `cache_key_chua_bon_truc_qua_nguon`: source-inspection khẳng định khoá chứa cả 4 trục + nhãn VD-257.
- `khoa_cache_ghim` (bin unit): hash mới khác hash cũ; `st246_cache_compat` (2/2) vẫn pass.

### 2.4 Wrong fee table / crafted bytecode / loop evading charges
| Test | Kết quả |
|---|---|
| `sai_event_table_hoac_weight_table_deu_bi_tu_choi` | sửa event 4→9 ⇒ `BadVersion(9)`; weight 1→9 ⇒ `BadWeightVersion(9)`; cả hai bị `load_verified_with_fee` từ chối. |
| `trailer_kind_la_bi_tu_choi` | tag sự kiện = 200 ⇒ `BadKind(200)`, không vào VM. |
| `fee_table_sai_cau_truc_bi_chan_khong_vao_vm` | sai func-count / bad pc / bad site / bảng rỗng ⇒ đều `Err`, `load_verified_with_fee` từ chối. |
| `loop_evading_charges_bi_verifier_chan` | CFG self-loop không điểm phí ⇒ `LoopGuardUncharged`/`CycleUncharged`; cạnh lùi vào header trống ⇒ `LoopGuardUncharged`. Không có artifact nào lọt vào VM. |

---

## 3. PART 2 — đo VM instruction counts & quan hệ fuel/budget

### 3.1 Phương pháp
`lat257_b_vm_steps.rs::measure_vm_instruction_counts` (chạy `--ignored`): compile source →
`Vm::run` → đọc `RunOutput.steps` (= số lệnh VM đã thực thi). Không bật budget; `SAFETY_FUEL = 10^7`
là hàng rào cuối. Chi phí ngữ nghĩa lấy từ hook đo VD-256 (`lat256_budget::measure_workloads`,
chạy lại sau các sửa codegen; số không đổi so với report VD-256). Debug, thread stack 64 MB.

### 3.2 Số đo (debug, seed = không dùng RNG, duration thực tế)

| Workload | VM steps | cost v1 (events×w) | steps/cost | wall (debug) |
|---|---:|---:|---:|---:|
| loop 100k | **900,009** | ~900,008 | **~1.000** | 0.125 s |
| recursion 200 | **2,213** | 2,815 | 0.786 | 0.8 ms |
| tasks 1000 | **14,012** | 17,008 | 0.824 | 10.6 ms |
| expr-chain 100k | **1,900,012** | 2,000,008 | 0.950 | 0.327 s |
| loop 2M | **10,000,001** (chạm fuel) | 18,000,008 | — | 1.35 s |

### 3.3 Quan hệ fuel ↔ budget — root cause
- Vòng 2M: cost ~18M **< budget đề xuất 40M**, nhưng VM chạm `SAFETY_FUEL` ở 10,000,001 lệnh
  (`VS0501-FUEL`, `exit=1`) — **fuel chặn chương trình compiler sinh trước khi budget cạn**.
  `lat257_b_budget::fuel_chay_truoc_khi_budget_khi_budget_lon_hon_fuel` (vòng 1.2M, budget hook
  20M) khẳng định chẩn đoán là `VS0501-FUEL`, **không** phải `VS0501-BUDGET`, `steps = FUEL+1`.
- Với mọi workload đo, `steps ≤ cost` ⇒ hệ số nguy hiểm nhất là **~1.0** (loop). Do đó:
  **`F ≥ B`** là điều kiện cần để budget (chứ không phải fuel) là giới hạn đầu tiên trong miền hỗ trợ.
- Đổi tên `MAX_STEPS` → `SAFETY_FUEL` (VD-256) **không** sửa được việc này; đây là lỗi **hiệu chỉnh
  hai ngưỡng**, không phải tên.

### 3.4 Ordered event trace — hoàn thiện trong miền hỗ trợ
`lat257_b_budget.rs` (5/5 pass):
- `ordered_event_trace_day_du_compiler_interp_vm`: chương trình **không vòng/gọi hàm** (`let/var/
  print`) — static fee table == dynamic trace **từng sự kiện**, assert **dãy** (không phải tổng) trên
  cả 3: compiler / interp / VM.
- `ordered_event_trace_trong_mien_ho_tro`: `fn add` + `while` + `print` — so **dãy động** interp↔VM
  (byte-for-byte) + mọi sự kiện động có trong bảng tĩnh + đủ 6 kind. Không assert static==dynamic
  (vô nghĩa khi có vòng/gọi: bảng tĩnh mỗi hàm 1 lần, trace động lặp lại).
- `ordered_event_trace_spawn_await`: `spawn`/`await` — engine↔engine + coverage; thân task chạy ở
  final join nên thứ tự tĩnh/động khác nhau, ghi rõ.
- **Parity fix trong codegen** (đều trong ownership B, không đổi opcode/ngữ nghĩa):
  - `codegen/than.rs` + `codegen/gia_tri.rs`: `HStmt::Expr` ở thân hàm / nhánh có giá trị bị sót
    `E_STMT` (VD-256 vá top-level nhưng sót 2 đường này) ⇒ thêm charge. Khớp interp.
  - `codegen.rs::cau`: với `while`/`loop`, PC của `E_STMT` trùng header vòng ⇒ VM charge lại mỗi
    lần nhảy lùi (interp chỉ 1 lần). Đẩy header qua cặp `PushUnit; Pop` net-zero ⇒ E_STMT nằm trước
    vòng, chỉ E_LOOP lặp. Đã kiểm bằng test dương.
- `ngan_sach_nho_ta_ranh_gioi_tac_dung_phu`: budget = `cost-1` ⇒ **không in** + `VS0501-BUDGET`
  (cả interp lẫn VM); budget = `cost` ⇒ in `42\n`, sạch. Ranh giới sát tác dụng phụ.

### 3.5 Miền được hỗ trợ (SUPPORTED DOMAIN)
- Bao gồm: 4 workload đo ở §3.2; chương trình flat (`let/var/print`); hàm + vòng + gọi hàm +
  builtin; `spawn`/`await` nguồn; closure gọi qua `CallValue`; `await` trong thân closure đã đóng
  (§4). Mọi charge đi qua test-hook VD-256.
- **Không** bao gồm / chưa chứng minh: `match`/`with`/`try/?`/field-index assign parity đầy đủ
  (stream A phủ trên frame path, nhưng chưa đo steps/cost riêng); kênh blocking (bị từ chối rõ
  `VS0505`); bytecode chế tạo (verifier từ chối); đo **release** (chỉ có debug). Không tuyên bố
  parity full-language.

### 3.6 Đề xuất số cuối cùng — **CHƯA DUYỆT, KHÔNG BẬT MẶC ĐỊNH**
| Thông số | Đề xuất | Căn cứ |
|---|---|---|
| Weights v1 | `Stmt/Expr/Assign/Loop=1, Builtin/Await=2, Call/Spawn=4` | Giữ nguyên từ VD-256; **experimental**, A0 chưa chốt. |
| `BUDGET` | **40,000,000** | = ~2.2× chi phí lớn nhất đo được (18M), cho phép vòng ~4M lần có headroom. **20M bị từ chối** (chỉ ~11% trên vòng 2M). |
| `SAFETY_FUEL` | **50,000,000** lệnh | `max(steps/cost) ≈ 1.0` ⇒ cần `F ≥ B = 40M`; +25% margin = 50M. Reset mỗi `run`, chẩn đoán riêng `VS0501-FUEL`. **Không** phải budget. |
| `Q` (hạn ngạch lượt chạy/CPU) | **chưa có số** | Nợ riêng, không tự ấn. |

Cả ba số trên **chỉ là đề xuất gửi A0**. Không có CLI flag, không đổi API công cho budget, không
đổi runtime mặc định. `MAX_STEPS` vẫn là bí danh của `SAFETY_FUEL` (giữ 10^7 trong code hiện tại —
đề xuất 50M ở trên **chưa áp dụng**).

---

## 4. Bổ sung theo yêu cầu leader — `await` trong thân closure (stream A §4.1)

**Đánh giá**: stream A chẩn đoán "lỗi codegen `vis-bytecode`" là **không chính xác**. Bằng chứng:
`vis_bytecode::verify(&m) = Ok(())`, `Vm::run` mặc định chạy bytecode đó `exit=0`; chỉ **frame path**
lỗi. Reproduce đúng config của stream A (`e1`), log frame: `enable → external → wait → fire → wake
→ reclaim → renable → rshutdown` rồi `bytecode hỏng: ngăn xếp rỗng`.

**Root cause** (trong `vm.rs`, ownership B): `Instr::CallValue` gọi `builtin::Host::call` →
`goi_ham_bytecode` (vm.rs:558) tạo một `Th` **lồng nhau** (`Th::new(...); th.exec(self)`). `Th` lồng
này là biến local — khi thân closure `$await` trả `Abort::Pending`, `call_builtin_frame` khôi phục
pc/args **trong `Th` lồng**, rồi `Pending` lan ra driver frame. Driver lưu `Th` **ngoài**, `Th` lồng
bị mất. Resume chạy lại `CallValue` trên ngăn xếp ngoài đã pop closure + args ⇒ `pop_n` underflow.

**Sửa** (`vm.rs:1166` `Instr::CallValue`): nếu callee là `Value::Vm(vf)`, mở khung **trong chính
`Th` hiện tại** — copy logic của `Instr::Call` (save `st` vào `st_cu`, push `Frame{fi, locals,
captured, writeback}` kèm mirror prologue `LoadCaptured/Assign`, `closure_cells`). `Value::Builtin`
giữ nguyên `Host::call`. Không đổi opcode/ngôn ngữ/runtime mặc định; **không** đụng file stream A
(`vm_frame.rs`, `frame/**`, `rt.rs`).

**Test dương** `lat257_b_closure_await.rs` (2/2 pass):
- `await_trong_than_closure_chay_duoc_tren_ca_hai_engine`: compile → `verify = Ok`; interp **và** VM
  frame path đều `exit=0`, `stdout = "42\n"`, không diagnostic (trước: VM `exit=1`, stdout rỗng).
- `goi_closure_khong_await_van_dung_va_giu_writeback`: regression closure bắt biến `var` —
  `11\n12\n` khớp interp/VM, chứng minh writeback giữa các lần gọi không hỏng.

**Blocker còn lại (không sửa ở stream B)**: biến thể async lồng nhau (`fn() -> await src()` với
`src` lại `await`) vẫn làm interp lặp scheduler (`guard > 200_000`) — stream A ghi nhận; nguyên nhân
nằm ở scheduler/l lowering, cần stream/runtime quyết. Không fallback, không che.

---

## 5. Trung thực về gaps

1. **Chỉ đo debug**, chưa đo release (A0 yêu cầu measurement ngắn/finite — đã làm). Số 50M cần
   xác nhận lại ở release trước khi chốt.
2. Tỷ lệ `steps/cost ≈ 1.0` lấy từ **4 workload**; construct chưa đo (`match`, `with`, `try/?`,
   method trên giá trị, field/index) có thể có tỷ lệ cao hơn ⇒ `F ≥ B` là **giả thiết cần đo thêm**,
   không phải định luật. Nếu có construct có `steps > cost`, phải tăng `F` tương ứng.
3. Budget **không phải default**; hook `enable_for_test`/`budget_enable_for_test` chỉ test gọi.
   Không có CLI flag/public API budget mới. `SAFETY_FUEL` vẫn **10^7** trong code — số 50M ở §3.6
   **chưa được áp dụng**.
4. Step-cap parity 2M vòng giữ **OPEN** theo VD-256 D9; report này **không** tuyên bố closed. Finding
   "fuel chặn trước budget" đã có root cause + đề xuất, nhưng **A0 chưa duyệt** ⇒ chưa đóng.
5. Không có số `Q`. Không rebuild HIR từ bytecode (đúng D4); verifier chỉ suy từ opcode/CFG.
6. Parity source: dãy sự kiện đã phủ flat + hàm/vòng + spawn/await + closure; **chưa** phủ full
   `match`/`with`/`try/?` dưới budget hook (stream A phủ frame path, chưa đo chi phí). Không nhận
   bảng phụ làm bằng chứng instrumentation nguồn đầy đủ.
7. Cache key giờ phủ 4 trục nhưng **cache path vẫn dùng artifact version 2** (budget không default).
   Bảo vệ direct-load đến từ module version 3 của artifact có trailer — không chỉ từ cache key.

---

## 6. Khẳng định "mặc định không đổi" & phạm vi file

- Đường mặc định (`vis run`, `--vm`, API `vis_interp::run`, `vis_vm::run`, `encode`, `decode`,
  `verify`, `load_verified`, `compile`, cache) **không charge**, không có ngưỡng mới, không đọc
  trailer phí. Mọi hook là `*_for_test` / `compile_with_fee` / `encode_with_fee` /
  `load_verified_with_fee`.
- Ngoại lệ duy nhất: artifact **có trailer** mang module version 3 + cache key đổi (đúng PART 1),
  và `Instr::CallValue` trên `Value::Vm` giờ push khung thay vì nested `Th` (§4) — **không** đổi
  ngữ nghĩa chương trình thường (đã regression test).
- File sửa (đều trong ownership B): `crates/vis-bytecode/**`, `crates/vis-cli/src/cache.rs`,
  `crates/vis-vm/src/vm.rs`, và 4 test mới `lat257_b_*`. **Không** sửa `rt.rs`, `frame/**`,
  `stdlib.rs`, `builtin.rs`, `vm_frame.rs`, `net.rs`, manifest/lock, `DECISIONS/QUEUE`, test/journal
  của stream khác, fixture/golden. Đã chạy `st246_cache_compat` (ngoài ownership) ở chế độ **đọc**
  — vẫn 2/2 pass.

## 7. Test evidence (targeted)

| Lệnh | Kết quả |
|---|---|
| `cargo test -p vis-bytecode --test lat257_b_compat` | 9 passed |
| `cargo test -p vis-cli --test lat257_b_budget` | 5 passed |
| `cargo test -p vis-cli --test lat257_b_closure_await` | 2 passed |
| `cargo test -p vis-bytecode --test fee_side_table` | 9 passed |
| `cargo test -p vis-cli --test lat256_budget` | 8 passed / 1 ignored |
| `cargo test -p vis-cli --bin vis cache::tests::` | 3 passed |
| `cargo test -p vis-cli --test st246_cache_compat` (đọc) | 2 passed |
| `cargo test -p vis-cli --test lat257_b_vm_steps -- --ignored --nocapture` | 1 passed (đo §3.2) |
| `cargo fmt --all -- --check` | sạch |
