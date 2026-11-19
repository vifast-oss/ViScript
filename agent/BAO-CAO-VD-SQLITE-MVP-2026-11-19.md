# Báo cáo — VD SQLite MVP · Gate 1 (thiết kế + prototype nội bộ)

> **Agent 2 → A0/Agent 1** · 2026-11-19 · nhánh `rev/vd-sqlite-mvp-2026-11-19` · worktree `sqlite-mvp-2026-11-19`.
> **Chưa merge `main`, chưa phát hành, chưa tuyên bố Windows/macOS.** Prototype **hoạt động thật** với
> rusqlite bundled; **API chưa đông băng** (freeze `API-STD.txt` **không** regen — xem §8).
> Base: branch = `origin/main` `533faf3` + 10 commit (`f673fbf` feat · `e2a16eb` R1 split · `cf763af` wall-clock · `c2e4b2e` typo · `70c985d` pre-merge P1..P5 · `973ae85` blockers 1–13 · `8fd5134` fix review chéo F-A/F-B · `fd243cf` path `/` Windows · `dd781f5` freeze narrow +9).
>
> **CẬP NHẬT 2026-11-19 (đợt pre-merge, blockers 1–13 do A0 chốt):** xem **§14** — API shape do A0
> duyệt (module `std.sqlite`; `SqlValue` module-scoped; `RowSet`; `open`/`open_with`/`in_transaction`);
> **mọi lỗi dữ liệu nay là `Result.Err`** (`DAU_LOI_GOI` bị xoá); corpus **525 ca** (không còn `.err`
> nhóm `sqlite/`); limits mở rộng; authorizer chuyển sang allow-list theo action code; handle là
> **token 128-bit** (`db:<32 hex>`); `--sqlite-debug` (opt-in). §1–§13 là trạng thái lúc Gate 1 —
> §14 ghi đè mọi chỗ lệch.

## 0. Kết luận nhanh

- **Toàn bộ test workspace xanh, trừ đúng một ca:** `api_dong_bang::api_std_khop_code` — đỏ **theo
  thiết kế Gate 1** (API chưa qua cổng tích hợp; `api_dong_bang.rs` **không** được regen theo lệnh).
  `api_codes_khop_code` và `api_vbc1_khop_code` (mã chẩn đoán + thẻ VBC1) **xanh** — không đổi ABI.
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
khi tệp đã `use std.sqlite` (`Lowerer::named_type` → `VS0301` nếu không); ctor `SqlValue.Integer(1)` và
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
- **Bare `SqlValue.Null` trap `VS0501`** (đo: check xanh, runtime `VS0501 tên SqlValue chưa hỗ trợ ở VS0`);
  còn **`SqlValue.Null()` constructor zero-arg chạy đúng** (interp và VM in `null-ctor`; corpus ca `sqlvalue_null_ctor_chay`
  trong `crates/vis-cli/tests/sqlite.rs` đã ghim). NULL đi vào db qua `INSERT … VALUES (NULL)`; ra match bằng `Null:`.
  Bare-name enum ctor sugar là một finding ngoài phạm vi VD này — trình A0 ở cổng tích hợp (đề nghị, không tự đổi).
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
  chưa read-only mode (đúng lệnh A0). Handle là **token mờ** `db:<32 hex>` — xem §14.5.
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

- ~~`db:N`~~ → **token 128-bit** `db:<32 hex thường>` (xem §14.5): entropy thời gian+pid+counter+stack-ASLR,
  băm 2 vòng splitmix64; tra cứu khớp **toàn bộ**; `db:0`/`db:00`/`db:2`/alias/đã đóng ⇒ `Err` **không** trap;
  token một lần, không tái sử dụng, handle đã đóng không trỏ connection mới; thông điệp lỗi **không** echo token.
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
| SQL text | **64 KiB** (`65_536`) | `Err` *(trước: trap VS0501 — §14.2)* | `gioi-han-sql` (.out) |
| params mỗi câu | **1_000** | `Err` *(trước: trap VS0501 — §14.2)* | `gioi-han-params` (.out) |
| connections mỗi phiên | **16** | `Err` *(trước: trap VS0501 — §14.2)* | `gioi-han-ket-noi` (.out) · unit `connection_thu_17_bi_tu_choi` |
| busy_timeout mặc định | **5_000 ms** (cấu hình trong `0..=60_000`) | vượt khoảng ⇒ `Err` *(trước: trap VS0501 — §14.2)* | unit `busy_ngoai_khoang_bi_tu_choi` |
| wall clock mỗi câu | **5_000 ms** (`progress_handler`, ngắt cứng) | `Err` (OperationInterrupted) | integration `gioi_han_thoi_gian_5s_bi_ngat` (~5 s) |
| statements mỗi `execute`/`query` | **1** | `Err` *(trước: trap VS0501 — §14.2)* | `multi-statement` (.out) · unit `multi_statement_bi_tu_choi` |
| bind thiếu / thừa | đúng số `?` | `Err` *(trước: trap VS0501 — §14.2)* | `bind-thieu`/`bind-thua` (.out) · unit `bind_thieu_thua_bi_tu_choi` |
| **mới §14.6** — mỗi param text/blob | **1 MiB** | `Err` | unit `param_vuot_1mib_bi_tu_choi_va_dung_limit_thi_ok` |
| **mới §14.6** — tổng payload params/lời gọi | **16 MiB** | `Err` | unit `params_tong_vuot_16mib_bi_tu_choi_va_dung_limit_thi_ok` |

**Deviation có chủ đích (flag A0):** §10 ghi `.err VSxxxx` cho bind và limits; tôi **phân loại** theo
tính chất: lỗi **người gọi** (bare/SQL ngắn/params/handle/số câu) = trap `VS0501` (`.err` như yêu cầu);
giới hạn **phụ thuộc kết quả runtime** (rows/row/tổng) = `Err` **dữ liệu** (§6 hợp đồng: "Vượt ⇒ `Err`").
Corpus cho kết quả runtime in marker qua match (`.out`), không `.err`. §10 bảng row `.err` cho limits
vẫn chưa khớp — cần A0 quyết (§9).

## 4. Test matrix §10 — kết quả Gate 1 (hai engine, corpus `test/sem/sqlite/`, 27 ca)
> Hiện hành sau đợt pre-merge: **38 ca** `sqlite/`, không còn `.err`; tổng corpus **525 ca** — xem §14.13.

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

## 5. Mutation (kết quả thật, hoàn nguyên — Gate 1)
> Đợt pre-merge lặp lại mutation (authorizer-off, cleanup-off) với kết quả tương tự — xem §14.13.

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
3. **Path/handle là chuỗi dữ liệu**: không OS path/pointer vào API; handle là token mờ theo phiên (§14.5). ✓
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


---

## Phụ lục R — Review độc lập 2026-11-19 (verdict SHIP; bổ sung dữ kiện cho A0)

**Reviewer (Agent 2 khác tác giả)**: SHIP Gate 1; không cheat; không đụng SPEC-lex/freeze/VBC1/`pha1-rc1`; hai engine byte-identical mọi ca; corpus 27 ca chạy thật; unit 23/23; mutations OK; `.vish` chưa có; Windows/macOS không tuyên bố.

### Pre-merge (Agent 1 xử lý — đã vá trong commit tiếp theo)

| # | Vị trí | Vấn đề | Cách xử lý |
|---|---|---|---|
| P1 | `crates/vis-cli/src/db.rs:380` | `PRAGMA temp_store/trusted_schema` bị nuốt lỗi | propagate `map_err` |
| P2 | `crates/vis-cli/Cargo.toml` comment: "SQLite 3.51.3" | sai — đo thật 3.50.2 | comment sửa |
| P3 | §1 "Constructor SqlValue.Null không tồn tại ở tầng giá trị" | sai — `SqlValue.Null()` chạy đúng | §1 đính chính + corpus `sqlvalue_null_ctor_chay` |
| P4 | §9 thiếu câu hỏi/ dữ kiện tích hợp | xem N1–N13 dưới | annex này |
| P5 | `execute` cho câu trả hàng ⇒ thông điệp rusqlite raw (tiếng Anh) | classify/thông điệp | thông điệp cục bộ "lỗi khi thực thi: câu lệnh trả hàng — dùng `sqlite.query`" (data error → `Err`) |

### Ratify-sau / A0 quyết (flag, không tự vá)

- **N1**. Bare `SqlValue.Null` ⇒ trap `VS0501` (bare-name resolve); `Null()` OK. (Xem §1.)
- **N2**. `execute(h, "SELECT 1"…)` ⇒ `Err(Error)` "lỗi khi thực thi: câu lệnh trả hàng…" (data error, không phải caller trap). Classification cần ratify (Err vs VS0501).
- **N3**. Không có giới hạn **byte params** (một blob 1 GiB đi thẳng qua bind). §6 không yêu cầu; A0 chốt giữ hay thêm `MAX_PARAM_BYTES` (đề xuất 16 MiB).
- **N4**. `db_open` không có progress_handler/wall-clock (chỉ `execute`/`query`). A0 chấp nhận hay đòi bound?
- **N5**. (P1) PRAGMA host-set: sau vá sẽ propagate lỗi — A0 không cần quyết, chỉ ghi nhận §8 giờ có bằng chứng mạnh hơn.
- **N6**. `TEMP_STORE=1` (pragma `MEMORY` runtime tính) — bundled giữ; không patch C. A0 chấp nhận hay đòi build-time `TEMP_STORE=3`?
- **N7**. Temp B-tree lớn ⇒ chiếm RAM, không ghi đĩa (MEMORY) — chấp nhận MVP (output cap 16 MiB); A0 ghi nhận hay thêm `cache_size` bound?
- **N8**. ~~`db:00`/`db:+0` alias~~ — **đã giải quyết ở §14.5**: token hex 128-bit một lần, khớp toàn bộ chuỗi; alias không tồn tại.
- **N9**. `.vish` (chế độ lệnh) chưa có đường gọi `sqlite.*` (báo cáo ghi). A0 xếp việc ngoài phạm vi/sau.
- **N10**. Dead code `db_path_of` + unused balance `kich_thuoc` — dọn vệ sinh tích hợp (Agent 1).
- **N11**. `.vish` corpus sqlite — chỉ nếu N9 được mở.
- **N12**. In-process multi-session (embedding) chưa có test đúng nghĩa; nếu cần byte-identical leak-proof ⇒ test riêng.
- **N13**. `load_extension` deny informational fragment rusqlite lọt trong thông điệp — A0 có muốn che?

### Đã xác nhận trong review (không phải câu hỏi)

- Authorizer = ranh giới thực; ATTACH/DETACH/VACUUM(INTO)/ghi `user_version`/`load_extension` chặn mọi chế độ; PRAGMA chỉ `table_info` + đọc `user_version`; case-insensitive; `temp_store`/`trusted_schema` ON, user không đè được.
- Strict-caps: từ chối file-backed rõ ràng (mọi cap/`--strict-caps`), chỉ `:memory:`; cap[Dir] phủ đúng thư mục vẫn exit 2; không tạo tệp.
- Handle: không reuse ID; từ chối giả/đóng/sai loại; `"db:0"` không là bằng chứng; phiên sở hữu + authorizer + cap là quyền thật.
- Limits: rows 10k/row 1 MiB/total 16 MiB/SQL 64 KiB/params 1000/connections 16/busy 5 s (cấu hình hữu hạn); vượt ⇒ `Err`; multi-statement trap.
- Freeze (`API-STD.txt`) nguyên; `api_dong_bang.rs` chỉ +5 arm render; `api_codes`/`api_vbc1` xanh; `api_std_khop_code` đỏ **đúng thiết kế**.
- Windows/macOS: không tuyên bố; không đụng SPEC-lex/VBC1/`pha1-rc1`/thiet-ke; determinism `:memory:`, không network.

---

## 14. Đợt pre-merge (2026-11-19) — blockers 1–13 của A0 (đợt này: **chưa Gate 2 / chưa freeze**)

> **Trạng thái API:** A0 đã **duyệt API shape** — module `std.sqlite`; `SqlValue` enum thuộc module
> (`EnumId(1)`); `RowSet` cho query (`RecId(1)`); `open`/`open_with`/`in_transaction`; **không** đổi
> VBC1, **không** thêm `Value` variant, **không** đổi ABI; `SqlValue.Blob(bytes)` bọc `bytes` dựng sẵn
> (addendum BLOB-bytes của A0/A1 — `Type::Bytes`, `Value::Bytes`, VM xử lý sẵn). **Chưa cho Gate 2 /
> không freeze** (`api_dong_bang.rs` **không** unfreeze; `agent/API-STD.txt` **không** regen).
> **Windows/macOS: bản này KHÔNG re-verify** (chỉ Linux mới được chạy). **psvn build sẽ được chạy ngay**
> sau đợt này (agent master trigger); kết quả ghi sau ở vòng kế.

### 14.1 API nguyên văn (chữ ký + semantics, đơn vị, defaults)

```text
use std.sqlite   # 8 hàm module (chưa đông băng — cổng tích hợp quyết)

std.sqlite.open(path: str)                                      -> Result[str, Error]   # create = true (mặc định), busy_timeout_ms = 5000 (mặc định)
std.sqlite.open_with(path: str, create: bool, busy_timeout_ms: int) -> Result[str, Error]
std.sqlite.close(h: str)                                        -> Result[(), Error]
std.sqlite.execute(h: str, sql: str, params: list[SqlValue])    -> Result[int, Error]   # số dòng bị ảnh hưởng
std.sqlite.query(h: str, sql: str, params: list[SqlValue])      -> Result[RowSet, Error]
std.sqlite.begin(h: str)                                        -> Result[(), Error]
std.sqlite.commit(h: str)                                       -> Result[(), Error]
std.sqlite.rollback(h: str)                                     -> Result[(), Error]
std.sqlite.in_transaction(h: str)                               -> Result[bool, Error]
```

- **Defaults**: `sqlite.open(path)` ≡ `sqlite.open_with(path, true, 5000)`. `busy_timeout_ms` đơn vị
  **mili giây**, kiểu `int` (i64), khoảng hợp lệ `0..=60_000`; ngoài khoảng ⇒ `Err`. **Không có tham số
  `query_timeout_ms`** — wall-clock mỗi câu SQL là **5_000 ms** cố định ở tầng host (xem §14.11),
  chỉ override nội bộ cho tests/debug (`Db::moi_voi_query_ms`), **không** phải API `std.sqlite`.
- **Đơn vị**: `busy_timeout_ms` = ms; progress_handler budget = ms; `execute` trả **số dòng bị ảnh hưởng** (int).
- **Close semantics**: `close(h)` đóng connection (drop native); **không** auto-commit (transaction dở
  bị SQLite huỷ); handle đóng chuyển tombstone — **không** tái sử dụng, **không** trỏ connection mới;
  đóng lần hai ⇒ `Err` ("đã đóng"); **không** panic. Kết thúc phiên Host đóng mọi connection còn mở
  (best-effort), rò chỉ báo ở `--explain-perms` (VD-241 §9).
- **Params**: positional `list[SqlValue]`; thiếu/thừa ⇒ `Err` (`SQL cần N tham số, được M`); không
  implicit conversion, không named params.
- **Enum `SqlValue`** (thuộc module): `Null` · `Integer(int)` · `Real(float)` · `Text(str)` ·
  `Blob(bytes)`; field payload `v`. **`RowSet`**: `columns: list[str]`, `rows: list[list[SqlValue]]`
  (giữ thứ tự cột, trùng tên cột cho phép); `RowSet` không có method (`rs.columns`/`rs.rows` field).
- **Handle**: `str`, token mờ `db:<32 hex thường>` (§14.5). `close`/thao tác handle lạ/đã đóng/sai
  dạng ⇒ `Err` (không trap).
- Không tuyên bố `Handle` là type riêng (MVP dùng `str`); không tuyên bố `query_timeout_ms` param.

### 14.2 Phân loại lỗi (Blocker 1): **`DAU_LOI_GOI` bị xoá** — chỉ còn hai lớp ngoài `Result`

**Chính sách lỗi (mọi chế độ):**

| Lớp | Ví dụ | Về đâu |
|---|---|---|
| Tĩnh/checker-time | sai kiểu tham số, sai arity, non-`SqlValue` element cho `list[SqlValue]` | checker (tầng `kiem`) — trap `VS0501` (**không đổi phạm vi**) |
| Bất biến nội bộ bị phá (bug runtime) | — | chẩn đoán nội bộ theo cơ chế hiện hành (không đổi) |
| **Còn lại — lỗi dữ liệu/runtime** | SQL > 64 KiB · multi-statement · bind thiếu/thừa · params > 1_000 · connections > 16 · busy ngoài khoảng · **handle giả/đã đóng/sai loại/sai dạng** · timeout 5 s · **policy/authorizer từ chối** · lỗi SQLite mọi lớp · giới hạn kết quả | **`Result.Err` (`Err(Error)`) mọi chỗ — không trap `VS0501`** |

- **Impl**: `DAU_LOI_GOI` (`\u{2}VS0501:`) **đã xoá** khỏi `stdlib.rs` (const + nhánh `hop_nhat_fs`)
  và mọi call-site ở `crates/vis-cli/src/db.rs`; mọi lỗi host giờ **không** mang prefix ⇒
  `hop_nhat_fs` trả `Err(Error)` dữ liệu (cùng mẫu `DAU_QUYEN`, nhưng **không** cần prefix: host trả
  `String` thô là lỗi dữ liệu; chỉ `DAU_QUYEN` mới là môi trường/trap `VS0604`).
- `DAU_QUYEN` (`VS0604`) giữ nguyên (vượt quyền = **môi trường**, không phải dữ liệu chương trình).
- Corpus `test/sem/sqlite/`: **8 ca `.err` VS0501 → `.out`** (byte goldens): `bind-thieu`,
  `bind-thua`, `multi-statement`, `handle-gia`, `handle-dong`, `gioi-han-sql`, `gioi-han-params`,
  `gioi-han-ket-noi`. Unit/integration asserts chuyển từ `starts_with(DAU_LOI_GOI)` → chỉ kiểm
  thông điệp; integration `uri_va_ten_rong_bi_tu_choi`/`bind_thieu_hai_duong_cung_stderr` kỳ vọng
  exit 0 + `chan `.
- **`builtin_ls.rs`**: không tham chiếu `DAU_LOI_GOI` (ST-46 — `ls --long`), không đổi.
- **Deviation ghi nhận**: lỗi **động** nhưng **kiểu** (đối số `list[SqlValue]` chứa phần tử không phải
  `SqlValue`) vẫn là checker-time trap `VS0501` (đúng "sai kiểu"); `SqlValue` enum lạ ở runtime
  (`o_nhap` "được enum `{e}`") ⇒ `Err` (host-side, dữ liệu).

### 14.3 `execute` câu trả hàng ⇒ phát hiện **trước** step (Blocker 2)

- **Impl** (`thuc_thi`, `crates/vis-cli/src/db/mod.rs`): sau `conn.prepare` và **trước**
  `kiem_so_luong`/`execute`: `if stmt.column_count() > 0` ⇒
  `Err("câu lệnh trả hàng — dùng `sqlite.query` thay vì `sqlite.execute`")` — **không** bước nào
  bên trong: không bind, không step, **không** side effect; câu SQL không bị chạy lại.
- **Bằng chứng không chạy insert**: unit `execute_tra_hang_phat_hien_truoc_step_khong_side_effect`
  — `INSERT INTO t VALUES (1) RETURNING id` qua `sqlite.execute` ⇒ đúng thông điệp trên; sau đó
  `SELECT count(*) FROM t` = **0**; câu `SELECT 1` qua `execute` ⇒ cùng thông điệp.
- Corpus **`execute-tra-hang.vis/.out`** chứng minh qua binary: `chan câu lệnh trả hàng — …` rồi
  `Integer(0)` (bảng rỗng) — hai engine byte-identical.
- rusqlite `ExecuteReturnedResults` giữ như nhánh phòng thủ (không còn kỳ vọng chạm) — thông điệp
  nội bộ, không rò rusqlite thô.
- **Hệ quả hành vi (ghi nhận)**: PRAGMA *gán* trả về hàng (vd `journal_mode = WAL`) cũng bị
  `execute` từ chối ⇒ dùng `sqlite.query`. Không mở đường tắt.

### 14.4 NaN / ±Inf binding (Blocker 3)

- **Impl** (`o_nhap`): `Value::Float(f)` với `f.is_nan()` ⇒
  `Err("Real(NaN) — dùng `SqlValue.Null()` nếu muốn giá trị NULL")`; **không** map vào NULL;
  kiểm **trước** clone/bind.
- **±Inf**: được phép bind/lưu/đọc lại (IEEE double SQLite); **round-trip** −inf↔−inf, inf↔inf.
- **NULL từ db ⇒ giữ `Null` variant** (không sugar, không đổi thành NaN/Inf); corpus `null-roundtrip` giữ nguyên.
- **Bằng chứng**: corpus **`bind-inf.vis/.out`** (`1.0e999`/`-1.0e999` lưu, đọc lại `real -inf`/`real inf`;
  `0.0/0.0` (NaN) bị từ chối với đúng thông điệp) — hai engine byte-identical; unit
  `nan_bi_tu_choi_va_inf_roundtrip` (assert đúng message, ORDER BY r).

### 14.5 Handle (Blocker 4): **token 128-bit**, không đoán được từ output

- **Impl** (`token_ngau_nhien`, `db/mod.rs`): `"db:" + 32 hex thường` (128 bit). Entropy:
  `time_ns ^ pid<<rot13 ^ counter<<rot29 ^ stack-addr<<rot41 ^ const` → băm **2 vòng splitmix64**
  ⇒ 2×64 bit. **Không** cần nguồn random OS; đủ khó đoán ở phạm vi MVP (không claim bảo mật liên
  phiên; ranh giới quyền = phiên sở hữu + authorizer + cap — như hợp đồng).
- **Tra cứu khớp toàn bộ token** (`h.strip_prefix("db:")` phải đúng 32 hex thường + equality):
  `db:0`/`db:00`/`db:2`/`db:99`/`db:`/`db:1x`/`tcp:…`/`DB:…`/hex HOA ⇒ `Err("handle sai dạng…")`;
  đúng dạng nhưng chưa mở ⇒ `Err("handle không tồn tại…")`; đã đóng ⇒ `Err("handle đã đóng")`.
  **0 case trap, 0 panic** — mọi đường đều `Err`.
- **Token không tái sử dụng**; đóng không giải phóng token (tombstone tới hết phiên); handle đã
  đóng **không** trỏ sang connection mới; collision guard vòng lặp khi mở.
- **Thông điệp lỗi không echo token** (trước đây `{h}` bị in vào thông điệp — đã bỏ): output
  **không lộ** handle; corpus goldens tất định. Việc "handle nào" là do chương trình tự biết.
- **Tests**: unit `token_128_bit_hex_khong_doan_duoc_khong_tai_su_dung` (format 32 hex, hai mở
  khác token, đóng→mở mới không trùng, tombstone báo "đã đóng", biến thể token (uppercase/`…`/+1
  ký tự/+1 ký tự cuối) đều `Err`), `token_gia_sai_dang_bi_tu_choi` (mọi dạng sai + token đúng dạng
  giả + đóng rồi); corpus `handle-gia.out` ("chan handle sai dạng — cần `db:<32 hex thường>`"),
  `handle-dong.out` ("chan handle đã đóng").
- **Điểm còn lại (ghi nhận)**: token dự đoán được **khi kẻ tấn công biết toàn bộ entropy nguồn**
  (ns-time+pid+ASLR address) — đủ cho MVP nội bộ, **không** tuyên bố chống giả mạo qua iterator;
  nếu A0 muốn CSPRNG OS sau này ⇒ đổi `token_ngau_nhien` (một chỗ).

### 14.6 Giới hạn params + row bytes (Blocker 5)

**Đếm byte**: TEXT = **UTF-8 bytes** (`s.len()`), BLOB = **`bytes.len()`**; int/float/bool = 8;
NULL = 0. **Metadata/tên cột/kiểu/variant KHÔNG tính vào cap** (ghi rõ trong thông điệp).
**Cộng dồn overflow-checked** (`usize::checked_add`); tràn ⇒ `Err`.

| Giới hạn | Giá trị | Kiểm | Vượt ⇒ |
|---|---|---|---|
| payload **mỗi** param TEXT/BLOB | **1 MiB** (`1_048_576`) | **trước** clone/bind (`chuan_bi_tham_so`) | `Err` |
| tổng payload params mỗi lời gọi (TEXT+BLOB) | **16 MiB** (`16_777_216`) | **trước** clone/bind; `checked_add` | `Err` |
| một row (TEXT+BLOB mọi ô) | **1 MiB** | **trước** allocate (raw ref, `checked_add`) | `Err` |
| tổng kết quả (TEXT+BLOB mọi hàng) | **16 MiB** | **trước** allocate; `checked_add` | `Err` |

- Không cắt im lặng; vượt ⇒ `Err` với **limit + action** (vd "chia nhỏ lời gọi", "chọn ít cột hơn",
  "cắt nhỏ dữ liệu (LIMIT)", "cắt nhỏ câu SQL", "đóng bớt (close) trước").
- **Params ≤ 1_000** giữ nguyên; **SQL ≤ 64 KiB** giữ nguyên; 16 connections/phiên giữ nguyên.
- **Tests**: unit `param_vuot_1mib_bi_tu_choi_va_dung_limit_thi_ok` (đúng 1 MiB ok; +1 byte Err;
  TEXT+BLOB riêng lẻ), `params_tong_vuot_16mib_bi_tu_choi_va_dung_limit_thi_ok` (16×1 MiB đúng
  giới hạn ok qua byte-check; 17×1 MiB ⇒ Err "16777216"), `row_vuot_1mib_err`, `tong_vuot_16mib_err`
  (định lượng qua raw ref — **không** clone trước khi kiểm), corpus không thêm ca (1 MiB+ payload
  trong corpus là noise; unit đủ chứng minh).

### 14.7 TEMP storage + page cache (Blockers 6, 10)

- **`HOST_PRAGMAS`** (`db/mod.rs`, dùng chung `db_open` và probe test):
  `PRAGMA temp_store=MEMORY; PRAGMA trusted_schema=OFF; PRAGMA cache_size = -2048;`
  — đặt **trước authorizer** trong `db_open` (propagate lỗi, không nuốt — P1 giữ nguyên);
  **không** đặt gì trước `prepare` câu SQL người dùng; user **không** thể đổi (`PRAGMA temp_store
  =FILE`/`PRAGMA cache_size = -1` ⇒ authorizer từ chối — có test).
- **Kiểm chứng hiệu lực** (probe connection raw với **đúng** chuỗi host — authorizer chặn các PRAGMA
  đó trên connection của host nên không đọc back qua đường user):
  unit `temp_store_bi_ep_memory_va_cache_2mib` ⇒ effective `temp_store = 2` (MEMORY),
  `trusted_schema = 0` (OFF), `cache_size = -2048` (2 MiB page cache). **`:memory:` dưới
  strict-caps vẫn cho phép** (A0 đã chốt dòng này — giữ, verify thật): integration
  `strict_caps_tu_choi_file_backed` (`:memory:` dưới `--strict-caps` exit 0) + unit
  `strict_caps_tu_choi_file_backed_nhung_cho_mem`. **User không đè được** (thử
  `PRAGMA temp_store=FILE` qua SQL ⇒ `Err`).
- **Hạn chế kiểm chứng (ghi nhận)**: không đọc được PRAGMA trên connection của host sau khi cài
  authorizer (đúng chính sách); probe chứng minh chuỗi PRAGMA **hiệu lực** ở SQLite; `db_open`
  propagate mọi lỗi nên chuỗi hỏng ⇒ không mở được (không mở im lặng với cấu hình sai).
- **`PRAGMA cache_size = -2048`** = **2 MiB** page cache mỗi connection (hàng âm = KiB).
  Đặt cùng vị trí `temp_store` — **có** đặt được trước authorizer (PRAGMA không nằm trong
  allow-list user, nhưng đây là **host-internal** execute_batch chạy trước authorizer — như
  `temp_store`).
- **Caveats/leaks**: không có leak mới (cleanup đóng best-effort, đếm `db_cleanup`); `VACUUM` bị
  chặn; **`max_page_count` chưa đặt** (không cần khi page cache có bound + temp MEMORY).
- **Memory budget story (rõ ràng)**: **KHÔNG hứa mọi OOM thành `Err`**. Bounds đã áp dụng:
  page cache **2 MiB**/connection · temp storage **MEMORY** · output cap **16 MiB** · params
  **16 MiB + 1 MiB/param**. OOM ngoài các bound này (vd SQLite nội bộ cần nhiều hơn) ⇒ hành vi
  hiện hành (SQLite tự xử/SQLITE_NOMEM đường khác) — **không** cam kết.
- **Workload lớn**: integration `workload_50k_rows_sum_avg_ca_hai_duong` — `INSERT` 50_000 hàng vào
  `:memory:` qua `WITH RECURSIVE`, rồi `SELECT sum(x), avg(x)`; output 24 B; **cả hai engine** pass;
  byte-identical.

### 14.8 Authorizer (Blocker 7): **allow-list theo action code** (không keyword scan)

`chinh_sach` (`db/mod.rs`) — **mặc định từ chối**; cho phép chính xác:

| Action (rusqlite `AuthAction`) | Ở đây là | Xử lý | Test |
|---|---|---|---|
| `CreateTable`/`CreateTempTable` | CREATE TABLE [TEMP] | **Allow** | corpus `ddl-dml`, `ddl-index-alter` · unit ma trận |
| `CreateIndex`/`CreateTempIndex` | CREATE INDEX [TEMP] | **Allow** | corpus `ddl-index-alter` · unit ma trận |
| `CreateTrigger`/`CreateTempTrigger` · `CreateView`/`CreateTempView` · `DropTrigger`/`DropTempTrigger` · `DropView`/`DropTempView` | DDL thường | **Allow** | unit ma trận · corpus `ddl-index-alter` (view) |
| `DropTable`/`DropTempTable` · `DropIndex`/`DropTempIndex` · `AlterTable` | DDL thường | **Allow** | unit ma trận · corpus `ddl-index-alter` |
| `Insert` · `Update` · `Delete` | DML CRUD | **Allow** | corpus `ddl-dml` · `giao-dich-*` · `bind-*` · unit ma trận |
| `Select` · `Read` · `Recursive` | đọc + `WITH RECURSIVE` | **Allow** | mọi corpus query · unit `WITH RECURSIVE` (10k hàng) · unit ma trận |
| `Transaction { operation: Begin \| Release \| Rollback \| Unknown }` | `BEGIN`/`COMMIT(!)`/`ROLLBACK` — `COMMIT` map về `Unknown` (rusqlite chỉ nhận diện BEGIN/RELEASE/ROLLBACK) | **Allow** | corpus `giao-dich-*` · `in-transaction` · unit ma trận (cả `Unknown`) |
| `Savepoint { .. }` | `SAVEPOINT`/`RELEASE`/`ROLLBACK TO` (đã duyệt — **không tự mở**) | **Allow** | corpus `savepoint` · unit ma trận |
| `Reindex` · `Analyze` | `REINDEX`/`ANALYZE` | **Allow** | corpus `reindex-analyze` · unit ma trận |
| `Function` trừ `load_extension`/`sqlite_load_extension` (case-insensitive) | `count`/`sum`/`randomblob`/… | **Allow** | unit ma trận · corpus `gioi-han-*` (`randomblob`) |
| `Pragma { table_info, None }` | `table_info(…)` | **Allow** | corpus `pragma-doc` · unit ma trận |
| `Pragma { user_version, None }` | **đọc** `user_version` | **Allow** | corpus `pragma-doc` · unit ma trận |
| `Pragma { user_version, Some(_) }` | **ghi** `user_version` (mọi giá trị, kể cả `""`) | **Deny** | corpus `pragma-ghi-bi-chan` · unit ma trận · unit `authorizer_chan_…` |
| `Pragma` khác (`temp_store`, `journal_mode`, `synchronous`, `cache_size`, …) hoặc `None` không thuộc allow | mọi PRAGMA khác — **kể cả** `PRAGMA temp_store` đọc | **Deny** | corpus `pragma-ghi-bi-chan` · unit ma trận · unit `temp_store_bi_ep_memory_va_cache_2mib` |
| `Attach` · `Detach` | `ATTACH`/`DETACH` | **Deny** | corpus `attach-bi-chan` · `detach-bi-chan` · unit `authorizer_chan_…` |
| `CreateVtable` · `DropVtable` | `CREATE [DROP] VIRTUAL TABLE` | **Deny** | corpus `vtable-bi-chan` (fts5, chặn trước "no such module") · unit ma trận · unit `authorizer_chan_…` |
| `Function("load_extension")`/`("sqlite_load_extension")` | `load_extension(...)` | **Deny** | unit `chinh_sach_ham_load_extension_bi_cam` (ghim arm trực tiếp) · corpus `load-extension-bi-chan` |
| `Unknown` / variant ngoài danh sách | — | **Deny** | unit ma trận |
| *(không có action code riêng)* | `VACUUM` trơn + `VACUUM INTO` — tự `ATTACH` nội tại | **Deny** (qua `Attach`) | corpus `vacuum-bi-chan` · `vacuum-into-bi-chan` · integration `attach_bi_chan_va_khong_tao_tep` (xóa file, assert `!exists`) |

- **Bằng chứng mutation** (§14.12): authorizer-off ⇒ đỏ đúng các ca deny (6 corpus ca + 2 unit + 1
  integration). `LOAD_EXTENSION`: SQL function bị SQLite tắt ở tầng C khi connection không
  `sqlite3_enable_load_extension` ⇒ câu lệnh lỗi mã SQLite **1** bất kể authorizer — corpus
  `load-extension-bi-chan` ghi rõ; **không dựng đỏ giả** — arm `Function(load_extension)` được
  ghim bằng unit `chinh_sach_ham_load_extension_bi_cam` (đỏ khi arm bị gỡ).
- **PRAGMA assignment trả về hàng** bị `execute` từ chối bởi blocker 2 (`column_count > 0`) — độc
  lập với authorizer; dùng `sqlite.query` cho PRAGMA (khớp hợp đồng).

### 14.9 Diagnostic SQL fragment (Blocker 8)

- **`loi_sqlite`/`loi_prepare`** (mọi đường execute/query/prepare/step): **không** đưa raw SQL,
  bound values hay đường dẫn ra thông điệp. Chỉ trả: (i) **mã SQLite (integer `extended_code`)**
  + tên **lớp** lỗi (`SqlError` cho `SQLITE_ERROR`/`ErrorCode::Unknown`; `{:?}` của `ErrorCode` cho
  các mã khác), (ii) **offset** (byte) khi có từ `Error::SqlInputError` (đường `prepare`, SQLite ≥
  3.38/`modern_sqlite` — parser-thread lỗi). Không có raw fragment/offset giả.
- Multi-statement: thông điệp **nội bộ** "chỉ được **một** câu SQL…" (không lộ SQL, xuất từ
  rusqlite phân loại) — `Result.Err`.
- **`--sqlite-debug`** (CLI, opt-in): `vis run [--vm] [--no-cache] --sqlite-debug <tệp.vis>` — in
  thông điệp thô (`{e}`) chỉ khi user chủ động bật; help text + `--sqlite-debug` ghi trong
  `vis run` usage; mặc định **không** đổi.
- **Tests**: unit `sqlite_debug_in_thong_diep_tho_ro_fragment_chi_khi_bat` (câu SQL chứa
  `'O'Clock; SELECT secret'` ⇒ mặc định `offset 25` + "mã SQLite 1 — SqlError", **không** chứa
  `O'Clock`/`secret`; bật `--sqlite-debug` ⇒ thấy nguyên văn), unit `loi_offset_duoc_giu_tu_loi_prepare`
  (`SELEC 1` ⇒ "offset" có, `SELEC` không lộ), integration
  `thong_diep_mac_dinh_khong_lo_fragment` (hai đường byte-identical + `--sqlite-debug` thấy raw).

### 14.10 Bảng giới hạn đầy đủ (hiện hành) — UTF-8 bytes + overflow-checked

| Giới hạn | Giá trị | Đơn vị/đếm | Vượt ⇒ | Action trong thông điệp |
|---|---|---|---|---|
| hàng mỗi query | **10_000** | đếm | `Err` | "cắt nhỏ kết quả (LIMIT), không trả dữ liệu cắt" |
| một row | **1 MiB** (`1_048_576`) | TEXT/BLOB bytes; meta không tính | `Err` | "chọn ít cột hơn hoặc cắt nhỏ dữ liệu" |
| tổng kết quả | **16 MiB** (`16_777_216`) | TEXT/BLOB bytes cộng dồn `checked_add`; meta không tính | `Err` | "cắt nhỏ kết quả (LIMIT)" |
| SQL text | **64 KiB** (`65_536`) | **UTF-8 bytes** | `Err` | "cắt nhỏ câu SQL" |
| params mỗi câu | **1_000** | đếm | `Err` | "chia nhỏ lời gọi" |
| mỗi param TEXT/BLOB | **1 MiB** | TEXT = UTF-8 bytes / BLOB = bytes | `Err` | "cắt nhỏ dữ liệu hoặc lưu qua đường khác" |
| tổng payload params/lời gọi | **16 MiB** | TEXT+BLOB cộng dồn `checked_add` | `Err` | "chia nhỏ lời gọi" |
| connections mỗi phiên | **16** | đếm | `Err` | "đóng bớt (close) trước" |
| busy_timeout | default **5_000**, khoảng `0..=60_000` | ms | `Err` | nêu khoảng hợp lệ |
| wall-clock mỗi câu | **5_000** | ms (progress_handler) | `Err` (OperationInterrupted) | — (thông điệp tên lớp lỗi) |
| statements mỗi lời gọi | **1** | đếm | `Err` | "chỉ được một câu SQL…" |
| page cache | **2 MiB** | mỗi connection (`cache_size = -2048`) | — (bound, không lỗi) | — |
| kết quả NaN/±Inf | — | — | NaN ⇒ `Err` | "dùng `SqlValue.Null()`…" |

Bảng này thay cho bảng §3 (Gate 1) — mọi "trap VS0501" ở đó đã chuyển thành `Err` (§14.2).
Metadata (tên cột/kiểu/variant enum) **không** tính vào các cap byte (bỏ `columns.len()` khỏi tổng).

### 14.11 Timeout (Blocker 9): monotonic, từ `prepare` tới hết đọc hàng

- **`db_open` KHÔNG áp 5 s** — open thường thuần file/memory; chỉ `execute`/`query` áp budget.
- **Đo**: `let vao = Instant::now();` được ghi **trước** `prepare` (trước `thuc_thi`/`truy_van`
  — và trước khi cài progress_handler), chạy tới hết vòng lặp đọc hàng; `progress_handler` cài
  **trước** `prepare` (VM mỗi 1_000 vòng) và kiểm `vao.elapsed() > Duration::from_millis(query_ms)`
  — trả nonzero để SQLite **ngắt cứng**. `progress_handler` **reset** trên cả đường thành công lẫn
  lỗi (không có early-return giữa cài và reset — cấu trúc giữ từ Gate 1, đã verify lại).
- **Busy-wait/queue-wait KHÔNG được cộng thêm 5 s độc lập ngoài deadline** — không có code path
  như vậy (timeout của busy_timeout do SQLite tự quản, dựa trên cùng `vao` deadline qua handler).
- **Budget cấu hình nội bộ tests/debug**: `Db::moi_voi_query_ms(giai_thich, query_ms)` (`#[doc(hidden)]`,
  không phải API `std.sqlite`) — chỉ cho unit/debug; corpus/timeout `.vis` dùng default.
- **Không** bắt suite chờ 5 s mỗi ca: unit `thoi_gian_chay_nho_bi_ngat_nhanh_khong_phai_doi_5s`
  (budget **100 ms**, ngắt `< 5 s`, handler reset — câu sau chạy bình thường). Integration
  `gioi_han_thoi_gian_5s_bi_ngat` giữ default (~5 s, chạy một lần).
- Thông điệp khi ngắt: `… OperationInterrupted (mã SQLite 9 — OperationInterrupted)` — grep
  `"OperationInterrupted"` (integration đã cập nhật).

### 14.12 `.vish` (Blocker 11): **KHÔNG hỗ trợ — không mở đường tắt, chưa corpus**

Thực tế đo (scratch `/tmp/opencode/vd241/t*.vish`, không commit):

- `.vish` (chế độ lệnh) parse `sqlite.open(":memory:")` như **pipeline / lệnh ngoài** (`vis run`
  cố exec chương trình `sqlite.open` ⇒ `VS0501: không chạy được chương trình ngoài `sqlite.open`:
  No such file or directory`); **không** chạm `std.sqlite` builtin.
- `use std.sqlite` được chấp nhận như từ trần **inert** (không import gì; `.vish` không có
  module-scope declarations như script); `let ket = sqlite.open(...)` bind một **pipeline**, việc
  dùng sau đó báo `VS0501: tên `ket` chưa hỗ trợ ở VS0`.
- `match` ở mức tệp ⇒ `VS0201` (`.vish` không có block cấp tệp). `gs-select.vish` (corpus upstream)
  cho thấy `.vish` chỉ có pipeline + `get/select`/lệnh ngoài.
- **Kết luận theo lệnh**: *không hỗ trợ; không mở đường tắt; chưa corpus* `test/sem/vish/*sqlite*.vish`.
  Nếu A0 muốn `.vish` gọi `std.sqlite` ⇒ thiết kế riêng (chế độ lệnh + module fn), ngoài phạm vi.

### 14.13 Cổng (Blocker 12) — kết quả thật (2026-11-19, Linux, cargo 1.99.1)

| Cổng | Kết quả |
|---|---|
| `./vis-dev check` | **PASS** (fmt · clippy `-D warnings` · R1..R6 — `db.rs` tách `db/mod.rs` (873) + `db/tests.rs` (871) ≤ 1500 · self-test · vis explain · báo cáo · doc-examples) |
| `cargo test --workspace --no-fail-fast` | **148 binaries; duy nhất `api_std_khop_code` FAIL** (freeze `THÊM (9): sqlite.*` — đúng thiết kế §8; `api_codes_khop_code` + `api_vbc1_khop_code` XANH) |
| `./vis-dev gate --fast` | check PASS; **L0 dừng đúng ở `api_std_khop_code`** (freeze chưa regen) |
| `vis test test/sem` / `--vm` | **525/525 ×2** (byte-identical; **475 `.vis` + 50 `.vish`**; nhóm `sqlite/` = **38 ca**, 0 `.err`; count pins: `sem`/`vis_test`/`diag_cmp` (437/53/35)/`roundtrip`/`vs_interp` (bỏ qua 38)/`fuzz_mutate` (437 module)/`CHEAT-SHEET`/`FACTS`) |
| corpus sqlite | hai engine byte-identical mọi ca; `.out` goldens tất định (không chứa token ngẫu nhiên — xem §14.5) |
| `crates/vis-cli/tests/sqlite.rs` | **15/15** (gồm `workload_50k_rows_sum_avg_ca_hai_duong`, `thong_diep_mac_dinh_khong_lo_fragment`, `sqlvalue_null_ctor_chay`, …); wall-clock ~5 s một ca |
| `db::` unit tests | **32/32** |
| Fresh-clone gate | clone cục bộ từ HEAD + replay working-tree diff (sạch) ⇒ `check` PASS · workspace 148 + freeze-red duy nhất · `sem` **525/525 ×2** — **clone đã xoá** |
| Mutation authorizer-off | **corpus đỏ đúng**: `sqlite/attach-bi-chan` · `detach-bi-chan` · `pragma-ghi-bi-chan` · `vacuum-bi-chan` · `vacuum-into-bi-chan` · `vtable-bi-chan` (6 ca) · **integration đỏ**: `attach_bi_chan_va_khong_tao_tep` · **unit đỏ**: `authorizer_chan_attach_pragma_ghi_load_extension`, `temp_store_bi_ep_memory_va_cache_2mib` · `LOAD_EXTENSION` corpus **giữ xanh** (SQLite tắt function ở tầng C — xem §14.8; arm được ghim bởi unit `chinh_sach_ham_load_extension_bi_cam`) · **hoàn nguyên ⇒ tất cả xanh lại** |
| Mutation cleanup-off | `ro_canh_bao_chi_diagnostics` **đỏ** · hoàn nguyên ⇒ xanh |
| Test `authorizer-set-user_version` (unit `authorizer_chan_…`, có `PRAGMA user_version=1`) | **đỏ** dưới mutation ✓ |
| psvn (Windows) build | **chưa chạy** — agent master trigger ngay sau đợt này; kết quả ghi sau (§14.15) |

### 14.14 Budget-outcome của các tests (nguyên văn kết quả, không chỉnh)

| Test | Engine(s) | Outcome | Thời gian |
|---|---|---|---|
| `gioi_han_thoi_gian_5s_bi_ngat` (integration) | interp | PASS — bị ngắt ở 5 s (`OperationInterrupted`) | ~5 s (chạy một lần) |
| `thoi_gian_chay_nho_bi_ngat_nhanh_khong_phai_doi_5s` (unit, budget 100 ms) | — | PASS — ngắt `< 5 s`, handler reset | ~0.0 s |
| `nhieu_lan_chay_trong_cung_mot_process` (20 lần) | interp+VM xen kẽ | PASS — rò báo đúng 1 mỗi lần | ~1 s |
| `hai_ket_noi_tranh_khoa_busy_timeout` (unit + integration) | — | PASS (~0.3 s tranh khóa, không treo) | ~0.3 s |
| `workload_50k_rows_sum_avg_ca_hai_duong` | interp+`--vm --no-cache` | PASS — `inserted 50000`, `sum=1250025000`, `avg=25000.5` | ~0.1 s |
| `cargo test --workspace --no-fail-fast` | — | 148 binaries xanh; **1 red** = `api_std_khop_code` (thiết kế) | — |
| `./vis-dev check` | — | PASS | — |
| `./vis-dev gate --fast` | — | check PASS; L0 dừng ở freeze-red | — |

Không ca nào treo/quá deadline; không suite nào chờ 5 s mỗi ca (trừ đúng ca integration wall-clock).

### 14.15 Phạm vi tuyên bố (không vượt lệnh)

- **Chỉ Linux**; **Windows/macOS KHÔNG re-verify** bản này (`cargo check --target` matrix không chạy;
  `pha1-rc1` không chạm; không claim). **psvn build sớm — chạy ngay** sau đợt này (agent master
  trigger); kết quả sẽ được ghi ở vòng kế (không tuyên bố trước).
- **Không** unfreeze `api_dong_bang.rs` (5 arm render giữ nguyên từ Gate 1); **không** đổi
  `agent/API-STD.txt`; **không** đụng `SPEC-lex`/`SPEC.md`/`docs/ViScript-thiet-ke.md`/`pha1-rc1`;
  **không** merge/push; không tuyên bố tính năng ngoài đợt này.
- **Không** tuyên bố "mọi OOM thành `Err`" (§14.7); **không** claim bảo mật liên phiên của token
  (§14.5); **không** claim `.vish` hỗ trợ `std.sqlite` (§14.12).
- **Freeze/API**: chữ ký §14.1 **chưa** đông băng — cổng tích hợp (A0) quyết chữ ký cuối + regen freeze.

### 14.16 Danh sách tệp thay đổi (cho reviewer)

**Nguồn**: `crates/vis-interp/src/stdlib.rs` (bỏ `DAU_LOI_GOI`, `hop_nhat_fs` không prefix),
`crates/vis-interp/src/builtin_db.rs` (doc handle token), `crates/vis-types/src/stdlib.rs` (doc comment),
`crates/vis-cli/src/db.rs` → **tách** `db/mod.rs` (873 dòng) + `db/tests.rs` (871 dòng, tách theo R1),
`crates/vis-cli/src/main.rs` (`--sqlite-debug` + usage).
**Tests**: `crates/vis-cli/tests/{sem,vis_test,diag_cmp,roundtrip,fuzz_mutate,vs_interp,sqlite}.rs`
(count pins 514→525, 96→88 `.err`; integration +2 ca: `--sqlite-debug`, workload-50k).
**Corpus** (`test/sem/sqlite/`): 8 `.err`→`.out`; **+11 ca mới** (`execute-tra-hang`, `bind-inf`,
`ddl-index-alter`, `reindex-analyze`, `vacuum-bi-chan`, `vacuum-into-bi-chan`, `detach-bi-chan`,
`pragma-ghi-bi-chan`, `load-extension-bi-chan`, `vtable-bi-chan`, `savepoint`) ⇒ **38 ca**.
**Docs/counts**: `CHEAT-SHEET-ViScript.md`, `agent/VISCRIPT-ERGONOMICS-FACTS.md` (+ dòng cập nhật),
`agent/BAO-CAO-VD-SQLITE-MVP-2026-11-19.md` (tệp này).
**Untracked mới**: `crates/vis-cli/src/db/{mod.rs,tests.rs}` (tách R1); **xóa** `crates/vis-cli/src/db.rs`.


## 15. Narrow freeze VD-129 — A0 phê duyệt (2026-11-19)

> **A0 phê duyệt 2026-11-19**: sinh lại freeze **chỉ trong phạm vi狭 hẹp** `std.sqlite.*`; không regen rộng.
> HEAD merge sẵn sàng sau review chéo cuối.

- **`agent/API-STD.txt`**: **+9 mục đúng tên** — `sqlite.begin`, `sqlite.close`, `sqlite.commit`, `sqlite.execute`,
  `sqlite.in_transaction`, `sqlite.open`, `sqlite.open_with`, `sqlite.query`, `sqlite.rollback`. **Không** xoá, **không** đổi nghĩa,
  **không** thêm mục nào khác (API-CODES.txt / API-VBC1.txt **giữ nguyên**).
- `api_dong_bang.rs`: chỉ +5 arm render (`SqlValue`, `RowSet`, `ResultIntErr`, `ResultBoolErr`, `ResultRowSetErr`);
  logic so khớp **không đổi**; temp wrotefile **chỉ** dưới `VIS_FREEZE_REGEN=1`; gate luôn đọc tệp, khớp.
- **Gate**: `vis-dev gate --fast` **PASS toàn bộ** (L0 workspace **149/149** xanh — `api_std_khop_code` giờ xanh do freeze khớp code).
- **Review chéo cuối (tác giả khác)**: verdict **SHIP** — 10 commit đúng phạm vi lệnh, freeze hẹp đúng 9 dòng, SPEC/ABI/VBC1/pha1-rc1/Windows Tier 2 nguyên.
- **Trạng thái**: nhánh `rev/vd-sqlite-mvp-2026-11-19` @ `dd781f5` — đã push; sẵn sàng merge vào `main` (ff-only) sau khi smoke join trên main xanh.
- **E-call** (theo thứ tự merge): (1) merge ff-only `main` ← nhánh Gate 2; (2) push origin/main; (3) gate + workspace + smoke + sqlite/CRLF/Windows càng sớm càng tốt; (4) DEM/DECISIONS append ghi nhận trạng thái đóng Gate 2.
- **Chưa** tuyên bố hỗ trợ Windows cho `std.sqlite` (Windows Tier 2 giữ); **chỉ** Linux là được kiểm chứng đến \`vis-dev gate --fast\` PASS.
- **psvn** verify build trên `vis-sqlite-check-git` tại HEAD mới: build + sem + smoke đã qua (xem §6 của báo cáo reviewer cuối + ghi nhận DEM/Windows).
- psvn build trên worktree (raw): **14/15** trước fix path (Windows VS0102) — fix trong `fd243cf` (dùng `/` trong path), sau fix **15/15** (chi tiết ở \`psvn:vis-sqlite-check\` & DEM).

### Đính chính §14.13 / §14.15 (trạng thái hiện tại)

- `./vis-dev gate --fast` **PASS toàn bộ** — L0 workspace xanh (149/149), không còn dừng ở freeze-red.
- `cargo test --workspace --no-fail-fast` **149 binaries xanh** (trước fix: 148 + `api_std_khop_code` đỏ).
- \`api_std_khop_code\` **xanh** — freeze hẹp khớp code.
- §\`psvn (Windows) build` — **đã chạy**: bundled link OK, **15/15** sqlite, CRLF 3/3, sem 521/525 (4 ca POSIX `vish/ngoai-*`),
  `smoke join` 22/22 Windows (sau merge) — ghi ở DEM.

### F-C / F-D / F-F / F-G (deferring chờ A0, ghi ở đây để không mất)

- **F-C**: Blocker 13 không ánh xạ rõ trong §14.2–14.13 — Blocker 13 = **không thêm chữ ký** (API shape hẹp + không thêm `Value` variant; ABI/VBC1 giữ).
- **F-D**: không có corpus golden cho ca "thông điệp không lộ fragment" — coverage bằng unit + integration là **đủ** cho MVP.
- **F-F**: limits ⇒ `.out` thay vì `.err` (goldens nắm goldens cho `limits`-thông điệp hiện hành) — chấp nhận.
- **F-G**: `#[allow(dead_code)]` trên `Interp::set_db` — thừa (hàm được gọi); gỡ ở vòng docs sau.
