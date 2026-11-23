<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — `spawn_shared(Arc<SendValue>, read-only)`

**Trạng thái**: thêm API opt-in/test-only, **không bật default**. Không nối evaluator.
**Baseline**: `a0ff400`; `git merge main` fast-forward trên `vs3/w5-spike`; **không** push/merge.
**Phạm vi sửa**: `crates/vis-interp/src/rt/worker.rs` (chỉ thêm `spawn_shared`),
`crates/vis-cli/tests/vs3_worker.rs`, `crates/vis-cli/examples/vs3_worker_bench.rs`,
journal/log. Không đụng `interp.rs`/`vm.rs`/`frame/**`/`net.rs`/`pkg.rs`.

## 1. API

```rust
pub fn spawn_shared<F>(&mut self, payload: Arc<SendValue>, f: F) -> Result<TaskHandle, WorkerError>
where
    F: FnOnce(&mut WorkerCtx, &SendValue) -> Result<SendValue, String> + Send + 'static
{
    self.spawn(move |ctx| f(ctx, &payload))
}
```

- Task giữ `Arc<SendValue>`; producer clone **Arc O(1)**, worker đọc trực tiếp — **không deep clone**.
- Closure chỉ nhận `&SendValue`: **không sửa được payload** (read-only ở mức kiểu).
- Cần sở hữu/sửa payload: dùng `spawn_worker_copy`/`spawn_with_payload` như cũ.
- Delegate `spawn` ⇒ W=1/W>1, print merge, cross-write, budget, cancel giữ nguyên.
- `spawn`/`spawn_with_payload`/`spawn_worker_copy` **không đổi**.

## 2. Test

`spawn_shared_map_list_w1_w4_parity` — map1k và list10k, mode **shared/worker-copy/producer**,
W=1/W=4, 16 task/mode:

- **Print merge**: output byte-identical giữa 3 mode × W=1/W=4; 16 dòng.
- **Budget**: `used=32`, `used+remaining==limit`, không exhausted.
- **Read-only / Arc**: sau join `Arc::strong_count(payload)==1`; nội dung map1k/list10k
  không đổi (1000/10.000 phần tử).
- **Cross-write**: W=1 `ctx.set("counter")` được merge env; W=4 `spawn_shared` vẫn trả
  `WorkerError::CrossWrite { vars:["counter"], tasks:8 }`; Arc count về 1.
- `cargo test -p vis-cli --test vs3_worker`: **13 passed / 0 failed / 0 ignored**
  (debug 14,02 s; release 10,63 s).
- `cargo test --release` cũng **13 passed / 0 ignored**.

## 3. Benchmark release (W4, median 3 lần)

`vs3_worker_bench` thêm `MATRIX_SHARED` và mode `shared` trong `PROBE`; cùng 64 task,
CPU ~200 µs/task, 100 charge/task, chunk 256. Load 1m lúc chạy **15–17,5**, calibration
**200–236 µs/task**.

| Payload | Producer (ms) | Worker-copy (ms) | **Shared (ms)** | Shared vs producer |
|---|---:|---:|---:|---:|
| unit (CPU-only) | 4,83 | 4,24 | 5,80 | 0,83× |
| list1k | 6,96 | 4,62 | 5,24 | 1,33× |
| **list10k** | 16,19 | 7,19 | **5,25** | **3,08×** |
| **map1k** | 24,63 | 5,81 | **5,62** | **4,38×** |
| record | 6,36 | 7,44 | 7,32 | 0,87× |
| enum | 7,24 | 9,86 | **5,37** | 1,35× |
| str1MiB | 18,91 | 12,88 | **5,31** | **3,56×** |

- Với payload lớn, shared **nhanh nhất**: map1k 5,62 ms (worker 5,81, producer 24,63);
  list10k 5,25 ms (worker 7,19, producer 16,19); str1MiB 5,31 ms (worker 12,88).
- Payload nhỏ/CPU-only: shared chậm hơn worker/producer (Arc + closure), không đáng chọn.
- `PROBE` W4 (median 3 lần):

| Payload | Producer | Move | Worker-copy | **Shared** |
|---|---:|---:|---:|---:|
| list10k | 17,44 | 11,16 | 8,81 | **5,37** |
| map1k | 26,31 | 13,58 | 7,36 | **5,52** |

- 3/3 lần: `print_merge 124 byte`, cross-write, `used+remaining==limit` OK; mọi dòng
  `MATRIX_SHARED` có `used=6.400/remaining=10.000/limit=16.400`.
- Speedup W4/W1 của shared thấp hơn (1,9–2,8×) vì W1 shared đã bỏ clone và load cao;
  **so sánh đúng là absolute ms cùng run** — shared thắng ở payload lớn.

## 4. Đọc kết quả

- `spawn_shared` giải đúng blocker map alloc: map1k producer 24,63 ms → shared **5,62 ms**
  (map key String không bị clone/free 1000 lần).
- Với list10k/str1MiB cũng bỏ được clone lớn; shared nhanh hơn worker-copy ~1,3–2,4×.
- Với map nhỏ/unit/record, overhead Arc/closure làm shared chậm hơn; chọn theo kích thước.
- Read-only là ràng buộc thật: task chỉ có `&SendValue`; muốn sửa phải worker-copy/move/COW.

## 5. Blocker / giới hạn

- **Chỉ read-only**: task không được sửa payload; nếu cần mutation phải copy-on-write hoặc
  worker-copy (deep clone).
- **Arc payload phải Send+Sync**: `SendValue` hiện thoả; evaluator phải chặn closure/handle/
  Task/cap bằng `VS0508` trước khi bọc Arc.
- **Chưa nối evaluator**: API mới chỉ test/example; W1 phải quyết nơi dùng shared (capture
  bất biến) vs worker-copy.
- **Load cao** 15–17,5 khi benchmark; calibration 200–236 µs; absolute ms phụ thuộc tải,
  speedup/shared-vs-mode trong cùng run đáng tin hơn.
- **Không đo RSS/threads riêng** cho shared trong vòng này; cơ chế `spawn`/completion không đổi.
- Shared không phù hợp payload nhỏ/CPU-only (chậm hơn); không bật default.
- Không tuyên bố production; không đổi `Value`/bytecode/cache.

## 6. Tái lập

```bash
cargo test -p vis-cli --test vs3_worker spawn_shared
CARGO_TARGET_DIR=/home/ps/ViScript/target/vs3-w5-release \
  cargo run --release -p vis-cli --example vs3_worker_bench
```

Log: [`vs3-w5-spawn-shared.log`](vs3-w5-spawn-shared.log) (test release + 3 lần benchmark).
