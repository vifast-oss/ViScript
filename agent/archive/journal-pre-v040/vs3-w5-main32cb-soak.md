<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — soak hỗn hợp 10 phút + benchmark matrix trên main `32cb511`

**Trạng thái**: hardening **test-only** — không sửa production/ABI/default, không nối evaluator,
không bật default, **không** GitHub Actions.
**Mốc**: main `32cb511` (`[VS3] docs: async-h2 cold-start 300/300 + Set-Cookie`; gate 3538/3;
W5 mixed soak đã tách sang `crates/vis-cli/tests/vs3_worker_soak.rs`).
**Nhánh**: `vs3/w5-spike` trong worktree `.../worktree/cd45e5/vs3-w5-spike`;
`git merge main` **fast-forward** `10705eb → 32cb511`; **không** push/merge/tag.
**Commit**: xem `git log -1` — chỉ journal + 2 log, `crates/**` production không đổi.

**Log đầy đủ**:
- [`vs3-w5-main32cb-soak.log`](vs3-w5-main32cb-soak.log) — soak 10 phút + sampler fd + release
  tests + fmt/clippy/rules/df + kiểm chứng tách tệp.
- [`vs3-w5-main32cb-bench.log`](vs3-w5-main32cb-bench.log) — 5 run benchmark matrix release,
  kèm bảng tổng hợp median.

## 1. Soak hỗn hợp 10 phút (release)

```bash
export PATH="$HOME/.rustup/toolchains/1.100.0-x86_64-unknown-linux-gnu/bin:$PATH"
VS3_W5_MIXED_SECS=600 VS3_W5_MIXED_ROUNDS=1000000 \
  cargo test --release -p vis-cli --test vs3_worker_soak mixed_soak -- \
    --nocapture --test-threads=1
```

Máy: **i5-1235U**, `nproc=12`, Linux 7.0.0-43, build release; load 1m **10,57** lúc bắt đầu,
**5,91** lúc kết thúc (5m 15,57 → 8,72; 15m 16,05 → 12,25). Swap **4.095/4.095 MB đã đầy**
từ trước khi chạy; RAM available ~8,4 GB. Đĩa trống **12 GB** (> 5 GB).

| Chỉ số | Giá trị |
|---|---|
| Kết quả | **PASS** — `1 passed; 0 failed; 0 ignored` |
| Thời lượng | **603,30 s** (mục tiêu 600 s; dừng ở biên batch) |
| Batch / vòng / task | **31 batch**, **155 vòng**, **12.400.000** spawn (155 × 80k) |
| Thời gian/vòng | min **2,570 s**, median **3,531 s**, max **8,966 s**, TB **3,892 s** |
| Thời gian/batch | min **15,97 s**, median **17,73 s**, max **36,41 s**, TB **19,46 s** |
| Pha (tổng) | print **162,23 s**, budget **134,20 s**, cancel **169,95 s**, cross **136,79 s** |
| RSS | 3.360 → **62.700 KB**; min **60.412**, median **62.700**, max **62.808 KB** |
| HWM | 3.364 → peak **63.392**, cuối **62.872 KB** |
| Threads | 2 → **2**; peak sample **6**, peak batch **3** — không rò |
| **fd** | **3 hằng định** (sampler 2 s × 290 mẫu; min = avg = max = 3) |
| Check | print **310**, budget **620**, cancel **310**, cross **310**, arc **1.240**, payload **3.720** — khớp công thức |
| Deadlock / lỗi bất biến | **không / 0** |

- Print merge 155/155 vòng W=1 == W=4 byte-identical; budget A `used=10.000/remaining=1.000`,
  B `used=5.000/remaining=0/exhausted`; cancel join đủ 10k, W=1 huỷ đúng 5.000; cross-write
  W=1 `counter=10.000`, W=4 luôn `Err(CrossWrite { vars:["counter"], tasks:10.000 })`.
- **fd** không có trong log tự thân của test; sampler ngoài đọc `/proc/<pid>/fd` của đúng tiến
  trình `out/vs3_worker_soak-*` mỗi 2 s (290 mẫu) — luôn **3** (stdin/stdout/stderr), không rò.
- Tốc độ **0,257 vòng/s** (155/603,3) so với soak 30 phút cũ **0,251 vòng/s** (455/1.813,54)
  trên `cadc4c8`; RSS cùng mức (~62,7 vs ~63,4 MB) ⇒ hành vi/throughput không đổi sau khi merge.

## 2. Tách tệp — xác nhận không đổi hành vi

Commit tách trên main: **`f90d6d0`** `[VS3][W5-fix] tach mixed soak sang vs3_worker_soak.rs
(R1 1228+794 dong)`.

- Khối mixed soak từ `const MIXED_N` đến hết tệp: bản cũ
  (`f90d6d0^:crates/vis-cli/tests/vs3_worker.rs`) **680 dòng**, bản mới
  (`crates/vis-cli/tests/vs3_worker_soak.rs`) **680 dòng**, `diff` **rỗng** (`DIFF_EXIT=0`)
  ⇒ di chuyển **nguyên văn**, không sửa logic.
- md5 tệp soak tại `f90d6d0` **bằng** hiện tại: `c10d927c813e303facf241145d2b8488`
  ⇒ không có chỉnh sửa nào sau khi tách.
- Kích thước sau tách: `vs3_worker.rs` **1.228** dòng + `vs3_worker_soak.rs` **794** dòng;
  R1 ≤ 1.500 dòng/tệp.
- `cargo test --release -p vis-cli --test vs3_worker --test vs3_worker_soak`:
  **14 passed** (vs3_worker) + **1 passed** (soak mặc định 1 vòng, 5,41 s), **0 failed**,
  **0 ignored** — tổng 15 test như trước khi tách.
- Soak 10 phút ở §1 chạy đúng bản đã tách: không panic, không deadlock, mọi bất biến giữ.

## 3. Benchmark matrix release — 5 run

```bash
cargo build --release -p vis-cli --example vs3_worker_bench
./target/release/examples/vs3_worker_bench
```

5 run liên tiếp, cùng máy; load 1m đầu mỗi run **5,06–5,33** (5m 8,04–8,38);
calibration **200,0–240,0 µs/task** (200 µs mục tiêu); cả 5 run `EXIT=0`, mọi `CHECK` OK,
mọi dòng `used=6.400 / remaining=10.000 / limit=16.400`.

### 3.1 So với mốc cũ (`vs3-w5-spawn-shared.md`, load 15,9–17,5, calib 200–236 µs)

| Chỉ số (W=4) | Cũ | **Mới median [min..max]** | Δ |
|---|---:|---:|---:|
| `MATRIX_SHARED` map1k | 5,62 ms | **4,08 [3,59..5,37]** | **−27,4 %** |
| `MATRIX_SHARED` list10k | 5,25 ms | **4,08 [3,55..5,47]** | **−22,3 %** |
| `MATRIX` map1k (producer) | 24,63 ms | **28,27 [25,71..28,48]** | **+14,8 %** |

- Shared map1k/list10k **nhanh hơn** số cũ; đây là mode nên dùng cho payload lớn/read-only.
  Mức nhanh hơn nằm trong biên nhiễu giữa các run (xem cột min..max), không quy cho thay đổi code.
- Producer map1k **chậm hơn 14,8 %** so với số cũ, nhưng: (a) mode producer là worst-case
  clone tuần tự trên producer, (b) 3 run cũ dao động **18,55–33,58 ms** (median 24,63),
  (c) nhánh này **không đụng production** — `git diff main..HEAD` chỉ có journal/log.
  Vì vậy **không kết luận hồi quy code**; coi là nhiễu tải/tần số CPU. Cần đo lại khi máy
  thật sự rảnh nếu muốn chốt số producer.
- So sánh đúng trong cùng run: shared **4,08 ms** vs worker-copy **5,74 ms** vs producer
  **28,27 ms** (map1k W4) — shared thắng rõ.

### 3.2 Bảng W4 đầy đủ (median 5 run, ms)

| Payload | Producer | Worker-copy | **Shared** |
|---|---:|---:|---:|
| unit | 4,06 | 4,42 | 4,23 |
| list1k | 4,42 | 4,59 | 4,26 |
| **list10k** | 8,39 | 6,94 | **4,08** |
| **map1k** | 28,27 | 5,74 | **4,08** |
| record | 4,88 | 4,63 | 4,05 |
| enum | 4,70 | 4,62 | 4,06 |
| str1MiB | 9,16 | 6,40 | 4,24 |

- Payload nhỏ (`unit`/`list1k`/`record`/`enum`) ba mode tương đương ~4–5 ms; shared không lợi
  vì overhead Arc/closure.
- `str1MiB`: shared 4,24 ms vs producer 9,16 và worker-copy 6,40 — shared rõ lợi.
- `PROBE` map1k W4: producer 28,08 / 27,50 / 29,57 / 24,72 / 24,33; shared 4,54 / 4,41 /
  3,61 / 4,10 / 5,72 — cùng xu hướng với matrix.

## 4. Test / fmt / clippy / rules / đĩa

| Kiểm | Kết quả |
|---|---|
| `cargo test --release -p vis-cli --test vs3_worker --test vs3_worker_soak` | **14 + 1 passed**, 0 failed, 0 ignored (11,36 s + 5,41 s) |
| `cargo fmt --all -- --check` | **PASS** |
| `cargo clippy -p vis-cli --all-targets -- -D warnings` | **PASS** (33,25 s) |
| `./vis-dev rules` | **PASS** — `--- rules: OK` |
| `df -h /home /tmp` | `/` trống **12 GB** (95 % dùng), `/tmp` trống **5,8 GB** — còn **> 5 GB** |
| Artifact > 5 GB | **không**; `target` **551 MB**, 2 log **52 KB + 28 KB** |

Không chạy `gate --fast`/nightly vì **không có thay đổi mã sản phẩm**; leader chạy gate trên
commit merge nếu cần.

## 5. Hồi quy / blocker

- **Hồi quy chức năng**: **không**. Soak 10 phút pass sạch; 15 release test pass; fmt/clippy/rules
  pass; tách tệp là di chuyển nguyên văn (diff rỗng, md5 khớp).
- **Tín hiệu cần theo dõi**: producer map1k W4 +14,8 % so với số cũ, nhưng nằm trong biên nhiễu
  lịch sử 18,55–33,58 ms và branch không đổi production ⇒ **chưa phải hồi quy**; nếu cần số
  producer chuẩn, đo lại 5 run khi load < 2.
- **Blocker**: **không**.
- **Cảnh báo môi trường**: swap đầy 4.095/4.095 MB và đĩa 95 % (12 GB trống) — không ảnh hưởng
  lần chạy này nhưng nên dọn trước các soak dài hơn.
- Không push/merge/tag; không GitHub Actions; không sửa `crates/**` production/ABI/default.
