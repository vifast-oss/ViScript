<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-250 / ST-266 · Nháp **đặc tả tính phí ngân sách** (5 câu hỏi còn mở)

**Trạng thái**: **DRAFT — CHỜ A0 CHỐT**. research + draft only; không sửa mã sản phẩm, không commit, không đổi VERSION/opcode, không chạy gate.
**Căn cứ**: `agent/DECISIONS.md:2487-2504` (VD-250 §4), `agent/JOURNAL/vd249-budget-draft.md:16-32`, `:64-71`, `agent/JOURNAL/vd249-exit101-triage.md:43-49`.
**Quy ước**: nhận định "hiện trạng" có `file:line`; mọi đề xuất gắn **[CHỜ A0 CHỐT]**. Bảy trọng số `1/1/1/4/2/4/2` **chưa được duyệt** (`agent/DECISIONS.md:2493`) — tài liệu này dùng ký hiệu `w(E_*)` chứ không ấn định số.

---

## 1. Phân ranh giới đã duyệt / chưa duyệt

**A0 đã duyệt** (VD-250 §4, `agent/DECISIONS.md:2489-2492`):

- Một pool cho toàn lần chạy, **gồm cả task**; **không reset** khi spawn/await/join; **không refund** khi huỷ.
- `chi phí == remaining` ⇒ **sự kiện được phép**; sự kiện kế tiếp bị từ chối.
- **Không rollback** tác dụng phụ đã xong.
- depth / bộ nhớ / I/O là **giới hạn riêng**, không gộp vào ngân sách.
- Dùng `VS0501`, giữ wrapper `VS0506` + exit **101**; duyệt **test-hook nội bộ** đặt budget nhỏ.

**A0 chưa duyệt** (cùng mục): bảy trọng số (`agent/DECISIONS.md:2493`); định dạng/biểu diễn (`agent/DECISIONS.md:2499-2503`); VERSION mới. Xem `vd250-fee-repr.md`.

### 1.1 Hiện trạng liên quan

| Điểm | Hiện trạng (file:line) |
|---|---|
| Interp `steps` | `pub steps: usize` `crates/vis-interp/src/interp.rs:101`, khởi tạo `:168`, tăng ở `exec_stmt` `:300` và `eval` `:645` — **đếm nút ngữ nghĩa**, không phải budget. Hàng rào thật nằm ở vòng lặp `while guard > 10_000_000` `:541` và `loop for _ in 0..10_000_000` `:559`. |
| VM `MAX_STEPS` | `pub const MAX_STEPS: usize = 10_000_000` `crates/vis-vm/src/vm.rs:97`; `vm.steps += 1` **trước mỗi lệnh** `:736`; kiểm tra `:737-743` ⇒ trap `VS0501`. |
| Lệch đã đo | 2M vòng: interp exit 0 / VM exit 101 — `agent/JOURNAL/vd249-exit101-triage.md:12`, `:33-35`, `:46`. |
| Cap riêng phải giữ | `MAX_CALL_DEPTH = 2_000` `crates/vis-interp/src/interp.rs:1401`, kiểm tra `crates/vis-interp/src/interp/goi.rs:128`, `:208`; `MAX_FRAMES = 2_000` `crates/vis-vm/src/vm.rs:95`, kiểm tra `:557`, `:818`; `MAX_TASK_NEST = 256` `crates/vis-interp/src/rt.rs:24`, kiểm tra `:209-213`. |
| Điểm chèn dùng chung | `vis_interp::builtin` và `vis_interp::rt` được VM reuse: `crates/vis-vm/src/vm.rs:13`, `:229`, `:851`; dispatcher builtin `crates/vis-interp/src/builtin.rs`. |
| Exit | `ma_thoat_khi_loi`: `VS05*` ⇒ 101 `crates/vis-cli/src/cli/run.rs:13-21`; task error bọc `VS0506` ở `crates/vis-interp/src/rt.rs:233-239`. |

---

## 2. Nguyên tắc chung để trả lời 5 câu

**[CHỜ A0 CHỐT] 2.1 — Một sự kiện = một lần `charge`, tại một điểm biên duy nhất.**
Mỗi `(mã sự kiện, vị trí)` chỉ được trừ phí **một lần** cho một lần thực thi. Hai engine phải sinh **cùng dãy `(mã, w)`**; nếu cùng tên sự kiện mà số lần khác nhau thì parity hỏng (VD-250 `agent/DECISIONS.md:2495-2496`). Khi một câu lệnh chứa nhiều khái niệm (gọi hàm *và* biểu thức *và* câu lệnh), A0 phải chọn **một** điểm biên cho mỗi khái niệm theo bảng dưới — không được để cả hai engine tự chọn.

**[CHỜ A0 CHỐT] 2.2 — Thứ tự kiểm tra.**
`charge(e, w)` nguyên tử: so `w` với `remaining` **trước** dựng khung, trước vào thân vòng, và **trước mọi tác dụng phụ** (VD-249 `vd249-budget-draft.md:66-67`; builtin đi qua `crates/vis-interp/src/builtin.rs`, VM qua `crates/vis-vm/src/vm.rs:839-859`). `w > remaining` ⇒ trap, **không trừ**, **không thi hành**; `w == remaining` ⇒ trừ về 0, thi hành, sự kiện sau trap; không rollback (đã duyệt `agent/DECISIONS.md:2489-2492`).

**[CHỜ A0 CHỐT] 2.3 — Không trừ phí khi không có công ngữ nghĩa tương ứng.**
Đây là tiêu chí quyết cả 5 câu: nếu một engine "đi qua" một điểm mà chương trình không thực hiện công gì (nhánh không chạy, resume không đánh giá lại biểu thức), điểm đó **không** được tính phí.

---

## 3. Năm câu hỏi — đề xuất và ví dụ

### 3.1 `E_CALL` có tính thêm `E_EXPR` không?

**Hiện trạng**: `E_CALL` được đề xuất gắn ở mở khung `crates/vis-interp/src/interp/goi.rs:128`, `:208` và `Instr::Call` `crates/vis-vm/src/vm.rs:809`, `:1157` (`vd249-budget-draft.md:25`, `:41`). Biểu thức **`f` ở vị trí callee** và các **đối số** là các `HExpr`, đi qua `eval` `crates/vis-interp/src/interp.rs:645`.

**[CHỜ A0 CHỐT] Đề xuất: KHÔNG tính `E_EXPR` cho chính nút lời gọi; có tính `E_EXPR` cho từng đối số.**
Điểm biên: khi `exec_stmt`/`eval` gặp nút `Call`, chỉ phát **một** `E_CALL` cho nút đó; các con (callee + đối số) được đánh giá bình thường và mỗi biểu thức con phát `E_EXPR` riêng. `E_CALL` **thay thế**, không **chồng lên**, `E_EXPR` của cùng một nút.

| Chương trình | Dãy sự kiện đề xuất |
|---|---|
| `f(1)` với `fn f(x)->int: x` | `E_STMT`(câu gọi) + `E_CALL`(vào `f`) + `E_EXPR`(biểu thức `1`) + … thân `f` |
| `f(g(1))` | `E_CALL(f)` + `E_CALL(g)` + `E_EXPR(1)` — **không** có `E_EXPR` thừa cho hai nút `Call` |

**Lý do**: nếu tính cả hai, cùng một lời gọi có trọng số `w(E_CALL)+w(E_EXPR)` ở chỗ này nhưng `w(E_EXPR)` ở chỗ khác tuỳ vào chỗ cắm của từng engine — đúng lỗi "cùng tên sự kiện mà tính khác số lần" mà A0 cấm. **[CHỜ A0 CHỐT]** nếu A0 muốn gộp `E_EXPR` vào `E_STMT` như `vd249-budget-draft.md:23` nêu, phải đổi định nghĩa này trước khi chốt trọng số.

### 3.2 Builtin có chịu **cả** call lẫn builtin không?

**Hiện trạng**: `E_BUILTIN` được định nghĩa là builtin/phương thức `print`, `std.fs.*`, `std.net.*`, `std.sqlite.*`, với điểm chèn chung `crates/vis-interp/src/builtin.rs`; VM gọi cùng dispatcher ở `crates/vis-vm/src/vm.rs:851` (`vd249-budget-draft.md:26`, `:42`). Trên bytecode, `CallBuiltin` (tag 9, `crates/vis-bytecode/src/format.rs:245`) và `CallMethod` (tag 10, `:246`) là **opcode riêng** với `Call` (tag 8, `:244`).

**[CHỜ A0 CHỐT] Đề xuất: chỉ `E_BUILTIN`, KHÔNG `E_CALL`.**
Tiêu chí: `E_CALL` = mở **khung ViScript do chương trình định nghĩa** (`interp/goi.rs:128`, `vm.rs:809`); builtin không mở khung ViScript, đi thẳng vào host. Vì vậy một lần gọi builtin là **một** sự kiện `E_BUILTIN`.

| Chương trình | Dãy sự kiện đề xuất |
|---|---|
| `print(1)` | `E_EXPR(1)` + `E_BUILTIN(print)` — **không** có `E_CALL` |
| `xs.sort_by(f)` | `E_EXPR(xs)` + `E_BUILTIN(sort_by)`; mỗi lần `f` được gọi là một `E_CALL` riêng (nếu `f` là hàm người dùng) |
| `spawn f(1)` / `await t` | `E_SPAWN` / `E_AWAIT` như `vd249-budget-draft.md:27-28`, **không** cộng thêm `E_BUILTIN` (đây là builtin nội bộ, không phải ranh giới host) |

**Lý do**: tránh tính đôi tại cùng một điểm — hai engine đều thấy đúng một lệnh `CallBuiltin`/`CallMethod`, nhưng nếu một bên cộng thêm `E_CALL` ở dispatcher dùng chung thì dãy `(mã, w)` lệch.

### 3.3 Short-circuit: "chỉ tính nhánh chạy" cụ thể hoá thế nào?

**Hiện trạng**: bytecode có `JumpIfNotTrue` "giữ giá trị — cho `and`/`or`" `crates/vis-bytecode/src/format.rs:145-147`, emitted ở `crates/vis-bytecode/src/codegen.rs:900`, `:908`; `and`/`or` không đoản mạch `BoolAnd`/`BoolOr` ở `:222-227`. Interp đánh giá `eval` `crates/vis-interp/src/interp.rs:645`.

**[CHỜ A0 CHỐT] Đề xuất: điểm tính phí nằm ở **việc đánh giá mỗi toán hạng**, không nằm ở nút `and`/`or` hay ở lệnh nhảy.**
- Không có `E_EXPR` cho bản thân nút `and`/`or` (nó chỉ điều khiển luồng, không tạo ra giá trị mới độc lập).
- Toán hạng trái luôn được đánh giá ⇒ luôn có `E_EXPR(trái)`.
- Toán hạng phải chỉ được đánh giá khi nhảy không xảy ra ⇒ **chỉ khi đó** mới có `E_EXPR(phải)`.
- Vì vậy điểm charge phải đặt **sau lệnh nhảy**, tại đầu khối toán hạng phải — không đặt tại `JumpIfNotTrue`.

| Chương trình | Dãy sự kiện đề xuất |
|---|---|
| `false and expensive()` | `E_EXPR(false)`; **không** có `E_EXPR(expensive())`, **không** có `E_BUILTIN` trong `expensive` |
| `true or expensive()` | `E_EXPR(true)`; **không** có phía phải |
| `true and expensive()` | `E_EXPR(true)` + `E_EXPR(expensive())` + các sự kiện thân `expensive` |
| `BoolOr`/`BoolAnd` (pattern, không đoản mạch) | Cả hai toán hạng đều có `E_EXPR` — đây là điểm dễ lệch giữa hai engine |

**Hệ quả cho biểu diễn**: bảng phụ/opcode phải đánh dấu được "điểm sau nhảy" (`vd250-fee-repr.md` §3.2, §3.4). **[CHỜ A0 CHỐT]**

### 3.4 Loop: tính ở kiểm điều kiện, vào thân, hay cả hai?

**Hiện trạng**: interp có hàng rào riêng cho `while` `crates/vis-interp/src/interp.rs:541` và `loop` `:559`; VM không phân biệt vòng — `vm.rs:736-743` đếm mọi lệnh. Codegen phát: `for` → `IterNext` `crates/vis-bytecode/src/codegen.rs:701` (VM `crates/vis-vm/src/vm.rs:1088`); `while` → `JumpIfFalse(0)` + `Jump` lùi `codegen.rs:721-756`; `loop` → `Jump` lùi `codegen.rs:758-767`. Đây là lý do 2M vòng lệch engine (`vd249-exit101-triage.md:46`).

**[CHỜ A0 CHỐT] Đề xuất: một `E_LOOP` cho **mỗi lần đánh giá điều kiện / kiểm back-edge**, kể cả lần cuối trả `false`; KHÔNG tính thêm khi vào thân.**
- `while`: mỗi lần thực hiện guard là một `E_LOOP` (kể cả lần guard sai thoát vòng).
- `loop`: mỗi lần tới lệnh nhảy lùi là một `E_LOOP`.
- `for`: mỗi lần `IterNext` kiểm tra iterator là một `E_LOOP` (kể cả lần hết phần tử).
- Thân vòng không có `E_LOOP` riêng; các câu lệnh trong thân trả `E_STMT` bình thường.

| Chương trình | Số `E_LOOP` đề xuất |
|---|---|
| `while i < 3: i = i + 1` (từ `i=0`) | **4** — guards `0<3,1<3,2<3` (đúng) + `3<3` (sai, thoát) |
| `for i in [1,2,3]` | **4** — 3 lần lấy phần tử + 1 lần `IterNext` báo hết |
| Vòng lặp `while true { break }` | **1** (một lần guard, rồi `break` thoát) |

**Lý do**: nếu chỉ tính lần guard đúng, hai engine sẽ phải "biết trước" kết quả guard để quyết có charge không — không thể định nghĩa trung lập được. Tính cả lần sai cho ra một quy tắc đơn: **mỗi lần máy đánh giá điều kiện là một lần charge**, bất kể kết quả.

**Cấm làm**: giữ đồng thời `guard > 10_000_000` (`interp.rs:541`) / `for _ in 0..10_000_000` (`:559`) như một ngưỡng ngầm song song budget; mọi hàng rào phải đi qua cùng hàm `charge()` (`vd249-budget-draft.md:40`, VD-250 `agent/DECISIONS.md:2497-2498`).

### 3.5 Await resume: có tính lại phí bắt đầu không?

**Hiện trạng**: `await_task` `crates/vis-interp/src/rt.rs:178-231` trả cache ngay nếu `TrangThai::Xong` `:180-194`, chặn lồng `:209-213`, gọi thân một lần `:218` rồi cache `:227`; lỗi bọc `VS0506` `:233-239`. Với frame heap mới, resume tiếp tục tại `pc` (`crates/vis-vm/src/vm.rs:124-135`, `:716-735`; đề xuất `vd249-async-design-draft.md:49-51`).

**[CHỜ A0 CHỐT] Đề xuất: `E_AWAIT` tính **một lần cho mỗi lần đánh giá biểu thức `await`**, tại thời điểm chạm `await`; một chu kỳ suspend→resume **không** tính lại.**
- Resume là **tiếp tục cùng một biểu thức**, không phải đánh giá lại nó ⇒ không có `E_AWAIT` thứ hai, không có `E_EXPR` lặp lại cho phần trước suspend. Đây cũng là hệ quả của tiêu chí Lát 0 "resume **không lặp tác dụng phụ**" (`agent/DECISIONS.md:2483-2485`).
- `await t` **thứ hai** (hai câu lệnh riêng, hoặc hai lần trong vòng lặp) là một đánh giá mới ⇒ tính `E_AWAIT` mới, kể cả khi `t` đã `Xong` và trả cache `rt.rs:180-194`. Khớp khuyến nghị A0-5 cũ `vd249-budget-draft.md:109`.

| Chương trình | Dãy sự kiện đề xuất |
|---|---|
| `await t` với `t` suspend 1 lần rồi resume | **1** `E_AWAIT` |
| `await t; await t` (`await-hai-lan-cache.vis`) | **2** `E_AWAIT` (lần 2 trả cache `rt.rs:180-194` nhưng vẫn tính) |
| `for i in ...: await t` (3 vòng) | **3** `E_AWAIT` + 4 `E_LOOP` |
| Resume do I/O ready / timer fire | **0** `E_AWAIT` mới |

**Yêu cầu phối hợp với biểu diễn**: điểm charge `E_AWAIT` phải nằm **trước** điểm suspend và `pc` resume phải nằm **sau** điểm đó; nếu không, verifier/test phải có cờ "đã charge" trong frame. Xem `vd250-fee-repr.md` §3.6. **[CHỜ A0 CHỐT]**

---

## 4. Bộ đếm hạn ngạch CPU — tách khỏi ngân sách

**A0 đã chốt** (VD-250 §1 `agent/DECISIONS.md:2447`, §4 `:2504`): bộ đếm **riêng** khỏi ngân sách; **reset theo lượt chạy** (scheduling turn); **có thể tích lũy từ cùng sự kiện ngữ nghĩa** đã chuẩn hoá.

**[CHỜ A0 CHỐT] Đề xuất cụ thể:**

| Thuộc tính | Ngân sách ngữ nghĩa (VD-249) | Hạn ngạch CPU (VD-250) |
|---|---|---|
| Phạm vi | Một pool cho **toàn lần chạy**, gồm task `agent/DECISIONS.md:2489-2490` | Mỗi **lượt chạy** của một task, reset khi task bị nhường `agent/DECISIONS.md:2504` |
| Reset | Không reset ở spawn/await/join, không refund `:2489-2491` | Reset mỗi turn; không ảnh hưởng pool chung |
| Đơn vị | Sự kiện ngữ nghĩa `E_*` | Cùng điểm sự kiện đã chuẩn hoá, nhưng **chỉ là bộ đếm nhường**; không phải chi phí |
| Ngưỡng | `BUDGET` chưa chọn `vd249-budget-draft.md:32` | `Q` chưa chọn; đo sau (`vd249-async-design-draft.md:63-67`) |
| Vượt | Trap `VS0501` / wrapper `VS0506`, exit 101 `agent/DECISIONS.md:2492` | Xếp task vào cuối `ready`, **không** báo lỗi, **không** trap |
| Nguồn thời gian | Không dùng wall-clock | Không dùng wall-clock (`vd250-determinism-contract.md` §3.1) |

**Không được** tái dùng `steps`: interp đếm nút (`interp.rs:101,300,645`), VM đếm lệnh (`vm.rs:97,736`) — gắn nhường vào đó phá parity đã đo (`vd249-exit101-triage.md:46`; `vd249-async-design-draft.md:64`). **[CHỜ A0 CHỐT]**

---

## 5. Cấm giữ `MAX_STEPS` làm giới hạn chương trình thường

**A0 đã chốt** (VD-250 `agent/DECISIONS.md:2497-2498`): **không duyệt** giữ VM `MAX_STEPS` đếm opcode làm giới hạn chương trình thường song song ngân sách mới — sẽ **tái tạo lỗi parity**. Hiện trạng: `vm.rs:97` (`10_000_000`), `:736-743` (trap `VS0501`); đã gây 2M vòng interp 0 / VM 101 (`vd249-exit101-triage.md:12`, `:33-35`).

**[CHỜ A0 CHỐT] Watchdog nội bộ (nếu có)**: phải là cơ chế **khác** với hết ngân sách ngữ nghĩa, báo **rõ tên/nguồn** trong chẩn đoán (vd. "watchdog nội bộ" thay vì "vượt ngân sách"), chỉ dùng cho mục đích an toàn runtime/test, không phải giới hạn chương trình áp lên người dùng. Không được âm thầm gọi chung một thông điệp `VS0501` với budget, cũng không được dùng nó để che crafted bytecode (việc đó thuộc verifier — `vd250-fee-repr.md` §3.5).

---

## 6. Câu hỏi gửi A0

| # | Câu hỏi | Khuyến nghị |
|---|---|---|
| C1 | `E_CALL` có cộng `E_EXPR` cho chính nút gọi không (§3.1)? | Không; `E_CALL` thay chỗ `E_EXPR` của nút đó, đối số vẫn tính riêng. |
| C2 | Builtin chịu `E_CALL` + `E_BUILTIN` hay chỉ `E_BUILTIN` (§3.2)? | Chỉ `E_BUILTIN`. |
| C3 | Short-circuit: charge ở đâu để "chỉ nhánh chạy" đúng ở cả hai engine (§3.3)? | Ở đầu mỗi toán hạng, sau lệnh nhảy; không charge nút `and`/`or`. |
| C4 | Loop tính ở guard, thân, hay cả hai; có tính guard sai cuối cùng không (§3.4)? | Một `E_LOOP` mỗi lần đánh giá điều kiện, **kể cả lần sai**; không charge thân. |
| C5 | Resume có tính lại `E_AWAIT` không (§3.5)? | Không cho cùng một biểu thức; có cho `await` thứ hai / lần lặp mới. |
| C6 | Chốt `Q` và đơn vị hạn ngạch CPU có tách hẳn khỏi `steps` không (§4)? | Tách; `Q` đo sau, không cam kết số. |
| C7 | Cho phép watchdog nội bộ distinct hay cấm hoàn toàn (§5)? | Cho phép distinct, báo rõ, không dùng làm giới hạn chương trình. |
| C8 | Bảy trọng số `1/1/1/4/2/4/2` — chốt nguyên vẹn hay chỉ chốt danh sách mã rồi đo (VD-250 `agent/DECISIONS.md:2493`)? | Chưa chốt số; cần A0 quyết trước khi viết bảng trọng số. |

---

## 7. Giới hạn phạm vi

Không đổi `interp.rs`, `vm.rs`, `builtin.rs`, `rt.rs`, không thêm opcode `Charge`, không ấn `VERSION`. Sau khi A0 trả lời C1–C8, phiếu tiếp theo mới được phép đề xuất bảng trọng số + biểu diễn (`vd250-fee-repr.md`).
