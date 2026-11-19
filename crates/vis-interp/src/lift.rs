// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-241 §3 (A0, 2026-11-19)** — helper nâng `Opt` tới đủ lớp ở biên annot/return.
//! Một chỗ duy nhất cho interp + builtin `$lift-opt` (VM). Đích `Opt` là khoảng lớp; giá trị
//! đã `Opt`/`Result` đủ lớp giữ nguyên (không bọc chồng).

use crate::value::Value;

/// Số lớp `Opt` của một **giá trị** (đếm từ ngoài vào); `None`/`Ok`/`Err` kết thúc chuỗi.
pub(crate) fn opt_depth_val(v: &Value) -> usize {
    let mut d = 0;
    let mut cur = v;
    while let Value::Enum(ea, va, args) = cur {
        let is_opt_like = (ea == "Option" || ea == "Result")
            && matches!(va.as_str(), "Some" | "None" | "Ok" | "Err");
        if !is_opt_like {
            break;
        }
        d += 1;
        match va.as_str() {
            "Some" if args.len() == 1 => {
                cur = &args[0];
            }
            _ => break,
        }
    }
    d
}

/// Nâng `v` tới ít nhất `depth` lớp `Opt` (bọc `Value::some` các lớp còn thiếu).
pub(crate) fn lift_opt(v: Value, depth: usize) -> Value {
    let mut dep = opt_depth_val(&v);
    let mut out = v;
    while dep < depth {
        out = Value::some(out);
        dep += 1;
    }
    out
}

/// `args = [value, depth-int]`; sai cấu trúc ⇒ nâng một lớp (một lớp là biên hợp lệ).
pub(crate) fn lift_opt_args(args: Vec<Value>) -> Value {
    let mut it = args.into_iter();
    let v = it.next().unwrap_or(Value::Unit);
    let depth = match it.next() {
        Some(Value::Int(d)) if d >= 0 => d as usize,
        _ => 1,
    };
    lift_opt(v, depth)
}
