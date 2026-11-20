// SPDX-License-Identifier: MIT OR Apache-2.0
//! **Bảng chữ ký `std.*`** (ST-26, VD-43): một nguồn sự thật cho `tyck` và (gián tiếp) `vis-interp`.
//!
//! Tên đầy đủ có tiền tố module (`str.split`, `list.sort`, `time.now_ms`) — nhờ vậy `std.str.join` và
//! `std.path.join` không đụng nhau, và không có tên trần nào bị nhập vào phạm vi người dùng.
//!
//! **ST-58 (VD-94/VD-99)**: chữ ký dùng **biến kiểu** (`Ty::Var`, `Ty::List`, `Ty::Map`) — `Ty::Any`
//! đã **xoá hẳn** (chính nó là lỗ "bỏ kiểm kiểu im lặng"). `tyck` **hợp nhất** đối số thật với biến
//! (trái→phải) rồi thay biến vào kiểu trả; biến **không gán** ⇒ `{error}` (giữ `json.decode`, VD-66).
//! **VD-222** (18/11/2026, uỷ quyền A0-proxy): mọi lỗi chuẩn (`Result[_, str]`) đổi thành
//! **bản ghi dựng sẵn `Error`** (`RecId(0)`; `message/source/span`; in `print` chỉ `{message}`);
//! `std.json.decode` → `Result[Json, Error]` — cây JSON là enum dựng sẵn `Json` (`EnumId(0)`:
//! `Null/Bool/Num/Str/Arr/Obj`), `json.decode` vẫn phải `match` (VD-66). `AnyError` = alias ban ghi Error (RecId(0)).

use crate::ty::Type;

/// Một chỗ trong chữ ký `std.*` — **const-friendly** (`Copy` + `&'static Ty`, dùng được trực tiếp
/// trong `const FUNCS` nhờ promotion), **không** còn "nhận mọi kiểu" (VD-99 (1)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ty {
    Int,
    Str,
    Bool,
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
    /// **VD-241 (Gate 1)** — `Result[bool, Error]` (`std.sqlite.in_transaction`).
    ResultBoolErr,
    /// **VD-241 (Gate 1)** — `Result[RowSet, Error]` (`std.sqlite.query`).
    ResultRowSetErr,
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
            Ty::ResultBoolErr => {
                Type::Result(Box::new(Type::Bool), Box::new(Self::Error.sang_type()))
            }
            Ty::ResultRowSetErr => Type::Result(
                Box::new(Ty::RowSet.sang_type()),
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
            Ty::ResultBoolErr => "Result[bool, Error]",
            Ty::ResultRowSetErr => "Result[RowSet, Error]",
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

/// Một hàm `std.*`: tên đầy đủ + tham số + kiểu trả.
#[derive(Debug, Clone, Copy)]
pub struct StdFn {
    /// Tên đầy đủ `"str.split"`.
    pub name: &'static str,
    pub params: &'static [Ty],
    pub ret: Ty,
}

impl StdFn {
    pub const fn so_doi(&self) -> usize {
        self.params.len()
    }
}

/// Module `std` được hỗ trợ ở ST-26 (mỗi phần tử là tên ngắn).
pub const MODULES: [&str; 10] = [
    "str", "list", "map", "path", "time", "json", "fs", "net", "http",
    // **VD-241 (Gate 1)** — `std.sqlite` MVP (prototype nội bộ, chưa freeze API).
    "sqlite",
];

/// `std.io` **chưa** làm — giữ tên để thông điệp rõ (ST-33: `std.fs` **đã có**).
pub const MODULES_HOAN: [&str; 1] = ["io"];

/// Toàn bộ hàm `std.*` của ST-26.
pub const FUNCS: [StdFn; 116] = [
    // ----- std.str -----
    StdFn {
        name: "str.split",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ListStr,
    },
    // **VD-270 (splitn)**: tối đa `n` phần, tách trái→phải; **phần cuối** giữ nguyên phần còn
    // lại (kể cả dấu phân cách); `n == 0` ⇒ `[]`, `n == 1` ⇒ `[s]`; `sep` rỗng theo nghĩa Rust
    // `str::splitn`; `n < 0` ⇒ trap `VS0501` ở tầng chạy. Sugar R5c `s.splitn(sep, n)`.
    StdFn {
        name: "str.splitn",
        params: &[Ty::Str, Ty::Str, Ty::Int],
        ret: Ty::ListStr,
    },
    // **VD-265**: tách dòng kiểu Rust `lines()` — `\n`, bỏ `\r` cuối dòng; tách khoảng trắng
    // Unicode. Cả hai nhận `s.` (sugar R5c) vì tham số đầu là `str`.
    StdFn {
        name: "str.lines",
        params: &[Ty::Str],
        ret: Ty::ListStr,
    },
    StdFn {
        name: "str.split_whitespace",
        params: &[Ty::Str],
        ret: Ty::ListStr,
    },
    StdFn {
        name: "str.join",
        params: &[Ty::ListStr, Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.trim",
        params: &[Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.upper",
        params: &[Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.lower",
        params: &[Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.starts_with",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::Bool,
    },
    StdFn {
        name: "str.ends_with",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::Bool,
    },
    StdFn {
        name: "str.contains",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::Bool,
    },
    // **VD-270 (strfind)**: vị trí tính theo **scalar** Unicode (cùng đơn vị `chars()`);
    // `find` đầu tiên, `rfind` cuối cùng, không khớp ⇒ `None`; `count` **không chồng**
    // (Rust `matches`). Needle rỗng: `find = Some(0)`, `rfind = Some(len)`, `count = len+1`.
    StdFn {
        name: "str.find",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::OptInt,
    },
    StdFn {
        name: "str.rfind",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::OptInt,
    },
    StdFn {
        name: "str.count",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::Int,
    },
    // **VD-270 (strstrip)**: bỏ **một lần** tiền tố/hậu tố đúng nguyên văn; khớp ⇒
    // `Some(phần còn lại)`, không khớp ⇒ `None`; affix rỗng ⇒ `Some(s)`. Cùng tham số đầu
    // `str` nên sugar R5c `s.strip_prefix(…)`/`s.strip_suffix(…)` chạy không cần `use`.
    StdFn {
        name: "str.strip_prefix",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::OptStr,
    },
    StdFn {
        name: "str.strip_suffix",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::OptStr,
    },
    StdFn {
        name: "str.repeat",
        params: &[Ty::Str, Ty::Int],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.replace",
        params: &[Ty::Str, Ty::Str, Ty::Str],
        ret: Ty::Str,
    },
    // **VD-270 (replace-first)**: thay **đúng lần đầu** của `from` bằng `to`
    // (Rust `replacen(…, 1)`), cùng đơn vị scalar Unicode với `str.replace`/`find`;
    // `from` rỗng ⇒ chèn `to` một lần ở đầu; không khớp ⇒ `s` nguyên vẹn. Cùng tham số
    // đầu `str` nên sugar R5c `s.replace_first(…)` chạy không cần `use`.
    StdFn {
        name: "str.replace_first",
        params: &[Ty::Str, Ty::Str, Ty::Str],
        ret: Ty::Str,
    },
    // **VD-269**: cắt khoảng trắng Unicode **một phía**; đệm theo **scalar** Unicode (`chars()`).
    // `fill` phải khác rỗng (rỗng ⇒ trap `VS0501`); `center` đặt scalar dư bên **phải**.
    StdFn {
        name: "str.trim_start",
        params: &[Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.trim_end",
        params: &[Ty::Str],
        ret: Ty::Str,
    },
    // **VD-270 (strtrim)**: cắt theo **tập scalar** `chars` (thứ tự/lặp không quan trọng);
    // `chars` rỗng ⇒ `s` nguyên vẹn; mọi scalar thuộc tập ⇒ `""`.
    StdFn {
        name: "str.trim_matches",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.trim_start_matches",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.trim_end_matches",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.pad_left",
        params: &[Ty::Str, Ty::Int, Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.pad_right",
        params: &[Ty::Str, Ty::Int, Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.center",
        params: &[Ty::Str, Ty::Int, Ty::Str],
        ret: Ty::Str,
    },
    // **VD-270 (strlen)**: `byte_len` = **byte UTF-8**; `char_count` = số **scalar Unicode**
    // (`chars()`, không phải grapheme); `str.is_empty` = chuỗi rỗng. Cùng tham số đầu `str`
    // nên sugar R5c `s.byte_len()`/`s.char_count()`/`s.is_empty()` chạy không cần `use`.
    // **VD-270 (parse-int)**: parse thập phân **chặt** (`i64::from_str`), sai/tràn ⇒ `None`;
    // trả `int?` để sugar `s.parse_int()` dùng chung `Ty::OptInt` với `find`/`index_of`.
    StdFn {
        name: "str.parse_int",
        params: &[Ty::Str],
        ret: Ty::OptInt,
    },
    // **VD-270 (parse-float)**: parse theo cú pháp thập phân Rust (`f64::from_str`: dấu,
    // `.5`/`5.`, mũ `e`/`E`) rồi **từ chối non-finite** (`inf`/`NaN`/tràn) ⇒ `None`;
    // trả `float?` để sugar `s.parse_float()` dùng `Ty::OptFloat`.
    StdFn {
        name: "str.parse_float",
        params: &[Ty::Str],
        ret: Ty::OptFloat,
    },
    StdFn {
        name: "str.byte_len",
        params: &[Ty::Str],
        ret: Ty::Int,
    },
    StdFn {
        name: "str.char_count",
        params: &[Ty::Str],
        ret: Ty::Int,
    },
    StdFn {
        name: "str.is_empty",
        params: &[Ty::Str],
        ret: Ty::Bool,
    },
    // ----- std.list — **ST-58 (VD-99 (4))**: `list[T]` thật (8 chữ ký đổi) -----
    // **VD-270 (strlen)**: rỗng ⇔ 0 phần tử; `bool`, không phụ thuộc `T`.
    StdFn {
        name: "list.is_empty",
        params: &[Ty::List(&Ty::Var(0))],
        ret: Ty::Bool,
    },
    StdFn {
        name: "list.sort",
        params: &[Ty::List(&Ty::Var(0))],
        ret: Ty::List(&Ty::Var(0)),
    },
    StdFn {
        name: "list.sort_by",
        params: &[Ty::List(&Ty::Var(0)), Ty::FnKhoa],
        ret: Ty::List(&Ty::Var(0)),
    },
    StdFn {
        name: "list.reverse",
        params: &[Ty::List(&Ty::Var(0))],
        ret: Ty::List(&Ty::Var(0)),
    },
    StdFn {
        name: "list.unique",
        params: &[Ty::List(&Ty::Var(0))],
        ret: Ty::List(&Ty::Var(0)),
    },
    StdFn {
        name: "list.concat",
        params: &[Ty::List(&Ty::Var(0)), Ty::List(&Ty::Var(0))],
        ret: Ty::List(&Ty::Var(0)),
    },
    // **VD-270 (flatten)**: nối **một cấp** các list con theo thứ tự; `xs: list[list[T]]`
    // ⇒ `list[T]`. List con rỗng không đóng góp gì; phần tử vốn là list vẫn giữ nguyên
    // (không làm phẳng đệ quy). `T` buộc từ `xs` nên sugar `xs.flatten()` theo R5c.
    StdFn {
        name: "list.flatten",
        params: &[Ty::List(&Ty::List(&Ty::Var(0)))],
        ret: Ty::List(&Ty::Var(0)),
    },
    StdFn {
        name: "list.chunk",
        params: &[Ty::List(&Ty::Var(0)), Ty::Int],
        ret: Ty::List(&Ty::List(&Ty::Var(0))),
    },
    // **VD-270**: cửa sổ **trượt** (chồng nhau `n-1`), khác `chunk` (không chồng). `n <= 0`
    // là trap `VS0501` ở tầng chạy (không phải lỗi kiểu); kiểu trả `list[list[T]]`.
    StdFn {
        name: "list.windows",
        params: &[Ty::List(&Ty::Var(0)), Ty::Int],
        ret: Ty::List(&Ty::List(&Ty::Var(0))),
    },
    // **VD-270 (indexof)**: vị trí **0-based** đầu/cuối có phần tử `==` `value` (so **cấu
    // trúc** như `==`); không có/`xs` rỗng ⇒ `None`; chữ ký `(list[T], T) -> int?`.
    StdFn {
        name: "list.index_of",
        params: &[Ty::List(&Ty::Var(0)), Ty::Var(0)],
        ret: Ty::OptInt,
    },
    StdFn {
        name: "list.last_index_of",
        params: &[Ty::List(&Ty::Var(0)), Ty::Var(0)],
        ret: Ty::OptInt,
    },
    // **VD-270 (list-contains)**: `true` iff có phần tử `==` `value` (cùng `eq_value` như
    // `index_of`; `T` buộc từ `xs` nên `value` cùng kiểu phần tử); sugar `xs.contains(v)` R5c.
    StdFn {
        name: "list.contains",
        params: &[Ty::List(&Ty::Var(0)), Ty::Var(0)],
        ret: Ty::Bool,
    },
    // **VD-270 (binary-search)**: `xs` phải **đã sắp tăng** theo `cmp_value` (cùng thứ tự
    // `list.sort`/`min`); trả **biên dưới** `Some(vị trí đầu của cụm bằng)` hoặc `None`.
    // `T` buộc từ `xs` nên `value` cùng kiểu phần tử; sugar `xs.binary_search(v)` theo R5c.
    StdFn {
        name: "list.binary_search",
        params: &[Ty::List(&Ty::Var(0)), Ty::Var(0)],
        ret: Ty::OptInt,
    },
    StdFn {
        name: "list.take",
        params: &[Ty::List(&Ty::Var(0)), Ty::Int],
        ret: Ty::List(&Ty::Var(0)),
    },
    StdFn {
        name: "list.drop",
        params: &[Ty::List(&Ty::Var(0)), Ty::Int],
        ret: Ty::List(&Ty::Var(0)),
    },
    // **VD-265** — ghép cặp (dừng ở danh sách ngắn hơn) và gộp nhỏ/lớn.
    // `min`/`max` trên danh sách rỗng là **trap sạch** (`VS0501`), không panic.
    StdFn {
        name: "list.zip",
        params: &[Ty::List(&Ty::Var(0)), Ty::List(&Ty::Var(0))],
        ret: Ty::List(&Ty::List(&Ty::Var(0))),
    },
    StdFn {
        name: "list.min",
        params: &[Ty::List(&Ty::Var(0))],
        ret: Ty::Var(0),
    },
    StdFn {
        name: "list.max",
        params: &[Ty::List(&Ty::Var(0))],
        ret: Ty::Var(0),
    },
    // ----- std.map — **ST-58**: `map[K, V]` thật (5 chữ ký đổi) -----
    // **VD-270 (strlen)**: rỗng ⇔ 0 cặp; `bool`, không phụ thuộc `K`/`V`.
    StdFn {
        name: "map.is_empty",
        params: &[Ty::Map(&Ty::Var(1), &Ty::Var(2))],
        ret: Ty::Bool,
    },
    StdFn {
        name: "map.keys",
        params: &[Ty::Map(&Ty::Var(1), &Ty::Var(2))],
        ret: Ty::List(&Ty::Var(1)),
    },
    StdFn {
        name: "map.values",
        params: &[Ty::Map(&Ty::Var(1), &Ty::Var(2))],
        ret: Ty::List(&Ty::Var(2)),
    },
    StdFn {
        name: "map.merge",
        params: &[
            Ty::Map(&Ty::Var(1), &Ty::Var(2)),
            Ty::Map(&Ty::Var(1), &Ty::Var(2)),
        ],
        ret: Ty::Map(&Ty::Var(1), &Ty::Var(2)),
    },
    StdFn {
        name: "map.get_or",
        params: &[Ty::Map(&Ty::Var(1), &Ty::Var(2)), Ty::Str, Ty::Var(2)],
        ret: Ty::Var(2),
    },
    StdFn {
        name: "map.remove",
        params: &[Ty::Map(&Ty::Var(1), &Ty::Var(2)), Ty::Str],
        ret: Ty::Map(&Ty::Var(1), &Ty::Var(2)),
    },
    // ----- std.path (thuần chuỗi) -----
    StdFn {
        name: "path.join",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "path.basename",
        params: &[Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "path.dirname",
        params: &[Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "path.ext",
        params: &[Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "path.stem",
        params: &[Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "path.is_abs",
        params: &[Ty::Str],
        ret: Ty::Bool,
    },
    // **VD-270 (path.normalize)**: chuẩn hoá **từ vựng** POSIX, thuần chuỗi — không chạm đĩa.
    StdFn {
        name: "path.normalize",
        params: &[Ty::Str],
        ret: Ty::Str,
    },
    // ----- std.time (đọc đồng hồ trong Rust, không qua Host) -----
    StdFn {
        name: "time.now_ms",
        params: &[],
        ret: Ty::Int,
    },
    StdFn {
        name: "time.monotonic_ms",
        params: &[],
        ret: Ty::Int,
    },
    // ----- std.json (chỉ encode; decode hoãn — xem `HA_CHUA_HO_TRO`) -----
    StdFn {
        name: "json.encode",
        params: &[Ty::Var(0)],
        ret: Ty::Str,
    },
    StdFn {
        name: "json.encode_pretty",
        params: &[Ty::Var(0)],
        ret: Ty::Str,
    },
    // ST-33/ST-58 (VD-222 điểm 4): `json.decode` trả `Result[Json, Error]` — dữ liệu JSON được
    // bọc thành enum `Json` dựng sẵn; lỗi ⇒ bản ghi `Error` dựng sẵn (`message/source/span`).
    StdFn {
        name: "json.decode",
        params: &[Ty::Str],
        ret: Ty::ResultJsonErr,
    },
    // ----- std.fs (ST-33) — **đi qua `Host`**, không OS trong crate này -----
    StdFn {
        name: "fs.read",
        params: &[Ty::Str],
        ret: Ty::ResultStrStr,
    },
    StdFn {
        name: "fs.write",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    StdFn {
        name: "fs.exists",
        params: &[Ty::Str],
        ret: Ty::Bool,
    },
    StdFn {
        name: "fs.list",
        params: &[Ty::Str],
        ret: Ty::ResultListStrStr,
    },
    // **VD-266**: bốn helper thực dụng — `read_lines` trả `list[str]`; ba thao tác ghi trả `()`.
    // `fs.mkdir` là **một cấp** (không phải `mkdir -p`); `fs.remove` chỉ xoá tệp.
    StdFn {
        name: "fs.read_lines",
        params: &[Ty::Str],
        ret: Ty::ResultListStrStr,
    },
    StdFn {
        name: "fs.append",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    StdFn {
        name: "fs.mkdir",
        params: &[Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    StdFn {
        name: "fs.remove",
        params: &[Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    // **VD-269**: chép/đổi tên **tệp** — từ chối đích đã tồn tại; lỗi dữ liệu ⇒ `Err(Error)`.
    StdFn {
        name: "fs.copy",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    StdFn {
        name: "fs.rename",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    // **VD-268**: duyệt đệ quy — tương đối so với root, thư mục kèm `/`, DFS, bỏ symlink.
    StdFn {
        name: "fs.walk",
        params: &[Ty::Str],
        ret: Ty::ResultListStrStr,
    },
    // **VD-268**: lọc kết quả `walk` bằng mẫu glob (`*`, `?`, `**`) — tương đối, sắp theo byte.
    StdFn {
        name: "fs.glob",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ResultListStrStr,
    },
    // **VD-270**: metadata **chỉ đọc** — `size` = số byte **tệp thường**; predicate theo
    // `symlink_metadata` (symlink ⇒ `false`/`false`, `size` ⇒ `Err`), thiếu path ⇒ predicate
    // `Ok(false)`.
    StdFn {
        name: "fs.size",
        params: &[Ty::Str],
        ret: Ty::ResultIntErr,
    },
    StdFn {
        name: "fs.is_file",
        params: &[Ty::Str],
        ret: Ty::ResultBoolErr,
    },
    StdFn {
        name: "fs.is_dir",
        params: &[Ty::Str],
        ret: Ty::ResultBoolErr,
    },
    // **VD-270 (fsbytes)**: đọc/ghi **byte thô** — mỗi phần tử là một byte `0..=255`;
    // `read_bytes` → `Result[list[int], Error]`; `write_bytes` nhận `list[int]`, tạo/cắt tệp và
    // từ chối (không chạm đĩa) nếu có phần tử ngoài phạm vi.
    StdFn {
        name: "fs.read_bytes",
        params: &[Ty::Str],
        ret: Ty::ResultListIntErr,
    },
    StdFn {
        name: "fs.write_bytes",
        params: &[Ty::Str, Ty::List(&Ty::Int)],
        ret: Ty::ResultUnitStr,
    },
    // ----- std.net (ST-63, VD-100/VD-105) — **đi qua `Host::net`**, không OS trong crate này -----
    StdFn {
        name: "net.connect",
        params: &[Ty::Str],
        ret: Ty::ResultStrStr,
    },
    StdFn {
        name: "net.connect_timeout",
        params: &[Ty::Str, Ty::Int],
        ret: Ty::ResultStrStr,
    },
    StdFn {
        name: "net.read",
        params: &[Ty::Str, Ty::Int, Ty::Int],
        ret: Ty::ResultStrStr,
    },
    StdFn {
        name: "net.write",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    StdFn {
        name: "net.close",
        params: &[Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    // ----- std.http (ST-64, VD-100/VD-105) — giao thức ở `vis-interp/src/net.rs`, socket qua `NetHost`.
    // ----- net server (ST-65) — `listen`/`accept` qua `NetHost`, chỉ loopback.
    StdFn {
        name: "net.listen",
        params: &[Ty::Str],
        ret: Ty::ResultStrStr,
    },
    StdFn {
        name: "net.local_addr",
        params: &[Ty::Str],
        ret: Ty::ResultStrStr,
    },
    StdFn {
        name: "net.accept",
        params: &[Ty::Str],
        ret: Ty::ResultStrStr,
    },
    StdFn {
        name: "net.accept_timeout",
        params: &[Ty::Str, Ty::Int],
        ret: Ty::ResultStrStr,
    },
    StdFn {
        name: "net.close_listen",
        params: &[Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    StdFn {
        name: "http.get",
        params: &[Ty::Str],
        ret: Ty::ResultStrStr,
    },
    StdFn {
        name: "http.get_timeout",
        params: &[Ty::Str, Ty::Int],
        ret: Ty::ResultStrStr,
    },
    StdFn {
        name: "http.head",
        params: &[Ty::Str],
        ret: Ty::ResultHead,
    },
    StdFn {
        name: "http.head_timeout",
        params: &[Ty::Str, Ty::Int],
        ret: Ty::ResultHead,
    },
    // ----- std.sqlite (VD-241, Gate 1 — prototype nội bộ, **chưa** đông băng API) -----
    // Handle là token mờ `db:<32 hex>` (128 bit ngẫu nhiên, không đoán được từ output, không tái sử
    // dụng trong phiên). Params positional `list[SqlValue]`.
    // `execute`/`query`/`query_one` **một** statement; thiếu/thừa params ⇒ lỗi; giới hạn tài
    // nguyên theo hợp đồng VD-241 §6. **VD-270 (db-batch)**: `execute_batch` nhận **nhiều** câu,
    // **không** params, tách an toàn trên `;` ngoài chuỗi/comment/định danh; lỗi đầu tiên ⇒ `Err`
    // (các câu trước đã có tác dụng, không transaction ngầm).
    StdFn {
        name: "sqlite.open",
        params: &[Ty::Str],
        ret: Ty::ResultStrStr,
    },
    StdFn {
        name: "sqlite.open_with",
        params: &[Ty::Str, Ty::Bool, Ty::Int],
        ret: Ty::ResultStrStr,
    },
    StdFn {
        name: "sqlite.close",
        params: &[Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    StdFn {
        name: "sqlite.execute",
        params: &[Ty::Str, Ty::Str, Ty::List(&Ty::SqlValue)],
        ret: Ty::ResultIntErr,
    },
    // **VD-270 (db-batch)** — nhiều câu DDL/DML, **không** params; trả số câu đã chạy.
    StdFn {
        name: "sqlite.execute_batch",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ResultIntErr,
    },
    StdFn {
        name: "sqlite.query",
        params: &[Ty::Str, Ty::Str, Ty::List(&Ty::SqlValue)],
        ret: Ty::ResultRowSetErr,
    },
    // **VD-266** — như `query` nhưng chỉ lấy **0 hoặc 1** hàng (dừng sau hàng đầu).
    StdFn {
        name: "sqlite.query_one",
        params: &[Ty::Str, Ty::Str, Ty::List(&Ty::SqlValue)],
        ret: Ty::ResultRowSetErr,
    },
    // **VD-270 (db-explain)** — `EXPLAIN QUERY PLAN` của câu **chỉ đọc** (`SELECT`/`WITH` sau
    // khoảng trắng/comment đầu); cùng params/authorizer/trần/timeout/cache như `query`.
    StdFn {
        name: "sqlite.explain",
        params: &[Ty::Str, Ty::Str, Ty::List(&Ty::SqlValue)],
        ret: Ty::ResultRowSetErr,
    },
    // **VD-266** — `rowid` của lần INSERT gần nhất trên connection (metadata chỉ đọc).
    StdFn {
        name: "sqlite.last_insert_rowid",
        params: &[Ty::Str],
        ret: Ty::ResultIntErr,
    },
    // **VD-266** — số hàng bị INSERT/UPDATE/DELETE gần nhất (metadata chỉ đọc).
    StdFn {
        name: "sqlite.changes",
        params: &[Ty::Str],
        ret: Ty::ResultIntErr,
    },
    StdFn {
        name: "sqlite.begin",
        params: &[Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    StdFn {
        name: "sqlite.commit",
        params: &[Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    StdFn {
        name: "sqlite.rollback",
        params: &[Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    StdFn {
        name: "sqlite.in_transaction",
        params: &[Ty::Str],
        ret: Ty::ResultBoolErr,
    },
    // **VD-270 (sqlite-tables)** — tên bảng/view người dùng của schema chính, sắp theo byte, bỏ
    // đối tượng nội bộ `sqlite_%`; chạy qua `sqlite_master` bằng đúng đường `query` (authorizer/
    // chỉ đọc/trần/timeout/cache). Bảng TEMP **không** thuộc schema chính nên không xuất hiện.
    StdFn {
        name: "sqlite.tables",
        params: &[Ty::Str],
        ret: Ty::ResultListStrStr,
    },
    // **VD-270 (sqlite-columns)** — tên cột của **bảng/view schema chính**, theo **thứ tự khai
    // báo** (`cid`), giữ nguyên hoa/thường; tên đối tượng là **tham số bind** (`pragma_table_info(?)`),
    // không nối chuỗi. Bảng/view không tồn tại ⇒ `Ok([])`; lỗi handle/host ⇒ `Err(Error)` dữ liệu.
    StdFn {
        name: "sqlite.columns",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ResultListStrStr,
    },
    // **VD-270 (sqlite-indexes)** — tên index **người dùng tạo** của một bảng trong **schema
    // chính**, sắp theo **byte**, bỏ index tự động `sqlite_autoindex_%`; tên bảng là **tham số
    // bind** (`tbl_name = ?`), không nối chuỗi. Bảng/view không tồn tại ⇒ `Ok([])`; lỗi
    // handle/host ⇒ `Err(Error)` dữ liệu.
    StdFn {
        name: "sqlite.indexes",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ResultListStrStr,
    },
];

/// **VD-267** — chữ ký **builtin trần** (không có tiền tố module, không thuộc [`FUNCS`] `std.*`).
///
/// `panic(msg) -> never`: dừng chương trình bằng trap `VS0501` (exit **101**) ở **cả hai** engine;
/// kiểu `never` cho phép dùng ở mọi vị trí biểu thức (nhánh `else`, `return panic(…)`).
pub const BUILTIN_FUNCS: [StdFn; 1] = [StdFn {
    name: "panic",
    // Nhận `str`; bản ghi dựng sẵn `Error` cũng được nhận ở checker/runtime (VD-267 —
    // payload `AnyError` của thiết kế §6.4), nhưng `Ty` không có union nên chữ ký ghi `str`.
    params: &[Ty::Str],
    ret: Ty::Never,
}];

/// Tra chữ ký **builtin trần** theo tên (`panic`).
pub fn sig_builtin(name: &str) -> Option<&'static StdFn> {
    BUILTIN_FUNCS.iter().find(|f| f.name == name)
}

/// Tên **đã biết nhưng chưa hỗ trợ** ở VS1c — dùng để báo `VS0301` với thông điệp riêng.
/// (ST-33: `json.decode` **đã có** ⇒ bảng **rỗng**; giữ API cho các tên hoãn sau này.)
pub const HA_CHUA_HO_TRO: [&str; 0] = [];

/// Tra chữ ký theo tên đầy đủ.
pub fn sig_of(name: &str) -> Option<&'static StdFn> {
    FUNCS.iter().find(|f| f.name == name)
}

/// `true` nếu `name` là hàm `std.*` đã cấp.
pub fn la_std(name: &str) -> bool {
    sig_of(name).is_some()
}

/// `true` nếu `name` là tên std **đã biết nhưng hoãn** (báo thông điệp riêng).
pub fn hoan(name: &str) -> bool {
    HA_CHUA_HO_TRO.contains(&name)
}

/// `true` nếu `m` là module std được hỗ trợ.
pub fn module_ho_tro(m: &str) -> bool {
    MODULES.contains(&m)
}

/// **VD-270 (path.normalize)** — module đích của sugar R5c khi tên **không** thuộc module lõi
/// của kiểu nhận: `s.normalize()` ≡ `path.normalize(s)` (đường dẫn POSIX là chuỗi). Chỉ trả
/// `Some` cho cặp đã được duyệt; caller giữ module mặc định khi `None`.
pub fn module_sugar(module: &str, ten: &str) -> Option<&'static str> {
    match (module, ten) {
        ("str", "normalize") => Some("path"),
        _ => None,
    }
}

/// Mọi hàm thuộc module `m` (tên đầy đủ), theo thứ tự khai.
pub fn funcs_cua(m: &str) -> impl Iterator<Item = &'static StdFn> {
    let tien = format!("{m}.");
    FUNCS.iter().filter(move |f| f.name.starts_with(&tien))
}
