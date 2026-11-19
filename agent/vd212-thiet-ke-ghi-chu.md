# Thiết kế ST-212 (VS3-QĐ-1 — async song song thật) — 2 hướng, Agent 0 chọn

**Người ghi**: Agent 1 · **Ngày**: 2026-11-19 (sau VD-235) · **Trạng thái**: **CHỈ TÀI LIỆU — KHÔNG merge mã**
**Nguồn**: VD-235 (Agent 0 ratify, 2026-11-19) — chuyển ST-212 sang pha 2. **Agent 0 sẽ chọn hướng**.

## 1. Hiện trạng (điểm xuất phát, đã đo)

- `crates/vis-interp/src/rt.rs` — `Scheduler` một luồng, FIFO, `run-to-completion`; `Rc<Task>` mang `Rc<RefCell<Chan>>`.
- `crates/vis-interp/src/value.rs` — `Value` chứa `Fn(Rc<Closure>)`, `Vm(Rc<VmFunc>)`, `Task/Async`, `Chan/RC<RefCell<…>>` → **không `Send`**.
- VM (`crates/vis-bytecode`) sinh `$value:<tên>` để dựng giá trị; VM bản hiện tại cũng một luồng, step cap `10^7`.
- `async fn` trong ViScript hiện chạy **tuần tự**; `spawn`-thời-điểm ghim `W=1` (một threaapoint); mọi đường đều FIFO.
- Baseline (đo ps, máy 12-core, idle): `heavy(2000)*4*6` = **0,128 s interp / 0,077 s VM**; `heavy(50000)*100` interp ≈ **12,9 s** (VM vượt step cap sớm hơn).

## 2. Điều kiện cứng cho mọi hướng

1. `chan(0)` **vẫn** bị từ chối `VS0505` (ràng buộc hiện hữu; không đổi).
2. `recv` sau `close` trả `none`; `send` sau `close` trả `VS0504`; deadlock → `VS0505` không treo (đang có).
3. Không đổi ngữ nghĩa spec cho các phần đã đóng pha 1 (VD-222..VD-229 đã được A0 ratify).
4. `Value` vẫn có ngữ nghĩa giống nhau giữa interp và VM (byte-identical nếu chỉ quyết định nơi tính toán) — đây là điểm dễ lệch nhất.

## 3. Hướng (a) — Worker pool **bên ngoài** `spawn`, gửi/nhận kênh bằng **sao chép sâu**

### 3.1 Kiến trúc
- Giữ nguyên `spawn` API và hàm `Chan`. Thay `Rc<...>` bằng `Arc<Mutex<Chan>>` ở **một lớp bên ngoài** chỉ quanh queue sẵn sàng (`ready: Mutex<VecDeque<Task>>`).
- `Value`: cho phép **đóng gói** thành `VisSendable` — một dạng sao chép sâu (deep clone) của giá trị vi phạm `Sendable` thì bị từ chối tại `spawn` với mã **VS0508**.
- Pool: `W = min(nproc, 4)` worker thread; mỗi worker chạy `vish_tap` trên list[Loop] trong `VM/interp` **riêng**; giao việc qua queue; join cuối đóng với mọi worker.
- `Chan`: `Arc<Mutex<Chan>>` + `Condvar`; gửi `Value` → **clone** (số học / giá trị dữ liệu); `Fn`/`Task`/`VmFunc` không gửi được ⇒ `VS0508` tại spawn (không tại `send`).
- Deterministic join: giữ thứ tự in bằng cách gom output theo task-index (buffer); in sau khi join xong ⇒ golden **có thể** giữ nguyên cho mọi test song song.
- `$await` luôn chạy task tới hoàn tất (không đổi). `spawn` trả `Task[T]` (handle, check_async trong `task_wait`).

### 3.2 Tác động lên golden / fixture
- Test plugin `tests/spawn*.vis`, `spawn-ben-trong-spawn.vis`: output order **đổi** theo thứ tự thread — cần **2 loại thân**: (i) test output không thứ tự: golden được chốt lại theo sorted; (ii) test print-trong-task: output gom lại ⇒ golden giữ nguyên nếu join-buffer duyệt theo index. Ước tính **shake 12–20 golden `.out`** + `spawn-*.vis` 8 cá.
- VM: `$value:<tên>` lỏng tham chiếu queue-local; worker riêng cache có chủ đích ⇒ phải recalc từ scratch mỗi worker (step cap vẫn đúng mỗi worker).

### 3.3 Rủi ro hai engine lệch
- Output in trong task **lệch thứ tự giữa interp và VM** nếu giữ in trong worker thay vì buffer-then-join. **Chặn**: mọi `print` trong `async fn` chỉ ghi vào buffer, join in theo thứ tự task-index ⇒ interp & VM in giống byte.
- `Task` handle đọc được độ trễ giữa worker threads nếu không mutex ⇒ panic tiềm tàng; đóng trong `Mutex<`**wait-list**.
- VM step cap: mỗi worker 10^7 cap ⇒ hành vi chạy > cap round-robin mode **giống interp** nếu interp không cap — interp không có cap theo `MAX_CALL_DEPTH` recursion only.

### 3.4 Công ước cho trường hợp drift
- Nếu drift interp ≠ VM ở **mất độ FIFO** — chấp nhận loss `FIFO` (VD-185 đã chọn hướng này); cần **thêm VS0508** ở mọi gửi giá trị không clone-được ⇒ ít test hơn tổng số bình luận của một test song song.

### 3.5 Ước lượng
- Audit `#[derive(Clone)]` Value + mọi path capture Rc (≈ 3 h)
- Rework queue/Channel + worker (≈ 8–12 h)
- Sao chép sâu (serialization) Value→Bytes khi qua Worker (≈ 4–6 h) ⇒ tăng try-as-clone; Memory owned độc lập của mỗi worker
- Golden sweep + re-trace (≈ 6–8 h)
- Verification (gate + sem + fuzz) — 2 h
- **Tổng: 2,0–2,5 tuần-người**, 1–2 golden-hash trên mỗi input là `spawn-ben-trong-spawn`

## 4. Hướng (b) — **Giữ một luồng**, chỉ thêm test grid (không đổi mã chạy)

### 4.1 Kiến trúc
- Không chạm scheduler, không thêm worker. Giữ FIFO run-to-completion.
- Thêm **test grid** trong `test/sem/async/` hiển thị hành vi hiện có một cách hiển tường:
  - `async-parallel-cy-1.vis`: 2 task CPU, in tuần tự (không song song thật; chạy tuần tự).
  - `spawn-spawn-send.vis`: spawn gửi qua kênh với copy giá trị plain data.
  - `async-join-timeout.vis`: join quanh task nhận lỗi — giống hệt ref hiện hữu.
  - `chan-many-send.vis`: 100 gửi qua buffer `chan[1]` để xác nhận không treo (VS0505 path qua test).
- Cải thiện docs/cheatsheet/comment về "async hiện một luồng"; gắn với backlog rõ ràng.

### 4.2 Tác động lên golden
- **0 golden đổi**. Mọi output `.out` giữ nguyên (hành vi không đổi).
- Fuzz parser không đổi (không có path mới).

### 4.3 Rủi ro
- **Lệch match gun chí mạng**: giữ "false advertising" — `news.spawn` converti Luật rất đơn giản nhưng callers hiểu nhầm là parallel-thật. Đây là rủi ro chính.
- Không giải quyết VD-185/VS3-QĐ-1; chỉ hoãn rõ ràng.

### 4.4 Ước lượng
- Viết ~8–12 ca `test/sem/async/*.vis` (≈ 1 buổi); cheatsheet note (≈ 20 phút); verify `vis test` xanh (≈ 30 phút).
- **Tổng: 0,5–1 ngày**.

## 5. Bảng so sánh

| Tiêu chí | Hướng (a) worker pool | Hướng (b) một luồng + test grid |
|---|---|---|
| Nội dung | Code mới: queue/Mutex/Condvar/Safe & clone | Test mới + ghi chú |
| Golden đổi | 12–20 `.out` + 8 test song song | **0** |
| Rủi ro hai engine lệch | Cao trung bình — cần buffer-then-join | Không tăng |
| Sửa spec? | Không (thuộc pha 2) | Không |
| Công ước | ~2 tuần-người | ~0,5–1 ngày |
| `Value` → `Arc/Mutex` | Bắt buộc (chỉ ở struct send queue + channels) | Không |
| `VS0508` | Bổ sung rõ ràng | Tài liệu ghi nhận |
| Chọn khi nào | Nếu A0 muốn ViScript có song song thật ở pha 2 | Nếu A0 muốn "tạm giữ ngữ nghĩa đã chốt, không phá workspace" |

## 6. Đề xuất cho A0 để chọn

- **Nếu kèm điều kiện**: AST phải duyệt `spawn` trước khi ratify bất cứ hướng nào; cả hai đều không sửa SPEC trong cùng bước này.
- **(a)** đúng ý "song song thật" VD-185/QĐ-1; cân nhắc thời điểm sau khi pha 1 đóng chính thức trên mac (VD-237).
- **(b)** an toàn hơn về golden; nhỏ 2 h; rủi ro giữ nguyên một luồng (điểm VD-185 sẵn sàng) và cộng thêm ca trắc nghiệm cho đội người dùng hiểu rõ hiện trạng.

**Agent 0 chọn trong đợt kế, đọt pha 2.**
