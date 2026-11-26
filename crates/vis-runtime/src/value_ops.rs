// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-316a (VD-292 §4.1)** — **một nguồn** thao tác `Value` cho **cả** `vis-interp` và `vis-vm`.
//!
//! Trước lát này, `vis-vm/src/vm/vm_helpers.rs` (`field`/`index`/`index_opt`/`set`/`with`) và
//! `vis-interp/src/gan.rs` (`doc_truong`/`doc_chi_so`/`gan_truong`/`gan_chi_so`/`gan_duong`) là
//! **hai bản sao** của cùng ngữ nghĩa, phải sửa tay từng chuỗi `VS0501`/`VS0503` ở hai nơi.
//! Module này giữ **một** bản; hai engine chỉ còn lớp mỏng gọi vào đây.
//!
//! ## Vì sao `dat`/`voi` nhận `&mut Value`
//!
//! `VD-292` §3.1 chỉ ra: bản cũ nhận `&Value` rồi `clone()` trước `Rc::make_mut`, nên khi ô biến
//! gốc vẫn giữ một bản `Rc` (`strong_count >= 2`) thì **mọi** lần ghi đều chép sâu `O(N)`.
//! Nay phép ghi sửa **tại chỗ** trên giá trị đã tách khỏi ô (xem `Th::tach_o`/`Interp::tach_o`):
//! ô được tạm đặt `Unit` trong lúc ghi ⇒ `Rc::make_mut` thấy `strong_count == 1` ⇒ cập nhật
//! `O(1)`, **không** đổi định dạng bytecode VBC1.

use std::rc::Rc;

use vis_core::trap::{Abort, Trap, R};
use vis_core::value::{Name, Value};
use vis_syntax::Span;

use crate::codes::{IDX, RT};

/// Một bước của **đường gán** (`a.b.f`, `xs[0].f`, `m["k"].f`, …), từ gốc ra lá.
///
/// **VD-268**: chỉ số/khoá đã đánh giá **một lần** ở chỗ dựng đường.
#[derive(Clone, Debug)]
pub enum Buoc {
    Truong(String),
    ChiSo(Value),
}

fn loi_truong(name: &str, span: Span) -> Abort {
    Abort::Trap(Trap::new(
        RT,
        format!("bản ghi không có trường `{name}`"),
        span,
    ))
}

/// Đọc trường **không sở hữu** (đường VM: `Instr::Field`/`?.`) — tự bóc `Some(x)` như interp.
pub fn truong(v: &Value, name: &str, span: Span) -> R<Value> {
    if let Some(x) = v.opt_payload() {
        return truong(x, name, span);
    }
    match v {
        Value::Record(_, fs) => fs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
            .ok_or_else(|| loi_truong(name, span)),
        // **VD-267/VD-204**: enum bộ số (`e.0`) và atom (`e.Ten`) — hợp nhất hành vi hai engine.
        Value::Enum(en, variant, args) => {
            if let Ok(i) = name.parse::<usize>() {
                return args.get(i).cloned().ok_or_else(|| {
                    Abort::Trap(Trap::new(RT, format!("không có đối số thứ {i}"), span))
                });
            }
            if variant.is_empty() && args.is_empty() {
                return Ok(Value::Enum(en.clone(), Name::new(name), Vec::new()));
            }
            Err(Abort::Trap(Trap::new(
                RT,
                format!("`{name}` không phải trường"),
                span,
            )))
        }
        other => Err(Abort::Trap(Trap::new(
            RT,
            format!("`{}` không có trường `{name}`", other.show()),
            span,
        ))),
    }
}

/// **ST-56b (VD-97)** — đọc trường **sở hữu** (đường interp): `Some(x)` bóc, `Enum` rỗng ⇒ atom
/// mới, còn lại cùng thông điệp `VS0501` với [`truong`].
pub fn doc_truong(b: Value, name: &str, span: Span) -> R<Value> {
    match b {
        Value::Record(_, fields) => fields
            .into_iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v)
            .ok_or_else(|| loi_truong(name, span)),
        // **VD-204 (1)**: `Some(v)` truy cập trường như `v`.
        Value::Enum(ea, va, args) if ea == "Option" && va == "Some" && args.len() == 1 => {
            let mut args = args;
            doc_truong(args.pop().expect("Some luôn có một đối số"), name, span)
        }
        Value::Enum(en, variant, args) => {
            if let Ok(i) = name.parse::<usize>() {
                return args.into_iter().nth(i).ok_or_else(|| {
                    Abort::Trap(Trap::new(RT, format!("không có đối số thứ {i}"), span))
                });
            }
            if variant.is_empty() && args.is_empty() {
                Ok(Value::Enum(en, Name::new(name), Vec::new()))
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

/// Đọc chỉ số — **cùng mã/thông điệp** `VS0503` cho cả hai engine.
pub fn chi_so(base: &Value, idx: &Value, span: Span) -> R<Value> {
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
        (Value::Map(kv), k) => kv.get(k).cloned().ok_or_else(|| {
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

/// Như [`chi_so`] nhưng **không** lỗi: `xs[i]?` trả `None` khi âm/ngoài phạm vi/khoá thiếu.
pub fn chi_so_opt(base: &Value, idx: &Value) -> Value {
    match (base, idx) {
        (Value::List(xs), Value::Int(i)) if *i >= 0 => Value::opt(xs.get(*i as usize).cloned()),
        (Value::Bytes(bs), Value::Int(i)) if *i >= 0 => {
            Value::opt(bs.get(*i as usize).map(|b| Value::Int(*b as i64)))
        }
        (Value::Map(kv), k) => Value::opt(kv.get(k).cloned()),
        _ => Value::none(),
    }
}

/// `x[i] = v` — sửa **tại chỗ**; list phải trong phạm vi, map thêm/cập nhật **last-wins**.
pub fn dat(base: &mut Value, idx: &Value, v: Value, span: Span) -> R<()> {
    match (base, idx) {
        (Value::List(xs), Value::Int(i)) => {
            if *i >= 0 && (*i as usize) < xs.len() {
                Rc::make_mut(xs)[*i as usize] = v;
                Ok(())
            } else {
                Err(Abort::Trap(Trap::new(
                    RT,
                    format!("chỉ số {i} ngoài phạm vi"),
                    span,
                )))
            }
        }
        (Value::Map(kv), k) => {
            Rc::make_mut(kv).insert(k.clone(), v);
            Ok(())
        }
        _ => Err(Abort::Trap(Trap::new(RT, "gán chỉ số không hợp lệ", span))),
    }
}

/// `x.f = v` — sửa **tại chỗ** đúng trường đã có; thiếu trường ⇒ `VS0501`.
pub fn dat_truong(base: &mut Value, name: &str, v: Value, span: Span) -> R<()> {
    match base {
        Value::Record(ten, fs) => match fs.iter_mut().find(|(k, _)| k == name) {
            Some(slot) => {
                slot.1 = v;
                Ok(())
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

/// `x with f: v, …` — kiểm **đủ** trường trước khi sửa để lỗi không để lại nửa vời.
pub fn voi(base: &mut Value, pairs: Vec<(String, Value)>, span: Span) -> R<()> {
    let Value::Record(ten, fs) = base else {
        return Err(Abort::Trap(Trap::new(
            RT,
            format!("`with` không dùng được cho `{}`", base.show()),
            span,
        )));
    };
    for (k, _) in &pairs {
        if !fs.iter().any(|(kk, _)| kk == k) {
            return Err(Abort::Trap(Trap::new(
                RT,
                format!("bản ghi `{ten}` không có trường `{k}`"),
                span,
            )));
        }
    }
    for (k, v) in pairs {
        if let Some(e) = fs.iter_mut().find(|(kk, _)| *kk == k) {
            e.1 = v;
        }
    }
    Ok(())
}

/// Đọc **lá** của đường (dùng cho gán ghép `a.b.n += 1`).
pub fn doc_duong(base: &Value, duong: &[Buoc], span: Span) -> R<Value> {
    let mut cur = base.clone();
    for b in duong {
        cur = match b {
            Buoc::Truong(n) => doc_truong(cur, n, span)?,
            Buoc::ChiSo(i) => chi_so(&cur, i, span)?,
        };
    }
    Ok(cur)
}

/// Ghi **lá** rồi dựng lại từng tầng; tầng đầu sửa **tại chỗ** khi có thể (không aliasing).
///
/// Hành vi/chuỗi lỗi giữ nguyên bản `vis-interp::gan::gan_duong` cũ.
pub fn gan_duong(base: &mut Value, duong: &[Buoc], v: Value, span: Span) -> R<()> {
    let Some((b, rest)) = duong.split_first() else {
        *base = v;
        return Ok(());
    };
    if rest.is_empty() {
        return match b {
            Buoc::Truong(n) => dat_truong(base, n, v, span),
            Buoc::ChiSo(i) => dat(base, i, v, span),
        };
    }
    // Tầng giữa: ưu tiên sửa tại chỗ (Record/List/Map); kiểu khác rơi về dựng lại như bản cũ.
    match (b, &mut *base) {
        (Buoc::Truong(n), Value::Record(_, fs)) => {
            let slot = fs
                .iter_mut()
                .find(|(k, _)| k == n)
                .ok_or_else(|| loi_truong(n, span))?;
            gan_duong(&mut slot.1, rest, v, span)
        }
        (Buoc::ChiSo(Value::Int(i)), Value::List(xs)) if *i >= 0 && (*i as usize) < xs.len() => {
            gan_duong(&mut Rc::make_mut(xs)[*i as usize], rest, v, span)
        }
        (Buoc::ChiSo(k), Value::Map(kv)) => {
            let mut con = kv.get(k).cloned().ok_or_else(|| {
                Abort::Trap(Trap::new(
                    IDX,
                    format!("khoá `{}` không có trong map", k.show()),
                    span,
                ))
            })?;
            gan_duong(&mut con, rest, v, span)?;
            Rc::make_mut(kv).insert(k.clone(), con);
            Ok(())
        }
        // Đường qua enum/`Some(x)`/…: dựng lại như bản cũ để giữ đúng ngữ nghĩa.
        (Buoc::Truong(n), _) => {
            let con = doc_truong(base.clone(), n, span)?;
            let moi = {
                let mut c = con;
                gan_duong(&mut c, rest, v, span)?;
                c
            };
            dat_truong(base, n, moi, span)
        }
        (Buoc::ChiSo(i), _) => {
            let con = chi_so(base, i, span)?;
            let moi = {
                let mut c = con;
                gan_duong(&mut c, rest, v, span)?;
                c
            };
            dat(base, i, moi, span)
        }
    }
}
