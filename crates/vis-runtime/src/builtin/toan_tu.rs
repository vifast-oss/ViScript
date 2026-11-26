// SPDX-License-Identifier: MIT OR Apache-2.0
//! **Toán tử** dùng chung interp/VM: `bin`, `arith`, `un` (ST-20, R6b/R7, VD-243).

use vis_hir::hir::BinOp;
use vis_syntax::Span;

use crate::codes::{DIV0, RT};
use vis_core::value::Value;

use super::{overflow, pct_moi, pct_nhan_f64, trap};

pub fn bin(op: BinOp, l: Value, r: Value, span: Span) -> vis_core::trap::R<Value> {
    use BinOp::*;
    match op {
        // **R7 (VD-178)**: `//` — chia lấy phần nguyên (làm tròn **xuống**).
        FloorDiv => arith(op, l, r, span),
        Range | RangeIncl => match (&l, &r) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Range(*a, *b, matches!(op, RangeIncl))),
            _ => Err(trap(
                RT,
                format!(
                    "`{}` cần hai `int`, được `{}` và `{}`",
                    op.text(),
                    l.show(),
                    r.show()
                ),
                span,
            )),
        },
        Eq => Ok(Value::Bool(l.eq_value(&r))),
        Ne => Ok(Value::Bool(!l.eq_value(&r))),
        Lt | Le | Gt | Ge => {
            let Some(ord) = l.cmp_value(&r) else {
                return Err(trap(
                    RT,
                    format!("không so sánh được `{}` và `{}`", l.show(), r.show()),
                    span,
                ));
            };
            use std::cmp::Ordering::*;
            let b = match op {
                Lt => ord == Less,
                Le => ord != Greater,
                Gt => ord == Greater,
                _ => ord != Less,
            };
            Ok(Value::Bool(b))
        }
        Add | Sub | Mul | Div | Rem | Pow => arith(op, l, r, span),
        And | Or | Coalesce | Pipe => Err(trap(
            RT,
            format!("toán tử `{op:?}` phải xử lý ở tầng biểu thức"),
            span,
        )),
    }
}

fn arith(op: BinOp, l: Value, r: Value, span: Span) -> vis_core::trap::R<Value> {
    use BinOp::*;
    // `int` ↔ `float`: nâng literal int (thiết kế §3.5, ngoại lệ duy nhất).
    let (l, r) = match (l, r) {
        (Value::Int(a), Value::Float(b)) => (Value::Float(a as f64), Value::Float(b)),
        (Value::Float(a), Value::Int(b)) => (Value::Float(a), Value::Float(b as f64)),
        (l, r) => (l, r),
    };
    // `percent` với số: `percent * int -> int`, `percent * size -> size` (§3.6).
    // **ST-240 / VD-243 (#172)**: mọi đường `f64 -> i64` ở đây đều phải **kiểm miền**, không được
    // dùng `as i64` (Rust bão hoà im lặng ở `200pct * i64::MAX`). Tính trung gian bằng `f64` rồi
    // kiểm **kết quả cuối** — không thay máy móc bằng `checked_mul` trên int (VD-243 §2).
    if let (Value::Percent(p), rhs) = (&l, &r) {
        match (op, rhs) {
            (Mul, Value::Int(n)) => return Ok(Value::Int(pct_nhan_f64(*p, *n as f64, span)?)),
            (Mul, Value::Size(n)) => return Ok(Value::Size(pct_nhan_f64(*p, *n as f64, span)?)),
            (Mul, Value::Duration(n)) => {
                return Ok(Value::Duration(pct_nhan_f64(*p, *n as f64, span)?))
            }
            (Add | Sub, Value::Percent(q)) => {
                let r = if matches!(op, Add) { p + q } else { p - q };
                return Ok(Value::Percent(pct_moi(r, span)?));
            }
            (Div, Value::Int(n)) => {
                if *n == 0 {
                    return Err(trap(DIV0, "chia cho 0", span));
                }
                return Ok(Value::Percent(pct_moi(p / (*n as f64), span)?));
            }
            _ => {}
        }
    }
    if let (Value::Int(n), Value::Percent(p)) = (&l, &r) {
        if matches!(op, Mul) {
            return Ok(Value::Int(pct_nhan_f64(*p, *n as f64, span)?));
        }
    }
    // ST-13b (mục 5): đối xứng `size/duration * percent` và `percent / percent` (VD-48: **float**).
    if let (Value::Size(n), Value::Percent(p)) = (&l, &r) {
        if matches!(op, Mul) {
            return Ok(Value::Size(pct_nhan_f64(*p, *n as f64, span)?));
        }
    }
    if let (Value::Duration(n), Value::Percent(p)) = (&l, &r) {
        if matches!(op, Mul) {
            return Ok(Value::Duration(pct_nhan_f64(*p, *n as f64, span)?));
        }
    }
    if let (Value::Percent(a), Value::Percent(b)) = (&l, &r) {
        match op {
            // VD-48(b): `percent / percent` ⇒ **float** (tỉ lệ của hai tỉ lệ).
            Div => {
                if *b == 0.0 {
                    return Err(trap(DIV0, "chia cho 0", span));
                }
                return Ok(Value::Float(a / b));
            }
            Rem => {
                if *b == 0.0 {
                    return Err(trap(DIV0, "lấy dư cho 0", span));
                }
                return Ok(Value::Percent(pct_moi(a % b, span)?));
            }
            _ => {}
        }
    }
    if let (Value::Size(a), Value::Size(b)) = (&l, &r) {
        if matches!(op, Rem) {
            if *b == 0 {
                return Err(trap(DIV0, "lấy dư cho 0", span));
            }
            return Ok(Value::Size(a % b));
        }
    }
    if let (Value::Duration(a), Value::Duration(b)) = (&l, &r) {
        if matches!(op, Rem) {
            if *b == 0 {
                return Err(trap(DIV0, "lấy dư cho 0", span));
            }
            return Ok(Value::Duration(a % b));
        }
    }
    match (op, l, r) {
        // **R6b (VD-178)**: `str + str` **nối** chuỗi.
        (Add, Value::Str(a), Value::Str(b)) => Ok(Value::Str(format!("{a}{b}"))),
        // int
        (Add, Value::Int(a), Value::Int(b)) => {
            Ok(Value::Int(a.checked_add(b).ok_or_else(|| overflow(span))?))
        }
        (Sub, Value::Int(a), Value::Int(b)) => {
            Ok(Value::Int(a.checked_sub(b).ok_or_else(|| overflow(span))?))
        }
        (Mul, Value::Int(a), Value::Int(b)) => {
            Ok(Value::Int(a.checked_mul(b).ok_or_else(|| overflow(span))?))
        }
        // **R7 (VD-178)**: `/` **luôn** `float` (kể cả `int / int`).
        (Div, Value::Int(a), Value::Int(b)) => {
            if b == 0 {
                return Err(trap(DIV0, "chia cho 0 (`/0`)", span));
            }
            Ok(Value::Float(a as f64 / b as f64))
        }
        // **R7 (VD-178)**: `//` chia lấy phần nguyên — làm tròn **xuống** (Python).
        // **R7-fix**: `div_euclid` làm tròn **lên** khi `b < 0` ⇒ trừ 1 khi dư ≠ 0; dùng
        // `checked_*` để `i64::MIN // -1` ra `VS0501` (tràn) thay vì panic Rust.
        (FloorDiv, Value::Int(a), Value::Int(b)) => {
            if b == 0 {
                return Err(trap(DIV0, "chia cho 0 (`//0`)", span));
            }
            let q = a.checked_div_euclid(b).ok_or_else(|| overflow(span))?;
            let r = a.checked_rem_euclid(b).ok_or_else(|| overflow(span))?;
            Ok(Value::Int(if r != 0 && b < 0 { q - 1 } else { q }))
        }
        // **R7 (VD-178)**: `%` khớp `//` — dư **cùng dấu số chia** (Python).
        (Rem, Value::Int(a), Value::Int(b)) => {
            if b == 0 {
                return Err(trap(DIV0, "lấy dư cho 0 (`%0`)", span));
            }
            let r = a.checked_rem_euclid(b).ok_or_else(|| overflow(span))?;
            Ok(Value::Int(if r != 0 && b < 0 { r + b } else { r }))
        }
        (Pow, Value::Int(a), Value::Int(b)) => {
            if b < 0 {
                return Err(trap(RT, "`int ** int` với số mũ âm (VD-43 #2)", span));
            }
            let e = u32::try_from(b).map_err(|_| overflow(span))?;
            Ok(Value::Int(a.checked_pow(e).ok_or_else(|| overflow(span))?))
        }
        // float (IEEE 754 — `/0.0` không lỗi)
        (Add, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
        (Sub, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
        (Mul, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
        (Div, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
        (Rem, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a % b)),
        (Pow, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.powf(b))),
        // size
        (Add, Value::Size(a), Value::Size(b)) => {
            Ok(Value::Size(a.checked_add(b).ok_or_else(|| overflow(span))?))
        }
        (Sub, Value::Size(a), Value::Size(b)) => {
            Ok(Value::Size(a.checked_sub(b).ok_or_else(|| overflow(span))?))
        }
        (Mul, Value::Size(a), Value::Int(b)) => {
            Ok(Value::Size(a.checked_mul(b).ok_or_else(|| overflow(span))?))
        }
        (Mul, Value::Int(a), Value::Size(b)) => {
            Ok(Value::Size(a.checked_mul(b).ok_or_else(|| overflow(span))?))
        }
        (Div, Value::Size(a), Value::Int(b)) => {
            if b == 0 {
                return Err(trap(DIV0, "chia cho 0", span));
            }
            Ok(Value::Size(a / b))
        }
        (Div, Value::Size(a), Value::Size(b)) => {
            if b == 0 {
                return Err(trap(DIV0, "chia cho 0", span));
            }
            Ok(Value::Float(a as f64 / b as f64))
        }
        (Rem, Value::Size(a), Value::Int(b)) => {
            if b == 0 {
                return Err(trap(DIV0, "lấy dư cho 0", span));
            }
            Ok(Value::Size(a % b))
        }
        // duration
        (Add, Value::Duration(a), Value::Duration(b)) => Ok(Value::Duration(
            a.checked_add(b).ok_or_else(|| overflow(span))?,
        )),
        (Sub, Value::Duration(a), Value::Duration(b)) => Ok(Value::Duration(
            a.checked_sub(b).ok_or_else(|| overflow(span))?,
        )),
        (Mul, Value::Duration(a), Value::Int(b)) => Ok(Value::Duration(
            a.checked_mul(b).ok_or_else(|| overflow(span))?,
        )),
        (Mul, Value::Int(a), Value::Duration(b)) => Ok(Value::Duration(
            a.checked_mul(b).ok_or_else(|| overflow(span))?,
        )),
        (Div, Value::Duration(a), Value::Int(b)) => {
            if b == 0 {
                return Err(trap(DIV0, "chia cho 0", span));
            }
            Ok(Value::Duration(a / b))
        }
        (Div, Value::Duration(a), Value::Duration(b)) => {
            if b == 0 {
                return Err(trap(DIV0, "chia cho 0", span));
            }
            Ok(Value::Float(a as f64 / b as f64))
        }
        (Rem, Value::Duration(a), Value::Int(b)) => {
            if b == 0 {
                return Err(trap(DIV0, "lấy dư cho 0", span));
            }
            Ok(Value::Duration(a % b))
        }
        // percent (phòng thủ — các ca trên đã xử lý sớm; giữ cùng kiểm miền VD-243)
        (Add, Value::Percent(a), Value::Percent(b)) => Ok(Value::Percent(pct_moi(a + b, span)?)),
        (Sub, Value::Percent(a), Value::Percent(b)) => Ok(Value::Percent(pct_moi(a - b, span)?)),
        // còn lại: chưa hỗ trợ ở VS0
        (op, l, r) => Err(trap(
            RT,
            format!(
                "`{op:?}` chưa hỗ trợ cho `{}` và `{}` ở VS0",
                l.show(),
                r.show()
            ),
            span,
        )),
    }
}

pub fn un(op: vis_hir::hir::UnOp, v: Value, span: Span) -> vis_core::trap::R<Value> {
    match op {
        vis_hir::hir::UnOp::Neg => match v {
            Value::Int(i) => i
                .checked_neg()
                .map(Value::Int)
                .ok_or_else(|| overflow(span)),
            Value::Float(f) => Ok(Value::Float(-f)),
            other => Err(trap(
                RT,
                format!("`-` không dùng được cho `{}`", other.show()),
                span,
            )),
        },
        vis_hir::hir::UnOp::Not => Ok(Value::Bool(!v.truthy(span)?)),
    }
}
