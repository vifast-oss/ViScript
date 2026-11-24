// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — `Ty`: biến kiểu trong chữ ký `std.*` + `sang_type` + `ten_bien`.
//! Tách nguyên văn khỏi `stdlib.rs` (không đổi API/hành vi).

use crate::ty::Type;

/// Một chỗ trong chữ ký `std.*` — **const-friendly** (`Copy` + `&'static Ty`, dùng được trực tiếp
/// trong `const FUNCS` nhờ promotion), **không** còn "nhận mọi kiểu" (VD-99 (1)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ty {
    Int,
    Str,
    Bool,
    /// **VD-270 (list-mean)** — `float` vô hướng: phần tử của tham số `list[float]` cho
    /// `std.list.mean`. Trước VD này `Ty` chưa có biến thể `float` vì chưa hàm `std.*` nào
    /// **nhận** float (chỉ có `float?` ở vị trí **trả**).
    Float,
    ListStr,
    /// Khoá của `list.sort_by`: `fn(T) -> int` **hoặc** `fn(T) -> str` (ST-56) — **ngoại lệ có ghi**
    /// duy nhất (VD-94 (3)/VD-99 (5)): lambda chưa chú thích là `{error}`, `Type` không có union.
    FnKhoa,
    /// `Result[str, str]` — ST-33 (`std.fs.read`, `std.fs.list` lỗi trả `str`).
    ResultStrStr,
    /// `Result[(), str]` — ST-33 (`std.fs.write`).
    ResultUnitStr,
    /// `Result[list[str], str]` — ST-33 (`std.fs.list`).
    ResultListStrStr,
    /// `Result[map[str, {error}], Error]` — ST-109 (`http.head`): map có `status` (int) + `headers`
    /// (map), nên value để `{error}` (động, không ép sai kiểu). **VD-222**: lỗi → bản ghi dựng sẵn `Error`.
    ResultHead,
    /// Bản ghi dựng sẵn `Error` (VD-222 điểm 2–3) — `RecId(0)` trong mọi bảng.
    Error,
    /// Enum dựng sẵn `Json` (VD-222 điểm 4) — `EnumId(0)` trong mọi bảng.
    Json,
    /// `Result[Json, Error]` — chữ ký `std.json.decode` (VD-222 điểm 4; VD-187 dòng buộc).
    ResultJsonErr,
    /// **VD-241 (Gate 1)** — enum `SqlValue` của module `std.sqlite` (`EnumId(1)`); **thuộc module**,
    /// không phải prelude toàn cục (hạ gate).
    SqlValue,
    /// **VD-241 (Gate 1)** — bản ghi `RowSet` của module `std.sqlite` (`RecId(1)`): `columns`
    /// + `rows`.
    RowSet,
    /// **VD-241 (Gate 1)** — `Result[int, Error]` (`std.sqlite.execute`).
    ResultIntErr,
    /// **VD-270 (sqlite-journal-mode)** — `Result[str, Error]` (`std.sqlite.journal_mode`).
    /// `ResultStrStr` cũng hạ ra **cùng** kiểu (`str` + bản ghi `Error` từ VD-222), nhưng tên cũ
    /// thuộc thời `Result[str, str]` (ST-33); biến thể này nêu **đúng** lớp lỗi `Error` theo mẫu
    /// `ResultIntErr`, không ép kiểu gần đúng.
    ResultStrErr,
    /// **VD-241 (Gate 1)** — `Result[bool, Error]` (`std.sqlite.in_transaction`,
    /// `std.sqlite.table_exists`, `std.sqlite.index_exists` — VD-270).
    ResultBoolErr,
    /// **VD-241 (Gate 1)** — `Result[RowSet, Error]` (`std.sqlite.query`).
    ResultRowSetErr,
    /// **VD-270 (sqlite-database-list)** — `list[RowSet]`: mỗi phần tử là **một** `RowSet` của
    /// một hàng `PRAGMA database_list`. `Ty` chưa có `list[RowSet]` cụ thể (chỉ `List(&Ty)` với
    /// `ten()` chung là `"list"`), nên giữ biến riêng để chữ ký/tên hiển thị đúng lớp phần tử.
    ListRowSet,
    /// **VD-270 (sqlite-database-list)** — `Result[list[RowSet], Error]`
    /// (`std.sqlite.database_list`). Theo mẫu [`Ty::ResultRowSetErr`]: giữ đúng cả lớp `list` lẫn
    /// lớp `Result`.
    ResultListRowSetErr,
    /// **VD-267** — `never`: kiểu của biểu thức **không bao giờ cho giá trị** (`panic(…)`).
    /// Chỉ dùng cho chữ ký builtin trần ([`BUILTIN_FUNCS`]); `std.*` chưa có hàm nào trả `never`.
    Never,
    /// **VD-270 (strfind)** — `int?` = `Option[int]`: kiểu trả của `std.str.find`/`rfind`
    /// (vị trí **scalar** Unicode, không khớp ⇒ `None`).
    OptInt,
    /// **VD-270 (fsbytes)** — `Result[list[int], Error]`: kiểu trả của `std.fs.read_bytes`
    /// (mỗi phần tử là một byte `0..=255`).
    ResultListIntErr,
    /// **VD-270 (strstrip)** — `str?` = `Option[str]`: kiểu trả của `std.str.strip_prefix`/
    /// `strip_suffix` (phần còn lại khi khớp; không khớp ⇒ `None`). `Ty` không có biến thể
    /// `Opt` tổng quát, nên giữ biến riêng theo đúng mẫu `OptInt`.
    OptStr,
    /// **VD-270 (parse-float)** — `float?` = `Option[float]`: kiểu trả của `std.str.parse_float`
    /// (chuỗi sai cú pháp/không hữu hạn/tràn ⇒ `None`). Cùng lý do `OptInt`/`OptStr`: `Ty`
    /// không có biến thể `Opt` tổng quát, nên giữ biến riêng theo đúng mẫu.
    OptFloat,
    /// **VD-270 (split-once)** — `list[str]?` = `Option[list[str]]`: kiểu trả của
    /// `std.str.split_once` (tách tại **lần xuất hiện đầu** của `sep` ⇒ `Some` đúng **2** phần;
    /// không khớp ⇒ `None`). Cùng lý do `OptStr`/`OptFloat`: `Ty` không có biến thể `Opt` tổng
    /// quát, nên giữ biến riêng; **không** tái dùng `ListStr` (mất lớp `Option`) hay `OptStr`
    /// (Option của `str`, không phải danh sách).
    OptListStr,
    /// **VD-270 (sqlite-schema-sql)** — `Result[str?, Error]` = `Result[Option[str], Error]`:
    /// kiểu trả của `std.sqlite.schema_sql` (đối tượng không tồn tại hoặc `sql` NULL ⇒ `Ok(None)`).
    /// Không tái dùng `ResultStrStr` (đó là `Result[str, Error]`, không có lớp `Option`) và
    /// `OptStr` (đó là `str?` trần, không có `Result`): `Ty` không có biến thể `Result`/`Opt`
    /// tổng quát, nên giữ một biến **chính xác** theo mẫu `OptStr`/`ResultIntErr` thay vì ép
    /// kiểu gần đúng làm mất lớp `Option` hoặc lớp lỗi.
    ResultOptStrErr,
    /// **Biến kiểu** (VD-99 (1)): `0=T`, `1=K`, `2=V` — **thay** `Ty::Any` (đã xoá).
    Var(u8),
    /// `list[T]` — phần tử là biến kiểu (hoặc kiểu lồng, vd `list[list[T]]`).
    List(&'static Ty),
    /// `map[K, V]`.
    Map(&'static Ty, &'static Ty),
}

impl Ty {
    /// Kiểu tĩnh **tương ứng** — **tổng** (không `Option`; thay `kieu()` cũ). `Var(i)` hạ thành
    /// `Type::TypeRef` tên `T`/`K`/`V` (tái dùng `TypeRef` — `hop_nhat` đã coi tên trong `gens` là
    /// biến kiểu; **không** thêm `Type::Param`).
    pub fn sang_type(self) -> Type {
        match self {
            Ty::Int => Type::Int,
            Ty::Str => Type::Str,
            Ty::Bool => Type::Bool,
            // **VD-270 (list-mean)** — phần tử `list[float]` là `float` thật.
            Ty::Float => Type::Float,
            Ty::ListStr => Type::List(Box::new(Type::Str)),
            // **VD-222 (điểm 2–3)**: mọi lỗi chuẩn **bản ghi dựng sẵn `Error`** (`RecId(0)` —
            // dựng sẵn chốn đẩy vào bảng trước item đầu tiên, id `0` ổn định).
            Ty::Error => Type::Record(crate::ty::RecId(0)),
            Ty::Json => Type::Enum(crate::ty::EnumId(0)),
            // **VD-241 (Gate 1)** — `SqlValue` `EnumId(1)` / `RowSet` `RecId(1)` (thứ tự dựng ở
            // `lower`: Json=0, SqlValue=1; Error=0, RowSet=1 — id ổn định).
            Ty::SqlValue => Type::Enum(crate::ty::EnumId(1)),
            Ty::RowSet => Type::Record(crate::ty::RecId(1)),
            Ty::ResultIntErr => {
                Type::Result(Box::new(Type::Int), Box::new(Self::Error.sang_type()))
            }
            // **VD-270 (sqlite-journal-mode)** — `Result[str, Error]`: `str` + bản ghi `Error`.
            Ty::ResultStrErr => {
                Type::Result(Box::new(Type::Str), Box::new(Self::Error.sang_type()))
            }
            Ty::ResultBoolErr => {
                Type::Result(Box::new(Type::Bool), Box::new(Self::Error.sang_type()))
            }
            Ty::ResultRowSetErr => Type::Result(
                Box::new(Ty::RowSet.sang_type()),
                Box::new(Self::Error.sang_type()),
            ),
            // **VD-270 (sqlite-database-list)** — `list[RowSet]`: `RowSet` là bản ghi dựng sẵn
            // `RecId(1)`, giữ đúng lớp danh sách.
            Ty::ListRowSet => Type::List(Box::new(Type::Record(crate::ty::RecId(1)))),
            // **VD-270 (sqlite-database-list)** — `Result[list[RowSet], Error]`.
            Ty::ResultListRowSetErr => Type::Result(
                Box::new(Ty::ListRowSet.sang_type()),
                Box::new(Self::Error.sang_type()),
            ),
            // **VD-267** — `panic(…)` phân kỳ: kiểu `never` (khớp mọi kiểu đích ở `coerce`).
            Ty::Never => Type::Never,
            // **VD-270 (strfind)** — `int?` là `Option[int]` thật (không phải `{error}`).
            Ty::OptInt => Type::Opt(Box::new(Type::Int)),
            // **VD-270 (fsbytes)** — `list[int]` (mỗi phần tử là byte `0..=255`).
            Ty::ResultListIntErr => Type::Result(
                Box::new(Type::List(Box::new(Type::Int))),
                Box::new(Self::Error.sang_type()),
            ),
            // **VD-270 (strstrip)** — `str?` là `Option[str]` thật.
            Ty::OptStr => Type::Opt(Box::new(Type::Str)),
            // **VD-270 (parse-float)** — `float?` là `Option[float]` thật.
            Ty::OptFloat => Type::Opt(Box::new(Type::Float)),
            // **VD-270 (split-once)** — `list[str]?` là `Option[list[str]]` thật.
            Ty::OptListStr => Type::Opt(Box::new(Type::List(Box::new(Type::Str)))),
            // **VD-270 (sqlite-schema-sql)** — `Result[Option[str], Error]`: giữ đúng cả hai lớp.
            Ty::ResultOptStrErr => Type::Result(
                Box::new(Type::Opt(Box::new(Type::Str))),
                Box::new(Self::Error.sang_type()),
            ),
            Ty::ResultJsonErr => Type::Result(
                Box::new(Type::Enum(crate::ty::EnumId(0))),
                Box::new(Type::Record(crate::ty::RecId(0))),
            ),
            Ty::ResultStrStr => {
                Type::Result(Box::new(Type::Str), Box::new(Self::Error.sang_type()))
            }
            Ty::ResultUnitStr => {
                Type::Result(Box::new(Type::Unit), Box::new(Self::Error.sang_type()))
            }
            Ty::ResultListStrStr => Type::Result(
                Box::new(Type::List(Box::new(Type::Str))),
                Box::new(Self::Error.sang_type()),
            ),
            Ty::ResultHead => Type::Result(
                Box::new(Type::Map(Box::new(Type::Str), Box::new(Type::Error))),
                Box::new(Self::Error.sang_type()),
            ),
            Ty::Var(i) => Type::TypeRef {
                name: ten_bien(i).to_string(),
                args: Vec::new(),
            },
            Ty::List(t) => Type::List(Box::new(t.sang_type())),
            Ty::Map(k, v) => Type::Map(Box::new(k.sang_type()), Box::new(v.sang_type())),
            // Closure chưa suy kiểu hai chiều (VS0 nợ, xem JOURNAL ST-26) ⇒ vẫn **không** kiểm tĩnh
            // ở tyck; tầng chạy kiểm "gọi được" và "khoá trả `int` **hoặc** `str`" (VS0501, ST-56).
            Ty::FnKhoa => Type::Error,
        }
    }

    /// Tên hiển thị trong chẩn đoán.
    pub fn ten(self) -> &'static str {
        match self {
            Ty::Int => "int",
            Ty::Str => "str",
            Ty::Bool => "bool",
            // **VD-270 (list-mean)**
            Ty::Float => "float",
            Ty::ListStr => "list[str]",
            // ST-56: khoá nhận **cả** `str` ⇒ tên hiển thị nói đúng hợp đồng mới.
            Ty::FnKhoa => "fn(T) -> int | str",
            Ty::Error => "Error",
            Ty::Json => "Json",
            Ty::ResultJsonErr => "Result[Json, Error]",
            // **VD-241 (Gate 1)**
            Ty::SqlValue => "SqlValue",
            Ty::RowSet => "RowSet",
            Ty::ResultIntErr => "Result[int, Error]",
            // **VD-270 (sqlite-journal-mode)**
            Ty::ResultStrErr => "Result[str, Error]",
            Ty::ResultBoolErr => "Result[bool, Error]",
            Ty::ResultRowSetErr => "Result[RowSet, Error]",
            // **VD-270 (sqlite-database-list)**
            Ty::ListRowSet => "list[RowSet]",
            Ty::ResultListRowSetErr => "Result[list[RowSet], Error]",
            // **VD-267**
            Ty::Never => "never",
            // **VD-270 (strfind)**
            Ty::OptInt => "int?",
            // **VD-270 (fsbytes)**
            Ty::ResultListIntErr => "Result[list[int], Error]",
            // **VD-270 (strstrip)**
            Ty::OptStr => "str?",
            // **VD-270 (parse-float)**
            Ty::OptFloat => "float?",
            // **VD-270 (split-once)**
            Ty::OptListStr => "list[str]?",
            // **VD-270 (sqlite-schema-sql)**
            Ty::ResultOptStrErr => "Result[str?, Error]",
            // **VD-222**: lỗi → bản ghi dựng sẵn `Error` (trước 18/11: `str`).
            Ty::ResultStrStr => "Result[str, Error]",
            Ty::ResultUnitStr => "Result[(), Error]",
            Ty::ResultListStrStr => "Result[list[str], Error]",
            Ty::ResultHead => "Result[map[str, {error}], Error]",
            Ty::Var(_) => "biến kiểu",
            Ty::List(_) => "list",
            Ty::Map(..) => "map",
        }
    }
}

/// Tên biến kiểu theo chỉ số: `0=T`, `1=K`, `2=V` (VD-99 (1)); ngoài phạm vi ⇒ `T` (không panic).
pub fn ten_bien(i: u8) -> &'static str {
    match i {
        1 => "K",
        2 => "V",
        _ => "T",
    }
}
