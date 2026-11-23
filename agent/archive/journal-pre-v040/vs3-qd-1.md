<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-QĐ-1 · Async **song song thật**
**Trạng thái**: **ĐÃ DUYỆT CÓ SỬA** (VD-178) · đã áp [SỬA] tại **ST-178** · **Thay thế**: — · **Phụ thuộc**: không (gốc) · **Mở đường cho**: QĐ-2, QĐ-3, QĐ-4

## 1. Mục tiêu đo được
1. **Song song thật**: chương trình `N` **task tính toán thuần** (vòng lặp cộng, không chia sẻ, **không** sleep/IO ảo — task sleep chạy chồng nhau được ngay cả trên 1 luồng nên **không** chứng minh song song thật) chạy trên `W` worker; đo **wall-clock** với `W = min(nproc, N)` và `W = 1`; **ngưỡng chốt: `W=min(nproc,N)` nhanh hơn `W=1` ít nhất `0,6 × min(nproc, N, 4)` lần** trên máy ≥ 4 nhân (giới hạn 4 để ngưỡng không phụ thuộc máy nhiều nhân). Ghi `nproc`, **load** trước/sau và **giữ khoá đo** (`~/.vifast-measure.lock`, VD-177 mục 3) vào log.
   > **đã sửa**: `agent/JOURNAL/vs3-qd-1.md:6` (ST-178) — thay ngưỡng cũ `0,80 × min(nproc, N)` và bỏ ca sleep/IO ảo.
2. **Tất định**: cùng một chương trình **không chia sẻ** chạy `W=1` và `W=4` cho **cùng stdout/exit** (byte-for-byte), ở interp **và** VM.
3. **Không `unsafe`**: `crates/vis-interp`, `crates/vis-vm`, `crates/vis-cli` giữ `#![forbid(unsafe_code)]`; chứng minh `Value: !Send` bằng **doctest `compile_fail`**.
4. **Không rò qua closure**: gửi closure/handle tệp/Task qua `Chan` liên luồng ⇒ lỗi **lúc biên dịch** nếu biết, nếu không ⇒ mã lỗi lúc chạy **VS0508** (xem §3).

## 2. Hiện trạng (dẫn chứng)
- `crates/vis-interp/src/rt.rs:39` — `struct Task`; `:95` `tao`; `:112` `spawn`; `:176` `await_task` — executor **đơn luồng**, chạy lần lượt.
- `crates/vis-interp/src/interp.rs:221-234` — dựng `argv` và gọi `main` (parity đã sửa ở ST-157).
- 5 câu VD-131 hiện hành: “Async không song song.” — câu này **phải đổi** khi VS3-QĐ-1 xong (Agent 0 duyệt lại câu chữ).

## 3. Thiết kế chốt (theo VD-160 + VD-178 QĐ-1)
- **Heap riêng mỗi worker**, bộ đếm tham chiếu **không atomic**; **loại** `Arc`/`Mutex` toàn cục.
- `Send`/`Sync`: `Value` **không/không** · `Task` **không/không** · `Chan` **có/có** · `Host` **có/có**.
- **`spawn` được gửi sang worker khác** — **không** cấm await task ở worker khác (cấm như vậy làm hỏng mẫu cơ bản `let t = spawn f(x)` rồi `await t`):
  - Đối số và giá trị bắt (capture) của closure phải **gửi được** (Sendable), và được **sao chép sâu** lúc spawn.
  - Giá trị trả về của task cũng phải **gửi được**, **sao chép sâu** khi await.
  - Kiểu gửi được: `int`, `float`, `bool`, `str`, `bytes`, đơn vị, `list/map/set/tuple/record/enum` chỉ chứa các kiểu này. **Closure, handle tệp, `Task`: không gửi được.**
  - Vi phạm: **lỗi biên dịch** khi biết kiểu; nếu không thì **lỗi lúc chạy**.
  > **đã sửa**: `agent/JOURNAL/vs3-qd-1.md:20` (ST-178) — thay luật cũ “await task khác worker bị chặn”.
- Qua `Chan` giữa 2 worker: **sao chép sâu** cùng tập kiểu gửi được ở trên.
- Chia sẻ `bytes`/`str` lớn bất biến qua `Arc`: **hoãn**, ghi nợ, chỉ làm khi có số đo.
- **Mã lỗi chốt**: **VS0508** “giá trị không gửi được sang worker khác” (VS0507 đã dành cho tràn i64 ở R9).

## 4. Chia task ≤ 3 h
| Task | Nội dung | Điều kiện xong | Phụ thuộc |
|---|---|---|---|
| VS3-1a | Khung scheduler `W` worker + `spawn` phân phối + tắt êm; **chưa** đổi ngữ nghĩa | `W=1` cho **cùng** kết quả hiện tại (toàn bộ L0 xanh) | — |
| VS3-1b | `suspend/resume` + `await_task` **liên worker chạy được**; kiểm **Sendable** (đối số, capture, giá trị trả) + **sao chép sâu** lúc spawn/await | ca: `let t = spawn f(x)` rồi `await t` ở worker khác chạy đúng; gửi closure/handle/Task ⇒ **VS0508** (biết kiểu: lỗi biên dịch; không: lỗi lúc chạy), không deadlock | 1a |
| VS3-1c | Sao chép sâu khi spawn/await; kênh là đường duy nhất qua worker | ca đỏ-trước: `Value` chạm worker khác ⇒ không biên dịch/lỗi **VS0508** | 1b |
| VS3-1d | Ca song song thật (**task tính toán**) + đo `W=1` vs `W=min(nproc,N)` | 2 số + `nproc` + load + khoá đo trong log; tỉ lệ ≥ `0,6 × min(nproc,N,4)` | 1c |
| VS3-1e | `compile_fail` doctest `Value: !Send`; differential `W=1` vs `W=4` | doctest chạy trong L0; differential 0 lệch | 1c |

## 5. Ca đỏ-trước (bắt buộc có trước khi sửa)
- `compile_fail`: hàm nhận `T: Send`, gọi với `Value` ⇒ không biên dịch.
- `let t = spawn f(x)` + `await t` ở worker khác ⇒ **chạy đúng** (trước: bị chặn).
- Gửi closure/handle/Task qua `Chan` hoặc capture trong `spawn` ⇒ **VS0508** (biết kiểu: lỗi biên dịch; không: lỗi lúc chạy); **không** treo.
- `W=1` vs `W=4`: cùng stdout (chương trình không chia sẻ).
- Đo song song: `N` task **tính toán thuần** ⇒ tỉ lệ ≥ `0,6 × min(nproc,N,4)` (máy ≥ 4 nhân).

## 6. Cổng
`gate --fast` (10/10, L0 0 đỏ) + `vis test` 447 + differential `W=1`/`W=4` + bảng đo song song (**task CPU**, ngưỡng `0,6 × min(nproc,N,4)`, log riêng, không tính vào L0). **Không** nới ngưỡng L0.

## 7. Rủi ro
(1) Bộ nhớ/GC: heap riêng làm `Value` không `Send` — mọi API công khai phải rà lại; (2) tính tất định của differential; (3) `nproc` ảo trên CI; (4) macOS thread stack nhỏ hơn Linux; (5) ngưỡng đo song song phụ thuộc `nproc`/load — đã giới hạn 4 và bắt buộc ghi load + giữ khoá đo.

## 8. Bảng [SỬA] VD-178 đã áp (ST-178)
| [SỬA] VD-178 | Nội dung | đã sửa |
|---|---|---|
| QĐ-1 mục 1 | Đo song song bằng **task CPU**; ngưỡng `0,6 × min(nproc,N,4)`; ghi `nproc`/load/khoá đo | `agent/JOURNAL/vs3-qd-1.md:6` |
| QĐ-1 mục 2 | **Bỏ cấm await worker khác**; luật Sendable + sao chép sâu; **VS0508** | `agent/JOURNAL/vs3-qd-1.md:20` |
