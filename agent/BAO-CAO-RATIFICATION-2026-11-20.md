<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Báo cáo ratification — trình Agent 0 (ST-244), 2026-11-20

> **Mục đích**: phân xử các ngữ nghĩa từng mang nhãn “proxy/A0-uỷ-quyền” trước nghiệm thu cuối.
> **Nguyên tắc A0**: không xin duyệt lại phần đã chốt; chỉ đưa các ngữ nghĩa **chưa có quyết định** thuộc phạm vi phát hành.
> **Bằng chứng**: `agent/DECISIONS.md` VD-215..VD-246; không chạy lại test chỉ để tăng độ tin cậy.

## 1. Kết luận ngắn

**Không còn ngữ nghĩa nào “chưa có quyết định” trong phạm vi phát hành.** Mọi mục từng là proxy đã được **VD-241 ratify toàn bộ VD-215..VD-240**; các phần bị loại/supersede đã có VD riêng và nằm ngoài phạm vi phát hành. ST-244 vì thế chỉ cần **đóng sổ ratification**, không trình A0 ký lại.

## 2. Bảng phân xử

| Nhóm | VD / mục | Hành vi đã chọn | Căn cứ quyết định | Trạng thái ratification | Việc cần A0 |
|---|---|---|---|---|---|
| Pha 1 lõi | VD-221..VD-224 (R4a/b/c, R5b′, R5d) | `trait Error: Show` 3 phương thức; `source: Error?`; `span: str?`; `Json` EnumDef thật + `json.decode -> Result[Json, Error]`; `AnyError` = alias bản ghi `Error` **theo VD-231**; std fns → `Result[_, Error]`; `--b` hợp lệ mọi hàm (VD-224 huỷ VD-223(i)); `VS0211` giữ | VD-221/222/223/224, amendment VD-231, ratify VD-241 | **Đã ratify VD-241**. Không revert. | **Không** |
| `while let` | VD-226 / ST-226 | `while let PAT = EXPR:` dùng chung `HCond::Let`; pattern không khớp ⇒ thoát vòng; interp/VM byte-identical; `vd_226.rs` 8/8 | VD-226, ratify VD-241 | **Đã ratify VD-241** | **Không** |
| Option lift | VD-227 / VD-228 / VD-229 | `let` annotate tự nâng `T→T?`; function-return + argument + closure-**biên** lift; closure **params không nâng** (VD-232 đưa #155 sang VS3) | VD-227/228/229, VD-232, ratify VD-241 | **Đã ratify VD-241** (VD-229 *bỏ phần closure-params*) | **Không** |
| Host patches | VD-186 (2 patch) | `HOST-BAO-DAM.md` + `docs/ViScript-thiet-ke.md` mô tả mã hiện tại, nhãn thiết kế VS3 | VD-186, VD-225, ratify VD-241 | **Đã ratify** | **Không** |
| Supersede | VD-222 §5 → VD-231 | `AnyError = {error}` bị thay bởi alias bản ghi `Error` `RecId(0)`; `{error}` cấm ở bề mặt người dùng | VD-231, ratify VD-241 | **Đã phân xử** | **Không** |
| Huỷ bỏ | VD-223(i) → VD-224 | `VS0209` (“`--b` chỉ chế độ lệnh”) bị huỷ; `all_codes()` 54→53 | VD-224, ratify VD-241 | **Đã phân xử** | **Không** |
| Ngoài phạm vi phát hành | VD-229 phần closure-params → #155 / VS3 | Lambda params `(x: int?) => …` không parse được; lift params closure chưa làm | VD-232 (#155 chuyển VS3) | **Không ratify trong bản này**, đúng thiết kế | **Không** |
| Quyết định sau VD-241 | VD-242..VD-246 | ổn định sau pha 1; percent lỗi; tương thích VERSION/cache; runtime trap global; capture/local trap | A0 trực tiếp, ghi trong `DECISIONS.md` | **Đã là quyết định A0** | **Không** |

## 3. Ghi nhận cho hồ sơ nghiệm thu

- Không-backfill số VD mới cho các mục trên: VD-241 đã ratify, việc thêm số mới chỉ tạo nhầm lẫn.
- `ST-244` có thể đóng với tư cách **đã phân xử bảng ratification**; không phát phiếu xin ý kiến A0.
- Nếu A0 muốn đảo một mục cụ thể (ví dụ VD-228 Option B), đây là **quyết định mới**, phải mở VD mới kèm phân tích revert — không phải phần “chưa ratify” của ST-244.
