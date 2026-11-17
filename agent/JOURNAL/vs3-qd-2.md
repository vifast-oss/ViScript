<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-QĐ-2 · Kênh: rendezvous `chan(0)` + `recv_any` + #145
**Trạng thái**: CHỜ AGENT 0 DUYỆT (VD-148) · **Phụ thuộc**: QĐ-1 · **Mở đường cho**: —

## 1. Mục tiêu đo được
1. `chan(0)` = **rendezvous**: `send` **chặn** cho tới khi có `recv` (đo: gửi rồi không nhận trong 50 ms ⇒ chưa trả về; nhận ⇒ cả hai trả về).
2. `recv_any(chans, timeout=None)` trả `(chi_so, gia_tri)`; hết giờ trả `none` (khớp quy ước kiểu trả về hiện có).
3. `chan(0)` phải bị chặn **lúc tyck** (exit 2) **hoặc** chú thích `rt.rs` phải sửa cho đúng thực tế (nợ **#145**); **không** để lệch tài liệu–mã.
4. Không thêm cú pháp mới; tên `select` **không** bị chiếm (lệnh chiếu cột).

## 2. Hiện trạng (dẫn chứng)
- `crates/vis-interp/src/builtin.rs:772` — `$kenh(n)` tạo kênh đệm `n`; `:1232` `chan_method`.
- `crates/vis-interp/src/rt.rs:120` — `tao_kenh(host, cap, span)`.
- `rt.rs:128-129` chú thích nói literal `chan(0)` bị **tyck** chặn, nhưng `crates/vis-types/src/tyck/expr/kenh.rs:16-33` **không** kiểm `cap`; runtime chặn (exit 1) — **#145**.

## 3. Thiết kế chốt (theo VD-163)
- Hàm std **`recv_any(chans, timeout=None)`**, không cú pháp mới; **không** nhánh gửi ở VS3.
- `chan(0)`: rendezvous thật (hàng đợi người gửi/nhận, đánh thức đúng cặp).
- #145 xử **trong** QĐ-2: thêm kiểm `cap == 0` ở tyck **hoặc** sửa chú thích; ưu tiên **tyck** (lỗi sớm), mã lỗi giữ `VS0501` trừ khi Agent 0 chốt mã khác.

## 4. Chia task ≤ 3 h
| Task | Nội dung | Điều kiện xong | Phụ thuộc |
|---|---|---|---|
| VS3-2a | Rendezvous `chan(0)` (hàng đợi + đánh thức) | ca gửi-không-nhận chặn; nhận ⇒ trả; không treo | QĐ-1 xong |
| VS3-2b | `recv_any` + timeout | 3 ca: chọn đúng kênh, timeout ⇒ `none`, nhiều kênh rỗng | 2a |
| VS3-2c | **#145**: tyck kiểm `cap` của `kenh(0)` | ca đỏ-trước `exit 2`, không phải runtime; sửa chú thích `rt.rs` | 2a |
| VS3-2d | Kiểm chéo interp/VM + tài liệu | `vis test` 447, doc-examples; mục kênh trong SPEC/thiết kế cập nhật (Agent 0 duyệt) | 2b, 2c |

## 5. Ca đỏ-trước
- `chan(0)`: gửi không ai nhận trong 50 ms ⇒ **chưa** trả (đo bằng cờ, không bằng ngủ dài).
- `recv_any` với 2 kênh, chọn đúng kênh có dữ liệu; timeout 50 ms ⇒ `none`.
- `kenh(0)` literal ⇒ `VS0501` lúc **biên dịch** (trước khi sửa: runtime exit 1).

## 6. Cổng
`gate --fast` + `vis test` 447 + ca đa nền (mac/Linux). Không đổi ngưỡng.

## 7. Rủi ro
Đánh thức sai thứ tự ⇒ bất định (test phải tất định); đói (starvation); tương tác với QĐ-1 (kênh là đường duy nhất qua worker).
