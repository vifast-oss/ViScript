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
pub const FUNCS: [StdFn; 61] = [
    // ----- std.str -----
    StdFn {
        name: "str.split",
        params: &[Ty::Str, Ty::Str],
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
    // ----- std.list — **ST-58 (VD-99 (4))**: `list[T]` thật (8 chữ ký đổi) -----
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
    StdFn {
        name: "list.chunk",
        params: &[Ty::List(&Ty::Var(0)), Ty::Int],
        ret: Ty::List(&Ty::List(&Ty::Var(0))),
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
    // ----- std.map — **ST-58**: `map[K, V]` thật (5 chữ ký đổi) -----
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
    // Mỗi câu SQL **một** statement; thiếu/thừa params ⇒ lỗi; giới hạn tài nguyên theo hợp đồng VD-241 §6.
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
    StdFn {
        name: "sqlite.query",
        params: &[Ty::Str, Ty::Str, Ty::List(&Ty::SqlValue)],
        ret: Ty::ResultRowSetErr,
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
];

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

/// Mọi hàm thuộc module `m` (tên đầy đủ), theo thứ tự khai.
pub fn funcs_cua(m: &str) -> impl Iterator<Item = &'static StdFn> {
    let tien = format!("{m}.");
    FUNCS.iter().filter(move |f| f.name.starts_with(&tien))
}
