<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b gate B7 — CPU-only + payload matrix (`WorkerPool` opt-in)

**Trạng thái**: **đo + đánh giá gate**, không nối evaluator, không bật default.
**Baseline**: `ec29204` (đã `git merge main`).
**Mã đo**: `crates/vis-cli/tests/vs3_worker_bench.rs` (`#[ignore]`, chạy tay); log:
`agent/JOURNAL/vs3-w5-worker-bench.log`.
**Máy**: i5-1235U, **2 P-core + 8 E-core**, `nproc=12`, Linux. Clean runs có
`load_start 2,13–2,34`; log đại diện `load_start 2,34`. Các lần đầu khi máy tải
`5,8–15,9` bị nhiễu, không dùng làm median.

---

## 1. Cách đo

- `WorkerPool` opt-in hiện tại: N worker, `SendValue`, budget `SharedBudget` atomic +
  chunk; W=1 tuần tự/shared env; W>1 thread/snapshot.
- 64 task/lần; CPU ~**228 µs/task** (calibrate sau warm 400 ms); mỗi task **100 charge**;
  chunk **256**; mỗi ô **5 reps nội bộ**, lấy median; matrix lặp **3 lần sạch** (run 9–11),
  báo median 3 lần + range.
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
| **unit (CPU-only)** | **12,34** | **8,61** | **4,59** | **2,69×** | **PASS** ngưỡng 2,4 |
| list1k | 17,09 | 9,38 | 4,85 | **3,54×** | PASS |
| list10k | 56,20 | 28,88 | 29,38 | **1,93×** | FAIL |
| map1k | 23,35 | 24,25 | 30,01 | **0,77×** | FAIL |
| record 16 + list1k | 16,85 | 9,72 | 5,09 | **3,45×** | PASS |
| enum(list1k) | 17,18 | 10,11 | 5,06 | **3,38×** | PASS |
| str1MiB | 41,98 | 11,74 | 9,06 | **4,63×** | PASS |

Range W=4 (3 lần): unit 4,42–4,62; list1k 4,71–4,89; list10k 29,15–30,32;
map1k 29,54–30,88; record 4,88–5,21; enum 4,86–5,19; str 8,40–9,28.

`used=6.400`, `remaining=10.000`, `limit=16.400` ở **mọi** ô; `print_merge` W=1==W=4
**124 byte**; `cross_write` W=1 allowed / W=4 refused **OK**.

### 2.1 Clone cost (producer, median 20 lần)

| Payload | clone | logical bytes |
|---|---:|---:|
| unit | 20 ns | 0 |
| list1k | 35 µs | 80.000 |
| list10k | 351 µs | 800.000 |
| map1k | 66 µs | 155.890 |
| record | 34 µs | 81.694 |
| enum | 34 µs | 80.072 |
| str1MiB | 47 µs | 1.048.576 |

64 task ⇒ copy producer **~2,2 ms** (list1k) / **22,5 ms** (list10k) / **4,2 ms** (map) /
**3,0 ms** (str). Đây là chi phí **tuần tự trên main** trước/trong khi dispatch.

## 3. Probe producer-copy / worker-copy / move (3 reps)

| Payload | mode | W=1 | W=4 | Speedup |
|---|---|---:|---:|---:|
| list10k | producer | 57,74 | 27,87 | 1,93× |
| list10k | worker-copy | 42,72 | 12,48 | 3,42× |
| list10k | move (prebuild) | 18,62 | 9,95 | 1,87× |
| map1k | producer | 23,04 | 30,01 | 0,77× |
| map1k | worker-copy | 21,28 | 6,83 | 3,12× |
| map1k | move (prebuild) | 16,40 | 11,64 | 1,41× |

Đọc số:
- **Producer-copy là nút cổ chai tuần tự**: list10k/map fail B7 vì copy trên main, không
  phải worker pool không song song.
- **Worker-copy** (copy trong worker) đạt 3,1–3,4× cho cả hai: copy song song được.
- **Move** (payload dựng sẵn, move ownership) giảm tuyệt đối mạnh (list10k 57,7→18,6;
  map 23,0→16,4) nhưng speedup W=4 của map chỉ 1,41× — còn **drop/free + allocator**
  của map 1k string trên worker/main, chưa song song hoá.
- Vì vậy kết luận gate phải tách: CPU-only + payload nhỏ đạt; payload lớn **chưa đạt
  nếu spawn copy trên producer**; move/worker-copy là hướng cần chốt.

## 4. Chunk reservation

Charges-only, W=4, 64 task × 2000 charge:

| chunk | ms | ns/charge |
|---:|---:|---:|
| 1 | 25,43 | **198,7** |
| 16 | 7,86 | 61,4 |
| **256** | **7,31** | **57,1** |
| 4096 | 7,09 | 55,4 |

Chunk 1 tranh chấp CAS rất đắt; **256–4096** tương đương, **256** là mặc định đề xuất
(đủ giảm contention, ít giữ budget dư). `SharedBudget::new(limit, chunk)` hiện do caller
chọn; W1 nên dùng 256 (hoặc hằng dùng chung) khi nối.

## 5. Khuyến nghị

1. **Gate cứng B7**: CPU-only W=4 **2,69× ≥ 2,4** → PASS trên máy này (caveat hybrid).
2. **Payload matrix**: báo cáo theo từng payload, không gộp một ngưỡng:
   - list1k / record / enum / str1MiB đạt (3,38–4,63×).
   - list10k (1,93×) và map1k (0,77×) **chưa đạt** trong mô hình copy producer.
3. Trước khi tuyên bố “payload lớn song song”, W1 cần chốt một trong:
   - **local-first + move** cho `spawn` khi giá trị không còn dùng (không copy producer);
   - **worker-copy** có kiểm soát (copy trong worker, giới hạn bộ nhớ);
   - hoặc công bố rõ “payload lớn không đạt B7, dùng nhiều task nhỏ/chunk dữ liệu”.
4. Map 1k cần điều tra **allocator/drop/free** (worker-copy đạt 3,12× nhưng move chỉ 1,41×).
   Có thể do 64 lần drop 1k string; cân nhắc per-thread allocator, cap, hoặc move.
5. Chunk **256** cho `SharedBudget`; không cần sửa `frame/fee.rs` (caller-controlled).
6. Benchmark phải chạy khi máy tải thấp (clean runs load 2,1–2,3); ghi topology/load.

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
  cargo test -p vis-cli --test vs3_worker_bench -- --ignored --nocapture --test-threads=1
```

Log đại diện: `agent/JOURNAL/vs3-w5-worker-bench.log` (run 9, load 2,34).
