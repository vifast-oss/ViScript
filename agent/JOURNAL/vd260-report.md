<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-258/259/260 — báo cáo tích hợp (A0, 2026-11-20/21)

> **Trạng thái tại thời điểm ghi**: main `7a93bea` (sau perf SQLite + release strip),
> gate **1728 passed / 3 skipped**. Core-test/release/toolchain PASS.

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
| Semantics 3 | `f5311ee` → `63aff1c` | refutable `let`/`for` rejected; checker scopes + unresolved `VS0301`; record unknown/positional/duplicate rules |
| Cache | `d780637` | `PHIEN_BAN` + pin hash `dd5794e2…` sau rest-pattern/parity |
| Docs | `1421616` → `5931968` | CHANGELOG + cheat-sheet (main->int, sqlite read-only, rest pattern) |

## 2. Bằng chứng cổng

- `./vis-dev gate --fast`: **PASS — 1728 passed / 3 skipped** tại `7a93bea`.
- `cargo test -p vis-cli --no-default-features` (tuần tự sau gate): **exit 0**.
- `./vis-dev core-test`: **PASS** (target riêng `target/core-test`).
- `bash test/release/run.sh`: **PASS 15/15**; artifact chỉ `vis` (không `test-child`).
- `test/toolchain/run.sh`: **12/12**.
- `host_posix_real`: `/bin/echo`, `/usr/bin/grep`, `/bin/sh`, `/usr/bin/printf` thật, interp + VM.
- QA độc lập @ `67f232a`: nextest **1727 passed / 3 skipped** (23.5 s); core-test **1151 passed**
  (118 binaries, 35.7 s); nightly `--fuzz-minutes 5` **44.103.531 cases / 1.001.085.722 tokens /
  0 vi phạm** (300 s), fmt-circle **475 tệp bất động**, 78 ca cú pháp sai bỏ qua; release artifact
  chỉ `vis` (7.264.080 B) + licenses, **0 test-child**; host suite **54/54 + POSIX real**.
- Lưu ý lịch sử: `63aff1c` (merge semantics3 trước khi cập nhật fixture rest-pattern) **đỏ core-test**
  tại 2 ca refutable; `1a0249d` sửa fixture + R1, `67f232a` xanh lại. Không còn đỏ ở HEAD.

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

## 5. Tối ưu (wave perf)

- **SQLite schema scan cache** (`2395a8d`): trước VD-261 quét `sqlite_master` mỗi `execute`/`query`
  (đo 6,8× với 300 bảng, +25–45 µs/lời gọi); nay cache theo `PRAGMA schema_version`, cho phép
  **đọc** pragma này (không ghi), chỉ quét lại khi DDL đổi. Test `schema_utf8_cache_theo_schema_version`
  chứng minh 20 lời gọi sau DDL không quét lại; DDL kế tiếp quét lại.
- **Release strip** (`7a93bea`): `[profile.release] strip = "symbols"`; `target/release/vis`
  **7.264.080 → 6.229.464 B** (−14,3%), artifact release vẫn chỉ `vis`.
- Perf audit (read-only) ghi nhận P2 range `Vec` 146 MB @2M và P3 enum alloc; **chưa sửa**
  (rủi ro/ABI), ghi thành nợ tiếp theo.

## 6. Còn mở / chưa tuyên bố hoàn tất

- Method `impl` body chưa được type-check đầy đủ (break/continue trong method có thể lọt).
- Cache label đã bump cho rest-pattern/parity (`d780637`); nếu sửa codegen tiếp phải bump lại.
- Refutable `let`/`for` nay bị từ chối tĩnh (đúng VD-260); `let … else` chỉ cho Option/Result — fixture rest pattern đã đổi sang `if let`.
- Frame vẫn cooperative; timer/interleave chỉ tại safe point; quota là test-hook.
- `list` rest pattern chưa hỗ trợ hậu tố `[.., z]` (parser từ chối tĩnh).
- `std.path` Windows chỉ test helper thuần; chưa chạy runtime Windows.
- Budget/fuel/Q vẫn chưa chốt; step-cap còn mở; runtime mặc định vẫn không đổi.
