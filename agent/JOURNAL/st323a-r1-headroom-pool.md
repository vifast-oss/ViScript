<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-323a — R1 headroom: tách `worker/pool.rs` (board/ctx)

- **Ngày**: 2026-11-28 · **Vai**: Builder (Agent 2) · **Nhánh**: `plan/st323a`
- **Nền**: `main` @ `eb41d61c3ef77a6c8384d2f13be40f189ce2676b`
- **Worktree**: `/home/ps/viswt/st323a` — **không** chạm repo chính (agent khác build song song), **không merge/push**.
- **Trạng thái**: mã + test + cổng **PASS**; chờ Gatekeeper audit.
- **Nguồn đề xuất**: `/home/ps/vislogs/r1-headroom-next-20261127T034711Z.md` (sha `b39aa0f6…`), mục 3 (đề xuất rẻ nhất) / §6.

## 0. TL;DR

`worker/pool.rs` 962 dòng → **645** + `worker/board.rs` **205** + `worker/ctx.rs` **133**.
Chuyển **nguyên văn** item dòng 76–271 (`TaskWaitBoard`/`WaitState`/`WaitError`/`OutcomeBoard`/
`OutcomeState` + impl) và dòng 272–392 (`WorkerCtx` + impl), thêm barrel trong `worker/mod.rs`.
API công khai `vis_runtime::worker::*` **không đổi** (67 dòng `pub …` giữ nguyên tên/chữ ký).

| Tệp | Trước | Sau |
|---|---:|---:|
| `crates/vis-runtime/src/worker/pool.rs` | 962 | **645** |
| `crates/vis-runtime/src/worker/board.rs` | — | **205** |
| `crates/vis-runtime/src/worker/ctx.rs` | — | **133** |
| `crates/vis-runtime/src/worker/mod.rs` | 24 | 31 |

## 1. Thay đổi

- **`worker/board.rs`** (mới): nguyên văn dòng 76–271 của file cũ. `use` riêng:
  `HashMap`, `{Condvar, Mutex}`, `{Duration, Instant}`, `super::pool::TaskOutcome`,
  `super::proto::{SendValue, TaskId}`.
- **`worker/ctx.rs`** (mới): nguyên văn dòng 272–392. `use` riêng: `BTreeSet`,
  `{AtomicBool, Ordering}`, `Arc`, `crate::rt::fee::{BudgetError, BudgetLease, SharedBudget}`,
  `super::pool::{TaskOutcome, TaskOutput}`, `super::proto::{EnvMap, SendValue, TaskId}`.
- **`worker/pool.rs`**: giữ dòng 1–75 + 394–962; bỏ `use` không còn dùng (`Condvar`, `Mutex`,
  `Duration`, `Instant`, `BudgetError`, `BudgetLease`), thêm `super::board::{OutcomeBoard,
  TaskWaitBoard}` + `super::ctx::WorkerCtx`.
- **`worker/mod.rs`**: thêm `mod board; mod ctx;` + `pub use board::*; pub use ctx::*;`;
  giữ nguyên `mod orchestrate/pool/proto` và 3 `pub use` cũ + doc giải thích ST-323a.

**Điểm khác duy nhất so với nguyên văn**: 3 phương thức private của `WorkerCtx` nâng lên
`pub(super)`: `new`, `globals`, `into_outcome` (bắt buộc vì module cha `pool` không gọi được
`fn` riêng của module con `ctx`; `pub(super)` chỉ mở trong `worker`, **không** lọt ra ngoài
crate). Thân hàm, chữ ký và ngữ nghĩa không đổi; không đổi `unsafe`/`TODO`.

## 2. Kiểm chứng move nguyên văn / API

Script tái lập `/home/ps/vislogs/tmp/st323a-split.py` + `/home/ps/vislogs/tmp/st323a-verify.py`.
Đối chiếu `difflib` với `git show HEAD:crates/vis-runtime/src/worker/pool.rs`:

```
board: ratio=1.000000, diffs=0
ctx:   ratio=1.000000, diffs=0
pool:  ratio=1.000000, diffs=0
API pub lines: 67 (giống hệt, không đổi tên/chữ ký)
VERIFY OK
```

(Chuẩn hoá: bỏ block `use` thay thế và 1 dòng trống cuối file bị `cargo fmt` xoá.)

Không có tệp nào dùng path `worker::pool::WorkerCtx` / `pool::OutcomeBoard` trong crate;
`orchestrate.rs` và `rt/exec_state.rs` vẫn dùng `crate::worker::{…}` qua barrel.

## 3. Cổng

Mọi lệnh chạy trong worktree, `PATH=/home/ps/.cargo/bin`, `TMPDIR=/home/ps/vislogs/tmp`
(đĩa, không tmpfs), `CARGO_BUILD_JOBS=2`, **không** `cargo clean`.
Disk `/`: **16 G** available (> ngưỡng dừng 5 G).

| Lệnh | Kết quả | Log |
|---|---|---|
| `cargo fmt --all` | rc 0, chỉ đổi 4 tệp worker | `st323a-fmt.log` |
| `cargo test -p vis-runtime -p vis-cli -p vis-conformance` | **PASS** — 2541 passed / 0 failed / 3 ignored | `st323a-test.log` |
| `./vis-dev gate --fast` | **PASS rc 0** (lần 2) — fmt/clippy/R1..R10/DAG/self-test/build/doc-examples + L0 **3942/3942** | `st323a-gate-retry.log` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **lech=0** — 100.000 chương trình, 7 test PASS (179.9 s) | `st323a-diff.log` |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS rc 0** (47.22 s, 0 warning) | `st323a-clippy.log` |

Ghi chú cổng: lần 1 `gate --fast` (`st323a-gate.log`) fail ở L0 do test flaky unrelated
`vis-cli::cli_it repl_term::format_la_la_loi_dung_exit_2`: `BrokenPipe` khi `write_all` vào
stdin của tiến trình `vis repl` đã thoát (mọi bước `check` trước đó đều OK). Cùng test đã xanh
trong `cargo test` (523/523 `cli_it`) và trong lần chạy lại gate; không đụng worker hay subprocess
nào của pool.

## 4. Luật

- **R1**: `pool.rs` **645 < 1000** (điểm mềm nội bộ); 0 tệp > 1500 dòng.
- **R8**: tổng `.rs` cấp 1 trong `crates/*/tests/` + `vis-host-linux/tests/` = **39/40**; riêng
  `crates/vis-cli/tests/` = **8/15**. **0 tệp test cấp 1 mới**.
- `grep -E 'unsafe|TODO'` trên 3 tệp worker: **NONE** (không có unsafe/TODO mới).
- `git status` trước commit: `M worker/mod.rs`, `M worker/pool.rs`, `?? worker/board.rs`,
  `?? worker/ctx.rs`, `??` journal — không đụng `agent/QUEUE.md`/`STATE.md`/`DECISIONS.md`.

## 5. Kết luận

Di chuyển thuần tuý, 0 đổi hành vi; cổng máy tái lập được **PASS**, `lech=0`, API `worker::*`
giữ nguyên. Không merge, không push. Chờ Gatekeeper (Agent 1) audit rồi A0 duyệt.
