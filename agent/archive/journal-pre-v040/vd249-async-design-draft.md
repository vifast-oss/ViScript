<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-262 — Nháp thiết kế runtime async "thực dụng một luồng" (VD-249)

**Người ghi**: Agent 1 (subagent) · **Ngày**: 2026-11-20 · **Trạng thái**: **CHỈ THIẾT KẾ — KHÔNG merge, KHÔNG commit, KHÔNG sửa mã sản phẩm**
**HEAD khi đọc**: `812feda430b1b9394bb1a5adf2c2c4396675072c` · **baseline sản phẩm**: `dbae70a` ([ST-258] L0 qua nextest)
**Căn cứ**: `agent/QUEUE.md:2058` (ST-262), `agent/DECISIONS.md:2387-2401` (VD-249 §4), `agent/BAO-CAO-SAN-SANG-THUC-TE-2026-11-20.md` (đo đạc).
**Phạm vi**: một ngoại lệ **design-only** đối với lệnh "chưa mở VS3" — **không** mở sáu hạng mục VS3, **không** worker pool, **không** shared heap/GC, **không** TLS, **không** thay đổi ngữ nghĩa task đã chốt. Mọi phương án dưới đây là **đề xuất chờ A0 chốt**, không phải quyết định. Không dùng ước lượng dòng/tuần của ST-155 hay ST-212 làm cam kết.

---

## 0. Điểm xuất phát (có mã + số đo, không suy diễn)

| Thành phần | Hiện trạng (file:line) | Hệ quả |
|---|---|---|
| Runtime | `crates/vis-interp/src/rt.rs:2-11` — 249 dòng; `Task` (`Rc`) `:39-52`, `Scheduler { tasks, ready, tiep, sau, kenh }` `:77-87`, một luồng, tất định, run-to-completion | Không có Future/Poll/Waker/state-machine; `await` là lời gọi **đồng bộ** `host.goi_than(...)` tại `rt.rs:218-219` |
| Trạng thái task | `TrangThai { Moi, DangChay, Xong }` `rt.rs:26-35`; kết quả cache `kq: RefCell<Option<Result<Value, Trap>>>` `rt.rs:51` | Không lưu **vị trí thực thi** hay local frame → không thể suspend/giữa chừng rồi resume |
| Giới hạn lồng | `MAX_TASK_NEST = 256` `rt.rs:23-24` — mỗi `await` là một khung gọi thật | Mô hình state-machine sẽ gỡ chặn này ⇒ **đổi ngữ nghĩa `VS0501`** (phải A0 chốt) |
| Kênh | `Chan { cap, buf, closed }` `rt.rs:54-72`; `cap == 0` bị từ chối `VS0505` `rt.rs:128-138` | Không wait-set, không đánh thức theo kênh |
| Dùng chung | `crates/vis-vm/src/vm.rs:156,186,521-540` — VM reuse `vis_interp::rt::Scheduler`; `join_all` gọi ở `interp.rs:221` và `vm.rs:229` | Một nền tảng, hai đường biên dịch ⇒ **parity là bất biến cứng** |
| Interp frame | `host.rs:26-35`: `goi_than` → `call_hfn` ⇒ khung nằm trên **Rust call stack (ẩn)** | Đây là nơi khó suspend nhất |
| VM frame | `vm.rs:124-135`: `Frame { fi, pc, locals, captured, st_cu, writeback }` — **đã là khung heap, có PC**; `goi_ham_bytecode` `vm.rs:556,628` | VM gần với suspend/resume hơn interp; dễ lệch hai đường |
| Mạng CLI | `crates/vis-cli/src/net.rs:119-135` (`connect_timeout` chặn), `:168-213` (`accept` timeout = `set_nonblocking(true)` + `std::thread::sleep(5ms)` busy-poll `:94-116`, `read` dùng `set_read_timeout` `:190`), `write_all` chặn `:208-212` | Busy-poll là **cơ chế timeout**, không phải tích hợp scheduler |
| HTTP | `crates/vis-interp/src/net.rs:217-269` (GET), `:354-398` (HEAD) — vòng lặp `net.read` với `Instant` deadline, mọi bước chặn | Không có điểm nhường trong lời gọi HTTP |
| File | `crates/vis-interp/src/stdlib.rs:479-515` — `fs.read/write/exists/list` đi qua `FsHost`, đồng bộ | Chặn (but ngắn/local — xem §3, phần công bố phần còn chặn) |
| SQLite | `crates/vis-cli/src/db/sqlite/mod.rs` — `DbHost` (`builtin.rs:181-185`), timeout wall-clock ~5 s/câu (§14.11) | Chặn |
| Phụ thuộc | **0** crate tokio/async-std/rayon/futures/mio/crossbeam/smol/polling trong **11** `Cargo.toml`; `unsafe` = 0 dòng sản phẩm | Mọi lựa chọn dependency là quyết định mới, phải qua allowlist |
| Stack | `vis-cli/src/main.rs:505`, `vm.rs` tests `:124,288` — luồng OS stack **256 MiB**; lý do: đệ quy sâu `MAX_CALL_DEPTH = 2000` (`interp.rs:1399`), đo 10^4 vòng tràn stack | Stack lớn là **workaround đệ quy đồng bộ**, KHÔNG phải lời giải async |

**Số đo làm baseline (giữ nguyên để so sau mỗi lát cắt)** — `agent/BAO-CAO-SAN-SANG-THUC-TE-2026-11-20.md:125-149`:

- **B1 — 20 task × server trễ 200 ms**: `1 → 0,27 s`, `5 → 1,09 s`, `20 → 4,11 s` (interp) / `4,10 s` (VM); tăng **tuyến tính N × 200 ms**; `MARK` spawn sau vẫn in cuối.
- **B2 — CPU + I/O**: `CPU_DONE` và `IO_START` **cùng mốc ms** trên cả hai engine; `IO_DONE − IO_START = 201 ms`. Starvation trực tiếp.
- B3 (phụ): 50k task tạo+join ~0,28 s, RSS 24→29 MB; `Scheduler.tasks` giữ mọi task tới hết chương trình (`rt.rs:78`).

**Kết luận ngắn**: A0 đã loại bỏ đích "một luồng + mọi I/O chặn". Đề xuất dưới đây giữ **một luồng làm nền tảng đầu tiên**, nhưng nâng cấp thành **cơ chế**: suspend/resume thật, scheduler chạy task khác khi một task chờ, timer/deadline, cancellation có luật kết thúc, I/O mạng không chặn scheduler, giới hạn + vòng đời tài nguyên rõ ràng. Đây là **nâng cấp cơ chế, không phải tối ưu vi mô**; không phải song song nhân CPU trước.

---

## 1. Tám câu bắt buộc

Mỗi câu: **(a) hiện trạng + file:line**, **(b) cơ chế đích đề xuất**, **(c) quyết định mở cho A0**.

### 1.1 Suspend/resume — giữ trạng thái hàm, local, capture, vị trí (interp AND VM)

**(a) Hiện trạng.**
`Task` `rt.rs:39-52` chỉ có `id, than, args, ten, span, trang_thai, kq` — **không** có PC, không có frame, không có locals. `await_task` gọi `host.goi_than(...)` một lần rồi đặt `Xong` `rt.rs:218-220`. Ở interp, `goi_than` → `call_hfn` (`host.rs:26-35`) dùng call stack Rust **ẩn**; ở VM, `goi_than` (`vm.rs:526-532`) → `goi_ham_bytecode` (`vm.rs:556,628`) đẩy `Frame { fi, pc, locals, captured, st_cu, writeback }` (`vm.rs:124-135`) — khung heap **đã có PC**. `spawn` chỉ nhận **tên `async fn` cấp tệp** (`interp.rs:1028-1043`), không nhận closure tùy ý — đây là điểm thuận lợi: thân task luôn là một hàm có biên rõ.

**(b) Cơ chế đích (đề xuất).**
- Đổi đơn vị thực thi từ "chạy thân một phát" thành **frame có thể tạm dừng**: `Task` giữ một `Resume` enum nội bộ `{ ChuaBatDau, TaiDiem(pc/statement), DangChay, Xong }`.
- **VM**: tái dùng `Frame` có sẵn (`vm.rs:124-135`) — `pc` là vị trí, `locals`/`captured`/`st_cu` là trạng thái, `writeback` (`vm.rs:131-134`) bảo toàn capture `var` giữa các lần resume. Khi gặp điểm chờ, **giữ nguyên Vec<Frame> trong task** thay vì để hàm trả về; khi wake, tiếp tục vòng lặp tại `fr.pc` (`vm.rs:716-735`).
- **Interp**: cần đưa phần lớn trạng thái eval ra **explicit continuation stack** (heap) thay vì Rust call stack. Lát đầu thu hẹp surface: chỉ suspend tại **điểm `await` và điểm gọi builtin I/O là biểu thức câu lệnh trực tiếp** trong thân `async fn`; local/capture đã nằm trong env `Rc<RefCell<...>>` (`vd212-thiet-ke-ghi-chu.md:14-16`) nên sống qua suspend; vị trí = chỉ số lệnh/câu trong thân `HFn`. Suspension trong biểu thức lồng sâu (vd. `f(g(await x))`) **chưa** hỗ trợ ở lát đầu — phải công bố là giới hạn.
- `MAX_TASK_NEST = 256` (`rt.rs:24`) không còn là độ sâu Rust stack mà là độ sâu cây await trên heap; ngữ nghĩa `VS0501` khi vượt phải được A0 định nghĩa lại (xem §1.7, §4).

**(c) A0 chốt.** (i) Chấp nhận đích "stackless frame trên heap" đúng định hướng `docs/ViScript-thiet-ke.md:947` không? (ii) Có cho phép **thu hẹp điểm suspend** ở lát đầu (chỉ câu lệnh trực tiếp) và công bố giới hạn, hay yêu cầu full expression-level continuation ngay? (iii) Giữ hay đổi `MAX_TASK_NEST`/`VS0501` — nếu đổi, định nghĩa ca run-to-completion cũ còn đúng hay không?

### 1.2 Scheduler — task nhường ở đâu, task CPU dài chiếm bao lâu

**(a) Hiện trạng.**
`Scheduler.ready: VecDeque<usize>` FIFO `rt.rs:79`; `spawn` đẩy vào `ready` `rt.rs:112-116`; `chay_mot` chỉ rút task `Moi`, chạy tới hoàn tất `rt.rs:157-175`; `join_all` join theo thứ tự tạo `rt.rs:242-249`. **Không có preempt, không time-slice, không chống đói** (`BAO-CAO...:90`). `rt.rs:4-5` ghi nợ nguyên văn. Task CPU dài chiếm runtime đến khi xong hoặc dính step cap (`vm.rs:96-97,736-740`, `interp.rs:1399`) — đo được B2.

**(b) Cơ chế đích (đề xuất).**
- `ready` chứa **task có frame đã sẵn sàng tiếp tục**, không chỉ task `Moi`. `poll` một task = chạy frame tới **điểm nhường** rồi trả lại `ready` (nếu vẫn còn việc) hoặc đưa vào wait-set (nếu chờ I/O/timer).
- Điểm nhường: (1) `await` trên task khác; (2) builtin I/O đăng ký chờ; (3) **hạn ngạch CPU** — một task chỉ chạy tối đa `Q` "đơn vị" rồi tự xếp cuối `ready`.
- **Đơn vị `Q` chưa được chọn.** Không được tái dùng ngay bộ đếm `steps` hiện có: ST-260 phát hiện interp đếm **vòng lặp**, VM đếm **từng lệnh**, cùng ngưỡng 10^7 ⇒ 2M vòng interp exit 0 / VM trap (`BAO-CAO...:170+`). Gắn nhường vào `steps` sẽ **phá parity**. Đề xuất đơn vị mới, xác định ở cả hai engine (vd. "số lần vào thân hàm/lần quay lại loop backedge"), do A0 chốt; hạn ngạch là **số**, không phải thời gian (thời gian = mất tất định).
- Reactor/timer **chỉ báo "đã sẵn sàng"**, không tự sắp thứ tự; thứ tự vẫn do `ready` của ta quyết ⇒ giữ tất định.

**(c) A0 chốt.** (i) Nhường **chỉ tại await/I/O** (đủ cho B1) hay **có hạn ngạch CPU** ngay (cần cho B2)? (ii) Đơn vị đếm hạn ngạch là gì, có chấp nhận tách khỏi `steps` hiện có và xử lý riêng lệch parity ST-260 không? (iii) Giá trị `Q`/số task chạy mỗi lượt — A0 chọn cơ chế, con số cụ thể để tiến hành đo hiệu chỉnh, không cam kết ở tài liệu này.

### 1.3 I/O — connect/accept/read/write đăng ký chờ và được đánh thức

**(a) Hiện trạng.**
`NetHost` (`builtin.rs:176-179`, impl `vis-cli/src/net.rs:119-213`): `connect_timeout` chặn `:120-135`; `accept` timeout dùng nonblocking + `sleep(5ms)` busy-poll `:85-116`; `read` dùng `set_read_timeout` `:185-206`; `write_all`/`flush` chặn `:208-213`. Không epoll/kqueue/selector/mio; mạng **loopback-only** (`chi_loopback()` `net.rs:122,138`). HTTP gọi `net.read` trong vòng lặp `vis-interp/src/net.rs:230-258,360-385`.

**(b) Cơ chế đích (đề xuất).**
- Tại host CLI, thay chỗ "chặn" bằng: đặt socket **nonblocking** → **đăng ký** `(fd, Ready::readable|writable, task_id)` vào một `Registry` nội bộ nằm trong runtime → trả `Pending` cho scheduler. Khi selector báo readiness, đẩy `task_id` vào `ready` theo FIFO; task resume tại frame và thử lại syscall.
- **`accept` timeout**: bỏ `sleep(5ms)` `net.rs:109`; đăng ký listener readable + một deadline timer; wake bởi kết nối hoặc bởi timer — cùng một đường.
- **`connect`/`write`**: ở lát đầu vẫn có thể chặn ngắn (loopback) — xem §3, phần "phần còn chặn" — nhưng **được phép** đăng ký writable theo cùng cơ chế; không đổi chữ ký ngôn ngữ.
- **Không thêm method public vào `trait Host`/`NetHost`**: registry là kiểu `pub(crate)`/private, giữ trong `Scheduler`; adapter nằm ở tầng host (`vis-cli`), giao tiếp qua kiểu nội bộ. ViScript vẫn thấy `net.read(...)` là lời gọi **đồng bộ về mặt ngữ nghĩa** — chỉ có task bị suspend, không phải toàn runtime.
- Reusable: deadline của toàn HTTP call dùng "thời gian còn lại" cho mỗi bước, khớp VD-249 §5 (`DECISIONS.md:2403-2408`).

**(c) A0 chốt.** (i) Đồng ý giữ bề mặt ngôn ngữ đồng bộ, chỉ đổi bên trong host (không thêm API công khai) — đúng không? (ii) Lát đầu đưa `read`/`accept` vào reactor, còn `connect`/`write` chặn — chấp nhận không? (iii) Cho phép đưa dependency selector mỏng (xem phương án A/B §2) — cần rà allowlist trước khi code.

### 1.4 Timer — monotonic clock, deadline, nhiều timer

**(a) Hiện trạng.**
`Instant::now()` đã dùng rải rác: `vis-cli/src/net.rs:94,105`, `vis-interp/src/net.rs:225,251,357,361`; mỗi chỗ tự tính `han`, tự clamp (`con.as_millis().clamp(1, 2_000)` `net.rs:258,368`). **Không có timer tập trung**, không heap deadline; `accept` poll 5 ms. HTTP timeout "tổng" chưa có hợp đồng thống nhất (VD-249 §5 giao chốt riêng).

**(b) Cơ chế đích (đề xuất).**
- Một **timer heap** (`BinaryHeap<Reverse<(Instant, u64 task_id, u64 gen)>>`) trong runtime; `Instant` (monotonic) là khóa; tie-break `(deadline, task_id)` để tất định.
- Mỗi vòng scheduler hỏi `deadline` nhỏ nhất làm timeout cho selector; khi đến hạn, đẩy task vào `ready` kèm cờ `TimedOut` để builtin trả thông điệp timeout hiện có (không đổi mã lỗi).
- Nhiều timer: **một heap, không một thread/timer**; hủy bằng `gen` tombstone (xóa lười) hoặc indexed handle — A0 chọn. Deadline HTTP tổng = một `Instant` cho toàn lời gọi, mỗi `read` nhận `han − now` (`net.rs:251`) thay vì `timeout` đầy đủ.
- Không dùng `std::thread::sleep` cho scheduling (chỉ có thể còn ở adapter test).

**(c) A0 chốt.** (i) Hợp đồng "deadline monotonic cho toàn lời gọi, không gia hạn vô hạn" (VD-249 §5) áp dụng cho timer runtime này đúng không? (ii) Hủy timer bằng tombstone hay indexed handle (đổi độ phức tạp/parity)? (iii) Khi timer và I/O cùng sẵn sàng, ưu tiên ai — cần chốt để byte-identical.

### 1.5 Cancellation — gỡ đăng ký, cleanup khi nào

**(a) Hiện trạng.**
**Không có cancellation API**. Task chỉ kết thúc khi thân trả về hoặc trap; `await_task` bọc lỗi `VS0506` `rt.rs:232-239`; join cuối chạy mọi task `rt.rs:242-249`. Kênh `close` không đánh thức ai (nợ #16, `st155-vs3-dieu-tra.md:46-53`). Không wait-set ⇒ không có gì để gỡ đăng ký.

**(b) Cơ chế đích (đề xuất).**
- Huỷ là **hp (cooperative) tại điểm nhường**: đánh dấu task `Huy`, **ngay lập tức** gỡ fd khỏi `Registry` và đưa timer vào tombstone, nhưng **dọn tài nguyên do task giữ** (socket/listener, buffer) khi frame chạy tới điểm `await`/I/O tiếp theo hoặc khi bị ép kết thúc — tránh đóng fd giữa syscall.
- Với task đang chờ I/O: wake do cancel → frame thấy cờ `Huy` → trả `Err(Abort::...)` theo mã **có sẵn** (ưu tiên tái dùng `VS0505`/`VS0506`; **không tự tạo mã mới**), rồi drop frame.
- Không có GC (`rt.rs:10`, `vd212...:14-16`): chu trình `Rc` vẫn là nợ đã biết; cancellation phải định nghĩa rõ resource owner = task, và scope cha huỷ con (định hướng `docs/ViScript-thiet-ke.md:758-760`) — nhưng **scope là tính năng ngôn ngữ mới**, không mở trong feature này; phạm vi dừng ở mức runtime nội bộ.
- Điểm huỷ chỉ tại await/I/O ⇒ tất định; không huỷ giữa lệnh CPU.

**(c) A0 chốt.** (i) Chấp nhận cancellation **hp tại điểm nhường**, không preempt giữa lệnh? (ii) Mã lỗi/trạng thái khi task bị huỷ dùng mã cũ nào (cấm tự thêm API/mã mới)? (iii) Có cần luật "scope cha huỷ con" ngay không, hay để sau (khuyến nghị: để sau, đúng giới hạn feature)?

### 1.6 Vòng đời — task lỗi, task không await, kết thúc chương trình, tài nguyên còn mở

**(a) Hiện trạng.**
Lỗi thân task → `VS0506` với **mã gốc + span lúc tạo** `rt.rs:225,232-239`; cache trong `kq` `rt.rs:51,180-194,227`. Task không await **vẫn được join ở cuối** theo thứ tự tạo `rt.rs:242-249`, gọi ở `interp.rs:221` / `vm.rs:229` — ngữ nghĩa này **phải giữ** (VD-249: "không đổi ngữ nghĩa task không await hoặc lỗi task"). `Scheduler.tasks` giữ mọi task tới hết chương trình `rt.rs:78`; không dọn theo task.

**(b) Cơ chế đích (đề xuất).**
- Giữ nguyên: cache kết quả, `VS0506` (mã gốc + span), join cuối theo thứ tự tạo cho mọi task đã tạo.
- Bổ sung luật cho **task còn đang chờ khi main kết thúc**: chạy một lượt `ready`, rồi (a) ép huỷ mọi task còn pending, đóng tài nguyên, báo số task chưa xong theo chẩn đoán **có sẵn** — **hoặc** (b) chờ tới deadline nhỏ nhất rồi huỷ. Khuyến nghị (a) để không treo chương trình.
- Tài nguyên còn mở: runtime giữ danh sách fd/handle do task đăng ký; khi kết thúc (bình thường hoặc huỷ) **đóng toàn bộ** và kiểm tra đếm về 0 ở test. Không đổi `join_all` thành "bỏ task chưa xong" — chỉ đổi cách task chờ được giải quyết.
- Ghi rõ: bộ nhớ của frame được giải phóng cùng task; **không** hứa thu hồi chu trình `Rc` (không GC).

**(c) A0 chốt.** (i) Task pending lúc kết thúc: ép huỷ ngay (a) hay chờ deadline (b)? (ii) Dùng thông điệp/mã chẩn đoán nào cho "task chưa xong" — tái dùng mã cũ hay A0 cấp mã mới (đây là thay đổi hợp đồng, phải chốt trước khi code)? (iii) Có yêu cầu đóng handle loopback (`net.rs:215-219`) bắt buộc ở mọi đường thoát không?

### 1.7 Giới hạn — số task, hàng đợi, buffer, tài nguyên đang chờ

**(a) Hiện trạng.**
`MAX_TASK_NEST=256` `rt.rs:24` (độ sâu await, **không** phải số task); `Scheduler.tasks` **không giới hạn** `rt.rs:78`; `ready` là `Vec` phẳng (ghi chú "10.000 tác vụ không đệ quy" `rt.rs:74-75`); kênh có `cap` `rt.rs:54-72`; `NET_READ_CAP` (`vis-interp/src/stdlib.rs`, dùng `net.rs:194`), `MAX_HEADER` 64 KiB (`net.rs:20-24`), `MAX_STEPS=10^7` (`vm.rs:96-97`), `MAX_CALL_DEPTH=2000` (`interp.rs:1399`). Không backpressure, không cap tổng số fd/timer.

**(b) Cơ chế đích (đề xuất).**
- Đặt các cap **nội bộ, rõ ràng**: `max_tasks`, `max_ready`, `max_pending_io`, `max_timers`, `max_total_pending_bytes`, giữ `NET_READ_CAP`/`MAX_HEADER`. Vượt cap ⇒ **trap tất định ngay** (không hang, không OOM, không panic) — tái dùng mã lỗi hiện có hoặc A0 cấp.
- Mỗi builtin I/O có buffer riêng bounded; không cấp phát theo `usize` caller (đã có tiền lệ `net.rs:192-194`).
- Đo số task/tài nguyên **sống** bên cạnh RSS (VD-249 §6, `DECISIONS.md:2410-2414`) — baseline B3 cho thấy `tasks` giữ mọi task, cần phân biệt "giữ để cache" với "rò rỉ".
- 256 MiB stack (`main.rs:505`): **đánh giá riêng** — nó phục vụ đệ quy đồng bộ (`interp.rs:1399`), **không** phải nhu cầu của async. Với frame trên heap, suspend/resume không cần stack OS lớn. **Không giữ nó như lời giải dài hạn**, nhưng cũng **không tháo trong feature này** (cần đo/đổi thiết kế đệ quy riêng).

**(c) A0 chốt.** (i) Các cap là **mặc định cứng** hay opt-in (theo tiền lệ `--strict-caps` opt-in ở VS2 — `ViScript-thiet-ke.md:1613`)? (ii) Con số mặc định do A0 ấn định sau khi đo, hay cho phép runtime tự đặt và ghi trong báo cáo? (iii) Xác nhận không tháo stack 256 MiB trong feature này, chỉ ghi nợ đánh giá riêng?

### 1.8 Host — phần chung vs adapter Linux/Windows

**(a) Hiện trạng.**
`trait Host` `builtin.rs:166-210` là `&mut dyn`, `Value` dùng `Rc`/`RefCell` ⇒ `!Send` (`st155-vs3-dieu-tra.md:9-11,21-22`). `Scheduler` nằm trong `vis-interp` (`rt.rs`), VM chỉ mượn (`vm.rs:156,186`). Host cụ thể: `vis-host-linux` (`vis-host-linux/Cargo.toml`), `vis-host-wasm` (`vis-host-wasm/Cargo.toml`), CLI impl `vis-cli/src/net.rs`. Windows chạy qua Git Bash trên psvn, **không có CI** (`st155-vs3-dieu-tra.md:128-156`). Không có selector backend nào.

**(b) Cơ chế đích (đề xuất).**
- **Dùng chung** (trong `vis-interp::rt`, nội bộ): `Scheduler` mở rộng, `Resume`/frame contract, timer heap, registry facade, thứ tự `ready` tất định, luật cancellation/lifecycle. Interp và VM **cùng** một bản — đúng VD-95.
- **Adapter theo nền** (nằm ở host crate, không lọt vào ngôn ngữ): backend selector — Linux `epoll`, macOS `kqueue`, Windows `IOCP`/`WSAEventSelect` — ánh xạ về một trait nội bộ `EventSource { register, deregister, wait(timeout) -> Vec<Ready> }`. Nếu chọn dependency (§2), adapter là phần của dependency đã xử lý; ta chỉ viết lớp mỏng map handle.
- **Windows**: không bắt buộc trong lát đầu; định nghĩa **hợp đồng adapter** (trait + ca giả) để sau này cắm backend mà không sửa runtime. Không mở mục "Windows hỗ trợ thật" (VS3-5) ở đây.
- **Giữ**: `Value`/`Task`/`Chan` không `Send`; `Host` vẫn `&mut dyn`; **không** `unsafe` (trừ khi A0 cấp ngoại lệ D2-17 — không mặc định).

**(c) A0 chốt.** (i) Chọn backendSelector ngay cho cả 3 nền hay chỉ Linux/macOS ở lát đầu, Windows để hợp đồng? (ii) Có chấp nhận dependency cung cấp adapter OS (xem §2) nhưng **vẫn tự giữ thứ tự `ready`** không? (iii) Xác nhận không đổi `Host`/`Value` sang `Send + Sync` trong feature này (đúng giới hạn VS3).

---

## 2. So sánh phương án + dependency (không mặc định Future/Poll/Waker, không mặc định tự viết)

Ba phương án dưới đều **một luồng**, đều **không** worker pool/shared heap/GC, đều giữ bề mặt ngôn ngữ hiện có. Khác biệt nằm ở (1) ai tạo frame suspend được, (2) ai làm selector OS, (3) ai giữ quyền sắp thứ tự wake.

### Phương án A — State-machine hạ xuống + selector OS mỏng (**đề xuất — chờ A0 chốt**)

- **Cơ chế**: hạ thân `async fn` thành state-machine stackless (heap frame). Interp: explicit continuation (xem §1.1). VM: tái dùng `Frame { pc, locals, ... }` `vm.rs:124-135`. Reactor: crate selector mỏng — ứng viên **`mio` 1.x** hoặc **`polling` 3.x** (đều MIT/Apache-2.0, thuần Rust, không C toolchain) — (**phải** xác nhận bằng `cargo metadata` + allowlist QD §9.3 trước khi dùng). Timer: heap `Instant` tự viết, không dependency.
- **Dependency**: 1 crate selector (+ transitive nhỏ). **Không** kéo tokio/async-std.
- **Trade-off**: kiểm soát **toàn bộ** thứ tự wake ⇒ tất định và parity tốt nhất; chi phí nằm ở khâu hạ frame (hai đường interp/VM) + một lớp adapter fd.
- **Determinism/parity**: cao — scheduler là ta, selector chỉ báo readiness; tie-break timer theo `(deadline, task_id)`. Rủi ro parity vẫn ở chỗ interp frame ẩn vs VM frame rõ (§1.1).
- **Điểm yếu**: phải tự viết phần "đợi + timer + huỷ"; thay đổi biểu diễn frame VM có thể cần opcode/định dạng lệnh mới (ảnh hưởng cache hash `ViScript-thiet-ke.md:952`) — **phải A0 duyệt riêng**, không làm trong bước design-only này.

### Phương án B — Event loop có sẵn + lớp task tự viết

- **Cơ chế**: dùng local executor của **`smol`** (LocalExecutor) hoặc **`tokio` current-thread** làm reactor + timer; `Task` của ta ánh xạ tới một Rust `async` block do host poll; `Scheduler` vẫn là facade giữ `ready`.
- **Dependency**: `smol` (MIT/Apache) hoặc `tokio` (MIT) + transitive (có thể đáng kể); cross-platform epoll/kqueue/IOCP **được lo sẵn**.
- **Trade-off**: ít mã hệ thống nhất, adapter Windows gần như miễn phí; nhưng **thứ tự wake do dependency quyết** — để khôi phục tất định phải tự xếp hàng lại sau mỗi wake (đang "chống lại" event loop); vẫn phải hạ frame ViScript → dạng poll được (không thể sinh Rust `async` từ bytecode lúc chạy), nên không tiết kiệm khâu khó nhất.
- **Determinism/parity**: trung bình — phụ thuộc vào việc ta có ép được FIFO qua dependency không; parity hai engine vẫn do ta đảm bảo.
- **Điểm yếu**: rủi ro allowlist/cây phụ thuộc lớn hơn A; "đói" do event loop có thể lọt vào nếu không kiểm soát.

### Phương án C — Generator/coroutine transform, **không** dependency event loop

- **Cơ chế**: vẫn hạ stackless frame + `yield` tại await/I/O, nhưng tự làm readiness bằng `std` — thực tế là **nonblocking + busy-poll `sleep(5ms)`** như `vis-cli/src/net.rs:109`, hoặc gọi trực tiếp OS API qua `libc`/`windows-sys` (cần `unsafe`, cổng D2-17 + QD §9.3).
- **Dependency**: 0 crate mới (hoặc `libc` đã nằm trong `Cargo.lock` nhưng 0 dòng dùng — `st155...:107`).
- **Trade-off**: ít rủi ro giấy phép nhất; **CPU phí nhất** — busy-poll lặp lại đúng hành vi B1 (`4,11 s`, N × 200 ms) và không đạt tiêu chí lát 2; nếu dùng OS API thì cần ngoại lệ `unsafe` và tự làm 3 backend.
- **Determinism/parity**: cao nếu tự giữ `ready`, nhưng **không đạt mục tiêu hiệu năng**; phù hợp làm **spike chứng minh suspend/resume** (lát 0), không phải đích.
- **Điểm yếu**: loại trừ làm kiến trúc đích.

### Bảng so sánh nhanh

| Tiêu chí | A (state machine + selector mỏng) | B (event loop có sẵn + task layer) | C (generator, 0 dependency) |
|---|---|---|---|
| Dependency mới | 1 selector mỏng | 1 event loop + transitive | 0 (hoặc `libc` + `unsafe`) |
| Ai sắp wake | **Ta** | Dependency (phải ép lại) | Ta |
| Tất định / parity | Cao | Trung bình | Cao |
| Cross-platform | Trung bình (viết adapter) | Cao (dep lo) | Thấp |
| Giấy phép | Phải rà 1 crate | Phải rà cả cây | Gần như sạch |
| Đạt chuẩn B1 (~0,3 s) | Có | Có | **Không** (busy-poll) |
| Rủi ro lớn nhất | hai đường frame interp/VM | mất kiểm soát thứ tự | CPU + `unsafe` |

### Đề xuất (chờ A0 chốt)

**Chọn A làm phương án đề xuất; B là phương án thay thế nếu A0 ưu tiên phủ OS nhanh hơn kiểm soát tất định; C giữ lại làm spike lát 0, không làm đích.** Lý do: A đáp ứng đúng ba use case ưu tiên (script tự động hoá → suspend/timer/huỷ; nhiều task chờ mạng → I/O không chặn; CPU+I/O → hạn ngạch) mà **không** mở worker/GC/API mới, và trao quyền sắp thứ tự cho ta — điều mà hợp đồng "byte-identical interp↔VM" (`diff_100k`, `test/sem/async/*.vis`) đòi hỏi. Mọi lựa chọn dependency phải qua `cargo metadata` + allowlist trước khi có dòng mã.

---

## 3. Lát cắt hiện thực được (runnable slices) — có tiêu chí, không có ước lượng cũ

Nguyên tắc: mỗi lát **chạy được và đo được độc lập**; B1/B2 luôn được chạy lại làm baseline (kể cả khi lát đó chưa sửa được chúng — ghi rõ "chưa đổi, kết quả giữ nguyên"). Không đính kèm số dòng/tuần.

**Lát 0 — Spike frame (không đổi mã sản phẩm)**
- Việc: prototype trong harness/test: pause/resume một `async fn` tại một `await` giả; chứng minh VM dùng `Frame.pc` `vm.rs:124-135`, interp dùng continuation tạm; thử nghiệm 2 mức suspend (câu lệnh trực tiếp vs biểu thức lồng).
- Tiêu chí: task A tạm dừng → task B chạy → A resume; output **byte-identical** với chạy tuần tự hiện tại trên cả hai engine; xác định rõ giới hạn suspend ở interp. Chạy B1/B2 để ghi baseline trước (4,11 s; cùng mốc ms).

**Lát 1 — Scheduler có frame + nhường tại await/I/O (I/O vẫn chặn thật)**
- Việc: `ready` chứa task có frame sẵn sàng; tách `await_task` `rt.rs:178-230` thành "bắt đầu / tiếp tục"; inject `Pending` giả từ test. **Chưa** đụng reactor.
- Tiêu chí: ca synthetic "task chờ giả + MARK spawn sau" ⇒ `MARK` in **trước** khi task chờ xong; **35 ca** `test/sem/async/*.vis` + `diff_100k` không đổi; `VS0506`, cache, join theo thứ tự tạo, task không await — giữ nguyên. B1/B2 **vẫn 4,11 s / cùng ms** (đúng dự kiến, ghi rõ chưa sửa I/O).

**Lát 2 — I/O mạng loopback không chặn (đạt B1)**
- Việc: nối `Registry`/selector vào `vis-cli/src/net.rs` cho `read` + `accept` (bỏ `sleep(5ms)` `net.rs:109`); `connect`/`write` giữ chặn ngắn, **công bố rõ**; timer heap tối thiểu cho deadline.
- Tiêu chí: B1 `20 × 200 ms` từ **4,11 s → ~một lần latency server (~0,3 s)** trên **cả hai engine**; 20/20 đọc ok; thứ tự output tất định; fixture async không đổi. B2 vẫn đói (chưa làm lát 4) — ghi rõ.

**Lát 3 — Timer/deadline + cancellation + cleanup**
- Việc: timer heap tập trung, deadline tổng HTTP dùng thời gian còn lại (`vis-interp/src/net.rs:225,251`); huỷ tại điểm nhường, gỡ fd/timer; đóng handle lúc kết thúc; luật task pending lúc main exit (theo A0 chốt §1.6).
- Tiêu chí: timeout fires **không có busy-poll** (đếm số lần `sleep` = 0); sau huỷ, số fd/timer đăng ký về đúng baseline; task bị huỷ trả mã cũ, không panic; server nhỏ giọt **không gia hạn vô hạn** (VD-249 §5).

**Lát 4 — Hạn ngạch CPU chống đói (đạt B2)**
- Việc: điểm nhường theo đơn vị do A0 chốt (§1.2), **tách khỏi `steps`** đang lệch interp/VM; xử lý/ghi rõ phát hiện ST-260.
- Tiêu chí: B2 `IO_START` **xuất hiện trước `CPU_DONE`**, task CPU vẫn hoàn tất, `IO_DONE − IO_START ≈ 201 ms`; output byte-identical interp↔VM; step cap 10^7 / `MAX_CALL_DEPTH` 2000 không bị né trái phép.

**Lát 5 — Giới hạn, vòng đời, đo dài**
- Việc: các cap nội bộ §1.7; backpressure trap tất định; đo số task/tài nguyên sống (VD-249 §6); đánh giá riêng stack 256 MiB (không tháo).
- Tiêu chí: vượt cap ⇒ trap có chẩn đoán, không hang/OOM; chạy dài số task/tài nguyên sống ổn định (phân biệt cache vs rò rỉ); mọi handle đóng ở mọi đường thoát; không lệch parity.

**Phần còn chặn — công bố ngay (không bắt nonblocking ở lát 1):**
- `fs.read/write/exists/list` (`stdlib.rs:479-515`), SQLite (`vis-cli/src/db/sqlite/mod.rs`, timeout ~5 s/câu), `exec.*` (`builtin.rs:187-197`) và `connect`/`write` mạng vẫn **đồng bộ** ở lát 1, chỉ dần chuyển ở lát 2/5. Lý do theo ưu tiên use case: script tự động hoá (dùng file/SQLite/process/local) cần chủ yếu **suspend/timer/huỷ**, không cần I/O file nonblocking; đo lại trước khi quyết định mở rộng. Không mở TLS/HTTP2/registry (VS3-3/6), không mở `chan(0)`/select (VS3-2).

---

## 4. Bất biến, ảnh hưởng ngôn ngữ/API/bytecode, rủi ro & ẩn số

### Bất biến phải giữ (đỏ là phải trình A0 trước khi sửa)
1. Byte-identical interp↔VM cho mọi fixture hiện có; `diff_100k` lech = 0; L0 hiện hành không giảm số ca xanh.
2. `spawn` không chạy tới `await`/join; join cuối **theo thứ tự tạo** (`rt.rs:242-249`); task không await vẫn được join; không đổi ngữ nghĩa này.
3. Lỗi task vẫn `VS0506` giữ **mã gốc + span lúc tạo** (`rt.rs:225,232-239`); cache `kq` trả lại lần hai.
4. `chan(0)` ⇒ `VS0505` (`rt.rs:128-138`); `recv` sau close ⇒ `None`, `send` sau close ⇒ `VS0504`; step cap 10^7 (`vm.rs:96-97`), `MAX_CALL_DEPTH` 2000 (`interp.rs:1399`).
5. Mạng loopback-only (`vis-cli/src/net.rs:122,138`); không TLS/HTTP2.
6. **Không** thêm public API: mọi kiểu/method mới là nội bộ (`pub(crate)`/private); `trait Host`/`Value` không đổi, không `Send+Sync`, không `unsafe` (trừ ngoại lệ A0), không worker pool, không GC.
7. **Không** đổi Cargo VERSION/opcode trong bước này; mọi thay đổi bytecode (nếu có) phải A0 duyệt + cập nhật hash cache (`ViScript-thiet-ke.md:952`).

### Ảnh hưởng dự kiến
- **Ngôn ngữ**: bề mặt giữ nguyên (`async fn`, `spawn`, `await`, `net.*`). Có thể đổi chi tiết `MAX_TASK_NEST`/`VS0501` (§1.1) và thêm thông điệp cho "task pending lúc exit"/"vượt cap" (§1.6-1.7) — **đều phải A0 chốt**.
- **API**: chỉ nội bộ; adapter selector nằm trong host crate, không lọt vào `pub trait`.
- **Bytecode**: VM đã có `Frame` có PC (`vm.rs:124-135`) nên có thể không cần opcode mới nếu tái dùng `Call`/`Return` + marker nội bộ; **nếu** cần lệnh `Suspend/Yield` thật thì đó là thay đổi định dạng + cache — báo cáo riêng, không tự làm.

### Rủi ro / ẩn số
- **Parity frame**: interp dùng Rust stack ẩn (`host.rs:26-35`), VM dùng khung heap (`vm.rs:124-135`) — hai cơ chế suspend dễ lệch output. Đây là rủi ro số 1.
- **Đơn vị hạn ngạch**: không được gắn vào `steps` vì interp đếm vòng/VM đếm lệnh (`BAO-CAO...:170+`); cần A0 chốt đơn vị mới.
- **Dependency & giấy phép**: allowlist QD §9.3 nằm ở repo `ViFastOs` (không có tệp ở đây); phải chạy `cargo metadata` xác nhận mọi crate/transitive trước khi cam kết A/B.
- **Cancellation/Rc**: không GC ⇒ chu trình `Rc` vẫn là nợ; định nghĩa owner tài nguyên = task, dọn tại điểm nhường.
- **Đo lường**: cần đo số task/tài nguyên sống, không chỉ RSS; B1/B2 phải lặp và báo phân bố (VD-249 §6).
- **Windows**: không có CI, adapter chưa định nghĩa (`st155...:128-156`); chỉ đặt hợp đồng ở feature này.
- **Stack 256 MiB**: giữ tạm, ghi nợ đánh giá; không dùng làm lời giải async dài hạn.
- **Ngữ nghĩa timeout tổng** (VD-249 §5): nếu SPEC đã chốt nghĩa khác, phải trình sửa hợp đồng trước khi đổi timer.

---

## 5. Phiếu trình Agent 0 (chốt rồi mới triển khai)

| # | Quyết định cần A0 | Phương án đề xuất | Thay thế | Mục |
|---|---|---|---|---|
| Q1 | Kiến trúc đích | **A** — state-machine + selector mỏng | B (event loop có sẵn); C chỉ làm spike | §2 |
| Q2 | Suspend interp | Thu hẹp điểm suspend ở lát đầu (câu lệnh trực tiếp), công bố giới hạn | Full expression-level ngay | §1.1 |
| Q3 | `MAX_TASK_NEST`/`VS0501` | Giữ ngữ nghĩa cũ tới khi có ca mới | Định nghĩa lại khi state-machine lên heap | §1.1, §4 |
| Q4 | Hạn ngạch CPU | Có, đơn vị mới tách khỏi `steps` | Chỉ nhường tại await/I/O | §1.2 |
| Q5 | Bề mặt I/O | Giữ đồng bộ, đổi trong host; `read`/`accept` trước | Đổi `NetHost` (public API) — không khuyến nghị | §1.3 |
| Q6 | Timer & huỷ | Heap `Instant`, tie-break `(deadline, task_id)`; huỷ hp tại điểm nhường | Event-loop timer (B); tombstone vs indexed handle | §1.4-1.5 |
| Q7 | Kết thúc chương trình | Ép huỷ task pending, đóng handle, báo bằng mã cũ | Chờ deadline rồi huỷ; cấp mã mới | §1.6 |
| Q8 | Cap | Nội bộ, A0 ấn số sau khi đo; trap tất định | Opt-in như `--strict-caps` | §1.7 |
| Q9 | Host | Shared runtime trong `vis-interp::rt`; adapter selector theo nền; Windows chỉ hợp đồng | Làm cả 3 nền ngay | §1.8 |
| Q10 | Phụ thuộc | Rà `cargo metadata` + allowlist trước khi chọn crate | Dùng `libc`/`unsafe` (cần ngoại lệ D2-17) | §2 |

**Đề xuất triển khai sau khi chốt**: lần lượt Lát 0 → 5 (§3), mỗi lát chạy B1 + B2 và báo kết quả; dừng lại trình A0 nếu lát nào làm đổi bất biến §4.

---

*ST-262 — design-only. Không dòng mã sản phẩm, không Cargo.toml, không opcode, không commit, không chạy gate đầy đủ. Mọi con số là số đo đã có trong `agent/BAO-CAO-SAN-SANG-THUC-TE-2026-11-20.md`; mọi lựa chọn là đề xuất `chờ A0 chốt`.*
