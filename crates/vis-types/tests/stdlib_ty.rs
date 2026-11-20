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
    // **SỬA KÈM LÝ DO (VD-270 sqlite-schema-sql, 2026-11-21)**: +**1** hàm `sqlite.schema_sql`
    // (`(str, str) -> Result[str?, Error]`; tra văn bản SQL khai báo của đối tượng schema chính
    // theo tên bind, thiếu/`sql` NULL ⇒ `Ok(None)`) ⇒ **120** hàm; không xoá/đổi hàm cũ nào.
    assert_eq!(FUNCS.len(), 120, "bảng chữ ký phải đủ 120 hàm");
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
