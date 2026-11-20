<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Báo cáo mức sẵn sàng sử dụng thực tế của ViScript — ưu tiên async/await (VD-248)

> **Căn cứ**: VD-248/A0 (tạm dừng tag/release; giữ `dbae70a` làm checkpoint kỹ thuật).
> **Phạm vi**: đánh giá async + luồng dùng thực tế, **không** audit lại toàn repository. Mã nguồn
> đọc ở HEAD `4af202e` (chỉ khác `dbae70a` ở tài liệu); baseline sản phẩm vẫn `dbae70a`.
> **Không** tạo/push tag, không công bố release, **không** mở VS3, **không** tự chọn GC/worker
> model/API mới. Mọi nhận định về mã kèm `tệp:dòng`; mọi số liệu là phép đo thật trên máy này.

## 0. Trả lời ngắn cho A0

| Câu hỏi | Trả lời |
|---|---|
| Đủ điều kiện gắn tag snapshot 0.x có giới hạn rõ? | **Có** — VD-247 đã ratify phạm vi đó, nhưng VD-248 đã **tạm dừng** quyền thực hiện. |
| Đủ tốt để dùng lâu dài / nền tảng thực dụng hoàn thiện? | **Chưa.** Async hiện là executor actor một luồng, run-to-completion; **mọi I/O chặn toàn runtime**; thiếu timeout, cancellation, fairness, async I/O, GC. |
| Phát hiện mới trong đợt đánh giá này | **Sai lệch parity giữa interp và VM về ngưỡng giới hạn thực thi** (chi tiết §3.4) — không nằm trong 525 ca TAP. |

Kết luận: checkpoint `dbae70a` vững trong phạm vi tính đúng đắn đã đo, nhưng **không** nên lấy
"test xanh" thay cho "đủ tốt để dùng". Điểm nghẽn thực dụng nằm ở **cơ chế async/I/O**, không phải
ở độ dài test hay tốc độ gate (mức tăng ~3,4× của ST-257/258 chỉ là tăng tốc quy trình kiểm thử).

---

## 1. Kiến trúc hiện tại

### 1.1 Bề mặt ngôn ngữ

| Có | Bằng chứng | Không có |
|---|---|---|
| `async fn` **chỉ hàm cấp tệp** | parse `crates/vis-syntax/src/parse/mod.rs:440`; type check `crates/vis-types/src/tyck/mod.rs:880-884`. Method/`cmd` ⇒ `VS0201` (`crates/vis-cli/tests/async_vis.rs:47`, nợ #13) | `async` trên method, `cmd` |
| `spawn tên_hàm(args)` — soft keyword, chỉ nhận **tên** hàm async cấp tệp, positional | `crates/vis-syntax/src/parse/expr.rs:496-505`; `crates/vis-types/src/tyck/expr.rs:339-377` | spawn biểu thức/hàm ẩn danh |
| `await t` — chạy task tới hoàn tất, cache kết quả | `crates/vis-syntax/src/parse/expr.rs:526-531`; `crates/vis-types/src/tyck/expr.rs:313`; `crates/vis-interp/src/rt.rs:177-230` | suspend giữa thân hàm |
| `chan[T](n)`, `n ≥ 1`; `send`/`recv`/`close` | `crates/vis-interp/src/rt.rs:128-138` (`chan(0)` ⇒ `VS0505`) | rendezvous, `select`, channel timeout |
| — | Registry stdlib `crates/vis-types/src/stdlib.rs:189-507` | **`std.task`, `sleep`, `yield`, `timeout`, `select`** — không tồn tại. Builtin `$tac_vu/$spawn/$await/$kenh` là nội bộ, tên có `$`, người dùng không gọi được (`crates/vis-interp/src/builtin.rs:760-816`) |

`docs/ViScript-thiet-ke.md:1611` nêu rõ mục VS2 mô tả **code hiện tại**, còn phần thiết kế phía trên
(`scope`, `timeout`, `select`, coroutine không ngăn xếp tại `:747-771, 947`) là **đích VS3, chưa có**.

### 1.2 Bộ máy thực thi — một luồng, tất định, run-to-completion

- Toàn bộ runtime là **249 dòng** trong `crates/vis-interp/src/rt.rs`. Header `:2, 4-8`:
  *"bộ lập lịch **một luồng, tất định**"*, *"`await` chạy tác vụ **tới hoàn tất** (run-to-completion,
  **không** nhường giữa thân) rồi trả cache; cuối chương trình **join** mọi tác vụ theo **thứ tự tạo**."*
- `Scheduler` (`rt.rs:74-87`): `tasks: Vec<Rc<Task>>` (mọi task theo thứ tự tạo), `ready: VecDeque<usize>`
  (FIFO task vừa spawn), `kenh: Vec<Rc<RefCell<Chan>>>`. **Không có `Future`/`Poll`/`Waker`/coroutine
  frame/state machine** nào trong repo.
- `await` là **lời gọi đồng bộ**: `rt.rs:218-219` đặt trạng thái `DangChay` rồi `host.goi_than(...)`
  — thân task chạy xong trước khi `await` trả về. Nó **không** trả quyền cho scheduler.
- Cơ chế duy nhất giống "nhường quyền": kênh đầy/rỗng gọi `chay_mot()` (`rt.rs:157-175`, gọi từ
  `builtin.rs:1265, 1284`) để chạy **một** task `ready` khác tới hoàn tất; hết ready ⇒ trap `VS0505`
  thay vì treo.
- Cuối chương trình `join_all` (`rt.rs:241-249`) chạy **cả task không được await** theo thứ tự tạo
  (fixture `test/sem/async/task-khong-await-van-chay.vis`).
- CLI chạy chương trình trên **một luồng OS riêng có stack 256 MB**
  (`crates/vis-cli/src/main.rs:501-506`) — đây là cách lách stack, **không phải song song**.
- Không dependency async/threading nào: đã kiểm tra 11 `Cargo.toml` — không `tokio`, `async-std`,
  `rayon`, `futures`, `mio`, `crossbeam`, `smol`; không `unsafe`/atomic trong runtime
  (`agent/vd212-thiet-ke-ghi-chu.md:16, 24`, `JOURNAL/st155-vs3-dieu-tra.md:24`).

### 1.3 I/O — không có thao tác nào tích hợp với scheduler

| Thao tác | Bằng chứng | Hành vi |
|---|---|---|
| TCP `connect` | `crates/vis-cli/src/net.rs:120-135` `connect_timeout` | **Chặn toàn runtime** (có hạn) |
| TCP `accept` | `net.rs:85-117`: timeout 0 ⇒ blocking; timeout > 0 ⇒ `set_nonblocking(true)` **+ vòng lặp `thread::sleep(5ms)`** | **Chặn toàn runtime**; nonblocking ở đây chỉ là cơ chế hạn giờ + busy-poll, **không** yield cho scheduler |
| TCP `read` / `write` / `close` | `net.rs:185-213`, `:215-229` | Chặn (có read timeout); close là instant |
| HTTP GET/HEAD | `crates/vis-interp/src/net.rs:217-269, 354-398`, gọi `net.read` blocking | Chặn. **Đính chính (ST-263)**: deadline được tính **một lần cho mỗi response** (`net.rs:225,357`), mỗi lần đọc chỉ dùng thời gian còn lại ⇒ slow-drip **trong một response đã bị chặn**; khoảng trống là **chuỗi redirect**, mỗi hop dùng deadline mới (`http_get:313-344`) ⇒ ≤ ~6× `timeout`. Nghĩa `timeout` giữa hai lần đọc đã được **VD-183 Q2(1)** chốt — xem `JOURNAL/vd249-lifecycle-measure.md` §1 |
| File `fs.*` | `crates/vis-interp/src/stdlib.rs:479-516` qua `FsHost`/`std::fs` | Chặn |
| SQLite | `crates/vis-cli/src/db/sqlite/mod.rs` (rusqlite bundled, `Cargo.toml:42-47`) | Chặn |
| Timer / sleep | **Không có** — chỉ `time.now_ms`/`monotonic_ms` (`stdlib.rs:437-448`) | N/A |
| Kênh | `builtin.rs:1250-1272` | Nhường sang task `ready` khác (vẫn run-to-completion cùng luồng) |

Không có epoll/kqueue/selector/`mio`. Mạng chỉ loopback (`net.rs:57-75`), nợ #22: *"Một luồng:
server phục vụ tuần tự một client, không `select`/epoll"* (`agent/SO-NO.md:108`). Hệ quả: **một task
chờ I/O làm mọi task khác đứng theo** — §3.2 đo điều này trực tiếp.

### 1.4 Interp và VM — cùng một mô hình

VM dùng **nguyên runtime của interp**: `crates/vis-vm/src/vm.rs:156` *"cùng runtime với interp"*,
`:186` tạo `vis_interp::rt::Scheduler::new()`, `:521-540` implement `Host`. Khác biệt duy nhất là
cách gọi thân hàm (sym của `HFn` vs chỉ số hàm bytecode). Cả hai đều hạ `await/spawn/chan` về cùng
builtin nội bộ; parity được khoá bởi `crates/vis-cli/tests/async_10k.rs:78-79`.

---

## 2. Vòng đời — cái có và cái thiếu

| Cơ chế | Hiện trạng | Bằng chứng / nợ |
|---|---|---|
| **Fairness** | FIFO thuần (`push_back`/`pop_front`), join theo thứ tự tạo. **Không preemption, không time-slice, không chống đói.** Task thân dài chặn mọi task khác tới khi xong hoặc dính giới hạn bước | `rt.rs:79, 114, 159, 243-247`; `rt.rs:4-5` |
| **Timeout** | **Không có primitive async.** Không `timeout(ms, fut)`, không channel timeout. `vis test --timeout` là watchdog **giết tiến trình con**, không liên quan task | nợ #17 (`SO-NO.md:103`); `crates/vis-cli/src/vistest.rs:218-247`; đích VS3 `docs/...:761, 1615` |
| **Cancellation** | **Không có API** (grep `cancel/detach/abort_task` ⇒ không có gì liên quan task). Task bị "bỏ rơi" vẫn **được join cuối** vì `Scheduler.tasks` giữ `Rc`. Không có scope huỷ khi ra khỏi phạm vi | `rt.rs:78, 107, 242-249`; nợ #16 (`SO-NO.md:102`); đích `scope` tại `docs/...:758-760` |
| **Lỗi task** | Lỗi là giá trị, bọc `VS0506` giữ mã/gốc, cache lại; await lần 2 trả cùng `Err`. Self-await và `await` lồng > 256 ⇒ `VS0501`. Join dùng `?` nên **task lỗi đầu tiên làm bỏ join các task sau**. Rust panic trong host **không** được `catch_unwind` ở ranh giới task | `rt.rs:180-230, 232-247`; `vis-diag/src/lib.rs:493-496`; `vistest_async.rs:107` |
| **Đóng tài nguyên** | **Không có hook theo task.** Socket/file chỉ đóng khi gọi `close` tường minh; task kết thúc không tự đóng tài nguyên nó mở. Kênh bị giữ cố ý trong `Scheduler.kenh` ⇒ có thể tạo chu trình `Rc`; runtime **không GC** | `net.rs:215-229`; `rt.rs:84-86, 147`; `rt.rs:10` |
| **Giới hạn task / backpressure** | **Không có cap số task**, không admission control, không queue bound. 10k task chạy được (tuyến tính). Có: capacity kênh, `MAX_TASK_NEST=256`, call depth/frames 2000, `MAX_STEPS=10^7` (xem §3.4), `NET_READ_CAP` 8 MiB. "Backpressure" chỉ là kênh đầy ép drain `ready` rồi trap | `rt.rs:24, 54-72, 95-109`; `builtin.rs:1250-1272`; `vm.rs:95-97` |

---

## 3. Phép đo đại diện

### 3.1 Phương pháp

- Máy: Linux x86_64, **12 lõi**, 14 GiB RAM; toolchain `rustc/cargo 1.99.1`; baseline `dbae70a`.
- Đo bằng `/usr/bin/time -f 'wall=%e rss=%MKB'` trên `vis run <f>` (interp) và
  `vis run --vm --no-cache <f>` (VM). Thời gian tính cả khởi động (~0,05–0,2 s) — so sánh tương đối.
- Server TCP loopback bằng Python, **trễ cố định 200 ms** rồi gửi 1 byte; chấp nhận song song
  (thread mỗi connection) nên server **không** phải nút thắt. Cổng 37195 (phiên bản đo).
- 4 nhóm: scheduler thuần (1k/10k/50k task), nhiều connection chờ I/O (1/5/20), trộn CPU+I/O
  (có timestamp `time.now_ms`), tạo/hủy lặp (1k/10k/50k spawn+await). Chương trình gốc được đính
  kèm ở Phụ lục để có thể lặp lại.

### 3.2 Kết quả

**Nhóm 1 — task thuần scheduler** (mỗi task `n*2`, join cuối, in `xong`):

| Số task | interp wall / RSS | VM wall / RSS | Đúng đắn |
|---|---|---|---|
| 1 000 | 0,21 s / 32 MB (lần đầu, cold) | 0,07 s / 24 MB | `xong` |
| 10 000 | 0,11 s / 24 MB | 0,10 s / 24 MB | `xong` |
| 50 000 | **0,28 s / 25 MB** | **0,28 s / 29 MB** | `xong` |

→ Tạo + join 50k task tuyến tính, ~0,28 s, bộ nhớ tăng nhẹ (24→29 MB). Không có bằng chứng
starvation ở quy mô này; nhưng đây là task **tính toán ngắn**, không phải tải hỗn hợp.

**Nhóm 2 — nhiều connection chờ I/O** (mỗi task `connect` + `read(16, 2000)`, server trễ 200 ms;
task `MARK` được **spawn sau cùng**):

| Số task I/O | interp wall | VM wall | Kết quả đọc | Thứ tự output |
|---|---|---|---|---|
| 1 | 0,27 s | 0,28 s | 1/1 ok | `io 0 1` → `MARK` |
| 5 | 1,09 s | 1,09 s | 5/5 ok | `io 0..4` → `MARK` |
| 20 | **4,11 s** | **4,10 s** | 20/20 ok | `io 0..19` → `MARK` |

→ **Thời gian tăng tuyến tính theo N × 200 ms** (1→0,27; 5→1,09; 20→4,11). Nếu I/O không chặn,
cả ba phải ~0,27 s. `MARK` luôn in **cuối cùng** dù spawn sau — join theo thứ tự tạo, không có
xen kẽ. **Kết luận đo được: task chờ mạng làm mọi task khác đứng theo; hai engine giống hệt.**

**Nhóm 3 — trộn CPU và I/O** (task CPU 500k vòng lặp spawn **trước**, task I/O spawn sau):

```
interp: CPU_DONE 1795151072097
        IO_START 1795151072097     ← cùng milicgiây với CPU_DONE
        IO_DONE  1795151072298 ok  ← +201 ms (đúng độ trễ server)
VM    : CPU_DONE …974 / IO_START …974 / IO_DONE …175 ok   (hành vi y hệt)
wall: interp 1,01 s / VM 0,86 s
```

→ Task I/O **không bắt đầu** cho tới khi task CPU kết thúc: `CPU_DONE` và `IO_START` cùng mốc ms.
Đây là starvation trực tiếp do run-to-completion — một task CPU dài làm task khác đói hoàn toàn.

**Nhóm 4 — tạo/hủy task lặp** (`spawn` + `await` ngay trong vòng lặp):

| Số lần | interp wall / RSS | VM wall / RSS | Đúng đắn |
|---|---|---|---|
| 1 000 | 0,08 s / 24 MB | 0,08 s / 24 MB | `xong` |
| 10 000 | 0,11 s / 24 MB | 0,11 s / 24 MB | `xong` |
| 50 000 | **0,28 s / 25 MB** | **0,27 s / 28 MB** | `xong` |

→ Ở quy mô này không thấy tăng bộ nhớ đáng kể. **Nhưng** đây là bằng chứng sơ bộ, không phải bảo
đảm: `Scheduler.tasks` vẫn giữ mọi `Task` tới hết chương trình (`rt.rs:78`) và không có dọn theo
task (§2) — cần đo dài hơn/đa luồng hơn mới kết luận về leak.

### 3.3 Độ trễ

- I/O: `IO_DONE - IO_START = 201 ms`, khớp độ trễ server 200 ms (độ trễ đo được là độ trễ mạng,
  **không** có chi phí scheduling đáng kể vì không có scheduling giữa thân).
- CPU+I/O: độ trễ của task I/O = **thời gian chạy toàn bộ task CPU đứng trước nó** cộng 200 ms
  (xem §3.4 để biết giới hạn của task CPU).

### 3.4 Phát hiện mới — sai lệch parity về giới hạn thực thi (interp ≠ VM)

Chương trình CPU 2.000.000 vòng lặp (cùng cú pháp, cùng baseline):

| Engine | Kết quả |
|---|---|
| interp | **exit 0**, `xong` sau **3,26 s** |
| VM | **exit 101**, stderr `VS0506: tác vụ 'cpu' lỗi: VS0501: chạy quá lâu (>10^7 lệnh)` sau 1,40 s |

**Nguyên nhân (đã đối chiếu mã):**
- interp `crates/vis-interp/src/interp.rs:541`: `guard > 10_000_000` — đếm **số lần lặp của một
  vòng `while`**.
- VM `crates/vis-vm/src/vm.rs:97, 737`: `MAX_STEPS = 10_000_000`, `vm.steps += 1` cho **mỗi lệnh**
  trên toàn chương trình.

Hai con số cùng `10^7` nhưng **đếm hai đơn vị khác nhau** ⇒ cùng một chương trình hợp lệ có thể
xanh ở interp, đỏ ở VM. Điều này nằm ngoài 525 ca TAP (vốn dùng vòng lặp ngắn) và ngoài phạm vi đã
ratify của VD-247. Phân loại: **lỗi tính đúng đắn / thiếu nhất quán**, không phải "chỉ thiếu bằng
chứng" và không phải hiệu năng. Không tự sửa trong báo cáo này — đề xuất xử lý ở §5.

---

## 4. Phân loại từng khoảng trống

| Khoảng trống | Phân loại | Bằng chứng | Tác động thực dụng |
|---|---|---|---|
| Ngưỡng `10^7` đếm khác đơn vị giữa interp/VM (§3.4) | **Lỗi đúng đắn / nhất quán (mới)** | `interp.rs:541` vs `vm.rs:97,737`; đo §3.4 | Chương trình CPU dài có thể chạy một engine, trap engine kia — phá vỡ cam kết parity |
| Mọi I/O chặn toàn runtime (§1.3, §3.2) | **Thiếu cơ chế** | `net.rs:85-135,185-213`; `SO-NO.md:108` (#22) | Không xử lý được nhiều kết nối chờ cùng lúc; async không mang lại lợi ích I/O |
| Không timer/sleep/timeout async | **Thiếu cơ chế** | `stdlib.rs:189-507`; nợ #17; `docs/...:1615` (VS3) | Không diễn đạt được deadline, retry, backoff trong ngôn ngữ |
| Không cancellation / scope huỷ task | **Thiếu cơ chế** | `rt.rs:78,242-249`; nợ #16; `docs/...:758-760` (đích) | Không dừng task đang chờ; "abandon" vẫn chạy tới cuối |
| Không fairness/preemption | **Thiếu cơ chế** (có thể chấp nhận nếu I/O không chặn) | `rt.rs:4-5,79,159`; §3.3 | Task CPU dài đói task khác — đo được trực tiếp |
| Không GC, chu trình `Rc` qua kênh | **Thiếu cơ chế** (VS3) | `rt.rs:10,84-86`; `JOURNAL/st155-vs3-dieu-tra.md` | Nguy cơ giữ tài nguyên lâu; chưa có bằng chứng leak ở 50k |
| Không dọn tài nguyên theo task | **Thiếu cơ chế** | `net.rs:215-229`; §2 | Socket/file có thể mở tới hết chương trình nếu quên `close` |
| HTTP deadline tổng cho toàn lời gọi (gồm redirect) | **Thiếu cơ chế / cần chốt hợp đồng** | nợ #148; `VD-183 Q2(1)` chốt `timeout` = giữa hai lần đọc; `JOURNAL/vd249-lifecycle-measure.md` §1 | Slow-drip **một response** đã bị chặn; chuỗi redirect có thể ~6× `timeout`. Cần A0 chọn A/B/C, **không tự đổi mã** |
| Chưa có test I/O đồng thời / fairness / memory dài | **Chỉ thiếu bằng chứng** | §5.3 của khảo sát; các test net dùng thread ở **harness**, không phải 2 task ViScript | Chưa biết hành vi thật dưới tải hỗn hợp ngoài các ca đã đo ở §3 |
| Test 10k chỉ đo scale + parity | **Chỉ thiếu bằng chứng** | `async_10k.rs:57-88` (comment `:9` ghi ~0,2 s) | Không đo latency/fairness/I/O — dễ bị hiểu nhầm là "async tốt" |
| Loopback only; không TLS/HTTP2/keep-alive; method `async` ⇒ `VS0201` | **Giới hạn thiết kế đã biết** | #22/#83/#84/#13; `docs/...:1618` | Giới hạn use-case, không phải lỗi |

---

## 5. Đề xuất thứ tự phát triển (không tự chọn kiến trúc)

> A0 chỉ thị: **không tự chọn GC, worker model, hoặc mở API mới.** Vì vậy dưới đây là **thứ tự
> và câu hỏi cần A0 quyết**, không phải phương án kiến trúc được tôi chốt.

**Nhóm 0 — làm rõ mục tiêu sử dụng (việc đầu tiên, trước khi chọn bước tiếp)**
1. Xác định 2–3 luồng dùng thực tế ưu tiên (ví dụ: script điều khiển tuần tự; server loopback nhiều
   kết nối; pipeline CPU + I/O). Không có danh sách này thì không trả lời được "async cần tối ưu
   thêm một chút hay thiếu cơ chế nền tảng".
2. A0 cần quyết: có chấp nhận **async một luồng + I/O chặn** như ngữ nghĩa được công bố (kèm giới
   hạn rõ) hay đầu tư cơ chế I/O không chặn/timer — đây là **quyết định phạm vi sản phẩm**, không
   phải quyết định kỹ thuật nhỏ.

**Nhóm 1 — sửa lỗi đúng đắn, không cần đổi kiến trúc (nên làm sớm, rẻ)**
3. Thống nhất ngữ nghĩa ngưỡng giới hạn giữa interp và VM (§3.4): cùng một chương trình phải cùng
   kết quả; kèm test parity cho vòng lặp dài. **Không** đổi kiến trúc, chỉ đổi cách đếm/ngưỡng —
   nhưng cần A0 chốt ngữ nghĩa mong muốn (đếm lệnh hay đếm vòng lặp).
4. Xử lý #148 (HTTP timeout tổng) nếu luồng dùng có HTTP.

**Nhóm 2 — bổ sung bằng chứng để quyết có cần cơ chế mới không (rẻ, không đổi mã sản phẩm)**
5. Test I/O đồng thời của 2+ task, test fairness (task CPU dài + task ngắn), đo RSS tải dài
   (≥10⁵ task, lặp nhiều lần) — hiện mới có số liệu sơ bộ ở §3.
6. Đo latency distribution, không chỉ thời gian trung bình, trên cả hai engine.

**Nhóm 3 — cơ chế lớn (chỉ mở khi A0 quyết; hiện thuộc VS3)**
7. Timer/timeout, cancellation/scope huỷ task, async I/O (selector/event loop), và — nếu mục tiêu
   là song song thật — worker model + mô hình bộ nhớ + GC. **Tôi không chọn phương án nào.**
   Ghi chú để A0 cân nhắc: `agent/vd212-thiet-ke-ghi-chu.md:66` ước hướng worker pool + deep-copy
   + `Arc/Mutex` tốn ~3,5–5 tuần-người và làm mất tất định; `JOURNAL/st155-vs3-dieu-tra.md:190`
   ước 4–6 task · 800–1 800 dòng · 5 tệp lõi. Đây là ước cũ, chưa được tôi kiểm chứng lại.

**Có thể để sau:** GC, song song đa lõi, mở API std.task/select/timeout mới — tất cả đều cần A0
chốt trước khi triển khai.

---

## 6. Kết luận

ViScript hiện là **một ngôn ngữ script chạy được với bộ công cụ khá rộng và đã được gia cố tính
đúng đắn** (525 ca semantics hai engine, runtime trap, parity spawn/await, 10k task). Đó là một
checkpoint đáng mừng và VD-247 đã ratify đúng phạm vi đó.

Nhưng theo số liệu §3, async **chưa** mang lại lợi ích thực dụng cốt lõi: I/O chặn toàn runtime
(4,11 s cho 20 kết nối trễ 200 ms), task CPU làm task khác đói (cùng mốc ms), không timeout/cancel/
fairness, và có sai lệch parity ở ngưỡng thực thi. Vì vậy **chưa** có căn cứ gọi đây là nền tảng
thực dụng hoàn thiện, và cũng chưa có đủ số liệu để khẳng định "chỉ cần tối ưu thêm một chút".

**Khuyến nghị gửi A0**: giữ `dbae70a` làm checkpoint, giữ nguyên tạm dừng tag/release, quyết trước
2 câu ở Nhóm 0, cho sửa lỗi ở Nhóm 1 và đo bổ sung ở Nhóm 2. Chỉ sau đó mới quyết có mở VS3 hay
không. Không revert kết quả đã đạt, không audit vô hạn.

---

## Phụ lục — chương trình đo (để lặp lại)

```vis
# SPDX-License-Identifier: MIT OR Apache-2.0
# Nhóm 1 (thay 1000 bằng 10000/50000):
async fn viec(n: int) -> int:
    n * 2
var i = 0
while i < 1000:
    let t = spawn viec(i)
    i = i + 1
print("xong")
```

```vis
# SPDX-License-Identifier: MIT OR Apache-2.0
# Nhóm 4 (thay 1000 bằng 10000/50000):
async fn noop(n: int) -> int:
    n
var i = 0
while i < 1000:
    let t = spawn noop(i)
    let r = await t
    i = i + 1
print("xong")
```

```vis
# SPDX-License-Identifier: MIT OR Apache-2.0
# Nhóm 2 — thay PORT bằng cổng server trễ 200 ms; thay 5 bằng 1/20:
use std.net
async fn doc(i: int) -> int:
    match net.connect("127.0.0.1:PORT"):
        Ok(t):
            let r = match net.read(t, 16, 2000):
                Ok(s): 1
                Err(e): 0
            let _ = net.close(t)
            print("io {i} {r}")
            r
        Err(e):
            print("io {i} err")
            0
var i = 0
while i < 5:
    let t = spawn doc(i)
    i = i + 1
async fn marker() -> ():
    print("MARK")
let m = spawn marker()
```

```vis
# SPDX-License-Identifier: MIT OR Apache-2.0
# Nhóm 3 — trộn CPU+I/O có timestamp (cần use std.time):
use std.net
use std.time
async fn cpu() -> ():
    var i = 0
    while i < 500000:
        i = i + 1
    let t = time.now_ms()
    print("CPU_DONE {t}")
async fn io() -> ():
    let t0 = time.now_ms()
    print("IO_START {t0}")
    match net.connect("127.0.0.1:PORT"):
        Ok(t):
            let r = match net.read(t, 16, 2000):
                Ok(s): "ok"
                Err(e): "err"
            let _ = net.close(t)
            let t1 = time.now_ms()
            print("IO_DONE {t1} {r}")
        Err(e): print("IO_CONN_ERR")
let c = spawn cpu()
let d = spawn io()
```

Server trễ (Python, chạy ngoài repo):

```python
import socket, threading, time
s = socket.socket(); s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
s.bind(('127.0.0.1', 0)); s.listen(128); print(s.getsockname()[1], flush=True)
def handle(c):
    with c:
        time.sleep(0.2)
        try: c.sendall(b'x')
        except OSError: pass
while True:
    c, _ = s.accept(); threading.Thread(target=handle, args=(c,), daemon=True).start()
```
