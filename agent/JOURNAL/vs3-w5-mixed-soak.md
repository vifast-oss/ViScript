<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — mixed soak 30 phút (`spawn_shared` + worker-copy + move)

**Trạng thái**: hardening **test-only** — không sửa production/ABI/default; không nối evaluator,
không bật default, **không** GitHub Actions.
**Mốc**: main `cadc4c8` (đã merge shared stress 100 vòng); tag `v0.3.3` = `bbf3484`.
**Nhánh**: `vs3/w5-spike` trong worktree `.../worktree/cd45e5/vs3-w5-spike`;
`git merge main` fast-forward `1fe6366 → cadc4c8`; **không** push/merge/tag.
**Commit**: xem `git log -1` — `crates/vis-cli/tests/vs3_worker_soak.rs` (tách khỏi `vs3_worker.rs` để giữ R1 ≤1500 dòng) + journal + 2 log,
`crates/**` production không đổi.
**Log đầy đủ**: [`vs3-w5-mixed-soak.log`](vs3-w5-mixed-soak.log) — soak pass + full suite;
[`vs3-w5-mixed-soak-abort.log`](vs3-w5-mixed-soak-abort.log) — lần đầu dừng ở 23,9 phút vì
assertion test quá chặt (xem §4).

## 1. Test và cách chạy

Test mới `mixed_soak_shared_copy_move_w1_w4` trong `crates/vis-cli/tests/vs3_worker_soak.rs`
(không `#[ignore]`), **env-gated**:

- `VS3_W5_MIXED_ROUNDS` — số vòng; **mặc định 1** (~3 s release) để gate không chậm.
- `VS3_W5_MIXED_SECS` — trần thời gian; mặc định 120 s. Khi chạy dài, chỉ dừng ở **biên batch**
  nên batch cuối luôn đủ 5 vòng.
- Soak 30 phút dùng `VS3_W5_MIXED_SECS=1800 VS3_W5_MIXED_ROUNDS=1000000`.

```bash
export PATH="$HOME/.rustup/toolchains/1.100.0-x86_64-unknown-linux-gnu/bin:$PATH"
VS3_W5_MIXED_SECS=1800 VS3_W5_MIXED_ROUNDS=1000000 \
  cargo test --release -p vis-cli --test vs3_worker_soak mixed_soak \
    -- --nocapture --test-threads=1
```

Mỗi vòng = **4 pha × 10.000 task**, W=1 và W=4 ⇒ **80.000 spawn/vòng**. Task xoay vòng 4 mode
theo `i % 4`:

| `i % 4` | API | Payload | Ghi chú |
|---:|---|---|---|
| 0 | `spawn_shared` | `map1k` (1.000 cặp `str→int`) | read-only `&SendValue` |
| 1 | `spawn_shared` | `list10k` (10.000 `int`) | read-only `&SendValue` |
| 2 | `spawn_worker_copy` | `list10k` (template `Arc`) | worker deep-clone rồi `push` 1 phần tử |
| 3 | `spawn_with_payload` | `list256` (`Int(7)`) | move; worker kiểm nội dung rồi `push` |

Bốn pha mỗi vòng:

| Pha | Nội dung | Bất biến kiểm |
|---|---|---|
| A. print + budget | 10k task in 1 dòng, `charge(1)`; budget `limit=N+1.000` | output W=1 == W=4 **byte-identical** và == chuỗi expected dựng từ mode; đủ 10k dòng; `used=10.000`, `remaining=1.000`, `used+remaining==limit`, không cạn; `Arc==1`; payload map/list/copy không đổi |
| B. budget cạn | 10k task `charge(1)`, `limit=N/2`, chunk 256 | `used=5.000`, `remaining=0`, `used+remaining==limit`, `exhausted`; tổng `budget_used` outcomes == `snap.used`; có task lỗi; `Arc==1`; payload không đổi |
| C. cancel | 10k task `check_cancel` trước/sau sleep 20 µs; huỷ handle chẵn trước `join` | join đủ 10k, không deadlock; W=1 huỷ **đúng 5.000**; W=4 `cancelled <= 5.000`; `Arc==1`; payload không đổi |
| D. cross-write | W=1 cộng dồn `counter`; W=4 10k task cùng `ctx.set("counter", …)` | W=1 `counter == 10.000`; W=4 `Err(CrossWrite { vars:["counter"], tasks:10k })`; `Arc==1`; payload không đổi |

**Batch = 5 vòng**. Mỗi batch kiểm lại đúng số check (print 10, budget 20, cancel 10, cross 10,
arc 40, payload 120), ghi RSS/threads/HWM, và log một dòng `MIXED_BATCH`; log từng vòng
`MIXED_ROUND`. Bất biến sai ⇒ panic dừng ngay. Sau 20 batch có thêm kiểm plateau: median RSS
10 batch cuối so median batch 6–15 phải tăng **< 64 MB**. Toàn bộ dừng nếu RSS tăng > 256 MB
hoặc thread rò (`threads > threads0 + 1`).

## 2. Kết quả soak 30 phút

Máy: **i5-1235U**, `nproc=12`, Linux, build **release**; load 1m ngoài tăng mạnh trong lúc chạy
(**~12,6 lúc bắt đầu → ~32,7 lúc kết thúc**), nên số vòng ít hơn ước tính.

| Chỉ số | Giá trị |
|---|---|
| Kết quả | **PASS** — `455/455` vòng, `91/91` batch đủ 5 vòng |
| Thời lượng | **1.813,54 s = 30 phút 13,5 s** (mục tiêu 1.800 s, dừng ở biên batch) |
| Tổng task | **36.400.000** spawn (455 × 80k) |
| Thời gian/vòng | min **2,088 s**, median **3,246 s**, max **12,189 s**, trung bình **3,986 s** |
| Thời gian/batch | min **11,55 s**, median **16,80 s**, max **39,56 s**, trung bình **19,93 s** |
| Pha (tổng) | print **487,02 s**, budget **393,92 s**, cancel **530,56 s**, cross **399,31 s** (~1.810,8 s; dư 2,7 s ngoài pha) |
| Check | print **910**, budget **1.820**, cancel **910**, cross **910**, arc **3.640**, payload **10.920** — khớp công thức mỗi batch |
| Deadlock | không; mọi `join` trả về, kể cả budget cạn và cross-write |
| Lỗi bất biến | **0** |

- Print merge: 455/455 vòng W=1 == W=4 **byte-identical** và khớp chuỗi expected của cả 4 mode;
  10.000 dòng/vòng, bắt đầu `sm:0:1000`, kết thúc `mv:9999:257`.
- Budget: A `used=10.000/remaining=1.000`; B `used=5.000/remaining=0/exhausted`; mọi vòng
  `used+remaining==limit`, tổng `budget_used` khớp.
- Cancel: join đủ 10k cả W=1/W=4; W=1 huỷ đúng 5.000/10.000 mỗi vòng.
- Cross-write: W=1 `counter=10.000`; W=4 luôn `Err(CrossWrite)` với đúng `vars=["counter"]`,
  `tasks=10.000`.
- Arc/payload: sau **mỗi** join (4 pha × 2 W), `Arc::strong_count` của map/list/template copy
  đều **1**; map còn 1.000 khoá, list10k còn 10.000 phần tử, copy template còn 10.000; nội dung
  `map[0]/map[999]`, `list[0]/list[9999]` không đổi. Task worker-copy chỉ sửa **bản clone**
  (10.000 → 10.001); move payload đúng 256 `Int(7)` rồi thành 257.

## 3. RSS / threads

| Mốc | RSS (KB) |
|---|---:|
| Baseline trước soak | **3.608** |
| Batch 1 | 62.492 |
| Batch 2 | 62.676 |
| Batch 4 | 63.084 |
| Batch 10 | 63.108 |
| Batch 20 | 63.368 |
| Batch 45 | **63.748** |
| Batch 90 | 63.432 |
| Batch 91 (cuối) | **63.432** |
| HWM đỉnh | **63.752** |
| RSS batch min / median / max | 62.156 / 63.748 / 63.748 |
| Tăng baseline → cuối | **+59.824 KB (~58,4 MB)**, dồn vào vòng đầu (allocator giữ output/outcome) |

- RSS **phẳng** sau vài batch: median batch 6–15 = **63.108 KB**, median 10 batch cuối =
  **63.432 KB** ⇒ tăng **+324 KB** qua ~360 vòng cuối, dưới xa ngưỡng 64 MB. Không thấy rò
  tuyến tính.
- Threads: **2 → 2**; cả 91 batch đều `threads=2`; đỉnh lấy mẫu khi W=4 đang chạy là **6**
  (2 baseline + 4 worker), sau join về 2. Không rò worker thread.
- Không có deadlock/timeout; vòng chậm nhất 12,2 s và batch chậm nhất 39,6 s đều do tải ngoài
  (load 1m lên ~33), không phải chờ vô hạn.

## 4. Lần chạy đầu — dừng ở 23,9 phút vì assertion test quá chặt

Lần đầu (cùng test, trước khi sửa một assertion) chạy **1.436,15 s** rồi panic ở vòng 373:

```
vòng 373: task huỷ W=4 phải trả Err
```

- Đã hoàn tất **373 vòng / 74 batch / 29,84 triệu task**; RSS **59.644 → 63.120 KB**, HWM
  **63.408 KB**, threads 2, peak 6 — cũng phẳng.
- Nguyên nhân: test tự đặt bất biến `cancelled == true ⇒ result.is_err()`. Với W>1, cờ huỷ có
  thể được đặt **sau `check_cancel` cuối** và trước khi outcome được tạo, nên task đã trả `Ok`
  nhưng `cancelled` vẫn `true`. Đây là race hợp lệ của cancel hợp tác, **không phải** rò hay
  deadlock; production không đổi.
- Sửa: bỏ assertion đó; giữ W=1 huỷ **đúng nửa** (kiểm ngữ nghĩa mạnh), W=4 chỉ yêu cầu
  `cancelled <= N/2`, join đủ, `Arc==1`, payload không đổi. Lần chạy 30 phút sau pass sạch.
- Bài học cho evaluator: `cancelled` là **yêu cầu huỷ tại thời điểm outcome**, không phải bằng
  chứng task đã dừng; muốn biết task dừng phải xét `result`.

## 5. Chất lượng / tái lập

```bash
# full suite cuối cùng (log trong vs3-w5-mixed-soak.log)
cargo test --release -p vis-cli --test vs3_worker --test vs3_worker_soak
# => 15 passed; 0 failed; 0 ignored — 10,33 s (gồm test mặc định 1 vòng)

cargo fmt --all -- --check                 # PASS
cargo clippy -p vis-cli --all-targets -- -D warnings   # PASS
```

- `git diff --stat`: **chỉ** `crates/vis-cli/tests/vs3_worker_soak.rs` (+794 dòng, tách khỏi `vs3_worker.rs`) và journal/log;
  `crates/vis-interp/src/rt/worker.rs` không đổi.
- Đĩa lúc chạy: trống **16 GB → 9,1 GB** (tải ngoài dùng thêm; worktree `target` chỉ **1,3 GB**,
  log 88 KB); vẫn còn **> 5 GB** trống, không có artifact >5 GB từ soak.
- Không GitHub Actions, không push/merge/tag.

## 6. Kết luận / giới hạn

- Qua **30 phút, 455 vòng, 36,4 triệu task**, hỗn hợp `spawn_shared` map1k/list10k,
  `spawn_worker_copy`, `spawn_with_payload`, W=1/W=4: **không rò RSS tuyến tính, không rò
  thread, không deadlock**, mọi bất biến print/budget/cancel/cross-write/Arc/payload đều giữ.
- Chỉ là mô hình `WorkerPool` test-only, chưa nối evaluator; chưa chứng minh cho workload thật.
- Một máy, tải ngoài cao và biến động (load 12→33); số vòng thấp hơn ước tính nhưng thời lượng
  đủ 30 phút. Cần chạy lại trên máy rảnh nếu muốn so tốc độ.
- **Blocker**: không. Hạn chế duy nhất là semantics `cancelled` nêu ở §4 (đã ghi để evaluator
  không dùng cờ này làm bằng chứng dừng task).
