# Khảo sát — Native integration và SQLite MVP (A0 2026-11-19)

> **Trình A0 · từ ps (Agent 1) · 2026-11-19 · trạng thái: KHẢO SÁT — chưa duyệt triển khai.**
> Không dòng mã nào bị thay đổi bởi tài liệu này. Không mở VD triển khai khi chưa có lệnh A0.
> Mục tiêu (nguyên văn A0): khả năng "bọc thư viện rồi dùng luôn" như Python `sqlite3`; **chưa**
> duyệt raw FFI/DLL tổng quát; SQLite là ứng viên tốt để mở đường; chưa gộp typed-row derive/ORM.

## 0. Phạm vi khảo sát A0 yêu cầu

1. Kiểm tra repo hiện có gì: **host function registry, native handle, FFI, dynamic loader; phân biệt với subprocess.**
2. Đề xuất cách dùng **chung** binding cho interp và VM, tránh hai implementation độc lập.
3. So sánh **SQLite bundled** với thư viện hệ thống (phân phối, cập nhật, tái lập build).
4. Thiết kế MVP: open/close, execute/query, bind tham số, SQL value, transaction và lỗi.
5. Vòng đời tài nguyên và **capability** — gồm **tệp phụ** (journal/WAL) và **ATTACH**.
6. Tạm hoãn: ORM, typed-row derive, callback SQL, loadable extensions.
7. Trình dependency với các task hiện tại; **chưa** tự mở sửa compiler hoặc ABI.

## 1. Hiện trạng thật trong repo (bằng chứng `file:symbol`)

### 1.1 Không có FFI/“dynamic loader” — `extern` không phải C FFI

- `grep -rn 'rusqlite|libsqlite3|dlopen|LoadLibrary|unsafe extern'` trong `crates/` → **không có kết quả**.
- Các test mang tên `extern` (`crates/vis-cli/tests/extern.rs`) kiểm tra **gọi chương trình ngoài**:
  `/bin/echo`, `true`, `false`, `grep`, `cat`, `printf` — tức là **subprocess**, không phải gọi hàm C.
- Các test `pipeline`/`ngoai_vm` tương tự (`host_conformance_suite` case 23–24, `vish_main`) đều là
  process + stdin/stdout → **không phải FFI**, không đại diện cho khả năng nạp DLL.
- Không có crate/dependency nào liên quan native binding (`cargo tree` không có sqlite).

### 1.2 Host function registry hiện có (và đúng chỗ mở rộng)

- **Một** dispatch cho cả hai engine: `crates/vis-interp/src/builtin.rs` `pub fn call(...)`.
  VM gọi `vis_interp::builtin::call` cho opcode `CallBuiltin` (`crates/vis-vm/src/vm.rs`: `Instr::CallBuiltin` → `builtin::call(vm, &ten, args, span)`).
  ⇒ **thân hàm native chỉ viết một lần**, interp và VM dùng chung.
- **Host trait** pattern: `FsHost` (`read/write/exists/list…`), `NetHost` (`connect/listen/read/write/accept/close` — handle là **chuỗi mờ**, thân nhị phân); host do `vis-cli` cài thực sự (OS/Rust), interp/VM **không** biết native (`Host::fs()/net()/ngoai()` → `Option<&mut dyn …>`, thiếu ⇒ `VS0604`).
- **Capability**: `cap[Dir/File/Net/Proc]` + `--strict-caps` + `--grant` (`crates/vis-cli/src/quyen*.rs`);
  fs/net đã qua lớp kiểm (mở-rồi-kiểm/`FsGuard` chống TOCTOU).
- **Handle mờ**: NetHost dùng `String` handle (`"tcp:0"`…) map vào bảng runtime; chương trình không thấy địa chỉ/pointer ⇒ **đúng mẫu cho connection handle**.
- **std registry**: `crates/vis-types/src/stdlib.rs` `FUNCS: &[StdFn]` + `sig_of` — thêm dòng `std.sqlite.*` = không thêm opcode, không đổi VBC1.
- **ABI VBC1 đóng băng** — không thêm `Value` variant cần thiết (ràng buộc §4.5).
- `vis-host-vifast` (Landlock probe) **không nằm trong repo ViScript** (AGENTS §4) — chỉ là host khác của ViScript (external).

### 1.3 Subprocess vs native in-process

| | `ngoai` (chương trình ngoài) | Native module đề xuất |
|---|---|---|
| Cơ chế | `exec` process, stdin/stdout | gọi hàm Rust/C trong **cùng process** |
| Chi phí | process spawn (μs→ms) + I/O | lệnh gọi hàm (ns→μs) |
| Kiểu | text (args) → text | kiểu dữ liệu native trực tiếp |
| Lỗi | exit code + stderr | `Result[_, Error]` giàu thông tin |
| Sandbox | con bị kê gian by `cap[Proc]` + giới hạn | **không tạo được sandbox cho code native** — xem §6 |
| Dùng cho | CLI/binary bên ngoài | thư viện (SQLite, sau này nén/ảnh…) |

## 2. Kiến trúc đề xuất (minh hoạ — chưa duyệt)

```
Chương trình ViScript (use std.sqlite)
        ↓
Module SQLite có API rõ kiểu (một implementation — Rust)
        ↓
Host trait mới `DbHost` tái sử dụng dispatch của `builtin::call`
        ↓
`vis-cli` cài host native (rusqlite bundled hoặc system)
```

- Không viết lại database engine; không có cách người dùng thấy con trỏ.
- Dùng **đúng mẫu `FsHost/NetHost`**: `vis-interp` định nghĩa `trait DbHost`, `vis-cli` cài thật (CLI chỉ là nơi OS động), interp/VM nhận qua `Host::db()`.
- Bản thân `crates/vis-interp` **không linking** native; chỉ `vis-cli` (và sau này `vis-host-linux`) cài `DbHost`.
- Binding gọi `rusqlite` (MIT, duy trì tích cực, bao bọc C library) — hoặc gọi `sqlite3.h` trực tiếp nếu A0 yêu cầu không dependency mới.

## 3. SQLite bundled vs system library

| Tiêu chí | **Bundled (`rusqlite` + `libsqlite3-sys` `bundled`)** | System (`libsqlite3-dev`) |
|---|---|---|
| Reproducible build | ✅ Tự đóng kèm `sqlite3.c`, không cần pkg-config | ❌ cần dev package tại máy build |
| Máy không có dev kit | ✅ vẫn build được | ❌ build fail |
| Version drift | ✅ ghim version trong lock | ❌ phụ thuộc distro/macOS/Windows |
| Cập nhật bảo mật | gắn với release của crate | do OS cập nhật (chậm/khác nhau) |
| Kích thước binary | +~1–2 MB | +~0 |
| Licensure | sqlite3.c **public domain**; `libsqlite3-sys` MIT; `rusqlite` MIT — tương thích với `MIT OR Apache-2.0` (ghi NOTICE/sqlite-license.txt) | như nhau |
| Windows/macOS | ✅ bundled build ổn (cc/MSVC) | ❌ system API phức tạp (vcpkg…) |
| Khả năng thay thế | fixed `SQLITE_VERSION` — ghi rõ trong build log | — |

**Đề xuất sơ bộ**: bundled — tái lập build quan trọng hơn ~1–2 MB; nhưng A0 chốt (§9).

## 4. MVP — phạm vi tối thiểu đề xuất

### 4.1 Hợp đồng chức năng (illustrative — chưa duyệt)

```vis
use std.sqlite

let db = sqlite.open("app.db")?             # cap[Dir] trên thư mục chứa file?
db.execute(                                # DDL/DML
    "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT)"
)?
db.execute(
    "INSERT INTO users (name) VALUES (?)",
    ["An"]                                  # bind tham số — không nối chuỗi SQL
)?
let rows = db.query(                        # SELECT
    "SELECT id, name FROM users WHERE name = ?",
    ["An"]
)?                                          # rows: xem §4.4
db.close()?
```

- Chỉ **một** câu SQL mỗi `execute`/`query` (multi-statement từ chối).
- Params **chỉ** qua bind; không có API nối chuỗi values vào text SQL.
- `Result[_, Error]` mọi chỗ (VD-222 dòng: lỗi chuẩn là `Result`, không panic).

### 4.2 SQL value — một kiểu giá trị rõ ràng (không lấy JSON làm trung gian)

Đề xuất enum dựng sẵn `SqlValue` (thêm vào prelude, **không** vào `Value` ABI? xem §4.5):

| SQL | ViScript |
|---|---|
| INTEGER | `int` (i64) |
| REAL | `float` (f64) |
| TEXT | `str` |
| BLOB | `bytes` |
| NULL | `Unit` hoặc `None` (đề xuất `Option[X]` ⇒ thiếu ⇒ `None`; **cần chốt**: `Option[sv]` cho mọi cell hay cell = `SqlValue` adt riêng) |
| BOOLEAN (SQLite has no `BOOL`) | `bool` ↔ `0/1` |

Không dùng JSON cho tất cả — `Value::Enum("Json", …)` làm trung gian bị loại.

### 4.3 Row representation — chọn MVP

| Phương án | shape | Ưu | Nhược |
|---|---|---|---|
| **(a) `record { columns: list[str], rows: list[list[SqlValue]] }`** | vị trí | không cần kiểu khai báo trước; rõ cột; ergonomics trung bình | người dùng phải index `rows[i][j]`; tên cột đi kèm |
| **(b) `list[record]` với tên cột động** | struct động | tiện hơn | **không làm được** với `Record` tĩnh (schema dynamic) |
| **(c)** opaque handle + `db.next(rs)? -> record …` | streaming | nhẹ | API kéo dài, phức tạp lifetime (VM pattern) |
| Đưa `SqlValue` adt **dựng sẵn** vào prelude | enum gồm `Null/Int/Real/Text/Blob` | typed rõ ràng, `match` làm được | code mới, không ảnh hưởng ABI VBC1 |

**Đề xuất sơ bộ**: (a) + `SqlValue` adt prelude; typed-row derive (b) hoãn tới serde/typed-decode `#[derive Json]` có hợp nhất schema tĩnh — xem RFC `agent/RFC-TYPED-SERIALIZATION-2026-11-19.md`.

### 4.4 Transaction & lỗi

- `db.execute("BEGIN")?` tự nhiên hoạt động (SQL chuẩn của SQLite) — **hoặc** chỉ dùng `Result` + đường `db.transaction(fn f())?` wrapper (hoãn, muốn API rõ ràng → hỏi A0).
- Lỗi giữa transaction: không tự rollback ẩn — contract: lỗi trả lại `Result` nguyên, connection **vẫn mở**, người dùng gọi `ROLLBACK` hoặc `close` (close không auto-commit im lặng; ghi rõ trong doc).
- `SQLITE_BUSY` / lock / timeout: `sqlite.open(path, busy_timeout_ms?)` — mặc định 5 s; vượt ⇒ `Err` (không treo vô hạn).
- `db.close()?` tường minh; **runtime safety net**: khi process exit (hoặc `close` không gọi), connection đóng theo `Host` drop (mẫu `NetHost` exit = OS đóng; ghi rõ không hứa 100 % durability semantics — "an toàn ở mức process kết thúc"); không phụ thuộc hoàn toàn vào người dùng gọi close.
- Person: **không** tự quyết `PRAGMA journal_mode=WAL` — mặc định SQLite (rollback journal) trừ khi opt-in khẩu khóa (cần quyết §9).

### 4.5 Khả năng không đổi ABI hiện có (quan trọng)

- `Value` enum **không thêm variant** trong MVP: `SqlValue` đạt được bằng cách dùng các `Value` sẵn có (None/Int/Float/Str/Bytes/Record) thay vì type mới — rows đi qua `list[_list[Value]]` hoặc record `{ columns, rows }` hiện có.
- Opcode **không thêm**: gọi `std.sqlite.*` qua `CallBuiltin` hiện có.
- `api_dong_bang.rs` thêm dòng `std.sqlite.*` ⇒ freeze API sẽ đổi (nếu A0 duyệt) — **đã là quyết định riêng**, không mặc định xuất hiện.
- ABI VBC1 — nguyên vẹn.

## 5. Capability & an toàn (bao gồm tệp phụ, ATTACH, PRAGMA, extension)

| Điểm | Rủi ro | Đề xuất MVP |
|---|---|---|
| Đường dẫn `open(path)` | đọc/ghi tuỳ ý hệ thống | bắt theo `cap[Dir]` trên thư mục chứa `path` (`--strict-caps`); mặc định OS lưu |
| Journal WAL/-shm/-wal | tệp phụ sinh cạnh db | cùng `cap[Dir]` trên thư mục chứa db — tài liệu hóa "open bao gồm các tệp phụ của SQLite" |
| `ATTACH DATABASE` trong SQL text | mở db ngoài quyền | **từ chối** trong MVP nếu `--strict-caps` (keyword pre-scan + chặn keyword ATTACH); hoặc chỉ allow khi `cap[Dir]` cho path đính kèm — A0 chốt |
| `PRAGMA` trong SQL text | cấu hình/tắt journal/ghi đè | **cấm** ngoài `user_version` + `table_info` (readonly) trong MVP nếu `--strict-caps` (pre-scan); hoặc chặn toàn bộ PRAGMA — A0 chốt |
| `LOAD_EXTENSION` | chạy mã native bất kỳ | **chặn cứng** trong MVP; module không bao giờ gọi `sqlite3_load_extension`; extension loading tắt mặc định trên `sqlite3_open_v2` |
| `sqlite3_open` vs `open_v2` | mở sai quyền | dùng `open_v2` với flags tối thiểu (no-create theo mặc định? hoặc `CREATE` cho phép create=true mặc định — chốt §9) |
| SQL injection | nối tham số | bind params only; không có hàm nối; từ chối nhiều câu một lúc |
| `busy_timeout` | treo | mặc định 5 s; tham số `open(path, busy_timeout_ms?)` |
| `fs.tmp` / in-memory `:memory:` | không ghi đĩa | `:memory:` hợp lệ; WASM/không đĩa không cam kết |
| Unicode/charset | db lưu bytes; TEXT đọc UTF-8 | validate UTF-8 từ SQLite; lỗi ⇒ `Err` |
| SFU/OOB buffers | C API ghi ngoài | `rusqlite` bao null-terminated → an toàn; không raw `sqlite3_prepare` trực tiếp |

**Điểm mấu chốt**: raw FFI trong cùng process **không tạo được sandbox** cho native code — chương trình gọi `dll.call` có thể làm bất cứ gì OS cho phép (đọc key file, xóa đĩa, mở socket không qua `cap[Net]`). ⇒ Native module **chỉ cho thư viện được duyệt**, không cho script tự nạp DLL (A0 đã nói).

## 6. Phạm vi còn cố tình hoãn (theo A0) + vì sao

| Hoãn | Lý do |
|---|---|
| **Raw C FFI/DLL từ script** | ABI/memory/rights chưa thiết kế; an toàn quá phức tạp cho đợt này; A0 đã chốt "chưa duyệt raw FFI thành mặc định" |
| **Typed-row derive** (VD `#[derive Row]`) | Phụ thuộc typed-serde RFC; schema dynamic không ánh xạ `Record` tĩnh |
| **ORM / query builder** | Không phải thay thế SQL |
| **`csv.join`/typed CSV** | RFC riêng nếu A0 muốn; không đi cùng |
| **Loadable extensions** | An toàn; **OFF** trong MVP; có thể mở lại qua cấp quyền riêng nếu A0 chốt |
| **Callback SQL / user-defined functions trong script** | phức tạp; hoãn |
| **WAL/journal mode opt-in** | A0 chốt hoặc giữ mặc định SQLite |
| **ATTACH** | MVP: chặn (strict caps) hoặc cấm hoàn toàn |
| **Concurrency** | single connection, single thread; không thread-safety promise |
| **typed decode JSON↔Row** | RFC typed serialization, riêng |

## 7. Dependencies & không mở trùng task

| Task/VD | Trạng thái | Tương tác |
|---|---|---|
| **Option lồng fix** (`3b2c67e`) | Done | không đụng |
| **RFC typed serialization** (`agent/RFC-TYPED-SERIALIZATION-2026-11-19.md`) | Chờ A0 | Row → typed record dùng sau typed-serde; **hoãn** |
| **Phase B-3** (impl xuyên module + bounds) | Chưa bắt đầu; chờ A0 | không cần cho MVP (concrete types); cần nếu typed-row derive sau |
| **Windows Tier 2** (`349af67`) | Đóng phạm vi | SQLite bundled build sạch Windows; **nếu** triển khai MVP sau, cần re-verify psvn |
| **RFC SQLite survey** | **Đây** | bước trình chuẩn bị |
| **`#147`/`#152` (mac)** | Chờ rảnh | không chặn; đo cùng HEAD khi Mac rảnh |
| **`ngoai`/subprocess** | Done (ST-34/89, VD-…) | phân biệt trong docs (§1.3) |
| **`std.fs`/`std.net`/`std.json`** | Done | mẫu host/handle tái dùng; **không** đổi API của chúng |
| **`vis-host-vifast`** (external) | ngoài repo | không ảnh hưởng khảo sát |
| **`api_dong_bang.rs`** | freeze | MVP thêm `std.sqlite.*` ⇒ freeze phải cập nhật (VD riêng nếu A0 duyệt) |
| **`docs/ViScript-thiet-ke.md`** | A0 quyết | cập nhật nếu thêm module — chờ ratify |

## 8. Kế hoạch kiểm chứng (khi A0 duyệt MVP)

- **Fresh-clone** gate PASS + `vis-dev gate --fast` + `cargo test --workspace --no-fail-fast` + `vis test test/sem`/`--vm` (MỚI 483→486 ca; sẽ thêm `sql-*` ca) — không đụng VBC1.
- Corpus: `.vis` với `:memory:` determinism (không network, không filesystem ngoài temp); một số ca `cap[Dir]` refuse/allow (như `cap_*` patterns hiện tại); `SQLITE_BUSY` tái lập khi 2 connection cạnh tranh; transaction rollback; NULL mapping; BLOB round trip; Unicode TEXT.
- **Two engines byte-identical** phải PASS mọi ca (nam Đường native chung — §1.2).
- **Mutation**: tắt `busy_timeout` ⇒ ca lock đỏ; tắt keyword pre-scan ⇒ ca ATTACH đỏ; thiếu bind param ⇒ ca injection đỏ.
- Windows `/psvn` re-verify (if triển khai — bundled build kiểm).

## 9. Câu hỏi A0 cần chốt (trước khi mở VD triển khai)

**A. Distribution**: bundled (`rusqlite` + `bundled` — đề xuất) hay system (`libsqlite3-dev`)? **A0 chốt**.
**B. Row shape**: (a) `record { columns, rows: list[list[SqlValue-adt]] }` + `SqlValue` prelude enum — đề xuất? hay khác (typed record? handle streaming`? **A0 chốt**.
**C. SQL value mapping**: INTEGER→int, REAL→float, TEXT→str, BLOB→bytes, NULL→None (bool↔0/1) — khớp đề xuất hay khác?
**D. Param API**: `[values…]` positional — có đồng ý không? hay `SqlValue`-only? hay không bulk?
**E. Transaction**: `execute("BEGIN")?` SQL thuần (đề xuất MVP) hay wrapper riêng `db.transaction(fn f())?` (hoãn)?
**F. close semantics & runtime net**: close tường minh + warn-on-leak (ghi stderr) khi process exit; không auto-commit bao giờ; khớp đề xuất?
**G. `--strict-caps` mapping**: `cap[Dir]` bao tất cả tệp phụ (journal/WAL/-shm); điểm nhắc trong tests; khớp đề xuất?
**H. ATTACH/PRAGMA/extension**: MVP chặn cứng (pre-scan) trong `--strict-caps`; `LOAD_EXTENSION` chặn mọi chế độ — khớp đề xuất? hay chặn luôn (không strict) vĩnh viễn?
**I. `:memory:` / `CREATE` flag**: `:memory:` hợp lệ; open mặc định **không tạo** file mới (như `O_RDWR` không `O_CREAT`) hay tạo mới đuợc? (Python mặc định tạo mới) — **A0 chốt**.
**J. Busy timeout default**: 5 s khớp? (§4.4)
**K. Module tên**: `std.sqlite` — đồng ý? hay package riêng (ngoài `std`)? (Non-ratified name — A0 chốt; ảnh hưởng API freeze.)
**L. `api_dong_bang`**: thêm `std.sqlite.*` vào freeze hay không? (A0 quyết; ảnh hưởng môn freeze API.)
**M. Windows/macOS tier**: nếu triển khai MVP — re-verify psvn/mps bắt buộc trước tuyên bố, cần máy rảnh?
**N. Thời gian tách đợt**: khảo sát này chỉ mở đường; **chưa** triển khai; A0 chốt khi nào mở VD triển khai?

## 10. Trình tự thực hiện (sau ratify A0)

1. Worktree riêng; **Agent 2**: stdlib `std.sqlite.*` `StdFn` rows; `DbHost` trait; `vis-cli` host cài (bundled SQLite); corpus + mutation; gate.
2. **Review chéo** (tác giả khác) trước merge; fresh-clone gate PASS.
3. Docs: `docs/ViScript-thiet-ke.md` + CHEAT-SHEET; `api_dong_bang` (§11-L); DEM.
4. Windows/macOS re-verify nếu thay đổi cover native.
5. Chỉ sau đó mới xem xét: typed-row derive (sau typed-serde RFC), extension allowlist, ATTACH opt-in, WAL opt-in.

## 11. Kết luận của Agent 1 (không phải quyết định)

- Native binding **khả thi đúng mẫu hiện có** (`builtin::call` chung hai engine; host traits; handle mờ) — chi phí thấp, rủi ro thấp hơn kỳ vọng.
- Seam mở rộng native đầu tiên nên là **SQLite** — tiêu chí đúng (thư viện ổn định, public domain, may đo được, nhu cầu thật).
- **Raw FFI từ script** để sau; **extension loading** mặc định tắt; capability cho tệp phụ SQLite phải là phần của hợp đồng, không phải chi tiết thực thi.
- **Không triển khai** cho tới khi A0 chốt §9.


---

## Addendum — Quyết định A0 2026-11-19 (duyệt mở VD "SQLite MVP", hai cổng)

> A0 duyệt mở VD **SQLite MVP**, chia hai cổng: **Cổng thiết kế + prototype nội bộ** (worktree riêng, **chưa merge API công khai**)
> và **Cổng tích hợp** (chỉ sau reviewer độc lập xác nhận hợp đồng kiểu/tài nguyên/quyền). **Option fix không phải DONE** (coercion còn mở).
> Hoãn vĩnh viễn đợt này: raw FFI, ORM, typed-row derive, callback SQL→script, loadable extensions.

### Chỉnh sửa kiến trúc bắt buộc (không được thay bằng phương án khác nếu không có bằng chứng)

1. **Không dùng keyword pre-scan làm ranh giới quyền** — dùng **SQLite authorizer callback** (`sqlite3_set_authorizer`, cài **trước khi** prepare SQL người dùng) để từ chối **hành động theo cấu trúc** SQL đã phân tích (xử lý VACUUM INTO, ATTACH, load_extension, PRAGMA-ngoài §H). Pre-scan **chỉ** dùng để cải thiện thông báo lỗi, không làm bằng chứng thực thi chính sách. Một số PRAGMA có tác dụng trong giai đoạn prepare — chặn sau prepare là quá muộn. Ngoài ATTACH: xét thêm **VACUUM INTO** (tạo db ở đường dẫn tuỳ ý).
2. **`cap[Dir]` chưa tự động bảo vệ mọi I/O của SQLite** — SQLite tạo tệp tạm ngoài journal/WAL (TEMP db, transient indexes, sort…). Prototype phải làm rõ: db-file + phụ kiện; symlink/reparse point + thay đổi đường dẫn sau check; temporary storage; URI filename; tên rỗng; tên đặc biệt; chính sách mở db đã có WAL; cơ chế bảo vệ thực sự (**VFS / OS sandbox / phương án khác có bằng chứng**). **Nếu chưa bảo đảm file-backed strict-caps ⇒ từ chối rõ trong strict mode, không âm thầm hạ mức.** `:memory:` chấp nhận trước (vẫn phải kiểm soát temporary storage).
3. **Bundled ≠ không cần toolchain native** — phải chứng minh C compiler/linker trên máy build; không ghi "không dev kit vẫn build" như bảo đảm chung; không ratify con số binary size/hiệu năng chưa đo.

### Chốt A–N (theo nguyên văn quyết định của A0)

- **A. Distribution**: **rusqlite bundled** cho CLI chính thức; ghim dependency; ghi phiên bản SQLite + compile options; không tự viết binding C thay thế để tránh dependency.
- **B. Row shape**: kết quả `columns: list[str]` + `rows: list[list[SqlValue]]`; giữ nguyên thứ tự và tên cột; chưa streaming.
- **C. SQL value**: **enum có kiểu** `SqlValue`: `Null`, `Integer(int)`, `Real(float)`, `Text(str)`, `Blob(bytes)`. **Không** dùng `Unit`/`Option` biểu diễn NULL; **không** tự suy `0/1` thành bool.
- **D. Params**: danh sách `SqlValue` theo vị trí; bind đủ số lượng; thiếu/thừa ⇒ lỗi; chưa implicit heterogeneous conversion, chưa named params.
- **E. Transaction**: cho SQL `BEGIN/COMMIT/ROLLBACK` trong `execute`; closure transaction wrapper hoãn; lỗi không được tự commit; không hứa transaction luôn còn hoạt động sau mọi lỗi — cần **cách đọc trạng thái transaction** (E-check, §hợp đồng bên dưới).
- **F. Close**: `close()` tường minh + cleanup khi kết thúc **mỗi phiên chạy Host**, không chỉ lúc process exit; **không auto-commit**; cảnh báo tài nguyên chưa đóng **chỉ** ở chế độ diagnostics, **không** tự in stderr mặc định.
- **G. Capability**: duyệt yêu cầu quyền thư mục cho file-backed DB nhưng **chưa duyệt** implementation "check path rồi open"; phải qua cổng bảo vệ I/O (mục 2.2 addendum); file-backed strict-caps chỉ mở khi có bằng chứng kiểm soát I/O đầy đủ — chưa đạt thì từ chối rõ.
- **H. SQL policy**: MVP **cấm ATTACH/DETACH, load extension và VACUUM (INTO / VACUUM nói chung theo quyết định A0)** ở mọi chế độ; `PRAGMA` chỉ `table_info` và **đọc** `user_version`; **không cho ghi `user_version`**; thực thi bằng **authorizer**, không keyword scan.
- **I. Open**: cho `:memory:`; file mặc định **read-write + create: true**; có `create: false`; chưa thêm read-only mode; từ chối **tên rỗng** và **URI filename** trong MVP.
- **K. Module**: tên module **`std.sqlite`** duyệt; `SqlValue` **thuộc module**, không đưa vào prelude toàn cục.
- **L. API freeze**: `api_dong_bang.rs` chỉ cập nhật sau khi chữ ký API qua cổng thiết kế; **không** tự đổi VBC1 hoặc thêm runtime Value variant.
- **M. Nền tảng**: Linux trước; Windows/macOS phải chạy native build + SQLite tests trước khi tuyên bố hỗ trợ nền; **không** mở lại Windows Tier 2 cũ.
- **N. Thời điểm**: mở VD và prototype **ngay**; chưa merge API công khai / phát hành trước cổng tích hợp.

### Chốt kiến trúc kiểu & handle (lệnh A0)

1. **`SqlValue` phải thực sự có kiểu** — dùng enum ngôn ngữ qua cơ chế enum runtime sẵn có: **`list[list[Value]]` ≠ `list[list[SqlValue]]`**. Agent 2 phải chứng minh checker, constructor, pattern và hai engine xử lý enum này đúng. Không dùng Any hoặc kiểu lỗi nội bộ để lách chữ ký. Nếu chưa làm được bằng cơ chế hiện tại mà không đổi ABI ⇒ **báo A0**, không tự mở rộng ABI.
2. **Connection không được là chuỗi dễ giả mạo** — tái sử dụng có thể bảng handle của host nhưng phải: kiểm tra loại handle + chủ sở hữu phiên chạy; từ chối handle đã đóng; **không tái sử dụng ID** khiến handle cũ trỏ nhầm connection mới; biết chuỗi `"db:0"` **không** là bằng chứng quyền.
3. `db.query()` method API là mục tiêu công thái học, **không phải lý do** mở rộng method dispatch ngoài phạm vi; prototype dùng **hàm module** trước, trình cách viết công khai sau.

### Bổ sung giới hạn tài nguyên và kiểm thử

- Giới hạn: **số hàng** và **tổng kích thước kết quả**; kích thước SQL, text/blob và số tham số; **thời gian thực thi / ngân sách thực thi có cơ chế ngắt**; **số connection** trong một phiên chạy. Vượt ⇒ **lỗi**, không trả dữ liệu bị cắt như kết quả thành công.
- Test matrix (cổng kiểm thử): kết quả độc lập + parity interp/VM; NULL/BLOB rỗng/Unicode/số biên/cột trùng tên; bind thiếu/thừa; chuỗi chứa dấu nháy; từ chối nhiều statement (cơ chế parser/tail); commit/rollback/lỗi trong transaction + cleanup khi thoát sớm; hai connection tranh khóa với timeout ngắn do test cấu hình; handle giả/handle cũ/rò tài nguyên qua nhiều lần chạy trong cùng process; đường I/O ngoài quyền (không chỉ ATTACH); **mutation trên authorizer/kiểm soát I/O thực tế**, không phải mutation keyword scan.

### Trạng thái các finding còn lại (A0 ghi chú)

- `let x: int?? = None` runtime **còn sai** (bảng A0 Option-lồng: phải là `None` lớp ngoài); bất đối xứng Sugar/spelled-out/alias chưa chốt — đều **đang mở**, không ghi DONE.
- RFC typed serialization và Windows Tier 2 không đụng tới quyết định này.
