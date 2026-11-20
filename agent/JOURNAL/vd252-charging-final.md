<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-252 §2 · Bảng tính phí CHỐT (5 quy tắc, có chỉnh lý)

**Trạng thái**: ghi nhận quyết định của A0 — **research + draft record only**, không áp vào mã,
không commit, không chạy gate. Thay thế phiếu đề xuất `vd251-charging-short.md` (nay đã được chốt,
có chỉnh lý). **Căn cứ duy nhất**: `agent/DECISIONS.md:2634-2649` (VD-252 §2).

---

## 1. Năm quy tắc đã chốt

| # | Quy tắc (đã chốt) | Hệ quả hiện thực | Ví dụ |
|---|---|---|---|
| 1 — Call/expression | Nút gọi chỉ tính **`E_CALL`**, **không** cộng `E_EXPR` cho chính nút đó. Callee và đối số đánh giá theo **thứ tự ngôn ngữ**, tính phí riêng **chỉ khi thực sự chạy**. | `E_CALL` thay chỗ `E_EXPR` của nút gọi; mỗi con (callee/đối số) phát sự kiện riêng tại điểm nó chạy. Không đếm đôi nút gọi. | `f(g(1))` ⇒ `E_CALL(f)` + `E_CALL(g)` + `E_EXPR(1)`; **không** có `E_EXPR` cho dấu `(` của `f` |
| 2 — Builtin | Chỉ **`E_BUILTIN`**, **không** cộng `E_CALL`. Tính **một lần, TRƯỚC khi thực hiện** builtin; `Pending`/resume **không tính lại**. | Điểm charge chung tại dispatcher `vis_interp::builtin` (`vd250-charging-spec.md:64-77`, VM gọi cùng dispatcher `vm.rs:851`) ⇒ hai engine cùng dãy sự kiện. | `print(str(1))` ⇒ `E_BUILTIN(print)` + `E_BUILTIN(str)`; không `E_CALL` |
| 3 — Short-circuit | Chỉ **toán hạng thực sự chạy** được tính. Nếu nút `&&`/`||` có phí riêng thì quy định **nhất quán**; **không được mất phí chỉ vì VM hạ thành lệnh nhảy**. | Charge ở đầu mỗi toán hạng, **sau** lệnh nhảy (`JumpIfNotTrue` `format.rs:145-147`, `codegen.rs:900,908`) — không charge nút nhảy thay cho toán hạng, cũng không để nhánh bị bỏ mang phí. | `false && f()` ⇒ chỉ `E_EXPR(false)`; `f()` không bị charge. `true \|\| f()` tương tự |
| 4 — Loop | **`E_LOOP` một lần cho mỗi lần thử điều kiện / tiến vòng, GỒM lần kết thúc (lần cuối sai)**. Điều kiện và thân vẫn tính sự kiện riêng khi thực thi. Với `for`/`while let` phải **ánh xạ cụ thể** vào "thử tiến vòng", **không ép mọi loop thành một dạng** rồi charge không đều. | Ánh xạ bắt buộc (xem §2): `while` → guard; `loop` → back-edge; `for` → `IterNext`; `while let` → lần thử khớp pattern. Thân không có `E_LOOP` riêng; câu trong thân trả sự kiện của nó. | `while i < 3` ⇒ 4 `E_LOOP` (3 đúng + 1 sai); `for i in [1,2,3]` ⇒ 4 (3 lấy phần tử + 1 hết) |
| 5 — Await | **Một `E_AWAIT` cho mỗi lần bắt đầu await trong source.** Resume **không tính lại**; await **lần hai là sự kiện mới, kể cả dùng cache**. | Điểm charge nằm **trước** điểm suspend; continuation lưu pc/work sau điểm đó (`frame.rs:614-624`; `vm_frame.rs:41-48`) ⇒ resume tự nhiên không charge lại. Cache `rt.rs:180-194` không miễn phí cho lời gọi `await` mới. | `let a = await t; let b = await t` ⇒ 2 `E_AWAIT`; suspend/resume của `a` không cộng thêm |

---

## 2. Ánh xạ "thử tiến vòng" (bắt buộc, không gộp)

| Dạng vòng | Điểm `E_LOOP` | Lần cuối | Ghi chú |
|---|---|---|---|
| `while <cond>` | Mỗi lần đánh giá guard (`JumpIfFalse`, `codegen.rs:721-756`) | **Có** — guard sai thoát vòng vẫn tính | Điều kiện tự charge `E_EXPR` riêng |
| `loop` | Mỗi lần **thực thi back-edge** (`Jump` lùi, `codegen.rs:758-767`) | **Có** — back-edge cuối vẫn là một lần tiến vòng | Không tạo guard giả chỉ để charge. `break` thoát **trước** back-edge ⇒ không có lần tiến vòng đó; phải định nghĩa thống nhất cho hai engine (điểm cần chốt khi hiện thực) |
| `for pat in iter` | Mỗi lần `IterNext` (`codegen.rs:691-701`, VM `vm.rs:1088`) | **Có** — lần báo hết phần tử | Không bắt `for` đi qua `while`-guard; `pat` bind theo sự kiện riêng |
| `while let PAT = EXPR` | Mỗi lần thử khớp `PAT` để quyết tiến vòng (`HCond::Let`, `hir.rs:430-433`) | **Có** — lần thử không khớp vẫn tính | Pattern binding/so khớp là công riêng; không gộp vào `while` thường rồi charge lệch |

> Cấm giữ nguyên `guard > 10_000_000` (`interp.rs:541`) / `for _ in 0..10_000_000` (`interp.rs:559`)
> làm ngưỡng song song với `charge()` — VD-250 đã cấm (`agent/DECISIONS.md:2497-2498`;
> `vd250-charging-spec.md:116`). Mọi hàng rào phải đi qua một điểm charge chung.

---

## 3. Thứ tự — điểm dễ sai nhất

`f(g(1))` **không** được tính phí gọi `f` trước khi đánh giá `g(1)` (`agent/DECISIONS.md:2644-2645`).
Phải giữ **thứ tự đánh giá của ngôn ngữ** và đặt **kiểm ngân sách ngay trước tác dụng phụ tương ứng**:

1. Đánh giá callee/đối số theo thứ tự nguồn; mỗi cái charge khi nó thực sự chạy.
2. Charge `E_CALL` ngay trước khi mở khung hàm (không trước khi đánh giá đối số).
3. Charge `E_BUILTIN` ngay trước khi gọi host; nếu builtin báo `Pending`, resume không charge lại.
4. `charge(e, w)` là nguyên tử: `w > remaining` ⇒ trap, **không trừ, không thi hành**;
   `w == remaining` ⇒ trừ về 0 rồi thi hành, sự kiện sau trap; **không rollback** tác dụng phụ đã xong
   (`vd250-charging-spec.md:40-44`; nguyên tắc "kiểm trước tác dụng phụ" `vd251-charging-short.md:6`).

Hai engine phải sinh **cùng dãy `(mã sự kiện, vị trí)`**; cùng tên sự kiện mà số lần khác nhau ⇒ parity hỏng
(`agent/DECISIONS.md:2495-2496`; `vd251-charging-short.md:17-18`).

---

## 4. CHƯA được duyệt / vẫn mở (không được nap dưới bảng trên)

| Việc | Trạng thái | Căn cứ |
|---|---|---|
| **Trọng số** | **Chưa có full catalog** — đặc biệt `E_STMT`, assignment, expression tổng hợp; bảy số cũ `1/1/1/4/2/4/2` **chưa duyệt**. Không ấn số lúc này | `agent/DECISIONS.md:2647`; `vd250-charging-spec.md:6,164-175` |
| **Biểu diễn bytecode** | **Chưa chọn** bảng phụ (side-table) hay `Instr::Charge` — hai biểu diễn của cùng hợp đồng; verifier CFG chặn artifact né phí vẫn bắt buộc | `agent/DECISIONS.md:2647-2648`; `vd250-fee-repr.md:25-44,48-50` |
| **`VERSION` / cache trong Lát 2** | **Không đổi** — Lát 2 không ấn VERSION/cache, không đổi artifact | `agent/DECISIONS.md:2648` |
| **Ngân sách mới vs reactor** | Ngân sách mới **KHÔNG phải prerequisite** của reactor — không chặn Lát 2 vì chưa có bảng trọng số | `agent/DECISIONS.md:2648` |
| **Step-cap parity** | **Finding vẫn MỞ** (VD-249/ST-260): 2M vòng ⇒ interp exit 0 / VM trap `VS0506→VS0501`; **không sửa tạm bằng tăng ngưỡng**, không tuyên bố parity cho workload chạm giới hạn | `agent/DECISIONS.md:2649`; `vd251-lat1-report.md:108-111` |
| **B2 / fairness CPU** | Vẫn đói CPU — ghi rõ chưa sửa, **không đổi tiêu chí** để gọi là đạt fairness | `agent/DECISIONS.md:2682`; `vd251-lat1-report.md:123-129` |
| **Watchdog nội bộ** | Chưa duyệt; nếu có phải khác cơ chế với hết ngân sách, báo rõ tên/nguồn, không che crafted bytecode | `vd250-charging-spec.md:156-160` |

---

## 5. Phạm vi

Tài liệu này chỉ ghi nhận 5 quy tắc A0 đã chốt và những gì còn mở; không sửa `interp.rs`, `vm.rs`,
`builtin.rs`, `rt.rs`, bytecode/verifier/cache, không thêm opcode `Charge`, không chạy gate. Việc hiện
thực hóa trọng số + biểu diễn chỉ tiến hành sau VD mới của A0.
