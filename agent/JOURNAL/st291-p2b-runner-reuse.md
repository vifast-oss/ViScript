<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-291 P2b-3/P2b-4 — completion slot / runner reuse: **NO-GO** (đo + thiết kế)

**Nhánh**: `plan/w1-p2b-runner-reuse` từ `main` **`f4cb136`**. Không merge/push/tag.
**Kết luận**: **không triển khai production** trong time-box 3 h — không đạt ≤1,5× W=1 và
không có đường an toàn trong phạm vi; giữ nguyên code main. 0 đổi API/ABI/`Value`/default,
0 `unsafe`.

## 1. Baseline (release, N=10k, best-of-8, load 3,5–5,8)

| Chỉ số | W=1 | W=4 | Tỉ lệ |
|---|---:|---:|---:|
| batch interp | **6,4–6,8 ms** | **25,7 ms** | ~4,0× |
| batch VM | **5,6–5,8 ms** | **23,0 ms** | ~4,0× |
| reps=5 (một lần) interp | 8,5 | 33,0 | 3,9× |
| reps=5 VM | 7,4 | 31,6 | 4,3× |

Mục tiêu 1,5× ⇒ W=4 phải ~9–10 ms; khoảng cách cần cắt **~15 ms/10k** (1,5 µs/task).

## 2. Phân rã chi phí (cùng máy, reps=5)

| Thành phần | Số đo |
|---|---:|
| Pool W=4, không board, không evaluator | **15,5 ms** |
| Pool W=4 + outcome board | 17,0 ms (**board +1,6 ms**) |
| Pool W=4 + evaluator (VM mới/task) | 13,8 ms |
| Pool + VM **reuse** thread-local (bench cũ) | 13,4 ms (**1,03×**, gần như 0) |
| `Vm::new` | **0,14 µs/task** (1,4 ms/10k) |
| `vis_interp::run` (dựng Interp + chạy rỗng) | **0,76 µs/task** (7,6 ms/10k) |

Nguồn 15 ms còn lại của batch so với W=1 là **envelope worker**: `Task`/proxy/pending,
snapshot/args SendValue, `WorkerCtx` + `BudgetLease` + `TaskOutput` mỗi task, channel
chunk/outcome và join — không phải constructor runner.

## 3. Đánh giá P2b-3 / P2b-4

- **P2b-3 (completion slot)**: board hiện tốn **1,6 ms/10k** (publish+wait). Thay HashMap
  bằng slot vec chỉ cắt tối đa ~1,6 ms ⇒ W=4 còn ~24 ms, vẫn **~3,6× W=1**. Không đủ.
- **P2b-4 (runner reuse)**: constructor `Vm` 0,14 µs/task; `Interp` 0,76 µs/task gồm cả chạy
  rỗng. Đo trực tiếp pool + VM reuse cho **1,03×** (không đáng). Reuse `Interp` an toàn còn
  vướng `Interp<'a>` borrow + reset `env/sched/out/steps/top_syms`; muốn dùng phải đổi
  ownership/lifetime hoặc thêm `reset_for_task`, và vẫn chỉ cắt ~1–7 ms tuỳ chương trình.
- **Đường duy nhất có thể tiến gần 1,5×**: chunk-local evaluator (một `Interp`/worker cho cả
  chunk, reset giữa task) **+** envelope per-task nhẹ (slot outcome, không `WorkerCtx` đầy đủ
  cho task CPU thuần). Đây là redesign P2b-4b > 3 h, đụng cancel/outcome/frame isolation và
  budget lease; rủi ro cao, không nên ép trong ST này.

## 4. Quyết định

**Dừng trước merge.** Không sửa production; chỉ journal + log đo. Đề xuất mở task riêng
(P2b-4b) với thiết kế chunk-local runner + envelope nhẹ, kèm test cancel/outcome/frame và
đo lại; nếu không có nhu cầu task cực nhỏ thực tế thì giữ P2b hiện tại (đã cải thiện
45→25–30 ms so với trước ST-291).

Log đo: `agent/JOURNAL/st291-p2b-reuse-baseline.log` (2,5 KB).

## 5. Cổng

- `./vis-dev gate --fast` ⇒ **PASS**: fmt/clippy/R1..R7/self-test/build/doc-examples OK;
  L0 **3660 passed / 4 skipped** (log ngoài repo `/home/ps/vislogs/st291-p2b-reuse-gate.log`,
  sha256 `4f081e76…a00261` — R7/VD-282).
- Không chạy diff 100k: **0 dòng production thay đổi** (nhánh chỉ thêm journal/log đo).
