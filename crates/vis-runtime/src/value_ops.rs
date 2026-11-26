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
use vis_core::value::Value;
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

/// **ST-317a F1** — con trỏ `Rc` nội bộ của **Map/List/Record** — nhận diện ô chia sẻ
/// (không chép dữ liệu).
///
/// Là **một nguồn** cho cả `vis-interp` (`Interp::tach_o`) và `vis-vm` (`Th::tach_dich`);
/// trước F1 chỉ có `Map`/`List` nên `Record` luôn bị `Rc::make_mut` chép sâu mỗi lần ghi.
pub fn ptr_rc(v: &Value) -> Option<*const ()> {
    match v {
        Value::Map(m) => Some(Rc::as_ptr(m) as *const ()),
        Value::List(l) => Some(Rc::as_ptr(l) as *const ()),
        Value::Record(r) => Some(Rc::as_ptr(r) as *const ()),
        _ => None,
    }
}

/// **ST-317a F1** — tạm tách ô `o` khỏi `Rc` **khi** ô còn trỏ cùng giá trị với `base`
/// (ô = `Unit` trong lúc ghi). Nhờ đó `Rc::make_mut` thấy `strong_count == 1` và cập nhật
/// **tại chỗ O(1)**; trả `true` nếu đã tách để chỗ gọi khôi phục ô sau khi ghi.
///
/// Ô **không** chia sẻ với `base` (ví dụ biến alias `b = a`) trả `false` và giữ nguyên —
/// đúng ngữ nghĩa giá trị: chỉ ô đích bị tách, alias vẫn giữ bản cũ.
pub fn tach_neu_cung_rc(o: &mut Value, base: &Value) -> bool {
    if let (Some(a), Some(b)) = (ptr_rc(o), ptr_rc(base)) {
        if a == b {
            *o = Value::Unit;
            return true;
        }
    }
    false
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
        Value::Record(r) => r
            .fields
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
            .ok_or_else(|| loi_truong(name, span)),
        // **VD-267/VD-204**: enum bộ số (`e.0`) và atom (`e.Ten`) — hợp nhất hành vi hai engine.
        Value::Enum(e) => {
            if let Ok(i) = name.parse::<usize>() {
                return e.args.get(i).cloned().ok_or_else(|| {
                    Abort::Trap(Trap::new(RT, format!("không có đối số thứ {i}"), span))
                });
            }
            if e.variant.is_empty() && e.args.is_empty() {
                return Ok(Value::enum_val(e.enum_name.clone(), name, Vec::new()));
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
        Value::Record(rc) => {
            let fields = match Rc::try_unwrap(rc) {
                Ok(r) => r.fields,
                Err(rc) => rc.fields.clone(),
            };
            fields
                .into_iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v)
                .ok_or_else(|| loi_truong(name, span))
        }
        // **VD-204 (1)**: `Some(v)` truy cập trường như `v`.
        Value::Enum(e) if e.enum_name == "Option" && e.variant == "Some" && e.args.len() == 1 => {
            let Some((_, _, mut args)) = (Value::Enum(e)).into_enum() else {
                unreachable!("vừa khớp Value::Enum")
            };
            doc_truong(args.pop().expect("Some luôn có một đối số"), name, span)
        }
        Value::Enum(e) => {
            let Some((en, variant, args)) = (Value::Enum(e)).into_enum() else {
                unreachable!("vừa khớp Value::Enum")
            };
            if let Ok(i) = name.parse::<usize>() {
                return args.into_iter().nth(i).ok_or_else(|| {
                    Abort::Trap(Trap::new(RT, format!("không có đối số thứ {i}"), span))
                });
            }
            if variant.is_empty() && args.is_empty() {
                Ok(Value::enum_val(en, name, Vec::new()))
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
        Value::Record(rc) => {
            if !rc.fields.iter().any(|(k, _)| k == name) {
                return Err(Abort::Trap(Trap::new(
                    RT,
                    format!("bản ghi `{}` không có trường `{name}`", rc.name),
                    span,
                )));
            }
            let r = Rc::make_mut(rc);
            if let Some(slot) = r.fields.iter_mut().find(|(k, _)| k == name) {
                slot.1 = v;
            }
            Ok(())
        }
        other => Err(Abort::Trap(Trap::new(
            RT,
            format!("`with` không dùng được cho `{}`", other.show()),
            span,
        ))),
    }
}

/// `x with f: v, …` — kiểm **đủ** trường trước khi sửa để lỗi không để lại nửa vời.
pub fn voi(base: &mut Value, pairs: Vec<(String, Value)>, span: Span) -> R<()> {
    let Value::Record(rc) = base else {
        return Err(Abort::Trap(Trap::new(
            RT,
            format!("`with` không dùng được cho `{}`", base.show()),
            span,
        )));
    };
    for (k, _) in &pairs {
        if !rc.fields.iter().any(|(kk, _)| kk == k) {
            return Err(Abort::Trap(Trap::new(
                RT,
                format!("bản ghi `{}` không có trường `{k}`", rc.name),
                span,
            )));
        }
    }
    // **ST-317a**: `x with f: v` sửa **tại chỗ** khi bản ghi không chia sẻ (`strong_count == 1`
    // sau khi engine tách ô đích); chia sẻ ⇒ CoW một lần, `b = a` vẫn giữ bản cũ.
    let r = Rc::make_mut(rc);
    for (k, v) in pairs {
        if let Some(e) = r.fields.iter_mut().find(|(kk, _)| *kk == k) {
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
        (Buoc::Truong(n), Value::Record(rc)) => {
            let i = rc
                .fields
                .iter()
                .position(|(k, _)| k == n)
                .ok_or_else(|| loi_truong(n, span))?;
            let r = Rc::make_mut(rc);
            gan_duong(&mut r.fields[i].1, rest, v, span)
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
