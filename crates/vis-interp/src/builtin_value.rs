// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-208 (2)** — dựng **giá trị dựng sẵn** qua builtin nội bộ `$value:<tên>`.
//!
//! `$value:` là một **sơ đồ nội bộ**: không tên nào viết được trong mã nguồn (`:` không hợp lệ
//! trong tên dựng sẵn) — chỉ codegen phát ra. Dùng lại opcode `CallBuiltin` (không thêm mã mới).

use crate::value::Value;

/// Dựng `Value::Builtin(<tên>)` nếu `ten` là tên dựng sẵn hợp lệ; `None` nếu không.
pub(crate) fn value_builtin(ten: &str) -> Option<Value> {
    match ten {
        "print" => Some(Value::Builtin("print")),
        "len" => Some(Value::Builtin("len")),
        "str" => Some(Value::Builtin("str")),
        "echo" => Some(Value::Builtin("echo")),
        "lines" => Some(Value::Builtin("lines")),
        "take" => Some(Value::Builtin("take")),
        "Ok" => Some(Value::Builtin("Ok")),
        "Err" => Some(Value::Builtin("Err")),
        "Some" => Some(Value::Builtin("Some")),
        "ref" => Some(Value::Builtin("ref")),
        _ => None,
    }
}
