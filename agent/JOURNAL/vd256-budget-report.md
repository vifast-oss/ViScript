<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-256 · ST-276 + ST-278 — báo cáo hiện thực ngân sách + sửa final-join

**Trạng thái**: không commit. Đường **MUST-NOT-BE-DEFAULT** — mọi charge chỉ chạy khi test-hook bật.
**Phạm vi**: genuinely **subset** có instrumentation thật, test thật; các lỗ hổng được liệt kê rõ ở §8.
Không đọc code/tài liệu ngoài phạm vi để chốt số — mọi số ở §7 là **đề xuất chờ A0**.

---

## 0. Tóm tắt

- Bảng phụ **S** theo PC: `vis-bytecode/src/fee.rs`, mã hoá `kind/site` vào trailer `VFEE` phía sau
  byte VBC1 (`decode` lõi bỏ qua trailer); **nhiều sự kiện cùng PC giữ thứ tự**.
- **Một định nghĩa sự kiện** dùng chung compiler/interp: `vis-bytecode/src/fee_shared.rs`, được
  `include!` ở `crates/vis-interp/src/frame/fee.rs` — **không** đổi `Cargo.toml/Cargo.lock`.
- Verifier mới `verify_fee`: cấu trúc (kind/PC/site/độ dài) + CFG (chu trình, guard vòng, đệ quy)
  + **từ chối bảng rỗng**; không tái dựng HIR.
- VM có **SAFETY_FUEL** riêng (`VS0501-FUEL`, 10^7, reset mỗi `run`) — khác chẩn đoán với ngân sách
  (`VS0501-BUDGET`). Ngân sách **không** bật mặc định; `MAX_STEPS` giữ làm bí danh API.
- **ST-278**: lỗi task cache ở final join không còn bị nuốt; `VS0506` + mã gốc surfaced ở **cả hai**
  engine; không đổi chính sách hủy task còn lại.
- Gate: `./vis-dev check: PASS`; `cargo test -p vis-interp -p vis-cli -p vis-bytecode` =
  **1151 passed / 0 failed / 1 ignored** (ignored = workload đo, chạy riêng).

---

## 1. D1–D9 → bằng chứng

| Quyết định | Bằng chứng |
|---|---|
| **D1** catalog per-program, no writeback, tách await/join | `fee_shared.rs:16` (`FeeKind` 8 loại, không có kind "writeback"). `E_AWAIT` chỉ phát ở `codegen.rs:1408` (nhánh `HExpr::Await`) và `interp.rs:1004`; final join (`rt.rs:621`) **không** charge — đo `tasks-1000`: `E_SPAWN=1000`, `E_AWAIT=0` dù 1000 thân task chạy ở join. |
| **D2** weights chưa duyệt, measure trước, không kế thừa 10^7 | `standard_weight()` (`fee_shared.rs:78`) là bảng **v1 đề xuất**, chỉ gọi từ test-hook. Không có ngưỡng mặc định mới; `BUDGET` chỉ tới khi `enable_for_test`/`budget_enable_for_test`. Số đo §7; không ai lấy 10^7 làm BUDGET. |
| **D3** S keyed by PC, nhiều sự kiện có thứ tự | `FeeTable { funcs: Vec<Vec<FeeEntry>> }` (`fee.rs:192`), `FeeEntry.pc/kind/site` (`fee_shared.rs:89`). Test `nhieu_su_kien_cung_pc_giu_thu_tu` (bytecode) + `nhieu_su_kien_cung_pc_giu_thu_tu_khi_charge` (CLI): 3 entry cùng pc giữ `Call,Expr,Assign`. |
| **D4** verifier structure+CFG, không nhận suy đầy đủ | `verify_fee` (`fee.rs:246`) + 9 ca âm/dương trong `tests/fee_side_table.rs`. Báo cáo **không** nhận bảng phụ là bằng chứng instrumentation nguồn đầy đủ (§8). |
| **D5** chỉ serialize kind/site, bảng chuẩn có phiên bản | `encode_with_fee`/`decode_fee` (`fee.rs:129,158`) ghi `pc:u32 + kind:u8 + site:u32` — **không** có trường weight. Runtime lấy `standard_weight(kind)` (`vm_frame.rs:83`, `fee.rs::charge`). Không đọc trọng số từ artifact. |
| **D6** continuation lưu trạng thái đã charge, pc sau điểm phí | VM tăng `pc` rồi mới `budget_charge` (`vm.rs:741` trước `pc += 1` ở dòng 742 → pc hiện hành được charge, pc lưu là pc kế) — xét theo đích nhảy, resume ở pc sau điểm charge. Test runtime `resume_khong_charge_lai_nhom_da_xong`. **Không** đẩy pc qua thao tác chưa xong. |
| **D7** VERSION chưa dùng + cache cùng đợt | `fee::VERSION = 4` (`fee_shared.rs:8`), cache key đưa số này (`cache.rs:90`) + nhãn `vd256-fee-side-table-v4` (`cache.rs:28`); hash ghim cập nhật `cca587…f20`. Bằng chứng "4 chưa dùng": 1/2 là VBC1, **3 đã là sentinel version lạ** tại `tests/decode_neg.rs:52-54` và `tests/verify.rs:68-69` — không ấn 3. |
| **D8** watchdog độc lập, không che verifier | `SAFETY_FUEL=10^7`, `SAFETY_FUEL_CODE` short `VS0501-FUEL` (`vm_frame.rs:25`); verifier chạy trước VM (`vm.rs:206`). Chẩn đoán ngân sách `VS0501-BUDGET` (`frame/core.rs`) — test `budget_va_safety_fuel_co_chan_doan_rieng` khẳng định 3 short khác nhau. |
| **D9** parity step-cap giữ OPEN, không tăng ngưỡng | `SAFETY_FUEL == MAX_STEPS == 10_000_000` (test assertions). Không tuyên bố parity ở workload chạm giới hạn; report này không sửa finding 2M vòng. |

### Ghi chú D7 (quan trọng, trung thực)

`vis_bytecode::VERSION` (VBC1 lõi) **vẫn là 2**. Lý do: bảng phụ là phần **bổ sung dạng trailer**,
không đổi `MAGIC`/opcode/bố cục lõi; tăng `VERSION` lõi lên 4 sẽ làm **2 assertion outside ownership**
(`crates/vis-cli/tests/st246_cache_compat.rs:109,121-125`) fail dù hàng rào ST-246 vẫn đúng. Thay vào đó
định dạng **mới** (bảng phí) nhận `VERSION=4` ngay trong module `vis_bytecode::fee`, và cache được cập
nhật **trong cùng đợt**。 Nếu A0 muốn `VERSION` lõi nhảy 2→4, đó là patch riêng kèm cập nhật các test
ST-246 — không tự làm ở đây.

---

## 2. Catalog mapping (file:line)

| Sự kiện | Compiler (`vis-bytecode`) | Interp (`vis-interp`) | Ghi chú |
|---|---|---|---|
| `E_STMT` | `codegen.rs:415` (`cau`), `:334` (Expr cuối) | `interp.rs:232` (`exec_stmt`) | 1 lần/statement, không re-charge khi resume |
| `E_EXPR` | `codegen.rs:862` (`expr`), `:995` (callee builtin) | `interp.rs:628` (`eval`) | composite có phí riêng + con thực sự chạy có phí riêng; skip Call/Spawn/Await |
| `E_ASSIGN` | `codegen.rs:427,446,475` | `interp.rs:244,303,337,352,376,427` | sau RHS+đích; **không** VM writeback/copy tham số/spill/temporary |
| `E_LOOP` | `codegen.rs:634,657,695` | `interp.rs:482,506,533` | For/While/Loop; `loop` có điểm phí trước lần vào thân đầu |
| `E_CALL` | `codegen.rs:990` | `interp.rs:749` | trước đối số; user function |
| `E_BUILTIN` | `codegen.rs:988` | `interp.rs:747` | host/builtin; **không** cộng E_CALL |
| `E_SPAWN` | `codegen.rs:1422` | `interp.rs:1025` | `$spawn` chỉ mang event này |
| `E_AWAIT` | `codegen.rs:1408` | `interp.rs:1004` | source await only; final join không có |

Capture writeback (`Assign` do VM ở `vm.rs` prologue closure) **không** charge — đúng D1 §3.

---

## 3. Verifier cho bytecode không đáng tin (nghĩa vụ B)

`verify_fee(m, fee)` (`fee.rs:246`), chạy sau `verify(m)` và trước VM qua
`load_verified_with_fee` (`fee.rs:308`); mọi sai ⇒ `LoadError::Verify`, **không** vào VM:

1. **Kích thước**: `funcs.len() == m.funcs.len()` (`FuncCount`); tổng ≤ `MAX_FEE_ENTRIES=10^6`;
   ≤ `MAX_PER_PC=16` entry/PC (`TooMany`, `TooManyAtPc`).
2. **PC/site**: `pc < code.len()` (`BadPc`); `site == 0` (node sinh) hoặc nằm trong `spans[pc]`
   (`BadSite`) — ràng buộc **suy độc lập từ cấu trúc**, không tin artifact tự khai.
3. **Bảng rỗng**: có code mà total == 0 ⇒ `EmptyTable` — **không** được coi là "zero work".
4. **CFG trong hàm**: cạnh lùi vào đầu vòng mà header không có phí ⇒ `LoopGuardUncharged`;
   SCC có chu trình mà không có điểm phí ⇒ `CycleUncharged` (Tarjan, viết tay, không dependency).
5. **CFG liên hàm**: SCC của đồ thị `Call`/`MakeClosure` có chu trình mà không có phí ⇒
   `CallCycleUncharged` — chỗ `MAX_STEPS` từng làm backstop, giờ verifier là hàng rào.

Ca âm: `tests/fee_side_table.rs` — 9 test, phủ bad func-count / bad pc / bad site / bad kind / rỗng /
chu trình / guard / đệ quy / multiple-order. **Không** rebuild HIR từ opcode (theo D4).

---

## 4. Kế hoạch fuel an toàn — ngắn (A0 yêu cầu trình trước hiện thực)

> Ghi chú trung thực: không có kênh gửi trước khi hiện thực trong phiên này; kế hoạch dưới đây được
> ghi **trong report này** và đã được giữ nguyên trong code.

- **Giới hạn**: 10^7 **lệnh VM**, giữ nguyên (`D9`), **không** đổi tên thành ngân sách, **không** là
  giới hạn chương trình thường song song budget (budget không bật mặc định).
- **Reset**: `Vm::new` đặt `steps=0`; `Vm::run` reset `steps=0` ở đầu mỗi lần chạy (`vm.rs:222`) ⇒
  không cộng dồn giữa các lần `run` trên cùng VM; cũng reset `globals` (ST-250) như cũ.
- **Chẩn đoán**: `VS0501-FUEL`, message nêu rõ "hàng rào an toàn bytecode, không phải ngân sách
  ngữ nghĩa" — khác `VS0501-BUDGET` và `VS0506`.
- **Không che verifier**: `verify()` luôn chạy trước; fuel chỉ là hàng rào cuối cho bytecode đã qua
  verify nhưng vẫn có cạnh lạ. Nếu fuel kích hoạt trên chương trình compiler sinh **trong miền ngân
  sách được hỗ trợ** ⇒ coi là **lỗi thiết kế/hiệu chỉnh**, không phải lỗi chương trình (chưa xảy ra
  trong đo §7).

---

## 5. VERSION / cache diff

| Tệp | Thay đổi |
|---|---|
| `vis-bytecode/src/fee_shared.rs:8` | `pub const VERSION: u16 = 4` (mới; 1/2 VBC1, 3 = sentinel lạ) |
| `vis-bytecode/src/fee.rs:129,158` | `encode_with_fee` / `decode_fee`: trailer `VFEE` + `VERSION` + `WEIGHT_TABLE_VERSION` + `pc/kind/site` |
| `vis-bytecode/src/verify.rs` | thêm `VerifyError::FeeDecode(String)` cho cửa nạp có budget |
| `vis-cli/src/cache.rs:28` | `PHIEN_BAN` += `+vd256-fee-side-table-v4-2026-11-20` |
| `vis-cli/src/cache.rs:90` | `khoa_multi_voi_version` đưa `vis_bytecode::fee::VERSION` vào khoá |
| `vis-cli/src/cache.rs:211` | hash ghim: `c4e923…bb8` → `cca587…f20`; assert `PHIEN_BAN`/`fee::VERSION` |

`decode()` lõi bỏ qua trailer ⇒ mọi test/module constructing `Func`/`Module` literal cũ **vẫn biên dịch**
(không đổi struct `Func`/`Module`).

---

## 6. Test evidence

### 6.1 Dãy sự kiện có thứ tự (nghĩa vụ A)

`lat256_budget.rs::ordered_event_trace_interp_vs_compiler` — chương trình `let a = 1 + 2; print(a)`:

```
want (compiler FeeTable.trace) = [E_STMT@0, E_EXPR@8, E_EXPR@8, E_EXPR@12,
                                  E_ASSIGN@0, E_BUILTIN@14, E_EXPR@14, E_EXPR@20]
got  (interp runtime hook)     = (byte-for-byte giống want)
VM   (budget_enable_for_test)   = (byte-for-byte giống got)
```

Assert là **dãy**, không phải tổng. Hai lệch thật đã được sửa trong lần chạy đầu: (a) câu `Expr` cuối
đi thẳng `expr()` trong codegen ⇒ thiếu `E_STMT` (`codegen.rs:334`); (b) callee builtin được interp
đánh giá như giá trị ⇒ codegen phải charge `E_EXPR` riêng (`codegen.rs:995`).

### 6.2 Final-join (ST-278)

`final_join_error_surfaces_hai_engine` — `async fn boom` gây `VS0502` (`1 // 0`), `spawn` nhưng main
**không await**. Trước fix: stdout `start\n`, exit 0, không diagnostic. Sau fix (cả `run_i` lẫn
`run_v`): stdout `start\n`, **exit 1**, diagnostic `VS0506` với message giữ **mã gốc `VS0502`**, cùng
`line/col` trên hai engine. `final_join_khong_doi_chinh_sach_huy_task_con_lai`: task `ok` vẫn join,
thứ tự tạo giữ nguyên — **không** đổi chính sách hủy.

Wiring: `rt.rs:621` (`frame_final_join` quét `kq` theo thứ tự tạo, trả `Trap`, lưu `final_join_error`),
`vm_frame.rs:327` và `frame.rs:944` (driver gán vào `trap` khi chưa có trap từ main).

### 6.3 Counts

- `./vis-dev check`: fmt/clippy/R1..R6/self-test/report/doc-examples/readme — **PASS**.
- `cargo test -p vis-interp -p vis-cli -p vis-bytecode`: **1151 passed, 0 failed, 1 ignored**.
- `cargo test -p vis-cli --test lat256_budget`: **8 passed, 0 failed, 1 ignored** (ignored = đo).
- `cargo test -p vis-vm`: tất cả result `ok`, 0 failed.
- `cargo test -p vis-bytecode --test fee_side_table`: **9 passed**.

---

## 7. Đo workload + đề xuất (CHƯA DUYỆT)

Chạy: `cargo test -p vis-cli --test lat256_budget -- --ignored --nocapture`.
Đếm theo kind (`enable_measure_for_test`, không lưu trace), proposed cost = `Σ n×w` với w đề xuất.

| Workload | wall (debug) | events | E_STMT | E_EXPR | E_ASSIGN | E_LOOP | E_CALL | E_BUILTIN | E_SPAWN | cost đề xuất |
|---|---|---|---|---|---|---|---|---|---|---|
| loop 2M (`while i<2e6`) | 3.43 s | 18,000,008 | 2,000,002 | 12,000,004 | 2,000,001 | 2,000,001 | 0 | 0 | 0 | 18,000,008 |
| recursion 200 | 7.1 ms | 2,211 | 403 | 1,606 | 0 | 0 | 201 | 1 | 0 | 2,815 |
| tasks 1000 (spawn + final join) | 5.5 ms | 14,008 | 3,002 | 7,004 | 2,001 | 1,001 | 0 | 0 | 1,000 | 17,008 |
| expr-chain 100k | 387 ms | 2,000,008 | 200,002 | 1,500,004 | 200,001 | 100,001 | 0 | 0 | 0 | 2,000,008 |

**Đề xuất cho A0 (KHÔNG phải default, chưa approved):**

- Trọng số `w` (bảng v1): `E_STMT/E_EXPR/E_ASSIGN/E_LOOP = 1`, `E_BUILTIN/E_AWAIT = 2`,
  `E_CALL/E_SPAWN = 4`. Lý do tương đối: 1 expression + 1 literal ≈ đơn vị công; call/spawn mở khung
  hoặc đăng ký nặng hơn; builtin/await là ranh giới host/wait. **Có thể sai** — cần đo release + phân
  lớp sâu hơn trước khi chốt.
- `BUDGET` đề xuất: **20,000,000** (~1.1× ca loop 2M, thoải mái dưới ngưỡng) — **không kế thừa 10^7**,
  không bật mặc định.
- VM safety fuel: **giữ 10^7 lệnh** (D9), reset mỗi `run`, chẩn đoán `VS0501-FUEL`; **không** coi là
  budget và **không** dùng tuyên bố parity.
- Chưa có số cho `Q` (hạn ngạch lượt chạy/CPU) — nợ riêng, không tự ấn.

---

## 8. Trung thực về phạm vi (không tuyên bố parity chưa test)

Đây là **subset có thật**, không phải instrumentation đầy đủ. Các điểm **chưa cover** / chưa chứng minh:

1. **Parity source chưa đầy đủ**: test chỉ so dãy cho chương trình top-level `let` + `print` +
   builtin-callee. Chưa so dãy cho `match`, `with`, `try/?`, closure capture, method người dùng,
   `CallValue`, và **gán field/index** (codegen charge `E_ASSIGN` ở plain assign + Let/Var; interp
   charge cả bốn đường ghi — hai đường field/index **chưa có charge codegen tương ứng**). Bảng phụ
   **không** là bằng chứng instrumentation nguồn đầy đủ (VD-256 §4A).
2. **Phân biệt user method / host method**: mới phân biệt theo `fmap_ten`/`la_builtin` cho callee dạng
   `Var`; nhánh `CallMethod` và method trên giá trị chưa có test parity riêng.
3. **Resume double-charge**: test `resume_khong_charge_lai_nhom_da_xong` kiểm **quy ước pc-sau-charge**
   (runtime), **chưa** chạy suspend/resume thật của frame scheduler dưới budget. `E_STMT` không
   re-charge khi resume là quy ước trong code, chưa có test frame-path chứng minh. Không tuyên bố
   verifier bắt hết double-charge (đúng D6/§4).
4. **VM budget hook chưa nối cache mặc định**: `encode_with_fee`/`decode_fee` tồn tại và được test,
   nhưng `cache::nap/luu` vẫn dùng `encode`/`load_verified` không trailer — cố ý, vì budget không phải
   mặc định.
5. **Không rebuild HIR từ bytecode** — đúng D4; verifier chỉ suy từ opcode/CFG.
6. **Step-cap parity 2M vòng giữ OPEN** — budget mới chưa thay thế được guard lệch giữa hai engine;
   report này không sửa finding đó.

---

## 9. Khẳng định "mặc định không đổi"

Đường mặc định (`vis run`, `vis run --vm`, API `vis_interp::run`, `vis_vm::run`, `encode`, `decode`,
`verify`, `load_verified`, `compile`, cache path thường) **không** charge, **không** có ngưỡng mới,
không đọc trailer phí. Mọi hook là `enable_for_test` / `budget_enable_for_test` / `compile_with_fee` /
`encode_with_fee` / `load_verified_with_fee`, chỉ test gọi. `Cargo.toml/Cargo.lock`, `rust-toolchain.toml`,
`agent/DECISIONS.md`, `agent/QUEUE.md`, `net.rs`, `stdlib.rs`, `lat256_net.rs`, mọi `vd255-*` **không bị
sửa bởi task này** (các tệp `DECISIONS.md`/`QUEUE.md`/`stdlib.rs` đang sửa là của agent song song).

**Ngoại lệ duy nhất so với "API/bytecode policy không đổi"**: định dạng **bảng phí mới** có
`VERSION=4` và cache key đổi (§5) — đúng D7; opcode/MAGIC/`VERSION` lõi VBC1, bảng thẻ và ngữ nghĩa
thực thi mặc định giữ nguyên.

---

## 10. Tệp thuộc quyền — ghi chú

Ngoài các tệp được phép, **một dòng wiring 3 dòng** đã được sửa ở `crates/vis-interp/src/frame.rs:944`
(đây là module `frame` chứa cả `frame/**`, cần để driver interp nhận lỗi final join). Không sửa
`crates/vis-interp/src/frame.rs` thì chỉ VM surfaced được; report nêu rõ để A0 quyết nếu muốn dời wiring
vào rt.rs bằng API khác. Mọi tệp khác nằm trong ownership: `vis-bytecode/**`, `vis-cli/src/cache.rs`,
`vis-cli/tests/lat256_budget.rs`, `vis-interp/src/{interp.rs,rt.rs,frame/**}`, `vis-vm/src/{vm.rs,vm_frame.rs}`,
và report này.
