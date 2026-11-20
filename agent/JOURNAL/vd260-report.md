<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-258/259/260 — báo cáo tích hợp (A0, 2026-11-20/21)

> **Trạng thái tại thời điểm ghi**: main `1eeeaab` (sau wave 3 feature/frame/parity),
> gate **1704 passed / 3 skipped**. Semantics3 (refutable/scope/record) còn chạy.

## 1. Dòng thay đổi đã merge

| Nhóm | Commit nguồn → merge | Nội dung |
|---|---|---|
| Checker | `166bc04` → `0a13b24` | `Result?` không còn bị coi là lớp Option; record ctor/default; `None` typed; `?.` lồng; lift ctor/`with`/push; multi-file builtin trait/ctor |
| Cache | `c8e3beb` | bump `PHIEN_BAN` + pin hash sau đổi checker/codegen |
| Semantics | `44844d8` | VM bind or-pattern theo alternative khớp; `()` = `Unit` |
| Frame | `1ae5cfd` → `f9aaeee`, `7a2ecbc` → `6e26836`, `2d2cefd` → `fcb3921` | Redispatch stack, method await, deadlock, closure/const env, fake-source join, exit code; HBody cache/sorted trace/lifecycle/purge; ready-budget timer |
| Fee/tooling | `0f733c7`,`7e131b0` → `f1348e5` | verify_fee OOB/panic; for E_LOOP; call fee parity; MSRV 1.100.0; core test gating |
| SQLite | `80af3b5`,`1d07072`,`6919da5` → `4638d04`; `3f7833f` → `e4990ab` | cell/overhead cap, pre-materialization bound, schema UTF-8, handle cap, token OS RNG, dbstat deny, read-only query, json float, LSP |
| Semantics 2 | `a4cf655` → `fb12645` | or-pattern checker đệ quy/type; break/continue context; `main -> int` exit 0 |
| Feature | `2b568e2` → `9735b76` | **list rest patterns** `[a, ..]`/`[a, ..r]`, no opcode/VERSION change |
| Parity | `c73f047` → `921b092` | type-directed Option lift, ST-248 field/index/self; branch plain→Option lift |
| Tooling | `5aa1431` → `e1a2f2f` | release only `vis`; `vis-dev core-test`; real POSIX host suite |

## 2. Bằng chứng cổng

- `./vis-dev gate --fast`: **PASS — 1704 passed / 3 skipped** tại `1eeeaab`.
- `cargo test -p vis-cli --no-default-features` (tuần tự sau gate): **exit 0**.
- `./vis-dev core-test`: **PASS** (target riêng `target/core-test`).
- `bash test/release/run.sh`: **PASS 15/15**; artifact chỉ `vis` (không `test-child`).
- `test/toolchain/run.sh`: **12/12**.
- `host_posix_real`: `/bin/echo`, `/usr/bin/grep`, `/bin/sh`, `/usr/bin/printf` thật, interp + VM.

## 3. Bug review đã đóng (tiêu biểu)

- `Result?`/`Option[Result]`, `?.` lồng, record ctor/default, `None` trong container/điều kiện/đối số.
- Or-pattern: VM bind đúng alternative; checker đệ quy + type consistency; if-let/while-let/for/let.
- Frame: operand stack `Redispatch`, `CallMethod` Pending, RedispatchMethod, deadlock exit 0, closure env, const env, fake-source body, exit code.
- Fee: `verify_fee` OOB; `for` E_LOOP; call/method fee class.
- SQLite: cell overhead/NULL 20M-cell OOM; `zeroblob` pre-materialization; non-UTF8 schema panic; tombstone growth; dbstat/raw pages; query write policy.
- JSON `Json.Num(Float)` roundtrip; LSP header/Interrupted/percent drive.
- ST-248 interp↔VM inference; branch plain→Option.
- Cache stale bytecode after checker/codegen changes.

## 4. Tính năng mới

1. `std.path` nhận Windows drive/UNC/cả hai separator (`is_abs`, `join`, `basename`, `dirname`, `ext`, `stem`), POSIX giữ nguyên.
2. **List rest patterns** `[a, ..]` / `[a, ..r]` end-to-end; no VERSION/opcode change; copy semantics; negative rest placement static `VS0201`.
3. `bin/vis-release` + `test/release/run.sh`: release chỉ `vis`, chứng minh `test-child` không lọt artifact.
4. `vis-dev core-test`: core test target/schedule độc lập.

## 5. Còn mở / chưa tuyên bố hoàn tất

- Semantics3 đang chạy: refutable `let`/`for`; checker scope leak/unresolved name; record unknown/positional/duplicate.
- Method `impl` body chưa được type-check đầy đủ (break/continue trong method có thể lọt).
- Cache label chưa bump cho feature/parity (sẽ làm cuối).
- Frame vẫn cooperative; timer/interleave chỉ tại safe point; quota là test-hook.
- `list` rest pattern chưa hỗ trợ hậu tố `[.., z]` (parser từ chối tĩnh).
- `std.path` Windows chỉ test helper thuần; chưa chạy runtime Windows.
- Budget/fuel/Q vẫn chưa chốt; step-cap còn mở; runtime mặc định vẫn không đổi.
