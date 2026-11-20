<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Kế hoạch tích hợp cuối tuần — VD-257 (leader: Agent 1)

## 1. Base & worktree

- Base ban đầu: `fc297ba2064fe9a598190177914644e7d9859ade`.
- `main` sau khi ghi quyết định: `41b17b2bf7e6e4cfe0d0e869cfd11ccd981f0600` (chỉ tài liệu VD-257/QUEUE).
- Worktree: `vd257-a-coverage`, `vd257-b-budget`, `vd257-c-lifecycle`, `vd257-d-fairness`
  (đều nhánh `work/vd257-*`, từ `fc297ba`). Tối đa 4 — đúng giới hạn A0.

## 2. Phân quyền tệp (đã giao trước khi dispatch)

| Luồng | Sở hữu | Không đụng |
|---|---|---|
| A | `vis-interp/src/frame/**`, `vis-vm/src/vm_frame.rs`, `lat257_a_coverage.rs` | `rt.rs`, `interp.rs`, `stdlib.rs`, `vm.rs`, bytecode, `cache.rs` |
| B | `vis-bytecode/**`, `cache.rs`, `interp.rs`, `vm.rs`, `lat257_b_*` | `rt.rs`, `frame/**`, `vm_frame.rs`, `stdlib.rs` |
| C | `rt.rs`, `stdlib.rs`, `net.rs`, `lat257_c_lifecycle.rs` | `frame/**`, `interp.rs`, `vm*`, bytecode, `cache.rs` |
| D | **Chỉ phân tích** (journal + `/tmp/vd257-d/`) | mọi source — tránh va A/C ở phase 1 |

## 3. Thứ tự tích hợp (không cherry-pick mù)

1. **A** → merge vào `main`, chạy gate đầy đủ **một làn**.
2. **C** → rebase/merge lên base mới, gate. (`rt.rs` không va A.)
3. **B** → rebase/merge, gate. (`interp.rs`/`vm.rs` đã được C/A giữ nguyên; đối chiếu diff.)
4. **D phase 2** (nếu được duyệt trong phạm vi) → chỉ sau khi A+C vào, vì cần `rt.rs`/`frame/**` ổn định.
5. Nếu conflict: dừng luồng đó, review chéo từng hunk, không tự ý chọn bản "xanh hơn".

## 4. Gate & tài nguyên (VD-257 §5)

- Build nặng / gate đầy đủ / benchmark: **một làn**, không chồng với agent khác.
- Sub-agent: **targeted test + fmt** only; không full gate, không benchmark, không `cargo clean`.
- Không ghi đè fixture/log luồng khác; thư mục tạm riêng; theo dõi đĩa/RAM.
- Không chạy lại full gate trên cùng commit chỉ để tăng số PASS.
- Fuzz/soak: seed + thời lượng hữu hạn, lưu seed lỗi.

## 5. Tiêu chí dừng

- Blocker ngoài phạm vi: đóng băng task riêng, chuyển task khác.
- Đổi cúyntax/API công khai, GC/worker/shared heap, sandbox, dependency lạ, hạ tiêu chí, đổi runtime
  mặc định, chính sách lỗi/cancellation chưa quyết, cạn đĩa/RAM ⇒ **dừng phần đó**.
- **08:00 JST 23/11**: không mở task lớn mới. **09:00 JST**: dừng mở rộng, báo cáo, cây xác định.

## 6. Báo cáo cuối (một bản duy nhất)

HEAD/remote/trạng thái cây/worktree · task xong/mở/chặn · coverage frame còn thiếu chính xác · 4 trục
VERSION + bằng chứng tương thích · ngân sách/fuel/Q (thử nghiệm vs chưa chốt) · B1/B2/latency/tài
nguyên sống + môi trường · gate trên đúng commit · đề xuất bước tiếp theo — **không tự kết luận đổi
runtime mặc định**.
