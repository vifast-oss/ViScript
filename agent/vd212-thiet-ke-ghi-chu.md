# Ghi nhận thiết kế ST-212 (VS3-QĐ-1 async song song thật) — 2026-11-19

**Người ghi**: Agent 1 (uỷ quyền thay A0, VD-221) · **Trạng thái**: **propose/design — KHÔNG merging vào mã** (cần A0 ratify thiết kế + quyết thay AV-185)

## 1. Hiện trạng "scheduler async"

- `crates/vis-interp/src/rt.rs` — `Scheduler` một luồng, FIFO, `run-to-completion`, `Rc<Task>` chứa `Rc<RefCell<Chan>>` trong nhiều lớp. **Không một lát cắt nào của `Value` là `Send`**.
- `crates/vis-interp/src/value.rs` — enum `Value` có `Fn(Rc<Closure>)`, `Vm(Rc<VmFunc>)`, `Task(Rc<Task>)`, `Chan(Rc<RefCell<Chan>>)`, `Cap { .. }`. Một số variant hoàn toàn dữ liệu (`Enum { String, Vec<Value> }` / `List / Map / Record …`) nhưng chứa `Vec<Value>` (cũng !Send do có `Fn`).
- `crates/vis-bytecode/src/codegen.rs` — VM cũng sinh ra `$value:<tên>` và chia sẻ state qua `Rc`/`RefCell`, không phân biệt trước VM executor và interp executor.
- `crates/vis-types/src/tyck/expr.rs` và `crates/vis-types/src/hir.rs` — hàm `fn` và async luôn là `HFn` với `body: Vec<HStmt>` (phòng ngoài-của-linux tạm; still in HIR record).

## 2. Số baseline (isapect Máy ps, 4 cores busy, `load 1`, mespunt is not the lowest)

| Phép đo (interp) | Kết quả |
|---|---|
| `heavy(2000)*4*6` | **0,128 s** (interp) / **0,077 s** (VM) |
| `heavy(50000)*100` | **12,92 s** interp; VM hit step-limit 10^7 ở <1 s ⇒ cần `--vm` steward step |
| `parallel(i)= heavy(i)*4` với 4 async-i | thời gian = tổng cả 4 (FIFO, một luồng) |

## 3. Để ST-212 hoàn thành, cần

1. **`Send` boundary for `Value`**: Audit enum `Value` và cho mọi `Rc`/`RefCell` wrap chuyển sang `Arc`/`Mutex` — **cost**: thay đổi mọi reference share trong code ngồi lên. Cần bản CBIFF toàn bộ VM + interp đều Sendable.
2. **Parallel worker**: spawn a pool of `W` OS threads, each running one `vis_vm::run` bytecode concurrently, fed from the ready-queue. But: **tasks share `Chan`** — `&Chan` hiện là `RefCell<Chan>` — phải chia sẻ qua Mutex. **Sập góc**: nếu không quan tâm tới deadlock, race logic sẽ hỏng mọi test `spawn-nhieu-join-cuoi` / `print-trong-task-thu-tu`.
3. **Deterministic join**: đảm bảo join cuối vẫn cho output giống nhau — hoặc chấp nhận tường trình "không tất định" và cập nhật golden stdout — impact: **28 task async output order đổi**; golden bẽ cần drop.
4. **VS0508**: values not Sendable rejected at spawn. **Cần, không kể gì** nếu không chuyển `Value` → `Arc + Mutex` — `Chan`, `Task`, `Fn` v.v. sẽ bị reject hết mọi `spawn f(closure)`.
5. **`$chan[T](n)`**: rendezvous — không thể với một luồng hiện tại — hoạt động với `Mutex<Chan>` cũng vậy (nếu đa luồng thực sự thì **deadlock** nếu không dùng Condvar đúng cách).

## 4. Lộ trình (đề xuất, cần A0 chốt)

(min 3 tuần-người thật)

- **Pha 1**: `Sendable`/`IsShareable` predicate + VS0508 assertion tại spawn sites (~2 h)
- **Pha 2**: thay đổi `scheduler` sang `Arc<Mutex<Chan>>`+ `Arc<Task>` + `Arc::clone` vào worker threads; thêm `Send + Sync` bound cho `Value` (~6–8 h, phá hết 469 golden nếu không cẩn thận)
- **Pha 3**: song song hóa interp hoàn toàn cho `async fn` spawn-ing (~1 tuần)
- **Pha 4**: bench dựng lại: `W=min(nproc,N)` với `task::heavy(100k)` CPU — đo scaling curve
- **Pha 5**: cân nhắc thay thế FIFO scheduler hoặc đổi sang "structured concurrency" (scope-limited)

## 5. Vì sao không nên làm ngay

- **Gây sập golden** — every `spawn`/`print-trong-task-thu-tu` golden phải regenerate; nếu A0 không cho phép, không làm được.
- **Compile boundary-level changes** — kéo dài kỹ thuật VM + interp + format VBC1 ≈ toàn bộ compiler core.
- **chan(0) deadlock** — VD-190 đã xác nhận `chan(0)` phải là rendezvous, và **ViScript đã chặn `chan(0)` từ VD-90** (cần một 5h audit kiểu hiển nhiên).
- **VM step cap 10^7** — nếu parallelism làm giảm thời gian interp xuống dưới mức này, VM delivery ổn — nhưng **cũng có thể** *doubled*execution (if deadlock code) và bắt runtime cap gãy.

## 6. Chốt luận

ST-212 trong threat model này **không thể** trong một đêm. Để hoàn thành cần:

1. A0 ratify hướng (định kỳ delegate) — toàn bộ proposed forward
2. Regenerate golden trước bước 1 (hazlink-test cho bước 2)
3. Dự kiến: 2 tuần toàn sức + verify ở ít nhất W=2 và W=min(nproc/2) — mỗi golden fetch lại từ source-PDF
4. Có thể cần VD mới lớn để đảm bảo không làm sập `chan(0)` semantics

**Nhãn**: VD-225 hạn cho phép proxy, nhưng phạm vi này lớn hơn ratification pattern trước (VD-184). Stick VD-185-ST-212 như một promise chứ không phải một action — ghi nhận Vor future run-through.
