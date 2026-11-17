<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# LUẬT · Agent 1 quyết định thay Agent 0 khi A0 vắng (2026-11-17)

**Nguồn**: người điều phối ban hành (2026-11-17, 09:55 JST). **Áp dụng mọi dòng**; dòng ViScript ghi ở
`DECISIONS.md` **VD-165** với nhãn **CHỜ A0 XÁC NHẬN**.

## Được tự quyết (khi A0 vắng và người dùng cho phép)
1. Nợ nhỏ **không cần thiết kế**, việc sổ, đo đạc, báo cáo.
2. **Thứ tự task** trong phạm vi đã được A0 duyệt.
3. **Phân loại sự cố** khi đã có bằng chứng (log, diff, `file:dòng`).

## KHÔNG được tự quyết — ghi **BLOCKED-CHO-A0** và chuyển việc khác
1. Thiết kế, **API**, **ABI** mới hoặc thay đổi.
2. Chính sách **crate**, **giấy phép**, **unsafe**.
3. Đổi **kỳ vọng ca cũ**, đổi **ngưỡng**, đổi **cách đo ngưỡng**.
4. **Hạ phạm vi** một cổng/mốc đã duyệt, hoặc **tuyên bố đạt** cổng/mốc.

## Mỗi quyết định thay phải
- Có **mã riêng** (`17e`, `17f`, …), ghi **lý do + bằng chứng**.
- Gắn nhãn **“CHỜ A0 XÁC NHẬN”**.
- Được **liệt kê trong báo cáo kế tiếp** (`R29`…) để A0 duyệt hồi tố (công nhận hoặc đảo lại).

## Ghi chú
Các quyết định `17`, `17b`, `17c`, `17d` (đêm 17/11) do Agent 1 ban hành thay A0 **đã được A0 công nhận hồi tố**
ở **VD-157** (A2 quyết định đợt 29). Từ nay mọi quyết định thay đều theo khuôn này.
