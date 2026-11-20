// SPDX-License-Identifier: MIT OR Apache-2.0
//! VD-257: tách helper giá trị ra khỏi `vm.rs` để qua luật R1 (≤1500 dòng).
use super::*;

/// `kind` số của const pool → `LitKind`.
pub(super) fn lit_kind(k: u8) -> Option<LitKind> {
    Some(match k {
        0 => LitKind::Int,
        1 => LitKind::Float,
        2 => LitKind::Size,
        3 => LitKind::Duration,
        4 => LitKind::Percent,
        5 => LitKind::Str,
        6 => LitKind::Bytes,
        7 => LitKind::Path,
        8 => LitKind::Char,
        9 => LitKind::Bool,
        _ => return None,
    })
}

pub(super) fn field(v: &Value, name: &str, span: Span) -> R<Value> {
    // `.` trên `Some(x)` tự bóc (interp làm vậy) — `?.` mới là đường có kiểm `None`.
    // `.` trên `Some(x)` tự bóc (interp làm vậy) — `?.` mới là đường có kiểm `None`.
    if let Some(x) = v.opt_payload() {
        return field(x, name, span);
    }
    match v {
        Value::Record(_, fs) => fs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
            .ok_or_else(|| {
                Abort::Trap(Trap::new(
                    RT,
                    format!("bản ghi không có trường `{name}`"),
                    span,
                ))
            }),
        Value::Enum(_, _, args) => {
            let i: usize = name.parse().map_err(|_| {
                Abort::Trap(Trap::new(RT, format!("`{name}` không phải trường"), span))
            })?;
            args.get(i)
                .cloned()
                .ok_or_else(|| Abort::Trap(Trap::new(RT, format!("không có đối số thứ {i}"), span)))
        }
        other => Err(Abort::Trap(Trap::new(
            RT,
            format!("`{}` không có trường `{name}`", other.show()),
            span,
        ))),
    }
}

pub(super) fn index(base: &Value, idx: &Value, span: Span) -> R<Value> {
    // ST-30b: **trùng đúng** chẩn đoán của interp (cùng mã `VS0503`, cùng thông điệp) — hai đường
    // phải cho **cùng** danh sách `(mã, dòng, cột, thông điệp)`; trước đây VM dùng `VS0501` + câu ngắn.
    match (base, idx) {
        (Value::List(xs), Value::Int(i)) => {
            let len = xs.len() as i64;
            if *i < 0 {
                return Err(Abort::Trap(Trap::new(
                    IDX,
                    format!("chỉ số âm `{i}` (dùng `.last()` hoặc `xs[i]?`)"),
                    span,
                )));
            }
            xs.get(*i as usize).cloned().ok_or_else(|| {
                Abort::Trap(Trap::new(
                    IDX,
                    format!("chỉ số {i} ngoài phạm vi (độ dài {len})"),
                    span,
                ))
            })
        }
        (Value::Bytes(bs), Value::Int(i)) => {
            let len = bs.len() as i64;
            if *i < 0 || *i >= len {
                return Err(Abort::Trap(Trap::new(
                    IDX,
                    format!("chỉ số {i} ngoài phạm vi (độ dài {len})"),
                    span,
                )));
            }
            Ok(Value::Int(bs[*i as usize] as i64))
        }
        (Value::Map(kv), k) => kv
            .iter()
            .find(|(kk, _)| value::eq(kk, k))
            .map(|(_, v)| v.clone())
            .ok_or_else(|| {
                Abort::Trap(Trap::new(
                    IDX,
                    format!("khoá `{}` không có trong map", k.show()),
                    span,
                ))
            }),
        (other, idx) => Err(Abort::Trap(Trap::new(
            RT,
            format!("không chỉ số được `{}` bằng `{}`", other.show(), idx.show()),
            span,
        ))),
    }
}

pub(super) fn set(base: &Value, idx: &Value, v: Value, span: Span) -> R<Value> {
    match (base, idx) {
        (Value::List(xs), Value::Int(i)) => {
            let mut xs = xs.clone();
            if (*i as usize) < xs.len() {
                xs[*i as usize] = v;
                Ok(Value::List(xs))
            } else {
                Err(Abort::Trap(Trap::new(
                    RT,
                    format!("chỉ số {i} ngoài phạm vi"),
                    span,
                )))
            }
        }
        (Value::Map(kv), k) => {
            let mut kv = kv.clone();
            if let Some(e) = kv.iter_mut().find(|(kk, _)| value::eq(kk, k)) {
                e.1 = v;
            } else {
                kv.push((k.clone(), v));
            }
            Ok(Value::Map(kv))
        }
        _ => Err(Abort::Trap(Trap::new(RT, "gán chỉ số không hợp lệ", span))),
    }
}

pub(super) fn with(base: &Value, pairs: Vec<(String, Value)>, span: Span) -> R<Value> {
    match base {
        Value::Record(ten, fs) => {
            let mut fs = fs.clone();
            for (k, v) in pairs {
                if let Some(e) = fs.iter_mut().find(|(kk, _)| *kk == k) {
                    e.1 = v;
                } else {
                    return Err(Abort::Trap(Trap::new(
                        RT,
                        format!("bản ghi `{ten}` không có trường `{k}`"),
                        span,
                    )));
                }
            }
            Ok(Value::Record(ten.clone(), fs))
        }
        other => Err(Abort::Trap(Trap::new(
            RT,
            format!("`with` không dùng được cho `{}`", other.show()),
            span,
        ))),
    }
}

/// **R8 (VD-178)**: `xs[i]?` — trả `Some(v)` nếu trong phạm vi, `None` nếu ngoài (không panic).
pub(super) fn index_opt(base: &Value, idx: &Value) -> Value {
    match (base, idx) {
        (Value::List(xs), Value::Int(i)) if *i >= 0 => Value::opt(xs.get(*i as usize).cloned()),
        (Value::Bytes(bs), Value::Int(i)) if *i >= 0 => {
            Value::opt(bs.get(*i as usize).map(|b| Value::Int(*b as i64)))
        }
        (Value::Map(kv), k) => Value::opt(
            kv.iter()
                .find(|(kk, _)| kk.eq_value(k))
                .map(|(_, v)| v.clone()),
        ),
        _ => Value::none(),
    }
}
