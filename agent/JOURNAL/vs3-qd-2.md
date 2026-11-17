<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-QĐ-2 · Kênh: rendezvous `chan(0)` + `recv_any` + #145
**Trạng thái**: **ĐÃ DUYỆT CÓ SỬA** (VD-178) · đã áp [SỬA] tại **ST-178** · **Phụ thuộc**: QĐ-1 · **Mở đường cho**: —

## 1. Mục tiêu đo được
1. `chan(0)` = **rendezvous hợp lệ**: `send` **chặn** cho tới khi có `recv` (đo: gửi rồi không nhận trong 50 ms ⇒ chưa trả về; nhận ⇒ cả hai trả về). **Không** bị chặn ở tyck, **không** bị chặn ở runtime.
   > **đã sửa**: `agent/JOURNAL/vs3-qd-2.md:@@L1@@` (ST-178) — chốt chan(0) hợp lệ, bỏ mâu thuẫn “vừa hợp lệ vừa bắt tyck báo lỗi”.
2. `recv_any(chans, timeout: …)` trả **`(int, T)?`**; hết giờ trả **`None`** (theo R2 — VS3 chạy sau pha 1, từ khoá `none` đã bị bỏ). Mọi kênh trong danh sách **cùng kiểu phần tử `T`**.
3. **Công bằng**: khi nhiều kênh cùng sẵn sàng, chọn theo **vòng xoay** (không luôn chọn kênh đầu) để tránh đói; test kiểm bằng **bộ đếm, tất định**.
4. `chan(0)` **không** bị chặn ở tyck và **không** bị chặn ở runtime; **#145** đóng bằng cách **xoá chú thích sai** `rt.rs:128-129` + **bỏ chặn runtime** — **không** thêm kiểm `cap == 0` ở tyck.
5. Không thêm cú pháp mới; tên `select` **không** bị chiếm (lệnh chiếu cột).

## 2. Hiện trạng (dẫn chứng)
- `crates/vis-interp/src/builtin.rs:772` — `$kenh(n)` tạo kênh đệm `n`; `:1232` `chan_method`.
- `crates/vis-interp/src/rt.rs:120` — `tao_kenh(host, cap, span)`.
- `rt.rs:128-129` chú thích nói literal `chan(0)` bị **tyck** chặn, nhưng `crates/vis-types/src/tyck/expr/kenh.rs:16-33` **không** kiểm `cap`; runtime chặn (exit 1) — **#145**.

## 3. Thiết kế chốt (theo VD-163 + VD-178 QĐ-2)
- Hàm std **`recv_any(chans, timeout: …)`**, không cú pháp mới; **không** nhánh gửi ở VS3. Kiểu trả **`(int, T)?`**; hết giờ ⇒ **`None`**; mọi kênh cùng kiểu `T`.
- `chan(0)`: **rendezvous thật** (hàng đợi người gửi/nhận, đánh thức đúng cặp) — **hợp lệ**, không phải lỗi.
- **#145** xử **trong** QĐ-2: **xoá chú thích sai** ở `rt.rs:128-129` và **bỏ chặn runtime**; **không** thêm kiểm `cap == 0` ở tyck (mâu thuẫn cũ: vừa cho rendezvous hợp lệ vừa bắt tyck báo lỗi).
- **Công bằng**: nhiều kênh cùng sẵn sàng ⇒ chọn theo **vòng xoay** (con trỏ xoay sau mỗi lần nhận), test bằng bộ đếm tất định.

## 4. Chia task ≤ 3 h
| Task | Nội dung | Điều kiện xong | Phụ thuộc |
|---|---|---|---|
| VS3-2a | Rendezvous `chan(0)` (hàng đợi + đánh thức) | ca gửi-không-nhận chặn; nhận ⇒ trả; không treo | QĐ-1 xong |
| VS3-2b | `recv_any` + timeout + **công bằng vòng xoay** | 3 ca: chọn đúng kênh, timeout ⇒ `None`, nhiều kênh rỗng; ca vòng xoay bằng bộ đếm | 2a |
| VS3-2c | **#145**: xoá chú thích sai `rt.rs:128-129` + **bỏ chặn runtime** | ca đỏ-trước: `chan(0)` chạy rendezvous đúng (trước: runtime exit 1); **không** thêm kiểm tyck | 2a |
| VS3-2d | Kiểm chéo interp/VM + tài liệu | `vis test` 447, doc-examples; mục kênh trong SPEC/thiết kế cập nhật (Agent 0 duyệt) | 2b, 2c |

## 5. Ca đỏ-trước
- `chan(0)`: gửi không ai nhận trong 50 ms ⇒ **chưa** trả (đo bằng cờ, không bằng ngủ dài).
- `recv_any` với 2 kênh, chọn đúng kênh có dữ liệu; timeout 50 ms ⇒ `None`.
- **Công bằng**: 2 kênh cùng sẵn sàng, nhận 4 lần ⇒ bộ đếm mỗi kênh = 2 (vòng xoay), không phải 4/0.
- `chan(0)` literal ⇒ **chạy rendezvous** (trước: runtime exit 1).

## 6. Cổng
`gate --fast` + `vis test` 447 + ca đa nền (mac/Linux). Không đổi ngưỡng.

## 7. Rủi ro
Đánh thức sai thứ tự ⇒ bất định (test phải tất định); đói (starvation) — đã chốt **vòng xoay**; tương tác với QĐ-1 (kênh là đường duy nhất qua worker).

## 8. Bảng [SỬA] VD-178 đã áp (ST-178)
| [SỬA] VD-178 | Nội dung | đã sửa |
|---|---|---|
| QĐ-2 mục 1 | `chan(0)` = **rendezvous hợp lệ**; #145 = xoá chú thích sai + bỏ chặn runtime (không thêm tyck) | `agent/JOURNAL/vs3-qd-2.md:@@L1@@` |
| QĐ-2 mục 2 (áp dụng) | `recv_any` trả `(int, T)?`; hết giờ ⇒ `None`; cùng kiểu `T` | `agent/JOURNAL/vs3-qd-2.md:@@L2@@` |
| QĐ-2 mục 3 (áp dụng) | Công bằng **vòng xoay**, test bằng bộ đếm tất định | `agent/JOURNAL/vs3-qd-2.md:@@L3@@` |
