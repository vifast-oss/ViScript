<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-250 — review Agent 1 (global chưa khởi tạo ⇒ trap, VD-245)

**Nguồn**: worktree `vd245-st250-init-trap`, commit Agent 2 `4285313`. **Người review**: Agent 1. **Ngày**: 2026-11-20.

## 1. Đối chiếu hợp đồng VD-245

| Yêu cầu | Kết quả | Verdict |
|---|---|---|
| Phân biệt trạng thái khởi tạo với `Unit` | `Vec<Option<Value>>`; `None` = chưa khởi tạo (sentinel nội bộ, không lọt `Value`/`show`/toán tử/serialization), `Some(Unit)` = đã ghi `Unit` hợp lệ | **Đạt** |
| Mọi đường đọc global qua **một** kiểm tra | `doc_global(k, span)` — `Some(Some(v))` trả `v`, else trap | **Đạt** |
| Chưa khởi tạo ⇒ trap chẩn đoán, exit ≠ 0 | trap `RT` dùng **`VS0501`** (mã hiện hành, không thêm mã mới) + thông điệp “global chưa được khởi tạo tại ô … (chỉ số bytecode …)”, exit 1 | **Đạt** |
| Chỉ mark đã khởi tạo sau ghi thành công | `globals[k] = Some(v)` sau `pop` value thành công; ô vượt kho trap `BadIdx` | **Đạt** |
| Reset theo vòng đời / không rò | `init_globals()` đầu `run()`, reset `None`, cấp phát từ lệnh ghi trong module đã verify — **không resize theo chỉ số lúc chạy** | **Đạt** |
| Không đổi verifier/codegen/opcode/bố cục | Đúng | **Đạt** |

## 2. 8 test nghiệm thu — 8/8 PASS

Nhánh không chạy / hàm chưa gọi ⇒ trap; gọi hàm ghi ⇒ `42`; `Unit` hợp lệ ⇒ `()`; 3 global chỉ ghi 0 và 2 ⇒ ô 1 trap; ngoài miền ⇒ verify chặn; `encode→decode→load_verified→execute` ⇒ trap; hai lần chạy không rò.

## 3. Lệch phạm vi tệp — Agent 1 chấp nhận

Ngoài `vm.rs` + test mới, Agent 2 sửa **2 assertion trong `tests/st247_global_domain.rs`** (đổi tên + assert trap thay vì `show == "()"`). Đây là **bắt buộc**: tệp ST-247 assert chính hành vi lỗi mà VD-245 cấm; nếu giữ nguyên gate sẽ đỏ. Agent 1 **đồng ý** với lý do này — ghi rõ để truy nguyên.

## 4. Finding riêng (không tự sửa, báo mức độ)

1. **Trung bình** — `Instr::LoadCaptured(i)` vẫn `get(i).unwrap_or(Unit)` và verifier (`verify.rs:449`) **không kiểm biên capture**. Crafted bytecode có thể đọc capture ngoài phạm vi thành `Unit` thành công — cùng lớp lỗi, nhưng là đường **local/capture**. Ghi nợ riêng **#189**; **không tuyên bố đường này an toàn** chỉ vì global đã sửa.
2. **Thấp** — `LoadLocal` nhánh local (non-global) cũng `unwrap_or(Unit)`, nhưng verifier chặn `s ≥ nlocals` và khung khởi tạo sẵn `Unit` ⇒ defense-in-depth. Ghi nhận trong #189.

## 5. Kết luận

**SHIP ST-250.** Merge `4285313`, đóng #188/`BLOCKED.md` §14, cập nhật VD-245 (mã `VS0501` thực tế), mở #189. **Sau đó Agent 1 bump `PHIEN_BAN`/re-pin cache** để bản cuối gồm ngữ nghĩa global mới (VD-245 §5). Chưa tag/release.
