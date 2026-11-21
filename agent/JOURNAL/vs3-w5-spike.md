<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3a (W5) — spike khả thi worker pool / heap theo luồng

**Trạng thái**: **SPIKE XONG — CHỈ ĐO + QUYẾT ĐỊNH, KHÔNG CODE PRODUCTION.**
**Nhánh**: `vs3/w5-spike` · **Worktree**: `/home/ps/.local/share/opencode/worktree/cd45e5/vs3-w5-spike`
· **Baseline**: `9ad29e1` (2026-11-21).
**Phạm vi**: đo chi phí dữ liệu (copy/move list/map/record/enum), chi phí spawn + channel 2–4 luồng,
RSS; so sánh ba phương án (a) task affine + heap theo luồng + copy/move tại biên, (b) `Arc`/COW,
(c) dừng single-thread. **Không** sửa mã sản phẩm, **không** thêm crate, **không** đụng
`Cargo.toml`, **không** push/merge.

**Không tuyên bố**: worker pool chưa xong; VS3-3b chưa mở; chưa có thay đổi runtime/bytecode/API.
Mọi số dưới đây là **số đo trên bản mô phỏng độc lập**, không phải benchmark tích hợp của `vis`.

---

## 0. Kết luận ngắn (3 dòng)

1. **(a) khả thi và nên giữ**, nhưng phải là **(a) có điều kiện**: `spawn`/`await` **local-first**
   (cùng worker thì không copy), chỉ copy khi thật sự qua biên worker; channel copy **một lần
   encode + một lần decode**; từ chối closure/handle/Task bằng `VS0508`.
2. **Không chọn (b) `Arc`/COW toàn cục**: clone/channel rất nhanh (10 ns vs 5–60 µs; channel
   ~0,2 µs vs 10–140 µs) nhưng COW **vẫn O(n)** khi ghi (5,7 µs/56,9 µs — ngang deep copy), đổi
   `Value` 64→48 byte, `Rc`→`Arc` (atomic), `RefCell`→khoá cho `Env`/`Chan`, và trái VD-160.
   Chỉ đáng làm **lát riêng cho lá lớn** `str`/`bytes` nếu workload thật cần.
3. **Không dừng single-thread**: đo được **2,45× ở W=4** (CPU thuần) / **2,65×** (pool CPU payload
   rỗng) và **6,83× ở W=12** trên máy 12 luồng; nhưng máy là **2 P-core + 8 E-core**, nên ngưỡng
   `0,6 × min(nproc,N,4) = 2,4` chỉ **vừa đạt**. Phải đo kèm topology/load, không dùng `nproc` như
   số nhân đồng nhất.

---

## 1. Câu hỏi quyết định và ràng buộc đã chốt

| Nguồn | Nội dung áp vào spike |
|---|---|
| **VD-160** (`DECISIONS.md:1028-1029`) | Mỗi worker **heap riêng**, bộ đếm tham chiếu **không atomic**; loại `Arc`/`Mutex` toàn cục. `Value` **!Send**; `Task` **!Send**; `Chan` **Send+Sync**; `Host` **Send+Sync**. Giá trị qua `Chan` giữa worker: **sao chép sâu**; closure/handle/Task **không gửi được**. Chia sẻ `bytes`/`str` lớn qua `Arc`: **hoãn, chỉ làm khi có số đo**. |
| **VD-178 QĐ-1** (`agent/JOURNAL/vs3-qd-1.md:17-28`) | `spawn` được gửi sang worker khác; đối số/capture/giá trị trả phải **Sendable** và **sao chép sâu**; kiểu gửi được: `int/float/bool/str/bytes/()/list/map/set/tuple/record/enum` chỉ chứa các kiểu này; closure/handle/Task **VS0508**. Đo song song bằng task **CPU**, ngưỡng `0,6 × min(nproc,N,4)`. |
| **VD-271** (`DECISIONS.md:3660+`) | VS3-3a **chỉ spike đo**, chưa production; VS3-1 single-thread trước; worker/GC sau khi có lát riêng. |
| **Thiết kế** (`docs/ViScript-thiet-ke.md:769-771`) | `parallel.map`/`thread.spawn` chạy luồng OS; giá trị qua lại phải `Send`; **mỗi luồng một VM với heap riêng**. |

**Câu hỏi**: với `Value` dùng `Rc`/`RefCell` và `Env = Rc<RefCell<HashMap<u32, Value>>>`
(`value.rs:272`), chi phí copy/move tại biên worker/channel có đủ rẻ để (a) đạt tiêu chí
VS3-3b (≥2× trên 2 core, theo QĐ-1) mà không phải đổi toàn bộ `Value` sang `Arc` (b), hoặc
từ bỏ song song (c)?

---

## 2. Phương pháp đo

| Mục | Cách làm |
|---|---|
| Prototype | `agent/JOURNAL/vs3-w5-spike/bench.rs` — **standalone**, chỉ `std`, chạy `rustc -O` (rustc 1.93.1). Không crate mới, không `Cargo.toml`. |
| Mô phỏng `Value` | Mirror đúng hình dạng `crates/vis-interp/src/value.rs:319-374`: `Unit/Int/Float/Size/Duration/Percent/Str(String)/Bool/Bytes/Char/Path/List(Vec<Value>)/Map(Vec<(Value,Value)>)/Record(String,Vec<(String,Value)>)/Enum(Rc<str>,Rc<str>,Vec<Value>)/Range/Builtin/Fn(Rc)/Vm(Rc)/Task(Rc)/Chan(Rc<RefCell>)/Cap`. Kích thước in ra **64 byte**, khớp VD-267 (`Value` 72→64). |
| `Wire` | Dạng **Sendable** sau copy: `Str(Box<str>)`, `List(Vec<Wire>)`, `Map`, `Record`, `Enum(Box<str>,Box<str>,...)`; không có `Rc`/`RefCell`; 56 byte. |
| `ArcValue` | Phương án (b): `List(Arc<Vec<ArcValue>>)`, `Map(Arc<...>)`, `Record(Arc<str>,Arc<...>)`, `Enum(Arc<str>,Arc<str>,Arc<Vec<...>>)`, `Str(Arc<str>)`, `Bytes(Arc<[u8]>)`; 48 byte. |
| Máy | Linux, **i5-1235U**: 2 P-core (4 luồng) + 8 E-core, tổng 12 luồng; `nproc=12`. Load ghi trong từng lần chạy (1,0–4,2). |
| Thời gian | Median của 5 lần lặp bên trong mỗi phép đo; toàn bộ suite lặp **14 lần** (run 16..29) để lấy median; trước đó **warm CPU ~800 ms** vì laptop tăng xung rất chậm (đo được 31% xung lúc rảnh). |
| RSS | Đọc `/proc/self/status` (`VmRSS`, `VmHWM`) trong **tiến trình con sạch** cho từng cảnh báo; tránh allocator giữ bộ nhớ giữa các cảnh báo. |
| Bằng chứng | `bench-run.txt` (một lần chạy đại diện), `bench-summary.txt` (median 14 lần + CPU scale). Lệnh tái lập ở §9. |

**Giới hạn thẳng thắn**: đây là mirror, không phải `Value` thật; các con số dùng để **so sánh tương
đối**, không phải hiệu năng tuyệt đối của `vis`. `Value` thật có thêm `Span`/`Trap`/`Name` interning
và đường eval; benchmark không mô phỏng chi phí eval, chỉ mô phỏng **chi phí dữ liệu tại biên**.
Máy hybrid + turbo không ổn định; median 14 lần và [min,max] được ghi kèm.

---

## 3. Số đo

### 3.1 Kích thước

| Kiểu | Byte | Ghi chú |
|---|---:|---|
| `Value` (mirror) | **64** | khớp VD-267; `Enum(Rc<str>,Rc<str>,Vec<Value>)` là biến thể lớn nhất |
| `Wire` (Sendable) | **56** | mất `Rc`, tên enum thành `Box<str>` |
| `ArcValue` (Arc/COW) | **48** | container thành `Arc<Vec<...>>`; lá `Arc<str>`/`Arc<[u8]>` |

### 3.2 Copy / move một luồng (median 14 lần; ns/µs mỗi lần)

| Dữ liệu | Clone | MiB/s logic | Alloc | KiB logic |
|---|---:|---:|---:|---:|
| `list` 1k `int` | **5,48 µs** | 11,4 GB/s | 1 | 62,5 |
| `list` 10k `int` | **59,7 µs** | 10,0 GB/s | 1 | 625 |
| `map` 1k `str→int` | **24,4 µs** | 5,0 GB/s | 1.001 | 128,8 |
| `map` 10k `str→int` | **252,8 µs** | 4,9 GB/s | 10.001 | 1.297,7 |
| `record` 16 trường + list 1k | **6,19 µs** | 10,6 GB/s | 19 | 63,9 |
| `record` 16 trường + list 10k | **62,5 µs** | 10,0 GB/s | 19 | 626,4 |
| `enum(Result.Ok(list1k))` | **5,68 µs** | 10,8 GB/s | 2 | 62,6 |
| `enum(Result.Ok(list10k))` | **62,0 µs** | 9,8 GB/s | 2 | 625,1 |
| **10k giá trị enum** `Some(i)` | **341,1 µs** | 3,5 GB/s | 10.001 | 1.250 |
| `str` 1 KiB | **14,5 ns** | 63 GB/s | 1 | 1 |
| `str` 1 MiB | **35,2 µs** | 28,5 GB/s | 1 | 1.024 |
| `bytes` 1 MiB | **35,0 µs** | 28,6 GB/s | 1 | 1.024 |
| closure (`Rc`, chỉ bump) | **2,4 ns** | — | 0 | 0 |
| **move** list 10k trong luồng | **3,7 ns** | — | 0 | 0 |
| `list1k.clone + push` | **5,68 µs** | — | 1 | — |
| `list10k.clone + push` | **62,6 µs** | — | 1 | — |

**Đọc số**: copy list là **memcpy có cấp phát**, đạt ~10 GB/s; map/enum nhiều khoá là **nghẽn cấp
phát** (10k `String`/10k `Vec` nhỏ), không phải băng thông. `move` trong cùng luồng là O(1) (3,7 ns),
nhưng **move qua biên không miễn phí** nếu phải đổi biểu diễn (xem 3.3).

### 3.3 `Wire` — copy-convert vs move-convert

| Phép | 1k | 10k |
|---|---:|---:|
| `Value → Wire` (mượn + clone, hot loop) | **7,20 µs** | **70,2 µs** |
| `Wire → Value` (mượn + clone, hot loop) | **7,13 µs** | **71,6 µs** |
| `Value → Wire` batch (mượn + clone) | 7,56 µs | 76,4 µs |
| `Value → Wire` batch (**move/tiêu thụ**) | **11,27 µs** | **118,9 µs** |
| `Wire → Value` batch (mượn + clone) | 7,49 µs | 77,2 µs |
| `Wire → Value` batch (**move/tiêu thụ**) | **10,09 µs** | **104,9 µs** |
| `str 1 MiB` batch copy-convert | — | **70,4 µs** |
| `str 1 MiB` batch move-convert | — | **38,5 µs** |

**Phát hiện phản trực giác**: **move không nhanh hơn** cho list phẳng — vì vẫn phải dựng container
`Wire` mới và xen kẽ cấp phát/giải phóng; move chỉ thắng rõ ở **lá lớn** (`str` 1 MiB: 38,5 µs so
với 70,4 µs). Vì vậy không được giả định “tiêu thụ giá trị là miễn phí”; nếu tối ưu biên, phải đo
đúng dạng dữ liệu.

**Hệ quả cho một vòng `spawn` + `await` qua worker khác** (copy cả đối số và giá trị trả):
- list 1k: ~**14,3 µs** (7,20 + 7,13);
- list 10k: ~**141,8 µs** (70,2 + 71,6);
- 1 MiB str: ~**70 µs** (35 + 35) — hoặc ~**0,02 µs** nếu lá được `Arc` chia sẻ.

### 3.4 `Arc`/COW — clone O(1), ghi vẫn O(n)

| Phép | Kết quả |
|---|---:|
| `arc.list1k.clone` | **10,8 ns** |
| `arc.list10k.clone` | **10,6 ns** |
| `arc.map1k.clone` | **10,8 ns** |
| `arc.record16+list1k.clone` | **18,8 ns** (3–4 lần bump) |
| `arc.enum-list1k.clone` | **27,5 ns** |
| `arc.str1KiB.clone` / `arc.str1MiB.clone` | **10,1 / 10,4 ns** |
| `arc.bytes1MiB.clone` | **10,3 ns** |
| `arc.list1k.cow-push` | **5,69 µs** (≈ `Value` clone+push 5,68 µs) |
| `arc.list10k.cow-push` | **56,9 µs** (≈ `Value` clone+push 62,6 µs) |
| `Rc` closure clone (đối chứng) | **2,4 ns** (Arc chậm ~4× cho giá trị nhỏ) |

**Đọc số**: Arc thắng tuyệt đối cho **đọc/chia sẻ** (clone 10 ns bất kể kích thước; channel
0,2–0,6 µs/msg). Nhưng **COW ghi list số vẫn O(n)** và ngang deep copy — Arc không giải quyết
workload `xs = xs.push(i)` trong vòng lặp. Arc cũng làm **giá trị nhỏ chậm hơn** (atomic) và đổi
shape `Value`.

### 3.5 Spawn + channel 2–4 luồng

Mỗi task CPU ~**200–235 µs** (calibrate warm; `cpu_work` vòng lặp). `copy` = encode đối số +
decode kết quả; `arc` = clone `Arc` O(1); `local` = cùng phép copy nhưng không luồng/channel.

| Cấu hình | W=1 | W=2 | W=4 | Speedup W=4 |
|---|---:|---:|---:|---:|
| Dispatch rỗng (`unit`, 20k task) | 3,2 ms | 3,0 ms | 3,5 ms | 0,91 (producer là nút cổ chai) |
| **CPU (pool), payload rỗng** | 127,5 ms | 73,2 ms | **48,0 ms** | **2,65×** |
| CPU + list1k, copy | 137,0 ms | 78,5 ms | **51,5 ms** | **2,51×** |
| CPU + list1k, arc | 126,8 ms | 76,0 ms | **48,1 ms** | **2,49×** |
| CPU + list1k, local | 129,8 ms | — | — | — |
| CPU + list10k, copy | 94,2 ms | 64,5 ms | **37,9 ms** | **2,58×** |
| CPU + list10k, arc | 63,3 ms | 39,7 ms | **24,4 ms** | **2,55×** |
| CPU + list10k, local | 95,5 ms | — | — | — |

**Đọc số**:
- Dispatch/spawn+channel **rất rẻ**: ~**0,16 µs/task** khi pipeline đã đầy; channel unit
  0,06–0,13 µs/msg. Nút cổ chai không phải lock mà là **copy dữ liệu**.
- Với payload 1k, copy thêm ~7 µs/task (≈3% của 235 µs) — gần như không ảnh hưởng.
- Với payload 10k, copy thêm ~**115 µs/task** (gần 50% công việc CPU); W=4 tuyệt đối 37,9 ms
  (copy) so với 24,4 ms (Arc, nhanh hơn 36%), nhưng **speedup tương đối** của Arc thấp hơn vì
  baseline W=1 của Arc đã nhanh hơn.
- **Ngưỡng QĐ-1 `2,4`**: copy đạt 2,51–2,58; Arc đạt 2,49–2,55; CPU pool 2,65 (CPU thuần
  không pool ở §3.8 là 2,45). Tất cả **vừa đạt**; W=2 chỉ 1,48–1,85 (ngưỡng W=2 là 1,2 — đạt).

### 3.6 Channel

`copy+decode` = sender `to_wire` + receiver `wire_to_value` (2 lần chuyển đổi). `arc` = clone
`Arc`. `move` = `Wire` dựng sẵn, chỉ chuyển ownership + drop (không copy payload).

| Payload | Copy+decode | Arc | Move + drop (sàn) |
|---|---:|---:|---:|
| unit | 0,07–0,11 µs | 0,08–0,13 µs | — |
| list1k | **12,6 µs** (4,2 GB/s) | **0,11–0,15 µs** | 3,9 µs |
| list10k | **132–141 µs** (3,8–4,0 GB/s) | **0,21–0,31 µs** | 56 µs |
| str1MiB | **125–155 µs** | **0,38–0,61 µs** | 38,9 µs |

**Đọc số**: channel **không** tốn ~10 µs vì lock; phần lớn là **encode + decode + cấp phát/giải
phóng**. `move` 1 MiB chỉ ~39 µs vì chỉ chuyển `Box` (O(1)) rồi consumer drop; chi phí còn lại là
free bộ nhớ lớn, không phải channel. Arc gần như miễn phí vì không cấp phát/không copy.

### 3.7 RSS (tiến trình con sạch; MB)

| Cảnh báo | RSS | HWM |
|---|---:|---:|
| baseline | 2,6 | 2,6 |
| 4 worker rảnh (stack mặc định) | 2,8 | 2,8 |
| 4 worker rảnh, stack **256 MiB** | 2,8 | 2,8 |
| 50 list 10k `Value` (dựng) | 33,3 | 33,3 |
| + clone toàn bộ (2 bộ) | 64,0 | 64,0 |
| drop bộ clone | 33,4 | 63,8 (HWM giữ đỉnh) |
| 4 worker × 25 list 10k (heap riêng) | 64,2 | 64,2 |
| 100 list 10k `ArcValue` (dựng) | 48,7 | 48,7 |
| + 4 “heap” clone `Arc` | 48,8 | 48,8 |
| queue copy 64 × 10k `Wire` | 36,9 | 36,9 |
| queue arc 64 × 10k (cùng một list) | 3,1 | 3,1 |
| 1 triệu `Value::Int` (list) | 63,7 | 63,7 |

**Đọc số**:
- 4 luồng rảnh chỉ +0,3 MB RSS; **stack 256 MiB là đặt trước địa chỉ ảo, không tốn RSS** cho tới khi
  chạm trang. Nhưng đệ quy sâu trên worker vẫn là rủi ro (xem §6).
- Copy per-thread **nhân đôi bộ nhớ**: 100 list × 10k = 64,2 MB cho 4 heap; Arc chia sẻ = 48,7 MB
  (25% ít hơn nhờ `ArcValue` 48 byte + không nhân bản).
- `queue-arc 3,1 MB` là trường hợp **cùng một list** được 64 lần clone; queue thật với 64 list
  khác nhau sẽ ~48 MB, đừng đọc 3,1 MB như “queue arc luôn rẻ”.

### 3.8 CPU scale (median 6 vòng, warm)

| Workers | 1 | 2 | 4 | 8 | 12 |
|---|---:|---:|---:|---:|---:|
| CPU thuần | 1,00 | **1,71** | **2,45** | **4,56** | **6,83** |
| Pool copy (payload rỗng) | 1,00 | **1,75** | **2,45** | — | — |

Máy **không đồng nhất**: 2 P-core (4 luồng) + 8 E-core. W=4 thường rơi vào 2P+2E, nên 2,45×;
W=8 mới dùng nhiều E-core (4,56×); W=12 mới gần 7×. Ngưỡng QĐ-1 tính `min(nproc,N,4)=4` ⇒ 2,4;
**W=4 vừa đạt**. Trên máy 4 nhân đồng nhất có thể cao hơn, nhưng **không được tuyên bố** khi chưa đo.

---

## 4. Phân tích ba phương án

### (a) Task affine + heap theo luồng + copy/move tại biên

**Đo được**:
- Copy list 1k/10k: 5,5/59,7 µs; copy đối số + trả về qua worker: ~14/142 µs.
- Channel copy+decode: 12,6 µs (1k) / 132–141 µs (10k) / 125–155 µs (1 MiB).
- Spawn/channel overhead thuần: 0,06–0,16 µs/msg; dispatch 20k task 3,2 ms.
- CPU + list1k W=4: 2,51×; CPU + list10k W=4: 2,58× (đạt 2,4).
- RSS: copy per-thread 64,2 MB cho 100×10k; gấp đôi so với một bộ.

**Ưu**: đúng VD-160, không khoá toàn cục, `Rc` rẻ (2,4 ns), dễ suy luận ownership, không đổi
`Value`/API; channel copy có thể dự đoán; phù hợp “mỗi luồng một VM/heap”.

**Nhược**: copy tỉ lệ với kích thước; task ngắn + payload lớn sẽ **không đạt** ngưỡng; memory nhân
theo W; closure/Task/handle không gửi được; global `var` và output order là vấn đề ngữ nghĩa
(xem §6).

**Khi nào (a) đủ**: task CPU ≥ ~100–200 µs hoặc payload ≤ ~1k; tránh copy khi `spawn`/`await` cùng
worker; chunk dữ liệu lớn; channel dùng cho message nhỏ hoặc chấp nhận 2 lần chuyển đổi.

### (b) `Arc`/COW toàn cục

**Đo được**:
- Clone O(1): 10,6–10,8 ns (list 1k/10k, map 1k), 18,8 ns (record), 27,5 ns (enum), ~10 ns (str/bytes).
- Channel Arc: 0,11–0,31 µs (1k/10k), 0,38–0,61 µs (1 MiB).
- RSS 100×10k: 48,7 MB (so với 64,0 MB copy), 4 heap clone +0,1 MB.
- **COW push vẫn O(n)**: 5,69 µs (1k), 56,9 µs (10k) — ngang `Value` clone+push.
- `Rc` closure clone 2,4 ns vs `Arc` clone ~10 ns: giá trị nhỏ chậm ~4×.

**Ưu**: đọc/chia sẻ gần như miễn phí; channel nhanh 100–1000×; giảm memory khi chia sẻ; không cần
encode/decode.

**Nhược**: đổi `Value` 64→48 byte và **mọi** match/đột biến; `Rc`→`Arc` atomic; `Env`/`Chan`
`RefCell`→`Mutex`/`RwLock`; COW O(n) khi ghi; dễ deadlock/lock contention; trái VD-160 (“heap
riêng, bộ đếm không atomic”); **không giải quyết** workload ghi nhiều; speedup tương đối có thể
giảm vì baseline W=1 nhanh hơn (đo được 2,55× cho list10k, sát ngưỡng). Đây là **viết lại toàn
cục**, không phải tối ưu cục bộ.

**Kết luận**: không chọn (b) toàn cục trong VS3-3b. Chỉ xem xét **lai hẹp** `Arc<str>`/`Arc<[u8]>`
cho lá lớn — nơi số đo cho thấy lợi ích rõ (1 MiB: clone 35 µs → 10 ns; channel 125–155 µs →
0,4–0,6 µs; batch move 38,5 µs so với copy 70,4 µs). Việc này vẫn đổi shape `Value` và đường
`String`, nên phải là lát riêng có đo, không gộp vào 3b.

### (c) Dừng single-thread

**Ưu**: zero rủi ro; giữ tất định tuyệt đối; không copy; không đổi `Value`.

**Nhược**: bỏ **2,45× ở W=4** và **6,83× ở W=12** đo được; không đáp ứng QĐ-1/VS3-3b; không có
“song song thật” cho task CPU. Chỉ nên chọn nếu A0 quyết định hoãn VS3-3 vô thời hạn — không phải
kết luận từ số đo này.

### (d) Frame, closure capture, channel — vì sao task phải affine

| Thành phần | Hiện trạng | Hệ quả cho pool |
|---|---|---|
| Frame interp | `IFrame` (`frame.rs:439-446`) giữ `Vec<CallFrame>`, `vals`, `loops`, `mutable: Rc<RefCell<HashSet<u32>>>`, `globals: Env`; `CallFrame` giữ `Env = Rc<RefCell<HashMap<u32,Value>>>` (`value.rs:272`) | Cả frame là **!Send**; không thể chuyển frame đang treo sang worker khác. Task phải **affine**: frame sống trên worker sở hữu từ lúc bắt đầu đến lúc xong. |
| Frame VM | `Th`/`Frame { fi, pc, locals, captured, st_cu, writeback }` (heap, có `pc`) — `vis-vm/src/vm_frame.rs`; `vis-vm/src/vm.rs` mượn `vis_interp::rt::Scheduler` | `pc` làm resume dễ, nhưng `locals/captured` chứa `Value` (`Rc`) ⇒ **!Send**; cũng không di trú giữa chừng. Mỗi worker cần `Vm` + `Scheduler` riêng. |
| Closure/capture | `Value::Fn(Rc<Closure>)`; `Closure.env: Env` (`value.rs:290-303`); `spawn` chỉ nhận **tên hàm async cấp tệp** (`interp.rs:1079-1096`), không nhận closure | Điểm thuận lợi: **spawn không phải chuyển env closure**. Nhưng `send`/trả về closure vẫn phải `VS0508`; không deep-copy `Env` (sẽ làm `var` phân kỳ). |
| Channel | `Value::Chan(Rc<RefCell<Chan>>)`; `Chan.buf: VecDeque<Value>` (`rt.rs:72-88`); `send` push `Value` (move) | Target: `Chan` là `Arc<Mutex<ChanBuf<Wire>>>` + `Condvar`; buffer **không** chứa `Value`; `send` encode, `recv` decode. |
| Timer/reactor | `FrameTest.reactor`, timer heap, `FrameReactorBackend` nằm trong `Scheduler` (`rt.rs:246-373`) | Mỗi worker có timer/reactor riêng; wake cross-worker phải qua channel/proxy, không chia sẻ `RefCell`. |

**Kết luận frame**: mô hình đúng là **task affine + frame ở lại worker sở hữu**; chỉ **đối số**
(vào) và **giá trị trả** (ra) đi qua biên. Work-stealing chỉ được **bắt đầu** task ở worker khác
trước khi chạy, không di trú frame đang treo. Đây là lý do kỹ thuật cho “local-first” ở §5.

### Bảng chọn nhanh

| Tiêu chí | (a) affine + copy | (b) Arc/COW | (c) single-thread |
|---|---|---|---|
| Copy 10k | 59,7 µs | 10,6 ns | 0 |
| Channel 10k | 132–141 µs | 0,2–0,3 µs | — |
| Ghi list 10k | 59,7 µs | 56,9 µs | 59,7 µs (nếu copy) |
| Memory 100×10k | 64,2 MB (4 heap) | 48,7 MB | 32 MB (1 heap) |
| Speedup W=4 (CPU) | 2,65× | 2,65× (cùng CPU) | 1× |
| Đổi `Value`/API | Không | **Có, toàn cục** | Không |
| Trái VD-160 | Không | **Có** | Không |
| Rủi ro chính | copy lớn, global var, output order | lock/COW/atomic, rewrite | không song song |

---

## 5. Khuyến nghị

### 5.1 Quyết định

**Giữ (a) làm mô hình nền cho VS3-3b**, với các điều kiện sau (không phải (a) “ngây thơ”):

1. **Local-first / affinity**: `spawn` đặt task vào **run queue của worker hiện tại** nếu có thể;
   chỉ chuyển sang worker khác khi (i) có lệnh song song tường minh (`parallel.map`/worker pool),
   hoặc (ii) work-stealing cho task đủ dài. Task/frame **không di trú** sau khi bắt đầu (frame chứa
   `Env`/`Value` !Send, §4.4); chỉ args/return qua biên. Như vậy `let t = spawn f(x); await t`
   **không copy** khi cùng worker — tránh 2 lần copy vô ích.
2. **Một lần copy tại biên, không hai lần**: nếu phải qua worker khác, encode `Value → Wire` **một
   lần** ở sender; receiver **adopt** `Wire` (hoặc decode một lần) — không copy thêm. Với `spawn`,
   đối số nên được đánh giá/đóng gói theo hướng `Wire` nếu biết đích là worker khác; với `send`,
   tận dụng quyền sở hữu của đối số để **move** thay vì clone khi có thể (nhưng xem §3.3: move
   không tự động nhanh hơn cho list phẳng).
3. **Sendable + `VS0508`**: giữ đúng danh sách QĐ-1; closure/handle/Task/Cap **không gửi**; kiểm
   ở checker khi biết kiểu, runtime khi không. Không silent-copy closure/env.
4. **Task handle opaque**: `Value::Task(Rc<Task>)` hiện **!Send**; QĐ-1 lại yêu cầu `await` task ở
   worker khác. Dùng **`TaskId` (u64) + bảng task theo worker + proxy completion** (channel một
   lần), **không** bọc `Task` bằng `Arc<Mutex>` toàn cục (trái VD-160). Đây là việc phải chốt ở
   3b-1/3b-2.
5. **Channel**: `Chan` thành `Arc<Mutex<ChanBuf>>` + `Condvar`, buffer chứa **`Wire`** (không chứa
   `Value`), capacity hữu hạn; `send` encode, `recv` decode; close/select/timeout theo wait-set.
   Chấp nhận chi phí 2 lần chuyển đổi cho 3b; nếu workload lớn (≥10k hoặc ≥1 MiB) chiếm tỷ lệ cao,
   mở lát 3b-5 (leaf Arc) — **không** đổi toàn cục.
6. **Output/task lifecycle**: mỗi task có buffer `out` riêng; merge theo thứ tự spawn/await để
   W=1 và W=4 **byte-identical** trên chương trình không chia sẻ. Global `var` phải có luật (xem
   §6) trước khi bật pool.

### 5.2 Vì sao không chọn (b)/(c)

- (b) clone/channel nhanh hơn nhiều nhưng **COW ghi vẫn O(n)**; chi phí là **rewrite toàn cục**
  `Value`/`Env`/`Chan`, atomic/lock, rủi ro deadlock, trái VD-160. Số đo **không** cho thấy cần
  đánh đổi đó cho payload thường (1k: copy 5,5 µs; 10k: 59,7 µs; task 200 µs vẫn đạt 2,5×).
  (b) chỉ hấp dẫn cho **lá lớn bất biến** — tách lát riêng.
- (c) bỏ 2,45–6,83× song song đo được; không đáp ứng VS3-3b/QĐ-1. Chỉ dùng nếu A0 hoãn VS3-3.

### 5.3 Điều kiện để tuyên bố (a) “đủ”

- Task CPU tham chiếu: **≥ ~200 µs nếu payload 10k**, hoặc **payload ≤ ~1k** với task ≥ ~50 µs;
  với payload 10k, phải chấp nhận W=4 ~2,5× (không phải 4×) và **ghi rõ** trong tiêu chí nghiệm thu.
- Có **local-first**; nếu mọi spawn đều sang worker khác, chi phí copy 10k (~142 µs) có thể làm
  W=4 tụt dưới ngưỡng khi task ngắn.
- Có đo **topology/load**; máy hybrid 2P+8E cho W=4 chỉ 2,45×, sát ngưỡng 2,4.

---

## 6. Rủi ro và blocker

### 6.1 Blocker cần A0 quyết trước VS3-3b

| # | Vấn đề | Bằng chứng | Đề xuất |
|---|---|---|---|
| **B1** | `Value::Task(Rc<Task>)` **!Send** nhưng QĐ-1 yêu cầu `await` task ở worker khác | `value.rs:364`; `rt.rs:55-68`; QĐ-1 §3 | `TaskId` opaque + proxy completion; **không** `Arc<Mutex<Task>>` toàn cục. Cần A0 duyệt mô hình handle. |
| **B2** | `var` cấp tệp dùng chung `Env` (`Rc<RefCell<HashMap>>`); mỗi worker một `Interp`/`Vm` ⇒ `var` thành **per-worker**, ghi trong task có thể mất/không thấy ở main | `value.rs:272`; `interp.rs:689-702`; `docs/ViScript-thiet-ke.md:771` | Chọn: (a) cấm truy cập `var` trong task có thể chạy xa (checker/`VS0508`), (b) **affinity**: task chạm `var` buộc ở worker sở hữu, (c) định nghĩa copy-in/copy-out. Khuyến nghị (b) cho 3b-1, kèm chẩn đoán. |
| **B3** | `print` ghi ngay vào `Host.out` chung; W>1 ⇒ thứ tự/nội dung có thể xen, không byte-identical | `host.rs:12-19`; `rt.rs`/`frame` dùng `out` | Buffer `out` theo task + merge theo thứ tự spawn/await; test W=1 vs W=4. |
| **B4** | Ngân sách ngữ nghĩa VD-256 là **thread-local** `BUDGET`; pool cần **một ngân sách chung** cho cả lần chạy | `frame/fee.rs:47-49`; VD-178/VD-256 | `AtomicI64`/shared budget hoặc chia quota theo worker; phải giữ “một pool, không reset khi spawn/await”. Chốt cùng VS3-1a/1b. |
| **B5** | Channel copy 2 lần: 10k 132–141 µs/msg; 1 MiB 125–155 µs/msg | §3.6 | Chấp nhận cho 3b; nếu workload lớn chiếm ưu thế, mở 3b-5 (`Arc<str>`/`Arc<[u8]>` lá). Không đổi toàn cục. |
| **B6** | Ngưỡng QĐ-1 `2,4` trên máy hybrid: W=4 đo 2,45–2,65, sát ngưỡng; `nproc=12` không phản ánh 2P+8E | §3.8 | Ghi topology/load; gate dùng **CPU-only** + payload matrix; không dùng `nproc` đơn thuần; nếu có máy 4 nhân đồng nhất thì đo lại. |

### 6.2 Rủi ro kỹ thuật (không cần A0 ngay, nhưng phải test ở 3b)

| # | Rủi ro | Ghi chú |
|---|---|---|
| R1 | `Host` là `&mut dyn` (`host.rs:12-35`), `out`/`fs`/`net`/`db` gắn một interpreter | Mỗi worker cần Host riêng; `NetHost`/`FsHost` state không chia sẻ. VD-160 nói Host Send+Sync nhưng trait hiện chưa. |
| R2 | Thread-local `Name`/`TAGS`/`NAME_CACHE` (`value.rs:55-133,237-239`) | Worker mới phải khởi tạo lại; `Name` so **nội dung** nên đúng, nhưng mất fast path `Rc::ptr_eq` khi so cross-worker. `Wire` decode phải intern lại tên enum. |
| R3 | `MAX_CALL_DEPTH=2000` + stack worker | 4 thread stack 256 MiB **không tốn RSS** (đo 2,8 MB) nhưng đệ quy đồng bộ trên worker có thể tràn nếu giảm stack; giữ 256 MiB địa chỉ ảo hoặc test giới hạn. |
| R4 | `Rc` cycle/không GC | Copy per-thread làm tăng bộ nhớ; task/handle giữ `Rc`; không có GC. Cần cap task/tài nguyên (VD-249 §1.7) trước khi pool chạy dài. |
| R5 | Determinism của work-stealing | W=1/W=4 cùng stdout yêu cầu buffer output + thứ tự merge; work-stealing không được đổi thứ tự merge. |
| R6 | Lock contention `Chan` | Channel copy dùng `mpsc` (không phải `Arc<Mutex>`); 3b phải đo lại với lock thật; 4 producer đã làm 10k copy chậm hơn 2 producer (141 vs 132 µs). |
| R7 | Lỗi/trap qua worker | `Trap` chứa `&'static Code`, `String`, `Span` — có thể chuyển; nhưng `Result<Value, Trap>` phải encode/decode, không gửi `Value`. Giữ `VS0506` + span lúc tạo. |
| R8 | Windows/macOS | Ngoài phạm vi VS3 (VD-271); bench chỉ Linux. Không tuyên bố. |

---

## 7. Đề xuất lát VS3-3b (nếu A0 duyệt (a))

> Mỗi lát có test **trước**, gate tại mốc 4 giờ; không gộp big-bang. Đây là **đề xuất**, không phải
> cam kết thời lượng.

| Lát | Việc | Tiêu chí xong |
|---|---|---|
| **3b-0** | Hợp đồng + harness: `Value: !Send` (`compile_fail`), `Sendable`/`VS0508`, `Wire` encode/decode, test W=1 vs W=4 stdout, output buffer, global `var` diagnostic | Test đỏ-trước có trong repo; danh sách Sendable khớp QĐ-1; không đổi runtime mặc định |
| **3b-1** | Khung pool W worker, mỗi worker `Scheduler`/`Interp`/`Vm` riêng; **local-first** + run queue; **chưa** cross-worker copy | W=1 cho **cùng** kết quả hiện tại (toàn bộ L0 xanh); W>1 chỉ chạy task local |
| **3b-2** | `spawn`/`await` cross-worker: `Wire` cho args/return, `TaskId` + proxy, `VS0508`, final join/error; output merge | `spawn f(x)` + `await t` khác worker đúng; closure/handle/Task → `VS0508`; W=1/W=4 byte-identical trên chương trình không chia sẻ; không deadlock |
| **3b-3** | Channel liên worker: `Arc<Mutex<ChanBuf<Wire>>>` + `Condvar`, bounded, close/timeout/select; **một** encode + **một** decode | `send/recv/close` 2 worker; timeout không treo; số fd/timer về baseline; đo 1k/10k/1MiB; ghi rõ nếu > ngưỡng |
| **3b-4** | Gate hiệu năng/RSS: CPU-only W=1 vs W=4; CPU+1k; CPU+10k; RSS; leak; topology/load | CPU-only ≥ **2,4×** trên máy tham chiếu; W=1/W=4 stdout byte-identical; RSS không tăng tuyến tính vô hạn; ghi `nproc`/load/topology |
| **3b-5** (tuỳ) | Leaf Arc: `Value::Str(Arc<str>)`, `Bytes(Arc<[u8]>)` (và có thể `Path`) — chỉ nếu 3b-4 cho thấy payload lớn chiếm >30% hoặc workload thật cần | Đo single-thread + channel 1MiB; không đổi container `List`/`Map`; test parity; A0 duyệt riêng |

**Không làm trong 3b**: shared heap/GC; đổi `Value` sang `Send`; `Arc<Mutex>` toàn cục;
cú pháp/API mới; TLS/registry; macOS.

---

## 8. Những gì không được tuyên bố

- **Không** “worker pool xong”, “VS3-3b xong”, “song song đã bật mặc định”.
- **Không** nói `Value` đã `Send`; hiện `Value` vẫn `!Send` (`Rc`/`RefCell`).
- **Không** nói Arc/COW bị loại vĩnh viễn; chỉ **không chọn toàn cục trong 3b**.
- **Không** dùng số đo mirror làm số hiệu năng chính thức của `vis`; cần benchmark tích hợp ở 3b-4.
- **Không** đổi `VERSION`/bytecode/cache/API; không đụng `Cargo.toml`.

---

## 9. Tái lập

```bash
cd /home/ps/.local/share/opencode/worktree/cd45e5/vs3-w5-spike

# build + chạy toàn bộ
rustc -O agent/JOURNAL/vs3-w5-spike/bench.rs -o /tmp/opencode/vs3w5bench
/tmp/opencode/vs3w5bench | tee agent/JOURNAL/vs3-w5-spike/bench-run.txt

# riêng CPU scale / pool scale
/tmp/opencode/vs3w5bench --cpu-scale
/tmp/opencode/vs3w5bench --pool-cpu-scale

# RSS trong tiến trình con (bench tự gọi)
/tmp/opencode/vs3w5bench --rss-child copy-50x10k
```

- `bench-run.txt`: một lần chạy đại diện (run 20; load ~2,45).
- `bench-summary.txt`: median 14 lần + CPU/pool scale.
- Máy đo: Linux, i5-1235U (2P+8E), `nproc=12`; load ghi trong `ENV`.
- Không chạy `gate --fast` vì **không có thay đổi mã sản phẩm**; leader chạy gate trên commit
  merge nếu cần. `cargo`/`Cargo.toml` không bị đụng.

---

## 10. Nguồn

- `agent/KE-HOACH-VS3-2026-11-21.md` §2 (VS3-3), §4 (VS3-3a), §6 (W1).
- `agent/DECISIONS.md`: VD-160 (heap riêng, non-atomic), VD-178 QĐ-1 (Sendable/VS0508/ngưỡng),
  VD-256/VD-257 (budget/cancel), VD-271 (mở VS3, macOS ngoài phạm vi).
- `agent/JOURNAL/vs3-qd-1.md` §1-§5 (mục tiêu đo, Sendable, lát 1a–1e).
- `agent/JOURNAL/vd249-async-design-draft.md`; `vd257-a-coverage-report.md`;
  `vd257-c-lifecycle-report.md`; `vd252-coverage-matrix.md`;
  `agent/DE-XUAT-KIEN-TRUC-ASYNC-2026-11-20.md`.
- Mã đọc: `crates/vis-interp/src/value.rs:272,319-374`; `rt.rs:55-88,378-391,418-476`;
  `frame/fee.rs:47-49`; `frame.rs:439-446`; `host.rs:12-35`; `crates/vis-diag/src/lib.rs:510`.
- `docs/ViScript-thiet-ke.md:769-771`.
