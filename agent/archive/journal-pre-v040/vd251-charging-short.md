<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Bảng tính phí ngắn — 5 câu A0 (VD-251 §5)

> Chỉ là đề xuất **chờ A0 chốt**; **không** áp dụng trong Lát 1. Lát 1 giữ hàng rào hiện hành và ghi
> step-cap vẫn mở. Nguyên tắc đã chốt: chỉ tính sự kiện thực thi; short-circuit không tính nhánh bị bỏ;
> resume không tính lại phần đã chạy; một pool cho toàn lần chạy; hạn ngạch lượt chạy là bộ đếm riêng;
> kiểm trước tác dụng phụ. **Chưa duyệt** trọng số/cộng chồng/biểu diễn bytecode.

| # | Câu hỏi | Đề xuất | Ví dụ tính phí |
|---|---|---|---|
| 1 | `E_CALL` có tính thêm `E_EXPR` cho chính nút gọi không? | **Không.** Nút gọi tính **một** `E_CALL`; các **đối số** được đánh giá riêng theo sự kiện của chúng. Tránh đếm đôi nút gọi. | `f(g(1))` ⇒ `E_CALL(f)` + `E_CALL(g)` + `E_EXPR(1 literal nếu có)`; **không** cộng `E_EXPR` cho dấu `(` của `f`. |
| 2 | Builtin có chịu cả `E_CALL` lẫn `E_BUILTIN` không? | **Chỉ `E_BUILTIN`.** Builtin là ranh giới host; tính một lần tại dispatcher dùng chung (`vis_interp::builtin`) để hai engine cùng điểm. | `print(str(1))` ⇒ `E_BUILTIN(print)` + `E_BUILTIN(str)`; **không** `E_CALL`. |
| 3 | Short-circuit chỉ tính nhánh thực sự chạy — thế nào? | Charge **ở đầu mỗi toán hạng sau lệnh nhảy**. Nhánh bị nhảy qua không có điểm charge ⇒ không tính. | `false && f()` ⇒ chỉ `E_EXPR(false)`; `f()` **không** bị charge. `true || f()` tương tự. |
| 4 | Loop tính ở kiểm điều kiện, vào thân, hay cả hai? | **Một `E_LOOP` mỗi lần đánh giá điều kiện, kể cả lần cuối sai.** Thân không có charge riêng (các câu trong thân tự tính). Đây là điểm thay thế guard cũ của interp và đếm lệnh của VM bằng một sự kiện chung. | `while i < 3` với thân `i = i + 1` ⇒ `E_LOOP` ×4 (3 lần đúng + 1 lần sai) + mỗi lần `E_STMT`/`E_EXPR` của thân. |
| 5 | Await resume có tính lại phí bắt đầu `await` không? | **Không.** `E_AWAIT` tính **một lần khi bắt đầu**; resume tiêu thụ continuation đã lưu, không charge lại. `await` **thứ hai** (kể cả kết quả cache) là sự kiện mới ⇒ tính lại. | `let a = await t; let b = await t` ⇒ `E_AWAIT` ×2; suspend/resume của `a` **không** cộng thêm. |

**Hai engine cùng tên sự kiện ⇒ cùng số lần.** Không được interp tính `E_LOOP` ở thân còn VM tính ở
điều kiện; mọi điểm charge phải nằm ở định nghĩa sự kiện chung, không phải ở opcode riêng của engine.

**Chưa chốt**: trọng số (`1/1/1/4/2/4/2` chỉ là đề xuất cũ), quy tắc cộng chồng, bảng phụ hay
`Instr::Charge`, `VERSION`, và watchdog nội bộ (phải phân biệt với hết ngân sách ngữ nghĩa).
