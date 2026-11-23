<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-245 — review Agent 1 (A0 bổ sung sau ST-238)

**Nguồn**: worktree `vd242-st245-boundary`, commit Agent 2 `3ceb041`. **Người review**: Agent 1. **Ngày**: 2026-11-20.
**Phạm vi**: chỉ bảo vệ bản sửa ST-238; không mở lại ST-238, không đổi VBC1/opcode/MAGIC/VERSION.

## 1. Bốn câu xác nhận A0 — verdict

| Câu hỏi A0 | Kết quả | Bằng chứng Agent 2 / review |
|---|---|---|
| Slot local không va miền `GLOBAL_SLOT_BASE` | **CÓ** | Verifier thêm điều kiện `f.nlocals > GLOBAL_SLOT_BASE ⇒ BadArity`; biên `nlocals == 0x8000` (ô cao nhất `0x7FFF`) vẫn hợp lệ — test `nlocals_bang_global_base_la_bien_bi`. |
| Chỉ số global kiểm với số global thực tế | **CÓ** | VBC1 không có `nglobals` ⇒ verifier **suy ra từ lệnh ghi** (`StoreLocal`/`Assign`/`BindLocal`) trong toàn module; mọi `s - BASE >= nglobals` bị từ chối — test `doc_global_vuot_so_thuc_te_bi_tu_choi`, `khong_co_global_ma_doc_global_bi_tu_choi`. |
| Bytecode sai không vượt kiểm tra / không panic | **CÓ** | Thêm `VerifyError::GlobalOverflow`; chặn cả `verify()` lẫn `load_verified()` (encode→decode→verify); `Vm::run` verify trước ⇒ chẩn đoán sạch. Test `catch_unwind` xác nhận không panic. |
| Ảnh hưởng tương thích được ghi rõ | **CÓ** | `format.rs::GLOBAL_SLOT_BASE` mục “*Ảnh hưởng tương thích (ST-245)*”: dù không đổi opcode/MAGIC/VERSION/bố cục, toán hạng `>= 0x8000` có **ngữ nghĩa mới**; VM cũ hiểu thành ô local `32768+k` ⇒ **kết quả sai, không crash**; khoá cache không đổi nên verifier là hàng rào; `VerifyError` thêm biến thể ⇒ đổi bề mặt chẩn đoán. **Không** viết “không đổi hợp đồng”. |

## 2. Lỗ hổng verifier đã phát hiện — phải báo A0 ngay

**Trước ST-245**, miễn trừ `LocalOverflow` của ST-238 chấp nhận **mọi** ô `>= 0x8000`. Repro tối thiểu: module 1 hàm, `code = [LoadLocal(0xFFFF), Return]`, không có lệnh ghi global ⇒ verifier cũ `Ok(())`, VM đọc `Unit` (và có thể `resize` khung ~32768 ô). Sau ST-245 ⇒ `Err(GlobalOverflow { slot: 0x7FFF, nglobals: 0 })`.

Agent 2 xử lý **trong quyền được cấp** (`verify.rs`), không sửa `vm.rs`. **Đây là lỗi đã đóng trong ST-245, không còn blocker mở** — nhưng A0 yêu cầu báo ngay khi phát hiện, nên ghi rõ ở đây và trong bàn giao.

## 3. Test / gate

- `crates/vis-bytecode/tests/st245_boundary.rs`: **9 test** (6 âm + 3 biên), dựng `Module` tay, không phụ thuộc codegen.
- `crates/vis-vm/tests/st245_async_regression.rs`: **4 test**, trong đó `async_task_doc_var_cap_tep_khong_tra_unit` tái hiện dạng “10k task VM trả `()`”. Chứng minh đỏ trên bản cũ: đảo `slot_cua → f.slot` ở nhánh `HStmt::Var` ⇒ 4/4 fail (`()` / trap `không so sánh được () và 3` / bytecode hỏng lọt verifier). Phép đo **10k nguyên vẫn ở `vis-cli/tests/async_10k.rs`**, không sửa.
- `./vis-dev check` PASS; `cargo test -p vis-bytecode -p vis-vm` PASS — 320 ca parity + 2.000 chương trình sinh vẫn qua (hardening không từ chối bytecode codegen thật).

## 4. Ghi nợ sổ sách (Agent 1 xử lý)

`STATE.md` có hai chỗ ghi **13 biến thể `VerifyError`** (các khối lịch sử). Nay là **14** (thêm `GlobalOverflow`). Giữ số lịch sử, thêm **addendum** ở header; không viết lại khối cũ.

## 5. Kết luận

**SHIP.** Merge `3ceb041`, cập nhật QUEUE ST-245 → DONE, addendum `VerifyError` 14 biến thể. Không mở VD mới; không đổi VBC1; không đụng `vm.rs` của ST-239.
