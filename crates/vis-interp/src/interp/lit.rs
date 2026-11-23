// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310a (VD-285)** — literal, tên dựng sẵn và định dạng `{expr:.N}`, tách khỏi
//! `interp.rs`. Chuyển **nguyên văn**, không đổi ngữ nghĩa.

use super::*;

/// Đọc literal thành `Value` (lỗi → thông điệp cho `VS0501`).
pub fn lit_value(lit: &Lit) -> std::result::Result<Value, String> {
    match lit.kind {
        LitKind::Int => crate::value::parse_int_lit(&lit.text)
            .map(Value::Int)
            .ok_or_else(|| format!("literal int không đọc được: `{}`", lit.text)),
        LitKind::Float => lit
            .text
            .replace('_', "")
            .parse::<f64>()
            .map(Value::Float)
            .map_err(|_| format!("literal float không đọc được: `{}`", lit.text)),
        LitKind::Bool => match lit.text.as_str() {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            other => Err(format!("literal bool lạ: `{other}`")),
        },
        LitKind::Char => parse_char_lit(&lit.text)
            .map(Value::Char)
            .ok_or_else(|| format!("literal char không đọc được: `{}`", lit.text)),
        LitKind::Str => Ok(Value::Str(parse_str_lit(&lit.text, false))),
        LitKind::Bytes => parse_bytes_lit(&lit.text)
            .map(Value::Bytes)
            .ok_or_else(|| format!("literal bytes không đọc được: `{}`", lit.text)),
        // **R6a (VD-178)**: `p"…"` **không** escape (đường dẫn Windows).
        LitKind::Path => Ok(Value::Path(parse_str_lit(&lit.text, true))),
        LitKind::Size => parse_size(&lit.text)
            .map(Value::Size)
            .ok_or_else(|| format!("literal size không đọc được: `{}`", lit.text)),
        LitKind::Duration => parse_duration(&lit.text)
            .map(Value::Duration)
            .ok_or_else(|| format!("literal duration không đọc được: `{}`", lit.text)),
        LitKind::Percent => parse_percent(&lit.text)
            .map(Value::Percent)
            .ok_or_else(|| format!("literal percent không đọc được: `{}`", lit.text)),
    }
}

pub(crate) const MAX_CALL_DEPTH: u32 = 2_000;

pub(crate) fn builtin_name(name: &str) -> Option<&'static str> {
    // ST-27: chương trình ngoài — `builtin::call` sẽ hỏi **host** (`None` ⇒ `VS0604`).
    if name.starts_with("exec.") {
        return Some("exec");
    }
    // ST-26: `std.<module>.<fn>` gọi qua bảng `stdlib` (dùng chung với VM, VD-55).
    if name.starts_with("std.") {
        return name
            .strip_prefix("std.")
            .and_then(vis_types::stdlib::sig_of)
            .map(|f| f.name);
    }
    match name {
        "print" => Some("print"),
        "assert" => Some("assert"),
        "panic" => Some("panic"),
        "len" => Some("len"),
        "str" => Some("str"),
        "Ok" => Some("Ok"),
        "Err" => Some("Err"),
        "Some" => Some("Some"),
        "None" => Some("None"),
        "ref" => Some("ref"),
        // ST-18: 3 lệnh builtin của chế độ lệnh (VS0) — chữ ký ở `vis-types/lower/cmd.rs`.
        "echo" => Some("echo"),
        "lines" => Some("lines"),
        "take" => Some("take"),
        // ST-38: closure-arg của chế độ lệnh.
        "sort_by" => Some("sort_by"),
        "render" => Some("render"),
        "ls" => Some("ls"),
        "open" => Some("open"),
        "get" => Some("get"),
        "select" => Some("select"),
        "where" => Some("where"),
        "each" => Some("each"),
        "first" => Some("first"),
        "count" => Some("count"),
        _ => None,
    }
}
/// **R6c (VD-178)**: định dạng `{expr:spec}` — hiện hỗ trợ `.N` (N chữ số thập phân).
pub fn dinh_dang(v: &Value, spec: &str, span: Span) -> Result<String, Abort> {
    let Some(n) = spec.strip_prefix('.') else {
        return Err(Abort::Trap(Trap::new(
            RT,
            format!("format spec `{spec}` chưa hỗ trợ — dùng `.N`"),
            span,
        )));
    };
    let n: usize = n.parse().map_err(|_| {
        Abort::Trap(Trap::new(
            RT,
            format!("format spec `{spec}` chưa hỗ trợ — dùng `.N`"),
            span,
        ))
    })?;
    Ok(match v {
        Value::Float(f) => format!("{f:.n$}"),
        Value::Int(i) => format!("{:.n$}", *i as f64),
        Value::Str(s) => s.clone(),
        other => other.show(),
    })
}
