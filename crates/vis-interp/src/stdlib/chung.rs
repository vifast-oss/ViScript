// SPDX-License-Identifier: MIT OR Apache-2.0
//! Helper dùng chung cho `stdlib` (tách khỏi `stdlib.rs` để giữ R1).

use vis_diag::{COMMAND_CODES, RUNTIME_CODES};
use vis_syntax::Span;
use vis_types::stdlib::{sig_of, Ty};

use crate::interp::{Abort, Trap};
use crate::value::Value;

/// `Err(String)` của host ⇒ `Result` của VS0 (`Err(msg)`), **không** phải trap.
/// **ST-72b (VD-103/VD-106)**: dấu **cưỡng chế quyền** trong `Err` của host — `hop_nhat_fs` đổi
/// thành **trap `VS0604`** (không bắt được), thay vì `Err(str)` dữ liệu (vượt quyền là lỗi **môi
/// trường**, không phải dữ liệu chương trình).
pub const DAU_QUYEN: &str = "\u{1}VS0604:";

pub(crate) fn hop_nhat_fs(r: Result<Value, String>, span: Span) -> Result<Value, Abort> {
    match r {
        Ok(v) => Ok(Value::ok(v)),
        Err(e) => match e.strip_prefix(DAU_QUYEN) {
            Some(msg) => Err(Abort::Trap(Trap::new(
                &COMMAND_CODES[3],
                msg.to_string(),
                span,
            ))),
            // Mọi lỗi host khác ⇒ `Err(Error)` dữ liệu (VD-241 pre-merge §1).
            None => Ok(Value::err(Value::ban_ghi_error(e, None))),
        },
    }
}

/// **VD-266** — `(columns, rows)` từ host → `Ok(RowSet { columns, rows })` (dùng chung
/// `sqlite.query`/`query_one`).
pub(crate) fn rowset(columns: Vec<String>, rows: Vec<Vec<Value>>) -> Value {
    let columns = Value::List(columns.into_iter().map(Value::Str).collect());
    let rows = Value::List(rows.into_iter().map(Value::List).collect());
    Value::ok(Value::Record(
        "RowSet".into(),
        vec![("columns".into(), columns), ("rows".into(), rows)],
    ))
}

/// Không có host ⇒ `VS0604` (cùng câu với chương trình ngoài, VD-60 (2)).
pub(crate) fn khong_host(name: &str, span: Span) -> Abort {
    Abort::Trap(Trap::new(
        &COMMAND_CODES[3],
        format!("`std.{name}` chưa hỗ trợ ở môi trường này (không có host)"),
        span,
    ))
}

pub(crate) fn loi(msg: impl Into<String>, span: Span) -> Abort {
    Abort::Trap(Trap::new(&RUNTIME_CODES[0], msg, span))
}

/// Kiểm số đối số + kiểu theo bảng chữ ký; trả `Err(Abort)` nếu sai.
pub(crate) fn kiem(name: &str, args: &[Value], span: Span) -> Result<(), Abort> {
    let Some(sig) = sig_of(name) else {
        return Err(loi(
            format!("hàm `std.{name}` không có trong bảng chữ ký"),
            span,
        ));
    };
    if args.len() != sig.so_doi() {
        return Err(loi(
            format!("`{name}` cần {} đối số, được {}", sig.so_doi(), args.len()),
            span,
        ));
    }
    for (i, (a, t)) in args.iter().zip(sig.params.iter()).enumerate() {
        let ok = match t {
            // **ST-58 (VD-94/VD-99)**: biến kiểu / `list[T]` / `map[K,V]` — thân hàm tự kiểm như
            // trước với `Any` (tyck đã chặn **tĩnh** phần lớn; dữ liệu không kiểu vẫn tới đây).
            Ty::Var(_) | Ty::Map(..) => true,
            // **VD-241**: `list[SqlValue]` (params của `std.sqlite`) — kiểm **phần tử** là `SqlValue`.
            Ty::List(inner) if **inner == Ty::SqlValue => {
                matches!(a, Value::List(xs) if xs.iter().all(|v| matches!(v, Value::Enum(e, _, _) if e == "SqlValue")))
            }
            Ty::List(_) => true,
            Ty::Int => matches!(a, Value::Int(_)),
            Ty::Str => matches!(a, Value::Str(_)),
            Ty::Bool => matches!(a, Value::Bool(_)),
            Ty::ListStr => {
                matches!(a, Value::List(xs) if xs.iter().all(|v| matches!(v, Value::Str(_))))
            }
            Ty::FnKhoa => matches!(a, Value::Fn(_) | Value::Vm(_) | Value::Builtin(_)),
            // **ST-33: `std.fs` trả `Result[..]` — kiểu **trả**, không dùng để kiểm đối số.
            // **VD-222**: `Error`/`Json`/`Result[Json, Error]` — kiểu **trả** của `std.*` (lỗi/JSON),
            // không dùng để kiểm đối số người dùng; runtime giữ nguyên **tín hiệu** như trước.
            Ty::ResultStrStr
            | Ty::ResultUnitStr
            | Ty::ResultListStrStr
            | Ty::ResultHead
            | Ty::ResultJsonErr
            | Ty::Error
            | Ty::Json
            // **VD-241 (Gate 1)**: `Result[RowSet, Error]` cũng là kiểu **trả** của `std.sqlite.query`.
            | Ty::ResultIntErr
            | Ty::ResultBoolErr
            | Ty::ResultRowSetErr
            // **VD-270 (fsbytes)**: `Result[list[int], Error]` là kiểu **trả** của `fs.read_bytes`.
            | Ty::ResultListIntErr
            // **VD-270 (strfind)**: `int?` là kiểu **trả** của `str.find`/`str.rfind`.
            | Ty::OptInt => true,
            // **VD-241**: `SqlValue` tham số (chưa dùng ở vị trí đối số)/`RowSet` — kiểm runtime nếu đi qua.
            Ty::SqlValue => matches!(a, Value::Enum(e, _, _) if e == "SqlValue"),
            Ty::RowSet => matches!(a, Value::Record(n, _) if n == "RowSet"),
            // **VD-267**: `never` chỉ là kiểu **trả** của builtin trần — không kiểm đối số.
            Ty::Never => true,
        };
        if !ok {
            return Err(loi(
                format!(
                    "`{name}`: đối số {} cần `{}`, được `{}`",
                    i + 1,
                    t.ten(),
                    a.show()
                ),
                span,
            ));
        }
    }
    Ok(())
}

pub(crate) fn chuoi<'a>(name: &str, v: &'a Value, i: usize, span: Span) -> Result<&'a str, Abort> {
    match v {
        Value::Str(s) => Ok(s),
        other => Err(loi(
            format!(
                "`{name}`: đối số {} cần `str`, được `{}`",
                i + 1,
                other.show()
            ),
            span,
        )),
    }
}

pub(crate) fn so(name: &str, v: &Value, i: usize, span: Span) -> Result<i64, Abort> {
    match v {
        Value::Int(n) => Ok(*n),
        other => Err(loi(
            format!(
                "`{name}`: đối số {} cần `int`, được `{}`",
                i + 1,
                other.show()
            ),
            span,
        )),
    }
}

pub(crate) fn danh_sach<'a>(
    name: &str,
    v: &'a Value,
    i: usize,
    span: Span,
) -> Result<&'a [Value], Abort> {
    match v {
        Value::List(xs) => Ok(xs),
        other => Err(loi(
            format!(
                "`{name}`: đối số {} cần `list`, được `{}`",
                i + 1,
                other.show()
            ),
            span,
        )),
    }
}

pub(crate) fn ban_do<'a>(
    name: &str,
    v: &'a Value,
    i: usize,
    span: Span,
) -> Result<&'a [(Value, Value)], Abort> {
    match v {
        Value::Map(kv) => Ok(kv),
        other => Err(loi(
            format!(
                "`{name}`: đối số {} cần `map`, được `{}`",
                i + 1,
                other.show()
            ),
            span,
        )),
    }
}
