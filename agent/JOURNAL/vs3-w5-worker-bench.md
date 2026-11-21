<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b gate B7 — CPU-only + payload matrix (`WorkerPool` opt-in)

**Trạng thái**: **đo + đánh giá gate**, không nối evaluator, không bật default.
**Baseline**: `a26ec4b` (đã `git merge main`).
**Mã đo**: `crates/vis-cli/examples/vs3_worker_bench.rs` (**example chạy tay**, không nằm
trong test suite); log: `agent/JOURNAL/vs3-w5-worker-bench.log`.
**Máy**: i5-1235U, **2 P-core + 8 E-core**, `nproc=12`, Linux. Median lấy từ **3 lần sạch
gần nhất** (`load_start 8,16–8,71`); log đại diện `load_start 8,71`. Các lần đầu load
`15–28` bị nhiễu, không dùng làm median.

---

## 1. Thay đổi API (W5 sở hữu `rt/worker.rs`)

Thêm 2 lối gửi payload lớn, không đổi `spawn` cũ:

| API | Ngữ nghĩa |
|---|---|
| `spawn_with_payload(payload: SendValue, f)` | **move** payload vào task; không copy producer. Dùng khi caller không còn cần giá trị. |
| `spawn_worker_copy(payload: Arc<SendValue>, f)` | producer chỉ clone `Arc` O(1); **worker tự clone** payload vào heap khi chạy. `Arc` chỉ là template bất biến, **không** shared mutable heap. |

Cả hai dùng cùng `WorkerCtx`/completion/cross-write/budget; W=1 chạy tuần tự, W>1 thread.
`vs3_worker` thêm ca `worker_copy_va_move_payload_w1_w4_parity` (9 ca tổng): W=1==W=4 output,
budget `used+remaining==limit`, move/worker-copy cùng hành vi.

## 2. Cách đo

- 64 task; CPU ~**229 µs/task**; 100 charge/task; chunk **256**; mỗi ô **5 reps nội bộ**;
  matrix lặp 3 lần sạch, lấy median.
- **Before — producer-copy**: mỗi task `payload.clone()` trước `spawn` (worst case
  `spawn f(biến)`), như report trước.
- **After — worker-copy**: `spawn_worker_copy`; producer clone `Arc` O(1), worker clone payload.
- Payload: `unit`, `list1k`, `list10k`, `map1k`, `record` 16 trường + list1k,
  `enum(Result.Ok(list1k))`, `str1MiB`.
- Kiểm: `used+remaining==limit`; print merge W=1 vs W=4; cross-write W=1 allowed/W=4 refused.
- Clone cost producer (median 20): list1k 34 µs, list10k 338 µs, map1k 50 µs, record 35 µs,
  enum 35 µs, str1MiB 46 µs.

## 3. Trước / sau

| Payload | Before W=4 (producer) | After W=4 (worker-copy) | After W=1 | After speedup |
|---|---:|---:|---:|---:|
| **unit (CPU-only)** | 4,90 ms — **2,85×** | **5,09 ms** | 15,16 | **2,93×** |
| list1k | 5,23 — 3,51× | 5,98 | 17,30 | **3,13×** |
| **list10k** | 28,18 — **2,02×** | **12,95** | 40,99 | **3,18×** |
| **map1k** | 27,09 — **0,90×** | **6,84** | 21,50 | **3,14×** |
| record | 5,36 — 3,63× | 5,85 | 17,72 | **3,03×** |
| enum | 5,31 — 3,53× | 5,85 | 17,38 | **3,01×** |
| str1MiB | 9,91 — 4,28× | 7,73 | 18,49 | **2,39×** |

- **Mục tiêu list10k/map1k ≥ 2,4×: ĐẠT** với worker-copy (3,18× / 3,14×), trong khi
  producer-copy chỉ 2,02× / 0,90×.
- CPU-only worker-copy **2,93×** — PASS ngưỡng 2,4 (caveat hybrid).
- list1k/record/enum pass (3,0–3,1×); str1MiB **2,39×** sát dưới 2,4 (producer-copy 4,28×
  do copy nhỏ 46 µs/task và pipeline; worker-copy thêm clone trong worker, không cần thiết cho
  payload này).
- `used=6.400`, `remaining=10.000`, `limit=16.400` mọi ô; print merge **124 byte**;
  cross-write W=1 allowed / W=4 refused.

Range W=4 worker-copy 3 lần: unit 5,01–5,47; list1k 5,53–6,43; list10k 12,45–13,17;
map1k 6,77–6,95; record 5,82–5,92; enum 5,53–5,92; str 7,20–7,74.

## 4. Chunk reservation

Charges-only, W=4, 64 task × 2000 charge:

| chunk | ms | ns/charge |
|---:|---:|---:|
| 1 | 24,69 | **192,9** |
| 16 | 6,69 | 52,3 |
| **256** | **6,17** | **48,2** |
| 4096 | 6,16 | 48,1 |

Chunk 1 tranh chấp CAS rất đắt; **256–4096** tương đương, **256** là mặc định đề xuất.
`SharedBudget::new(limit, chunk)` do caller chọn; W1 nên dùng 256 khi nối.

## 5. Khuyến nghị

1. **Dùng `spawn_worker_copy` cho payload lớn** (list10k/map1k) khi caller còn giữ template:
   producer O(1), clone song song trong worker; mục tiêu B7 đạt.
2. **Dùng `spawn_with_payload` (move)** khi giá trị không còn dùng; tránh copy hoàn toàn.
3. Giữ producer-copy cho payload nhỏ/str1MiB nếu đo thấy nhanh hơn (str producer 4,28× vs
   worker-copy 2,39×); chọn theo kích thước/ownership, không một chế độ cho mọi payload.
4. Chunk **256**; không cần sửa `frame/fee.rs`.
5. Gate cứng B7 dùng **CPU-only** (2,93× PASS); payload matrix báo từng payload, không gộp.
6. Chạy benchmark khi tải thấp; ghi topology/load.

## 6. Blocker

- **Chưa nối evaluator**: WorkerPool nhận closure + `SendValue`, chưa chạy `async fn` thật;
  W1 1d/3b phải map `spawn`/`await` sang `spawn_worker_copy`/`spawn_with_payload` theo kiểu
  payload.
- **`Arc<SendValue>` template**: chỉ bất biến; không shared mutable heap. Nếu payload chứa
  handle/closure vẫn phải `VS0508`; W1 kiểm ở checker/spawn.
- **Map allocator**: worker-copy đạt 3,14×; move/producer vẫn kém (map1k producer 0,90×);
  nếu W1 chọn move cho map, cần đo lại drop/free/allocator.
- **str1MiB**: worker-copy 2,39× sát dưới 2,4; chọn producer-copy hoặc leaf-Arc theo workload.
- **Load/hybrid**: 2P+8E, kết quả phụ thuộc load; chỉ dùng median 3 lần load 8,2–8,7.
- **RSS** chưa đo trong gate này; B7 chỉ tham khảo. Không tuyên bố production/default.

## 7. Tái lập

```bash
CARGO_TARGET_DIR=/home/ps/ViScript/target/vs3-w5 \
  cargo run -p vis-cli --example vs3_worker_bench
```

Log đại diện: `agent/JOURNAL/vs3-w5-worker-bench.log` (load 8,71).
