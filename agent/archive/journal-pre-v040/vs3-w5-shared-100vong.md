<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — 100 vòng stress **release-mode** cho `spawn_shared` read-only

**Trạng thái**: hardening **test-only** — không sửa production/ABI/default, API cũ giữ nguyên.
Không nối evaluator, không bật default, **không** GitHub Actions.
**Mốc**: main `0c911b3` (đã merge `spawn_shared` opt-in); tag `v0.3.3` = `bbf3484`.
**Nhánh**: `vs3/w5-spike` · `git merge main` fast-forward `976bcde → 0c911b3`; **không** push/merge/tag.
**Commit**: chỉ `crates/vis-cli/tests/vs3_worker.rs` + journal/log này; `crates/**` production không đổi.
**Log đầy đủ**: [`vs3-w5-shared-100vong.log`](vs3-w5-shared-100vong.log) — 100 vòng + full suite + fmt/clippy.

## 1. Phạm vi và cách chạy

Thêm một test + helper vào `crates/vis-cli/tests/vs3_worker.rs`:

- Test `spawn_shared_stress_rounds_read_only`.
- Số vòng lấy từ `VS3_W5_SHARED_ROUNDS`; **mặc định 10** để L0/gate không chậm, bản hardening
  chạy **100** đúng yêu cầu. Không `#[ignore]`, không đổi test cũ.
- Mỗi vòng dùng `N = 10.000` task/pha. Payload **xen kẽ**: vòng chẵn `map1k` (1.000 khoá
  `str→int`), vòng lẻ `list10k` (10.000 `int`). Một vòng có 6 pha × 10k = **60k spawn**;
  100 vòng = **6.000.000 spawn**.

```bash
VS3_W5_SHARED_ROUNDS=100 \
  cargo test --release -p vis-cli --test vs3_worker spawn_shared_stress \
    -- --nocapture --test-threads=1
```

Các pha và kiểm tra **mỗi vòng**:

| Pha | W | Nội dung | Kiểm tra |
|---|---:|---|---|
| A. in + budget | 1, 4 | 10k `spawn_shared` đọc payload, `print_line`, `charge(1)` | output W=1 == W=4 **byte-identical**, đủ 10k dòng/đầu/cuối; `used=10.000`; `used+remaining==limit`; `remaining=1.000`; không `exhausted`; `Arc::strong_count==1`; payload không đổi |
| B. cancel | 1, 4 | 10k `spawn_shared`, huỷ handle chẵn trước join; task gọi `check_cancel` | join đủ 10k, không deadlock; W=1 huỷ **đúng 5.000** (task chẵn chưa chạy); W=4 `cancelled <= 5.000`; `Arc==1`; payload không đổi |
| C. cross-write | 4 | 10k `spawn_shared` cùng `ctx.set("counter", …)` | vẫn `Err(CrossWrite { vars:["counter"], tasks:10k })`; `Arc==1`; payload không đổi |
| D. budget cạn | 4 | 10k task, pool `limit=N/2`, chunk 256 | `used=5.000`, `remaining=0`, `used+remaining==limit`, `exhausted`; có task lỗi; `Arc==1`; payload không đổi |

Cứ **10 vòng** (10 checkpoint) đo median **shared / worker-copy / producer** trên **cả
map1k và list10k**, W=1/W=4, 256 task, 10 charge/task, CPU hiệu chỉnh ~100 µs/task, 3 lần lặp
lấy median mỗi ô. Không lặp toàn ma trận payload (unit/record/enum/str1MiB) — ma trận đó đã có
ở journal `vs3-w5-spawn-shared.md`.

Máy: **i5-1235U** (2 P-core + 8 E-core), `nproc=12`, Linux; build release mới trong worktree.
Load 1m lúc chạy: **13,11–18,47** (hệ thống có tải ngoài suốt thời gian đo).

## 2. Kết quả 100 vòng

Đã chạy **hai lần 100 vòng**, cả hai **100/100 pass**. Log đầy đủ là lần 2 (đúng code commit,
có thêm dòng `rss_plateau_kb`):

| Lần | Load 1m | Tổng (s) | Secs/vòng median | Min–max (s) | Kết quả |
|---|---:|---:|---:|---:|---|
| 1 | 13,11 | **37,27** | **0,209** | 0,108–0,314 | 100/100 pass |
| 2 (log) | 14,97 → 18,47 | 57,48 | 0,342 | 0,096–0,718 | 100/100 pass |

- **Print merge**: 100/100 vòng, mỗi vòng 10.000 dòng, W=1 == W=4 **byte-identical**.
- **Budget**: 100/100 vòng A đạt `used=10.000`, `remaining=1.000`, `used+remaining==limit=11.000`;
  100/100 vòng D đạt `used=5.000`, `remaining=0`, `used+remaining==limit=5.000`, `exhausted=true`.
- **Cancel**: 100/100 vòng W=1 huỷ đúng 5.000/10.000; W=4 `cancelled <= 5.000`; join đủ 10k,
  không deadlock.
- **Cross-write**: 100/100 vòng W=4 trả `Err(CrossWrite)` với `vars=["counter"]`, đủ 10.000 task id.
- **Arc**: 100/100 vòng, sau **mỗi** join (A/B/C/D và benchmark) `Arc::strong_count(payload)==1`.
- **Payload**: 100/100 vòng map1k còn 1.000 khoá, list10k còn 10.000 phần tử; spot-check
  `map[0]/map[999]`, `list[0]/list[9999]` không đổi.

## 3. So sánh mỗi 10 vòng (median 10 checkpoint)

Lần 2 (log, load 14,97→18,47):

| Payload | W | Shared (ms) | Worker-copy (ms) | Producer (ms) | Shared vs producer |
|---|---:|---:|---:|---:|---:|
| map1k | 1 | **26,17** | 57,77 | 64,07 | **2,45×** |
| map1k | 4 | **19,86** | 40,66 | 74,35 | **3,74×** |
| list10k | 1 | **21,67** | 88,63 | 190,04 | **8,77×** |
| list10k | 4 | **17,48** | 54,92 | 62,99 | **3,60×** |

Lần 1 (load 13,11) — cùng code trừ dòng summary plateau:

| Payload | W | Shared (ms) | Worker-copy (ms) | Producer (ms) | Shared vs producer |
|---|---:|---:|---:|---:|---:|
| map1k | 1 | **25,70** | 44,28 | 45,07 | **1,75×** |
| map1k | 4 | **8,22** | 14,81 | 88,55 | **10,77×** |
| list10k | 1 | **23,96** | 66,65 | 132,91 | **5,55×** |
| list10k | 4 | **8,51** | 25,68 | 34,32 | **4,03×** |

- **Shared nhanh nhất ở cả 4 tổ hợp, cả hai lần chạy**; worker-copy ở giữa; producer nặng nhất
  với payload lớn.
- Shared vs worker-copy: 1,7–2,2× (map1k), 2,8–4,1× (list10k). Bỏ được deep clone trong worker.
- W=4/W=1 của shared: lần 1 **3,13×** (map1k) / **2,82×** (list10k); lần 2 chỉ 1,32×/1,24× vì
  load tăng lên ~18. Số tuyệt đối phụ thuộc tải; so sánh đúng là **trong cùng checkpoint**.
- Producer W=4 map1k **không ổn định** (52–129 ms qua các checkpoint), có lúc chậm hơn W=1:
  producer deep-clone 256 map (256k `String`) trong khi 4 worker chạy CPU trên 2 P-core dưới tải
  cao. Đây là quan sát tải/allocator, **không phải** hồi quy của `spawn_shared`; cần máy rảnh
  (hoặc tách producer khỏi worker) mới kết luận. Không dùng ô này để tuyên bố speedup.
- 120 dòng `SHARED_BENCH10` (10 checkpoint × 2 payload × 2 W × 3 mode) nằm trong log.

## 4. RSS / threads / rò

Lần 2 (log):

| Mốc | KB |
|---|---:|
| RSS trước chạy | 3.324 |
| RSS các vòng 0–9 | 8.856–10.960 |
| RSS vòng 10 (checkpoint benchmark đầu) | 55.744 |
| **RSS plateau (median vòng 10–19)** | **55.744** |
| **RSS 10 vòng cuối (90–99)** | **55.980** |
| RSS sau chạy | 55.992 |
| **HWM toàn test** | **193.828** |
| Threads sau mỗi vòng | **2** (một mẫu chớp 3) |
| Threads peak khi W=4 | **6** (main + 4 worker + harness) |
| Threads sau chạy | **2** |

- **Không rò thread**: `threads0=2 → threads1=2`; peak 6 đúng W=4, về 2 sau mỗi pool.
- RSS nhảy một lần ở vòng 10 (checkpoint benchmark) rồi **đi ngang** 90 vòng:
  plateau 55.744 KB → cuối 55.980 KB, **+236 KB**. Lần 1: 55.804 → 56.024 KB, **+220 KB**.
  Đây là allocator giữ buffer benchmark (256 payload deep-clone ở mode producer), **không phải rò**;
  nếu rò theo vòng, RSS sẽ tăng đơn điệu qua 100 vòng.
- HWM 193 MB cũng là đỉnh nhất thời của benchmark producer (256 × map1k/list10k clone trong
  queue W=1), không phải mức sống của stress (stress ~9–11 MB, sau benchmark plateau ~56 MB).
- Assertion rò: `threads1 <= threads0+1`, `rss1-rss0 < 256 MB`, `rss 10 vòng cuối - plateau < 64 MB`.
- RSS/threads **sau từng vòng** in ở dòng `SHARED_STRESS round=…`; peak/HWM trong dòng summary.
- Đây là RSS của **test process**, không phải cap production.

## 5. Full suite / fmt / clippy / đĩa

- `cargo test --release -p vis-cli --test vs3_worker`: **14 passed / 0 failed / 0 ignored**,
  **10,50 s** (load ~14,6; gồm test mới chạy mặc định 10 vòng + 13 test cũ).
- `cargo fmt --all -- --check`: **PASS**.
- `cargo clippy -p vis-cli --all-targets -- -D warnings`: **PASS** (19,64 s).
- `df /home`: **16 GB free** (>5G) trước và sau; không GitHub Actions.
- `git status`: chỉ `crates/vis-cli/tests/vs3_worker.rs` + journal/log; **không** đụng
  `worker.rs`/`Value`/bytecode/cache/ABI/default.

## 6. Blocker / giới hạn

- **Load cao 13–18,5** và máy hybrid 2P+8E: ms tuyệt đối và W=4/W=1 thay đổi mạnh giữa hai lần
  chạy. Chỉ nên đọc so sánh shared/worker/producer **trong cùng checkpoint**; cần máy rảnh để
  chốt số hiệu năng.
- **Mặc định 10 vòng** (không phải 100) để gate không tăng ~40–60 s; muốn 100 vòng phải đặt
  `VS3_W5_SHARED_ROUNDS=100`. Nếu A0 muốn mặc định 100, đổi một dòng nhưng phải chấp nhận gate chậm.
- **Producer W=4 map1k** dao động lớn (52–129 ms, có lúc chậm hơn W=1); nguyên nhân là deep clone
  trên producer cạnh tranh CPU/allocator với 4 worker dưới tải, chưa tách bạch được trong vòng này.
- **Chưa nối evaluator**: vẫn closure + `SendValue`, chưa phải `async fn`/frame/VM thật.
- **Read-only**: task chỉ nhận `&SendValue`; muốn sửa payload phải worker-copy/move/COW. Arc payload
  phải `Send + Sync`; evaluator vẫn phải chặn closure/handle/Task/cap bằng `VS0508` trước khi bọc Arc.
- **Chưa soak 24h**: 100 vòng ~1 phút (release) đủ bắt rò cấp vòng/thread, chưa phải soak dài.
- **Không tuyên bố production/default**; không đổi `Value`/bytecode/cache; không đo Windows/macOS.
- Không push/merge/tag; không chạy GitHub Actions.

## 7. Tái lập

```bash
cd /home/ps/.local/share/opencode/worktree/cd45e5/vs3-w5-spike
export PATH="$HOME/.cargo/bin:$PATH"          # cargo/rustc 1.100.0

# 100 vòng hardening (release)
VS3_W5_SHARED_ROUNDS=100 \
  cargo test --release -p vis-cli --test vs3_worker spawn_shared_stress \
    -- --nocapture --test-threads=1

# Full suite release (test mới chạy mặc định 10 vòng)
cargo test --release -p vis-cli --test vs3_worker

cargo fmt --all -- --check
cargo clippy -p vis-cli --all-targets -- -D warnings
```

## 8. Nguồn

- `AGENTS.md` §1–§4; `agent/DECISIONS.md` VD-271 (VS3-3b), VD-160 (heap riêng), VD-178 QĐ-1
  (Sendable/VS0508).
- [`vs3-w5-spawn-shared.md`](vs3-w5-spawn-shared.md) — API `spawn_shared` + benchmark 7 payload.
- [`vs3-w5-50vong-release.md`](vs3-w5-50vong-release.md) — mẫu stress release + RSS/thread.
- [`vs3-w5-spike.md`](vs3-w5-spike.md) — chi phí copy/move/Arc và ngưỡng W=4.
- Code: `crates/vis-interp/src/rt/worker.rs` (`spawn_shared`, `join`, `WorkerCtx`),
  `crates/vis-interp/src/frame/fee.rs` (`SharedBudget`), `crates/vis-cli/tests/vs3_worker.rs`.
