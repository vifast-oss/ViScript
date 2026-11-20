# VD-249 / ST-263 — #148 contract check + targeted lifecycle measurement

- **Date**: 2026-11-20 (UTC), **HEAD** `812feda430b1b9394bb1a5adf2c2c4396675072c`
- **Authority**: VD-249 §5 (#148) and §6 (supplementary measurement); task brief ST-263.
- **Repo state**: `crates/vis-interp/src/net.rs` **clean** (no product code changed by this task);
  `agent/DECISIONS.md` and `agent/QUEUE.md` were already modified by other agents — untouched here.
- **PATH**: `export PATH="$HOME/.cargo/bin:$PATH"` used for all commands.

---

## PART 1 — #148: NO CODE CHANGE; contract conflict escalated to A0

### 1.1 Required pre-check → result: a different meaning IS already chốt

The brief and VD-249 §5 both require: *if the existing `timeout` parameter carries a meaning
already chốt by SPEC that differs from "total request deadline", do not silently change it —
present the contract change to A0 first.* That condition is **true**. Evidence:

| # | Source (verbatim) | Meaning of the existing `timeout` parameter |
|---|---|---|
| 1 | `agent/DECISIONS.md:1103` — VD-183 Q2(1): "`timeout_read = 30 s` (**giữa hai lần đọc**, **không** phải tổng)" | A0-chốt: **between two reads**, explicitly **not** total |
| 2 | `agent/DECISIONS.md:1103` — VD-183 Q2(6): "**Mở nợ mới** cho **hạn chót tổng** của một request (xem #148)" | A0-chốt: total deadline is a **separate, new** thing (#148), not a redefinition of `timeout_read` |
| 3 | `agent/SO-NO.md:236` — #148: "cần tham số `timeout_total` (đề xuất 120 s) + ca đỏ-trước slow-drip" | Debt entry expects a **new** `timeout_total` parameter |
| 4 | `agent/STATE.md:525` (ST-104 journal): "timeout **mỗi lần đọc** 2 s (tổng request có thể lâu hơn với `get_timeout` lớn)" | Documented current semantics: **per-read**, total may be longer |
| 5 | `agent/STATE.md:522`: "mỗi lần đọc cap **2000 ms** (và `get_timeout` nhỏ hơn)" | per-read cap 2 s |
| 6 | `docs/ViScript-thiet-ke.md:1618`: "HTTP/1.1 loopback … HEAD, timeout đọc" | read timeout |
| 7 | `agent/CHO-A0-QUYET-DINH-2026-11-18.md:163-164`, `agent/JOURNAL/vs3-qd-3.md:14-15`: `timeout_connect` 10 s / `timeout_read` 30 s (proposal → chốt VD-183) | same chốt, two conceptual knobs |
| 8 | `docs/SPEC-lex.md`, `SPEC.md` (absent in this split repo), `crates/vis-interp/src/stdlib.rs:592-625`: `http.get` / `http.get_timeout` expose **one** `int` timeout (default 5000 ms) | the single public `timeout` is the parameter in question |

**Conclusion of the pre-check**: the `timeout` argument of `http.get_timeout` / `http.head_timeout`
has a chốt, documented meaning of *between two reads, not total* (VD-183). Redefining it as
*one deadline for the whole call* is a **contract change**, even though VD-249 approved the *goal*.
VD-249 §5 anticipates exactly this: "Nếu tham số timeout hiện có mang nghĩa khác đã được SPEC chốt,
**trình phần thay đổi hợp đồng trước khi sửa**." Per the task brief I therefore **stopped code
changes for part 1** and wrote the proposal below.

### 1.2 What the code actually does today (read-only analysis)

- `net.rs:225` and `:357`: `let han = Instant::now() + Duration::from_millis(timeout.max(1));`
  — computed **once** before the loop (the brief's description "recomputed per read" does **not**
  match the file; blame: `b3323e27` ST-64 / `96e31bca` ST-109).
- `net.rs:251-258` / `:361-368`: `con = han.saturating_duration_since(Instant::now());` then
  `net.read(tay, 16*1024, con.as_millis().clamp(1, 2_000))`. Each wait uses only the **remaining**
  time and is capped at 2 s per read. A slow-drip server therefore **cannot** extend a *single
  response* beyond `timeout`; a silent server fails early (≤2 s), which the existing test
  `http_server_im_lang_timeout_duoi_3s` (`tests/http.rs:504`) already locks in.
- Resource cleanup already exists: `net.close(&tay)` at `:322` (GET, all paths) and `:399` (HEAD).
- **Remaining gap**: `http_get` (`:313-344`) calls `doc_http_mot` again for every redirect hop,
  each time with a **fresh** `han` and a fresh `net.connect(authority, timeout)`. Up to 6 hops ⇒
  the **whole `http.get_timeout` call** can occupy up to ~6 × `timeout`. That is the part of
  "one monotonic deadline for the whole call" not yet satisfied.

So: slow-drip is already bounded **per response**; the unbounded-in-practice piece is the
**redirect chain**, and fixing it by reusing `timeout` would change VD-183's chốt meaning.

### 1.3 Proposed contract change for A0 (decision needed — nothing implemented)

**Option A — re-chốt `timeout` as a whole-call deadline (recommended if A0 keeps "no new timeout API").**
- Re-declare the 2nd argument of `get_timeout`/`head_timeout` as *one monotonic deadline for the
  entire call*: connect + every redirect hop + all reads; each wait receives only the remaining
  time; per-read clamp stays 2 000 ms; `net.close` on expiry as today.
- Explicitly supersede VD-183 Q2(1)'s "giữa hai lần đọc, **không** phải tổng" **for this parameter**
  (the `timeout_connect`/`timeout_read` default table can stay as a design reference).
- Hoist `han` from `doc_http_mot`/`head_mot` into `http_get`/`http_head` and pass remaining ms down
  to `connect`, `doc_http_mot`, `head_mot`. Default stays 5 000 ms unless A0 also rules to adopt
  #148's 120 s suggestion (that is a **second**, separate decision).
- **Visible behavior change**: multi-hop redirects and slow-drip responses that previously could
  take up to 6 × `timeout` now fail at `timeout`; callers relying on per-read semantics see earlier
  timeouts. Needs a red-before test (slow-drip through ≥2 hops) + the existing 21 HTTP cases green.

**Option B — keep VD-183 meaning, add `timeout_total` (≈120 s) as #148 originally proposed.**
- Preserves the chốt contract and matches `SO-NO.md:236`, but violates VD-249 §5's "Không … API
  timeout mới" — requires A0 to ratify an explicit exception.

**Option C — accept current behavior as sufficient.**
- Close #148 with "bounded per response; redirect chain ≤ 6 × timeout by construction". Only viable
  if A0 accepts a bounded-but-multiplied deadline as "hạn chót tổng".

**A0 is asked to pick A / B / C.** I recommend **A** for consistency with "no new timeout API",
but it **must** be an explicit re-chốt, not a silent code change. No test was added and no behavior
was changed pending that decision.

### 1.4 Validation performed (baseline, since no code changed)

- `cargo test -p vis-interp -p vis-cli` → **rc 0, 987 passed / 0 failed** (log:
  `/tmp/vd249-lifecycle/cargo-test.log`).
- `./vis-dev check` → **PASS** (fmt, clippy, rules, self-test, build, doc-examples, report check;
  log: `/tmp/vd249-lifecycle/vis-dev-check.log`).
- Harness note: I did **not** write the requested slow-drip test, because adding it would assert the
  new contract before A0 has re-chốt it. It is specified in Option A above and ready to add on
  approval.

---

## PART 2 — Targeted lifecycle measurement

### 2.1 What was measured, and what was deliberately not

- A0 correction applied: the earlier 50k "create/destroy" numbers only proved spawn/await; they did
  **not** prove tasks were released. I therefore measured **RSS together with a task-count proxy
  over a 12.5 s run**, with a flat control of the same wall time.
- **Not re-proven**: that blocking I/O blocks (already established; the 20 × 200 ms case is kept
  only as a baseline workload and for its time/RSS numbers).
- **Monotonic time**: `time.monotonic_ms()` (Rust `Instant`, `stdlib.rs:444-448`) inside the program;
  Python `time.monotonic()` externally for wall time. `time.now_ms()` (wall clock) was **not** used.
- **Distribution**: every workload repeated; min / median / max reported (not a single wall time).
- **No product code changed** for the measurement (see §2.5 for the observability limitation).

### 2.2 Environment

`Linux x86_64`, 12 cores, 14 651 MB RAM, load average **0.63 / 0.33 / 0.32** at start,
`rustc 1.99.1`, `vis` = `/home/ps/ViScript/target/debug/vis` (dev profile), HEAD `812feda`.
RSS sampled from `/proc/<pid>/status:VmRSS` every 20 ms (first samples are the just-forked process,
hence a few hundred KB — peak and plateau values are the meaningful ones).

### 2.3 Workloads kept / added (all programs and logs in `/tmp/vd249-lifecycle/`)

| ID | Program | Content |
|---|---|---|
| W1 | `spawn50k.vis` | baseline: 50 000 `spawn`, no `await`, join at end |
| W2 | `spawnawait50k.vis` | baseline: 50 000 `spawn` + immediate `await` |
| W3 | `net20.vis` | baseline: 20 tasks × `net.read(16, 2000)` against a 200 ms-delay loopback server + `MARK` task |
| W4 | `longwave.vis` | **new long-duration variant**: 20 waves × (3 000 spawn+await tasks + 3 network reads @200 ms) = 60 060 tasks, ~12.5 s; prints `(wave, cumulative-task-proxy, monotonic_ms)` per wave |
| W5 | `flat_io_long.vis` | **new flat control**: same ~12 s and 60 network waits, **no task spawn** — isolates retention from general runtime/I/O cost |

Server: `server.py <port> 0.2`, thread-per-connection, sends 1 byte after 200 ms. (First W3 attempt
had a harness bug: the server waited for request bytes the workload never sends ⇒ 2 s read timeouts;
that run was discarded and the server fixed. All numbers below are from the corrected server.)

### 2.4 Raw numbers and distributions (interp; vt. /tmp/vd249-lifecycle/raw_*.json for every sample)

**W1 — 50 000 spawn** (5 runs, all rc=0):

| run | wall ms | peak RSS KB | program monotonic END−BEGIN ms |
|---|---:|---:|---:|
| 1 | 223.5 | 25 420 | 128 |
| 2 | 243.6 | 25 788 | 151 |
| 3 | 223.1 | 25 580 | 126 |
| 4 | 222.8 | 25 724 | 117 |
| 5 | 223.0 | 25 484 | 127 |

Distribution: wall **min 222.8 / med 223.1 / max 243.6 ms**; peak RSS **min 25 420 / med 25 580 /
max 25 788 KB**; program-monotonic span **min 117 / med 127 / max 151 ms**.

**W2 — 50 000 spawn+await** (5 runs, all rc=0):

| run | wall ms | peak RSS KB | program monotonic END−BEGIN ms |
|---|---:|---:|---:|
| 1 | 244.6 | 23 988 | 214 |
| 2 | 244.5 | 24 988 | 215 |
| 3 | 244.7 | 25 628 | 218 |
| 4 | 224.3 | 24 356 | 212 |
| 5 | 243.8 | 25 268 | 216 |

Distribution: wall **min 224.3 / med 244.5 / max 244.7 ms**; peak RSS **min 23 988 / med 24 988 /
max 25 628 KB**; program-monotonic span **min 212 / med 215 / max 218 ms**.

**W3 — 20 × 200 ms network** (5 runs, all rc=0, 20/20 reads ok):

| run | wall ms | peak RSS KB | per-connection latency range ms | `MARK` monotonic ms |
|---|---:|---:|---|---:|
| 1 | 4 066.4 | 9 632 | 201–202 | 4 035 |
| 2 | 4 053.6 | 9 848 | 201–202 | 4 032 |
| 3 | 4 061.7 | 9 612 | 200–202 | 4 034 |
| 4 | 4 053.9 | 9 548 | 201–203 | 4 027 |
| 5 | 4 061.0 | 9 608 | 201–202 | 4 031 |

Distribution: wall **min 4 053.6 / med 4 061.0 / max 4 066.4 ms**; peak RSS **min 9 548 / med 9 612 /
max 9 848 KB**; latency over all 100 connections **min 200 / med 202 / max 203 ms**. `MARK` is the
last output in every run (workload characterization, not a re-proof of blocking I/O). Scheduler
retained tasks at end: 21 (20 `doc` + 1 `marker`).

**W4 — long-duration, 60 060 tasks, ~12.5 s** (3 runs, all rc=0):

| run | wall ms | peak RSS KB | program monotonic BEGIN→DONE ms | RSS slope KB / 1 000 tasks |
|---|---:|---:|---:|---:|
| 1 | 12 530.1 | 28 976 | 0 → 12 496 | 312.5 |
| 2 | 12 512.3 | 28 452 | 0 → 12 483 | 312.5 |
| 3 | 12 549.7 | 29 016 | 0 → 12 514 | 312.5 |

Distribution: wall **min 12 512.3 / med 12 530.1 / max 12 549.7 ms**; peak RSS **min 28 452 /
med 28 976 / max 29 016 KB**; monotonic span **min 12 483 / med 12 496 / max 12 514 ms**.

Monotonic checkpoints and paired RSS samples from **run 1** (external sampling time ≈ program time,
offset < 30 ms):

| cumulative tasks (proxy) | program monotonic ms | VmRSS KB |
|---:|---:|---:|
| 3 003 | 624 | 10 856 |
| 6 006 | 1 253 | 11 796 |
| 9 009 | 1 878 | 12 728 |
| 12 012 | 2 505 | 13 668 |
| 15 015 | 3 129 | 14 608 |
| 18 018 | 3 754 | 15 548 |
| 21 021 | 4 380 | 16 488 |
| 24 024 | 5 005 | 17 420 |
| 27 027 | 5 632 | 18 356 |
| 30 030 | 6 252 | 19 296 |
| 33 033 | 6 877 | 20 236 |
| 36 036 | 7 504 | 21 176 |
| 39 039 | 8 126 | 22 116 |
| 42 042 | 8 748 | 23 056 |
| 45 045 | 9 374 | 23 988 |
| 48 048 | 10 002 | 24 928 |
| 51 051 | 10 628 | 25 868 |
| 54 054 | 11 252 | 26 808 |
| 57 057 | 11 874 | 27 744 |
| 60 060 | 12 496 | 28 684 |

From 3 003 → 60 060 tasks, RSS rose **10 856 → 28 684 KB** (Δ **17 828 KB** in **all three** runs),
a **constant** linear slope of **312.5 KB per 1 000 tasks ≈ 320 bytes/task**. No plateau within the run.

**W5 — flat control, no task spawn** (2 runs, all rc=0):

| run | wall ms | peak RSS KB | program monotonic BEGIN→DONE ms | RSS after 2 s → 12 s |
|---|---:|---:|---|---|
| 1 | 12 106.8 | 9 496 | 0 → 12 092 | 9 496 → 9 496 KB (flat) |
| 2 | 12 107.4 | 9 468 | 0 → 12 091 | 9 468 → 9 468 KB (flat) |

Distribution: wall **min 12 106.8 / med 12 107.1 / max 12 107.4 ms**; peak RSS **min 9 468 /
med 9 482 / max 9 496 KB**. Same wall time and same 60 network waits as W4, but RSS **plateaus
after ~2 s** and never grows — the W4 growth is attributable to task retention, not to the runtime,
the network host, or sampling noise.

### 2.5 Live-task/resource count: observability limitation + proposed hook

**Limitation (precise).** There is **no public way to read the live task count** from outside the
process or from a ViScript program:
- `Scheduler.tasks` is a private-to-crate field reached via `Host::sched()` (`builtin.rs:201`);
  `Interp::new` and `Interp::run_program` are `pub(crate)` (`interp.rs:128,217`), and
  `vis_interp::run*` creates and drops the `Interp` internally (`lib.rs:110-189`). A temporary
  standalone Rust harness in `/tmp` therefore **cannot** inspect it without forking product code.
- `join_all` (`rt.rs:242-249`) **clones** the task vec and awaits each; no code path removes,
  clears or retains-fewer entries. Grep over `crates/` for `tasks.remove|clear|retain|pop|truncate`
  found none. Hence `Scheduler.tasks.len()` == cumulative tasks created, for the whole run.
- The `cumu` column in W4 is therefore a **derived proxy** (each wave creates exactly 3 003 tasks),
  not a directly observed counter. It is exact by construction from `rt.rs:95-116` and the workload,
  but A0 should treat it as source-derived, not instrumented.
- "Live" needs qualification: at each W4 checkpoint ~3 tasks are in flight (network wave) and the
  rest are `Xong` with cached `kq`; all remain **reachable** from `Scheduler.tasks` until the process
  drops the scheduler at exit. Runnable (`ready`) count is 0 at checkpoints by construction.

**Minimal diagnostic hook proposed for A0 approval (NOT implemented — no product code changed):**
- In `vis-cli`, gated by an env var e.g. `VIS_RUNTIME_DIAG=1` (default off, no behavior change, no
  new language API, no new public Rust API needed for users): emit **one stderr line** at program
  exit with `tasks_total`, `tasks_new`, `tasks_running`, `tasks_done`, `ready_len`, `channels`,
  and `VmRSS` — read via the existing `Host::sched()` from inside `vis-cli`, ~10–15 lines.
- For **during-run** sampling, add behind the same env var a counter bump in `rt.rs:tao` that emits
  the same fields + `Instant` monotonic ms every N tasks (N configurable). This is a diagnostic
  only; it does not alter scheduling or results and stays compiled out of normal runs.
- Avoid a `std.task.stats()` builtin for now: that is a language/API change and needs its own VD.

### 2.6 Conclusion — retained by design vs unexpected growth (careful separation)

1. **Not an unreachable-memory leak.** Every `Rc<Task>` stays reachable from `Scheduler.tasks`;
   nothing is lost. Retaining completed tasks is a **design consequence** of one-thread
   run-to-completion + cached `await` results + no GC (`rt.rs:10-11`, `39-52`, `241-249`). The flat
   control (W5) proves the runtime and network host themselves do not grow: RSS is constant at
   ~9.5 MB for 12 s. So the earlier "50k ≈ 25 MB, no leak" reading was **correct but only covered
   end-of-run**.
2. **But there is sustained, unbounded growth in a target workload.** W4 shows a **constant
   ≈312.5 KB / 1 000 tasks** (≈320 B/task) with **no plateau** over 12.5 s and 60 060 tasks:
   10.9 MB → 28.7 MB, reproducible in 3/3 runs. For a long-running program that keeps spawning
   tasks (the exact workload class in question), memory grows linearly without bound and there is
   no admission control or task reclamation (`BAO-CAO-SAN-SANG-THUC-TE-2026-11-20.md:95`).
   Indicative extrapolation only (not measured): 1 M tasks ≈ 312 MB, 10 M ≈ 3.1 GB.
3. **A0's correction honored**: "không có GC không tự động là blocker" — I do **not** claim absence
   of GC is itself a blocker. The finding is narrower and measurable: *uncontrolled lifecycle /
   unbounded memory growth in a repeated-spawn workload*, with a flat control isolating the cause.
   Whether that is a blocker for the planned target workloads is an **A0 decision**, informed by
   §2.5's hook if during-run state counts are wanted.
4. **Baseline preserved for future architecture comparison**: W1–W5 programs, the 200 ms server,
   the sampling driver, all per-run samples and the summarised distributions are in
   `/tmp/vd249-lifecycle/`. A future multi-worker/GC architecture should re-run the **same** W1–W5
   and compare: wall distribution, peak RSS, RSS-vs-cumulative-tasks slope, and (once the hook is
   approved) live-task count.

### 2.7 Artifacts (all outside the repo)

```
/tmp/vd249-lifecycle/
  spawn50k.vis  spawnawait50k.vis  net20.vis  longwave.vis  flat_io_long.vis
  server.py  measure.py
  raw_spawn50k.json  raw_spawnawait50k.json  raw_net20.json
  raw_longwave.json  raw_flat_io_long.json
  log_spawn50k.txt  log_spawnawait50k.txt  log_net20.txt  log_longwave.txt  log_flat_io_long.txt
  cargo-test.log  vis-dev-check.log
```

---

## Deliverable summary for A0

- **#148**: no product code changed. VD-183 Q2(1) chốt the `timeout` parameter as *per-read, not
  total*; redefining it needs an explicit re-chốt. Read-only analysis found slow-drip already bounded
  **per response**, with the gap being the **redirect chain** (fresh deadline per hop). Three
  contract options (A re-chöt / B new `timeout_total` / C accept) are in §1.3; **A is recommended**.
  Tests are green (987/0; `vis-dev check` PASS) but no slow-drip test was added before A0 decides.
- **Lifecycle**: long-duration W4 (60 060 tasks, 12.5 s) shows **linear RSS growth ≈320 B/task with
  no plateau** (3/3 runs), while the matched no-spawn control W5 stays flat at ~9.5 MB. This is
  task retention by design becoming unbounded growth in a repeated-spawn workload — **not** a
  conventional leak. Live-task count is not publicly observable; a minimal env-gated diagnostic
  hook is proposed in §2.5 and awaits A0 approval. No product code was changed.
