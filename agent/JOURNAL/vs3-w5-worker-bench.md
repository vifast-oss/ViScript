<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b gate B7 — CPU-only + payload matrix (`WorkerPool` opt-in)

**Trạng thái**: **đo + đánh giá gate**, không nối evaluator, không bật default.
**Baseline**: `ec29204` (đã `git merge main`).
**Mã đo**: `crates/vis-cli/examples/vs3_worker_bench.rs` (**example chạy tay**, không nằm
trong test suite); log: `agent/JOURNAL/vs3-w5-worker-bench.log`.
**Máy**: i5-1235U, **2 P-core + 8 E-core**, `nproc=12`, Linux. Clean runs có
`load_start 2,23–2,85`; log đại diện `load_start 2,23`. Các lần đầu khi máy tải
`5,8–15,9` bị nhiễu, không dùng làm median.

---

## 1. Cách đo

- `WorkerPool` opt-in hiện tại: N worker, `SendValue`, budget `SharedBudget` atomic +
  chunk; W=1 tuần tự/shared env; W>1 thread/snapshot.
- 64 task/lần; CPU ~**229 µs/task** (calibrate sau warm 400 ms); mỗi task **100 charge**;
  chunk **256**; mỗi ô **5 reps nội bộ**, lấy median; matrix lặp **3 lần sạch** example
  (`load_start 2,23–2,85`), báo median 3 lần + range.
- **Payload copy trên producer** (biên spawn): mỗi task `payload.clone()` trước `spawn`
  — mô hình xấu nhất của `spawn f(biến)`. Có thêm probe `producer` / `worker-copy` /
  `move` cho `list10k`/`map1k` để tách chi phí copy.
- Payload: `unit`, `list1k`, `list10k`, `map1k`, `record` 16 trường + list1k,
  `enum(Result.Ok(list1k))`, `str1MiB`.
- Kiểm kèm: `used + remaining == limit` mỗi lần; print merge W=1 vs W=4;
  cross-write W=1 allowed / W=4 refused.

## 2. Kết quả (median 3 lần sạch; ms)

| Payload | W=1 | W=2 | W=4 | Speedup W=4 | Đánh giá B7 |
|---|---:|---:|---:|---:|---|
| **unit (CPU-only)** | **11,87** | **8,89** | **4,58** | **2,59×** | **PASS** ngưỡng 2,4 |
| list1k | 15,68 | 9,26 | 4,95 | **3,13×** | PASS |
| list10k | 41,63 | 19,90 | 19,46 | **2,22×** | FAIL |
| map1k | 17,72 | 16,38 | 18,57 | **0,95×** | FAIL |
| record 16 + list1k | 14,21 | 9,28 | 4,69 | **3,03×** | PASS |
| enum(list1k) | 14,51 | 9,30 | 4,68 | **3,11×** | PASS |
| str1MiB | 32,64 | 10,71 | 7,11 | **4,84×** | PASS |

Range W=4 (3 lần): unit 4,55–4,62; list1k 4,85–5,00; list10k 18,74–19,72;
map1k 18,50–19,28; record 4,68–4,74; enum 4,66–4,93; str 6,62–7,20.

`used=6.400`, `remaining=10.000`, `limit=16.400` ở **mọi** ô; `print_merge` W=1==W=4
**124 byte**; `cross_write` W=1 allowed / W=4 refused **OK**.

### 2.1 Clone cost (producer, median 20 lần)

| Payload | clone | logical bytes |
|---|---:|---:|
| unit | 20 ns | 0 |
| list1k | 25 µs | 80.000 |
| list10k | 265 µs | 800.000 |
| map1k | 38 µs | 155.890 |
| record | 26 µs | 81.694 |
| enum | 26 µs | 80.072 |
| str1MiB | 35 µs | 1.048.576 |

64 task ⇒ copy producer **~1,6 ms** (list1k) / **17,0 ms** (list10k) / **2,4 ms** (map) /
**2,2 ms** (str). Đây là chi phí **tuần tự trên main** trước/trong khi dispatch.

## 3. Probe producer-copy / worker-copy / move (3 reps)

| Payload | mode | W=1 | W=4 | Speedup |
|---|---|---:|---:|---:|
| list10k | producer | 42,30 | 19,33 | 2,19× |
| list10k | worker-copy | 31,97 | 10,56 | 3,03× |
| list10k | move (prebuild) | 16,85 | 9,80 | 1,72× |
| map1k | producer | 19,05 | 19,36 | 1,02× |
| map1k | worker-copy | 17,64 | 6,10 | 2,89× |
| map1k | move (prebuild) | 14,47 | 7,31 | 1,98× |

Đọc số:
- **Producer-copy là nút cổ chai tuần tự**: list10k/map fail B7 vì copy trên main, không
  phải worker pool không song song.
- **Worker-copy** (copy trong worker) đạt **2,9–3,0×** cho cả hai: copy song song được.
- **Move** (payload dựng sẵn, move ownership) giảm tuyệt đối mạnh (list10k 42,3→16,9;
  map 19,1→14,5) nhưng speedup W=4 của map chỉ **1,98×** — còn **drop/free + allocator**
  của map 1k string trên worker/main, chưa song song hoá.
- Vì vậy kết luận gate phải tách: CPU-only + payload nhỏ đạt; payload lớn **chưa đạt
  nếu spawn copy trên producer**; move/worker-copy là hướng cần chốt.

## 4. Chunk reservation

Charges-only, W=4, 64 task × 2000 charge:

| chunk | ms | ns/charge |
|---:|---:|---:|
| 1 | 17,95 | **140,3** |
| 16 | 4,91 | 38,4 |
| **256** | **4,59** | **35,9** |
| 4096 | 5,10 | 39,9 |

Chunk 1 tranh chấp CAS rất đắt; **256–4096** tương đương, **256** là mặc định đề xuất
(đủ giảm contention, ít giữ budget dư). `SharedBudget::new(limit, chunk)` hiện do caller
chọn; W1 nên dùng 256 (hoặc hằng dùng chung) khi nối.

## 5. Khuyến nghị

1. **Gate cứng B7**: CPU-only W=4 **2,59× ≥ 2,4** → PASS trên máy này (caveat hybrid).
2. **Payload matrix**: báo cáo theo từng payload, không gộp một ngưỡng:
   - list1k / record / enum / str1MiB đạt (3,03–4,84×).
   - list10k (2,22×) và map1k (0,95×) **chưa đạt** trong mô hình copy producer.
3. Trước khi tuyên bố “payload lớn song song”, W1 cần chốt một trong:
   - **local-first + move** cho `spawn` khi giá trị không còn dùng (không copy producer);
   - **worker-copy** có kiểm soát (copy trong worker, giới hạn bộ nhớ);
   - hoặc công bố rõ “payload lớn không đạt B7, dùng nhiều task nhỏ/chunk dữ liệu”.
4. Map 1k cần điều tra **allocator/drop/free** (worker-copy đạt 2,89× nhưng move 1,98×, producer 1,02×).
   Có thể do 64 lần drop 1k string; cân nhắc per-thread allocator, cap, hoặc move.
5. Chunk **256** cho `SharedBudget`; không cần sửa `frame/fee.rs` (caller-controlled).
6. Benchmark phải chạy khi máy tải thấp (clean runs load 2,2–2,9); ghi topology/load.

## 6. Blocker

- **Chưa nối evaluator**: WorkerPool nhận closure + `SendValue`, không chạy `async fn` thật;
  gate B7 mới là gate cơ chế, chưa phải nghiệm thu runtime (W1 1d/3b).
- **Copy semantics**: chưa chốt `spawn` copy hay move; producer-copy matrix là worst case.
  W1 sở hữu `interp.rs`/`vm.rs`/`frame/**`, cần nối `spawn`/`await` thật.
- **Map allocator**: W=4 chậm hơn W=1 khi copy producer; worker-copy đạt nhưng move không;
  cần điều tra trước khi nhận payload map.
- **Hybrid CPU**: 2P+8E, W=4 threshold 2,4 sát; load thấp mới ổn định. Không dùng số cũ
  load 5,8–15,9 làm median.
- **RSS**: chưa đo trong gate này (B7 nói chỉ tham khảo); nếu cần cap bộ nhớ, đo riêng.
- **Không** kết luận worker pool production; default vẫn single-thread.

## 7. Tái lập

```bash
CARGO_TARGET_DIR=/home/ps/ViScript/target/vs3-w5 \
  cargo run -p vis-cli --example vs3_worker_bench
```

Log đại diện: `agent/JOURNAL/vs3-w5-worker-bench.log` (example run, load 2,23).
