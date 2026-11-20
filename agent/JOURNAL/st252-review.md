<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-252 — review Agent 1 (capture/local ngoài miền trap, VD-246)

**Nguồn**: worktree `vd246-st252-capture-local`, commit Agent 2 `7a62861`. **Người review**: Agent 1. **Ngày**: 2026-11-20.

## 1. Đối chiếu hợp đồng VD-246

| Yêu cầu | Kết quả | Verdict |
|---|---|---|
| Verifier kiểm quan hệ hàm đích/số capture/`MakeClosure` | Giữ nguyên `ClosureFree` sẵn có (`nfree ≥ max(LoadCaptured)+1` của hàm đích); thay khối no-op bằng ghi chú giới hạn. Đúng yêu cầu “kiểm trong khả năng metadata hiện có” | **Đạt** |
| Runtime kiểm capture/khung thực tế, ngoài miền trap | `doc_local` / `doc_captured` / `ghi_local`; trap `VS0501` (mã hiện hành), thông điệp kèm chỉ số + độ dài thực tế; bỏ `resize` âm thầm | **Đạt** |
| Không trả `Unit` mặc định; `Unit` hợp lệ vẫn trả `Unit` | Có test riêng: capture chứa `Unit` chạy sạch, local `Unit` chạy sạch | **Đạt** |
| Không đổi local sang sentinel / không thêm luật đọc-trước-ghi | Đúng | **Đạt** |
| Không đổi VERSION/opcode/bố cục, không phân tích liên hàm | Đúng | **Đạt** |
| Mỗi ca ghi rõ verifier hay runtime | 8 ca đều ghi rõ; có ca verifier lọt (entry + `encode→decode→load_verified`) ⇒ runtime trap | **Đạt** |
| Chứng minh hết thành công giả | Mọi ca trap khẳng định `!is_clean`, `exit≠0`, `value=None`, `VS0501`, thông điệp cụ thể | **Đạt** |

R1 `vm.rs` 1491 dòng (≤1500). Không đổi `format.rs`/`cache.rs`/codegen/cli.

## 2. Finding riêng — ghi sổ, không tự sửa

1. **`pop_frame` writeback silent skip** (`vm.rs:686`): `if let (Some(dst), Some(src)) = ...` bỏ qua khi ô mirror ngoài miền. **Khác ngữ nghĩa** (ghi ngược trạng thái closure, không phải đọc trả `Unit`) và **không thể xảy ra** với bytecode đã verify + thực thi vì mọi `LoadCaptured(i)` trong prologue mirror đã trap trước writeback. Đổi `pop_frame` (`Option<Frame>` → `R<…>`) là refactor 3 call-site, vượt phạm vi “không viết lại VM”. Ghi **#190** / `BLOCKED.md` §18.
2. **Giới hạn verifier đã ghi rõ trong code**: hàm `entry`/gọi thẳng bằng `Call` không có `MakeClosure` để so ⇒ verifier lọt, runtime là hàng rào cuối; `captured_used: u16` ⇒ `MakeClosure(fi, u16::MAX)` lọt ở mép, runtime vẫn trap. Không mở phân tích liên hàm (VD-246 §1).

## 3. Quyết định cache (A0 yêu cầu đánh giá, không mặc định)

**Không bump `PHIEN_BAN` lần nữa.** Căn cứ:
- Không đổi format/opcode/`VERSION`/codegen ⇒ artifact trong cache **byte-identical và hợp lệ**; `ghi()` chỉ lưu module đã `verify`, bytecode crafted không lọt cache.
- Không có **chương trình hợp lệ nào đổi kết quả**: đọc local chưa ghi vẫn `Unit` (VD-246 cấm thêm luật), capture `Unit` vẫn `Unit`; khác ST-249/ST-250 vốn đổi output trên chương trình hợp lệ.
- Hành vi mới chỉ chặn đường **ngoài miền** mà compiler không sinh.

Đây là căn cứ đúng đắn, không phải mặc định “hậu tố ST-250 đã bao phủ”. Nếu A0 muốn traceability thuần túy có thể thêm nhãn, nhưng VD-246 không bắt buộc; Agent 1 chọn **không bump** và ghi căn cứ.

## 4. Kết luận

**SHIP ST-252.** Merge `7a62861`, đóng #189/§17, mở #190/§18, ghi quyết định cache. Đây là ứng viên tích hợp; chưa tag/nghiệm thu phát hành cuối.
