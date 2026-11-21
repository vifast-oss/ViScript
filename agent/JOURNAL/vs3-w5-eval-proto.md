<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — prototype evaluator độc lập trên worker thread (test/example-only)

**Trạng thái**: **PROTOTYPE PASS** — chỉ thêm example + journal; **không** sửa
production/ABI/default, không push/merge/tag.
**Nhánh**: `vs3/w5-spike`, đã `git merge main` → **`bc7a543`** (merge plan evaluator 850 dòng;
tag `v0.3.3` ở `f75b246`).
**Mã đo**: `crates/vis-cli/examples/vs3_worker_eval_proto.rs` (437 dòng, example chạy tay —
không nằm trong test suite, không thêm `#[ignore]`/skipped vào gate).
**Log**: `agent/JOURNAL/vs3-w5-eval-proto.log`.
**Timebox**: 2 h. `df /home` lúc bắt đầu **11G** trống, lúc kết thúc **11G** (>5G).
**Máy**: i5-1235U (2 P-core + 8 E-core), `nproc=12`, Linux; load đầu mỗi lần chạy **8–11**
(ghi trong log) — số wall chỉ tham khảo, không dùng làm ngưỡng.

---

## 1. Kết luận ngắn

1. **Không có blocker `Send + Sync`.** Ràng buộc biên dịch
   `fn khang_dinh_send_sync<T: Send + Sync>()` qua được cho `Hir`, `Symbols`, `TypeTable`,
   `vis_bytecode::Module`, `SendValue` và ảnh tổng `Anh` (gồm `Arc<Module>`). Không `unsafe`,
   không sửa kiểu nào. Vậy ảnh chương trình **chia sẻ được** qua worker, đúng giả định P1/P2
   của [`vs3-w5-evaluator-plan.md`](vs3-w5-evaluator-plan.md) §2.4/§11-R1.
2. **W=1 và W=4 evaluator độc lập cho output/result byte-identical** trên chương trình mẫu
   (in + `var`/`let` + `while` + số học), chạy 3 vòng, cả đường thread OS trực tiếp lẫn
   `WorkerPool`. Không rò thread/fd sau khi join.
3. **Ghép được `WorkerPool`** cho budget atomic và cross-write: W=1 ghi env chung, W=4 trả
   `CrossWrite`; `used + remaining == limit` ở cả W=1/W=4, `used` đúng số charge.
4. **Chưa nối evaluator vào pool như production** (ngoài phạm vi time-box): mỗi job vẫn dựng
   `Interp`/`Vm` mới rồi chạy cả chương trình; chưa có `spawn`/`await` ngôn ngữ, snapshot
   top-level, dirty hook trên `HVar`/`HAssign`, cancel/budget trong thân task. Đây là việc P1/P2
   của plan, không phải kết quả của prototype này.

---

## 2. API hiện có (đọc mã, không suy đoán)

| Thành phần | Đường dẫn công khai | Ghi chú cho P1/P2 |
|---|---|---|
| HIR | `vis_types::Hir`, `Symbols`, `TypeTable` | Thuần dữ liệu; `Send + Sync` (đã chứng minh). |
| Lower/check | `vis_types::lower_with`, `vis_types::check` | Example dùng đúng pipeline của test `vs3_default.rs`. |
| Bytecode | `vis_bytecode::compile(&hir, &symbols, text) -> Module` | `Module: Clone + Send + Sync`; example bọc `Arc<Module>`. |
| Interp | `vis_interp::run(text, &hir, &syms, &tys, &args) -> RunOutput` | `vis_interp::interp::Interp::new` là `pub(crate)`; bên ngoài **mỗi lần gọi `run` là một `Interp` mới**. P1 cần API/runner trong crate (như plan §6.2 `interp/worker.rs`). |
| VM | `vis_vm::vm::Vm::new(&Module, &str)`, `Vm::run(&[String])` | `pub`; dựng VM riêng trên cùng `Arc<Module>` — đúng mô hình “ảnh bất biến, evaluator theo worker”. |
| Worker | `vis_interp::value::worker::{WorkerPool, WorkerCtx, SendValue, WorkerError}` | `WorkerPool::{new, with_budget, spawn, spawn_shared, join}`; `join` merge output theo **spawn order**, trả `CrossWrite` nếu W>1 có dirty. |

Không đổi `Value`, `Task`, `VERSION`, cache key, `Show`, `VBC1` — example chỉ đọc API.

---

## 3. Prototype làm gì

Chương trình mẫu (`SRC` trong example):

```text
fn work(n: int) -> int:
    var acc = 0
    var i = 0
    while i < n:
        acc = acc + (i * 3) % 7
        i = i + 1
    acc

fn main():
    var total = 0
    var k = 1
    while k <= 8:
        total = total + work(k * 500)
        k = k + 1
    let nhan = 2
    print("tong")
    print(total)
    print(total * nhan - 7)
```

Oracle một luồng: `stdout = "tong\n53993\n107979\n"`, `exit=0`,
`steps_hir=270_218`, `steps_vm=306_251` (cả hai engine, đều > 0 — chống xanh-giả).
`Hir` có 2 item, `Module` có 3 func / 66 lệnh.

Ba lớp kiểm:

1. **`assert_send_sync`**: 6 ràng buộc biên dịch (mục 1). Nếu fail ⇒ blocker + dừng, nhưng
   **không** fail.
2. **Thread OS độc lập**: `std::thread::scope` 4 thread, mỗi thread giữ `&Anh` (Sync) và tự gọi
   `vis_interp::run` (Interp mới) + `Vm::new` (VM mới) trên cùng ảnh; so từng thread với oracle.
3. **`WorkerPool` W=1 vs W=4**: 4 job, mỗi job `Arc<Anh>` → chạy **cả hai engine**, in stdout
   vào buffer của task (`ctx.print`), `ctx.charge(1_000)`, trả `SendValue::Record{exit,
   steps_hir, steps_vm}`. `join` merge output theo spawn order. So:
   - output W=1 = W=4 = oracle × 4 **byte-identical** (72 byte);
   - `results` (record) W=1 = W=4;
   - 3 vòng lặp, lấy **median** wall;
   - budget `used=4_000`, `remaining=96_000`, `limit=100_000`, `used+remaining==limit`,
     `exhausted=false` ở cả W=1/W=4;
   - cross-write: pool root `counter=0`; W=1 `ctx.set` ghi env chung (thấy `counter=3`),
     W=4 trả `CrossWrite { vars: ["counter"], tasks: 4 }`.

Đo thêm: số thread `/proc/self/task`, fd `/proc/self/fd`, `VmRSS`/`VmHWM`
`/proc/self/status`; kiểm `sau <= trước` sau khi mọi pool/scope join.

---

## 4. Số đo (3 lần chạy sạch, log đầy đủ ở `.log`)

| Lần | load đầu | W=1 wall (median 3 vòng) | W=4 wall (median 3 vòng) | speedup | RSS sau / HWM |
|---|---|---:|---:|---:|---:|
| 1 | 9,10 | 1.315,4 ms | 337,4 ms | 3,90× | 9.668 / 10.616 kB |
| 2 | 8,69 | 933,9 ms | 340,2 ms | 2,74× | 9.660 / 10.660 kB |
| 3 | 7,97 | 1.278,7 ms | 288,6 ms | 4,43× | 9.580 / 10.460 kB |

- **Byte-identical**: 3/3 lần, output 72 byte, `results` 4 record khớp W=1/W=4.
- **Thread**: trước=1, sau=1 (`ro=0`) ở cả 3 lần; sau từng pool cũng 1.
- **fd**: trước=4, sau=4 (`ro=0`).
- **Budget**: W=1 và W=4 đều `used=4.000`, `remaining=96.000`, `limit=100.000`.
- **Cross-write**: W=1 `shared-env counter=3`; W=4 `CrossWrite vars=["counter"] tasks=4`.
- **Wall tổng example** (3 vòng pool + thread + cross): 4,2–5,7 s (debug build).
- Speedup **không phải ngưỡng gate**: máy hybrid + load 8–11 nên chỉ báo cáo, không assert.
  Điều được assert là **độc lập/byte-identical/không rò**, không phải thời gian.

---

## 5. Blocker và đề xuất

### 5.1 Blocker

**Không có blocker `Send + Sync`.** `Hir`/`Module`/`SendValue` đều qua ràng buộc biên dịch,
nên phương án B của plan (worker tự parse/lower từ `Arc<str>`) **không cần** cho ảnh chương
trình. Các blocker còn lại vẫn là **quyết định** của A0, không phải lỗi kỹ thuật của spike:

- mã lỗi cho cross-write W>1 (`VS0509`?) — plan §7/Q1;
- chính sách W>1 bỏ frame/`chan(0)`/host — plan §11 (a)/(b);
- cần API runner trong crate: `Interp::new` đang `pub(crate)`, nên P1 phải thêm
  `interp/worker.rs`/`run_workers_*` ẩn để worker dựng `Interp` từ ảnh mà không lộ type nội bộ.

### 5.2 Đề xuất cho P1/P2 (không code trong phiên này)

1. `Arc<HirImage>` và `Arc<Module>` là **khả thi đã chứng minh**; giữ một ảnh/lần chạy như plan
   D3, không clone HIR/Module theo job.
2. **Không** gửi `Interp`/`Vm` qua biên (chúng mượn ảnh + `Rc`/`RefCell`); mỗi job dựng
   evaluator mới, đúng như prototype. Chi phí dựng evaluator/job chưa tách khỏi wall — P2 phải
   đo riêng “empty task” trước khi tối ưu (plan R2).
3. Prototype chưa chứng minh snapshot env/`spawn`/`await` ngôn ngữ; đó là P1/P2 thật. Kết quả
   này chỉ đóng câu hỏi **ảnh chương trình có gửi được không** và **output merge có tất định
   không** ở mức pool + cả hai engine.
4. `CrossWrite` của `WorkerCtx` là mức **pool**, không phải hook ngôn ngữ: P2 vẫn phải đánh
   dấu dirty ở mọi đường ghi top-level (`Var`/`Assign`/`Field`/`Index`) như plan D4/R5.

---

## 6. Lệnh đã chạy

```bash
# toolchain hợp đồng (PATH mặc định là rustc 1.93.1, không đạt rust-version 1.100.0)
~/.cargo/bin/cargo check -p vis-cli --example vs3_worker_eval_proto
~/.cargo/bin/cargo run -q -p vis-cli --example vs3_worker_eval_proto   # ×3
~/.cargo/bin/cargo check --all-targets                                # PASS, ~11 s
~/.cargo/bin/cargo fmt --all -- --check                                # PASS
~/.cargo/bin/cargo clippy -p vis-cli --all-targets -- -D warnings      # PASS
~/.cargo/bin/cargo test -q -p vis-cli --test vs3_worker -- --test-threads=1
```

- `cargo check --all-targets`: **PASS** (không warning ở example; warning dead field đã sửa
  bằng cách dùng `threads_sau` để kiểm rò thread từng pool).
- `cargo fmt --all -- --check`: **PASS**.
- `cargo clippy -p vis-cli --all-targets -- -D warnings`: **PASS**.
- `vs3_worker`: **14 passed, 0 failed** (16,59 s, `--test-threads=1`).
- R1: tệp mới **437 dòng** ≤ 1.500; không đụng `interp.rs`/`vm.rs`/`value.rs`/`frame/**`/`net.rs`.
- Không GitHub Actions, không push/merge/tag.
- `df /home`: **11G** trống trước và sau (ngưỡng dừng <5G không chạm).
