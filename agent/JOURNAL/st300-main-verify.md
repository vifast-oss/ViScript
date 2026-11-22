<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-300 — verify harness trên main `3581c9a` + hàng targets `wasm32-wasip1`

> **Người làm**: subagent VS3-W2, 2026-11-23. **Nhánh** `plan/w2-st300-main-verify` từ
> **`3581c9a`** (ST-300 harness đã merge). Không merge/push/tag; `CARGO_BUILD_JOBS=3`,
> `TMPDIR=/home/ps/tmp`.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| `bin/vis-wasm-sem` trên main | **388/390 eligible = 99,5%**, fail 2 (stack overflow interp), runtime **Node** |
| Oracle native | `sem-native-baseline.tsv` sha256 `482b35c2…a308d` (475/475) |
| Suite cũ | **28/28** |
| Native `vis-host-wasm` | **29/29** (17 unit + 12 integration) |
| `bin/vis-targets` | thêm hàng **`wasm32-wasip1 (runtime)`**; core 18/18; sqlite 16+2 EXPECTED_RED |
| Gate Linux | **3660 passed / 0 failed / 4 skipped** |
| fmt/clippy/R1..R7 | **PASS** |
| Blocker | **không** (2 stack-overflow đã ghi là known gap) |

## 1. Sem trên main

- `./bin/vis-wasm-sem --build --runtime node`:
  ```
  # sem total=525 eligible=390 excluded=135 runtime=node
  # oracle_sha256=482b35c276d76fca9200770f78b3cba891535b09b65dbce0fc52bce6379a308d
  # SUMMARY total=525 eligible=390 excluded=135 pass=388 fail=2 coverage=99.5% target=85%
  ```
- 2 fail **giữ nguyên có chủ đích**: `ham/de-quy-1000.vis`, `ham/de-quy-vo-han.vis` —
  interp WASI stack overflow; VM pass rc/hash. Không che/exclude.
- Suite cũ `bin/vis-wasm-suite --runtime node`: **28/28** (14 ca × interp/VM).
- Native host: `cargo test --manifest-path vis-host-wasm/Cargo.toml`: **17 + 12 = 29/29**.

## 2. `bin/vis-targets` row mới

Thêm báo cáo (không phải ô cargo check, không tham gia PASS/FAIL):

```
=== wasm32-wasip1 (runtime) ===
vis-wasm-sem    wasm32-wasip1   388/390 eligible = 99.5%
vis-wasm-suite  wasm32-wasip1   28/28 (14 ca x interp/VM)
known-gap       wasm32-wasip1   2 stack-overflow interp: ham/de-quy-1000, ham/de-quy-vo-han (VM pass)
coverage-note   wasm32-wasip1   eligible only; KHÔNG tuyên bố full support
wasm32-wasip1: coverage 388/390 eligible (99.5%), suite 28/28; known gap 2; không full
```

- `./bin/vis-targets`: exit 0; core **18 PASS / 0 FAIL**; sqlite **16 PASS + 2 EXPECTED_RED**
  (#156); row runtime in đúng như trên.

## 3. Cổng / log

- `./vis-dev gate --fast`: **PASS — 3660 passed / 0 failed / 4 skipped** (69,9 s);
  fmt/clippy/R1..R7 OK.
- Log ngoài repo (R7):
  - `/home/ps/vislogs/st300-main-sem.log` (61 KB) — sha256
    `3896c4169d8ef97a57beda16339d076beb3f5fa1a5a2325c42aef16dc27d493b`.
  - `/home/ps/vislogs/st300-main-gate.log` (365 KB) — sha256
    `4c3ec5306cc28c678fe67573ef2853360c924ffad9235e8ce7839ef866c05efb`.
  - `/home/ps/vislogs/st300-main-suite28.log` — sha256
    `ce74109e91db887b17459a669facfa82cfc9751dfccfbfe7e9d24f23e4797c73`.
  - `/home/ps/vislogs/st300-main-targets.log` — sha256
    `0f03e66f1389440d32d03f17c11ea029eb717970918d24d1405b9506cfff9bf7`.

## 4. Kết luận

- Main `3581c9a` giữ đúng kết quả ST-300: **388/390 eligible (99,5%)**, suite 28/28, native 29/29.
- `bin/vis-targets` nay báo trung thực coverage/known gaps `wasm32-wasip1`, không tuyên bố full.
- **Không blocker**; không sửa `QUEUE`/`SO-NO`; không merge/push/tag.
