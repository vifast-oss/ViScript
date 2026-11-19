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

/// `true` nếu `v` là **`None` trần ở lớp ngoài cùng** (Option ngoài để vacant).
fn la_none_tran(v: &Value) -> bool {
    matches!(v, Value::Enum(ea, va, ag) if ea == "Option" && va == "None" && ag.is_empty())
}

/// Nâng `v` tới ít nhất `depth` lớp `Opt` (bọc `Value::some` các lớp còn thiếu).
///
/// **VD-241 §3+ (A0, 2026-11-19, đợt coercion)** — `None` **trần** là giá trị trực tiếp hợp lệ
/// của **mọi** đích `Option[…]` (lớp ngoài vacant; lõi không bị ràng buộc), kể cả
/// `Option[Option[T]]` trở lên — **giữ nguyên**, không bọc `Some(None)` ở biên annot/return/
/// tham số/`let`. Giá trị khác (`Some(…)`, lõi plain) vẫn nâng **đúng một lớp** theo
/// `opt_depth_val` như cũ; checker đã chặn nguồn cần nâng quá một lớp (VD-227/228, §3.5).
pub(crate) fn lift_opt(v: Value, depth: usize) -> Value {
    if la_none_tran(&v) {
        // None ngoài vacant = lớp ngoài của đích đã có mặt — không bọc chồng.
        return v;
    }
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
