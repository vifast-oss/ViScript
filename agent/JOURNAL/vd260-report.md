<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-258/259/260 — báo cáo tích hợp (A0, 2026-11-20/21)

> **Trạng thái tại thời điểm ghi**: main `0eb6b32` (sau method/globals/withunit/ctrl/generic/letelse/infer/range,
> wave 7/8: stdlib/frame/genericres/pexpr/fs/db/intlit/methodret + LSP + CLI + fs/db/stdlib mới),
> gate **2698 passed / 3 skipped**. Core-test/release/toolchain PASS.

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
| Flatten | `2d1ea0f` → `122e7e6` | `list.flatten`: nối một cấp |
| LSP implementation | `24e1127` → `9cae14a` | `textDocument/implementation` cho method impl/trait |
| Db-explain | `f2eb442` → `78b9626` | `sqlite.explain`: EXPLAIN QUERY PLAN qua đường query chỉ đọc |
| Vistest JSON-out | trực tiếp → `ae53ee6`/`759e45d` | `vis test --json-out <path>`: TAP stdout + báo cáo JSON |
| Fs bytes | `d5a5208` → `dfd5ee4` | `fs.read_bytes`/`write_bytes`: byte thô `list[int]` |
| Str strip | `94eed35` → `0c410b0` | `str.strip_prefix`/`strip_suffix` → `str?` |
| Path normalize | `e468a29` → `c0cba70` | `path.normalize`: chuẩn hoá từ vựng POSIX |
| Check JSON | `487dc94` → `e17998f` | khoá `check --json` + sửa cờ lạ exit 2 |
| Binary search | `e1906a7` → `1a5cc32` | `list.binary_search`: biên dưới, `int?` |
| Replace first | `df611e7` → `03950f4` | `str.replace_first`: chỉ lần khớp đầu |
| LSP codeLens | `d88ccaa` → `cf94e77` | `textDocument/codeLens` cho `test fn` |
| SQLite tables | `c0003bb` → `254c03f` | `sqlite.tables`: bảng/view schema chính |
| Batch trigger | trực tiếp → `8b758fc` | `execute_batch` hiểu `BEGIN…END` trigger |
| Splitn | `90c51c6` → `71396aa` | `str.splitn`: tối đa `n` phần |
| LSP inlayHint | `f0f8819` → `05d6caa` | `textDocument/inlayHint` cho let/var không nhãn |
| SQLite columns | `5e6e672` → `c75c0da` | `sqlite.columns`: cột theo `cid`, bind tham số |
| Parse int | trực tiếp | `str.parse_int`: thập phân chặt → `int?` |
| Trim perf | `81bb7eb` | `trim_matches` binary-search cho tập >8 scalar (~70–84×) |
| R5c arity | `f014695` → `a82ec35` | sugar sai số đối số ⇒ `VS0401` ở checker |
| List contains | `0a1282e` → `85130e6` | `list.contains` theo `eq_value` |
| SQLite indexes | `4ecc182` → `6084191` | `sqlite.indexes`: index người dùng, bind tham số |
| Parse float | `0f880d9` → `a6ac405` | `str.parse_float`: thập phân Rust, bỏ non-finite |
| List sum/product | `e43ff02` → `e047dff` | `list.sum`/`product` checked, tràn `VS0507` |
| Frame nested | `8bcbf71` → `c6e058e` | frame engine gán lồng mọi đường |
| LSP typeDefinition | `097c042` → `bfb41f4` | `textDocument/typeDefinition` record/enum |
| Replace last | `d7c3d7f` → `5dead58` | `str.replace_last` lần khớp cuối |
| Doctor | `7182aa3` → `09823e1` | `vis doctor [--json]` tất định |
| Path relative | `eff3ef3` → `318a5c4` | `path.relative` lexical POSIX |
| Schema SQL | `a6ebf63` → `8ce3246` | `sqlite.schema_sql` → `Result[str?, Error]` |
| Str title | `ae86c9b` → `60d0961` | `str.title` hoa đầu từ theo khoảng trắng Unicode |
| SQLite index columns | `409756b` → `146cf32` | `sqlite.index_columns` theo thứ tự index |
| List mean | `60902ac` → `b55f233` | `list.mean` — trung bình `list[float]` |
| Path join all | `a1c7378` → `96c8547` | `path.join_all` gấp trái bằng `path.join` |
| SQLite primary keys | `9bc6f3a` → `1833df8` | `sqlite.primary_keys` theo thứ tự `pk` |
| Mkdir all | `8f046b4` → `0eb6b32` | `fs.mkdir_all` đệ quy, chặn symlink |
| Version | trực tiếp | `vis version [--json]` |
| Runstdin | `806e9ae` → `2128d88` | `vis run --stdin`: script ảo `-`, TAP stderr, exit 0/1/2/101, không cache |

## 2. Bằng chứng cổng

- `./vis-dev gate --fast`: **PASS — 2698 passed / 3 skipped** tại `0eb6b32`.
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
- **flatten** (`2d1ea0f` → `122e7e6`): `list.flatten` một cấp, list rỗng/lồng giữ nguyên; API +1.\n- **lsp-impl** (`24e1127` → `9cae14a`): `textDocument/implementation` lọc theo owner tĩnh, trait name-only, cap 50.\n- **db-explain** (`f2eb442` → `78b9626`): `sqlite.explain` chỉ SELECT/WITH, chạy qua `db_query`, comment đầu, với-write bị chặn.\n- **vistest-jsonout** (`759e45d`): `--json-out` ghi JSON report, stdout vẫn TAP; `--json` kèm thì stdout = tệp.\n- Smoke mới @ `78b9626`: flatten + `sqlite.explain` trên `:memory:` — **interp=VM byte-identical**.\n- `vis test test/sem` @ `8ab2213`: **525/525 PASS** (interp) và **525/525 PASS** (`--vm`), sau wave 11.
- **fsbytes** (`d5a5208` → `dfd5ee4`): byte thô roundtrip; validate hết trước khi ghi; `FsGuard`/`--fs-mock`; R1 tách `stdlib/chung.rs` (`75e4536`).
- **str-strip** (`94eed35` → `0c410b0`): `strip_prefix`/`strip_suffix`, `Ty::OptStr`, affix rỗng ⇒ `Some(s)`, sugar R5c.
- **path-normalize** (`e468a29` → `c0cba70`): lexical POSIX; `..` giữ khi không phân giải; rỗng ⇒ `.`; sugar `p.normalize()`.
- **check-json** (`487dc94` → `e17998f`): 16 ca khoá JSON hiện có; sửa `check --json --co-lạ` từ exit 1 thành exit 2.
- `./vis-dev core-test` PASS sau khi thêm `#![cfg(feature = "sqlite")]` cho `vd270_db_explain`.
- Smoke mới @ `e17998f`: `str.strip_prefix/strip_suffix`, `path.normalize` — **interp=VM byte-identical**.
- `./vis-dev core-test` + `test/release/run.sh` + `test/toolchain/run.sh` + `bin/vis-host-suite` @ `e17998f`: **PASS** (artifact chỉ `vis`; toolchain 1.100.0; host 54/54 + POSIX real).
- `vis test test/sem` @ `e17998f`: **525/525 PASS** (interp) và **525/525 PASS** (`--vm`), sau wave 12.
- Bench tham chiếu @ `5e1488e`: `test/bench/opt-bench.vis` → interp **1247 ms**, VM **1115 ms** (cùng kết quả 38995450).
- `VIS_DIFF_N=100000` seed 20261130 @ `0bcbed7`: **0 lệch** (93.769 chạy, 160,2 s).
- `./vis-dev nightly` @ `0bcbed7`: **PASS** — parser fuzz 5.644.311 ca / 128.083.645 token / 0 vi phạm; fmt 475 tệp bất động.
- **binary-search** (`e1906a7` → `1a5cc32`): `list.binary_search` biên dưới trên `cmp_value`; chưa sắp ⇒ không xác định nhưng không trap.
- **replace-first** (`df611e7` → `03950f4`): `str.replace_first` chỉ thay lần đầu; `from` rỗng ⇒ prepend.
- **lsp-codelens** (`d88ccaa` → `cf94e77`): lens `Chạy test <name>` cho `test fn`, UTF-16, cap 200.
- **sqlite-tables** (`c0003bb` → `254c03f`): `sqlite.tables` sắp byte, bỏ `sqlite_%`, bảng TEMP không liệt kê.
- `./vis-dev core-test` + `test/release/run.sh` + `test/toolchain/run.sh` + `bin/vis-host-suite` @ `885879d`: **PASS** (artifact chỉ `vis`; toolchain 1.100.0; host 54/54 + POSIX real).
- **batch-trigger** (`8b758fc`): `execute_batch` quét `CREATE [TEMP] TRIGGER … BEGIN … END` nguyên khối; `;`/`CASE`/chuỗi trong thân không tách; thiếu `END` ⇒ `Err`; 3 ca interp=VM.
- `vis test test/sem` @ `8b758fc`: **525/525 PASS** (interp) và **525/525 PASS** (`--vm`).
- `./vis-dev core-test` + `test/release/run.sh` + `test/toolchain/run.sh` + `bin/vis-host-suite` @ `3779394`: **PASS** (artifact chỉ `vis`; toolchain 1.100.0; host 54/54 + POSIX real).
- **splitn** (`90c51c6` → `71396aa`): `str.splitn` đúng Rust, `n=0` ⇒ `[]`, phần cuối giữ nguyên; API +1.
- **lsp-inlay** (`f0f8819` → `05d6caa`): hint `: <kiểu>` cho `let`/`var` không nhãn; 10 ca; hợp nhất capability với codeLens.
- **sqlite-columns** (`5e6e672` → `c75c0da`): `sqlite.columns` theo `cid`, `pragma_table_info(?, 'main')`, bind an toàn; API +1.
- `VIS_DIFF_N=100000` seed 20261201 @ `3fb7adb`: **0 lệch** (93.767 chạy, 96,9 s).
- **r5c-arity** (`f014695` → `a82ec35`): sugar thiếu/thừa đối số bị checker bắt `VS0401` (exit 2), không còn trap `VS0501`/101; cập nhật 9 test cũ + `vd270_sugar_arity.rs`.
- **list-contains** (`0a1282e` → `85130e6`): `list.contains` dùng chung `eq_value` với `index_of`; sugar R5c.
- **sqlite-indexes** (`4ecc182` → `6084191`): `sqlite.indexes` chỉ index người dùng, sắp byte, bind tham số, bỏ autoindex.
- `vis test test/sem` @ `65c3e1f`: **525/525 PASS** (interp) và **525/525 PASS** (`--vm`).
- `VIS_DIFF_N=100000` seed 20261202 @ `4dad2ab`: **0 lệch** (93.767 chạy, 113,1 s).
- `./vis-dev core-test` + `test/release/run.sh` + `test/toolchain/run.sh` + `bin/vis-host-suite` @ `4dad2ab`: **PASS** (artifact chỉ `vis`; toolchain 1.100.0; host 54/54 + POSIX real).
- `./vis-dev nightly` @ `3976328`: **PASS** — parser fuzz **8.074.278 ca / 183.212.821 token / 0 vi phạm** (120 s), fmt 475 tệp bất động.
- **parse-float** (`0f880d9` → `a6ac405`, fix `4c57285`): `str.parse_float` theo `f64::from_str`, từ chối non-finite/tràn; `Ty::OptFloat`; 7 ca interp=VM.
- **list-math** (`e43ff02` → `e047dff`): `list.sum`/`product` checked, rỗng 0/1, tràn `VS0507`; `xs.sum()` lõi giữ nguyên.
- **recv-opt-infer** (`9589f59` → `5b8e0b1`): `recv?.m()` dùng kiểu **suy ra** của `let`; `impl` trên instantiation (`impl Hop[int]`) được đăng ký; fallback tên method duy nhất; 11 ca interp=VM.
- `VIS_DIFF_N=100000` seed 20261204 @ `3c842e0`: **0 lệch** (93.767 chạy, 154,1 s).
- `./vis-dev core-test` + `test/release/run.sh` + `test/toolchain/run.sh` + `bin/vis-host-suite` @ `3c842e0`: **PASS**.
- `./vis-dev nightly` @ `c99339e`: **PASS** — parser fuzz **8.740.029 ca / 198.309.965 token / 0 vi phạm** (120 s), fmt 475 tệp bất động.
- **frame-nested** (`8bcbf71` → `c6e058e`): frame engine gán lồng field/index/hỗn hợp, thứ tự index→index→RHS, dùng chung `gan`/`gan_long`; 18 ca frame interp↔VM.
- **lsp-type-def** (`097c042` → `bfb41f4`): `textDocument/typeDefinition` trả khai báo record/enum cho biến/trường/tham số; capability thứ 3.
- **replace-last** (`d7c3d7f` → `5dead58`): `str.replace_last` lần khớp cuối scalar; `from` rỗng ⇒ nối cuối; API 118→119.
- `VIS_DIFF_N=100000` seed 20261205 @ `fd2a19a`: **0 lệch** (93.767 chạy, 124,7 s).
- `./vis-dev core-test` + `test/release/run.sh` + `test/toolchain/run.sh` + `bin/vis-host-suite` @ `fd2a19a`: **PASS** (artifact chỉ `vis`; toolchain 1.100.0; host 54/54 + POSIX real).
- `./vis-dev nightly` @ `e9c2d38`: **PASS** — parser fuzz **8.857.961 ca / 200.975.208 token / 0 vi phạm** (120 s), fmt 475 tệp bất động.
- `vis test test/sem` @ `09823e1`: **525/525 PASS** (interp) và **525/525 PASS** (`--vm`).
- **doctor** (`7182aa3` → `09823e1`): `vis doctor [--json]` in version/os/arch/sqlite/features, không phụ thuộc môi trường.
- **path-relative** (`eff3ef3` → `318a5c4`): `path.relative` thuần từ vựng; cùng loại ⇒ tương đối, trộn loại ⇒ `to` chuẩn hoá.
- `VIS_DIFF_N=100000` seed 20261206 @ `318a5c4`: **0 lệch** (93.767 chạy).
- **schema-sql** (`a6ebf63`/`ca92c3c` → `8ce3246`): `sqlite.schema_sql` tra SQL khai báo theo tên bind; thiếu/NULL ⇒ `Ok(None)`; `Ty::ResultOptStrErr`.
- **str-title** (`ae86c9b` → `60d0961`): `str.title` hoa scalar đầu mỗi từ theo whitespace Unicode, `ß` ⇒ `SS`; giữ nguyên khoảng trắng.
- **sqlite-index-columns** (`409756b` → `146cf32`): `sqlite.index_columns` theo thứ tự index; fallback `sqlite_master.sql` + `pragma_table_info` do authorizer chặn `pragma_index_info`; bỏ mục biểu thức.
- `VIS_DIFF_N=100000` seed 20261207 @ `54f5db4`: **0 lệch** (93.767 chạy, 82,4 s).
- `./vis-dev core-test` + `test/release/run.sh` + `test/toolchain/run.sh` + `bin/vis-host-suite` @ `54f5db4`: **PASS** (artifact chỉ `vis`; toolchain 1.100.0; host 54/54 + POSIX real).
- `./vis-dev nightly` @ `5e41548`: **PASS** — parser fuzz **15.289.413 ca / 346.969.831 token / 0 vi phạm** (120 s), fmt 475 tệp bất động.
- `vis test test/sem` @ `6dc6182`: **525/525 PASS** (interp) và **525/525 PASS** (`--vm`).
- `VIS_DIFF_N=100000` seed 20261208 @ `5865211`: **0 lệch** (93.767 chạy, 96,5 s).
- **list-mean** (`60902ac` → `b55f233`): `list.mean(list[float]) -> float?`, rỗng `None`, IEEE NaN/inf; thêm `Ty::Float`.
- **path-join-all** (`a1c7378` → `96c8547`): `path.join_all(list[str])`, gấp trái đúng `path.join`; không sugar.
- **sqlite-pk** (`9bc6f3a` → `1833df8`): `sqlite.primary_keys` theo `pk`, bind tên; thiếu/view ⇒ `Ok([])`.
- **mkdir-all** (`8f046b4` → `0eb6b32`): `fs.mkdir_all` đệ quy idempotent, chặn file/symlink, `Quyen::chuan_de_quy`; mock parity.
- `./vis-dev core-test` + `test/release/run.sh` + `test/toolchain/run.sh` + `bin/vis-host-suite` @ `73f5db6`: **PASS** (artifact chỉ `vis`; toolchain 1.100.0; host 54/54 + POSIX real).
- `./vis-dev nightly` @ `504795b`: **PASS** — parser fuzz **8.089.984 ca / 183.570.150 token / 0 vi phạm** (120 s), fmt 475 tệp bất động.
- `vis test test/sem` @ `e561dc1`: **525/525 PASS** (interp) và **525/525 PASS** (`--vm`).
