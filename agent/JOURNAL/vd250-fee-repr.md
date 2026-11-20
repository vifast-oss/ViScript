<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-250 / ST-266 · Nháp **biểu diễn dữ liệu tính phí**: bảng phụ vs `Instr::Charge`

**Trạng thái**: **DRAFT — CHỜ A0 CHỐT**. research + draft only; không thêm opcode, không đổi VERSION, không sửa mã sản phẩm, không commit, không chạy gate.
**Căn cứ**: `agent/DECISIONS.md:2499-2503` (VD-250 §4 — A0 **chưa chốt** biểu diễn), `agent/JOURNAL/vd249-budget-draft.md:44-49`, `:80-85`, `agent/JOURNAL/vd250-charging-spec.md` §3.
**Nguyên tắc nền**: hai phương án dưới là **hai biểu diễn của CÙNG một hợp đồng** (dãy sự kiện ngữ nghĩa ở `vd250-charging-spec.md`). A0 nói rõ: bảng phụ **không tự động được duyệt** chỉ vì không thêm opcode (`agent/DECISIONS.md:2499-2503`). Cả hai đều **[CHỜ A0 CHỐT]**.

---

## 1. Hiện trạng định dạng (có file:line)

| Điểm | Hiện trạng |
|---|---|
| Tập lệnh | `pub enum Instr` `crates/vis-bytecode/src/format.rs:119-223`; `Call`(tag 8), `CallBuiltin`(9), `CallMethod`(10), `Jump`(11), `JumpIfFalse`(12), `JumpIfNotTrue`(13), `IterNext`(35), `CallValue`(46) tại `format.rs:244-284`. **Không có** `Charge`. |
| Quy tắc tag | "Thêm lệnh ⇒ **thêm tag mới**, không đổi số cũ" `crates/vis-bytecode/src/format.rs:232-234`; tag được map trong `Instr::tag()` `format.rs:235-287`. |
| Bố cục | magic `VBC1` `format.rs:36`; `version: u16` `format.rs:9-10`; mỗi hàm có `n_code + lệnh` rồi `n_spans + span` (bảng song song) `format.rs:13-16`. ⇒ **đã có tiền lệ bảng song song** (span), nhưng chưa có bảng phí. |
| VERSION | `pub const VERSION: u16 = 2` `crates/vis-bytecode/src/format.rs:44`; `decode` so trước khi đọc tiếp `crates/vis-bytecode/src/decode.rs:246-252`; `verify` so `crates/vis-bytecode/src/verify.rs:314-316`. |
| Verifier | `verify(m: &Module)` `crates/vis-bytecode/src/verify.rs:312-315`; mọi bytecode từ cache **luôn** qua `load_verified` `crates/vis-cli/src/cache.rs:10-11`, `crates/vis-bytecode/src/format.rs:24-27`. |
| Codegen | Vòng: `For`→`IterNext` `crates/vis-bytecode/src/codegen.rs:691-701`, `While` `codegen.rs:721-756`, `Loop` `codegen.rs:758-767`; gọi hàm/builtin `codegen.rs:809`, `:839` tương ứng VM `crates/vis-vm/src/vm.rs:809`, `:839`. **Không phát thông tin phí nào**. |
| Cache | Khoá gồm `PHIEN_BAN` `crates/vis-cli/src/cache.rs:23-28`, `:79` và `vis_bytecode::VERSION` `cache.rs:55`, `:61`; có tiền lệ đổi nhãn logic thay vì đổi VERSION cho thay đổi ngữ nghĩa `cache.rs:194-201`. |
| VM fetch | `vm.steps += 1` trước mỗi lệnh `crates/vis-vm/src/vm.rs:736-743` — đây là **hành vi bị VD-250 cấm giữ** (`agent/DECISIONS.md:2497-2498`). |

---

## 2. Hai biểu diễn — so sánh trung lập

**[CHỜ A0 CHỐT] S — Bảng phụ sự kiện (side-table).**
Mỗi hàm có thêm một mảng **song song với `code`**: tại mỗi `pc` có `FeeEntry? = { kind: EventKind, weight: u64, site_id: u32 }` hoặc `None`. Codegen điền entry tại cùng điểm nó phát lệnh tương ứng (`vd249-budget-draft.md:47`). VM đọc `fee[pc]` trong chu kỳ fetch, thay vì `vm.steps += 1` (`vm.rs:736`). **Không thêm opcode**, nhưng **đổi `Module`/`Func`** ⇒ đổi định dạng.

**[CHỜ A0 CHỐT] O — Opcode `Instr::Charge(kind, weight)`.**
Codegen phát một lệnh mới tại mỗi điểm sự kiện; VM thực thi lệnh này như một lệnh bình thường: charge rồi mới thi hành lệnh kế. Lấy **tag mới**, không đổi tag 0..52 (`format.rs:232-234`, `:287`). Cũng đổi định dạng (tập lệnh).

| Tiêu chí | S — bảng phụ | O — opcode `Charge` |
|---|---|---|
| Điểm charge có tự mô tả trong artifact không | Có (`fee[pc]`) | Có (`Charge` nằm trong luồng lệnh) |
| Nhảy tới `pc` khác có tự động mang charge không | Không — phụ thuộc `pc` đích | Có — charge là một lệnh, nhảy qua là mất charge |
| Bề mặt giả mạo | Sửa `weight`, sửa `kind`, xoá entry, dời entry sang `pc` khác | Xoá `Charge`, đổi `weight`, đổi `kind`, đặt `Charge` sai chỗ |
| Kích thước artifact | Mỗi `pc` một ô (có thể nén giống span `format.rs:16`) | Mỗi điểm sự kiện một lệnh + toán hạng |
| VM runtime | Một lần tra mảng/lookup | Một lần match thêm trong vòng lặp lệnh `vm.rs:716-735` |
| Giống cấu trúc hiện có | Có (span đã là bảng song song `format.rs:13-16`) | Có (mọi thứ khác đã là opcode) |
| VERSION/cache | Đổi `Module` ⇒ phải xử lý VERSION/cache `cache.rs:55,61` | Đổi tập lệnh ⇒ phải xử lý VERSION/cache; tag mới `format.rs:232-234` |
| Chưa được A0 duyệt | Đúng (`agent/DECISIONS.md:2499-2503`) | Đúng (`agent/DECISIONS.md:2499-2503`) |

**Khuyến nghị cá nhân (chỉ là đề xuất, [CHỜ A0 CHỐT])**: nghiêng về **S** vì dữ liệu phí tách biệt khỏi luồng điều khiển, dễ bị verifier kiểm chéo với cấu trúc lệnh, và không làm phình vòng fetch; **nhưng A0 có thể chọn O**, và tài liệu này **không** coi việc "không thêm opcode" là lý do tự động duyệt S.

---

## 3. Kiểm tra bắt buộc cho **cả hai** biểu diễn

**Tiền đề chung [CHỜ A0 CHỐT]**: **không tin dữ liệu tính phí trong artifact chỉ vì decode thành công.** `decode` hiện chỉ kiểm magic/version/độ dài (`crates/vis-bytecode/src/decode.rs:237-252`), `verify` kiểm cấu trúc/stack/span/version (`crates/vis-bytecode/src/verify.rs:312-316`); không có khái niệm phí. Bất kể S hay O, phải thêm một **bước kiểm phí trong verifier** với các bất biến dưới đây. Nếu thiếu một bước ⇒ artifact crafted có thể né phí.

### 3.1 Thiếu entry phí (missing fee entry)

| | Kiểm tra đề xuất [CHỜ A0 CHỐT] |
|---|---|
| S | `len(fee) == len(code)`; mọi `pc` thuộc tập điểm sự kiện phải có `FeeEntry`; mọi `pc` không thuộc tập đó phải `None`. Tập điểm sự kiện được **suy từ cấu trúc lệnh** trong cùng hàm, không đọc từ `fee`. |
| O | Với mỗi điểm sự kiện suy được từ cấu trúc, lệnh ngay trước/áp mảnh đó phải là `Charge(kind, w)`; ngược lại `Charge` xuất hiện ở `pc` không phải điểm sự kiện thì từ chối. |

**Cả hai** cần một **định nghĩa điểm sự kiện do verifier tự suy** (derived expectation). Nếu không có cái này, kẻ tạo artifact chỉ cần bỏ entry/`Charge` là né phí — decode vẫn thành công. **[CHỜ A0 CHỐT]** định nghĩa suy: `Call`/`CallBuiltin`/`CallMethod`/`CallValue` ⇒ điểm call/builtin; back-edge/condition (`JumpIfFalse`, `IterNext`, `Jump` lùi) ⇒ điểm loop; lệnh đánh giá biểu thức ⇒ điểm `E_EXPR` (xem giới hạn ở §3.2); `$spawn`/`$await` hạ thành `CallBuiltin` (`vd249-budget-draft.md:27-28`).

### 3.2 Sai vị trí (wrong position)

| | Kiểm tra đề xuất [CHỜ A0 CHỐT] |
|---|---|
| S | `FeeEntry` tại `pc` phải khớp **loại lệnh tại chính `pc` đó**; entry dời sang `pc` khác ⇒ `VerifyError` mới (vd. `FeeWrongPosition`). |
| O | `Charge(kind,w)` phải nằm ngay trước lệnh tạo sự kiện tương ứng; `Charge` giữa hai lệnh không liên quan ⇒ từ chối. |

**Khó khăn đã biết**: interp chèn `E_EXPR` tại `eval` (`crates/vis-interp/src/interp.rs:645`), nhưng codegen hiện **không** phát lệnh riêng cho mọi biểu thức (`format.rs:119-223`: `Bin`, `Un`, `Field`… không mang nhãn "start expr"). Vì vậy:
- Với **O**, codegen phải **tự phát `Charge`** tại mỗi điểm — verifier kiểm được rằng mọi `Charge` là hợp lệ, nhưng **không thể chứng minh không thiếu** nếu không có derived expectation từ HIR. **[CHỜ A0 CHỐT]** cần rule "mọi chuỗi lệnh tính toán phải có ít nhất một charge trước nó" hoặc chấp nhận để codegen (được tin) chịu trách nhiệm đầy đủ, còn verifier chỉ chặn artifact crafted.
- Với **S**, có thể yêu cầu `site_id` tham chiếu tới một bảng site do codegen phát; nhưng bảng site **cũng là dữ liệu artifact** ⇒ không được tin khi chưa kiểm chéo. **[CHỜ A0 CHỐT]**

### 3.3 Entry bị zero hoá / giả mạo (zeroed / tampered)

| | Kiểm tra đề xuất [CHỜ A0 CHỐT] |
|---|---|
| S | `weight` phải **bằng trị số chuẩn** của `kind`; hoặc tốt hơn: **không serialize `weight`**, chỉ serialize `kind`, runtime lấy `w = W[kind]` từ bảng trọng số chuẩn đã chốt (xem `vd250-charging-spec.md` §1 — số chưa duyệt). `FeeEntry { kind: NoCharge }` tại điểm sự kiện ⇒ từ chối. |
| O | `Charge(kind, 0)` ⇒ từ chối (mọi trọng số phải dương); `kind` không khớp lệnh kế ⇒ từ chối; tương tự, có thể chỉ serialize `kind` và suy `w`. |

**Biến thể nguy hiểm**: đổi `kind` sang một mã có trọng số nhỏ hơn mà vẫn "được phép" ⇒ verifier phải so `kind` với **loại lệnh/điểm sự kiện**, không chỉ so `weight > 0`. **[CHỜ A0 CHỐT]** bảng trọng số chuẩn phải được **phiên bản hoá cùng định dạng** — không được đọc từ artifact.

### 3.4 Nhảy có bỏ qua điểm charge không (can a jump skip a charge?)

**Hiện trạng**: đích nhảy là `i32` tương đối `crates/vis-bytecode/src/format.rs:145-151`, `decode` dịch offset `crates/vis-bytecode/src/codegen.rs:97-102`; `while` phát `JumpIfFalse` + `Jump` lùi `codegen.rs:721-756`, `loop` phát `Jump` lùi `codegen.rs:758-767`, `for` dùng `IterNext(i32)` `codegen.rs:701`. Artifact crafted có thể sửa offset để nhảy **tới sau** điểm charge.

**[CHỜ A0 CHỐT] Kiểm tra đề xuất (cả S và O)**: dựng CFG của từng hàm từ lệnh nhảy, rồi áp:
1. **Mọi chu trình trong CFG phải chứa ít nhất một điểm charge.** (Chặn loop không bị tính.)
2. **Không có cạnh vào thân vòng mà bỏ qua charge guard của vòng đó** — đích của mọi `Jump`/`JumpIfFalse`/`IterNext` phải là một `pc` mà đường tới nó trong cùng khối đã đi qua charge, hoặc chính là điểm charge.
3. Đích nhảy phải nằm trong miền `code`; kiểm này đã có ở verifier hiện tại cho span/offset và phải được mở rộng sang tập điểm charge.
4. Với short-circuit (`JumpIfNotTrue`, `format.rs:145-147`, `codegen.rs:900,908`): nhảy bỏ qua toán hạng phải ⇒ **không** có charge cho toán hạng đó (đúng luật `vd250-charging-spec.md` §3.3), nhưng **không được** nhảy qua charge của chính nút điều khiển nếu A0 chọn charge nút đó.

### 3.5 Crafted bytecode có loop không bị tính không

**[CHỜ A0 CHỐT] Kiểm tra đề xuất**: ngoài §3.4, cần hai hàng rào nữa:
1. **Từ entry (`format.rs:14`, `format.rs:529`, `Module.entry`) đến mọi chu trình phải đi qua ít nhất một charge** — phân tích reachability, không chỉ từng chu trình riêng lẻ.
2. **Mọi lệnh gọi đệ quy (đường CFG quay lại `Call` của chính hàm / chu trình qua `Call`) cũng phải chứa charge** — nếu không, đệ quy vô hạn có thể chạy mà không tiêu ngân sách. Đây là chỗ `MAX_STEPS` từng đóng vai trò backstop; VD-250 cấm dùng nó làm giới hạn chương trình thường (`agent/DECISIONS.md:2497-2498`) ⇒ **verifier phải là hàng rào**, watchdog chỉ là cơ chế an toàn riêng, báo rõ (`vd250-charging-spec.md` §5).
3. Nghiệm thu bằng **ca âm** (negative cases) theo quy ước repo: `crates/vis-bytecode/tests/decode_neg.rs`, `crates/vis-bytecode/tests/verify.rs` hiện là nơi đặt test artifact hỏng; mỗi kiểm tra §3.1–§3.5 phải có một crafted artifact tương ứng bị từ chối **trước khi VM chạy**.

### 3.6 Resume có tính trùng không (double-charge)

**Hiện trạng**: frame VM có `pc` `crates/vis-vm/src/vm.rs:124-135`; vòng lặp tiếp tục tại `fr.pc` `vm.rs:716-735`. Đề xuất frame heap lưu `Resume` `vd249-async-design-draft.md:49-51`. Nếu charge nằm ở đầu await/loop, resume có thể nhảy vào đúng điểm đó lần nữa.

**[CHỜ A0 CHỐT] Kiểm tra đề xuất**:
- **Quy ước vị trí**: điểm charge cho `E_AWAIT`/`E_LOOP` nằm **trước** điểm suspend; `pc` lưu lúc suspend phải nằm **sau** điểm charge. Khi đó resume tự nhiên không charge lại — khớp `vd250-charging-spec.md` §3.5.
- **Kiểm verifier/test**: với mọi `pc` có thể là resume target, assert `pc` không trùng `pc` của charge site đã thi hành trong lần chạy trước đó của cùng frame. Nếu A0 chọn giữ cờ `charged` trong frame thay vì dời `pc`, cờ đó phải là **trạng thái nội bộ**, không được đọc từ artifact.
- **Cả S và O** đều phải có test: A chạy→suspend→B chạy→A resume, so dãy `(mã, w)` trong trace; dãy không được có `E_AWAIT` thứ hai cho cùng một biểu thức.

---

## 4. Đối chiếu với 5 câu hỏi tính phí

Mọi quy tắc ở `vd250-charging-spec.md` §3 phải **biết dịch được sang cả hai biểu diễn**; nếu một quy tắc chỉ diễn được bằng bảng phụ hoặc chỉ bằng opcode, A0 phải chọn lại quy tắc, không chọn biểu diễn cho phù hợp.

| Quy tắc (`vd250-charging-spec.md`) | S diễn thế nào | O diễn thế nào |
|---|---|---|
| `E_CALL` không cộng `E_EXPR` (§3.1) | `fee[pc_call]` = `Call`; đối số có entry riêng | `Charge(Call)` ngay trước `Instr::Call`; không có `Charge(Expr)` thừa |
| Builtin chỉ `E_BUILTIN` (§3.2) | `fee[pc]` = `Builtin` tại `CallBuiltin` | `Charge(Builtin)` trước `Instr::CallBuiltin` |
| Short-circuit (§3.3) | Entry nằm ở đầu toán hạng phải, **sau** `JumpIfNotTrue` | `Charge(Expr)` nằm sau nhánh nhảy của `JumpIfNotTrue` |
| Loop kể cả guard sai (§3.4) | Entry tại `pc` guard / back-edge / `IterNext` | `Charge(Loop)` trước mỗi guard/back-edge/`IterNext` |
| Resume không tính lại (§3.5) | `pc` resume > `pc` entry | `pc` resume > `pc` của `Charge(Await)` |

---

## 5. VERSION / cache — **không ấn định lúc này**

**Hiện trạng**: `VERSION = 2` `crates/vis-bytecode/src/format.rs:44`; `decode` `decode.rs:251-252` và `verify` `verify.rs:315-316` từ chối version lệch; khoá cache đưa `VERSION` `crates/vis-cli/src/cache.rs:55`, `:61` và `PHIEN_BAN` `cache.rs:23-28`, `:79`; có tiền lệ **chỉ đổi `PHIEN_BAN`** cho thay đổi ngữ nghĩa mà không đổi VERSION `cache.rs:194-201`.

**A0 đã chốt** (`agent/DECISIONS.md:2501-2503`): sau khi **chọn biểu diễn**, mới cấp **phiên bản định dạng chưa dùng** + cập nhật cache; **chưa ấn định VERSION 3 lúc này**.

**[CHỜ A0 CHỐT] Đề xuất trình tự** (không thực hiện ở ST-266):
1. A0 chọn S hoặc O (hoặc phương án khác).
2. Xác định đây là **đổi bố cục** (S: thêm mảng; O: thêm tag) ⇒ đề xuất tăng `VERSION` lên một giá trị **chưa từng dùng**, không tự gán 3; nếu A0 muốn đi theo tiền lệ `PHIEN_BAN` (`cache.rs:194-201`), phải nêu rõ lý do vì sao đổi bố cục mà không đổi VERSION.
3. Cập nhật `encode`/`decode`/`verify` (`vd249-budget-draft.md:80`) **cùng** bảng trọng số chuẩn đã chốt.
4. Thêm ca âm: artifact có phí bị xoá/zero/dời/nhảy qua/resume-trùng bị **từ chối ở verify**, không phải lúc chạy.
5. Cache cũ tự vô hiệu qua `VERSION` và/hoặc `PHIEN_BAN`; không đổi `MAGIC` (`format.rs:36`) và không đổi tag cũ (`format.rs:232-234`).

---

## 6. Câu hỏi gửi A0

| # | Câu hỏi | Khuyến nghị |
|---|---|---|
| R1 | Chọn **S** hay **O** (hai biểu diễn của cùng hợp đồng)? | Nghiêng S, nhưng không coi "không thêm opcode" là tự động duyệt. |
| R2 | Có chấp nhận chi phí verifier CFG (§3.4/§3.5) cho cả hai không? | Bắt buộc; nếu không, crafted bytecode né phí. |
| R3 | Serialize `weight` hay chỉ `kind` + bảng chuẩn (§3.3)? | Chỉ `kind`; suy `w` từ bảng chuẩn có phiên bản. |
| R4 | Bảng trọng số chuẩn được chốt riêng với định dạng không (VD-250 `agent/DECISIONS.md:2493`)? | Chốt sau C8 của `vd250-charging-spec.md`. |
| R5 | Dùng quy ước "charge trước suspend, resume `pc` sau charge" hay cờ `charged` trong frame (§3.6)? | Quy ước `pc`; cờ chỉ là phương án dự phòng. |
| R6 | VERSION mới là bao nhiêu, và có dùng thêm `PHIEN_BAN` không (§5)? | Quyết **sau** R1; không gán 3 ngay. |

---

## 7. Giới hạn phạm vi

Không thêm `Instr::Charge`, không sửa `format.rs`/`encode.rs`/`decode.rs`/`verify.rs`/`codegen.rs`/`cache.rs`, không đổi `VERSION`, không commit. Mọi nội dung trên là **[CHỜ A0 CHỐT]**.
