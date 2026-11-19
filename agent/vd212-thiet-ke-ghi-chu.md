# Thiết kế ST-212 (VS3-QĐ-1) — hai hướng, Agent 0 chọn (2026-11-19)

**Người ghi**: Agent 1 · **Ngày**: 2026-11-19 (~10:30) · **Trạng thái**: **CHỈ TÀI LIỆU — KHÔNG merge mã**
**Quyết định — VD-240 (Agent 0, 2026-11-19)**: **chọn hướng (b)** — giữ một luồng + test grid,
~0,5–1 ngày, làm **sau** khi pha 1 đóng chính thức. Hướng (a) để sang pha sau; nếu mở lại sẽ ưu tiên
deep-copy/byte-serialize qua kênh (đúng VD-178 QĐ-1), **không** chuyển `Value` sang `Arc+Mutex`
(chưa phải quyết định cuối). **Quyết định trước đó**: VD-235 (chuyển pha 2) ⇒ ST-212 sang pha 2. Đây là bản thiết kế lưu lại cả hai hướng.
**Ngân sách**: ~3 h, chỉ tài liệu; không refactor code ngoài `docs`.
**Mục tiêu ST-212**: `async fn` + `spawn` chạy **song song thật** (nhiều luồng), thay vì scheduler một luồng run-to-completion hiện tại.

## 1. Hiện trạng (dẫn tệp/thật)

- `crates/vis-interp/src/rt.rs` — `Scheduler` một luồng (`Rc<Task>`, `Rc<RefCell<Chan>>`); FIFO; run-to-completion; không `"Send"` ở bất cứ đâu.
- `crates/vis-interp/src/value.rs` (line 37-78) — `Value` gồm `Proc/…/Closure(Fn)`, `Vm(Rc<VmFunc>)`, `Task(Rc<Task>)`, `Chan(Rc<RefCell<Chan>>)`, `Cap { … }` ⇒ **không `Send`**.
  - `Closure.env: Rc<RefCell<HashMap<u32, Value>>>` (`value.rs`) — environment điện tử dùng `RefCell` (khó `Sync`).
  - `Chan` chứa `RefCell<Chan>` (`crates/vis-interp/src/value.rs`); đọc/ghi một luồng — không chia sẻ được.
- `crates/vis-bytecode/src/codegen.rs` — VM cũng một luồng; cap `10^7` bước (cùng ngưỡng interp, `vm.rs:87`); `$value:<tên>` dựng builtin qua bytecode, vẫn đơn luồng.
- Fixtures hiện hữu: `test/sem/async/*.vis` (35 ca, trong đó `spawn-*.vis` 4 ca; `await-trong-vong-lap.vis` 1 ca) — **mọi ca đều ngầm giả định một luồng** (output theo thứ tự khai báo).
- `spawn` ngữ nghĩa hiện tại: **hàm async gọi sẽ trả `Task[T]` ngay** nhưng **KHÔNG chạy** cho tới `await`; output task chỉ hiện khi `await` — byte-identical giữa interp và VM.

## 2. Ràng buộc không được phá (cả 2 hướng)

1. `chan(0)` trả `VS0505` (runtime) — code hiện reject (VD-90); **VD-178 QĐ-2** (chan-rendezvous, ratify của A0) và **VD-190 (Q9 = A)** cho phép mở rendezvous ở **pha 2** (SO-NO `#145` đang mở chuyển VS3). Doc này không tự mở `chan(0)`; chỉ đề cập "nếu A0 cho phép mở rendezvous, `chan(0)` nằm trong phạm vi pha 2" và ghi nhận drift hiện tại.
2. `recv` sau `close` trả `None`; `send` sau `close` trả `VS0504`; deadlock → `VS0505` — giữ nguyên.
3. Fixtures `test/sem/async/*.vis` chạy **byte-identical** interp vs VM cho mọi chương trình đã có (nhiều chương trình hiện tại in output theo thứ tự `spawn`/`await` trong **một** luồng — song song thật cũng phải cho output tương đương).
4. Không đổi SPEC trong bước này (SPEC-lex chỉ sửa qua Agent 0).

## 3. Hướng (a) — Worker pool **ngoài `spawn`**; gửi/nhận qua kênh bằng **sao chép sâu**

### 3.1 Cơ chế (theo VD-178 QĐ-1, nó ratify mô hình này)

- **Thay đổi cấu trúc tối thiểu nằm ở ngoài `spawn`**:
  - Một `ready: Mutex<VecDeque<TaskMsg>>`; `W = min(nproc, 4)` worker OS-thread.
  - `TaskMsg = (fidx, args: Vec<u8>)` — **args tuần tự-hoá** (không chuyển `Value` muộn xuống luồng); `Value` được deep-clone trên đường vào queue (xã hội theo từng byte → tránh triệt để `Rc`/`RefCell` sang thread khác).
  - Kết quả từ worker (MßJoinCell) được nhận theo thread-index (round-robin buffer); main-thread in theo index ⇒ **thứ tự in byte-identical** với một luồng phiên bản cũ nhất.
- `Chan`: `Arc<Mutex<Chan>>` + `Condvar`; gửi giá trị-pointer (raw pointers / `Send`-able payload); chỉ dữ liệu (int/float/bool/size/`list`/`map`/`record`/`enum` chứa dữ liệu) và `&str` ghim vào nội dung được gửi.
- **`spawn f(args)`**: deep-clone `args` (mọi closure không gửi được vì `env: Rc<RefCell<HashMap>>` không `Send`; `Fn`/`Task`/`VmFunc`/`Chan` — gửi bất kỳ → `VS0508`).
- **Compile-time regex** trên AST: mọi giá trị đóng gói `spawn` phải `Sendable` (intène `Send`) — đồ để kiểm tra tại `let x: Some(Fn-x)` cũng trả `VS0508` ngay tại lower/check; tại runtime kiểm tra lần 2 không thừa.

### 3.2 Tác động test/golden (ước lượng thận trọng)

- `test/sem/async/print-trong-task-thu-tu.vis` — hiện kỳ vọng `print("f1")` rồi `print("f2")`; với buffer-then-join theo index, output vẫn `f1\nf2\n` ⇒ **golden giữ nguyên**.
- `spawn-nhieu-join-cuoi.vis` — output `print(1)\nprint(2)\nprint(3)\nmain` theo thứ tự trái→phải-not → đổi thành `main` trước hay sau tùy buffer; golden **đã phải đổi** thành `main\n1\n2\n3` hoặc giữ `1\n2\n3\nmain` — cần dùng lại expected file với **".out" trong do cấu hình deterministic**.

| File async | Golden | Sau hướng (a) |
|---|---|---|
| print-trong-task-thu-tu | `f1\nf2\n` | giữ nguyên |
| spawn-nhieu-join-cuoi | `1\n2\n3\nmain` (theo .out hiện hữu) | **đổi** (buffer order) — cần re-record |
| task-in-trong-than | phụ thuộc await ordering | có thể giữ (FIFO thật) |

- **Ước lượng thay đổi golden**: 3–6 `.out`; 12–16 file `test/sem/async/*.vis` + kiêm tra output lại.
- Thay `MAX_CALL_DEPTH`/step cap nào cũng không chạm — mỗi worker có trong nội bộ mình.

### 3.3 Rủi ro hai engine lệch (chi tiết thật)

- VM (một luồng có step cap; interp **cũng** có **10^7 bước** cho vòng lặp — `crates/vis-interp/src/interp.rs:502-530`). Nếu W worker chạy task CPU-intensity với ngắt 10^7 giữa kỳ lệch thì **interp vs VM không byte-identical** về state (nhưng byte-identical về behavior nếu phát hiện).
- Capture semantics: closure không thể deep-cụpe `env` mà giữ mọi side-effect của `var` trên biến của hàm cha — đây là **điểm vỡ chính**; hướng này **không cho phép `spawn fn(…)` với closure capture `var`** ⇒ bật `VS0508` ở compile-time nếu bám `var` trong body async gửi đi.
- `print` bên trong `async fn`: **buffer-then-join** trả ra sau join — **cần định nghĩa rõ** thêm Spec rõ ràng (VD nào tiếp theo phải bổ sung `spawn` semantics cho printed output).

### 3.4 Ước lượng công

- Phân tích Send từng biến thể `Value` (chỉ `Sendable` whitelist): **2–3 h**
- `SendableValue` + deep-copy: **4–6 h** (dữ liệu) + serialization paths cho `Record/Enum list map`: **4 h** (20 đường khác nhau)
- Worker pool + ready-queue + termination detection: **6–10 h**
- Golden sweep & re-record (~5 files): **2–3 h**
- Verification (gate + sem + workspace): **3 h**
- **Tổng: 3,5–5 tuần-người** (thực tế khi trừ đất: ~4 tuần), phá 0 crate-level, giữ byte-identical nếu phát hiện nguy hiểm.
- Nếu A0 muốn nhanh hơn: implementation "chỉ `spawn` cấp tệp`, không closure capture" thu hẹp surface (ước lượng ~2 tuần).

## 4. Hướng (b) — **Giữ một luồng**, chỉ thêm test grid

### 4.1 Cơ chế

- **Không chạm scheduler**; không có worker; không đụng Queue; giữ FIFO run-to-completion.
- Thêm **test grid** `test/sem/async/*.vis` để khoá hành vi một-luồng hiện hữu:
  - `async-khong-goi-khong-chay.vis` — không gọi: chưa chạy ✓
  - `spawn-*.vis` — spawn không chạy cho tới `await` ✓ (có sẵn; thêm ca spawn-task-throw)
  - `await-trong-vong-lap.vis` — cache đúng một vòng ✓ (có sẵn)
- **Docs cheatsheet note** ~ 5 dòng: "async/spawn hiện một luồng — chờ ST-212 (pha 2)".

### 4.2 Tác động golden

- **0 golden đổi** — hành vi chạy giữ nguyên, output không bị sắp xếp lại.

### 4.3 Rủi ro

- **Lệch kỳ vọng người dùng**: `spawn` phải là "chạy song song" về mặt tinh thần lập trình — hướng (b) **không** giải quyết đâu, chỉ ghi rõ nó là một luồng.
- Không decuple động viên-đóng (VD-185 pha 2).

### 4.4 Ước lượng

- Thêm 3–4 test marker `.vis` (~0,5 day)
- Cheatsheet note + `docs/ViScript-thiet-ke.md` reflect: "hiện tại một luồng" (~0,5 h)
- **Tổng: 0,5–1 ngày**.

## 5. Bảng so sánh

| Tiêu chí | (a) worker pool + deep-clone | (b) một luồng + test grid |
|---|---|---|
| Phạm vi | Code lớn (sau pha 1) | Test + docs |
| Golden đổi | **3–6 `.out`** | **0** |
| Byte-identical giữ được? | Có, nếu `print buffer-then-join + index` | Có, không thay đổi |
| Rủi ro hai engine | Trung bình (capture shared Rc) | None (hành vi không đổi) |
| Công ước | ~3,5–5 tuần-người | ~0,5–1 ngày |
| Sửa spec? | Không (thuộc pha 2) | Không |
| `Value → Arc` bắt buộc? | Không (chỉ bytes) | Không |
| `chan(0)` rendezvous | **Có thể** mở nếu A0 cho phép (VD-178 QĐ-2 / VD-190) | Không |
| `VS0508` | **Có** (chỉ gửi dữ liệu thuần) | Không cần |
| Chọn khi nào | "Muốn async song song thật sớm" | "Muốn duy trì ổn định, treo async đến pha 1 chính thức xong" |

## 6. A0 chọn hướng — lưu ý trước khi ratify

1. **Trích dẫn**: VD-178 QĐ-1 (spawn = deep-copy; `VS0508` cho không-Send) + VD-178 QĐ-2 / VD-190 (chan(0) rendezvous = pha 2); VD-185 (VS3-QĐ-1 = async song song thật); VD-180/196 (pha 2); **không** dùng VD-185 làm căn cứ cho "loss FIFO" — UT-pre-existing đã ưu tiên một-luồng.
2. Cả hai hướng đều **không chạm** các phần đã đóng pha 1 (VD-222..VD-229).
3. (a) chi phí > **3 tuần** do capture semantics + serialization paths; nếu A0 muốn nhanh, (a) chỉ khả thi khi **capture loại bỏ** (spawn chỉ cho cấp tệp cấp chức năng cấp, không closure `var` bắt giữ) — thu hẹp surface ⇒ ~2 tuần.
4. (b) chốt được **trong một buổi**; lệch giảm sát zero; đủ cho "người dùng biết rõ scheduler hiện tại một luồng".

**Agent 0 chọn khi tiếp nhận; Agent 1 chỉ thực thi theo quyết định đó.**
