// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (strstrip)** — ép kiểu đối số của `std.*` sang `str`/`int`/`list`/`map`.
//!
//! Tách khỏi `stdlib.rs` để giữ R1 (< 1500 dòng) khi thêm `str.strip_prefix`/`str.strip_suffix`.
//! Thông điệp lỗi **giữ nguyên văn** (số đối số 1-based + kiểu nhận được): `kiem` đã bắt phần lớn
//! trường hợp, nhưng dữ liệu không kiểu (từ `Value` thô/host) vẫn tới đây ở **cả** interp và VM.

use vis_syntax::Span;

use crate::interp::Abort;
use crate::value::Value;

fn loi(msg: impl Into<String>, span: Span) -> Abort {
    super::loi(msg, span)
}

pub(super) fn chuoi<'a>(name: &str, v: &'a Value, i: usize, span: Span) -> Result<&'a str, Abort> {
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

pub(super) fn so(name: &str, v: &Value, i: usize, span: Span) -> Result<i64, Abort> {
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

pub(super) fn danh_sach<'a>(
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

pub(super) fn ban_do<'a>(
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
