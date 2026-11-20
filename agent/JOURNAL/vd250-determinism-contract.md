<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-250 / ST-266 · Nháp **hợp đồng tất định theo event trace**

**Trạng thái**: **DRAFT — CHỜ A0 CHỐT**. research + draft only; không sửa mã sản phẩm, không commit, không đổi VERSION/opcode, không chạy gate.
**Căn cứ**: `agent/DECISIONS.md:2454-2473` (VD-250 §2.1/§2.2/§2.3), `agent/QUEUE.md:2063` (ST-266).
**Quy ước**: mọi nhận định "hiện trạng" có `file:line`; mọi đề xuất đều gắn **[CHỜ A0 CHỐT]**. Phần §2 là nội dung A0 **đã chốt**, phần còn lại là diễn giải/đề xuất.

---

## 1. Hiện trạng có trích dẫn

| Thành phần | Hiện trạng (file:line) | Hệ quả cho hợp đồng |
|---|---|---|
| Scheduler | Một luồng, tất định, run-to-completion, ghi rõ ở `crates/vis-interp/src/rt.rs:2-11`; `Scheduler { tasks, ready, tiep, sau, kenh }` `rt.rs:77-87`, `ready` là `VecDeque<usize>` FIFO `rt.rs:79`; `spawn` đẩy vào `ready` `rt.rs:112-116`; `chay_mot` rút task `Moi` rồi chạy tới hoàn tất `rt.rs:157-175`; `join_all` duyệt `tasks` theo thứ tự tạo `rt.rs:242-249`. | Tất định hiện tại là tất định **của thứ tự gọi hàm đồng bộ**, chưa phải tất định của một scheduler có suspend. |
| Hai engine dùng chung | VM reuse scheduler của interp: `crates/vis-vm/src/vm.rs:156`, `:186`, `:521-540`; `join_all` gọi ở `crates/vis-interp/src/interp.rs:221` và `crates/vis-vm/src/vm.rs:229`. | Parity là bất biến cứng; mọi quy tắc xếp hàng phải có một định nghĩa duy nhất cho cả hai. |
| Mạng thật | `accept` timeout = nonblocking + `std::thread::sleep(5ms)` + `Instant` `crates/vis-cli/src/net.rs:85-116`; `connect_timeout` `net.rs:119-135`; `read` dùng `set_read_timeout` `net.rs:185-206`; `write_all`/`flush` chặn `net.rs:208-213`; HTTP tự tính `Instant`/deadline `crates/vis-interp/src/net.rs:217-269`, `:354-398`. | Thứ tự sự kiện mạng ngoài đời **không** do runtime quyết ⇒ hai engine có thể quan sát hai ready-set khác nhau trong cùng một poll. Đây là lý do VD-250 §2.1 tách mạng thật khỏi trace. |
| Harness differential | So `stdout`, `exit`, `(mã, dòng, cột)`; **không so `steps`** — `crates/vis-vm/tests/vs_interp.rs:1-7`; ghi chú tương tự `crates/vis-vm/src/lib.rs:17-20`. | Byte-identical phải được định nghĩa trên **trace scheduling**, không trên bộ đếm nội bộ của engine. |
| Runner semantic | `steps_cua` đọc `steps=` từ stderr `crates/vis-cli/tests/sem.rs:107-117`; ca xanh/lỗi runtime đều bắt `steps > 0` `sem.rs:270-290`; CLI in `steps=` ở `crates/vis-cli/src/main.rs:642`, `:650`. | Nếu đổi nghĩa `steps` (xem `vd250-charging-spec.md`), phải phân loại lại runner này — **không** sửa hàng loạt kỳ vọng. |
| Tiền lệ reactor giả | `--fs-mock` là `FsHost` tất định đọc kịch bản JSON `crates/vis-cli/src/fsmock.rs:1-16`, dùng để test hermetic. | Có tiền lệ trong repo cho "host giả theo kịch bản"; mở rộng sang net/timer là đề xuất, chưa có. |

## 2. Hợp đồng A0 đã chốt (VD-250 §2.1, `agent/DECISIONS.md:2456-2462`)

1. **Tự sắp wake không bảo đảm** interp và VM nhận sự kiện mạng ngoài đời **cùng thứ tự**; tốc độ engine khác nhau ⇒ ready-set mỗi poll khác nhau.
2. **Hợp đồng**: cùng chương trình + cùng cấu hình + **cùng chuỗi sự kiện đầu vào** ⇒ **scheduling tái lập được**.
3. **Test trace**: dùng **reactor giả / event trace** ⇒ **byte-identical giữa hai engine**.
4. **Test mạng thật**: kiểm **đúng đắn / tiến triển / giới hạn tài nguyên / quan hệ thứ tự bắt buộc** — **không** bắt output giữa các kết nối độc lập giống từng byte khi ngôn ngữ không quy định thứ tự đó.
5. **Tie-break chỉ có nghĩa trong tập sự kiện đã quan sát**; đồng hồ treo tường (wall-clock) **không** được trở thành nguồn tất định.
6. **Không dùng "tất định tuyệt đối"** để bắt runtime chờ task chậm rồi mất concurrency.

## 3. Diễn giải chính xác để current-state không trượt thành promise sai

**[CHỜ A0 CHỐT] 3.1 — "Chuỗi sự kiện đầu vào" là gì.**
Đề xuất định nghĩa trace là một dãy hữu hạn, đã sắp thứ tự, gồm các bản ghi do **reactor giả** phát ra:
`(seq, kind, target, payload, tick)`, với `kind ∈ {IoReady, TimerFired, WakeExternal}`, `seq` tăng đơn điệu do reactor cấp, `tick` là **số lượt poll logic** (không phải `Instant`/ms). Deadline được mô hình hoá thành sự kiện `TimerFired` mang `seq`; thời gian treo tường không bao giờ nằm trong trace, dù timer thật dùng `Instant` ở `crates/vis-cli/src/net.rs:94`, `:105` và `crates/vis-interp/src/net.rs:225`, `:251`, `:357`, `:361`.

**[CHỜ A0 CHỐT] 3.2 — "Scheduling tái lập được" đo bằng gì.**
Đề xuất so sánh dãy quyết định của scheduler — ví dụ `(task_id, action, event_seq)` với `action ∈ {Poll, Suspend, Wake, Complete}` — **song song với** bộ `(stdout, exit, chẩn đoán)` đã có trong `crates/vis-vm/tests/vs_interp.rs:1-7`. Không so `steps`: `crates/vis-vm/src/lib.rs:17-20` ghi rõ khác đơn vị.

**[CHỜ A0 CHỐT] 3.3 — Quy tắc tie-break.**
Đề xuất khoá sắp xếp `(tick, seq, task_id, generation)`; `task_id` là số thứ tự tạo `rt.rs:95-110`, `generation` phân biệt timer bị huỷ rồi đặt lại. Khoá này **chỉ** áp cho các sự kiện **đã quan sát trong cùng poll**; không suy ra thứ tự của sự kiện chưa tới.

**[CHỜ A0 CHỐT] 3.4 — Hạn ngạch CPU trong trace.**
Đề xuất: lần nhường do hạn ngạch (`vd249-async-design-draft.md:63-67`, VD-250 `agent/DECISIONS.md:2447`, `:2504`) là một **sự kiện nội bộ có `seq` riêng**, được ghi vào trace để byte-identical; đơn vị và giá trị `Q` chưa chọn, không lấy từ `steps` hiện tại (`interp.rs:101,300,645` đếm nút/sự kiện ngữ nghĩa; `vm.rs:97,736-743` đếm lệnh).

**[CHỜ A0 CHỐT] 3.5 — "Quan hệ thứ tự bắt buộc" khi test mạng thật.**
Đề xuất liệt kê bởi **hợp đồng ngôn ngữ/API**, ví dụ: `listen` trước `accept`, `connect` trước `read`, final join sau mọi task đã tạo (`rt.rs:242-249`), deadline tổng của một lời gồm connect/redirect/đọc (VD-250 §1 `agent/DECISIONS.md:2448`, §5 `:2510-2516`). Ngoài tập đó, test mạng thật **không** assert thứ tự byte giữa hai kết nối.

**[CHỜ A0 CHỐT] 3.6 — Điểm cắm reactor giả.**
Đề xuất cắm ở **adapter host** (`trait NetHost`, `crates/vis-interp/src/builtin.rs:176-179`; impl thật `crates/vis-cli/src/net.rs:117-213`) + timer heap nội bộ; **không** thêm API ngôn ngữ, không đổi chữ ký `net.*` (VD-250 §5 `agent/DECISIONS.md:2508-2516`: giữ bề mặt `net.*`, cho phép sửa interface nội bộ).

## 4. Phân loại fixture — **[CHỜ A0 CHỐT]**

Nguyên tắc (VD-250 §2.2, `agent/DECISIONS.md:2467-2468`): fixture thuần tính toán không suspend **giữ hành vi hiện tại**; fixture có I/O/interleaving phải **phân loại**, **không cập nhật golden hàng loạt**. Mọi thay đổi kỳ vọng phải có ca đỏ-trước (`CONTRIBUTING.md:36`).

| Lớp | Fixture cụ thể (hiện trạng, `test/sem/async/…`) | Phân loại đề xuất | Hành động |
|---|---|---|---|
| **P — thuần tính toán, không suspend thực sự** | `await-co-ban.vis` (`a+b`, `.out` = `3`), `de-quy-async.vis` (`await` đệ quy giai thừa, `.out` = `24`), `await-trong-vong-lap.vis` (`.out` = `14`), `await-hai-lan-cache.vis` (`.out` = `chay/7/7`) | Giữ nguyên; đây là nhóm "giữ hành vi hiện tại". | Không đổi golden. Chỉ chạy lại để xác nhận không hồi quy. |
| **O — nhạy thứ tự do spawn/await (chưa có I/O)** | `hai-task-thu-tu.vis` + `.out` `1,2`; `print-trong-task-thu-tu.vis` + `.out` `f1,f2`; `spawn-nhieu-join-cuoi.vis` + `.out` `main,1,2,3`; `task-khong-await-van-chay.vis` + `.out` `main,chay 1,chay 2`; `grid-await-nghich-thu-tu.vis` + `.out` (dòng 2-6) `main,task 2,2,task 1,1`; `grid-chan-hai-task-kenh.vis` | Chia đôi: (O1) thứ tự **bắt buộc theo hợp đồng** — comment trong `grid-await-nghich-thu-tu.vis:2-3` khẳng định thứ tự do **thứ tự await** quyết định, không phải thứ tự spawn; phải giữ. (O2) thứ tự **ngoài hợp đồng**, chỉ là hệ quả run-to-completion hiện tại (`rt.rs:157-175`) — không được phong thành bảo đảm ngôn ngữ. | O1: viết ca red-before khoá quan hệ. O2: ghi nhãn "thứ tự không cam kết" trong tài liệu; **không** tự đổi `.out`. Nếu Lát 1 làm O2 đổi output thì trình A0 quyết riêng, không sửa golden hàng loạt. |
| **I — I/O / mạng thật / HTTP** | `crates/vis-cli/tests/net.rs`, `crates/vis-cli/tests/net_server.rs`, `crates/vis-cli/tests/http.rs`, `crates/vis-cli/tests/vistest_async.rs` (join cuối + `VS0505`/`VS0506`, `vistest_async.rs:3`, `:113`, `:144-152`) | Test đúng đắn/tiến triển/giới hạn/quan hệ thứ tự bắt buộc; **không** so byte giữa kết nối độc lập. | Giữ tiêu chí hiện tại; bổ sung assert "ready rỗng ≠ deadlock" (VD-250 §2.3 `agent/DECISIONS.md:2469-2473`). |
| **T — trace-based mới (cần tạo)** | Chưa có; đề xuất thêm fixture chạy với reactor giả, tái dùng ý tưởng kịch bản của `crates/vis-cli/src/fsmock.rs:1-16` | Một kịch bản trace ⇒ interp/VM byte-identical cả stdout lẫn dãy quyết định scheduling (§3.2). | Tạo mới **sau khi A0 chốt** §3; không đụng `test/sem/**` hiện hữu. |

**Quy trình phân loại (đề xuất, [CHỜ A0 CHỐT])**: (1) gắn nhãn P/O1/O2/I cho từng fixture; (2) với O1/I, viết ca đỏ-trước thể hiện **quan hệ** thay vì toàn bộ output; (3) nếu một golden phải đổi, phải kèm VD giải thích quan hệ ngôn ngữ thay đổi (`CONTRIBUTING.md:40-41`); (4) chạy parity trên cả hai engine theo harness hiện tại `crates/vis-vm/tests/vs_interp.rs:1-7`.

## 5. Câu hỏi gửi A0

| # | Câu hỏi | Khuyến nghị |
|---|---|---|
| D1 | Chấp nhận định nghĩa trace `(seq, kind, target, payload, tick)` với `tick` là số lượt poll logic (§3.1)? | Có; cấm wall-clock trong trace. |
| D2 | Byte-identical có được mở rộng thành "cùng dãy quyết định scheduling" không (§3.2), hay chỉ `(stdout, exit, chẩn đoán)` như `vs_interp.rs:1-7`? | Mở rộng; nếu không, hợp đồng không kiểm được "scheduling tái lập". |
| D3 | Khoá tie-break `(tick, seq, task_id, generation)` có được chốt không (§3.3)? | Có, nhưng chỉ trong tập đã quan sát. |
| D4 | Nhường do hạn ngạch có phải sự kiện trace không, và đơn vị `Q` lấy khi nào (§3.4)? | Có; `Q` đo sau, không lấy `steps`. |
| D5 | Danh sách "quan hệ thứ tự bắt buộc" cho test mạng thật gồm những gì (§3.5)? | Chỉ các quan hệ suy từ hợp đồng ngôn ngữ/VD-250. |
| D6 | Reactor giả cắm ở adapter host, không thêm API ngôn ngữ — chốt chứ (§3.6)? | Chốt. |
| D7 | Phân loại O1/O2 ở §4: fixture nào được phong "thứ tự bắt buộc"? | Chỉ `grid-await-nghich-thu-tu` (có comment hợp đồng) và final join; phần còn lại để ngỏ. |

## 6. Giới hạn phạm vi

Tệp này chỉ là đặc tả. Không sửa `rt.rs`, `net.rs`, harness, fixture, `DECISIONS.md`, `QUEUE.md`; không ấn VERSION; không mở Lát 1. Việc cập nhật `test/sem/**` chỉ được thực hiện sau khi A0 trả lời D1–D7.
