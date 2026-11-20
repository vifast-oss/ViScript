<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-258/259/260 — báo cáo tích hợp (A0, 2026-11-20/21)

> **Trạng thái tại thời điểm ghi**: main `580123b` (sau method/globals/withunit/ctrl/generic/letelse/infer/range,
> wave 7/8: stdlib/frame/genericres/pexpr/fs/db/intlit/methodret + LSP + CLI + fs/db/stdlib mới),
> gate **2290 passed / 3 skipped**. Core-test/release/toolchain PASS.

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
| Method | `240c2a0` → `3b0757a` | type-check thân method `impl`: break/continue, tên lạ, return |
| Globals | `d518975` → `6c2c9c2` | khởi tạo top-level `let`/`var` trước `fn main`, cả 2 engine |
| Perf | `2395a8d`, `7a93bea`, `74f2a37` | cache quét schema SQLite theo schema/data version; strip release 6.23 MB |
| VM/with | `7255065` → `23f2ee3` | `with` lấy kiểu tĩnh mọi gốc; `TestList` tách khỏi `TestTuple`/Unit |
| Ctrl | `a4e00fb` → `409dc15` | interp lan `break`/`continue` ra khỏi expression if/match |
| Generic | `f84c0d2` → `4626406` | field instantiation `G[int?]`/enum/with/gán trường; API `compile_voi_bang` |
| Let-else | `59fd3d4` → `1dae9be` | `else` phải thoát mọi nhánh (VS0206); scope theo snapshot; binder `Err`/`()`; interp=VM |
| Infer | `f640514` → `131a899` | kiểu sau pattern lồng (tuple/list/record/ctor/or/for/if-let/while-let); `Call`/`Try`/`With`/`If`/`Match` |
| Range | `cec83ac` → `70f4624` | `for` range duyệt lười (interp/VM/frame), RSS 146 MB → ~9,5 MB @2M |
| Cache | `d485d1a` | bump `PHIEN_BAN` + ghim khoá `f20c2ade…` sau let-else/generic/range |
| Cache | `f9748a7`, `20bfb16` | bump `PHIEN_BAN` + ghim khoá `1bafa949…` (methodret/genericres/intlit), `204af1c2…` (nestassign) |
| Stdlib | `bc94f61` → `40063f8` | `list.zip/min/max`, `str.lines/split_whitespace`; API-STD +5, interp=VM |
| Frame | `79d00cb` → `9f306f2` | frame let-else theo pattern, binder chung, hoàn nguyên env |
| LSP | `bdedaaa`, `98d269d`, `044aa92`, `9026490`, `5fc00c5`, `f372446`, `c5705d1`, `ab43766`, `47defc8`, `e5fc5fc` | `definition` + `documentSymbol` + `references` + `rename` + `signatureHelp` + `foldingRange` + hover variant + completion `std.*` + `workspace/symbol` + formatting + prepareRename |
| Testfilter | `d06a807` → `5405489` | `vis test --filter <substr>` + `--fail-fast` (TAP plan theo số đã chạy) |
| Panic | `5c8b550` → `0c22c76` | `panic(msg) -> never`, trap `VS0501` exit 101, cả 2 engine; tách `builtin_sig.rs` |
| Fmt-size | `1ab5d8b` → `350d7c4` | size thập phân ≤2 lẻ, duration ghép `2h30m`; cập nhật show/tests/docs |
| Fmtstdin | `8d2ae13` → `8820d4d` | `vis fmt --stdin [--check]`, exit 0/1/2, không đụng đĩa |
| Nestassign | `88dac79` → `2b18749` | gán lồng `x.f.g`/`x[i].f`/`x[i][j]`, `+=` trên lá, COW; parser assign_op sau lvalue |
| Glob | `9a89e30` → `55756dd` | `fs.glob(root, pattern)` với `*`/`?`/`**`, basename nếu không `/`, walk-based |
| Strpad | `3fbcc8e` → `1d20f5b` | `str.pad_left/right/center` + `trim_start/end`; scalar, fill lặp/cắt, trap rỗng |
| Dbcache | `c9e719d` → `b6828b8` | `prepare_cached` LRU 16; flush theo schema/temp schema; release 1,5–1,74× |
| Checkstdin | `0ed20cf` → `0eabbd1` | `vis check --stdin` (TAP/JSON, tệp ảo `-`, exit 0/1/2) |
| Fmt-mode | `022c127` → `226245b` | `vis fmt --stdin --mode script|command`; chỉ với stdin |
| Fscopy | `09283a4` → `5457b9d` | `fs.copy/rename`: chỉ tệp, từ chối đích tồn tại/symlink, guard hai đầu |
| FS | `c4774a8` → `34aa2f0` | `read_lines/append/mkdir/remove`; FsHost default + FsGuard; API +4 |
| DB | `2f583ec` → `b619c08` | `query_one` (dừng sau hàng đầu), `last_insert_rowid`, `changes`; read-only/caps giữ nguyên |
| Intlit | `fd702c7` → `38c8bff` | `i64::MIN` viết trực tiếp; bare `2^63` vẫn `VS0104` tĩnh |
| Methodret | `768522e` → `4791032`/`b6a8274` | bảng kiểu trả method + `self` dispatch + checker sig; ưu tiên `self_ty`, bỏ `{error}` |
| Genericres | `633a1a2` → `d7d875d` | pre-pass chữ ký trước thân; tail if/match, literal ẩn danh, RHS Field/Index, method/closure instantiation |
| Enumalloc | `9580e23` → `b1d962f` | `Value::Enum` dùng `Name(Rc<str>)` nội bộ hoá; Value 72→64 B; microbench Some 1,8×, Ok 1,9× |
| Windows | `026d679` → `8b1609e` | `list.windows` trượt; R5c buộc biến kiểu ⇒ sugar generic `sort/take/zip/...` |
| Optmethod | `db9e5d4` → `38dec30` | `recv?.m()` lift Option một lớp, đoản mạch đối số; interp=VM |
| Fsmeta | `60cac03` → `3d30be1` | `fs.size/is_file/is_dir` dùng `symlink_metadata`, không theo symlink |
| Fmt-json | `c306b25` → `c121b6c` | `vis fmt --check --json` (tệp/dir/stdin), schema `ok/files` |
| Strfind | `791f0c9` → `6f7a51f` | `str.find/rfind/count`: offset scalar, `int?`, không chồng lấn |
| Fmt-list | `4cd3fde` → `5f52e9f` | `vis fmt --check --list`: mỗi tệp cần sửa một dòng |
| Explain-json | `d5c9698` → `e8e5332` | `vis explain [--json]` / `--list --json` một dòng |
| Fmt-diff | `daddba5` | `vis fmt --check --diff`: unified diff một hunk/tệp |
| Strlen | `3157b45` → `edc4d2e` | `str.byte_len/char_count/is_empty` + `list/map.is_empty` |
| Vistest-repeat | `ec0b5cc` → `207e111` | `vis test --repeat N`: hậu tố `[k/N]`, TAP/JSON |
| LSP selection | `304a857` → `8916490` | `textDocument/selectionRange`: token→line→decl→doc |
| Run-time | `245ab23` → `000011f` | `vis run --time` in `# time: N ms/us` ra stderr, không đổi exit/output |
| Indexof | `2e10889` → `f217ec9` | `list.index_of/last_index_of`: vị trí 0-based, `int?` |
| Vistest-shuffle | `ff8d712` → `f11a159` | `vis test --shuffle [--seed u64]`: SplitMix64 + Fisher-Yates |
| LSP hover method | `f2f103c` → `4babfb0` | hover phương thức `impl`/`trait`, fallback cuối |
| Db-batch | `e81c298` → `1b51c62` | `sqlite.execute_batch`: tách câu an toàn, per-statement policy |
| Str-trim | `31c43a0` → `51744bc` | `str.trim_matches/trim_start_matches/trim_end_matches` theo tập scalar |
| Vistest-timeout | `8730a8d` → `b785a6a` | `--timeout` áp cả fixture `.out/.err`, kill + `quá hạn` |
| LSP hover builtin | `be9ad23` → `53f87b7` | hover phương thức lõi `list/map/str` (bảng 28 chữ ký) |
| Fmt-stat | `e73f00e` → `580123b` | `vis fmt --check --stat`: một dòng đếm tệp/cần sửa/lỗi |
| Runstdin | `806e9ae` → `2128d88` | `vis run --stdin`: script ảo `-`, TAP stderr, exit 0/1/2/101, không cache |

## 2. Bằng chứng cổng

- `./vis-dev gate --fast`: **PASS — 2290 passed / 3 skipped** tại `580123b`.
- `cargo test -p vis-cli --no-default-features` (tuần tự sau gate): **exit 0**.
- `./vis-dev core-test`: **PASS** (target riêng `target/core-test`).
- `bash test/release/run.sh`: **PASS**; artifact chỉ `vis` (không `test-child`).
- `test/toolchain/run.sh`: **PASS**.
- `./vis-dev nightly` @ `5099dee`: **PASS** — parser fuzz **18.574.043 ca / 421.534.861 token / 0 vi phạm**
  (120 s), vòng tròn fmt **475 tệp bất động**, gate L0 **1822/0**.
- `./bin/vis-diff` @ `5099dee`: **0 lệch** (2.000 + 100.000 ca, 7 test PASS).
- `VIS_DIFF_N=200000` @ `d7d875d`: **0 lệch** (200.000 ca, 187.537 chạy, 175,7 s, seed 20261121).
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
- **Range lười** (`cec83ac`): `for i in 0..2_000_000` không còn dựng `Vec<Value>`; RSS debug
  **149.860 → 9.536 KB** (interp), **150.132 → 9.688 KB** (VM); trần `10^7` đếm bằng `i128`
  trước khi vào thân vòng, `E_LOOP` và `break`/`continue` giữ nguyên. Không đổi `Value`/opcode/VERSION.
- P3 enum alloc (#152) vẫn mở; perf audit ghi nhận là nợ tiếp theo.

## 6. Còn mở / chưa tuyên bố hoàn tất

- Method `impl` body đã được type-check cơ bản; `self`/kiểu phép gán sâu còn nợ.
- Wave 6 đã đóng: let-else, break/continue expression, `with` non-Var, `[]` vs `()`, generic field,
  pattern/call inference, range lười. **Wave 7 đang chạy**: method-return, frame parity,
  generic residual, stdlib helpers.
- Cache label đã bump cho let-else/generic/range (`d485d1a`); nếu sửa codegen tiếp phải bump lại.
- Refutable `let`/`for` nay bị từ chối tĩnh (đúng VD-260); `let … else` chỉ cho Option/Result — fixture rest pattern đã đổi sang `if let`.
- Frame vẫn cooperative; timer/interleave chỉ tại safe point; quota là test-hook.
- `list` rest pattern chưa hỗ trợ hậu tố `[.., z]` (parser từ chối tĩnh).
- `std.path` Windows chỉ test helper thuần; chưa chạy runtime Windows.
- Parser chưa nhận `if`/`match` nhiều dòng làm biểu thức con trong đối số gọi (`VS0201`) — nợ parser.
- P3 enum alloc (#152) chưa sửa.
- Budget/fuel/Q vẫn chưa chốt; step-cap còn mở; runtime mặc định vẫn không đổi.
- **pexpr** (`88c1331` → `9d9a7f1`): `if`/`match` biểu thức trong ngoặc/đối số/list (một dòng và nhiều dòng); parser đếm delimiter, suite ngầm kết thúc, `match` theo dòng/cột; 14 ca hai engine.
- **LSP** (`bdedaaa`, `98d269d`): `textDocument/definition` + `documentSymbol`; capability mới + snapshot `initialize` cập nhật.
- **fmt-size** (`1ab5d8b` → `350d7c4`): size thập phân ≤2 lẻ, duration ghép; cập nhật show/test/docs.
- **fmtstdin** (`8d2ae13` → `8820d4d`): `vis fmt --stdin [--check]`, exit 0/1/2, không đụng đĩa.
- `VIS_DIFF_N=100000` @ `350d7c4`: **0 lệch** (93.770 chạy, 164,5 s, seed 20261122).
- **fswalk** (`894e45c` → `a76c945`): `fs.walk` đệ quy, bỏ symlink, mock/VM khớp; API +1.
- **LSP xuyên tệp** (`0365ecb`): `definition` fallback quét `rootUri` khi không thấy trong tệp.
- `./vis-dev nightly` @ `ab43766`: **PASS** — parser fuzz **6.621.319 ca / 150.252.151 token / 0 vi phạm** (120 s), fmt 475 tệp bất động.
- **test --list** (`18c8587`): `vis test --list [--filter]` in tên ca, không chạy; `--list --json` bị từ chối.
- **enumalloc** (`9580e23` → `b1d962f`): nội bộ hoá tên enum bằng `Name(Rc<str>)` + cache thread-local; `Value::Enum(Name, Name, …)`, không đổi show/eq/JSON.
- **strpad** (`3fbcc8e` → `1d20f5b`): `str.pad_*`/trim một phía; width theo scalar, trần 10^7.
- **dbcache** (`c9e719d` → `b6828b8`): prepared statement LRU 16, flush khi schema/temp schema đổi; release 1,74× (`SELECT 1`), 1,50× (WHERE), 1,52× (INSERT); debug chậm hơn ~1,25–1,37×.
- **fmt-mode** (`022c127` → `226245b`): `--mode script|command` cho stdin; đường tệp vẫn suy theo đuôi, `--mode` ở đường tệp bị từ chối.
- **fscopy** (`09283a4` → `5457b9d`): `fs.copy`/`fs.rename` từ chối ghi đè/symlink/thư mục; guard đọc+ghi; API +2.
- **runstdin** (`806e9ae` → `2128d88`): `vis run --stdin`, tệp ảo `-`, TAP stderr, mã thoát như tệp, không cache.
- **windows** (`026d679` → `8b1609e`): `list.windows` cửa sổ trượt; R5c buộc `T/K/V` từ receiver ⇒ sugar generic (`xs.sort()`, `.take()`, `.zip()`, `.min/max()`, `.windows()`) hoạt động; API +1.
- **enumalloc** (`9580e23` → `b1d962f`): tên enum nội bộ hoá `Name(Rc<str>)`, `Value` 72→64 B; microbench `Some` 1,8×, `Ok` 1,9×; loop interp `Some` −12,5%, VM user-enum −22,9% (máy tải, số chỉ hướng); không đổi VBC1/opcode.
- `test/release/run.sh` / `test/toolchain/run.sh` / `bin/vis-host-suite` @ `6f0b71a`: **PASS** (artifact chỉ `vis`; toolchain 1.100.0; host 54/54 + POSIX real).
- **optmethod** (`db9e5d4` → `38dec30`): `recv?.m()` — base đánh giá một lần, `None` không đánh giá đối số; `U→U?`, `U?→U?` không bọc hai lớp; `()` giữ `()`; chain `a?.m()?.f`.
- **fsmeta** (`60cac03` → `3d30be1`): `fs.size/is_file/is_dir` (`symlink_metadata`, symlink ⇒ false/false + size Err; missing predicate Ok(false)); API +3.
- **fmt-json** (`c306b25` → `c121b6c`): `vis fmt --check --json` tệp/dir/stdin; `ok/files/path/formatted/error`; exit 0/1/2.
- `VIS_DIFF_N=100000` (seed 20261124, @ `484d570`): **0 lệch** (93.770 chạy, 162,7 s).
- `vis test test/sem` @ `3d30be1`: **525/525 PASS** (interp) và **525/525 PASS** (`--vm`).
- `./vis-dev nightly` @ `3d30be1`: **PASS** — parser fuzz **8.543.006 ca / 193.835.169 token / 0 vi phạm** (120 s), fmt 475 tệp bất động.
- `bin/vis-host-suite` @ `3d30be1`: **54/54 PASS** + POSIX real.
- `vd270_r5c_sugar`: khoá sugar generic `sort/take/drop/chunk/unique/zip/min/max/windows/concat` cả hai engine.
- **strfind** (`791f0c9` → `6f7a51f`): `str.find/rfind` trả `int?` offset scalar, `count` không chồng lấn; `Ty::OptInt`; API +3.
- **fmt-list** (`4cd3fde` → `5f52e9f`): `vis fmt --check --list` in tệp cần sửa, exit 0/1/2; `--stdin --check --list` in `-`.
- **explain-json** (`d5c9698` → `e8e5332`): `vis explain [--json]`/`--list --json` một dòng; người dùng thường giữ byte.
- `./vis-dev core-test` @ `e8e5332`: **PASS** (target `target/core-test`).
- `test/release/run.sh` / `test/toolchain/run.sh` / `bin/vis-host-suite` @ `e8e5332`: **PASS** (artifact chỉ `vis`; toolchain 1.100.0; host 54/54 + POSIX real).

## 7. Wave 11 đang chạy (tại thời điểm ghi)

- `vd270-flatten` — `list.flatten(list[list[T]]) -> list[T]` một lớp.
- `vd270-lsp-implementation` — `textDocument/implementation` cho method `impl`/trait.
- `vd270-db-explain` — `sqlite.explain` chạy `EXPLAIN QUERY PLAN` qua đường query chỉ đọc.
- `vd270-docs` — audit README/CHEAT-SHEET/CHANGELOG theo tính năng hiện có.
Wave 10 đã merge: str-trim, vistest-timeout fixture, lsp-hover-builtin, fmt-stat.
Kết quả wave 11 chưa merge tại mốc ghi; gate/QA cuối sẽ chạy lại sau khi merge.
- `VIS_DIFF_N=100000` seed 20261126 @ `e8e5332`: **0 lệch** (93.770 chạy, 60,7 s).
- **strlen** (`3157b45` → `edc4d2e`): `str.byte_len/char_count/is_empty`, `list/map.is_empty`; API +5; grapheme_count hoãn vì dependency.
- **vistest-repeat** (`ec0b5cc` → `207e111`): `vis test --repeat N` (case-major, `[k/N]`, TAP/JSON, fail-fast).
- **lsp-select2** (`304a857` → `8916490`): `selectionRange` token→line→decl→doc, test riêng.
- **fmt-diff** (`daddba5`): `vis fmt --check --diff` in unified diff một hunk/tệp, exit 0/1/2.
- **run-time** (`245ab23` → `000011f`): `vis run --time` đo quanh lời gọi runtime, in `# time: N ms|us` ra stderr; lỗi tĩnh/`--dry` không in; `--tap` giữ TAP hợp lệ.
- **indexof** (`2e10889` → `f217ec9`): `index_of/last_index_of` 0-based, so cấu trúc `==`, `int?`; API +2.
- **vistest-shuffle** (`ff8d712` → `f11a159`): `--shuffle`/`--seed u64` (mặc định 0), deterministic, trước repeat/list.
- **lsp-hover-method** (`f2f103c` → `4babfb0`): hover method `impl`/`trait`, chữ ký + chủ; ưu tiên sau các hover cũ.
- `./vis-dev nightly` @ `e8e5332`: **PASS** — parser fuzz **7.241.771 ca / 164.328.256 token / 0 vi phạm** (120 s), fmt 475 tệp bất động.
- `./vis-dev core-test` + `test/release/run.sh` + `test/toolchain/run.sh` + `bin/vis-host-suite` @ `4babfb0`: **PASS** (artifact chỉ `vis`; toolchain 1.100.0; host 54/54 + POSIX real).
- **db-batch** (`e81c298` → `1b51c62`): `sqlite.execute_batch` tách `;` ngoài string/comment, chạy từng câu qua `db_execute` (policy/cap/timeout per câu), không tự transaction; trigger body chưa hiểu.
- `VIS_DIFF_N=100000` seed 20261127 @ `4babfb0`: **0 lệch** (93.769 chạy, 122,6 s).
- Smoke mới @ `1b51c62`: `list.index_of/last_index_of`, `str.byte_len/char_count`, `is_empty` cho str/list/map — **interp=VM byte-identical**.
- **str-trim** (`31c43a0` → `51744bc`): `trim*_matches` theo tập scalar Unicode; rỗng/all-stripped đúng; API +3.
- **vistest-timeout** (`8730a8d` → `b785a6a`): `--timeout` áp fixture, kill + `not ok/# quá hạn`, JSON/fail-fast/repeat giữ.
- **lsp-hover-builtin** (`be9ad23` → `53f87b7`): hover `list/map/str` method lõi + sugar R5c; bảng lõi cục bộ 28 chữ ký.
- **fmt-stat** (`e73f00e` → `580123b`): `--check --stat` một dòng `fmt-check: N tệp, M cần sửa, K lỗi`; stdin hỗ trợ.
- `VIS_DIFF_N=200000` seed 20261128 @ `580123b`: **0 lệch** (187.536 chạy, 270,1 s).
- `vis test test/sem` @ `580123b`: **525/525 PASS** (interp) và **525/525 PASS** (`--vm`).
- `./vis-dev nightly` @ `580123b`: **PASS** — parser fuzz **8.396.336 ca / 190.517.523 token / 0 vi phạm** (120 s), fmt 475 tệp bất động.
- `./vis-dev core-test` + `test/release/run.sh` + `test/toolchain/run.sh` + `bin/vis-host-suite` @ `580123b`: **PASS** (artifact chỉ `vis`; toolchain 1.100.0; host 54/54 + POSIX real).
