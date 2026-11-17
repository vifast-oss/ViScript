<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-QĐ-1 · Async **song song thật**
**Trạng thái**: CHỜ AGENT 0 DUYỆT (VD-148) · **Thay thế**: — · **Phụ thuộc**: không (gốc) · **Mở đường cho**: QĐ-2, QĐ-3, QĐ-4

## 1. Mục tiêu đo được
1. **Song song thật**: chương trình `N` task độc lập (mỗi task sleep/IO ảo) chạy trên `W` worker; đo **wall-clock** với `W = min(N, nproc)` và `W = 1`; **tỉ lệ ≥ 0,80 × min(nproc, N)** trên máy ≥ 4 core, ghi cả 2 số + `nproc` vào log (ngưỡng này Agent 0 chốt khi duyệt; chưa dùng để "đạt" trước khi có số).
2. **Tất định**: cùng một chương trình **không chia sẻ** chạy `W=1` và `W=4` cho **cùng stdout/exit** (byte-for-byte), ở interp **và** VM.
3. **Không `unsafe`**: `crates/vis-interp`, `crates/vis-vm`, `crates/vis-cli` giữ `#![forbid(unsafe_code)]`; chứng minh `Value: !Send` bằng **doctest `compile_fail`**.
4. **Không rò qua closure**: gửi closure/handle tệp/Task qua `Chan` liên luồng ⇒ lỗi **lúc biên dịch** nếu biết, nếu không ⇒ mã lỗi lúc chạy (đề xuất mã mới, xem §3).

## 2. Hiện trạng (dẫn chứng)
- `crates/vis-interp/src/rt.rs:39` — `struct Task`; `:95` `tao`; `:112` `spawn`; `:176` `await_task` — executor **đơn luồng**, chạy lần lượt.
- `crates/vis-interp/src/interp.rs:221-234` — dựng `argv` và gọi `main` (parity đã sửa ở ST-157).
- 5 câu VD-131 hiện hành: “Async không song song.” — câu này **phải đổi** khi VS3-QĐ-1 xong (Agent 0 duyệt lại câu chữ).

## 3. Thiết kế chốt (theo VD-160)
- **Heap riêng mỗi worker**, bộ đếm tham chiếu **không atomic**; **loại** `Arc`/`Mutex` toàn cục.
- `Send`/`Sync`: `Value` **không/không** · `Task` **không/không** · `Chan` **có/có** · `Host` **có/có**.
- Qua `Chan` giữa 2 worker: **sao chép sâu** (`int/float/bool/str/bytes/list/dict/record` của các kiểu này); closure/handle/Task **không truyền được**.
- Chia sẻ `bytes`/`str` lớn bất biến qua `Arc`: **hoãn**, ghi nợ, chỉ làm khi có số đo.
- Mã lỗi mới (đề xuất, **Agent 0 chốt số**): `VS05xx` “giá trị không gửi được qua kênh giữa hai worker”.

## 4. Chia task ≤ 3 h
| Task | Nội dung | Điều kiện xong | Phụ thuộc |
|---|---|---|---|
| VS3-1a | Khung scheduler `W` worker + `spawn` phân phối + tắt êm; **chưa** đổi ngữ nghĩa | `W=1` cho **cùng** kết quả hiện tại (toàn bộ L0 xanh) | — |
| VS3-1b | `suspend/resume` + `await_task` liên worker **bị chặn** (báo lỗi rõ) | ca: await task khác worker ⇒ lỗi `VS05xx`, không deadlock | 1a |
| VS3-1c | Wait-set: `await` chỉ trong worker sở hữu; kênh là đường duy nhất | ca đỏ-trước: `Value` chạm worker khác ⇒ không thể biên dịch/lỗi | 1b |
| VS3-1d | Ca song song thật + đo `W=1` vs `W=min(nproc,N)` | 2 số + `nproc` trong log; tỉ lệ ≥ ngưỡng Agent 0 chốt | 1c |
| VS3-1e | `compile_fail` doctest `Value: !Send`; differential `W=1` vs `W=4` | doctest chạy trong L0; differential 0 lệch | 1c |

## 5. Ca đỏ-trước (bắt buộc có trước khi sửa)
- `compile_fail`: hàm nhận `T: Send`, gọi với `Value` ⇒ không biên dịch.
- Gửi closure qua `Chan` ⇒ lỗi kiểu (nếu biết) hoặc lỗi lúc chạy; **không** treo.
- `W=1` vs `W=4`: cùng stdout (chương trình không chia sẻ).

## 6. Cổng
`gate --fast` (10/10, L0 0 đỏ) + `vis test` 447 + differential `W=1`/`W=4` + bảng đo song song (log riêng, không tính vào L0). **Không** nới ngưỡng L0.

## 7. Rủi ro
(1) Bộ nhớ/GC: heap riêng làm `Value` không `Send` — mọi API công khai phải rà lại; (2) tính tất định của differential; (3) `nproc` ảo trên CI; (4) macOS thread stack nhỏ hơn Linux.
