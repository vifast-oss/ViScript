// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-58 (VD-94/VD-99)** — `Ty::Any` **xoá hẳn**: chữ ký `std.*` dùng **biến kiểu** (`Var`/`List`/`Map`).
//!
//! Khoá ba việc:
//! 1. **Không còn** `Ty::Any` — *bảo đảm ở mức biên dịch* (biến thể đã xoá khỏi `enum Ty`; test này chỉ
//!    còn khẳng định **mọi** chỗ trong bảng đều hạ được ra `Type` và `Var` chỉ dùng `0..3`).
//! 2. 16 chữ ký đã đổi có **đúng hình dạng** (`list[T]`, `map[K,V]`, trả `T`/`list[K]`/…).
//! 3. `sang_type()` **tổng** (không `Option`): `Var(i)` → `TypeRef` tên `T`/`K`/`V`.

use vis_types::stdlib::{sig_builtin, sig_of, Ty, FUNCS};
use vis_types::Type;

/// Tên biến kiểu theo chỉ số (VD-99 (1): `0=T, 1=K, 2=V`).
const TEN_BIEN: [&str; 3] = ["T", "K", "V"];

fn kiem_var(t: Ty) {
    if let Ty::Var(i) = t {
        assert!((i as usize) < TEN_BIEN.len(), "Ty::Var({i}) ngoài 0..3");
    }
    match t {
        Ty::List(x) => kiem_var(*x),
        Ty::Map(k, v) => {
            kiem_var(*k);
            kiem_var(*v);
        }
        _ => {}
    }
}

#[test]
fn moi_chu_ky_ha_duoc_va_bien_kieu_trong_0_3() {
    for f in &FUNCS {
        for p in f.params {
            kiem_var(*p);
            // `FnKhoa` là **ngoại lệ có ghi** ⇒ hạ thành `{error}` (tầng chạy kiểm, ST-56);
            // còn lại **mọi** tham số phải hạ được (tổng).
            if matches!(p, Ty::FnKhoa) {
                assert_eq!(p.sang_type(), Type::Error);
                continue;
            }
            assert_ne!(
                p.sang_type(),
                Type::Error,
                "{}: tham số không hạ được",
                f.name
            );
        }
        kiem_var(f.ret);
        assert_ne!(
            f.ret.sang_type(),
            Type::Unit,
            "{}: ret hạ thành unit",
            f.name
        );
    }
    // **SỬA KÈM LÝ DO (ST-63, VD-105)**: +5 hàm `std.net.*` (connect, connect_timeout, read, write,
    // close) ⇒ **43** hàm; `MODULES` 7 → **8** (thêm `net`). Không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (ST-64)**: +2 hàm `std.http.*` (`http.get`, `http.get_timeout`) ⇒ **45** hàm;
    // `MODULES` 8 → **9** (thêm `http`). Bỏ `http.body_len` — không cần cho ca cap (xem STATE ST-64).
    // **SỬA KÈM LÝ DO (ST-65)**: +5 hàm server `std.net.*` (listen, local_addr, accept,
    // accept_timeout, close_listen) ⇒ **50** hàm. Bỏ `net.is_listening` (không cần; 5 hàm vừa đủ).
    // **SỬA KÈM LÝ DO (ST-109)**: +2 hàm `std.http.*` (`http.head`, `http.head_timeout`) ⇒ **52** hàm.
    // **SỬA KÈM LÝ DO (VD-241 Gate 1, 2026-11-19)**: +**9** hàm `std.sqlite.*` (open, open_with, close,
    //   execute, query, begin, commit, rollback, in_transaction) ⇒ **61** hàm; `MODULES` 9 → **10**
    //   (thêm `sqlite`). Prototype nội bộ — **không** freeze API (`api_dong_bang` chỉ chỉnh sau
    //   cổng tích hợp).
    // **SỬA KÈM LÝ DO (VD-265, 2026-11-21)**: +**5** hàm thực dụng — `list.zip`, `list.min`,
    //   `list.max`, `str.lines`, `str.split_whitespace` ⇒ **66** hàm. Không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-266, 2026-11-21)**: +**4** hàm `std.fs.*` (`read_lines`, `append`,
    // `mkdir`, `remove`) và +**3** hàm SQLite (`sqlite.query_one`, `sqlite.last_insert_rowid`,
    // `sqlite.changes`) ⇒ **73** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-268, 2026-11-21)**: +**1** hàm `std.fs.walk` ⇒ **74** hàm; không
    // xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-268, 2026-11-21)**: +**1** hàm `std.fs.glob` ⇒ **75** hàm; không
    // xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-269, 2026-11-21)**: +**5** hàm `std.str.*` (`trim_start`, `trim_end`,
    // `pad_left`, `pad_right`, `center`) ⇒ **80** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-269, 2026-11-21)**: +**2** hàm `std.fs.*` (`copy`, `rename`) ⇒ **82**
    // hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270, 2026-11-21)**: +**1** hàm `std.list.windows` (cửa sổ trượt,
    // `list[list[T]]`) ⇒ **83** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270, 2026-11-21)**: +**3** hàm `std.fs.*` metadata chỉ đọc (`size`,
    // `is_file`, `is_dir`) ⇒ **86** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 strfind, 2026-11-21)**: +**3** hàm `std.str.*` (`find`, `rfind`,
    // `count`; `find`/`rfind` trả `Ty::OptInt` = `int?`) ⇒ **89** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 strlen, 2026-11-21)**: +**5** hàm — `str.byte_len`,
    // `str.char_count`, `str.is_empty`, `list.is_empty`, `map.is_empty` ⇒ **94** hàm; không
    // xoá/đổi hàm cũ nào (`grapheme_count` cố ý **không** thêm: cần dependency Unicode mới).
    // **SỬA KÈM LÝ DO (VD-270 indexof, 2026-11-21)**: +**2** hàm `std.list.*` (`index_of`,
    // `last_index_of`; `(list[T], T) -> int?`) ⇒ **96** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 db-batch, 2026-11-21)**: +**1** hàm `sqlite.execute_batch`
    // (`(str, str) -> Result[int, Error]`; nhiều câu, không params) ⇒ **97** hàm; không
    // xoá/đổi hàm cũ nào (`execute` vẫn một câu).
    // **SỬA KÈM LÝ DO (VD-270 strtrim, 2026-11-21)**: +**3** hàm `std.str.*` (`trim_matches`,
    // `trim_start_matches`, `trim_end_matches`; `(str, str) -> str`) ⇒ **100** hàm; không
    // xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 flatten, 2026-11-21)**: +**1** hàm `std.list.flatten`
    // (`(list[list[T]]) -> list[T]`; nối **một cấp**, list con rỗng không đóng góp) ⇒ **101**
    // hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 db-explain, 2026-11-21)**: +**1** hàm `sqlite.explain`
    // (`(str, str, list[SqlValue]) -> Result[RowSet, Error]`; chỉ SELECT/WITH, chạy
    // `EXPLAIN QUERY PLAN` qua đúng đường `query`) ⇒ **102** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 fsbytes, 2026-11-21)**: +**2** hàm `std.fs.*` (`read_bytes`,
    // `write_bytes`; byte thô `list[int]`, mỗi phần tử `0..=255`) ⇒ **104** hàm; không xoá/đổi
    // hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 strstrip, 2026-11-21)**: +**2** hàm `std.str.*` (`strip_prefix`,
    // `strip_suffix`; `(str, str) -> str?` = `Option[str]`) ⇒ **106** hàm; không xoá/đổi hàm
    // cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 path-normalize, 2026-11-21)**: +**1** hàm `std.path.normalize`
    // (`(str) -> str`; chuẩn hoá từ vựng POSIX, thuần chuỗi) ⇒ **107** hàm.
    // **SỬA KÈM LÝ DO (VD-270 binary-search, 2026-11-21)**: +**1** hàm `std.list.binary_search`
    // (`(list[T], T) -> int?`; biên dưới, `xs` phải đã sắp tăng) ⇒ **108** hàm; không xoá/đổi
    // hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 replace-first, 2026-11-21)**: +**1** hàm `std.str.replace_first`
    // (`(str, str, str) -> str`; chỉ thay lần khớp **đầu**, `from` rỗng ⇒ chèn `to` ở đầu) ⇒
    // **109** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 sqlite-tables, 2026-11-21)**: +**1** hàm `sqlite.tables`
    // (`(str) -> Result[list[str], Error]`; bảng/view người dùng của schema chính, sắp byte)
    // ⇒ **110** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 splitn, 2026-11-21)**: +**1** hàm `std.str.splitn`
    // (`(str, str, int) -> list[str]`; tối đa `n` phần, phần cuối giữ nguyên phần còn lại)
    // ⇒ **111** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 sqlite-columns, 2026-11-21)**: +**1** hàm `sqlite.columns`
    // (`(str, str) -> Result[list[str], Error]`; tên cột bảng/view schema chính theo thứ tự
    // khai báo, tên đối tượng bind làm tham số) ⇒ **112** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 parse-int, 2026-11-21)**: +**1** hàm `std.str.parse_int`
    // (`(str) -> int?`; parse thập phân chặt, sai/tràn ⇒ `None`) ⇒ **113** hàm; không xoá/đổi
    // hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 list-contains, 2026-11-21)**: +**1** hàm `std.list.contains`
    // (`(list[T], T) -> bool`; cùng `eq_value` như `index_of`, rỗng ⇒ `false`) ⇒ **114** hàm;
    // không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 sqlite-indexes, 2026-11-21)**: +**1** hàm `sqlite.indexes`
    // (`(str, str) -> Result[list[str], Error]`; tên index người dùng tạo của bảng schema chính,
    // sắp byte, bỏ autoindex, tên bảng bind làm tham số) ⇒ **115** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 parse-float, 2026-11-21)**: +**1** hàm `std.str.parse_float`
    // (`(str) -> float?`; cú pháp thập phân Rust, non-finite/tràn ⇒ `None`) ⇒ **116** hàm;
    // không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 list-sum-product, 2026-11-21)**: +**2** hàm `std.list.sum` /
    // `std.list.product` (`(list[int]) -> int`; gộp checked, rỗng ⇒ `0`/`1`, tràn ⇒ `VS0507`)
    // ⇒ **118** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 replace-last, 2026-11-21)**: +**1** hàm `std.str.replace_last`
    // (`(str, str, str) -> str`; chỉ thay lần khớp **cuối**, `from` rỗng ⇒ nối `to` ở cuối) ⇒
    // **119** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 path-relative, 2026-11-21)**: +**1** hàm `std.path.relative`
    // (`(str, str) -> str`; tương đối từ vựng POSIX, không sugar) ⇒ **120** hàm; không xoá/đổi
    // hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 sqlite-schema-sql, 2026-11-21)**: +**1** hàm `sqlite.schema_sql`
    // (`(str, str) -> Result[str?, Error]`; tra văn bản SQL khai báo của đối tượng schema chính
    // theo tên bind, thiếu/`sql` NULL ⇒ `Ok(None)`) ⇒ **121** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 str-title, 2026-11-21)**: +**1** hàm `std.str.title`
    // (`(str) -> str`; hoa scalar đầu mỗi từ theo khoảng trắng Unicode, `ß` ⇒ `SS`) ⇒ **122** hàm;
    // không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 sqlite-index-columns, 2026-11-21)**: +**1** hàm
    // `sqlite.index_columns` (`(str, str) -> Result[list[str], Error]`; tên cột của index schema
    // chính theo thứ tự index, tra theo tên, tên index bind làm tham số) ⇒ **123** hàm;
    // không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 path-join-all, 2026-11-21)**: +**1** hàm `std.path.join_all`
    // (`(list[str]) -> str`; nối trái→phải bằng đúng quy tắc `path.join`, rỗng ⇒ `""`,
    // **không** sugar) ⇒ **124** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 list-mean, 2026-11-21)**: +**1** hàm `std.list.mean`
    // (`(list[float]) -> float?`; rỗng ⇒ `None`, ngược lại cộng trái→phải `f64` rồi chia `len`,
    // IEEE: NaN/inf lan truyền) ⇒ **125** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 sqlite-primary-keys, 2026-11-21)**: +**1** hàm
    // `sqlite.primary_keys` (`(str, str) -> Result[list[str], Error]`; tên cột khoá chính của
    // bảng schema chính theo thứ tự `pk`, tên bảng bind làm tham số) ⇒ **126** hàm;
    // không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 mkdir-all, 2026-11-21)**: +**1** hàm `std.fs.mkdir_all`
    // (`(str) -> Result[(), Error]`; tạo đệ quy mọi thư mục thiếu, đã có ⇒ `Ok`,
    // tệp/symlink trên đường ⇒ `Err`) ⇒ **127** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 path-is-relative, 2026-11-21)**: +**1** hàm
    // `std.path.is_relative` (`(str) -> bool`; phần bù từ vựng của `path.is_abs`, rỗng ⇒
    // `true`, **không** sugar) ⇒ **128** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 list-median, 2026-11-21)**: +**1** hàm `std.list.median`
    // (`(list[float]) -> float?`; rỗng ⇒ `None`, có `NaN` ⇒ `None`, sao chép rồi sắp tăng,
    // lẻ ⇒ giữa, chẵn ⇒ `(a + b) / 2.0`) ⇒ **129** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 str-is-ascii, 2026-11-21)**: +**1** hàm `std.str.is_ascii`
    // (`(str) -> bool`; đúng ⇔ mọi byte thuộc `0x00..=0x7F`, rỗng ⇒ `true`) ⇒ **130** hàm;
    // không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 sqlite-table-exists, 2026-11-21)**: +**1** hàm
    // `sqlite.table_exists` (`(str, str) -> Result[bool, Error]`; bảng/view tên đúng trong schema
    // chính, tên bind, so khớp BINARY, thiếu ⇒ `Ok(false)`) ⇒ **131** hàm; không xoá/đổi hàm cũ.
    // **SỬA KÈM LÝ DO (VD-270 list-sum-float, 2026-11-21)**: +**1** hàm `std.list.sum_float`
    // (`(list[float]) -> float`; hạt giống `+0.0`, rỗng ⇒ `0`, cộng trái→phải `f64`, IEEE:
    // NaN/±inf lan truyền, mọi tổng 0 đều `+0.0`) ⇒ **132** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 path-with-extension, 2026-11-21)**: +**1** hàm
    // `std.path.with_extension` (`(str, str) -> str`; thay/đặt/bỏ phần mở rộng thành phần cuối,
    // ext rỗng ⇒ xoá, tên bắt đầu `.` không có ext, **không** sugar) ⇒ **133** hàm;
    // không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 sqlite-index-exists, 2026-11-21)**: +**1** hàm
    // `sqlite.index_exists` (`(str, str) -> Result[bool, Error]`; index tên đúng trong schema
    // chính, tên bind, so khớp BINARY, thiếu/rỗng ⇒ `Ok(false)`) ⇒ **134** hàm; không xoá/đổi hàm cũ.
    // **SỬA KÈM LÝ DO (VD-270 replace-n, 2026-11-21)**: +**1** hàm `std.str.replace_n`
    // (`(str, str, str, int) -> str`; thay tối đa `n` lần khớp **không chồng** đầu tiên theo
    // Rust `replacen`, `n == 0` ⇒ nguyên vẹn, `n` quá số lần ⇒ thay tất cả, `n < 0` ⇒ trap
    // `VS0501`) ⇒ **135** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 map-contains-key, 2026-11-21)**: +**1** hàm
    // `std.map.contains_key` (`(map[K, V], K) -> bool`; đúng ⇔ có khoá bằng theo `eq_value`
    // như `map.has`/`get`, rỗng/không có ⇒ `false`, không trap; sugar R5c) ⇒ **136** hàm;
    // không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 char-at, 2026-11-21)**: +**1** hàm `std.str.char_at`
    // (`(str, int) -> str?`; scalar Unicode thứ `i` zero-based, âm/quá dài/rỗng ⇒ `None`,
    // không trap) ⇒ **137** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 path-is-absolute, 2026-11-21)**: +**1** hàm
    // `std.path.is_absolute` (`(str) -> bool`; cùng quy tắc từ vựng `path.is_abs`, rỗng ⇒
    // `false`, **không** sugar) ⇒ **138** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 is-blank, 2026-11-21)**: +**1** hàm `std.str.is_blank`
    // (`(str) -> bool`; rỗng hoặc **mọi scalar** `char::is_whitespace` ⇒ `true`, một scalar
    // khác ⇒ `false`) ⇒ **136** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 sqlite-schema-version, 2026-11-21)**: +**1** hàm
    // `sqlite.schema_version` (`(str) -> Result[int, Error]`; schema cookie của schema chính
    // đọc qua `PRAGMA schema_version`, tăng khi DDL đổi schema, chỉ đọc) ⇒ **136** hàm;
    // không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 path-with-stem, 2026-11-21)**: +**1** hàm
    // `std.path.with_stem` (`(str, str) -> str`; thay stem thành phần cuối, giữ ext hiện có,
    // **không** sugar) ⇒ **141** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 split-once, 2026-11-21)**: +**1** hàm `std.str.split_once`
    // (`(str, str) -> list[str]?`; tách tại lần xuất hiện **đầu** của `sep`, `Some` đúng 2
    // phần, không khớp ⇒ `None`, `sep` rỗng ⇒ `Some(["", s])`) ⇒ **141** hàm;
    // không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 sqlite-user-version, 2026-11-21)**: +**1** hàm
    // `sqlite.user_version` (`(str) -> Result[int, Error]`; số ứng dụng tự định nghĩa trong
    // header tệp DB đọc qua `PRAGMA user_version`, DDL/ghi dữ liệu không đổi, lưu theo tệp
    // trong `0..=i32::MAX`, chỉ đọc) ⇒ **141** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 fs-is-symlink, 2026-11-21)**: +**1** hàm `std.fs.is_symlink`
    // (`(str) -> Result[bool, Error]`; `symlink_metadata`, symlink tới tệp/thư mục/đích thiếu
    // ⇒ `true`, tệp/thư mục thường/thiếu ⇒ `false`, lỗi khác ⇒ `Err`; **không** sugar) ⇒ **141**
    // hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 rsplit-once, 2026-11-21)**: +**1** hàm `std.str.rsplit_once`
    // (`(str, str) -> list[str]?`; tách tại lần xuất hiện **cuối** của `sep`, `Some` đúng 2
    // phần, không khớp ⇒ `None`, `sep` rỗng ⇒ `Some([s, ""])`; tái dùng `Ty::OptListStr`,
    // không thêm biến thể `Ty`) ⇒ **145** hàm; không xoá/đổi hàm cũ nào.
    // **SỬA KÈM LÝ DO (VD-270 list-minmax-float, 2026-11-21)**: +**2** hàm
    // `std.list.min_float`/`std.list.max_float` (`(list[float]) -> float?`; rỗng ⇒ `None`, có
    // `NaN` ⇒ `None`, `±inf` so bình thường, phần tử bằng nhau kể cả `±0.0` giữ đầu tiên) ⇒ **146**
    // hàm; không xoá/đổi hàm cũ nào.
    assert_eq!(FUNCS.len(), 147, "bảng chữ ký phải đủ 147 hàm");
}

#[test]
fn fs_mkdir_all_kieu_str_result_va_khong_sugar() {
    // **VD-270 (mkdir-all)**: `(str) -> Result[(), Error]`; fs là module-only nên **không** sugar
    // (`p.mkdir_all()` không được ánh xạ sang `fs`).
    let f = sig_of("fs.mkdir_all").expect("fs.mkdir_all");
    assert_eq!(f.params, &[Ty::Str], "`fs.mkdir_all`: tham số");
    assert_eq!(f.ret, Ty::ResultUnitStr, "`fs.mkdir_all`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(Box::new(Type::Unit), Box::new(Ty::Error.sang_type())),
        "`fs.mkdir_all`: hạ kiểu trả"
    );
    assert_eq!(vis_types::stdlib::module_sugar("str", "mkdir_all"), None);
}

#[test]
fn fs_is_symlink_kieu_str_result_va_khong_sugar() {
    // **VD-270 (fs-is-symlink)**: `(str) -> Result[bool, Error]`, tái dùng `Ty::ResultBoolErr`
    // (cùng biến thể của `fs.is_file`/`fs.is_dir`); fs là module-only nên **không** sugar —
    // `p.is_symlink()` bị `VS0403`, không ánh xạ sang `fs`.
    let f = sig_of("fs.is_symlink").expect("fs.is_symlink");
    assert_eq!(f.params, &[Ty::Str], "`fs.is_symlink`: tham số");
    assert_eq!(
        f.ret,
        Ty::ResultBoolErr,
        "`fs.is_symlink`: kiểu trả phải tái dùng `ResultBoolErr`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(Box::new(Type::Bool), Box::new(Ty::Error.sang_type())),
        "`fs.is_symlink`: hạ kiểu trả"
    );
    assert_eq!(vis_types::stdlib::module_sugar("str", "is_symlink"), None);
}

#[test]
fn splitn_kieu_str_str_int_va_sugar() {
    // **VD-270 (splitn)**: chữ ký `(str, str, int) -> list[str]`; tham số đầu `str` để sugar
    // R5c `s.splitn(sep, n)` nhận, `n` là `int` (không phải `str`).
    let f = sig_of("str.splitn").expect("str.splitn");
    assert_eq!(
        f.params,
        &[Ty::Str, Ty::Str, Ty::Int],
        "`str.splitn`: tham số"
    );
    assert_eq!(f.ret, Ty::ListStr, "`str.splitn`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::List(Box::new(Type::Str)),
        "`str.splitn`: hạ kiểu trả"
    );
    // Không cần bảng `module_sugar`: module mặc định của người nhận `str` đã là `str`.
    assert_eq!(vis_types::stdlib::module_sugar("str", "splitn"), None);
}

#[test]
fn split_once_kieu_str_str_va_sugar() {
    // **VD-270 (split-once)**: chữ ký `(str, str) -> list[str]?`; tham số đầu `str` để sugar
    // R5c `s.split_once(sep)` nhận. `Ty` không có `Opt` tổng quát nên dùng `OptListStr`
    // (`list[str]?` = `Option[list[str]]`), không tái dùng `ListStr`/`OptStr`.
    let f = sig_of("str.split_once").expect("str.split_once");
    assert_eq!(f.params, &[Ty::Str, Ty::Str], "`str.split_once`: tham số");
    assert_eq!(f.ret, Ty::OptListStr, "`str.split_once`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::Opt(Box::new(Type::List(Box::new(Type::Str)))),
        "`str.split_once`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "list[str]?", "`str.split_once`: tên hiển thị");
    // Không cần bảng `module_sugar`: module mặc định của người nhận `str` đã là `str`.
    assert_eq!(vis_types::stdlib::module_sugar("str", "split_once"), None);
}

#[test]
fn rsplit_once_kieu_str_str_va_sugar() {
    // **VD-270 (rsplit-once)**: chữ ký `(str, str) -> list[str]?`; tham số đầu `str` để sugar
    // R5c `s.rsplit_once(sep)` nhận. Tái dùng **đúng** `Ty::OptListStr` của `split_once`
    // (`list[str]?` = `Option[list[str]]`), không thêm biến thể `Ty` và không tái dùng
    // `ListStr`/`OptStr`.
    let f = sig_of("str.rsplit_once").expect("str.rsplit_once");
    assert_eq!(f.params, &[Ty::Str, Ty::Str], "`str.rsplit_once`: tham số");
    assert_eq!(f.ret, Ty::OptListStr, "`str.rsplit_once`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::Opt(Box::new(Type::List(Box::new(Type::Str)))),
        "`str.rsplit_once`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "list[str]?", "`str.rsplit_once`: tên hiển thị");
    assert_eq!(f.ret, sig_of("str.split_once").unwrap().ret);
    // Không cần bảng `module_sugar`: module mặc định của người nhận `str` đã là `str`.
    assert_eq!(vis_types::stdlib::module_sugar("str", "rsplit_once"), None);
}

#[test]
fn flatten_kieu_long_mot_cap() {
    // **VD-270 (flatten)**: tham số `list[list[T]]`, trả `list[T]`; chỉ **một** cấp nên
    // `list[list[list[int]]]` ⇒ `list[list[int]]` (phần tử list không bị đệ quy làm phẳng).
    let f = sig_of("list.flatten").expect("list.flatten");
    assert_eq!(
        f.params,
        &[Ty::List(&Ty::List(&Ty::Var(0)))],
        "`list.flatten`: tham số"
    );
    assert_eq!(f.ret, Ty::List(&Ty::Var(0)), "`list.flatten`: kiểu trả");
    let long = Ty::List(&Ty::List(&Ty::Var(0))).sang_type();
    assert_eq!(
        f.params[0].sang_type(),
        long,
        "`list.flatten`: hạ `list[list[T]]`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::List(Box::new(Type::TypeRef {
            name: "T".into(),
            args: Vec::new()
        })),
        "`list.flatten`: hạ `list[T]`"
    );
}

#[test]
fn path_normalize_kieu_str_sang_str_va_sugar() {
    // **VD-270 (path.normalize)**: chữ ký `(str) -> str`; sugar R5c của `str` chỉ mở đúng
    // `normalize` → `path`, không mở các hàm `path.*` khác (tránh đổi nghĩa `s.join(...)`).
    let f = sig_of("path.normalize").expect("path.normalize");
    assert_eq!(f.params, &[Ty::Str], "`path.normalize`: tham số");
    assert_eq!(f.ret, Ty::Str, "`path.normalize`: kiểu trả");
    assert_eq!(f.ret.sang_type(), Type::Str, "`path.normalize`: hạ kiểu");
    assert_eq!(
        vis_types::stdlib::module_sugar("str", "normalize"),
        Some("path")
    );
    assert_eq!(vis_types::stdlib::module_sugar("str", "upper"), None);
    assert_eq!(vis_types::stdlib::module_sugar("str", "join"), None);
}

#[test]
fn path_is_absolute_kieu_str_sang_bool_va_khong_sugar() {
    // **VD-270 (path.is_absolute)**: chữ ký `(str) -> bool` — **cùng** quy tắc của
    // `path.is_abs`; **không** sugar (`is_absolute` không nằm trong `module_sugar`, tránh
    // đoán module nhận).
    let f = sig_of("path.is_absolute").expect("path.is_absolute");
    assert_eq!(f.params, &[Ty::Str], "`path.is_absolute`: tham số");
    assert_eq!(f.ret, Ty::Bool, "`path.is_absolute`: kiểu trả");
    assert_eq!(f.ret.sang_type(), Type::Bool, "`path.is_absolute`: hạ kiểu");
    assert_eq!(vis_types::stdlib::module_sugar("str", "is_absolute"), None);
    assert_eq!(vis_types::stdlib::module_sugar("path", "is_absolute"), None);
}

#[test]
fn path_is_relative_kieu_str_sang_bool_va_khong_sugar() {
    // **VD-270 (path.is_relative)**: chữ ký `(str) -> bool` — phần bù của `path.is_abs`;
    // **không** sugar (`is_relative` không nằm trong `module_sugar`, tránh đoán module nhận).
    let f = sig_of("path.is_relative").expect("path.is_relative");
    assert_eq!(f.params, &[Ty::Str], "`path.is_relative`: tham số");
    assert_eq!(f.ret, Ty::Bool, "`path.is_relative`: kiểu trả");
    assert_eq!(f.ret.sang_type(), Type::Bool, "`path.is_relative`: hạ kiểu");
    assert_eq!(vis_types::stdlib::module_sugar("str", "is_relative"), None);
    assert_eq!(vis_types::stdlib::module_sugar("path", "is_relative"), None);
}

#[test]
fn path_relative_hai_tham_so_va_khong_sugar() {
    // **VD-270 (path.relative)**: chữ ký `(str, str) -> str`; **không** sugar — `relative` không
    // nằm trong `module_sugar`, nên `s.relative(to)` bị từ chối thay vì đoán module nhận.
    let f = sig_of("path.relative").expect("path.relative");
    assert_eq!(f.params, &[Ty::Str, Ty::Str], "`path.relative`: tham số");
    assert_eq!(f.ret, Ty::Str, "`path.relative`: kiểu trả");
    assert_eq!(f.ret.sang_type(), Type::Str, "`path.relative`: hạ kiểu");
    assert_eq!(vis_types::stdlib::module_sugar("str", "relative"), None);
    assert_eq!(vis_types::stdlib::module_sugar("path", "relative"), None);
}

#[test]
fn path_with_extension_hai_tham_so_va_khong_sugar() {
    // **VD-270 (path.with_extension)**: chữ ký `(str, str) -> str`; **không** sugar — receiver
    // `str` không có trường `with_extension`, tránh đoán module nhận.
    let f = sig_of("path.with_extension").expect("path.with_extension");
    assert_eq!(
        f.params,
        &[Ty::Str, Ty::Str],
        "`path.with_extension`: tham số"
    );
    assert_eq!(f.ret, Ty::Str, "`path.with_extension`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::Str,
        "`path.with_extension`: hạ kiểu"
    );
    assert_eq!(
        vis_types::stdlib::module_sugar("str", "with_extension"),
        None
    );
    assert_eq!(
        vis_types::stdlib::module_sugar("path", "with_extension"),
        None
    );
}

#[test]
fn path_with_stem_hai_tham_so_va_khong_sugar() {
    // **VD-270 (path.with_stem)**: chữ ký `(str, str) -> str`; **không** sugar — receiver `str`
    // không có trường `with_stem`, tránh đoán module nhận.
    let f = sig_of("path.with_stem").expect("path.with_stem");
    assert_eq!(f.params, &[Ty::Str, Ty::Str], "`path.with_stem`: tham số");
    assert_eq!(f.ret, Ty::Str, "`path.with_stem`: kiểu trả");
    assert_eq!(f.ret.sang_type(), Type::Str, "`path.with_stem`: hạ kiểu");
    assert_eq!(vis_types::stdlib::module_sugar("str", "with_stem"), None);
    assert_eq!(vis_types::stdlib::module_sugar("path", "with_stem"), None);
}

#[test]
fn path_join_all_list_str_sang_str_va_khong_sugar() {
    // **VD-270 (path.join_all)**: chữ ký `(list[str]) -> str`; **không** sugar — `join_all`
    // không nằm trong `module_sugar`, nên `s.join_all(...)`/`xs.join_all()` bị từ chối thay vì
    // đoán module nhận (receiver `str`/`list` đều nhập nhằng với `path`).
    let f = sig_of("path.join_all").expect("path.join_all");
    assert_eq!(f.params, &[Ty::ListStr], "`path.join_all`: tham số");
    assert_eq!(f.ret, Ty::Str, "`path.join_all`: kiểu trả");
    assert_eq!(
        f.params[0].sang_type(),
        Type::List(Box::new(Type::Str)),
        "`path.join_all`: hạ kiểu tham số"
    );
    assert_eq!(f.ret.sang_type(), Type::Str, "`path.join_all`: hạ kiểu trả");
    assert_eq!(vis_types::stdlib::module_sugar("str", "join_all"), None);
    assert_eq!(vis_types::stdlib::module_sugar("list", "join_all"), None);
}

#[test]
fn parse_int_tra_int_option_va_sugar() {
    // **VD-270 (parse-int)**: `(str) -> int?`; tham số đầu `str` để sugar R5c nhận.
    let f = sig_of("str.parse_int").expect("str.parse_int");
    assert_eq!(f.params, &[Ty::Str], "`str.parse_int`: tham số");
    assert_eq!(f.ret, Ty::OptInt, "`str.parse_int`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::Opt(Box::new(Type::Int)),
        "`str.parse_int`: hạ `int?`"
    );
}

#[test]
fn parse_float_tra_float_option_va_sugar() {
    // **VD-270 (parse-float)**: `(str) -> float?`; tham số đầu `str` để sugar R5c nhận.
    let f = sig_of("str.parse_float").expect("str.parse_float");
    assert_eq!(f.params, &[Ty::Str], "`str.parse_float`: tham số");
    assert_eq!(f.ret, Ty::OptFloat, "`str.parse_float`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::Opt(Box::new(Type::Float)),
        "`str.parse_float`: hạ `float?`"
    );
    assert_eq!(f.ret.ten(), "float?", "`str.parse_float`: tên hiển thị");
}

#[test]
fn strlen_kieu_int_va_bool() {
    // **VD-270 (strlen)**: `byte_len`/`char_count` trả `int`; `str.is_empty`/`list.is_empty`/
    // `map.is_empty` trả `bool`; tham số đầu đúng kiểu để R5c nhận sugar `s.byte_len()`, v.v.
    for ten in ["str.byte_len", "str.char_count"] {
        let f = sig_of(ten).unwrap_or_else(|| panic!("thiếu chữ ký `{ten}`"));
        assert_eq!(f.params, &[Ty::Str], "`{ten}`: tham số");
        assert_eq!(f.ret, Ty::Int, "`{ten}`: kiểu trả");
        assert_eq!(f.ret.sang_type(), Type::Int, "`{ten}`: hạ kiểu");
    }
    let s = sig_of("str.is_empty").expect("str.is_empty");
    assert_eq!(s.params, &[Ty::Str]);
    assert_eq!(s.ret, Ty::Bool);
    assert_eq!(s.ret.sang_type(), Type::Bool);
    let l = sig_of("list.is_empty").expect("list.is_empty");
    assert_eq!(l.params, &[Ty::List(&Ty::Var(0))]);
    assert_eq!(l.ret, Ty::Bool);
    let m = sig_of("map.is_empty").expect("map.is_empty");
    assert_eq!(m.params, &[Ty::Map(&Ty::Var(1), &Ty::Var(2))]);
    assert_eq!(m.ret, Ty::Bool);
}

#[test]
fn map_contains_key_kieu_map_k_bool() {
    // **VD-270 (map-contains-key)**: `(map[K, V], K) -> bool`; `K` buộc từ map nên sai kiểu
    // khoá ⇒ `VS0412`; tham số đầu `map` để sugar R5c `m.contains_key(k)` nhận.
    let f = sig_of("map.contains_key").expect("map.contains_key");
    assert_eq!(
        f.params,
        &[Ty::Map(&Ty::Var(1), &Ty::Var(2)), Ty::Var(1)],
        "`map.contains_key`: tham số"
    );
    assert_eq!(f.ret, Ty::Bool, "`map.contains_key`: kiểu trả");
    assert_eq!(f.ret.sang_type(), Type::Bool, "`map.contains_key`: hạ kiểu");
    assert_eq!(f.so_doi(), 2, "`map.contains_key`: arity");
    assert_eq!(vis_types::stdlib::module_sugar("map", "contains_key"), None);
}

#[test]
fn str_is_ascii_kieu_str_bool() {
    // **VD-270 (str-is-ascii)**: `(str) -> bool`; tham số đầu `str` để R5c nhận sugar
    // `s.is_ascii()` (không nằm trong bảng `module_sugar`, chỉ dùng đúng module `str`).
    let f = sig_of("str.is_ascii").expect("str.is_ascii");
    assert_eq!(f.params, &[Ty::Str], "`str.is_ascii`: tham số");
    assert_eq!(f.ret, Ty::Bool, "`str.is_ascii`: kiểu trả");
    assert_eq!(f.ret.sang_type(), Type::Bool, "`str.is_ascii`: hạ kiểu");
    assert_eq!(f.so_doi(), 1, "`str.is_ascii`: arity");
    assert_eq!(vis_types::stdlib::module_sugar("str", "is_ascii"), None);
}

#[test]
fn str_is_blank_kieu_str_bool() {
    // **VD-270 (is-blank)**: `(str) -> bool`; tham số đầu `str` để R5c nhận sugar
    // `s.is_blank()` (không nằm trong bảng `module_sugar`, chỉ dùng đúng module `str`).
    let f = sig_of("str.is_blank").expect("str.is_blank");
    assert_eq!(f.params, &[Ty::Str], "`str.is_blank`: tham số");
    assert_eq!(f.ret, Ty::Bool, "`str.is_blank`: kiểu trả");
    assert_eq!(f.ret.sang_type(), Type::Bool, "`str.is_blank`: hạ kiểu");
    assert_eq!(f.so_doi(), 1, "`str.is_blank`: arity");
    assert_eq!(vis_types::stdlib::module_sugar("str", "is_blank"), None);
}

#[test]
fn panic_la_builtin_tra_never() {
    // **VD-267**: `panic` là builtin **trần** (không `std.`) với kiểu trả `never`; chữ ký nằm ở
    // `BUILTIN_FUNCS`, không trộn vào `FUNCS` (freeze API `std.*`).
    assert!(sig_of("panic").is_none(), "`panic` không phải hàm `std.*`");
    let f = sig_builtin("panic").expect("phải có chữ ký builtin `panic`");
    assert_eq!(f.params, &[Ty::Str]);
    assert_eq!(f.ret, Ty::Never);
    assert_eq!(f.ret.sang_type(), Type::Never);
    assert_eq!(f.ret.ten(), "never");
    assert_eq!(f.so_doi(), 1);
}

#[test]
fn strfind_tra_int_option_va_int() {
    // **VD-270 (strfind)**: `find`/`rfind` trả `int?` = `Option[int]` **thật**, `count` trả `int`;
    // cùng tham số `(str, str)` nên sugar `s.find(...)`/`s.count(...)` thoả R5c.
    for ten in ["str.find", "str.rfind"] {
        let f = sig_of(ten).unwrap_or_else(|| panic!("thiếu chữ ký `{ten}`"));
        assert_eq!(f.params, &[Ty::Str, Ty::Str], "`{ten}`: tham số");
        assert_eq!(f.ret, Ty::OptInt, "`{ten}`: kiểu trả");
        assert_eq!(
            f.ret.sang_type(),
            Type::Opt(Box::new(Type::Int)),
            "`{ten}`: phải hạ thành `Option[int]`"
        );
        assert_eq!(f.ret.ten(), "int?", "`{ten}`: tên hiển thị");
    }
    let c = sig_of("str.count").expect("str.count");
    assert_eq!(c.params, &[Ty::Str, Ty::Str]);
    assert_eq!(c.ret, Ty::Int);
    assert_eq!(c.ret.sang_type(), Type::Int);
}

#[test]
fn strstrip_tra_str_option() {
    // **VD-270 (strstrip)**: `strip_prefix`/`strip_suffix` trả `str?` = `Option[str]` **thật**
    // (`Ty::OptStr`), tham số `(str, str)` nên sugar `s.strip_prefix(…)` thoả R5c.
    for ten in ["str.strip_prefix", "str.strip_suffix"] {
        let f = sig_of(ten).unwrap_or_else(|| panic!("thiếu chữ ký `{ten}`"));
        assert_eq!(f.params, &[Ty::Str, Ty::Str], "`{ten}`: tham số");
        assert_eq!(f.ret, Ty::OptStr, "`{ten}`: kiểu trả");
        assert_eq!(
            f.ret.sang_type(),
            Type::Opt(Box::new(Type::Str)),
            "`{ten}`: phải hạ thành `Option[str]`"
        );
        assert_eq!(f.ret.ten(), "str?", "`{ten}`: tên hiển thị");
        assert_eq!(f.so_doi(), 2, "`{ten}`: số đối số");
    }
}

#[test]
fn replace_first_kieu_str_sang_str() {
    // **VD-270 (replace-first)**: `(str, str, str) -> str`; cùng tham số đầu `str` nên sugar
    // R5c `s.replace_first(…)` chạy không cần `use`, và kết quả là `str` (không phải `str?`).
    let f = sig_of("str.replace_first").expect("str.replace_first");
    assert_eq!(f.params, &[Ty::Str, Ty::Str, Ty::Str], "tham số");
    assert_eq!(f.ret, Ty::Str, "kiểu trả");
    assert_eq!(f.ret.sang_type(), Type::Str, "hạ kiểu trả");
    assert_eq!(f.so_doi(), 3, "số đối số");
}

#[test]
fn replace_last_kieu_str_sang_str() {
    // **VD-270 (replace-last)**: `(str, str, str) -> str`; cùng tham số đầu `str` nên sugar
    // R5c `s.replace_last(…)` chạy không cần `use`, và kết quả là `str` (không phải `str?`).
    let f = sig_of("str.replace_last").expect("str.replace_last");
    assert_eq!(f.params, &[Ty::Str, Ty::Str, Ty::Str], "tham số");
    assert_eq!(f.ret, Ty::Str, "kiểu trả");
    assert_eq!(f.ret.sang_type(), Type::Str, "hạ kiểu trả");
    assert_eq!(f.so_doi(), 3, "số đối số");
}

#[test]
fn replace_n_kieu_str_sang_str() {
    // **VD-270 (replace-n)**: `(str, str, str, int) -> str`; cùng tham số đầu `str` nên sugar
    // R5c `s.replace_n(…)` chạy không cần `use`, và kết quả là `str` (không phải `str?`).
    let f = sig_of("str.replace_n").expect("str.replace_n");
    assert_eq!(f.params, &[Ty::Str, Ty::Str, Ty::Str, Ty::Int], "tham số");
    assert_eq!(f.ret, Ty::Str, "kiểu trả");
    assert_eq!(f.ret.sang_type(), Type::Str, "hạ kiểu trả");
    assert_eq!(f.so_doi(), 4, "số đối số");
}

#[test]
fn char_at_kieu_str_int_sang_str_option() {
    // **VD-270 (char-at)**: `(str, int) -> str?`; cùng tham số đầu `str` nên sugar
    // R5c `s.char_at(i)` chạy không cần `use`, và kết quả là `Option[str]` thật.
    let f = sig_of("str.char_at").expect("str.char_at");
    assert_eq!(f.params, &[Ty::Str, Ty::Int], "tham số");
    assert_eq!(f.ret, Ty::OptStr, "kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::Opt(Box::new(Type::Str)),
        "hạ kiểu trả"
    );
    assert_eq!(f.so_doi(), 2, "số đối số");
}

#[test]
fn title_kieu_str_sang_str() {
    // **VD-270 (str-title)**: `(str) -> str`; cùng tham số đầu `str` nên sugar R5c `s.title()`
    // chạy không cần `use`, và kết quả là `str` (không phải `str?`).
    let f = sig_of("str.title").expect("str.title");
    assert_eq!(f.params, &[Ty::Str], "tham số");
    assert_eq!(f.ret, Ty::Str, "kiểu trả");
    assert_eq!(f.ret.sang_type(), Type::Str, "hạ kiểu trả");
    assert_eq!(f.so_doi(), 1, "số đối số");
    // Không cần bảng `module_sugar`: module mặc định của người nhận `str` đã là `str`.
    assert_eq!(vis_types::stdlib::module_sugar("str", "title"), None);
}

#[test]
fn indexof_tra_int_option() {
    // **VD-270 (indexof)**: `index_of`/`last_index_of` là `(list[T], T) -> int?`; `T` buộc từ
    // `xs` nên sugar `xs.index_of(v)` thoả R5c và `value` cùng kiểu phần tử.
    // **VD-270 (binary-search)**: `binary_search` dùng **đúng** hình dạng đó (biên dưới).
    for ten in ["list.index_of", "list.last_index_of", "list.binary_search"] {
        let f = sig_of(ten).unwrap_or_else(|| panic!("thiếu chữ ký `{ten}`"));
        assert_eq!(
            f.params,
            &[Ty::List(&Ty::Var(0)), Ty::Var(0)],
            "`{ten}`: tham số"
        );
        assert_eq!(f.ret, Ty::OptInt, "`{ten}`: kiểu trả");
        assert_eq!(
            f.ret.sang_type(),
            Type::Opt(Box::new(Type::Int)),
            "`{ten}`: phải hạ thành `Option[int]`"
        );
        assert_eq!(f.ret.ten(), "int?", "`{ten}`: tên hiển thị");
    }
}

#[test]
fn list_contains_tra_bool_va_sugar() {
    // **VD-270 (list-contains)**: `(list[T], T) -> bool`; `T` buộc từ `xs` nên `value` cùng
    // kiểu phần tử và sugar `xs.contains(v)` thoả R5c; khác `index_of` ở kiểu trả `bool`.
    let f = sig_of("list.contains").expect("list.contains");
    assert_eq!(f.params, &[Ty::List(&Ty::Var(0)), Ty::Var(0)], "tham số");
    assert_eq!(f.ret, Ty::Bool, "kiểu trả");
    assert_eq!(f.ret.sang_type(), Type::Bool, "hạ kiểu trả");
    assert_eq!(f.ret.ten(), "bool", "tên hiển thị");
    assert_eq!(f.so_doi(), 2, "số đối số");
}

#[test]
fn list_sum_product_chi_list_int_tra_int() {
    // **VD-270 (list-sum-product)**: cả hai **chỉ** `list[int]` (không `list[T]`), trả `int`;
    // sugar `xs.product()` đi R5c; `xs.sum()` vẫn là phương thức lõi (cùng ngữ nghĩa cho
    // `list[int]`, còn `list[float]` do lõi xử lý).
    for ten in ["list.sum", "list.product"] {
        let f = sig_of(ten).unwrap_or_else(|| panic!("thiếu chữ ký `{ten}`"));
        assert_eq!(f.params, &[Ty::List(&Ty::Int)], "`{ten}`: tham số");
        assert_eq!(f.ret, Ty::Int, "`{ten}`: kiểu trả");
        assert_eq!(f.ret.sang_type(), Type::Int, "`{ten}`: phải hạ thành `int`");
        assert_eq!(f.ret.ten(), "int", "`{ten}`: tên hiển thị");
        assert_eq!(f.so_doi(), 1, "`{ten}`: số đối số");
    }
}

#[test]
fn list_mean_chi_list_float_tra_opt_float() {
    // **VD-270 (list-mean)**: **chỉ** `list[float]` (không `list[T]`, không `list[int]`), trả
    // `float?` = `Option[float]`; `mean` không phải phương thức lõi nên sugar `xs.mean()` R5c.
    let f = sig_of("list.mean").expect("list.mean");
    assert_eq!(f.params, &[Ty::List(&Ty::Float)], "`list.mean`: tham số");
    assert_eq!(f.ret, Ty::OptFloat, "`list.mean`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::Opt(Box::new(Type::Float)),
        "`list.mean`: phải hạ thành `Option[float]`"
    );
    assert_eq!(f.ret.ten(), "float?", "`list.mean`: tên hiển thị");
    assert_eq!(f.so_doi(), 1, "`list.mean`: số đối số");
}

#[test]
fn list_median_chi_list_float_tra_opt_float() {
    // **VD-270 (list-median)**: **chỉ** `list[float]` (không `list[T]`, không `list[int]`), trả
    // `float?` = `Option[float]`; `median` không phải phương thức lõi nên sugar `xs.median()` R5c.
    let f = sig_of("list.median").expect("list.median");
    assert_eq!(f.params, &[Ty::List(&Ty::Float)], "`list.median`: tham số");
    assert_eq!(f.ret, Ty::OptFloat, "`list.median`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::Opt(Box::new(Type::Float)),
        "`list.median`: phải hạ thành `Option[float]`"
    );
    assert_eq!(f.ret.ten(), "float?", "`list.median`: tên hiển thị");
    assert_eq!(f.so_doi(), 1, "`list.median`: số đối số");
}

#[test]
fn list_sum_float_chi_list_float_tra_float() {
    // **VD-270 (list-sum-float)**: **chỉ** `list[float]`, trả `float` trần (không `Option`);
    // `sum_float` không phải phương thức lõi nên sugar `xs.sum_float()` R5c.
    let f = sig_of("list.sum_float").expect("list.sum_float");
    assert_eq!(
        f.params,
        &[Ty::List(&Ty::Float)],
        "`list.sum_float`: tham số"
    );
    assert_eq!(f.ret, Ty::Float, "`list.sum_float`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::Float,
        "`list.sum_float`: phải hạ thành `float`"
    );
    assert_eq!(f.ret.ten(), "float", "`list.sum_float`: tên hiển thị");
    assert_eq!(f.so_doi(), 1, "`list.sum_float`: số đối số");
}

#[test]
fn list_minmax_float_chi_list_float_tra_opt_float() {
    // **VD-270 (list-minmax-float)**: **chỉ** `list[float]` (không `list[T]`, không
    // `list[int]`), trả `float?` = `Option[float]`; `min_float`/`max_float` không phải phương
    // thức lõi nên sugar `xs.min_float()`/`xs.max_float()` R5c.
    for name in ["list.min_float", "list.max_float"] {
        let f = sig_of(name).unwrap_or_else(|| panic!("{name}"));
        assert_eq!(f.params, &[Ty::List(&Ty::Float)], "`{name}`: tham số");
        assert_eq!(f.ret, Ty::OptFloat, "`{name}`: kiểu trả");
        assert_eq!(
            f.ret.sang_type(),
            Type::Opt(Box::new(Type::Float)),
            "`{name}`: phải hạ thành `Option[float]`"
        );
        assert_eq!(f.ret.ten(), "float?", "`{name}`: tên hiển thị");
        assert_eq!(f.so_doi(), 1, "`{name}`: số đối số");
    }
}

#[test]
fn sang_type_dung_ten_bien_kieu() {
    assert_eq!(
        Ty::Var(0).sang_type(),
        Type::TypeRef {
            name: "T".into(),
            args: Vec::new()
        }
    );
    assert_eq!(
        Ty::List(&Ty::Var(1)).sang_type(),
        Type::List(Box::new(Type::TypeRef {
            name: "K".into(),
            args: Vec::new()
        }))
    );
    // `list[list[T]]` — promotion trong `const FUNCS` đã chứng minh; đây khoá **hình dạng**.
    assert_eq!(
        Ty::List(&Ty::List(&Ty::Var(0))).sang_type(),
        Type::List(Box::new(Type::List(Box::new(Type::TypeRef {
            name: "T".into(),
            args: Vec::new()
        }))))
    );
    assert_eq!(
        Ty::Map(&Ty::Var(1), &Ty::Var(2)).sang_type(),
        Type::Map(
            Box::new(Type::TypeRef {
                name: "K".into(),
                args: Vec::new()
            }),
            Box::new(Type::TypeRef {
                name: "V".into(),
                args: Vec::new()
            })
        )
    );
}

/// 16/38 chữ ký đổi (VD-99 (4)): `std.list` 8 + `std.map` 5 + `std.json` 3.
#[test]
fn muoi_sau_chu_ky_generic_dung_hinh_dang() {
    let t = Ty::Var(0);
    let list_t = Ty::List(&Ty::Var(0));
    let k = Ty::Var(1);
    let v = Ty::Var(2);
    let map_kv = Ty::Map(&Ty::Var(1), &Ty::Var(2));
    let ca: [(&str, &[Ty], Ty); 16] = [
        ("list.sort", &[list_t], list_t),
        ("list.sort_by", &[list_t, Ty::FnKhoa], list_t),
        ("list.reverse", &[list_t], list_t),
        ("list.unique", &[list_t], list_t),
        ("list.concat", &[list_t, list_t], list_t),
        (
            "list.chunk",
            &[list_t, Ty::Int],
            Ty::List(&Ty::List(&Ty::Var(0))),
        ),
        ("list.take", &[list_t, Ty::Int], list_t),
        ("list.drop", &[list_t, Ty::Int], list_t),
        ("map.keys", &[map_kv], Ty::List(&Ty::Var(1))),
        ("map.values", &[map_kv], Ty::List(&Ty::Var(2))),
        ("map.merge", &[map_kv, map_kv], map_kv),
        ("map.get_or", &[map_kv, Ty::Str, v], v),
        ("map.remove", &[map_kv, Ty::Str], map_kv),
        ("json.encode", &[t], Ty::Str),
        ("json.encode_pretty", &[t], Ty::Str),
        ("json.decode", &[Ty::Str], Ty::ResultJsonErr),
    ];
    for (ten, params, ret) in ca {
        let f = sig_of(ten).unwrap_or_else(|| panic!("thiếu chữ ký `{ten}`"));
        assert_eq!(f.params, params, "`{ten}`: tham số sai hình dạng");
        assert_eq!(f.ret, ret, "`{ten}`: kiểu trả sai hình dạng");
    }
    let _ = (k,);
}

/// `list.sort_by` **giữ** `Ty::FnKhoa` (ngoại lệ **có ghi** duy nhất, VD-94 (3)/VD-99 (5)):
/// lambda chưa chú thích là `{error}`; `Type` không có union `int|str` (ST-56) ⇒ tầng chạy kiểm.
#[test]
fn sort_by_giu_ngoai_le_fn_khoa() {
    let f = sig_of("list.sort_by").expect("list.sort_by");
    assert!(matches!(f.params[1], Ty::FnKhoa), "phải giữ `FnKhoa`");
    assert!(
        matches!(f.params[0], Ty::List(_)),
        "tham số 1 phải là `list[T]`"
    );
}

/// **VD-270 (sqlite-indexes)**: chữ ký `(str, str) -> Result[list[str], Error]` — tái dùng
/// `Ty::ResultListStrStr` (không thêm biến thể `Ty`), hạ ra `Result` với lỗi là bản ghi `Error`.
#[test]
fn sqlite_indexes_kieu_str_str_va_result_list_str() {
    let f = sig_of("sqlite.indexes").expect("sqlite.indexes");
    assert_eq!(
        f.params,
        &[Ty::Str, Ty::Str],
        "`sqlite.indexes`: tham số (handle, table)"
    );
    assert_eq!(
        f.ret,
        Ty::ResultListStrStr,
        "`sqlite.indexes`: kiểu trả phải tái dùng `ResultListStrStr`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(
            Box::new(Type::List(Box::new(Type::Str))),
            Box::new(Ty::Error.sang_type())
        ),
        "`sqlite.indexes`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "Result[list[str], Error]");
}

/// **VD-270 (sqlite-schema-version)**: chữ ký `(str) -> Result[int, Error]` — tái dùng **đúng**
/// `Ty::ResultIntErr` (như `sqlite.changes`/`sqlite.last_insert_rowid`), không thêm biến thể.
#[test]
fn sqlite_schema_version_kieu_str_va_result_int() {
    let f = sig_of("sqlite.schema_version").expect("sqlite.schema_version");
    assert_eq!(
        f.params,
        &[Ty::Str],
        "`sqlite.schema_version`: tham số (handle)"
    );
    assert_eq!(
        f.ret,
        Ty::ResultIntErr,
        "`sqlite.schema_version`: kiểu trả phải là `ResultIntErr`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(Box::new(Type::Int), Box::new(Ty::Error.sang_type())),
        "`sqlite.schema_version`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "Result[int, Error]");
}

/// **VD-270 (sqlite-user-version)**: chữ ký `(str) -> Result[int, Error]` — tái dùng **đúng**
/// `Ty::ResultIntErr` (như `sqlite.changes`/`sqlite.last_insert_rowid`/`sqlite.schema_version`),
/// không thêm biến thể.
#[test]
fn sqlite_user_version_kieu_str_va_result_int() {
    let f = sig_of("sqlite.user_version").expect("sqlite.user_version");
    assert_eq!(
        f.params,
        &[Ty::Str],
        "`sqlite.user_version`: tham số (handle)"
    );
    assert_eq!(
        f.ret,
        Ty::ResultIntErr,
        "`sqlite.user_version`: kiểu trả phải là `ResultIntErr`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(Box::new(Type::Int), Box::new(Ty::Error.sang_type())),
        "`sqlite.user_version`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "Result[int, Error]");
}

/// **VD-270 (sqlite-schema-sql)**: chữ ký `(str, str) -> Result[str?, Error]` — biến thể **riêng**
/// `Ty::ResultOptStrErr` (không tái dùng `ResultStrStr`/`OptStr`, vì phải giữ **cả** lớp
/// `Result` **và** lớp `Option`), hạ ra `Result[Option[str], Error]`.
#[test]
fn sqlite_schema_sql_kieu_str_str_va_result_opt_str() {
    let f = sig_of("sqlite.schema_sql").expect("sqlite.schema_sql");
    assert_eq!(
        f.params,
        &[Ty::Str, Ty::Str],
        "`sqlite.schema_sql`: tham số (handle, name)"
    );
    assert_eq!(
        f.ret,
        Ty::ResultOptStrErr,
        "`sqlite.schema_sql`: kiểu trả phải là biến thể riêng `ResultOptStrErr`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(
            Box::new(Type::Opt(Box::new(Type::Str))),
            Box::new(Ty::Error.sang_type())
        ),
        "`sqlite.schema_sql`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "Result[str?, Error]");
}

/// **VD-270 (sqlite-index-columns)**: chữ ký `(str, str) -> Result[list[str], Error]` — tái
/// dùng `Ty::ResultListStrStr` (không thêm biến thể `Ty`), hạ ra `Result` với lỗi là bản ghi
/// `Error`.
#[test]
fn sqlite_index_columns_kieu_str_str_va_result_list_str() {
    let f = sig_of("sqlite.index_columns").expect("sqlite.index_columns");
    assert_eq!(
        f.params,
        &[Ty::Str, Ty::Str],
        "`sqlite.index_columns`: tham số (handle, index)"
    );
    assert_eq!(
        f.ret,
        Ty::ResultListStrStr,
        "`sqlite.index_columns`: kiểu trả phải tái dùng `ResultListStrStr`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(
            Box::new(Type::List(Box::new(Type::Str))),
            Box::new(Ty::Error.sang_type())
        ),
        "`sqlite.index_columns`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "Result[list[str], Error]");
}

/// **VD-270 (sqlite-primary-keys)**: chữ ký `(str, str) -> Result[list[str], Error]` — tái dùng
/// `Ty::ResultListStrStr` (không thêm biến thể `Ty`), hạ ra `Result` với lỗi là bản ghi `Error`.
#[test]
fn sqlite_primary_keys_kieu_str_str_va_result_list_str() {
    let f = sig_of("sqlite.primary_keys").expect("sqlite.primary_keys");
    assert_eq!(
        f.params,
        &[Ty::Str, Ty::Str],
        "`sqlite.primary_keys`: tham số (handle, table)"
    );
    assert_eq!(
        f.ret,
        Ty::ResultListStrStr,
        "`sqlite.primary_keys`: kiểu trả phải tái dùng `ResultListStrStr`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(
            Box::new(Type::List(Box::new(Type::Str))),
            Box::new(Ty::Error.sang_type())
        ),
        "`sqlite.primary_keys`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "Result[list[str], Error]");
}

/// **VD-270 (sqlite-table-exists)**: chữ ký `(str, str) -> Result[bool, Error]` — tái dùng
/// `Ty::ResultBoolErr` (cùng biến thể của `sqlite.in_transaction`, không thêm biến thể `Ty`),
/// hạ ra `Result` với lỗi là bản ghi `Error`.
#[test]
fn sqlite_table_exists_kieu_str_str_va_result_bool() {
    let f = sig_of("sqlite.table_exists").expect("sqlite.table_exists");
    assert_eq!(
        f.params,
        &[Ty::Str, Ty::Str],
        "`sqlite.table_exists`: tham số (handle, name)"
    );
    assert_eq!(
        f.ret,
        Ty::ResultBoolErr,
        "`sqlite.table_exists`: kiểu trả phải tái dùng `ResultBoolErr`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(Box::new(Type::Bool), Box::new(Ty::Error.sang_type())),
        "`sqlite.table_exists`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "Result[bool, Error]");
}

/// **VD-270 (sqlite-index-exists)**: chữ ký `(str, str) -> Result[bool, Error]` — tái dùng
/// `Ty::ResultBoolErr` (cùng biến thể của `sqlite.in_transaction`/`sqlite.table_exists`, không
/// thêm biến thể `Ty`), hạ ra `Result` với lỗi là bản ghi `Error`.
#[test]
fn sqlite_index_exists_kieu_str_str_va_result_bool() {
    let f = sig_of("sqlite.index_exists").expect("sqlite.index_exists");
    assert_eq!(
        f.params,
        &[Ty::Str, Ty::Str],
        "`sqlite.index_exists`: tham số (handle, name)"
    );
    assert_eq!(
        f.ret,
        Ty::ResultBoolErr,
        "`sqlite.index_exists`: kiểu trả phải tái dùng `ResultBoolErr`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(Box::new(Type::Bool), Box::new(Ty::Error.sang_type())),
        "`sqlite.index_exists`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "Result[bool, Error]");
}
