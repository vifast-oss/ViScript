# VD-250 / ST-264 — Lát 0: prototype suspend/resume hai engine (nội bộ)

- Ngày: 2026-11-20
- Phạm vi: **Lát 0, prototype nội bộ**. Không commit. Không đụng runtime mặc định, bytecode
  phát hành, `Cargo.toml`/VERSION/opcode, `agent/DECISIONS.md`, `agent/QUEUE.md`,
  `crates/vis-interp/src/net.rs`, `crates/vis-cli/tests/http.rs`, hay bất kỳ tệp
  `vd249-*`/`vd250-*` nào của agent khác.
- Tệp tạo mới duy nhất về code: `crates/vis-interp/tests/lat0_suspend.rs`.
- Báo cáo này là tệp thứ hai được phép tạo.

## 1. Kết quả lệnh

```
cargo test -p vis-interp --test lat0_suspend
  → running 10 tests; test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

./vis-dev check
  → fmt OK; clippy OK (-D warnings); luật R1..R6 OK; self-test OK;
    build OK; doc-examples OK; vis-dev check: PASS
```

Không có warning từ tệp test. (Dòng `/tmp/vrepfx.out: Permission denied` trong phần fixture
báo cáo là của script có sẵn, fixtures vẫn `PASS`, không liên quan tới prototype.)

## 2. Kiến trúc prototype (modeled)

Hai mô hình engine nằm trong cùng một tệp integration test, dùng chung:

- `V` (giá trị tối thiểu: `Int`/`Unit`/`Err` — `Err` là **value** để đi qua continuation như
  `Trap`), `Ctx` (log tác dụng phụ + cờ event giả + stdout giả),
  `Scheduler<F> { frames: Vec<F>, ready: VecDeque<usize> }` (heap frame + FIFO),
  trait `HFrame`, `pump()` (vòng lặp phẳng) và `wake()` (bật cờ event, đánh thức frame
  `Pending`, đưa resume value vào, xếp lại `ready`).

**Engine 1 — VM-like** (`vm_model::VmFrame`, mirror `crates/vis-vm/src/vm.rs:124-135`):
các trường `fi, pc, locals, captured, st_cu, writeback` được giữ **nguyên tên và ngữ nghĩa**
(`ClosureCells = Rc<RefCell<Vec<V>>>`, `CapturedMirror = Vec<(u16, usize)>` như `vm.rs:120-122`).
Resume = **tiến `pc` qua instruction `Await`**, đẩy giá trị resume vào `st_cu`, rồi tiếp tục
vòng `while pc < code.len()`. Không instruction nào trước `Await` được chạy lại.

**Engine 2 — Interp-like** (`interp_model::IFrame`): vị trí là **statement index `pc`** trên một
statement list HIR-like (`SetLocal`/`SetCap`/`Eval`/`Return`) + `work: Vec<Work>` là **continuation
stack nằm trên heap** (frame field), và `vals`/`locals`/`captured` là heap cells. Toàn bộ việc
đánh giá biểu thức (kể cả `Add`, `Call`-like `LogVal`, `Await`) đi qua `Work` — **không qua Rust
call stack**. Resume = đẩy giá trị I/O vào `vals`; continuation cha đang ở đỉnh `work` tiêu thụ
nó; operand của `Await` không bị đánh giá lại.

"I/O wait" là cờ trong `Ctx::events` (predicate `fired(ev)`), **không có socket thật**.

## 3. 6 tiêu chí Lát 0 → evidence table

| # | Tiêu chí A0 | Test (engine VM) | Test (engine interp) | Assertion khoá |
|---|---|---|---|---|
| 1 | A chạy→suspend→B chạy→A resume đúng vị trí | `vm_a_suspends_b_runs_a_resumes` (dòng 661) | `interp_a_suspends_b_runs_a_resumes` (dòng 812) | log = `["A:pre","B:run","A:post"]`; sau `pump` đầu, A `Pending`, B `Ready`; `A:post` nằm **sau** `B:run` |
| 2 | Local + capture giữ giá trị qua suspend | `vm_locals_captures_survive_suspend` (dòng 705) | `interp_locals_captures_survive_suspend` (dòng 845) | `result == Int(17)` = `10 + 7` sau resume; ô capture chia sẻ == `Int(7)` (writeback) |
| 3 | Resume không lặp side-effect | `vm_a_suspends_b_runs_a_resumes` (dòng 661) | `interp_a_suspends_b_runs_a_resumes` (dòng 812) | `A:pre` xuất hiện **đúng 1 lần** trong log cuối (so sánh vector đầy đủ) |
| 4 | Result **và** error đi qua continuation | `vm_result_and_error_through_continuation` (dòng 741) | `interp_result_and_error_through_continuation` (dòng 873) | Ok: `Int(41)+1 == Int(42)`; Err: `result == Err(..)` và `after-await` **không** được log (short-circuit) |
| 5 | `await` trong biểu thức lồng, thứ tự đánh giá | `vm_nested_expression_order` (dòng 780) | `interp_nested_expression_order` (dòng 909) | log = `["operand-eval","g","call=42"]` — operand trước, `g` sau resume, outer `call` cuối |
| 6 | Không đổi runtime/bytecode; không stack lớn/đệ quy | `both_engines_parity_and_flat_stack` (dòng 932) + phần mô tả mã | cùng test | `max_stack <= 4`, `max_work <= 8`; `frames.len()==1`, `ready.is_empty()`; `pump()` là `while let Some(id) = ready.pop_front()` |

Ngoài ra `baseline_matches_current_vis` (dòng 995) là baseline so với `vis` hiện tại.

### Cách mỗi frame giữ locals/captures/position

- VM: frame là phần tử `Scheduler::frames` ⇒ **sống trên heap** qua suspend. `pc` không đổi khi
  park (vẫn trỏ `Await`), resume `pc += 1`. `locals`/`captured` là `Vec<V>` trong frame; `Err`
  trước suspend được writeback ra `ClosureCells` ở `Ret`.
- Interp: `pc` (statement index) + `work` (continuation stack) + `vals` + `locals`/`captured`
  đều là field của `IFrame` trên heap; `step()` trả `Suspended` mà **không xoá** các field này.
  Resume chỉ đẩy result vào `vals`, tiếp tục pop `work` — đều là thao tác trên heap.

## 4. Phát hiện về biểu thức lồng (nested `await`) và phần chưa làm

Ca mẫu: `call(add(await ioA(), g()))`.

- VM (bytecode nhỏ): `EffectVal("operand-eval",0) ; Await(ioA) ; EffectVal("g",2) ; Add ;
  LogVal("call") ; Ret`.
- Interp (HIR-like): `LogVal("call", Add(Await(ioA, Effect("operand-eval",0)), Effect("g",2)))`.

Cả hai cho log giống hệt `["operand-eval","g","call=42"]`. **Quy tắc thứ tự được minh hoạ**:
(1) toán hạng của `await` được đánh giá **trước** khi park (`operand-eval` vào log trước suspend);
(2) `await` chỉ park **sau** khi có operand — không đánh giá lại operand lúc resume;
(3) vế phải `g()` **chưa** được đánh giá lúc suspend, chỉ chạy sau resume;
(4) outer `call` nhận đúng `42` từ continuation, tức kết quả resume chảy đúng chỗ.

**Chưa hiện thực đầy đủ trong prototype** (phải ghi rõ, không được coi là đã xong):

- Chỉ có `await` ở dạng biểu thức đơn trong prototype; chưa generalization tới **mọi vị trí
  `await` mà checker hiện chấp nhận** (VD-250 §3). Không có short-circuit `&&`/`||`, vòng lặp,
  record/field, pattern, hay `await` làm callee.
- Chưa có call frame thật cho hàm user: `LogVal` đóng vai "outer call" nhưng **không spawn frame
  con**, nên chưa chứng minh continuation đi qua **nhiều frame** (chỉ trong một frame). Đây là
  điểm cần kiểm chứng ở Lát 1.
- `Err` được mô hình bằng `V::Err` và lan bằng "top-of-stack is Err ⇒ bỏ qua log/effect", chưa
  phải cơ chế `Trap`/`VS0506` thật; span wrapping / final join chưa được mô hình.
- Event là cờ do harness bật tay (`wake`), chưa phải event trace byte-identical giữa hai engine
  theo VD-250 §2.1; chưa đo dependency/ngân sách/hạn ngạch.

## 5. Khẳng định về runtime mặc định / bytecode

Prototype **không** sửa default runtime, không đổi bytecode phát hành, không đổi `Cargo.toml`
`VERSION`, không thêm/chỉnh opcode, không tăng stack, không gọi đệ quy scheduler. Mọi mã nằm
trong integration test mới; không symbol nào được export sang crate, không đường `goi_than` thật
bị chạm. `vis-dev check` xanh ⇒ build/doc-examples/clippy/quy tắc không regression.

Baseline `baseline_matches_current_vis` chạy `vis run` **hiện tại** (default lẫn
`--vm --no-cache`) trên một file `.vis` trong `std::env::temp_dir()` (`/tmp`, xoá sau test),
cả hai in `42\n`; prototype (hai engine) cũng in đúng `42\n` cho phép tính tương đương. Đây là
bằng chứng **case không suspend giữ hành vi hiện tại**; nó **không** chứng minh gì về suspend
trong runtime thật.

## 6. Integration gap trước Lát 1 (model spike, chứ chưa phải implementation)

Đây là **model spike**: frame/continuation được viết lại trong test, **không phải** frame thật
của `vis-vm` hay continuation thật của `vis-interp`. Trước khi xin A0 mở Lát 1 phải giải quyết:

1. **Nối continuation vào HIR/bytecode thật.** Interp: thay recursion `eval`/`exec` bằng explicit
   work stack trong `Interp` (hiện `crates/vis-interp/src/interp.rs` + `interp/goi.rs` dùng Rust
   call stack qua `eval`/`call_hfn`). VM: tách `Await` thành opcode thật với `pc` resume;
   `Frame` hiện thực (`vm.rs:124-135`) đã có `pc/locals/captured/st_cu/writeback` nhưng chưa có
   điểm suspend giữa instruction — cần map `Pending` + resume value vào `Vm`.
2. **Sửa `goi_than` path** (`crates/vis-interp/src/host.rs:26`, `builtin.rs:205`,
   `vm.rs:526`) để trả `Pending` thay vì chạy run-to-completion (`rt.rs:157-230` `chay_mot`/
   `await_task` hiện đặt `DangChay` rồi gọi `goi_than` tới xong). Đây là integration gap cốt lõi:
   prototype **chưa** chạm đường này.
3. **Hợp đồng event trace** (VD-250 §2.1): fixture replay sự kiện giả ⇒ byte-identical giữa hai
   engine; tách test mạng thật (đúng đắn/tiến triển) khỏi test tất định.
4. **Lazy spawn + final join** (§2.2/§2.3): `spawn` không chạy thân; join theo thứ tự tạo nhưng
   không ép thứ tự hoàn thành; không huỳnh task lúc exit.
5. **Mọi vị trí `await` checker chấp nhận**, nhiều frame continuation (await trong hàm được gọi
   từ hàm khác), và `Err` đi qua `wrap_task_error`/`VS0506` thật.
6. Đo budget/hạn ngạch và ownership (§4/§5) trước khi đưa mạng thật vào scheduler.

**Kết luận**: 6 tiêu chí Lát 0 đã được **chứng minh ở mức model** (10/10 test, `vis-dev check`
xanh). Chưa được chứng minh ở mức tích hợp. Theo VD-250 §3, phải trình checkpoint A0 trước khi
mở Lát 1 — không tự động triển khai 0→5.
