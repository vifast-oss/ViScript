<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-252 / ST-269 · Ma trận coverage — đường frame test Lát 1

**Trạng thái**: **RESEARCH + DRAFT ONLY** — không sửa mã sản phẩm, không commit, không chạy gate.
**Căn cứ**: VD-252 §1 (`agent/DECISIONS.md:2625-2632`), báo cáo Lát 1 §4
(`agent/JOURNAL/vd251-lat1-report.md:87-113`), `crates/vis-interp/src/frame.rs`,
`crates/vis-vm/src/vm_frame.rs`, HIR (`crates/vis-types/src/hir.rs`).

---

## 0. Bất biến cứng (đọc đầu)

1. **Checker acceptance phải KHÔNG thu hẹp.** `vis_types::check` tiếp tục chấp nhận mọi vị trí
   `await`/cấu trúc như trước (`vd251-lat1-report.md:95-96`; VD-252 `agent/DECISIONS.md:2630-2632`).
   Không được sửa checker để "đỡ phải phủ" ở đường test.
2. **Không được phủ ⇒ từ chối rõ.** Mọi cấu trúc đi vào evaluator frame mà không có ngữ nghĩa được
   mô phỏng phải phát một **chẩn đoán/Trap tường minh, có `Span`**, ngay tại điểm gặp cấu trúc —
   **trước khi** đẩy operand/con đánh giá, để không chạy side-effect rồi mới báo lỗi.
   * Không fallback âm thầm sang đường đồng bộ / `Interp::run_program` / `Vm::run`
     (`frame.rs:10`, `vm_frame.rs:35-37` — các nhánh đó là runtime mặc định, không phải kết quả test).
   * Không "chạy lại từ đầu" (re-run từ entry) khi gặp cấu trúc lạ: mọi trạng thái nằm trên
     `IFrame`/`Th` (`frame.rs:95-100`; `vm_frame.rs:163-165`), resume là tiếp tục work stack/pc đã lưu
     (`frame.rs:614-624`; `vm_frame.rs:41-48`), không re-eval.
3. **Một pass chỉ có nghĩa nếu chạy đúng đường frame.** Nếu unsupported mà vẫn trả kết quả giống
   golden nhờ fallback, ca test đó **vô giá trị**. Vì vậy mỗi dòng "KHÔNG" trong bảng dưới cần một ca
   âm (negative case): chạy qua `run_frame_for_test` phải `exit != 0` + chẩn đoán, trong khi cùng
   nguồn chạy qua runtime mặc định vẫn đúng golden.

**Phân biệt hai lớp** (bảng dưới tách cột): *checker chấp nhận* (ngôn ngữ cho phép) ≠ *đường frame
test phủ* (evaluator Lát 1 mô phỏng được). Lát 1 chỉ là tập con có chủ đích
(`frame.rs:12-16`; `vd251-lat1-report.md:98-104`).

---

## 1. Ma trận

| Cấu trúc HIR / bytecode | Checker chấp nhận? | Đường frame test phủ? | Bằng chứng (file:line) | Hành vi khi unsupported |
|---|---|---|---|---|
| Literal `HExpr::Lit` | Có | **Có** | `frame.rs:467-471` (`lit_value` thật) | — |
| Biến `HExpr::Var` (sym/argv/scope) | Có | **Có** | `frame.rs:472-490` (env → `u32::MAX` → `it.env`) | Trap rõ "biến … chưa định nghĩa" `frame.rs:483-487` |
| `Bin` số học/so sánh (`+ - * / // % ** .. ..= == != < <= > >=`) | Có | **Có** | `frame.rs:491-498`, `594-599`; `builtin.rs:364-401` | `ops::bin` trả lỗi rõ `builtin.rs:402-406` |
| `&&` / `||` (`BinOp::And`/`Or`) | Có (`hir.rs:932-933`) | **KHÔNG** — ⚠ **khoảng trống** | Khớp chung `HExpr::Bin` nhưng không có work short-circuit: cả `Eval(rhs)` lẫn `Eval(lhs)` bị đẩy `frame.rs:491-498`; `ops::bin` chỉ báo lỗi ở `ApplyBin` `builtin.rs:402-406` | **Hiện chưa từ chối sớm**: vế phải bị đánh giá (side-effect có thể chạy) rồi mới trap. Cần từ chối tại entry `Work::Eval` cho `And`/`Or`/`Coalesce`/`Pipe` **trước** khi đẩy operand; VM hạ thành `JumpIfNotTrue` (`vd250-fee-repr.md:83`, `format.rs:145-147`) — không được coi là "đã phủ" |
| `Un` (`-`, `not`) | Có | **KHÔNG** | trap tường minh `frame.rs:499-505` | Trap rõ `"frame test: Un chưa được phủ"` |
| `Call` — callee là tên hàm cấp tệp (sync/async) hoặc builtin, không named arg | Có | **Có** | `frame.rs:506-533`, `classify_callee :657-692`, `dispatch :694-750` | Callee lạ → trap `"callee không phải tên hàm/builtin"` `frame.rs:527-532` |
| `Call` — callee là **closure/lambda** / giá trị (`HExpr::Lambda` `hir.rs:668-673`, `CallValue`) | Có | **KHÔNG** (interp evaluator) | `classify_callee` chỉ nhận `HExpr::Var` có sym/builtin `frame.rs:657-692` | Trap rõ `frame.rs:527-532`. VM dùng bytecode thật nên lệnh `CallValue` chạy được, nhưng **suspend/resume bên trong closure chưa được kiểm chứng** (`vd251-lat1-report.md:105-107`) |
| `Await` | Có | **Có** | `frame.rs:578-581`, `600-627`; hook VM `vm_frame.rs:28-52` | Không đúng `Task[T]` → trap `frame.rs:604-610` |
| `Spawn` — callee tên `async fn` cấp tệp | Có | **Có** | `frame.rs:534-577`, `dispatch :707-714` | Trap rõ `frame.rs:569-575` |
| `Spawn` — callee không phải tên hàm async | **Không** (tyck `VS0412`, `hir.rs:683-684`) | **KHÔNG** | guard kép `frame.rs:569-575` | Trap rõ, kể cả khi checker đã chặn |
| `Let` / `Var` / `Expr` / `Return` | Có | **Có** | `frame.rs:394-413` | `other => Trap` `frame.rs:414-424` |
| Pattern `Bind` / `Wild` | Có | **Có** | `frame.rs:426-445` | `other => Trap` `frame.rs:434-443` |
| Pattern khác (`Lit`/`Ctor`/`Tuple`/`List`/`Record`/`Range`/`Or`/`Rest`) | Có | **KHÔNG** | `hir.rs:455-495`; trap `frame.rs:434-443` | Trap rõ `"pattern … chưa được phủ"` |
| `Assign` (+ `+=`/…), `Scope`, `Block`, `Break`, `Continue` | Có | **KHÔNG** | `hir.rs:278-283, 307-321, 328-332`; trap `frame.rs:414-424` | Trap rõ `"HStmt … chưa được phủ ở đường test Lát 1"` |
| `while` / `loop` / `for` / `while let` (`HCond::Let`) | Có | **KHÔNG** | `hir.rs:292-306, 428-433`; trap `frame.rs:414-424` | Trap rõ `frame.rs:415-423` |
| `if` / `match` (stmt lẫn expr) | Có | **KHÔNG** | `hir.rs:285-291, 322-326, 654-667`; trap `frame.rs:414-424`, `582-591` | Trap rõ |
| Field / Index (cả `?.`) | Có | **KHÔNG** | `hir.rs:598-609`; trap `frame.rs:582-591` | Trap rõ `"HExpr … chưa được phủ"` |
| Record / Tuple / List / Set / Map / EnumLit / Opt / Try / With / ChanNew / Interp / Ref / CmdBlock | Có | **KHÔNG** | `hir.rs:610-704`; trap `frame.rs:582-591` | Trap rõ `frame.rs:583-590` |
| Hằng cấp tệp `HItem::Const` | Có | **KHÔNG** — ⚠ cần rõ hơn | Đường mặc định đánh giá const trước main `interp.rs:229-234`; frame dựng globals rỗng `frame.rs:139-140`, Var rơi xuống `it.env` `frame.rs:481` (cũng rỗng) | Trap **"biến … chưa định nghĩa"** `frame.rs:483-487` — đúng là lỗi, nhưng thông điệp gây hiểu lầm; nên đổi thành `"hằng cấp tệp chưa được phủ ở đường test Lát 1"` |
| Đối số tên `--ten: giá_trị` (call/spawn) | Có | **KHÔNG** | `frame.rs:512-517`, `540-545` | Trap rõ `"đối số tên chưa được phủ"` |
| Multi-frame continuation qua **closure capture** (`captured`/`writeback`) | Có | **KHÔNG** — ⚠ cần từ chối rõ | `CallFrame` mới chỉ có `env` `frame.rs:86-93, 740-747`; VM `task_th` dùng `captured = Vec::new()`, `writeback = None` `vm_frame.rs:232-241`; `vd251-lat1-report.md:102-103, 105-107` | Hiện **không có guard tường minh** — có thể chạy với capture rỗng ⇒ kết quả sai mà vẫn trông như pass. Cần phát Trap khi task/frame yêu cầu capture mà path test không mô phỏng |
| Bytecode: verify, entry/`main_th`, `$await` (`CallBuiltin`) | — (bytecode) | **Có** (VM, real module) | `vm_frame.rs:63-78`, `:244-271`, hook `vm_frame.rs:28-52` (gọi từ `vm.rs:860`, `vd251-lat1-report.md:71-77`) | Bytecode hỏng → `FrameOutput` chẩn đoán `exit=1` `vm_frame.rs:64-77` |

---

## 2. Kết luận ngắn gửi A0

- Tập phủ hiện tại đúng bằng tập fixture Lát 1: `Lit`, `Var`, `Bin` (không gồm `&&`/`||`), `Call`/
  `Await`/`Spawn` với callee là tên hàm/builtin, `Let`/`Var`/`Expr`/`Return`, pattern `Bind`/`Wild`.
  Phần còn lại đều có trap — **ngoại trừ 3 chỗ phải sửa trước khi coi là "từ chối rõ"**:
  (a) `&&`/`||` bị đánh giá cả hai vế trước khi lỗi; (b) hằng cấp tệp báo nhầm thành "biến chưa định
  nghĩa"; (c) closure capture không có guard, có thể chạy sai mà không báo lỗi.
- Không được mở rộng phủ bằng cách đổi checker; muốn phủ thêm cấu trúc nào thì thêm work item trong
  `step_frame_owned` (`frame.rs:358-655`) kèm ca âm/parity, và vẫn giữ nguyên runtime mặc định.
- VM phủ rộng hơn vì chạy bytecode thật, nhưng **không** được dùng để tuyên bố closure/async task có
  capture đã parity (`vd251-lat1-report.md:105-107`).
- Đây chỉ là ma trận + khuyến nghị; A0 đã cho phép thử reactor mà **không** yêu cầu hoàn thiện
  evaluator trước (`agent/DECISIONS.md:2630-2631`). Việc sửa 3 chỗ trên là nghĩa vụ "từ chối rõ",
  không phải mở rộng tính năng ngôn ngữ.
