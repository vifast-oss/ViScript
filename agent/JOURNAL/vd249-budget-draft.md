<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-249 / ST-261 · Draft **hợp đồng tính phí ngân sách thực thi** (để A0 chốt)
**Trạng thái**: **DRAFT — CHỜ A0 DUYỆT**. research/draft, không commit, không đổi VERSION, không thêm opcode, không đổi mã sản phẩm.
**Căn cứ**: `agent/DECISIONS.md:2368-2385` (VD-249 §3), đặc biệt các ràng buộc (a)–(e) ở `:2372-2377` và checkpoint đầu tiên ở `:2378-2381`.
**Phạm vi**: đây là **tài liệu quyết định**, không phải bản thiết kế triển khai chi tiết. Mọi con số trọng số và ngưỡng dưới đây là **đề xuất**, cần A0 chốt. Phân lập exit 101 xem `agent/JOURNAL/vd249-exit101-triage.md`.

**Bối cảnh hiện trạng (có file:line)** — trích dẫn làm căn cứ cho các lựa chọn ở sáu mục dưới:

- Interp: `steps` (`crates/vis-interp/src/interp.rs:101`, khởi tạo `:168`) tăng ở `exec_stmt` (`:300`) và `eval` (`:645`) — đếm **nút ngữ nghĩa**, nhưng **không có ngưỡng** trên field này; hàng rào thật nằm ở vòng lặp: `while` `guard > 10_000_000` (`:541`), `loop` `for _ in 0..10_000_000usize` (`:559`), `for` không có guard riêng (`:519-533`).
- VM: `MAX_STEPS = 10_000_000` (`crates/vis-vm/src/vm.rs:97`) và `vm.steps += 1` **trước mọi lệnh** (`:736`), kiểm tra ở `:737-743` ⇒ `VS0501`.
- Hệ quả đã đo: 2M vòng lặp `while` chạy được trên interp nhưng trap trên VM (`vd249-exit101-triage.md`).
- Giới hạn **khác** đã tồn tại và phải giữ riêng: độ sâu gọi interp `MAX_CALL_DEPTH = 2_000` (`interp.rs:1401`, kiểm tra `interp/goi.rs:128` và `:208`); VM `MAX_FRAMES = 2_000` (`vm.rs:95`, kiểm tra `:557`, `:818`); lồng `await` `MAX_TASK_NEST = 256` (`crates/vis-interp/src/rt.rs:24`, kiểm tra `:209-213`). Bytecode `VERSION = 2` (`crates/vis-bytecode/src/format.rs:44`), khoá cache đã đưa `VERSION` vào (`crates/vis-cli/src/cache.rs:43-44`, `:55`, `:61`).

---

## 1. Sự kiện bị tính phí và trọng số (đề xuất)

Nguyên tắc: tính phí theo **sự kiện ngữ nghĩa của chương trình**, không theo opcode VM và không theo số lần lặp riêng lẻ của một engine. Một sự kiện phải **có mặt trong cả hai engine** và có cùng tên/trọng số.

| Mã sự kiện | Sự kiện ngữ nghĩa | Trọng số | Lý do |
|---|---|---|---|
| `E_STMT` | Một **câu lệnh** HIR bắt đầu thực thi | 1 | Đơn vị ngữ nghĩa nhỏ nhất có nghĩa với người viết; interp đã có điểm chèn tự nhiên ở `exec_stmt` (`interp.rs:299-300`). |
| `E_EXPR` | Một **biểu thức** HIR được đánh giá | 1 | Chặn trường hợp biểu thức lớn/đệ quy điều kiện tiêu tốn nhiều công mà vẫn nằm trong một câu lệnh; interp có điểm chèn ở `eval` (`interp.rs:644-645`). **Câu hỏi mở A0-1**: có gộp vào `E_STMT` (trọng số 0) hay không. |
| `E_LOOP` | Một **vòng thân vòng lặp** (`while` / `loop` / `for`) bắt đầu một lần lặp | 1 | Thay thế trực tiếp hai hàng rào cũ (`interp.rs:541`, `:559`) bằng một điểm tính phí chung; bảo đảm "không âm thầm bỏ hàng rào chống chạy vô hạn" (VD-249 §3(e)). |
| `E_CALL` | Một **lời gọi hàm người dùng** mở khung mới | 4 | Gọi hàm kéo theo dựng khung/biến local; trọng số lớn hơn câu lệnh thường phản ánh công đó mà vẫn không gộp với cap độ sâu (xem §5). Điểm chèn: `interp/goi.rs:128`, `:208`; VM `Instr::Call` (`vm.rs:809`), `CallValue` (`:1157`). |
| `E_BUILTIN` | Một **builtin/phương thức** được gọi (kể cả `print`, `std.fs.*`, `std.net.*`, `std.sqlite.*`) | 2 | Builtin là ranh giới ra ngoài runtime; tính phí ở đây để mọi tác dụng phụ nằm **sau** lần kiểm tra ngân sách (xem §4). `builtin::call` là mã dùng chung: interp gọi ở `builtin.rs`, VM gọi ở `vm.rs:851` (`use vis_interp::builtin` ở `vm.rs:13`) — một điểm chèn phục vụ cả hai. |
| `E_SPAWN` | `spawn` một tác vụ | 4 | Tạo sổ điều phối/kênh; điểm chèn `crates/vis-interp/src/rt.rs:112` (`Sched::spawn`), gọi từ `interp/builtin.rs:804-810`; bytecode hạ `$spawn` thành `CallBuiltin` (`crates/vis-bytecode/src/codegen.rs:1446-1472`) ⇒ VM gặp ở `vm.rs:839`. |
| `E_AWAIT` | `await` / join một tác vụ (kể cả khi kết quả đã cache) | 2 | Đánh thức/chuyển điều khiển có công riêng; tính phí **mỗi lần** để chương trình không tránh được phí bằng cách await lại kết quả cache. Điểm chèn `rt.rs:178` (`await_task`), `rt.rs:242-247` (`join_all`); bytecode hạ `$await` (`codegen.rs:1439-1444`). **Câu hỏi mở A0-5**. |

**Không tính phí / tách riêng** (theo VD-249 §3(d), `agent/DECISIONS.md:2375-2376`): bộ nhớ, **độ sâu gọi** (`MAX_CALL_DEPTH`/`MAX_FRAMES`), **thời gian chờ I/O** (VD-249 §5, `agent/DECISIONS.md:2403-2408`), **lồng await** (`MAX_TASK_NEST`). Không gộp các thứ này vào một số `steps` duy nhất. Các cap cũ giữ nguyên giá trị và vẫn báo lỗi riêng.

**Ngưỡng ban đầu** (`BUDGET`): **chưa chốt**. Đề xuất chọn bằng đo đạc trên workload chuẩn (VD-249 §6, `agent/DECISIONS.md:2410-2417`) sao cho ca 2M vòng (đang thắng trên interp) nằm thoải mái dưới ngưỡng — **không** lấy "10^7 lệnh" và chỉ đổi đơn vị. **Câu hỏi mở A0-2**.

---

## 2. Ánh xạ vào interpreter và bytecode

### Interpreter
- Tái dùng `Interp.steps` (`interp.rs:101`) nhưng **đổi nghĩa**: từ "đếm nút" thành "số đơn vị ngân sách đã tiêu", và đổi kiểu sang `u64` nếu cần. Giữ các điểm `+=` hiện có làm nơi gọi hàm `charge(E_…, w)`: `exec_stmt` (`:300`) → `E_STMT`; `eval` (`:645`) → `E_EXPR`.
- Thêm `E_LOOP` tại các đầu vòng: `while` (`:535-547`, thay/giữ `guard` ở `:541`), `loop` (`:557-566`, thay `for _ in 0..10_000_000` ở `:559`), `for` (`:519-533`). **Không xoá hàng rào cũ một cách im lặng** — chuyển nó thành `charge()`; nếu A0 muốn giữ backstop thì backstop phải là cùng hàm `charge()`, không phải ngưỡng riêng.
- `E_CALL`: `interp/goi.rs:128`, `:208` (cạnh kiểm tra `MAX_CALL_DEPTH` — nhưng **không gộp** hai kiểm tra).
- `E_BUILTIN` / `E_SPAWN` / `E_AWAIT`: mã nằm ở `crates/vis-interp/src/builtin.rs` (dispatcher dùng chung) và `crates/vis-interp/src/rt.rs:112`, `:178`, `:242`. Vì VM dùng chung `vis_interp::builtin` (`vm.rs:13`, `:851`) và `rt::join_all` (`vm.rs:229`), một phần tính phí có thể đặt **duy nhất ở tầng dùng chung** — cần A0 chọn cách tổ chức (xem A0-3).

### Bytecode / VM
- **Không** tái dùng `vm.steps += 1` ở `vm.rs:736` như điểm tính phí chính — đó chính là hành vi bị VD-249 §3 loại bỏ. Giữ `MAX_STEPS` (`:97`) chỉ làm backstop tạm thời; việc bỏ/đổi nó phải được A0 duyệt rõ, không "ân thầm bỏ" (§3(e)).
- Vấn đề kỹ thuật: HIR không có opcode riêng cho "câu lệnh" hay "lần lặp while". Codegen `while` phát `JumpIfFalse` + `Jump` lùi (`codegen.rs:721-756`), `loop` phát `Jump` lùi (`:758-767`), chỉ `for` có `IterNext` (`codegen.rs:701`, VM `vm.rs:1088`). Vì vậy **không thể** suy ra `E_STMT`/`E_LOOP` từ loại opcode mà không đổi định dạng.
- **Phương án đề xuất (S)**: thêm **bảng phụ sự kiện** trong `Module` — một mảng song song với `code`, mỗi pc cho biết `(có tính phí?, mã sự kiện, trọng số)`. Codegen (`crates/vis-bytecode/src/codegen.rs`) điền bảng này tại cùng điểm nó phát lệnh tương ứng (`:691-767` cho vòng, `:809`-tương đương cho gọi hàm, v.v.). VM đọc bảng ở chu kỳ fetch (`vm.rs:729-736`) và gọi cùng hàm `charge()` thay vì `vm.steps += 1`. **Không thêm opcode mới.**
- **Phương án thay thế (O)**: thêm lệnh `Instr::Charge(w)` do codegen phát ở mỗi điểm sự kiện. Sạch hơn lúc chạy nhưng là **thêm opcode** — hiện bị đề bài cấm trong bước khảo sát; chỉ là phương án cho A0 quyết ở bước triển khai. Lưu ý `format.rs:232-234`: thêm lệnh ⇒ **thêm tag mới, không đổi tag cũ**.
- **Phương án bị loại (M)**: ánh xạ cố định opcode-class → trọng số, không đổi định dạng. Đây là "đếm lệnh VM" dưới tên khác, **vi phạm trực tiếp** VD-249 §3 (`agent/DECISIONS.md:2370-2371`), và không phân biệt được câu lệnh/lần lặp `while` (xem đoạn trên).

---

## 3. Quy tắc ngân sách cha/con (task)

- **Một pool duy nhất** cho toàn lần chạy: `Budget { remaining: u64 }` thuộc phiên chạy, được both engines chia sẻ (truyền qua `Host`/context dùng chung hiện đã có: `crates/vis-interp/src/rt.rs:75-78`, `:112`, và VM dùng chung host ở `vm.rs:228-234`). Không có budget riêng cho `main` hay cho task.
- **Không reset khi spawn** (`rt.rs:112`): task con bắt đầu với `remaining` hiện tại; `E_SPAWN` chỉ trừ phí tạo, không cấp thêm.
- **Không reset khi await** (`rt.rs:178`) và **không reset ở join cuối** (`rt.rs:242-247`, gọi từ `vm.rs:229`). `await` một task đã xong (kết quả cache, `rt.rs:33`, `:50`) vẫn trừ `E_AWAIT` (A0-5).
- **Thứ tự tiêu ngân sách**: các task chạy run-to-completion theo **thứ tự tạo** — hiện `join_all` duyệt `sched().tasks` theo thứ tự tạo (`rt.rs:243-246`) và VM gọi chung hàm này (`vm.rs:229`). Đề xuất chốt: thứ tự tiêu phí = thứ tự sự kiện thực thi trên luồng đơn, **không** phụ thuộc vào việc engine là interp hay VM.
- **Khi pool cạn bên trong task**: `charge()` trả `Abort::Trap(…)` ngay trong thân task ⇒ `wrap_task_error` (`rt.rs:225`, `:232-239`) bọc thành `VS0506`, CLI vẫn exit **101** (`crates/vis-cli/src/cli/run.rs:13-21`; chi tiết `vd249-exit101-triage.md`). Cần A0 chốt có dùng mã chẩn đoán riêng không (A0-4).
- **Không có "refund"**: hủy task/await không trả lại ngân sách đã tiêu (tránh lợi dụng spawn–hủy để kéo dài vô hạn).

---

## 4. Hành vi tại đúng biên ngân sách

1. **Thứ tự kiểm tra**: `charge(event, w)` thực hiện **nguyên tử**: so sánh `w` với `remaining` **trước khi** tạo khung gọi, trước khi mở vòng lặp, và **đặc biệt trước mọi tác dụng phụ** (`E_BUILTIN`: `print`/ghi filesystem/ghi SQLite/gọi chương trình ngoài đi qua `builtin.rs`; VM qua `vm.rs:839-859`). Không gọi host trước khi trừ phí.
2. **Công thức**: nếu `w > remaining` ⇒ trap, **không trừ**, **không thực hiện sự kiện**; ngược lại `remaining -= w` rồi mới thi hành.
3. **Biên chính xác**: đề xuất `w == remaining` được **cho phép** (sau đó `remaining = 0`); sự kiện tiếp theo cần `w > 0` sẽ trap. Kết quả phải **y hệt** ở hai engine (không được một bên trap sớm hơn một sự kiện). **Câu hỏi mở A0-6**: có muốn trap ngay khi `remaining` chạm 0 không.
4. **Tính tất định**: cùng một chương trình ⇒ cùng một dãy `(mã sự kiện, trọng số)` ⇒ cùng điểm trap, cùng `(stdout, exit, mã+thông điệp chẩn đoán)` ở cả hai engine. Không so sánh raw `steps` giữa hai engine — `crates/vis-vm/src/lib.rs:17-20` đã ghi rõ differential test hiện **không** so `steps` vì khác đơn vị; sau khi chốt hợp đồng, điều kiện này phải được xem lại bởi A0.
5. **Lỗi và thông điệp**: đề xuất tái dùng `VS0501` với thông điệp phân biệt, ví dụ `vượt ngân sách thực thi (điểm tính phí ngữ nghĩa); đã dùng …`; trong task trở thành `VS0506` kèm mã gốc. CLI exit 101 như hiện nay (`cli/run.rs:13-21`). **A0-4**.
6. **Phần còn dở**: **không có rollback/giao dịch**. Sự kiện gây trap **không** tạo tác dụng phụ (vì kiểm tra trước). Mọi tác dụng phụ của các sự kiện **trước đó** vẫn giữ lại: output đã in, tệp đã ghi, SQLite đã commit từng lệnh trước vẫn còn. Phải ghi rõ điều này trong SPEC/`vis explain` sau khi A0 chốt; đặc biệt kiểm tra ca "print ngay trước biên" và "ghi tệp ngay trước biên" (dùng `--fs-mock` tại `crates/vis-cli/src/main.rs:311-319` để hermetic).

---

## 5. Chi phí triển khai và ảnh hưởng cache/bytecode

Không cam kết ước lượng dòng/tuần-người (VD-249 §7, `agent/DECISIONS.md:2428`). Chỉ phân loại công việc:

- **HIR/frontend**: khả năng lớn là **không đổi** — các điểm sự kiện đã có trong `HStmt`/`HExpr`. Cần rà soát để bảo đảm dãy sự kiện ổn định giữa hai đường (đều nhận cùng HIR: `crates/vis-cli/src/main.rs:471-484`, `:533-600`).
- **Codegen + định dạng bytecode (phương án S/O)**: thêm bảng phụ hoặc opcode ⇒ **đổi `Module`**. Hệ quả bắt buộc: tăng `VERSION` (`crates/vis-bytecode/src/format.rs:44`, hiện 2) và cập nhật `encode` (`encode.rs:114-116`), `decode` (`decode.rs:250-251`), `verify` (`verify.rs:315`). Khoá cache đã chứa `VERSION` (`crates/vis-cli/src/cache.rs:55`, `:61`) ⇒ **cache cũ tự động invalid**, không cần sửa khoá. Cần A0 duyệt VERSION mới (A0-3).
- **Interp**: thêm `Budget` vào `Interp` (`interp.rs:91-120`), thay các `+=` ở `:300`, `:645` và các guard ở `:541`, `:559` bằng `charge()`; chèn tại `interp/goi.rs:128`, `:208`, `rt.rs:112`, `:178`.
- **VM**: thay `vm.steps += 1`/check ở `vm.rs:736-743` bằng tra bảng sự kiện; cân nhắc giữ `MAX_STEPS` làm backstop tới khi A0 cho bỏ. Các điểm gọi sẵn có để đối chiếu: `Call` (`:809`), `CallBuiltin` (`:839`), `CallMethod` (`:854`), `CallValue` (`:1157`), `IterNext` (`:1088`).
- **Tầng dùng chung**: `vis_interp::builtin` và `vis_interp::rt` (`vm.rs:13`, `:229`, `:851`) là đòn bẩy để "hai engine tính phí tại cùng sự kiện" với ít điểm sửa nhất; cần A0 chọn đặt `charge()` ở tầng này hay ở từng engine (A0-3).
- **CLI**: **không đổi** bảng exit (`cli/run.rs:13-21`) nếu mã lỗi vẫn là `VS05xx`; chỉ đổi thông điệp/ghi tài liệu.
- **Cache/bytecode tóm lại**: **có ảnh hưởng** nếu chọn S hoặc O (VERSION + cache key tự invalid một lần); **không ảnh hưởng** nếu A0 chọn giữ định dạng — nhưng phương án đó (M) bị loại vì vi phạm §3. Không đổi `MAGIC` (`format.rs:36`) và không đổi tag cũ (`format.rs:232-234`).

---

## 6. Kế hoạch test

Nguyên tắc: parity giữa interp và VM trên **cùng dãy sự kiện**, không phải cùng số opcode; harness hiện tại `crates/vis-vm/tests/vs_interp.rs` và ghi chú `crates/vis-vm/src/lib.rs:17-20` cần được mở rộng. Cần một **test-hook nội bộ** để đặt `BUDGET` nhỏ mà không thêm cờ CLI (A0-7).

1. **Vòng lặp**: ca 2M vòng (trong `main` và trong async task) — trước sửa: interp 0 / VM 101; sau sửa: hai engine cùng kết quả. Thêm `while true {}`, `loop {}`, `for` với miền lặp lớn, vòng lặp lồng nhau; assert cùng điểm trap và cùng exit 101.
2. **Gọi hàm / đệ quy**: (a) đệ quy tới ~1999 khung vẫn xong — chứng minh **cap độ sâu** (`interp.rs:1401`, `vm.rs:95`) là giới hạn riêng, không phải ngân sách; (b) đệ quy vượt 2000 ⇒ lỗi độ sâu riêng; (c) đệ quy không dừng mà **tiêu ngân sách trước khi chạm cap độ sâu** ⇒ trap ngân sách thắng; kiểm tra điều này ở cả hai engine.
3. **Nhiều task**: spawn N tasks cùng spin; chứng minh pool **dùng chung**, **không reset** ở spawn/await/join (`rt.rs:112`, `:178`, `:242`); await lại task đã cache vẫn trừ phí (A0-5); task lỗi bọc `VS0506` vẫn exit 101.
4. **Tác dụng phụ sát biên**: chọn điểm biên sao cho sự kiện cuối cùng được phép là `print`/ghi tệp/SQLite, sự kiện ngay sau đó bị trap. Assert: output đúng **một** lần, tệp có nội dung đúng (dùng `--fs-mock`, `main.rs:311-319`), **không** có nửa bản ghi từ sự kiện bị trap; lặp lại trên cả hai engine. Thêm ca `print` ngay **trước** `E_CALL` để kiểm tra thứ tự kiểm tra-trước-tác-dụng-phụ (§4.1).
5. **Ma trận parity**: mỗi ca trên assert bộ `(stdout, exit, [(mã, dòng, cột, thông điệp)])` giống hệt; không assert raw `steps` (đến khi A0 đổi rule ở `lib.rs:17-20`). Chạy cả `test/sem/**` hiện hữu để bảo đảm không hồi quy 525 ca semantics.

---

**Câu hỏi mở gửi A0 (phải chốt trước khi triển khai)** — các mã `A0-n` được trích dẫn ở trên:

| # | Câu hỏi | Khuyến nghị |
|---|---|---|
| A0-1 | Có tính phí riêng `E_EXPR` (w=1) hay gộp vào `E_STMT`? | Tính riêng để khớp `interp.rs:300` + `:645` và chặn biểu thức lớn. |
| A0-2 | Giá trị `BUDGET` ban đầu là bao nhiêu (đo trên workload nào)? | Chọn bằng đo đạc VD-249 §6; **không** kế thừa 10^7. |
| A0-3 | Biểu diễn bytecode: bảng phụ (S), opcode `Charge` (O), hay giữ nguyên (M)? Chấp nhận tăng `VERSION`? | Chọn **S**; duyệt VERSION 3; không đổi `MAGIC`/tag cũ. |
| A0-4 | Mã chẩn đoán riêng cho "vượt ngân sách" hay tái dùng `VS0501`/`VS0506`? | Tái dùng `VS0501`/`VS0506` với thông điệp riêng để giữ exit 101; nếu mã mới nằm ngoài `VS05xx`, phải sửa `cli/run.rs:13-21`. |
| A0-5 | `await` kết quả đã cache có bị tính phí không? | Có — tránh lợi dụng cache để né phí. |
| A0-6 | Tại `remaining == w`: cho phép sự kiện đó hay trap ngay? | Cho phép; trap ở sự kiện kế tiếp. |
| A0-7 | Cơ chế đặt budget nhỏ cho test (test-hook nội bộ vs cờ CLI)? | Test-hook nội bộ; không thêm cờ CLI. |
| A0-8 | Giữ `MAX_STEPS`/guard cũ làm backstop hay thay hẳn bằng `charge()`? | Giữ tạm làm back-stop, nhưng mọi guard phải gọi chung `charge()`; việc bỏ phải là quyết định riêng, không âm thầm. |
