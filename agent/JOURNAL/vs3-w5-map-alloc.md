<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — điều tra `map1k` producer-copy <1× (allocator/drop)

**Trạng thái**: điều tra + benchmark, **không sửa runtime/API production**. Thêm **example
probe** (`crates/vis-cli/examples/vs3_worker_map_probe.rs`) để tái lập; không đổi `Value`/
`SendValue`/WorkerPool.
**Baseline**: `5103da4`; `git merge main` fast-forward trên `vs3/w5-spike`; **không** push/merge,
**không** GitHub Actions.
**Commit**: example probe + journal này + `vs3-w5-map-alloc.log`; không đổi `crates/**` production.

## 1. Kết luận ngắn

**Không phải CPU**: map1k producer-copy chậm vì **1000 khoá `String` được cấp phát mỗi lần
clone**, rồi được **free trên worker thread khác** (cross-thread allocator/free contention).
Bằng chứng:

- `map1k` khoá `Str` (1000 String alloc/clone): pool-drop **không CPU** 64 task,
  producer W1 **6,28 ms** → W4 **19,01 ms** (**0,33×**).
- Cùng 1000 entry nhưng khoá `Int` (**1 Vec alloc/clone**): producer W1 **3,93 ms** →
  W4 **1,09 ms** (**3,6×**).
- `list10k` (**1 Vec 720 KB**): producer W1 18,60 ms → W4 **6,70 ms** (**2,78×**).
- **Worker-copy** map_str: W1 4,05 ms → W4 **1,46 ms** (2,77×) — tránh cả clone tuần tự
  lẫn free chéo thread.
- **Move** prebuilt map_str: W1 1,68 ms → W4 **8,47 ms** (0,20×) — free chéo thread vẫn đắt;
  move **không** phải fix.
- `with_capacity`/reserve **không giúp**: `Vec::clone` đã cấp đúng len; 60,5 µs so 59,7 µs.
- Shared `Arc` read-only: 0,16 ms W4 — O(1), không deep clone; chỉ dùng được khi task không sửa.

## 2. Kích thước / cấp phát

`SIZE` (release):

| Thành phần | Giá trị |
|---|---:|
| `size_of::<SendValue>()` | **72 B** |
| `size_of::<(SendValue, SendValue)>()` | **144 B** |
| map1k `Vec` (1000 cặp) | **144.000 B** |
| list10k `Vec` | **720.000 B** |
| map1k logical (B7, gồm khoá) | 155.890 B |
| alloc/clone map1k khoá `Str` | **1 Vec + 1000 String** = **1001 alloc** |
| alloc/clone map1k khoá `Int` | **1 Vec** |
| drop map1k khoá `Str` | **1001 free** |

Clone/drop median (64 reps, release, 3 lần):

| Payload | Clone | Drop / phần tử |
|---|---:|---:|
| list10k | 295.391 ns | 69.824 ns |
| map1k `Int` keys | 43.186 ns | 13.347 ns |
| map1k `Str` keys | **59.729 ns** | **19.484 ns** |
| map1k `Str` + reserve | 60.455 ns | — |

64 task ⇒ map_str **64.064 alloc + 64.064 free**; map_int chỉ **64 + 64**. Đây là nguồn
contention, không phải CPU 200 µs/task.

## 3. Pool drop không CPU (64 task, median 3 lần, release)

| Payload | Mode | W1 (ms) | W4 (ms) | W4/W1 |
|---|---|---:|---:|---:|
| map1k Str | producer | 6,28 | **19,01** | **0,33×** |
| map1k Str | move (prebuilt) | 1,68 | **8,47** | **0,20×** |
| map1k Str | worker-copy | 4,05 | **1,46** | **2,77×** |
| map1k Str | shared Arc (read-only) | 0,018 | **0,16** | — |
| map1k Int | producer | 3,93 | **1,09** | **3,6×** |
| list10k | producer | 18,60 | **6,70** | **2,78×** |
| list10k | worker-copy | 6,57 | **2,20** | **3,0×** |

- W2 cũng tệ với map_str producer (21,0 ms) → contention tăng theo số worker.
- Move vẫn tệ vì map được cấp trên main và free trên worker; worker-copy cấp/free cùng worker.
- Shared Arc chỉ đo task đọc; muốn task sửa cần copy-on-write.

## 4. Ngưỡng kích thước với CPU ~200 µs/task

`MAPCPU`: 64 task, CPU calibrate ~204–240 µs/task, median 3 lần; speedup W4/W1:

| entries map (Str keys) | producer W4 | worker-copy W4 |
|---:|---:|---:|
| 64 | **3,71×** | 3,79× |
| 256 | **1,95×** | 3,66× |
| 1024 | **0,71×** | 2,73× |

- Với workload 64 task/CPU ~200 µs: producer-copy chỉ còn tốt ở map **≤64 entry**;
  map **256 đã <2,4×**; map **1k <1×**.
- Ngưỡng phụ thuộc CPU/task và thành phần khoá; cần đo lại với workload W1 thật.
  Khuyến nghị an toàn: **producer-copy chỉ cho map nhỏ ≤64 entry**, còn lại worker-copy.

## 5. Benchmark release list10k/map1k (5 lần)

`vs3_worker_bench`, median 5 lần; W4 ms và speedup W4/W1:

| Payload | Producer W4 | Producer × | Worker-copy W4 | Worker-copy × |
|---|---:|---:|---:|---:|
| list10k | 8,62 ms | **4,03×** | 7,68 ms | 2,49× |
| **map1k** | **23,06 ms** | **1,15×** | **6,54 ms** | **2,57×** |

- W4 map producer range **17,39–27,0 ms**; các lần trước 0,82–0,94× — luôn **xa 2,4×**.
- `PROBE` 5 lần: map1k producer W4 **25,51 ms**, worker **6,16 ms**; list10k 8,75 / 7,33 ms.
- 5/5 lần: `print_merge 124 byte`, cross-write, `used+remaining==limit` OK; matrix rows
  `used=6.400/remaining=10.000/limit=16.400`.
- Calibration 200–207 µs lần đo sạch; load 3,3–10; W1 có outlier (list10k worker 111 ms)
  nên đọc median + range, không chỉ một lần.

## 6. Đề xuất fix cụ thể

1. **Chọn mode theo payload (không đổi runtime)**:
   - map lớn (≥256 entry, đặc biệt khoá `Str`) ⇒ `spawn_worker_copy`.
   - map nhỏ (≤64 entry) hoặc payload CPU lớn ⇒ producer-copy có thể giữ.
   - **Không dùng move làm fix allocator**: move prebuilt vẫn free chéo thread (0,20×).
2. **Thêm API read-only `spawn_shared`** (đề xuất, chưa làm):
   ```rust
   pub fn spawn_shared<F>(&mut self, payload: Arc<SendValue>, f: F)
   where F: FnOnce(&mut WorkerCtx, &SendValue) -> Result<SendValue, String> + Send + 'static
   ```
   Task giữ `Arc`, đọc `&SendValue`; O(1)/task, không deep clone. Chỉ dùng cho capture
   **bất biến**; task sửa payload phải copy-on-write. Đây là fix rẻ nhất cho map read-only
   (probe shared 0,16 ms W4 so 19 ms producer).
3. **Biểu diễn map copy-on-write / intern khoá** (dài hạn, cần A0/W1 duyệt):
   `SendValue::Map` dùng `Arc<[(SendValue, SendValue)]>` hoặc khoá `Arc<str>`/`Name` để clone
   chỉ tăng refcount; sửa map dùng `Arc::make_mut`. Xoá 1000 alloc/free mỗi clone nhưng đổi
   `SendValue`/`Value` ABI — **không tự làm**.
4. **Reserve/with_capacity**: **không có lợi** (đo 60,5 vs 59,7 µs); bỏ khỏi hướng tối ưu.
5. **Chấp nhận + giới hạn**: nếu chưa đổi representation, ghi rõ producer-copy map chỉ cho
   map nhỏ; map1k phải worker-copy. Đây là khuyến nghị ngắn hạn.
6. **Allocator per-worker/arena**: có thể giảm remote free nhưng phức tạp và ảnh hưởng toàn
   cục; worker-copy đã giải quyết rẻ hơn.

## 7. Kiểm chứng

- `cargo test --release -p vis-cli --test vs3_worker`: **12 passed / 0 failed / 0 ignored**,
  10,29 s.
- `cargo fmt --all -- --check`: PASS; `cargo clippy -p vis-cli --all-targets -- -D warnings`:
  PASS (gồm example probe).
- df `/home`: **19 GB free** (>5G).
- Log: [`vs3-w5-map-alloc.log`](vs3-w5-map-alloc.log) (probe 3 lần + benchmark 5 lần + test).

## 8. Blocker / giới hạn

- Probe **không CPU** và `MAPCPU` là synthetic; threshold thật phụ thuộc CPU/task, kích thước
  map, kiểu khoá và workload W1.
- `Value::Map`/`SendValue::Map` hiện vẫn deep-clone; **chưa sửa production**. `spawn_shared`
  mới là đề xuất, chưa có API.
- `spawn_shared` chỉ đúng cho read-only; task sửa map cần COW hoặc worker-copy.
- Allocator phụ thuộc platform (glibc/Linux); Windows/allocator khác có thể khác.
- `map1k` producer-copy vẫn **<2,4×** (0,82–1,15× tuỳ lần); không tuyên bố đã fix.
- Chưa nối evaluator; chưa đo map1k trong `async fn`/frame thật.
- Example probe là tooling, không nằm trong test suite/CI; không GitHub Actions.
