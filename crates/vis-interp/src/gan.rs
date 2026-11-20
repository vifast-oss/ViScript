// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-56b (VD-97)**: gán **trường** / **chỉ số** — tách khỏi `interp.rs` cho luật **R1**.
//!
//! Ngữ nghĩa **đúng** như `Instr::With`/`Instr::Set` của VM (cùng thông điệp ⇒ interp ↔ `--vm`
//! **cùng byte**); thiếu trường / sai kiểu / ngoài phạm vi ⇒ `VS0501`.

use vis_syntax::Span;

use crate::interp::{Abort, Trap, IDX, R, RT};
use crate::value::eq;
use crate::value::Value;

/// **ST-56b (VD-97)**: `x.f = v` ⇒ `x = x with f: v` — **đúng** ngữ nghĩa `Instr::With` của VM
/// (cùng thông điệp để interp ↔ `--vm` **cùng byte**); thiếu trường/không phải bản ghi ⇒ `VS0501`.
pub(crate) fn gan_truong(base: Value, name: &str, v: Value, span: Span) -> R<Value> {
    match base {
        Value::Record(ten, mut fs) => match fs.iter_mut().find(|(k, _)| k == name) {
            Some(slot) => {
                slot.1 = v;
                Ok(Value::Record(ten, fs))
            }
            None => Err(Abort::Trap(Trap::new(
                RT,
                format!("bản ghi `{ten}` không có trường `{name}`"),
                span,
            ))),
        },
        other => Err(Abort::Trap(Trap::new(
            RT,
            format!("`with` không dùng được cho `{}`", other.show()),
            span,
        ))),
    }
}

/// **ST-56b (VD-97)**: `x[i] = v` — **đúng** ngữ nghĩa `Instr::Set` của VM (list: tại chỗ theo chỉ
/// số, ngoài phạm vi ⇒ `VS0501`; map: thay khoá **hoặc thêm** khoá mới).
pub(crate) fn gan_chi_so(base: Value, idx: Value, v: Value, span: Span) -> R<Value> {
    match (base, idx) {
        (Value::List(mut xs), Value::Int(i)) => {
            if (i as usize) < xs.len() {
                xs[i as usize] = v;
                Ok(Value::List(xs))
            } else {
                Err(Abort::Trap(Trap::new(
                    RT,
                    format!("chỉ số {i} ngoài phạm vi"),
                    span,
                )))
            }
        }
        (Value::Map(mut kv), k) => {
            if let Some(e) = kv.iter_mut().find(|(kk, _)| eq(kk, &k)) {
                e.1 = v;
            } else {
                kv.push((k, v));
            }
            Ok(Value::Map(kv))
        }
        _ => Err(Abort::Trap(Trap::new(RT, "gán chỉ số không hợp lệ", span))),
    }
}

// ---------------------------------------------------------------------------
// **VD-268** — gán **đường** (`a.b.f`, `xs[0].f`, `m["k"].f`, `a.bs[i].f`, …)
// ---------------------------------------------------------------------------

/// Một bước của đường gán, từ **gốc** ra **lá**; chỉ số/khoá đã đánh giá **một lần**.
pub(crate) enum Buoc {
    Truong(String),
    ChiSo(Value),
}

/// Đọc trường — **nguồn duy nhất** cho cả truy cập thường lẫn đường gán (VD-268), để thông điệp
/// `VS0501` không lệch giữa hai đường.
pub(crate) fn doc_truong(b: Value, name: &str, span: Span) -> R<Value> {
    match b {
        Value::Record(_, fields) => fields
            .into_iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v)
            .ok_or_else(|| {
                Abort::Trap(Trap::new(
                    RT,
                    format!("bản ghi không có trường `{name}`"),
                    span,
                ))
            }),
        // **VD-204 (1)**: `Some(v)` truy cập trường như `v` (tương thích `Value::Opt` cũ).
        Value::Enum(ea, va, args) if ea == "Option" && va == "Some" && args.len() == 1 => {
            let mut args = args;
            doc_truong(args.pop().expect("Some luôn có một đối số"), name, span)
        }
        // **Parity interp ↔ VM**: field trên `Enum` ⇒ `VS0501`; ngoại lệ atom biến thể rỗng.
        Value::Enum(en, variant, args) => {
            if variant.is_empty() && args.is_empty() {
                Ok(Value::Enum(en, name.to_string(), Vec::new()))
            } else {
                Err(Abort::Trap(Trap::new(
                    RT,
                    format!("`{name}` không phải trường"),
                    span,
                )))
            }
        }
        other => Err(Abort::Trap(Trap::new(
            RT,
            format!("`{}` không có trường `{name}`", other.show()),
            span,
        ))),
    }
}

/// Đọc chỉ số — **cùng mã/thông điệp** như đọc thường (`VS0503`: âm/ngoài phạm vi/khoá thiếu).
pub(crate) fn doc_chi_so(b: Value, i: Value, span: Span) -> R<Value> {
    match (b, i) {
        (Value::List(xs), Value::Int(n)) => {
            let len = xs.len() as i64;
            if n < 0 {
                return Err(Abort::Trap(Trap::new(
                    IDX,
                    format!("chỉ số âm `{n}` (dùng `.last()` hoặc `xs[i]?`)"),
                    span,
                )));
            }
            xs.get(n as usize).cloned().ok_or_else(|| {
                Abort::Trap(Trap::new(
                    IDX,
                    format!("chỉ số {n} ngoài phạm vi (độ dài {len})"),
                    span,
                ))
            })
        }
        (Value::Map(kv), k) => kv
            .iter()
            .find(|(kk, _)| kk.eq_value(&k))
            .map(|(_, v)| v.clone())
            .ok_or_else(|| {
                Abort::Trap(Trap::new(
                    IDX,
                    format!("khoá `{}` không có trong map", k.show()),
                    span,
                ))
            }),
        (Value::Bytes(bs), Value::Int(n)) => {
            let len = bs.len() as i64;
            if n < 0 || n >= len {
                return Err(Abort::Trap(Trap::new(
                    IDX,
                    format!("chỉ số {n} ngoài phạm vi (độ dài {len})"),
                    span,
                )));
            }
            Ok(Value::Int(bs[n as usize] as i64))
        }
        (other, idx) => Err(Abort::Trap(Trap::new(
            RT,
            format!("không chỉ số được `{}` bằng `{}`", other.show(), idx.show()),
            span,
        ))),
    }
}

/// Đọc **lá** của đường (dùng cho gán ghép: `a.b.n += 1` đọc `n` cũ trước vế phải).
pub(crate) fn doc_duong(base: &Value, duong: &[Buoc], span: Span) -> R<Value> {
    let mut cur = base.clone();
    for b in duong {
        cur = match b {
            Buoc::Truong(n) => doc_truong(cur, n, span)?,
            Buoc::ChiSo(i) => doc_chi_so(cur, i.clone(), span)?,
        };
    }
    Ok(cur)
}

/// Ghi **lá** rồi dựng lại **từng tầng** (giá trị): trả gốc mới. Không aliasing với bản sao cũ.
pub(crate) fn gan_duong(base: Value, duong: &[Buoc], v: Value, span: Span) -> R<Value> {
    let Some((b, rest)) = duong.split_first() else {
        return Ok(v);
    };
    if rest.is_empty() {
        return match b {
            Buoc::Truong(n) => gan_truong(base, n, v, span),
            Buoc::ChiSo(i) => gan_chi_so(base, i.clone(), v, span),
        };
    }
    let con = match b {
        Buoc::Truong(n) => doc_truong(base.clone(), n, span)?,
        Buoc::ChiSo(i) => doc_chi_so(base.clone(), i.clone(), span)?,
    };
    let moi = gan_duong(con, rest, v, span)?;
    match b {
        Buoc::Truong(n) => gan_truong(base, n, moi, span),
        Buoc::ChiSo(i) => gan_chi_so(base, i.clone(), moi, span),
    }
}
