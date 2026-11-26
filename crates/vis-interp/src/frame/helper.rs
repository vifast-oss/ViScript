// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-318h (VD-298)** — helper thuần của frame: dựng work khối/if, khớp pattern,
//! truy cập trường/chỉ số, tên kiểu, lấy đối số khỏi `vals`.
//!
//! Tách **nguyên văn** từ `frame.rs` (R0, 0 đổi hành vi); dùng chung qua `super::*`.

use super::*;

/// Trả work của một khối theo **thứ tự thực thi**: ResetLast rồi Exec cho mỗi câu.
/// `append_work` chạy theo đúng thứ tự phần tử trong vec (phần tử đầu chạy trước).
pub(super) fn block_works<'a>(body: &'a [HStmt]) -> Vec<Work<'a>> {
    let mut w = Vec::with_capacity(body.len() * 2);
    for s in body.iter() {
        w.push(Work::ResetLast);
        w.push(Work::Exec(s));
    }
    w
}

/// Vec work **sẵn sàng gán trực tiếp** vào `cf.work` (LIFO — đảo thứ tự thực thi).
pub(super) fn initial_work<'a>(body: &'a [HStmt]) -> Vec<Work<'a>> {
    let mut w = block_works(body);
    w.reverse();
    w
}

/// Nối `works` vào đỉnh `cf.work` sao cho chạy theo thứ tự trong `works`.
pub(super) fn append_work<'a>(cf: &mut CallFrame<'a>, mut works: Vec<Work<'a>>) {
    while let Some(w) = works.pop() {
        cf.work.push(w);
    }
}

/// Khớp pattern và ghi binding vào `env` (mirror `pat.rs` nhưng dùng env của khung).
pub(super) fn bind_pat_env(pat: &HPat, v: &Value, env: &Env) -> bool {
    match pat {
        HPat::Wild { .. } | HPat::Rest { .. } => true,
        HPat::Bind { sym, .. } => {
            env.borrow_mut().insert(*sym, v.clone());
            true
        }
        HPat::Lit { lit, .. } => lit_value(lit).map(|l| l.eq_value(v)).unwrap_or(false),
        HPat::Or { alts, .. } => {
            for a in alts {
                let snap = env.borrow().clone();
                if bind_pat_env(a, v, env) {
                    return true;
                }
                *env.borrow_mut() = snap;
            }
            false
        }
        HPat::Ctor { path, args, .. } => {
            let name = path.last().cloned().unwrap_or_default();
            let low = name.to_ascii_lowercase();
            match v {
                Value::Enum(__en) => {
                    let (_, variant, vargs) = (&__en.enum_name, &__en.variant, &__en.args);
                    {
                        if variant != &name {
                            return false;
                        }
                        bind_all_env(args, vargs, env)
                    }
                }
                Value::Bool(b) => match low.as_str() {
                    "true" => *b,
                    "false" => !*b,
                    _ => false,
                },
                _ => false,
            }
        }
        HPat::Tuple { items, .. } => match v {
            Value::Unit if items.is_empty() => true,
            Value::List(xs) if xs.len() == items.len() => items
                .iter()
                .zip(xs.iter())
                .all(|(p, x)| bind_pat_env(p, x, env)),
            _ => false,
        },
        HPat::List { items, rest, .. } => match v {
            Value::List(xs) if xs.len() >= items.len() => {
                for (p, x) in items.iter().zip(xs.iter()) {
                    if !bind_pat_env(p, x, env) {
                        return false;
                    }
                }
                if let Some(r) = rest {
                    let tail = Value::list(xs[items.len()..].to_vec());
                    bind_pat_env(r, &tail, env)
                } else {
                    xs.len() == items.len()
                }
            }
            _ => false,
        },
        HPat::Record { fields, .. } => match v {
            Value::Record(r) => {
                let vfields = &r.fields;
                for (n, sub) in fields {
                    if n.is_empty() {
                        continue;
                    }
                    let Some((_, val)) = vfields.iter().find(|(f, _)| f == n) else {
                        return false;
                    };
                    if let Some(p) = sub {
                        if !bind_pat_env(p, val, env) {
                            return false;
                        }
                    }
                }
                true
            }
            _ => false,
        },
        HPat::Range {
            lo, hi, inclusive, ..
        } => {
            let lit_of = |p: &HPat| -> Option<Value> {
                if let HPat::Lit { lit, .. } = p {
                    lit_value(lit).ok()
                } else {
                    None
                }
            };
            let (Some(a0), Some(b0)) = (
                lo.as_ref().and_then(|p| lit_of(p)),
                hi.as_ref().and_then(|p| lit_of(p)),
            ) else {
                return false;
            };
            let (Some(a), Some(b)) = (v.cmp_value(&a0), v.cmp_value(&b0)) else {
                return false;
            };
            let ok_lo = a != std::cmp::Ordering::Less;
            let ok_hi = if *inclusive {
                b != std::cmp::Ordering::Greater
            } else {
                b == std::cmp::Ordering::Less
            };
            ok_lo && ok_hi
        }
    }
}

pub(super) fn bind_all_env(pats: &[HPat], vals: &[Value], env: &Env) -> bool {
    if pats.len() != vals.len() {
        return false;
    }
    pats.iter()
        .zip(vals.iter())
        .all(|(p, v)| bind_pat_env(p, v, env))
}

/// Truy cập trường (mirror `Interp::field_of`).
pub(super) fn field_of(b: Value, name: &str, span: Span) -> R<Value> {
    match b {
        Value::Record(r) => r
            .fields
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.clone())
            .ok_or_else(|| {
                Abort::Trap(Trap::new(
                    RT,
                    format!("bản ghi không có trường `{name}`"),
                    span,
                ))
            }),
        Value::Enum(e) if e.enum_name == "Option" && e.variant == "Some" && e.args.len() == 1 => {
            field_of(
                e.args.get(0).expect("Some luôn có một đối số").clone(),
                name,
                span,
            )
        }
        Value::Enum(e) => {
            if e.variant.is_empty() && e.args.is_empty() {
                Ok(Value::enum_val(
                    e.enum_name.clone(),
                    Name::new(name),
                    Vec::new(),
                ))
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

/// Chỉ số (mirror `Interp::index_of`).
pub(super) fn index_of(b: Value, i: Value, span: Span) -> R<Value> {
    match (b, i) {
        (Value::List(xs), Value::Int(n)) => {
            let len = xs.len() as i64;
            if n < 0 {
                return Err(Abort::Trap(Trap::new(
                    crate::interp::IDX,
                    format!("chỉ số âm `{n}` (dùng `.last()` hoặc `xs[i]?`)"),
                    span,
                )));
            }
            xs.get(n as usize).cloned().ok_or_else(|| {
                Abort::Trap(Trap::new(
                    crate::interp::IDX,
                    format!("chỉ số {n} ngoài phạm vi (độ dài {len})"),
                    span,
                ))
            })
        }
        (Value::Map(kv), k) => kv.get(&k).cloned().ok_or_else(|| {
            Abort::Trap(Trap::new(
                crate::interp::IDX,
                format!("khoá `{}` không có trong map", k.show()),
                span,
            ))
        }),
        (Value::Bytes(bs), Value::Int(n)) => {
            let len = bs.len() as i64;
            if n < 0 || n >= len {
                return Err(Abort::Trap(Trap::new(
                    crate::interp::IDX,
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

/// Tên kiểu record/enum từ `TypeTable` (mirror `Interp::type_name`).
pub(super) fn type_name(it: &Interp<'_>, ty: &vis_hir::Type) -> String {
    match ty {
        vis_hir::Type::Record(id) => it
            .types
            .rec(*id)
            .and_then(|r| r.name.clone())
            .unwrap_or_default(),
        vis_hir::Type::Enum(id) => it
            .types
            .enum_def(*id)
            .map(|e| e.name.clone())
            .unwrap_or_default(),
        _ => String::new(),
    }
}
/// Dựng danh sách work cho if/elif/else (dùng cho cả stmt lẫn expr).
pub(super) fn if_work<'a>(
    cond: &'a HCond,
    then: &'a [HStmt],
    elifs: &'a [(HCond, Vec<HStmt>)],
    else_: Option<&'a [HStmt]>,
    span: Span,
    stmt_mode: bool,
) -> Vec<Work<'a>> {
    let branch = |body: &'a [HStmt]| -> Vec<Work<'a>> {
        if stmt_mode {
            block_works(body)
        } else {
            vec![Work::EvalBlock(body)]
        }
    };
    let mut on_false: Vec<Work<'a>> = match else_ {
        Some(b) => branch(b),
        None => {
            if stmt_mode {
                vec![Work::ResetLast]
            } else {
                vec![Work::PushLast] // else rỗng ⇒ Unit
            }
        }
    };
    for (c, body) in elifs.iter().rev() {
        let on_true = branch(body);
        on_false = vec![Work::EvalCond {
            cond: c,
            on_true,
            on_false,
            span,
        }];
    }
    let on_true = branch(then);
    vec![Work::EvalCond {
        cond,
        on_true,
        on_false,
        span,
    }]
}

pub(super) fn args_works<'a>(
    args: &'a [HExpr],
    named: &'a [(String, HExpr)],
    span: Span,
    cont: Vec<Work<'a>>,
) -> Work<'a> {
    Work::EvalArgsCont {
        args,
        named,
        idx: 0,
        span,
        cont,
    }
}
pub(super) fn run_arm_body<'a>(frame: &mut IFrame<'a>, arm: &'a HArm, stmt_mode: bool) {
    if stmt_mode {
        append_work(frame.cur(), block_works(&arm.body));
    } else {
        frame.cur().work.push(Work::EvalBlock(&arm.body));
    }
}

pub(super) fn pop_n(frame: &mut IFrame<'_>, n: usize) -> R<Vec<Value>> {
    let mut vs = Vec::with_capacity(n);
    for _ in 0..n {
        vs.push(frame.pop_val()?);
    }
    Ok(vs)
}

/// Lấy `(pos, named)` đã eval từ `vals` theo thứ tự nguồn; `names` là tên đối số theo thứ tự.
#[allow(clippy::type_complexity)]
pub(super) fn take_call_args_names(
    frame: &mut IFrame<'_>,
    npos: usize,
    names: &[String],
) -> R<(Vec<Value>, Vec<(String, Value)>)> {
    let mut all = Vec::with_capacity(npos + names.len());
    for _ in 0..(npos + names.len()) {
        all.push(frame.pop_val()?);
    }
    all.reverse();
    let pos = all[..npos].to_vec();
    let named = all[npos..]
        .iter()
        .cloned()
        .zip(names.iter().cloned())
        .map(|(v, n)| (n, v))
        .collect();
    Ok((pos, named))
}

pub(super) fn underflow() -> Abort {
    Abort::Trap(Trap::new(
        RT,
        "frame test: ngăn xếp giá trị rỗng",
        Span::new(0, 0),
    ))
}
