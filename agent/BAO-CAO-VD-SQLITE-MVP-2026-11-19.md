# Báo cáo — VD SQLite MVP · Gate 1 (thiết kế + prototype nội bộ)

> **Agent 2 → A0/Agent 1** · 2026-11-19 · nhánh `rev/vd-sqlite-mvp-2026-11-19` · worktree `sqlite-mvp-2026-11-19`.
> **Chưa merge `main`, chưa phát hành, chưa tuyên bố Windows/macOS.** Prototype **hoạt động thật** với
> rusqlite bundled; **API chưa đông băng** (freeze `API-STD.txt` **không** regen — xem §8).
> Base: branch = `origin/main` `533faf3` + 2 commit (`f673fbf` feat + `e2a16eb` R1 split).

## 0. Kết luận nhanh

- **Toàn bộ test workspace xanh, trừ đúng một ca:** `api_dong_bang::api_std_khop_code` — đỏ **theo
  thiết kế Gate 1** (API chưa qua cổng tích hợp; `api_dong_bang.rs` **không** được regen theo lệnh).
  `api_codes_khop_code` và `api_vbc1_khop_code` (mã chẩn đoán + thẻ VBC1) **xoan/pass** — không đổi ABI.
- `./vis-dev check` **PASS** (fmt/clippy/R1..R6/self-test/doc-examples). `vis test test/sem` và
  `vis test --vm test/sem` = **514/514** (corpus **+27 ca `sqlite/`**; hai engine byte-identical).
- Fresh-clone gate (clone cục bộ từ nhánh, sạch): cùng trạng thái — `check` PASS; `cargo test --workspace`
  148 binaries, duy nhất `api_std_khop_code` đỏ; `vis test test/sem`/`--vm` = **514/514** ×2.
  Clone đã **xoá** sau khi đo (không giữ rác `/tmp`).
- **Mutation**: tắt authorizer ⇒ ca cấm đỏ (đúng kỳ vọng); tắt cleanup phiên ⇒ ca thoát sớm đỏ (đúng kỳ vọng). Cả hai **hoàn nguyên**, xanh lại.
- `crates/vis-interp` **không** linking native; `rusqlite`/`libsqlite3-sys` **chỉ** ở `vis-cli`; trait
  `DbHost` chỉ thấy `String`/`Value` — ràng buộc §13 (portability) thoả (xem §12).

## 1. §2 hợp đồng — `SqlValue` enum (thuộc module, `EnumId(1)`)

Enum dựng sẵn `SqlValue` khai trong `crates/vis-types/src/lower.rs` (`declare_kieu_dung_san`, **không** vào
prelude toàn cục ở nghĩa API): `Null` (không field) · `Integer(int)` · `Real(float)` · `Text(str)` · `Blob(bytes)`
— payload field tên `v`. `Value::Enum("SqlValue", …)` (enum runtime sẵn có, **không** Value variant mới).
`RowSet` = bản ghi dựng sẵn `RecId(1)` (`columns: list[str]`, `rows: list[list[SqlValue]]`).

**Gate theo module (đích lệnh A0 "thuộc module, không prelude"):** tên kiểu `SqlValue`/`RowSet` chỉ thấy
khỉ tệp đã `use std.sqlite` (`Lowerer::named_type` → `VS0301` nếu không); ctor `SqlValue.Integer(1)` và
`SqlValue.Null`-dạng-có-đối-số ngoài phạm vi ⇒ lỗi biên dịch; **pattern trên scrutinee `SqlValue` KHÔNG
gate** (đúng vai trò của pattern — người dùng nhận `RowSet.rows[i][j]` không cần nêu tên kiểu).
`SqlValue`/`RowSet` nằm trong **danh sách tên dự trữ phạm vi tệp** (như `Json`/`Error`).
Unit đa tệp: enum dựng sẵn tái lập **bảng tên** ở mỗi `Lowerer` (bảng kiểu dùng chung, id ổn định).

**Bằng chứng checker (constructor + match exhaustive + print):**

```
$ ./bin/vis run test/sem/sqlite/option-enum.vis
int(42)
real
text(abc)
blob(3)
null
$ ./bin/vis run --vm test/sem/sqlite/option-enum.vis   # byte-identical
int(42)
real
text(abc)
blob(3)
null
$ ./bin/vis run /tmp/opencode/enum2.vis   # match thiếu Blob
enum2.vis:4:5: error VS0402: `match` thiếu nhánh: Blob
```

`list[list[SqlValue]]` ≠ `list[list[Value-enum]]` — tyck xử đúng cả hai chiều (`list[SqlValue]`
phần tử được kiểm ở `kiem()`: `Value::Enum(e, …) ∧ e == "SqlValue"`; người dùng `list[list[SqlValue]]`
in ra hàng/ô đúng shape; **không** Any/hack exhaustive; hai engine chung đường `builtin::call`).

**Giới hạn đã biết (ghi nhận để A0 quyết, KHÔNG tự vá):**
- **Không dựng được `Null` trần** từ source (đối số 0 giống `Json.Null` — tên trần `Null` resolve về
  `Json.Null` theo thứ tự khai). Constructor `SqlValue.Null` (không-đối-số) không tồn tại ở tầng giá trị
  (mẫu `Json.Null`). NULL đi vào db qua `INSERT … VALUES (NULL)` (không cần ctor) và ra match bằng `Null:`.
  Đề nghị cổng tích hợp xem xét sugar/alias (ngoài phạm vi VD này).
- `list[SqlValue]` **không** chấp nhận biến thể enum khác (`Json`) hay sai arity — kiểm ở runtime `kiem()`
  (trap `VS0501`).

## 2. §3/§4 — RowSet + API prototype (hàm module, chưa method)

```text
use std.sqlite
std.sqlite.open(path: str)                 -> Result[str, Error]        # ':memory:' hoặc đường dẫn
std.sqlite.open_with(path: str, create: bool, busy_timeout_ms: int) -> Result[str, Error]
std.sqlite.close(h: str)                   -> Result[(), Error]
std.sqlite.execute(h: str, sql: str, params: list[SqlValue]) -> Result[int, Error]   # affected rows
std.sqlite.query(h: str, sql: str, params: list[SqlValue])   -> Result[RowSet, Error]
std.sqlite.begin(h: str) / commit(h: str) / rollback(h: str) -> Result[(), Error]
std.sqlite.in_transaction(h: str)          -> Result[bool, Error]       # sqlite3_get_autocommit() == 0 ⇒ false
```

- Module `std.sqlite` (Addendum K); `SqlValue` **thuộc module** (§1). `:memory:` hợp lệ; **không URI**
  `file:…`; **tên rỗng** & `":memory:"+đuôi` từ chối (trap `VS0501`, §4). `create:false` từ chối tạo mới;
  chưa read-only mode (đúng lệnh A0). Handle là **chuỗi mờ** `db:N`.
- Mỗi câu SQL **một statement** (`rusqlite` báo `MultipleStatement` ⇒ trap `VS0501`). Bind **đủ số
  lượng**, thiếu/thừa ⇒ trap `VS0501` (đối chiếu §10: `.err VSxxxx`). Params positional `list[SqlValue]`;
  không implicit conversion, không named params.
- `db.query()` method API **không** mở rộng (Addendum "K handle/method"); prototype dùng **hàm module**;
  tên module/`fn`/`open_with` và chữ ký sẽ trình A0 ở cổng tích hợp (xem §9).
- `Result[_, Error]` mọi chỗ (VD-222). Lỗi **người gọi** (danh tính/đối số sai) = trap `VS0501`;
  **lỗi dữ liệu** (SQLite từ chối/authorizer/busy/giới hạn) = `Err(Error)` (§7 lệnh H, §6 hợp đồng).

## 3. §5–§9 — DbHost, handle, limits, authorizer, capability

### DbHost (trait, vis-interp — **không** biết backend native)

Đúng mẫu `NetHost` (SHA `crates/vis-interp/src/builtin_db.rs`): `db_open/db_close/db_execute/db_query/
db_in_transaction/db_cleanup/db_path_of`. `Host::db()` mặc định `None` ⇒ `VS0604` (host không SQLite vẫn
chạy bình thường các tính năng khác). VM gọi cùng đường `vis_interp::builtin::call` (chung hai engine).

### Handle contract

- `db:N` — `N` là bộ đếm đơn điệu của **phiên host**, **không tái sử dụng**; tombstone giữ tới khi phiên kết thúc
  (đóng `db:1` rồi mở `db:2`, id cũ vẫn báo "đã đóng"). Chủ sở hữu phiên == host sinh handle; kiểm **loại
  + tồn tại + đã đóng** ở mọi thao tác; `"db:99"`, `"tcp:0"`, `"db:"`, `"db:1x"` ⇒ trap `VS0501`.
- `"db:0"` **không** là bằng chứng quyền — quyền là phiên sở hữu + authorizer + capability (§7), không phải
  nội dung handle. Handle không phải credential; `vis test` mỗi ca một phiên riêng.
- **Cleanup mỗi phiên Host**: cuối `run`, mọi connection còn mở đều đóng (best-effort, **không**
  auto-commit — SQLite hủy transaction dở khi đóng); rò **chỉ** báo ở `--explain-perms` (`SQLITE leak:
  <n> connection chưa đóng (đã đóng khi kết thúc phiên)`); stderr mặc định **không** in. Hai test
  integration (`ro_canh_bao_chi_diagnostics`, `nhieu_lan_chay_trong_cung_mot_process`) xác nhận
  `leak: 2`/`leak: 1` lặp lại qua **20 lần chạy**; mutation tắt cleanup ⇒ test đỏ (đã hoàn nguyên).

### Authorizer (Addendum 1 — **bắt buộc**, cài TRƯỚC prepare người dùng)

`sqlite3_set_authorizer` (rusqlite `Connection::authorizer`) cài ngay sau open, trước mọi prepare.
Chính sách (mọi chế độ, kể cả ngoài strict-caps) — **tắt ⇒ ca cấm đỏ** (§mutation):

| Hành động | Action code | Xử lý |
|---|---|---|
| `ATTACH` / `DETACH` | `SQLITE_ATTACH/DETACH` | **Deny** (prepare-time error "not authorized") |
| `VACUUM` / `VACUUM INTO` | (không có code riêng — VACUUM tự `ATTACH` nội tại) | internal ATTACH bị Deny ⇒ câu lệnh lỗi, **không tạo tệp** |
| `SELECT load_extension(...)` / `sqlite_load_extension` | `SQLITE_FUNCTION` | **Deny** (+ mặc định SQLite tắt sẵn load_extension ở tầng C) |
| `PRAGMA user_version = …` (ghi) | `SQLITE_PRAGMA` (có value) | **Deny**; **đọc** `PRAGMA user_version` (không value) cho phép |
| `PRAGMA table_info(…)` | `SQLITE_PRAGMA` | **Allow** |
| `PRAGMA` khác (`journal_mode`, `wal_checkpoint`, `temp_store`…) | `SQLITE_PRAGMA` | **Deny** |
| DDL/DML thường, `BEGIN/COMMIT/ROLLBACK`, `SAVEPOINT`, `SELECT/READ/INSERT/UPDATE/DELETE/CREATE_*/DROP_*/` | — | **Allow** (MVP — hợp đồng §7.6) |

`VACUUM INTO` và `VACUUM` trơn **luôn** bị chặn (tạo tệp tuỳ ý / quét lại db) — không file nào được
tạo ở ca `attach_bi_chan_va_khong_tao_tep` (integration, assert `!db.exists()`).
Authorizer deny ⇒ **Err(Error)** dữ liệu với thông điệp rõ ("bị từ chối bởi quyền (authorizer)"),
không lộ đường dẫn/nội bộ.

### Capability / filesystem (§8 hợp đồng — cổng chặt nhất)

**Quyết định (đề xuất A0 ratify):** **từ chối file-backed khi có bất kỳ `cap[...]` khai báo hoặc
`--strict-caps`**; chỉ `:memory:` được phép. Không cap ⇒ file-backed được (§6 hợp đồng: mặc định OS lưu).
Đây là **từ chối rõ**, không hạ mức bảo vệ âm thầm. Bằng chứng:

- `--strict-caps` + file-backed ⇒ `exit 2`, `[quyền] std.sqlite file-backed chưa có bằng chứng kiểm soát I/O (journal/WAL/temp) …`; `:memory:` dưới strict-cap **vẫn xoan**.
- `fn main(d: cap[Dir] @ p"…" (write))` + file-backed trong thư mục đó ⇒ **vẫn từ chối** (I/O của SQLite không qua `FsGuard`; `check-then-open không đủ` — đúng lệnh Addendum 2).
- `:memory:` **không chạm đĩa**; `temp_store=MEMORY` bắt buộc + `trusted_schema=OFF` (đặt trước authorizer).
- Symlink/reparse: trong strict-caps không mở được gì (từ chối mọi file-backed) ⇒ không có cửa sổ TOCTOU;
  trong chế độ mặc định (không cap), đường dẫn thường được xử lý như file thường (tài liệu hoá, không hứa an toàn).
- **WAL đã có**: mở được, đọc được dữ liệu đã commit (test `db_da_co_wal_van_mo_duoc`); vis **không** tự
  quyết `journal_mode`; WAL/journal/-shm phụ thuộc phiên SQLite — **khóa cứng strict-caps từ chối file-backed** thay vì kiểm soát từng tệp phụ.
- Tệp tạm `sqlite-temp-*`: `temp_store=MEMORY` ⇒ không sinh; `PRAGMA temp_store` mặc định `0/FILE` **không** còn hiệu lực.

### Limits (vượt ⇒ `Err`, không trả dữ liệu cắt)

| Giới hạn | Giá trị | Vượt ⇒ | Ca xác nhận |
|---|---|---|---|
| rows mỗi query | **10_000** | `Err` | `gioi-han-hang` (.out) |
| 1 row (text+blob) | **1 MiB** (`1_048_576`) | `Err` | `gioi-han-row` (.out) · unit `row_vuot_1mib_err` |
| tổng kết quả | **16 MiB** (`16_777_216`) | `Err` | `gioi-han-tong` (.out) · unit `tong_vuot_16mib_err` |
| SQL text | **64 KiB** (`65_536`) | **trap VS0501** (đối số gọi) | `gioi-han-sql` (.err) |
| params mỗi câu | **1_000** | **trap VS0501** | `gioi-han-params` (.err) |
| connections mỗi phiên | **16** | **trap VS0501** | `gioi-han-ket-noi` (.err) · unit `connection_thu_17_bi_tu_choi` |
| busy_timeout mặc định | **5_000 ms** (cấu hình trong `0..=60_000`) | vượt khoảng ⇒ trap VS0501 | unit `busy_ngoai_khoang_bi_tu_choi` |
| wall clock mỗi câu | **5_000 ms** (`progress_handler`, ngắt cứng) | `Err` (OperationInterrupted) | integration `gioi_han_thoi_gian_5s_bi_ngat` (query dài bị ngắt ~5 s) |
| statements mỗi `execute`/`query` | **1** | trap VS0501 | `multi-statement` (.err) · unit `multi_statement_bi_tu_choi` |
| bind thiếu / thừa | đúng số `?` | trap VS0501 | `bind-thieu`/`bind-thua` (.err) · unit `bind_thieu_thua_bi_tu_choi` |

**Deviation có chủ đích (flag A0):** §10 ghi `.err VSxxxx` cho bind và limits; tôi **phân loại** theo
tính chất: lỗi **người gọi** (bare/SQL ngắn/params/handle/số câu) = trap `VS0501` (`.err` như yêu cầu);
giới hạn **phụ thuộc kết quả runtime** (rows/row/tổng) = `Err` **dữ liệu** (§6 hợp đồng: "Vượt ⇒ `Err`").
Corpus cho kết quả runtime in marker qua match (`.out`), không `.err`. §10 bảng row `.err` cho limits
vẫn chưa khớp — cần A0 quyết (§9).

## 4. Test matrix §10 — kết quả (hai engine, corpus `test/sem/sqlite/`, 27 ca)

`.out` (19): `mo-dong` · `tao-them-chon` · `null-roundtrip` · `blob-roundtrip` (rỗng + 4 byte) · `unicode-text`
(Việt/日/😀) · `trung-ten-cot` (`select 1 as x, 2 as x`, columns giữ thứ tự/trùng tên) · `cot-rong` (0 hàng) ·
`chuoi-chua-nhay` (`o'clock`) · `giao-dich-rollback` · `giao-dich-loi` (không auto-commit) · `in-transaction`
(false/true/false) · `commit-ro` · `ddl-dml` · `option-enum` (đủ 5 biến thể) · `gioi-han-hang` ·
`gioi-han-row` · `gioi-han-tong` · `attach-bi-chan` (authorizer) · `pragma-doc`.

`.err` (8): `bind-thieu` · `bind-thua` · `multi-statement` · `handle-gia` · `handle-dong` ·
`gioi-han-sql` · `gioi-han-params` · `gioi-han-ket-noi` (tất cả `VS0501`, exit 101, `steps>0`).

Rust integration `crates/vis-cli/tests/sqlite.rs` (12 ca, hai đường byte-identical): open/close memory;
leak warning chỉ diagnostics; rò qua **20 lần chạy**; hai connection tranh khóa + `busy_timeout`
(`database is locked`, không treo); `--strict-caps` từ chối file-backed; URI/tên rỗng từ chối; `create:false`
+ thiếu tệp ⇒ `Err` dữ liệu; phiên bản SQLite + compile options log ở `--explain-perms`; WAL-db đọc được;
ATTACH/VACUUM INTO không tạo tệp. Unit tests trong `db.rs` (23 ca) phủ: version/compile options thật
(`3.50.2`, `ENABLE_FTS5`…), handle ids đơn điệu, temp_store=MEMORY (set **trước** authorizer ⇒ người dùng
không đụng được), limits chính xác, `in_transaction` theo verb, authorizer policy, cleanup đếm, WAL-db,
`db_da_co_wal_van_mo_duoc`.

**Parity**: `vis test test/sem` = **514/514** và `vis test --vm test/sem` = **514/514** (byte-identical
stdout/exit/диагностика). `vs_interp` bỏ qua nhóm `sqlite/` (cần DbHost native — đếm rõ 27 ca); parity thật
qua `vis test(--vm)` + integration. `diag_cmp` cập nhật 418/53/43. `roundtrip`, `fuzz_mutate` (418 module
nguồn), `vis_test`, `sem`, `diag_cmp`, `stdlib_ty` — mọi khoá cứng cập nhật kèm lý do.

## 5. Mutation (kết quả thật, hoàn nguyên)

| Mutation | Kỳ vọng | Kết quả |
|---|---|---|
| Tắt `conn.authorizer(Some(chinh_sach))` | ca cấm đỏ | **Đúng**: `authorizer_chan_attach_pragma_ghi_load_extension` RED; `attach_bi_chan_va_khong_tao_tep` RED (`KHONG DUOC PHU`); corpus `attach-bi-chan` in `KHONG DUOC PHU ×4` (chỉ `load_extension` vẫn bị C-level default chặn). **Hoàn nguyên ⇒ xanh lại.** |
| Tắt `DbHost::db_cleanup` ở cuối phiên | ca thoát sớm đỏ | `ro_canh_bao_chi_diagnostics` RED (mất dòng `SQLITE leak:`). **Hoàn nguyên ⇒ xanh lại.** |

## 6. Dependencies & phân phối

- `rusqlite = "=0.37.0"` (MIT) + `libsqlite3-sys 0.35.0` (feature `bundled` → **bundled SQLite 3.50.2**,
  sqlite3.c public domain; không cần dev kit HĐH; build proven trên máy này với `cc = gcc 15.2.0` +
  `cargo 1.99.1`; `libsqlite3.a` sinh trong `target/debug/build/…`). `Cargo.lock` đã cập nhật.
  **Không** tự viết binding C; **không** `links` native ngoài rusqlite; **không** raw FFI ở ViScript;
  **không** loadable extension (tắt sẵn + authorizer chặn); giao diện = `DbHost` trait + `Value`.
- Phiên bản SQLite + compile options **log ở diagnostics** (`--explain-perms`):
  `SQLITE version=3.50.2 (libsqlite3-sys bundled) compile_options=ATOMIC_INTRINSICS=1,COMPILER=gcc-15.2.0,…,ENABLE_FTS5,ENABLE_LOAD_EXTENSION,ENABLE_RTREE,MALLOC_SOFT_LIMIT=1024,MAX_ATTACHED=10,MAX_COLUMN=2000,MUTEX_PTHREADS,TEMP_STORE=1,THREADSAFE=1,USE_URI,…`
  (đo ở máy build; **không** hứa platform khác — §13). `ENABLE_LOAD_EXTENSION` có trong build nhưng
  SQL `load_extension` bị tắt ở tầng C + chặn ở authorizer.

## 7. A0 ràng buộc §13 (portability) — tuân thủ

1. **DbHost độc lập backend**: đúng — không kiểu rusqlite nào vào trait/`StdFn`/`Value`/diagnostic. ✓
2. **Host thiếu SQLite ⇒ `VS0604`**: `Host::db()` mặc định `None`; cả hai engine nhận thông điệp giống nhau. ✓
3. **Path/handle là chuỗi dữ liệu**: không OS path/pointer vào API; handle là `db:N` mờ theo phiên. ✓
4. **ViFastOs port**: **chưa** mở (đúng lệnh). Đường A/B (VFS/POSIX-tương-thích) để đợt riêng. Trong
   đợt này cũng **không** `PRAGMA journal_mode=WAL`/`synchronous`/`temp_store` ngoài `MEMORY`/`trusted_schema=OFF`.
5. **Bundled không hứa nền khác**: chỉ claim Linux; không đụng `pha1-rc1`; không tuyên bố Windows/macOS
   (Tier 2 không mở lại). Windows/macOS re-verify native build = công việc cổng tích hợp.
6. **Đo trước khi hứa**: version+compile options log được (trên); không con số hiệu năng/size nào được hứa.

## 8. `api_dong_bang.rs` & freeze — **không regen**

`agent/API-STD.txt` giữ nguyên (không thêm `sqlite.*`). `api_dong_bang::api_std_khop_code` **đỏ thiết kế**
với thông điệp `THÊM (9): sqlite.begin/close/commit/execute/in_transaction/open/open_with/query/rollback`.
Chỉ sửa tối thiểu `kieu_ty()` trong `api_dong_bang.rs` (+5 arm render cho `Ty::{SqlValue,RowSet,ResultIntErr,
ResultBoolErr,ResultRowSetErr}` — **không** đổi logic so khớp, **không** regen tệp). `api_codes` và `api_vbc1`
**xoan**. Freeze chỉ regen ở cổng tích hợp sau khi A0 phê duyệt chữ ký (`VIS_FREEZE_REGEN=1 …` — Agent 0/VD sau).

## 9. Cần A0 ratify ở cổng tích hợp (checklist)

1. **API signatures**: `open(path)`/`open_with(path, create, busy)` (opts-record bị bỏ vì thiếu `Ty` record
   ẩn danh; ghi nhận mọi lựa chọn `open_with` thay `open(.., opts)`); `query→RowSet`; `in_transaction→Result[bool, Error]`;
   `columns()` **chưa** cài; `begin/commit/rollback`. Hàm module, **không** method dispatch.
2. **Tên/cách đặt**: module `std.sqlite`; enum `SqlValue` (tc `ColumnName/RowSet/…` giữ nguyên); biến thể
   `Null/Integer/Real/Text/Blob` và field `v`.
3. **Strict-caps/file-backed decision** (§3): "refused whenever any `cap`‖`--strict-caps`‖file-backed; only
   `:memory:`" — ratify hay chọn VFS-kiểm-proof khác.
4. **§6 vs §10 rows**: limits (rows>10k…) tôi để `Err` **dữ liệu**, không `.err VSxxxx` — ratify classification.
   bind thiếu/thừa = trap VS0501 (đúng §10); authorizer deny = `Err(Error)` (§7 H) — ratify.
5. **§10 row `.err` cho limits**: corpus in marker `.out` — chấp nhận hay đổi sang trap.
6. **`VACUUM` trơn** cũng bị chặn (internal ATTACH bị deny) — đúng §H ("VACUUM INTO/VACUUM nói chung") chưa
   có ratify rõ cho bản trơn; giữ chặn.
7. **§8 danh sách match** `SQLite *`: giữ `ANALYZE/REINDEX/DDL/DML/SAVEPOINT/…` cho phép (MVP) — ratify hay siết.
8. **`wall clock` qua `progress_handler`** — đã cài 5_000 ms (ngắt → `Err`); ratify giá trị/cách đo.
9. **NaN**: SQLite lưu NaN ⇒ NULL (đã đo) — Real(NaN) round-trip thành `Null`; `±Inf` lưu thành Real(±inf). OK?
10. **Không có `pid` salt trong handle id** (mẫu `NetHost`): session-ownership là ranh giới quyền, handle
    không phải credential; defense-in-depth pid salt **không** thêm. Ratify hay yêu cầu salt.
11. **API freeze regen**: chỉ sau ratify (`VIS_FREEZE_REGEN=1 cargo test -p vis-cli --test api_dong_bang`).

**Cần gì từ A0 để đóng Gate 1**: review chéo bởi Agent 2 khác tác giả; quyết items ở §9; phê duyệt
 chữ ký để regen freeze; Windows/macOS re-verify nếu portable build thay đổi; (không) phát hành.

## 10. Những chỗ tôi muốn vượt lệnh (để A0 quyết — tôi **không** tự làm)

1. `api_dong_bang.rs` **kieu_ty arms** (+5 arm, không đổi logic freeze) — bắt buộc để workspace biên dịch
   được (enum `Ty` mới không exhaustive). Freeze snapshot **không** đổi.
2. `test/sem` count: 486/487 → **514** (487 upstream + 27 sqlite) ở mọi khoá cứng (sem/vis_test/diag_cmp/
   roundtrip/fuzz_mutate/CHEAT-SHEET/FACTS) — cập nhật bắt buộc để gate xanh.
3. `CHEAT-SHEET`/`VISCRIPT-ERGONOMICS-FACTS.md` chỉ dòng đếm corpus; **không** đổi nội dung khác.
4. **Không** đụng `docs/ViScript-thiet-ke.md`, **không** đụng SPEC-lex/SPEC.md, **không** đụng
   `api_dong_bang.rs` ngoài arms; **không** merge/push; **không** claim Windows/macOS.
5. **`RowSet` `record`** được đăng ký **dựng sẵn** `RecId(1)` (như `Error`) — không phải module-scoped code
   riêng; cùng bucket với `SqlValue` (không prelude API; tên dự trữ tệp). Nếu A0 muốn `RowSet` tự khai
   trong source của `std.sqlite` (không builtin), cần cơ chế mới — ngoài lệnh này.
6. **Không** regen `agent/API-STD.txt`; **không** `VIS_FREEZE_REGEN=1`.
7. **Không** `PRAGMA journal_mode=WAL`/`synchronous`/`temp_store` ngoài `MEMORY`; **không** thêm
   `listen`/`transaction(fn…)` wrapper; **không** named params; **không** streaming cursor; **không** WAL-mode customization.
8. **`load_extension` message** từ SQLite có echo SQL fragment ("not authorized to use function: …") —
   giữ nguyên (thông điệp rõ, không lộ đường dẫn thật vì user tự gõ); nếu A0 muốn che, phải wrapper riêng.
9. **`":memory:"`** là chuỗi duy nhất in-memory; `:memory:x` và `FILE:`/`file:` đều từ chối — strict hơn có thể.

## 11. Kết quả cổng (số đo thật)

| Cổng | Kết quả |
|---|---|
| `./vis-dev check` | **PASS** (fmt · clippy `-D warnings` · R1..R6 · self-test · vis explain · báo cáo · doc-examples 24 khối) |
| `cargo test --workspace --no-fail-fast` | **148 binaries; duy nhất `api_std_khop_code` FAIL** (freeze, §8) |
| `vis test test/sem` | **514/514** (463+1 `.vis` + 50 `.vish`) |
| `vis test --vm test/sem` | **514/514** (byte-identical) |
| `crates/vis-cli/tests/sqlite.rs` | 12/12 · `db.rs` unit 23/23 |
| Fresh clone (local, đã xoá) | `check` PASS · workspace 148 + freeze-red duy nhất · `sem` 514/514 ×2 |
| `crates/vis-bytecode/tests/roundtrip.rs` | 487→514 asserts cập nhật |
| `fuzz_mutate` | 418 module nguồn |

## 12. Windows/macOS

**Không tuyên bố.** Linux only; `cargo check --target` matrix không chạy Windows/macOS target ở đợt này;
`pha1-rc1` không chạm. SQLite bundled build chỉ chứng minh trên máy Linux này (gcc 15.2.0).

## 13. Các finding/trước sau

- `vis test --vm` cần `--no-cache` hoặc cache valid — như upstream. UTF-8 invalid trong TEXT ⇒ `Err` (kiểm riêng).
- Bind thiếu/thừa và multi-statement = trap `VS0501` (§10); kết quả limits = `Err` (§6) — phân loại đã ghi ở §9(4).
- `vs_interp` không thể chạy DbHost native (host chỉ ở `vis-cli`) ⇒ sqlite/ bỏ qua 27 ca ở differential
  in-process; parity thật qua `vis test(--vm)` + integration. `.vish` chưa có ca sqlite (chưa phơi nghi vấn).
- `read.vis` scratch nằm ở `/tmp/opencode` (không commit); `/tmp/opencode` còn các scratch nhỏ (logs,
  enum*/lim*/han*/nan.vis…) — có thể xoá nếu cần; repo **không** có file rác (`git status` sạch).
- `agent/QUEUE.md`/STATE/DEM **không** cập nhật — Gate 1 là lệnh mở, hàng đợi do A0 điều phối.
