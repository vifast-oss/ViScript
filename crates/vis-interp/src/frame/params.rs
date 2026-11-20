// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-259** — gắn tham số + default cho cả hàm tên, `main`, và task `async`.
//!
//! Tách khỏi `frame.rs` để giữ luật R1 (≤1500 dòng/tệp). Đây là module con của `frame` nên
//! thấy `Interp`/`IFrame`/`eval_expr_in`/`Env` qua `super::*`; API không đổi.
use super::*;

/// Số lớp `Opt` của một kiểu (dùng ở biên `let`/`var`/tham số/trả về).
pub(super) fn opt_depth_ty(t: &vis_types::Type) -> usize {
    let mut d = 0;
    let mut cur = t;
    while let vis_types::Type::Opt(inner) = cur {
        d += 1;
        cur = inner;
    }
    d
}

/// **VD-259**: gắn tham số + default vào một env mới — **một** helper dùng chung cho hàm tên
/// (`dispatch`), `main`, và thân task `async`. Nhờ vậy default không bị bỏ qua ở hai đường sau.
#[allow(clippy::too_many_arguments)]
pub(super) fn env_for_params<'a>(
    it: &mut Interp<'a>,
    params: &'a [HParam],
    positional: &[Value],
    named: &[(String, Value)],
    globals: &Env,
    mutable: &Rc<RefCell<HashSet<u32>>>,
    ten: &str,
    span: Span,
) -> R<Env> {
    let env = Rc::new(RefCell::new(globals.borrow().clone()));
    for (i, p) in params.iter().enumerate() {
        if p.variadic {
            let con: Vec<Value> = positional.iter().skip(i).cloned().collect();
            env.borrow_mut().insert(p.sym, Value::List(con));
            continue;
        }
        let v = positional.get(i).cloned().or_else(|| {
            named
                .iter()
                .find(|(n, _)| *n == p.name)
                .map(|(_, v)| v.clone())
        });
        let v = match v {
            Some(v) => v,
            None => match &p.default {
                Some(d) => eval_expr_in(it, d, &env, mutable)?,
                None => {
                    return Err(Abort::Trap(Trap::new(
                        RT,
                        format!("thiếu đối số `{}` của `{ten}`", p.name),
                        span,
                    )))
                }
            },
        };
        let v = match &p.ty {
            vis_types::Type::Opt(_) => crate::lift::lift_opt(v, opt_depth_ty(&p.ty)),
            _ => v,
        };
        if p.is_var_self {
            mutable.borrow_mut().insert(p.sym);
        }
        env.borrow_mut().insert(p.sym, v);
    }
    Ok(env)
}
