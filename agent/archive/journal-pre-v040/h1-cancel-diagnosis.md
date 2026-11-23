<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Chẩn đoán flake h1 cancel trên Windows — đối chiếu code + ma trận Linux

> **Người chạy**: W3 (hỗ trợ W2/ST-287), 2026-11-22 16:24–16:38 JST (~20 phút; time-box 1 h).
> **Phạm vi**: **chỉ đọc code + đo**, **không sửa code/test**, không tag/push/merge, không GitHub Actions.
> **Mốc**: `plan/w3-platform` merge `main` **`557c516`** (`651537c` + W2 matrix `4117493`), version **0.3.6**;
> mã/test `vs3_http_cancel.rs` giống `a1930cb` (`git diff a1930cb..HEAD -- crates` rỗng).

## 0. TL;DR

1. **Không tìm thấy static/thread-local leak chứng minh được** giữa các test là nguyên nhân chính. Các
   thread-local liên quan đều được dọn (`rt.rs:502-528`, `async_http.rs:69-83`), và test **đầu tiên**
   trong suite serial (`dong_socket_va_parity`, W2 D1) cũng đỏ — loại trừ leak từ test trước.
2. Nghi phạm chính là **race giữa mốc huỷ cố định 20 turn và đường lỗi socket Windows `10053`**: huỷ chỉ
   áp tại **safe point** `frame_next` (`rt/sched.rs:102-125`), còn `step_frame` chạy trọn một bước; khi
   `read` trả `10053`, `http.get_timeout` trả `Err` cho script, script in `V:post` **trong cùng bước**
   trước khi safe point kế tiếp kịp huỷ. Windows trả `10053` sớm hơn Linux (Linux 100/100 + 50/50 ca
   `server_khong_tra_header`/`dong_socket` xanh dưới 2 burner).
3. Lỗi **có thể là sản phẩm** ở đường đóng socket Windows: `AsyncNet::close` bỏ `MioStream` **trước**
   khi deregister (`async_net.rs:567-600`), và `10053` không nằm trong `la_would_block`
   (`async_net_nguon.rs:16-18`). Cần fix/đo thêm — **đề xuất cụ thể ở §6**.

## 1. Test đang giả định gì?

- `SO_TURN = 20` (`vs3_http_cancel.rs:182`); `chay_i` đặt trigger huỷ theo turn rồi chạy:
  `frame_cancel_clear_triggers_for_test()` → `frame_cancel_after_turns_for_test(20, 1)` →
  `run_frame_for_test` (`:210-217`).
- `kiem_sau_huy` kiểm **theo thứ tự**: exit 0, có `main`, có `V:pre`, **không có `V:post`**
  (`:240-248`), rồi mới kiểm `cancel:1` (`:249-...`). Vì assert no-`V:post` đứng trước, failure không
  cho biết `cancel:1` đã xảy ra hay chưa.
- Script in `V:pre`, gọi `http.get_timeout(url, 4000)`, in `loi: …` nếu Err, rồi **luôn** in `V:post`
  (`:191-208`). Không có cơ chế "dừng ngay sau khi bị huỷ" ở tầng script.

## 2. Huỷ chỉ áp tại safe point, không áp giữa một bước

- `frame_next` là safe point duy nhất xử lý trigger turn/seq rồi gọi `frame_cancel_process_for_test`
  (`rt/sched.rs:102-125`). Comment ghi rõ "**Không bao giờ cắt ngang `step_frame`**" (`:103-104`).
- `frame_cancel_one` gỡ waiter/đăng ký, đóng tay HTTP, bỏ ready và **sau cùng** mới đặt
  `TrangThai::Huy` (`rt/sched.rs:637-769`; terminal ở `:765-769`).
- Driver chỉ kiểm trạng thái **trước** khi chạy frame (`frame/worker.rs:133-150`); sau khi
  `step_frame` trả `Ok`/`Trap`/`Ret`, nó ghi cache/`Xong` **không kiểm `frame.cancelled`**
  (`frame/worker.rs:150-167` cho interp, `vm_frame.rs:656-668` cho VM).
- Hệ quả: nếu một bước đang chạy kịp xử lý lỗi I/O + `print("V:post")` + trả về, task có thể **Xong
  trước** khi turn 20 tới; `frame_cancel_one` sau đó thấy `Xong` và chỉ log `cancelskip:done`
  (`rt/sched.rs:650-654`), không có `cancel:1`.

## 3. Đường `WSAECONNABORTED 10053`

Chuỗi gọi (file:dòng):

1. `AsyncNet::try_read` gọi `MioStream::read`; lỗi không thuộc WouldBlock/Interrupted ⇒
   `ReactorIoOutcome::Err("lỗi đọc `C1`: … (os error 10053)")`
   (`crates/vis-cli/src/async_net.rs:372-385`; `la_would_block` chỉ nhận 115/10035/10036 —
   `async_net_nguon.rs:16-18`).
2. `frame_real_poll` nhận `ReactorIoOutcome::Err` ⇒ `RealPoll::Loi(msg)` (`rt.rs:888-894`) ⇒
   `frame_reactor_poll_bytes` trả `Ok(Err(msg))` (`rt.rs:678-680`).
3. `HttpAsync::drive` thấy `Ok(Err(e))` ⇒ `Ok(Some(loi_http(e)))` — **không kiểm cờ huỷ**
   (`net/async_http.rs:289-302`).
4. `HttpAsync::chay` thấy `Ok(Some(v))` ⇒ `st.dong(host)` rồi `Ok(v)` (`async_http.rs:130-133`).
5. Interpreter nhận `Value::Err` như kết quả builtin (`frame/expr.rs:735/811`), chạy tiếp các work
   item của script: `print("loi: …")` rồi `print("V:post")` (`vs3_http_cancel.rs:197-200`).
6. Safe point kế tiếp mới thấy cancel trigger; nếu task đã Xong ⇒ `cancelskip:done`; nếu chưa ⇒
   `cancel:1` nhưng `V:post` đã in rồi.

**Không có** kiểm `frame.cancelled` trong `step_frame`/`process_work` sau khi builtin trả lỗi, cũng
không có kiểm trước `print` kế tiếp. Đó là lý do trực tiếp của stdout `V:post` sau `10053`.

## 4. Static / thread-local / test leak — đã rà

| State | Nơi | Dọn khi nào | Kết luận |
|---|---|---|---|
| `FRAME_CANCEL_TEST` / `FRAME_CANCEL_TURNS` | `rt.rs:502-528` | `frame_cancel_clear_triggers_for_test` (`chay_i`/`chay_v` gọi trước & sau); `frame_enable_for_test` **drain** vào sched (`sched.rs:27-47`) | Không leak giữa ca; trigger thuộc **thread-local của test thread**, mỗi process riêng |
| `DANG_CHAY` (state HTTP theo `frame.current`) | `net/async_http.rs:69-83` | `xoa_trang_thai_frame` khi huỷ (`sched.rs:755`); `chay` lấy state ra bằng `remove` (`:117`) và chỉ insert lại khi `Pending` (`:136`) | Không leak nếu cancel/success; có thể leak nếu **panic giữa chừng** — chỉ là rủi ro phụ, không giải thích D1 (test đầu suite) |
| `FRAME_NET_IO.writes` (`W:<tay>`) | `stdlib/frame_net.rs:17-35` | `frame_net_take_write` sau mỗi poll (`async_http.rs:269`) và khi cancel gỡ reg (`sched.rs:738`) | Không thấy leak trong đường test này |
| `MayChuH1` server thread | `vs3_http_cancel.rs:47-135` | `Drop` join listener + mọi thread | Test panic vẫn Drop (unwrap); không thấy thread sống sang ca sau |
| Name cache/quota (`value.rs`, `frame/fee.rs`) | thread-local | per-thread, thuần hiệu năng | Không liên quan socket/cancel |

**Vì sao 2 process song song 0 % nhưng serialized 80 %?** Không phải chữ ký của leak (2 process vẫn
có thread-local riêng; leak trong một process sẽ **không** biến mất khi chạy 2 process). Hợp lý hơn:
- Serialized chạy **nhanh/ít cạnh tranh** ⇒ task kịp đi hết nhánh lỗi `10053` + `V:post` **trước** turn 20.
- 2 process song song làm mỗi bên **chậm** ⇒ safe point turn 20 tới trước, huỷ thắng ⇒ 0 %.
- W2 cũng thấy ca riêng `server_khong_tra_header` **3/10 đỏ** khi chạy một mình ⇒ flake **nội tại
  từng ca**, không phải chỉ do thứ tự test; D1 (test đầu suite serial) đỏ càng loại trừ leak.

Đo bổ sung Linux (cùng binary, 2 burner): `server_khong_tra_header` **50/50**, `dong_socket_va_parity`
**50/50**, `lap_nhieu_vong` **100/100**, full suite **20/20** — không tái hiện. Log
`/tmp/opencode/linux-h1-matrix-diag/summary.txt` (không có log fail).

## 5. Vì sao Linux không lộ

- Linux `read` khi peer đóng/reset thường trả `Ok(0)` (EOF) hoặc `ECONNRESET`; đường EOF được xử lý
  thành `ket_thuc_eof` **sau** khi parser đã có header/body, không tạo `V:post` sớm như `10053`.
- Timing Linux + server loopback: cancel turn 20 thường tới trước khi có lỗi cứng; 100/100 +
  50/50 + 50/50 xanh dưới 2 burner.
- Windows `10053` (WSAECONNABORTED) là lỗi **abort cứng** do stack/AFD khi socket bị đóng/reset;
  nó đi thẳng vào nhánh `Err` của `http`, không phải WouldBlock.

## 6. Đề xuất fix cho W2 (theo thứ tự)

### P0 — làm test tất định (không cần đổi sản phẩm trước)

1. **Đổi trigger**: thay `SO_TURN = 20` bằng mốc **sau khi đã có body/đã thấy `C1`**:
   - thêm test-hook `frame_cancel_after_handle_for_test(handle, id)` (hoặc đếm **số byte** server đã
     gửi, không đếm turn), kích hoạt ngay safe point kế tiếp sau khi `note_task_handle("C1")`;
   - hoặc giảm `SO_TURN` còn 3–5 và assert `da_gui() > 4096`.
2. **Đổi thứ tự assert**: kiểm `out.log` có `cancel:1` **trước** assert `!V:post`; nếu task đã
   `Xong` trước cancel, log sẽ có `cancelskip:done` — phân biệt rõ "hoàn tất trước huỷ" với
   "chạy tiếp sau huỷ". Đưa `out.log` vào message panic.
3. Nếu mục tiêu là kiểm **sản phẩm** (task bị huỷ không chạy tiếp), test phải chờ tín hiệu
   "đã huỷ" trước khi cho server gửi byte lỗi; hiện test chỉ dựa vào turn.

### P0 — kiểm cờ huỷ trong driver (nếu muốn chặn tại sản phẩm)

4. Trong `frame/worker.rs:150-167` và `vm_frame.rs:656-668`, sau `step_frame`/`exec` trả
   `Ok/Trap/Ret`, kiểm `sched.frame.cancelled.contains(current)`; nếu có, **không** ghi `kq`/`Xong`,
   không `frame_task_done`, log `cancelrace:<id>` và bỏ output của bước đó (hoặc coi là Huy).
5. Trong `frame_real_poll` (`rt.rs:866-895`), trước khi map `ReactorIoOutcome::Err`, kiểm
   `self.frame.cancelled.contains(&self.frame.current)`; nếu đúng, trả một abort/`Pending`
   "đã huỷ" thay vì `RealPoll::Loi` để `http` không biến `10053` thành `Value::Err` cho script.

### P1 — Windows close/deregister ordering (nghi phạm sản phẩm thật)

6. `AsyncNet::close` (`async_net.rs:567-600`) đang `streams.remove(handle)` / drop `MioStream`
   **trước** vòng `deregister` và `bo_sach_nguon`. Trên Windows (IOCP/AFD) nên:
   - gom `keys` + gọi `deregister`/`huy_nguon` **trước**, chỉ sau đó `streams.remove`/drop;
   - gọi `shutdown(Both)` (nếu còn) trước khi drop để tránh AFD phát abort muộn;
   - giữ một cờ `closed_by_cancel` theo token/handle để `try_read` map `10053` thành
     `ReactorIoOutcome::Cancelled` thay vì lỗi script.
7. `cancel_task_handles` (`async_net.rs:622-634`) nên **không** đóng handle nếu `frame_tay` đã bị
   `xoa_so_tay` bởi lần đóng trước; hiện đã xoá theo handle nhưng cần log/đo để chắc không đóng
   `C1` của frame khác (nhất là khi `frame id` tái dùng giữa các vòng `lap_nhieu_vong`).

### P2 — dọn test-state phòng panic

8. Thêm `clear_all_for_test()` cho `DANG_CHAY` + `FRAME_NET_IO.writes` và gọi ở đầu/cuối mỗi ca
   (hoặc `Drop` guard), để một ca panic không để state `frame id 1` sang ca sau. Đây là hardening,
   không phải nguyên nhân đã chứng minh.

## 7. Cách đo xác nhận cho W2 (không sửa sản phẩm)

Trên `psvn`, chạy ca đỏ với `--nocapture` và **in `out.log`** trong message assert; xác định:
- Nếu log có `cancel:1` **trước** `V:post` ⇒ task chạy sau huỷ → bug driver/close ordering (P0/P1).
- Nếu log có `cancelskip:done` hoặc không có `cancel:1` ⇒ task hoàn tất **trước** turn 20 → test race
  (P0 test), không phải "cancel không dừng task".
- Chạy lại sau khi đổi trigger sang handle/byte; nếu 20/20 xanh thì xác nhận nguyên nhân test-timing.

## 8. Kết luận / blocker

- **Chưa chứng minh static/thread-local leak**; không có bằng chứng leak giải thích serial 80 %.
- **Nghi phạm chính**: test dùng mốc turn cố định trong khi `10053` Windows kết thúc request sớm;
  driver không kiểm cờ huỷ sau một bước và `AsyncNet::close` có thứ tự drop/deregister đáng ngờ trên
  Windows.
- **Blocker `v0.3.6` vẫn nguyên**: cần W2 quyết định fix P0 test + P1 close ordering, rồi chạy lại
  Windows `vs3_http_cancel` ×10 + workspace 1 vòng. Không tag trước đó.
- Không sửa code/test trong phiên này. Đĩa cuối **8,7 G** (> 5 G).
