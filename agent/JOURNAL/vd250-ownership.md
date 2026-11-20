<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-250 / ST-266 · Nháp **ownership task, handle & tài nguyên** (kèm bảng trạng thái)

**Trạng thái**: **DRAFT — CHỜ A0 CHỐT**. research + draft only; không sửa mã sản phẩm, không commit, không chạy gate.
**Căn cứ**: `agent/DECISIONS.md:2463-2473` (VD-250 §2.2/§2.3), `:2518-2524` (§5 ownership + hook), `agent/QUEUE.md:2063` (ST-266).
**Quy ước**: nhận định "hiện trạng" có `file:line`; mọi thiết kế/bảng đề xuất gắn **[CHỜ A0 CHỐT]**.

---

## 1. Hiện trạng có trích dẫn

| Đối tượng | Hiện trạng (file:line) |
|---|---|
| Task | `Task { id, than, args, ten, span, trang_thai, kq }` `crates/vis-interp/src/rt.rs:39-52`; `kq: RefCell<Option<Result<Value, Trap>>>` cache kết quả `:51`; `TrangThai { Moi, DangChay, Xong }` `:28-35`. `than` là **handle của host** (interp: `sym`; VM: chỉ số hàm bytecode) `:43-44` — không phải handle mạng. |
| Scheduler giữ mạnh | `Scheduler.tasks: Vec<Rc<Task>>` `crates/vis-interp/src/rt.rs:78`; `tao()` tạo `Rc` và `push` vào `tasks` `:95-110`; `spawn()` đẩy chỉ số vào `ready` `:112-116`; `kenh: Vec<Rc<RefCell<Chan>>>` `:86` giữ kênh. `join_all` clone **toàn bộ** `tasks` `:242-249`. Đo đạc: 50k task → RSS 24→29 MB, `tasks` giữ mọi task tới hết chương trình (`vd249-async-design-draft.md:33`). |
| Kết quả/cache | `await_task` trả cache khi `Xong` `rt.rs:180-194`; gọi thân một lần `:218`, ghi cache `:227`; lỗi bọc `VS0506` `:233-239`. |
| Handle mạng thoát task | `Net { tay: Vec<(String,TcpStream)>, tiep, nghe: Vec<(String,TcpListener)>, tiep_nghe }` `crates/vis-cli/src/net.rs:15-24`; `giu()` cấp handle `"C<n>"` `:46-50`; listener `"L<n>"` `:137-160`; `connect` trả handle chuỗi `:119-135`; `read`/`write`/`close` tra handle `:185-215`. Handle là **giá trị ngôn ngữ** (`Value::Str`), có thể được `return` từ `async fn` hoặc chia sẻ — không gắn với vòng đời task. |
| Kênh | `Chan { cap, buf, closed }` `rt.rs:54-72`; chương trình cầm `Rc<RefCell<Chan>>` riêng, scheduler cũng giữ một `Rc` `:86`. |
| Huỷ / wait-set | **Không có cancellation API**; kênh `close` không đánh thức ai (nợ #16) — `vd249-async-design-draft.md:96-104`. |
| Đo lường hiện có | `--explain-perms` in `NET …` ra stderr `crates/vis-cli/src/net.rs:17`, `:32-35` — tiền lệ instrumentation **đầu ra debug**, không phải hook đếm tài nguyên, không phải env interface. |

---

## 2. Yêu cầu A0 đã chốt (VD-250 §5, `agent/DECISIONS.md:2518-2524`)

1. **Không cần GC để bắt đầu**, nhưng Scheduler **không giữ mạnh vô thời hạn** task đã xong.
2. **Handle còn sống giữ kết quả** để await lại (`crates/vis-interp/src/rt.rs:180-194` là hành vi hiện tại cần bảo toàn).
3. **Hết handle VÀ xong nghĩa vụ join ⇒ trạng thái có thể thu hồi.**
4. **Task chưa await giữ đủ lâu cho final join**, **không được giải phóng sớm** (`rt.rs:242-249`).
5. **Cap là hàng rào bổ sung**, **chưa chọn số trước khi đo**.
6. **Không mặc định** "task kết thúc thì đóng mọi handle nó mở" — phân biệt **tài nguyên đang chờ do task sở hữu** với **handle có thể thoát khỏi task** (`agent/DECISIONS.md:2521-2522`).
7. **Hook chẩn đoán chỉ test/debug**, không giao diện môi trường hỗ trợ phát hành; đếm task, frame, đăng ký I/O, timer, handle còn sống (`agent/DECISIONS.md:2523-2524`).

---

## 3. Mô hình ownership đề xuất — bảng trạng thái **[CHỜ A0 CHỐT]**

### 3.1 Hai trục độc lập

Đề xuất tách rõ hai điều kiện mà hiện trạng gộp trong một `Rc<Task>` (`rt.rs:39-52`, `:78`):

- **Trục A — Ai còn giữ handle tới task/result**: giá trị `Task[T]` trong chương trình; result cache `kq` (`rt.rs:51`). Handle có thể thoát khỏi task tạo ra nó.
- **Trục B — Nghĩa vụ join**: mỗi task đã tạo có **đúng một** nghĩa vụ join, được tiêu ở `join_all` (`rt.rs:242-249`); **không** bị tiêu bởi việc drop handle. Task `spawn` mà không ai await vẫn mang nghĩa vụ này (`test/sem/async/task-khong-await-van-chay.vis`, `.out` = `main,chay 1,chay 2`).

**Luật thu hồi đề xuất [CHỜ A0 CHỐT]**: thu hồi `Task` + `kq` + frame khi **cả hai** đúng: (A) không còn handle/result reference nào, **và** (B) nghĩa vụ join đã tiêu. Trước đó thì giữ — kể cả khi task đã `Xong`.

### 3.2 Bảng trạng thái

**[CHỜ A0 CHỐT]** — tên trạng thái là đề xuất; `Moi`/`DangChay`/`Xong` hiện tại `rt.rs:28-35`.

| Trạng thái | Giữ bởi (đề xuất) | `kq` | Cho thu hồi? | Chuyển tiếp |
|---|---|---|---|---|
| `Moi` (tạo, chưa chạy) | Scheduler `tasks` + handle + nghĩa vụ join | `None` | **Không** | `spawn` (VD-250 §2.2 `agent/DECISIONS.md:2463-2464`) ⇒ `Ready` |
| `Ready` | `ready` queue `rt.rs:79` + handle + nghĩa vụ join | `None` | **Không** | poll ⇒ `Running` |
| `Running` | frame đang chạy + handle + nghĩa vụ join | `None` | **Không** | chạm await/I/O/quota ⇒ `Suspended`; xong ⇒ `Done(Ok)`; trap ⇒ `Done(Err)` |
| `Suspended(kind)` | wait-set/registry/timer + handle + nghĩa vụ join | `None` | **Không** | event/timer ready ⇒ `Ready`; cancel ⇒ `Cancelling` (Lát 3) |
| `Done(Ok/Err)` | handle (nếu còn) + nghĩa vụ join (nếu chưa tiêu) | `Some(Result)` — giữ để await lại `rt.rs:180-194` | **Chỉ khi** A && B (§3.1) | join tiêu ⇒ xét thu hồi |
| `Reclaimable` | không ai giữ; nghĩa vụ join đã tiêu | đã bỏ | Có — bước ngay trước giải phóng | Scheduler dọn ⇒ `Reclaimed` |
| `Reclaimed` | — | — | Đã thu hồi | — (không quay lại) |
| `Cancelling` / `Cancelled` | registry + nghĩa vụ join | `Err` theo mã **có sẵn** (ưu tiên `VS0505`/`VS0506`, `vd249-async-design-draft.md:103`) | Sau khi join tiêu + hết handle | **Thiết kế trước Lát 3, chưa hiện thực** |

### 3.3 Bảng chuyển tiếp (sự kiện → hành động)

**[CHỜ A0 CHỐT]**

| Sự kiện | Trước | Sau | Hành động ownership |
|---|---|---|---|
| `spawn f(...)` | — | `Moi`→`Ready` | Tạo task + nghĩa vụ join; đẩy `ready` (hiện `rt.rs:112-116`). Thân **không chạy ngay** (VD-250 §2.2). |
| `await t` lần đầu | `Moi`/`Ready` | `Running` | Đánh thức scheduler; giữ handle của `t`. |
| chạm await/I/O/quota | `Running` | `Suspended(kind)` | Lưu `pc`/frame; đăng ký vào wait-set/registry/timer; **không** đóng handle mạng. |
| event/timer ready | `Suspended` | `Ready` | Gỡ đăng ký cũ, đẩy vào `ready` theo thứ tự đã chốt (`vd250-determinism-contract.md` §3.3). |
| thân trả kết quả | `Running` | `Done(Ok)` | Ghi `kq` (`rt.rs:227`); giữ frame? → **bỏ frame**, giữ result. |
| thân trap | `Running` | `Done(Err)` | Bọc `VS0506` (`rt.rs:233-239`); giữ result để await lại; exit 101 (`crates/vis-cli/src/cli/run.rs:13-21`). |
| `await t` khi `Done` | `Done` | `Done` | Trả cache `rt.rs:180-194`; vẫn tính `E_AWAIT` mới (`vd250-charging-spec.md` §3.5); **không** tạo thêm handle giữ task. |
| final join | bất kỳ | tiêu nghĩa vụ join | Duyệt theo thứ tự tạo `rt.rs:242-249`; chờ task pending theo deadline (`net.rs:94,105`), **không** ép huỷ (VD-250 §2.3 `agent/DECISIONS.md:2469-2473`). |
| drop handle cuối, join chưa xong | `Done` | `Done` (giữ) | **Không thu hồi** — nghĩa vụ join còn (§3.1). |
| drop handle cuối, join đã xong | `Done` | `Reclaimable`→`Reclaimed` | Thu hồi `Task`/`kq`;Scheduler bỏ `Rc` khỏi `tasks` (hiện **không** làm — `rt.rs:78`). |
| áp lực cap (số chưa chọn) | `Done`+hết handle, hoặc `Moi` quá hạn | (tuỳ A0) | Cap là **hàng rào bổ sung**, không thay luật thu hồi; **chưa chọn số** (`agent/DECISIONS.md:2449`, `:2520-2521`). |

### 3.4 Phân biệt tài nguyên task-owned vs handle thoát task **[CHỜ A0 CHỐT]**

| Tài nguyên | Owner đề xuất | Đóng/giải phóng khi | Có thoát task? | Căn cứ hiện trạng |
|---|---|---|---|---|
| Frame + locals + captures | Task | Xong / bị huỷ (Lát 3) | **Không** | VM `Frame` `crates/vis-vm/src/vm.rs:124-135`; đề xuất heap `vd249-async-design-draft.md:49-51` |
| Đăng ký I/O `(fd, ready, task_id)` | Scheduler/registry nội bộ | wake, error, hoặc cancel | **Không** | hiện chưa có registry; adapter thật `crates/vis-cli/src/net.rs:117-213` |
| Timer entry | Timer heap nội bộ | fire / tombstone huỷ | **Không** | `Instant` rải rác `net.rs:94,105`, `vis-interp/src/net.rs:225,251,357,361` |
| Handle mạng `"C<n>"` / `"L<n>"` | Host `Net` **+ giá trị ngôn ngữ** | **Chỉ `net.close`** hoặc kết thúc chương trình | **CÓ** | `net.rs:15-24`, `:46-50`, `:185-215`; handle là `Value::Str` trả về chương trình |
| Kênh `Rc<RefCell<Chan>>` | Giá trị ngôn ngữ (+ `Rc` chẩn đoán của Scheduler) | Khi chương trình không còn giữ; `close` không đánh thức ai hiện nay | **CÓ** | `rt.rs:54-72`, `:86`; nợ #16 `vd249-async-design-draft.md:98-99` |
| Result cache `kq` | Task, giữ theo §3.1 | Khi hết handle **và** join xong | **CÓ** (qua handle) | `rt.rs:51`, `:180-194`, `:227` |

**Kết luận chống hiểu nhầm [CHỜ A0 CHỐT]**: **không** implement "task ends ⇒ close every handle it opened". Mạng handle là chuỗi mờ do host cấp và trả cho chương trình (`net.rs:46-50`), có thể được `return` từ task rồi tiếp tục dùng ở task khác. Chỉ **đăng ký chờ** (fd+task_id, timer) mới bị gỡ khi task kết thúc/bị huỷ; socket/listener thì không.

---

## 4. Kế hoạch hook chẩn đoán — test/debug only **[CHỜ A0 CHỐT]**

A0 duyệt hook **chỉ test/debug**, **không** giao diện môi trường hỗ trợ phát hành (`agent/DECISIONS.md:2523-2524`).

| Bộ đếm | Định nghĩa đề xuất | Invariant assert ở cuối test |
|---|---|---|
| `live_tasks` | Số task chưa `Reclaimed`, chia theo trạng thái §3.2 | Bằng số task dự kiến; `Done` không còn handle + join xong ⇒ 0 |
| `frames` | Số frame còn giữ (VM `Frame` `vm.rs:124-135` + continuation interp đề xuất) | 0 sau final join |
| `io_regs` | Số đăng ký `(fd, ready, task_id)` còn sống | 0 sau mọi wake/error/cancel; **không** đếm handle mạng đã đóng hay chưa |
| `timers` | Số timer entry chưa fire / chưa tombstone | 0 sau khi deadline cuối cùng fire |
| `handles` | Số handle còn sống (task/result + net/channel) tuỳ nhóm | Theo kỳ vọng từng ca; phân biệt handle task vs handle tài nguyên |

**Hình thức đề xuất [CHỜ A0 CHỐT]**:
- Một hàm `diag_snapshot()` trong Rust API nội bộ, để **test trong workspace** gọi trực tiếp; **không** biến thành cờ CLI, **không** đọc biến môi trường phát hành (VD-250 cấm rõ "giao diện môi trường hỗ trợ phát hành").
- Chỉ bật khi `cfg(test)`/debug; bản release không expose; nếu cần instrument tạm, dùng cơ chế nội bộ rồi gỡ trước khi merge (khác với `--explain-perms` ở `net.rs:32-35` vốn là đầu ra debug có chủ đích).
- Chạy cùng hai engine: mỗi ca ownership assert snapshot **giống nhau** ở interp và VM; trace scheduling theo `vd250-determinism-contract.md` §3.2.
- Bổ sung test áp lực: spawn/await dài (đo B3 `vd249-async-design-draft.md:33`) để chứng minh `tasks` không tăng tuyến tính vô hạn — hiện `rt.rs:78` giữ mọi task.

---

## 5. Giữ nguyên A0 §2.3 (không tự huỳnh task lúc main exit)

**[Đã chốt — `agent/DECISIONS.md:2469-2473`]**

- **Final join vẫn xét theo thứ tự tạo** (`rt.rs:242-249`); task chưa await vẫn chạy (`test/sem/async/task-khong-await-van-chay.vis`).
- Task pending tiếp tục được chờ/điều phối theo **deadline hiện có**: `net.rs:94-109` (accept), `:120-126` (connect), `crates/vis-interp/src/net.rs:225,251,357,361` (HTTP); #148 chọn A — deadline tổng toàn lời gọi, supersede phần timeout của VD-183 (`agent/DECISIONS.md:2448`, `:2510-2516`, `agent/QUEUE.md:2062` ST-265).
- Nếu **chỉ còn chờ nội bộ không tiến triển** ⇒ xử lý theo **hợp đồng deadlock đã duyệt**: `VS0505` cho kênh deadlock (`crates/vis-cli/tests/vistest_async.rs:3`, `:144-152`); **không** tự tạo mã mới.
- **Socket chờ bên ngoài không phải deadlock** chỉ vì `ready` rỗng. Đây là lý do bác bỏ phương án "pending ⇒ ép huỷ" trong nháp cũ `vd249-async-design-draft.md:116` (đã bị VD-250 bác — `agent/DECISIONS.md:2469-2470`).

---

## 6. Bảng cancellation / first-error / dọn task còn lại — **thiết kế trước Lát 3, chưa hiện thực**

**[CHỜ A0 CHỐT TOÀN BẢNG]** — VD-250 yêu cầu bảng riêng trước Lát 3 (`agent/DECISIONS.md:2472-2473`). Dưới đây là **khung để A0 điền**, không phải quyết định.

| Kịch bản | Trạng thái → trạng thái | Kết quả cho awaiter | Hành động tài nguyên | Thứ tự dọn | Mã/chẩn đoán |
|---|---|---|---|---|---|
| Task đầu bị trap, các task khác đang chạy | `Running(A)`→`Done(Err)`; các task khác ? | Chưa quyết | Chưa quyết | Chưa quyết | Hiện `VS0506` bọc `rt.rs:233-239`; có dừng sớm không? |
| Huỷ tác vụ đang `Suspended(I/O)` | `Suspended`→`Cancelling`→? | Chưa quyết | Gỡ registry ngay; đóng socket khi frame tới điểm nhường (`vd249-async-design-draft.md:102-105`) | Chưa quyết | Ưu tiên `VS0505`/`VS0506`, **không** tạo mã mới |
| Huỷ tác vụ đang `Ready`/`Running` CPU | ? | ? | Không huỷ giữa lệnh CPU (đề xuất cũ `:105`) | ? | ? |
| Deadline tổng của HTTP call hết hạn | ? | ? | Đóng tài nguyên trên success/error/timeout (VD-250 §5 `agent/DECISIONS.md:2516`) | ? | Thông điệp timeout hiện có `net.rs:107`, `:202` |
| Main kết thúc mà task vẫn pending | Giữ final join; chờ deadline (§5) | ? | **Không** đóng handle mạng do task trả ra | ? | Không báo deadlock chỉ vì `ready` rỗng |
| Áp lực cap (số chưa đo) | ? | ? | Cap là hàng rào, không thay luật thu hồi | ? | ? |
| Handle cuối bị drop trước final join | `Done`→giữ `Done` (§3.1) | `Err`/`Ok` vẫn ở `kq` | Không thu hồi | — | — |

**Câu hỏi kèm theo [CHỜ A0 CHỐT]**: (1) first-error có dừng các task khác không, hay vẫn join hết như hiện tại (`rt.rs:242-249`)? (2) Ai là owner quyết định huỷ khi nhiều task cùng lỗi? (3) Cancellation có được phép là API ngôn ngữ không, hay chỉ nội bộ (VD-250: chưa mở API mới `agent/DECISIONS.md:2452`)? (4) Khi nào thì "nghĩa vụ join" được coi là tiêu trong kịch bản lỗi?

---

## 7. Câu hỏi gửi A0

| # | Câu hỏi | Khuyến nghị |
|---|---|---|
| O1 | Chấp nhận tách **handle reference** và **nghĩa vụ join** thành 2 điều kiện thu hồi (§3.1)? | Có; nếu không, task chưa await bị thu hồi sớm. |
| O2 | Bảng trạng thái §3.2 có cần thêm trạng thái `Joining` riêng không? | Không cần nếu nghĩa vụ join là flag, nhưng phải test được. |
| O3 | Hook `diag_snapshot()` nội bộ, không env/CLI — chốt chứ (§4)? | Chốt. |
| O4 | Phân loại tài nguyên §3.4: chỉ gỡ đăng ký chờ, không đóng handle mạng — chốt chứ? | Chốt (đúng VD-250 `agent/DECISIONS.md:2521-2522`). |
| O5 | Số cap đo trên workload nào (50k task B3 `vd249-async-design-draft.md:33`)? | Chưa chọn; đo trước khi đề xuất số. |
| O6 | Khi nào phải nộp bảng §6 cho A0? | **Trước Lát 3** (`agent/DECISIONS.md:2472-2473`); không hiện thực ở ST-266. |

## 8. Giới hạn phạm vi

Không sửa `rt.rs`, `net.rs`, `vm.rs`, không thêm hook vào sản phẩm, không đổi hành vi join/deadlock, không commit, không chạy gate.
