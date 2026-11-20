<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Nhật ký các đợt (CHANGELOG)

Tóm tắt theo từng đợt phát triển. Chi tiết đầy đủ ở [`agent/reports/`](agent/reports/) (mỗi đợt một cặp `.json` + `.md`).

| Đợt | HEAD (mac) | Cổng `--fast` | Số task trong báo cáo |
|---|---|---|---|
| R1 | `b4d7d469` | PASS | 11 |
| R2 | `43107a40` | PASS | 20 |
| R3 | `d971db57` | PASS | 27 |
| R4 | `0463512b` | PASS | 36 |
| R5 | `10353111` | PASS | 49 |
| R6 | `e0f6c234` | PASS | 49 |
| R7 | `29c13395` | PASS | 53 |
| R8 | `bb8393fe` | PASS | 58 |
| R9 | `d3b11346` | PASS | 61 |
| R10 | `8e57eef2` | PASS | 85 |
| R11 | `2f490b41` | PASS | 86 |
| R12 | `8b234a1f` | PASS | 92 |
| R13 | `c1228f1c` | PASS | 100 |
| R14 | `5f068f77` | PASS | 104 |
| R15 | `d012624a` | PASS | 108 |
| R16 | `005b12a8` | PASS | 112 |
| R17 | `e446481c` | PASS | 117 |
| R18 | `637bbebb` | PASS | 121 |
| R19 | `4bb3d0c7` | PASS | 125 |
| R20 | `536f802d` | PASS | 127 |
| R21 | `fcf3ae63` | PASS | 130 |
| R22 | `87fd4f9d` | PASS | 134 |
| R23 | `37d57145` | PASS | 153 |
| R24 | `5e364155` | PASS | 156 |
| R25 | `43b19dfe` | PASS | 164 |
| R26 | `b1c07802` | PASS | 171 |
| R27 | `dd088322` | PASS | 174 |
| R28 | `fdb21d15` | PASS | 177 |
| R29 | `39041b3f` | PASS | 197 |
| R30 | `0d5e0e57` | PASS | 223 (222 DONE) |

## Mốc đáng chú ý

- **2026-11-21** — VD-267 (R10 — hiển thị đơn vị): `size` in **thập phân** `b kb mb gb tb`, chọn đơn vị lớn nhất có phần nguyên ≥ 1, tối đa **2** chữ số lẻ (làm tròn nửa lên, bỏ 0 thừa), âm giữ dấu, `0` ⇒ `0b` (`1024` ⇒ `1.02kb`, `1500` ⇒ `1.5kb`; `kib`/`mib` chỉ còn là thang literal); `duration` in **ghép** `w d h m s ms us ns`, bỏ thành phần 0, không cách, phân rã nguyên chính xác, âm giữ dấu, `0` ⇒ `0ns` (`90s` ⇒ `1m30s`, `2h+30m` ⇒ `2h30m`). `print`, `str(x)`, nội suy `{x}` và cả hai engine dùng chung `Value::show`; không đổi số học/literal. `crates/vis-cli/tests/vd267_fmt_size.rs`; `gate --fast` PASS.
- **2026-11-21** — VD-268 (`std.fs.walk`): liệt kê **đệ quy** dưới một thư mục — đường dẫn **tương đối** so với root, **thư mục kèm `/`**, DFS với mỗi cấp sắp theo byte tên, gồm mục ẩn; **bỏ hoàn toàn symlink** (không in, không đệ quy) nên vòng symlink không lặp và không thoát root; root phải qua `cap[Dir]`/`FsGuard` (thiếu ⇒ `VS0604`), lỗi dữ liệu (thiếu path/không phải thư mục) ⇒ `Result.Err`; host thật + `--fs-mock`, interp ↔ VM byte-identical; API-STD 73 → 74 hàm; `gate --fast` PASS (1976 passed / 3 skipped).
- **2026-11-21** — VD-266 (thư viện `std.fs`): thêm `read_lines` (tách dòng CRLF, bỏ newline cuối tệp), `append` (tạo tệp nếu thiếu, không cắt), `mkdir` (**một cấp**, lỗi nếu đã tồn tại), `remove` (**chỉ tệp**, từ chối thư mục); mọi thao tác ghi vẫn qua `FsGuard`/`cap[Dir] (write)`; `--fs-mock` và hai đường interp/VM khớp byte; API-STD 61 → 65 hàm.
- **R29** — số 3 nền cùng HEAD: mac **1184/0**, ps **1188/0**, psvn **152/4** (4 ca `cap_proc` do thiếu `/bin/echo`, `/bin/sh` trên Windows).
- **R30** — pha 1 công thái học (C3 VD-178), tách repo; trạng thái theo `agent/reports/R30-viscript.{md,json}`.
- **2026-11-21** — VD-267 (builtin `panic`): `panic(msg)` là **builtin trần** (`BUILTIN_VALUE_NAMES`), kiểu tĩnh **`never`** nên dùng được ở mọi vị trí biểu thức (`let y: int = if c: 1 else: panic("bad")`, `return panic(...)`, `let f = panic`); payload `str` hoặc bản ghi `Error` (lấy `message`, thiết kế §6.4). Runtime dùng **chung** lõi `builtin::call` ⇒ interp ↔ VM **byte-identical**, trap `VS0501` + exit **101**, không Rust panic/abort; checker báo `VS0401` (số đối số) / `VS0412` (kiểu). `stdlib::Ty::Never` + bảng `BUILTIN_FUNCS` (không đụng freeze `FUNCS`/`API-STD.txt`); `SIGS` tách sang `builtin_sig.rs` để giữ R1. Gate `--fast` PASS (**1906** passed / 3 skipped).
- **2026-11-21** — VD-261/263/264 (an toàn ngữ nghĩa + hiệu năng): `let … else` buộc khối `else` **thoát mọi nhánh** (`VS0206`), binder theo pattern (`Err` payload hoặc `()`), scope snapshot đúng; interpreter lan `break`/`continue` ra khỏi `if`/`match` biểu thức (khớp VM); `with` lấy kiểu tĩnh từ mọi gốc; `[]` pattern **chỉ** khớp list (không khớp `()`); generic `G[int?]`/`enum Hop[T]` mang instantiation vào trường/ctor/`with`/gán; suy kiểu sau pattern lồng + `Call`/`Try`/`With`/`If`/`Match`; cache SQLite theo `(schema_version, data_version)`; **range duyệt lười** trong `for` (RSS 146 MB → ~9,5 MB @2M); cache `PHIEN_BAN` ghim lại (`f20c2ade…`); gate **1822 passed / 3 skipped**, core-test/release/toolchain PASS.
- **2026-11-21** — VD-265 (stdlib thực dụng): thêm `std.list.zip` (ghép cặp, dừng ở danh sách ngắn hơn), `std.list.min`/`max` (gộp theo thứ tự `list.sort`; danh sách rỗng ⇒ **trap `VS0501`**, không panic), `std.str.lines` (tách `\n`, bỏ `\r`, dòng cuối không sinh dòng rỗng) và `std.str.split_whitespace` (khoảng trắng Unicode); interp ↔ VM dùng **cùng** lõi `stdlib::call`, sugar `s.lines()`/`s.split_whitespace()` theo R5c; `gate --fast` PASS.
- **2026-11-21** — VD-266 (SQLite thực dụng): thêm `std.sqlite.query_one` (như `query`, **chỉ đọc**, trả **0/1** hàng và **dừng sau hàng đầu** — không materialize/quét hết; cùng cap hàng/ô/tên cột/timeout), `std.sqlite.last_insert_rowid` và `std.sqlite.changes` (metadata chỉ đọc của connection). Không đổi authorizer/chính sách bảo mật; thiếu host ⇒ `VS0604`, lỗi SQL/params ⇒ `Result.Err`; interp ↔ VM byte-identical; `gate --fast` PASS (1903 passed / 3 skipped).
- **2026-11-21** — VD-258/259/260 (checker, frame/async, SQLite, tính năng, tooling): checker sửa `Result?` (giữ đúng một lớp `Option`), `None` có kiểu, record ctor/default, `?.` lồng, nâng Option ở biên dựng/`with`/`push` và builtin trait/ctor đa tệp; frame/async sửa redispatch stack, method await, deadlock, closure/const env, fake-source join, exit code (`Result.Err` ⇒ **1**; `main -> int` **không** phải mã thoát) cùng ready-budget/purge; SQLite siết cell/overhead cap, chặn `dbstat`/trang thô, schema UTF-8, `query` **chỉ đọc**; thêm `std.path` cho đường dẫn Windows (drive/UNC/hai separator) và list rest `[a, ..]`/`[a, ..r]`; tooling `bin/vis-release` (chỉ `vis`) + `vis-dev core-test`; `gate --fast` PASS, `core-test` PASS.
- **2026-11-20** — VD-winport: Linux xanh toàn workspace @ `c7cef71` (gate ×3 PASS; nightly PASS @ `f9430b9`); **Windows**: @ `6ccf367` **7 suite đỏ** → @ `f9430b9` **1 ca đỏ** → @ `c7cef71` & **@ `2d93101` exit 0, toàn workspace XANH** (150 khối ok, 0 đỏ) — VD-winport hoàn tất fix A–E; `pha1-rc1` nguyên; Tier 2 giữ.
- **ST-174** — đóng nợ **#146** (TOCTOU macOS): vá `symlink_metadata` + chứng minh tất định 100/100, 6 triệu vòng 0 đỏ.
- **ST-171** — tách repo ViScript khỏi ViFastOs: 459 commit giữ lịch sử, gate mac/ps khớp R29.
- **ST-176** — nợ **#147**: mac 1489 s vs ps 261 s = 5,70×; nguyên nhân tạo tiến trình/syscall macOS (chưa đóng).
- **ST-173** — Landlock chỉ Linux: mac hết lỗi biên dịch, ps 15/15 không đổi.
