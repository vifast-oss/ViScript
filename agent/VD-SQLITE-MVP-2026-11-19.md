# VD — SQLite MVP (Gate 1: thiết kế + prototype nội bộ)

> Agent 1 hoàn thiện · từ lệnh A0 "mở VD SQLite MVP theo hai cổng" · 2026-11-19 · **Gate 1: worktree riêng, chưa merge API công khai.**
> Base: `f4ea00b` (docs) / code == `3b2c67e` (Option fix — **chưa** DONE, coercion còn mở — không phụ thuộc).
> Không raw FFI, không ABI mới, không pre-scan làm ranh giới bảo vệ, không merge API công khai.

## 0. Mục tiêu & tiêu chí Gate 1

- Prototype **hoạt động thật** với `rustc`+`cc`+bundled SQLite: `open/close/execute/query`, bind, transaction, lỗi, capability, limits.
- Kiểm chứng **hai engine** (interp + VM) byte-identical mọi ca.
- **Agent 2 báo cáo** trước cổng tích hợp: chữ ký API công khai đề xuất + phương án bảo vệ filesystem + kết quả prototype.
- Reviewer độc lập: Agent 2 khác tác giả (sau deploy) xác nhận hợp đồng kiểu/vòng đời/quyền.
- **Chưa** merge `main`, **chưa** phát hành, **chưa** cập nhật `api_dong_bang.rs`.

## 1. Phạm vi trong / ngoài

**Trong đợt này**: module `std.sqlite`; bundled `rusqlite`+`libsqlite3-sys` (bundled feature), ghim version, ghi SQLite version + compile options; SQL value enum `SqlValue`; rows shape `columns/rows`; params positional; transaction SQL; close semantics; limits; capability; corpus; mutation.

**Ngoài** (hoãn đúng lệnh A0): raw FFI/DLL; ORM; typed-row derive; callback SQL→script; loadable extension; `VACUUM`; `ATTACH/DETACH`; PRAGMA khác `table_info`/đọc `user_version`; closure transaction wrapper; streaming cursor; named params; implicit conversion; WAL/journal mode customization; `create: false`? (I có duyệt `create: false`); read-only mode; URI filename; tên rỗng; ATTACH/PRAGMA ghi.

## 2. Kiểu dữ liệu — `SqlValue` enum (module `std.sqlite`, không vào prelude toàn cục)

```
// Enum dựng sẵn của module std.sqlite (enum runtime sẵn có):
enum SqlValue:
    Null
    Integer(int)          # i64
    Real(float)           # f64
    Text(str)
    Blob(bytes)
```

**Quy tắc bất biến**:
1. **Không** dùng `Unit`/`None` cho SQL NULL; mọi cell thu được từ engine phải là `SqlValue` — `NULL` (cả NULL storage class) ⇒ variant `Null`.
2. **Không** tự suy `0/1` thành `bool` (SQLite không có BOOL).
3. `try`-test thực tế: `INTEGER` > i64 max ⇒ `Result.Err`; `REAL` không finite (NaN/±Inf) ⇒ giữ nguyên (SQLite chấp nhận doubles) **hoặc** lỗi — ghi lựa chọn và lý do trong báo cáo; `REAL` `NaN` round-trip.
4. **`list[list[SqlValue]]` ≠ `list[list[Value::Enum]]`** — Agent 2 phải chứng minh checker + constructor + pattern + hai engine xử lý enum đúng (`SqlValue` constructor, `match` exhaustive, `print`). Không dùng Any/exhaustive hack. Nếu chưa làm được trên cơ chế hiện tại ⇒ **báo A0**, không mở rộng ABI.

## 3. Shape kết quả (A0 §B)

```
record RowSet (tái dựng — không khai báo mới):
    columns: list[str]     # tên cột theo thứ tự select (trùng tên giữ nguyên)
    rows:    list[list[SqlValue]]
```

- Giữ nguyên **thứ tự** và **tên cột** trùng nhau. Không streaming.
- Kết quả vượt §6 (limits) ⇒ `Err`, **không** trả dữ liệu cắt.
- Duplicate column names (`select 1, 1`) ⇒ giữ cả hai theo thứ tự.
- Empty result set ⇒ `rows: []`, `columns:` theo schema tham chiếu (SQLite trả cả với 0 row).

## 4. API prototype (hàm module — công khai viết như sau; A0 duyệt trong cổng tích hợp)

```
use std.sqlite

std.sqlite.open(path: str, opts?) -> Result[Handle, Error]   # :memory: hoặc đường dẫn (KHÔNG URI)
std.sqlite.memory()?  → bytes? — hoặc open(":memory:") trực tiếp (A0 muốn :memory:)
std.sqlite.close(h) -> Result[Unit, Error]                   # close tường minh
std.sqlite.execute(h, sql: str, params: list[SqlValue]) -> Result[Int, Error]    # affected rows
std.sqlite.query(h, sql: str, params: list[SqlValue])   -> Result[RowSet, Error]
std.sqlite.begin(h) / commit(h) / rollback(h)           -> Result[Unit, Error]   # hoặc execute(h, "BEGIN", [])
std.sqlite.in_transaction(h)                            -> Result[Bool, Error]   # cách đọc trạng thái transaction (A0 E)
std.sqlite.columns(h, sql: str, params: list[SqlValue]) -> Result[list[str], Error]  # nếu query có metadata dễ hơn
std.sqlite.open_with(path: str, create: bool, busy_timeout_ms: int)  — hoặc open(path, opts) với defaults create=true, busy=5000
```

- **Hàm module**, chưa phải method (`db.query()`); A0: method dispatch không mở rộng ngoài phạm vi; prototype có thể trình `fn` module trước, cách viết công khai sau.
- `:memory:` là giá trị hợp lệ cho path (chế độ A0 I); tên rỗng ⇒ lỗi tĩnh; URI filename (`file:...`) ⇒ từ chối; ký tự đường dẫn thường thì `open` xử lý như file thường.
- `open(path)` mặc định **read-write + create: true** (thuận tiện, A0 I), có `create: false` từ chối tạo mới (không read-only yet).
- **Không** inject chuỗi vào SQL; params bind đủ số lượng, thiếu/thừa ⇒ `Err` tĩnh/dynamic (chốt lúc dev; cổng integration xét); chưa named params, chưa implicit conversion.

## 5. `DbHost` — host trait (mẫu `NetHost`, `crates/vis-interp/src/builtin.rs`)

```rust
// vis-interp crate — KHÔNG chạm OS; vis-cli cài thực sự (bundled rusqlite)
pub trait DbHost {
    fn db_open(&mut self, path: &str, create: bool, busy_ms: i64) -> Result<String, String>; // handle id chuỗi (vd "db1:1") — KHÔNG đáng đoán nếu generate ngẫu nhiên
    fn db_close(&mut self, h: &str) -> Result<(), String>;
    fn db_execute(&mut self, h: &str, sql: &str, params: &[Value]) -> Result<i64, String>;
    fn db_query(&mut self, h: &str, sql: &str, params: &[Value]) -> Result<(Vec<String>, Vec<Vec<Value>>), String>; // columns, rows — Value nội suy thành enum variant tĩnh ở tầng trên
    fn db_in_transaction(&mut self, h: &str) -> Result<bool, String>;
    fn db_row_count(&mut self, h: &str) -> Result<Option<i64>, String>; // optional handle-side cursor count — hoãn streaming
    fn db_set_authorizer?(…) — do DbHost cài trực tiếp bên trong trước khi prepare (mục 7) — agent 2 chọn cách cài sạch; mục đích là bắt buộc, không phải optional
    fn db_path_of(&mut self, h: &str) -> Result<String, String>; // phục vụ capability diagnostic & leak log
}

pub trait Host { … fn db(&mut self) -> Option<&mut dyn DbHost> { None } … }
```

**Handle contract** (A0): loại handle & chủ sở hữu phiên chạy được kiểm; từ chối handle đã đóng; **ID không tái sử dụng**; `"db:0"` biết không là quyền — check phải đến chủ + phiên.

**Một implementation phục vụ cả hai engine** — VM gọi `vis_interp::builtin::call` (`crates/vis-vm/src/vm.rs`, `Instr::CallBuiltin`); hàm host chỉ viết một lần trong `vis-interp` + `vis-cli` cài chạy. Không hai implementation độc lập.

## 6. Giới hạn tài nguyên (vượt ⇒ `Err`, không trả dữ liệu cắt)

| Giới hạn | Giá trị đề xuất (A0 duyệt trong cổng tích hợp) |
|---|---|
| max rows mỗi query | 10_000 |
| max kích thước một row (text+blob) | 1 MiB |
| max tổng kích thước kết quả | 16 MiB |
| max SQL text | 64 KiB |
| max params mỗi câu | 1_000 |
| max connections mỗi phiên chạy Host | 16 |
| busy timeout mặc định / cấu hình hữu hạn | 5_000 ms, max 60_000 ms |
| wall clock / query (nếu khả thi qua `progress_handler`) | 5_000 ms (đo thật khi làm) |
| max plain statement | 1 câu (từ chối multi) |

## 7. Authorizer & quyền (thay pre-scan — §2.1 addendum bắt buộc)

1. `DbHost` cài `sqlite3_set_authorizer` **trước khi** prepare SQL người dùng (rusqlite `Connection::set_authorizer` hỗ trợ).
2. Chính sách MVP (lệnh H): **cấm** `ATTACH`, `DETACH`, `LOAD_EXTENSION`, `VACUUM INTO` (và `VACUUM` không INTO nếu A0 xác nhận phạm vi rộng), `PRAGMA` ngoài `table_info` + đọc `user_version`; **không cho ghi `user_version`** (write bịauthorizer từ chối).
3. Authorizer đối xử theo **action code** cấu trúc, không theo chuỗi.
4. `libsqlite3-sys::SQLITE_AUTH_DENY`/`SQLITE_AUTH_IGNORE` ⇒ trả `Err` với thông điệp rõ (thêm cấp quyền không lộ hint thực thi).
5. **Pre-scan** (nếu có) chỉ để cải thiện thông điệp; không làm bằng chứng thực thi chính sách.
6. `SQLITE_ALTER_TABLE`/`CREATE_INDEX`/`DROP_TABLE`/`INSERT`/`SELECT`/`BEGIN`/`COMMIT`/`ROLLBACK`/`TRANSACTION` — tuỳ theo policy cuối cùng (MVP: cho phép các DDL/DML thường, chỉ deny nhóm ở mục 2) — Agent 2 ghi rõ trong báo cáo.
7. Mutation: **tắt authorizer** ⇒ ca cấm ATTACH/VACUUM-INTO phải đỏ (chứng minh authorizer là bằng chứng thực thi); tắt cleanup đúng giờ ⇒ ca thoát sớm đỏ.

## 8. Bảo vệ filesystem & temp storage (§2.2 addendum — cổng chặn lớn nhất)

Prototype phải làm rõ trong báo cáo:

1. Đường dẫn database và các tệp phụ (journal, `-wal`, `-shm`, `temp journal`, `sqlite-temp-*` khi cần VACUUM, `:memory:`-cạnh temp).
2. **Symlink/reparse point**: thay đổi target sau khi check; xử lý (open handle rồi check `/proc/self/fd` như `read_theo_handle` của `FsGuard`? hoặc từ chối strict-cap trên symlink? đóng cửa sổ TOCTOU bằng open-then-check).
3. **Temporary storage**: biết SQLite đang dùng gì (`PRAGMA temp_store` mặc định `=0`/FILE-either; `:memory:` khai `temp_store=MEMORY`?) — **phải** được kiểm soát (đề xuất force `temp_store=MEMORY`? — mất hiệu năng nhưng an toàn cho strict; **hoặc** `temp_store=FILE` + temp dir nằm trong `cap[Dir]` chặt hơn; **hoặc** file-backed strict-cap từ chối thẳng).
4. **URI filename** (`file:…`) — từ chối trong MVP.
5. Tên rỗng/tên đặc biệt — từ chối.
6. Chính sách khi mở file đã có `-wal`/`-shm` cạnh `--strict-caps`? (journal của db có sẵn nội dung ⇒ đọc được nội dung dở đã commit? — cần chốt.)
7. **Mô hình bằng chứng**: đề xuất (trước A0) = **VFS override** tối thiểu (chặn write khi open-major-paths ngoài cap qua custom `sqlite3_vfs` bond to cap map), hoặc `if (cap check fail) => Err`-every-VFS-call wrapper. Phương án rẻ hơn: **từ chối file-backed trong strict-caps, chỉ `:memory:` + explicit temp**. A0 chấp nhận "File-backed strict-caps chưa đạt ⇒ từ chối rõ" — nên prototype **ưu tiên tính năng safe**, không ép file-backed nếu chưa bằng chứng kín.
8. `ATTACH`/`VACUUM INTO` luôn cấm (authorizer), kể cả ngoài strict-caps.

## 9. Handle contract & rò

- Handle id có loại `"db:"+N`, N **không tái sử dụng** trong phiên; `pid+session_id` salt sơ bộ để tránh guess qua phiên (defense in depth; không phải ranh giới bảo vệ — authorizer/capability mới là ranh giới).
- `db_close` từ chối handle loại khác/không tồn tại/đã đóng; trả `Err` không panic.
- Kết thúc phiên Host (`Host::drop` / `vis CLI command end`) ⇒ đóng mọi DbHost connection trong bảng phiên; không auto-commit; cảnh báo rò **chỉ** ở chế độ diagnostics (vd `--explain-perms`/lệnh tương ứng), không stderr mặc định (A0 F).
- Không leak giữa các lần `vis run` trong cùng process (test macroscopi: nhiều lần run trong một test process, đếm connection, chỉ số `db_count`/`fs_handle_count`).

## 10. Test/kiểm chứng (độc lập + parity)

Nguyên tắc: kỳ vọng độc lập, không thi nhau; byte-identical hai engine; test đỏ trước khi fix.

Trong `test/sem/` (hoặc `crates/vis-cli/tests/sqlite_*`), tối thiểu:

| Ca | Hình thức | Engine |
|---|---|---|
| open `:memory:` + close | `.vis` + `.out` | ×2 |
| create table + insert + select | `.vis` + `.out` | ×2 |
| `NULL` round trip | select NULL → match arm | ×2 |
| BLOB rỗng / BLOB lớn / Unicode TEXT | bytes literals | ×2 |
| cột trùng tên `select 1 as x, 2 as x` | columns len 2 | ×2 |
| bind thiếu / thừa | `.err` VSxxxx | ×2 |
| chuỗi chứa dấu nháy `'` (bind) | insert text `o'clock` | ×2 |
| multi-statement `select 1; select 2` | `.err` từ chối | ×2 |
| transaction rollback | `BEGIN…INSERT…ROLLBACK…SELECT` | ×2 |
| lỗi trong transaction ⇒ không auto-commit | select sau rollback ⇒ row không có | ×2 |
| `BEGIN` lồng nhau / trạng thái `in_transaction` | true/false | ×2 |
| hai connection tranh khóa + busy_timeout | deadlock không treo | ×2 |
| handle giả `db="db99"` | `.err` | ×2 |
| handle đã đóng | `.err` | ×2 |
| xóa file giữa đường khi open | `.err` | ×2 |
| mở lúc `create: false` khi file chưa tồn tại | `.err` | ×2 |
| `ATTACH`/`VACUUM INTO`/`LOAD_EXTENSION`/`PRAGMA user_version=…` | `.err` (authorizer) | ×2 |
| `:memory:` tải nhiều/lần reset | rows giữ, state phiên | ×2 |
| limits (rows > 10k, ring > 16 MiB, SQL > 64 KiB, params > 1000, connections > 16) | `.err` | ×2 |
| cleanup khi thoát sớm | kết thúc phiên → connection mất | ×2 |
| leak qua nhiều `vis run` trong một test process | đếm handle | ×2 |
| mutation: tắt authorizer | ca ATTACH đỏ | — |
| mutation: tắt cleanup | ca thoát sớm đỏ | — |

## 11. Giao nộp Gate 1 & lịch trình

1. Worktree `/home/ps/.local/share/opencode/worktree/cd45e5/sqlite-mvp-<id>` nhánh `rev/vd-sqlite-mvp-2026-11-19`.
2. Bước: (a) `DbHost` trait + `vis-cli` host (rusqlite bundled, ghim version, SQLite version + compile options log); (b) `std.sqlite` stdlib rows trong `crates/vis-types/src/stdlib.rs` (StdFn); (c) `builtin::call` cases/match arms + registration; (d) corpus theo §10; (e) gate + two-engine byte-identical; (f) capability/authorizer/limits; (g) diagnostics leak warning (chỉ diagnostics).
3. **Báo cáo Gate 1** gồm: chữ ký API đề xuất cuối; phương án bảo vệ filesystem đã chọn + bằng chứng (§8); bảng limits; kết quả corpus + parity; mutation results; dependencies; các finding/blocker; những gì A0 cần ratify tiếp (API freeze, tên method/module naming, PRAGMA scope, max values, strict-caps file-backed decision); Windows/macOS chưa tuyên bố.
4. **Review** chéo (tác giả khác) trước khi trình cổng tích hợp; không mở merge API công khai.

## 12. Không thay đổi khác trong đợt này

- **Option coercion** (3b2c67e / đợt hiện tại) — không đụng; vẫn mở, không liên quan VD này.
- Windows Tier 2 @ `349af67` — không mở; SQLite native build chạy Linux; **chưa** tuyên bố Windows/macOS.
- ABI VBC1, `Value` variants, `api_dong_bang.rs`, SPEC-lex, `pha1-rc1` — không đụng.
- RFC typed serialization — không đụng; `typed-row derive` hoãn.
- B-3 — chưa bắt đầu; không phụ thuộc đợt này.
