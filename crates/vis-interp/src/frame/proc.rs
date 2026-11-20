// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-274 / VD-255** — continuation stack (`process_work`) của frame evaluator.
//! Tách từ `frame.rs` cho luật R1 (≤1500 dòng/file). Mọi item dùng chung lấy qua `super`.
use super::*;
use crate::rt::fee::FeeKind;

/// Xử lý một work-item: tiêu thụ giá trị trên `vals`, đẩy continuation mới.
pub(super) fn process_work<'a>(it: &mut Interp<'a>, frame: &mut IFrame<'a>, w: Work<'a>) -> R<()> {
    match w {
        // ================= nền tảng =================
        Work::Eval(e) => eval_work(it, frame, e),
        Work::Exec(s) => exec_work(it, frame, s),
        Work::ApplyAwait(span) => {
            let v = frame.pop_val()?;
            let t = match v {
                Value::Task(t) => t,
                _ => {
                    return Err(Abort::Trap(Trap::new(
                        RT,
                        "`await` cần một `Task[T]`",
                        span,
                    )))
                }
            };
            match crate::rt::await_task(it, &t, span) {
                Ok(r) => frame.push_val(r),
                Err(Abort::Pending(p)) => {
                    frame.cur().work.push(Work::ApplyAwait(span));
                    frame.push_val(Value::Task(t));
                    return Err(Abort::Pending(p));
                }
                Err(e) => return Err(e),
            }
            Ok(())
        }
        Work::ApplyBin { op, span } => {
            let r = frame.pop_val()?;
            let l = frame.pop_val()?;
            let v = crate::ops::bin(op, l, r, span)?;
            frame.push_val(v);
            Ok(())
        }
        Work::EvalArgsCont {
            args,
            named,
            idx,
            span,
            cont,
        } => {
            if idx < args.len() {
                let cf = frame.cur();
                cf.work.push(Work::EvalArgsCont {
                    args,
                    named,
                    idx: idx + 1,
                    span,
                    cont,
                });
                cf.work.push(Work::Eval(&args[idx]));
                return Ok(());
            }
            if idx - args.len() < named.len() {
                let j = idx - args.len();
                let cf = frame.cur();
                cf.work.push(Work::EvalArgsCont {
                    args,
                    named,
                    idx: idx + 1,
                    span,
                    cont,
                });
                cf.work.push(Work::Eval(&named[j].1));
                return Ok(());
            }
            append_work(frame.cur(), cont);
            Ok(())
        }
        Work::Bind {
            pat,
            opt_depth,
            span,
        } => {
            // VD-257/D — E_ASSIGN ở điểm RHS đã xong, TRƯỚC khi ghi (VD-252 §3).
            if let Some(a) = crate::rt::fee::frame_charge(it, FeeKind::Assign, span) {
                return Err(a);
            }
            let v = frame.pop_val()?;
            let v = crate::lift::lift_opt(v, opt_depth);
            bind_pat_env(pat, &v, &frame.cur().env);
            Ok(())
        }
        Work::ApplyLetElse {
            pat,
            else_body,
            bind_sym,
            opt_depth,
            span,
        } => {
            if let Some(a) = crate::rt::fee::frame_charge(it, FeeKind::Assign, span) {
                return Err(a);
            }
            let v = frame.pop_val()?;
            let v = crate::lift::lift_opt(v, opt_depth);
            match v {
                Value::Enum(ref ea, ref va, ref ag) if ea == "Option" && va == "None" => {
                    frame.cur().work.push(Work::ExecBlock(else_body));
                    Ok(())
                }
                Value::Enum(ea, va, mut ag)
                    if (ea == "Result" && va == "Ok" || ea == "Option" && va == "Some")
                        && ag.len() == 1 =>
                {
                    bind_pat_env(pat, &ag.pop().unwrap(), &frame.cur().env);
                    Ok(())
                }
                Value::Enum(ea, va, mut ag) if ea == "Result" && va == "Err" && ag.len() == 1 => {
                    if let Some(sym) = bind_sym {
                        frame.cur().env.borrow_mut().insert(sym, ag.pop().unwrap());
                    }
                    frame.cur().work.push(Work::ExecBlock(else_body));
                    Ok(())
                }
                other => {
                    bind_pat_env(pat, &other, &frame.cur().env);
                    Ok(())
                }
            }
        }
        Work::SetVar {
            sym,
            opt_depth,
            span,
        } => {
            if let Some(a) = crate::rt::fee::frame_charge(it, FeeKind::Assign, span) {
                return Err(a);
            }
            let v = frame.pop_val()?;
            let v = crate::lift::lift_opt(v, opt_depth);
            frame.cur().env.borrow_mut().insert(sym, v);
            frame.mutable.borrow_mut().insert(sym);
            Ok(())
        }
        Work::SetLast => {
            let v = frame.pop_val()?;
            frame.cur().last = Some(v);
            Ok(())
        }
        Work::ResetLast => {
            frame.cur().last = None;
            Ok(())
        }
        Work::PushLast => {
            let v = frame.cur().last.take().unwrap_or(Value::Unit);
            frame.push_val(v);
            Ok(())
        }
        Work::DoReturn => {
            let v = frame.pop_val()?;
            {
                let cf = frame.cur();
                cf.last = Some(v);
                cf.work.clear();
                cf.loops.clear();
            }
            Ok(())
        }
        Work::Redispatch {
            callee,
            args,
            named,
            span,
        } => {
            for a in &args {
                frame.push_val(a.clone());
            }
            for (_, v) in &named {
                frame.push_val(v.clone());
            }
            match dispatch(it, frame, callee, args.clone(), named.clone(), span) {
                Ok(()) => {}
                Err(Abort::Pending(p)) => {
                    frame.cur().work.push(Work::Redispatch {
                        callee,
                        args,
                        named,
                        span,
                    });
                    return Err(Abort::Pending(p));
                }
                Err(e) => return Err(e),
            }
            Ok(())
        }

        // ================= Group A: điều khiển =================
        Work::ExecBlock(body) => {
            append_work(frame.cur(), block_works(body));
            Ok(())
        }
        Work::EvalBlock(body) => {
            let mut w = vec![Work::ResetLast];
            w.extend(block_works(body));
            w.push(Work::PushLast);
            append_work(frame.cur(), w);
            Ok(())
        }
        Work::EvalCond {
            cond,
            on_true,
            on_false,
            span,
        } => {
            match cond {
                HCond::Expr(e) => {
                    append_work(
                        frame.cur(),
                        vec![
                            Work::Eval(e),
                            Work::ApplyCond {
                                on_true,
                                on_false,
                                span,
                            },
                        ],
                    );
                }
                HCond::Let { pat, value, .. } => {
                    append_work(
                        frame.cur(),
                        vec![
                            Work::Eval(value),
                            Work::ApplyCondLet {
                                pat,
                                on_true,
                                on_false,
                            },
                        ],
                    );
                }
            }
            Ok(())
        }
        Work::ApplyCond {
            on_true,
            on_false,
            span,
        } => {
            let v = frame.pop_val()?;
            if v.truthy(span)? {
                append_work(frame.cur(), on_true);
            } else {
                append_work(frame.cur(), on_false);
            }
            Ok(())
        }
        Work::ApplyCondLet {
            pat,
            on_true,
            on_false,
        } => {
            let v = frame.pop_val()?;
            let env = frame.cur().env.clone();
            let snap = env.borrow().clone();
            if bind_pat_env(pat, &v, &env) {
                append_work(frame.cur(), on_true);
            } else {
                *env.borrow_mut() = snap;
                append_work(frame.cur(), on_false);
            }
            Ok(())
        }
        Work::LoopInit { kind, body, span } => {
            let k = match kind {
                LoopKindInit::While { cond } => LoopKindState::While { cond },
                LoopKindInit::For { pat, iter } => {
                    let it_v = frame.pop_val()?;
                    let items = crate::value::iter_items(it_v, iter.span())?;
                    LoopKindState::For { pat, items, idx: 0 }
                }
                LoopKindInit::Loop => LoopKindState::Loop,
            };
            let id = frame.cur().loops.len();
            frame.cur().loops.push(LoopState {
                kind: k,
                body,
                span,
            });
            frame.cur().work.push(Work::LoopStep { id });
            Ok(())
        }
        Work::LoopMark { id } => {
            frame.cur().work.push(Work::LoopStep { id });
            Ok(())
        }
        Work::LoopStep { id } => {
            // VD-252 quy tắc 4: E_LOOP MỘT lần cho mỗi lần thử tiến vòng, GỒM
            // lần cuối sai. `LoopStep` chạy đúng một lần mỗi lần thử guard.
            let lspan = frame.cur().loops.last().expect("loop state").span;
            if let Some(a) = crate::rt::fee::frame_charge(it, FeeKind::Loop, lspan) {
                return Err(a);
            }
            enum Step<'x> {
                While(&'x HCond, Span),
                For(&'x HPat, Option<Value>),
                Loop,
            }
            let step = {
                let cf = frame.cur();
                let state = cf.loops.last_mut().expect("loop state");
                let span = state.span;
                match &mut state.kind {
                    LoopKindState::While { cond } => Step::While(cond, span),
                    LoopKindState::For { pat, items, idx } => {
                        if *idx >= items.len() {
                            Step::For(pat, None)
                        } else {
                            let v = items[*idx].clone();
                            *idx += 1;
                            Step::For(pat, Some(v))
                        }
                    }
                    LoopKindState::Loop => Step::Loop,
                }
            };
            let cf = frame.cur();
            match step {
                Step::While(cond, span) => {
                    append_work(
                        cf,
                        vec![Work::EvalCond {
                            cond,
                            on_true: vec![Work::LoopEnter { id }],
                            on_false: vec![Work::LoopExit { id }],
                            span,
                        }],
                    );
                }
                Step::For(pat, Some(v)) => {
                    let env = cf.env.clone();
                    bind_pat_env(pat, &v, &env);
                    cf.work.push(Work::LoopEnter { id });
                }
                Step::For(_, None) => {
                    cf.work.push(Work::LoopExit { id });
                }
                Step::Loop => {
                    cf.work.push(Work::LoopEnter { id });
                }
            }
            Ok(())
        }
        Work::LoopEnter { id } => {
            let body = frame.cur().loops.last().expect("loop state").body;
            frame.cur().work.push(Work::LoopMark { id });
            append_work(frame.cur(), block_works(body));
            Ok(())
        }
        Work::LoopExit { id } => {
            let cf = frame.cur();
            while cf.loops.len() > id {
                cf.loops.pop();
            }
            cf.last = None;
            Ok(())
        }
        Work::DoBreak { span } => {
            let cf = frame.cur();
            if cf.loops.is_empty() {
                return Err(Abort::Trap(Trap::new(
                    RT,
                    "`break` ngoài vòng lặp (chỉ dùng trong `for`/`while`/`loop`)",
                    span,
                )));
            }
            let id = cf.loops.len() - 1;
            while let Some(w) = cf.work.pop() {
                if matches!(w, Work::LoopMark { id: m } if m == id) {
                    break;
                }
            }
            cf.work.push(Work::LoopExit { id });
            Ok(())
        }
        Work::DoContinue { span } => {
            let cf = frame.cur();
            if cf.loops.is_empty() {
                return Err(Abort::Trap(Trap::new(
                    RT,
                    "`continue` ngoài vòng lặp",
                    span,
                )));
            }
            let id = cf.loops.len() - 1;
            loop {
                match cf.work.last() {
                    Some(Work::LoopMark { id: m }) if *m == id => break,
                    Some(_) => {
                        cf.work.pop();
                    }
                    None => break,
                }
            }
            Ok(())
        }

        // ================= Group B: biểu thức & gọi =================
        Work::ApplyUn { op, span } => {
            let v = frame.pop_val()?;
            let r = crate::ops::un(op, v, span)?;
            frame.push_val(r);
            Ok(())
        }
        Work::ApplyField {
            name,
            optional,
            span,
        } => {
            let b = frame.pop_val()?;
            let (b, nang) = if optional {
                match b {
                    Value::Enum(ea, va, mut ag) if ea == "Option" => match va.as_str() {
                        "None" => {
                            frame.push_val(Value::none());
                            return Ok(());
                        }
                        "Some" if ag.len() == 1 => (ag.pop().unwrap(), true),
                        _ => (Value::Enum(ea, va, ag), false),
                    },
                    other => (other, false),
                }
            } else {
                (b, false)
            };
            let v = field_of(b, &name, span)?;
            frame.push_val(if nang { crate::lift::lift_opt(v, 1) } else { v });
            Ok(())
        }
        Work::ApplyIndex { span } => {
            let i = frame.pop_val()?;
            let b = frame.pop_val()?;
            let v = index_of(b, i, span)?;
            frame.push_val(v);
            Ok(())
        }
        Work::BuildList { n } => {
            let mut vs = pop_n(frame, n)?;
            vs.reverse();
            frame.push_val(Value::List(vs));
            Ok(())
        }
        Work::BuildSet { n } => {
            let mut vs = pop_n(frame, n)?;
            vs.reverse();
            let mut out: Vec<Value> = Vec::new();
            for v in vs {
                if !out.iter().any(|x| x.eq_value(&v)) {
                    out.push(v);
                }
            }
            frame.push_val(Value::List(out));
            Ok(())
        }
        Work::BuildTuple { n } => {
            let mut vs = pop_n(frame, n)?;
            vs.reverse();
            frame.push_val(Value::List(vs));
            Ok(())
        }
        Work::BuildMap { n } => {
            let mut flat = pop_n(frame, n * 2)?;
            flat.reverse();
            let mut kv: Vec<(Value, Value)> = Vec::new();
            let mut it = flat.into_iter();
            while let (Some(k), Some(v)) = (it.next(), it.next()) {
                if let Some(e) = kv.iter_mut().find(|(kk, _)| kk.eq_value(&k)) {
                    e.1 = v;
                } else {
                    kv.push((k, v));
                }
            }
            frame.push_val(Value::Map(kv));
            Ok(())
        }
        Work::EvalRecordFields { fields, idx, span } => {
            if idx >= fields.len() {
                return Ok(());
            }
            frame.cur().work.push(Work::EvalRecordFields {
                fields,
                idx: idx + 1,
                span,
            });
            frame.cur().work.push(Work::Eval(&fields[idx].1));
            Ok(())
        }
        Work::BuildRecord { name, fields, span } => {
            let vals = pop_n(frame, fields.len())?;
            let mut out = Vec::with_capacity(fields.len());
            for ((n, _), v) in fields.iter().zip(vals.into_iter().rev()) {
                out.push((n.clone(), v));
            }
            let _ = span;
            frame.push_val(Value::Record(name, out));
            Ok(())
        }
        Work::BuildEnum {
            name,
            variant,
            n,
            span,
        } => {
            let mut vs = pop_n(frame, n)?;
            vs.reverse();
            let _ = span;
            frame.push_val(Value::Enum(name, variant, vs));
            Ok(())
        }
        Work::ApplySome => {
            let v = frame.pop_val()?;
            frame.push_val(Value::some(v));
            Ok(())
        }
        Work::WithCollect { names, n, span } => {
            let mut vals = pop_n(frame, n)?;
            vals.reverse();
            let base = frame.pop_val()?;
            match base {
                Value::Record(name, mut cur) => {
                    for (n, nv) in names.into_iter().zip(vals) {
                        match cur.iter_mut().find(|(f, _)| *f == n) {
                            Some(slot) => slot.1 = nv,
                            None => {
                                return Err(Abort::Trap(Trap::new(
                                    RT,
                                    format!("bản ghi không có trường `{n}`"),
                                    span,
                                )))
                            }
                        }
                    }
                    frame.push_val(Value::Record(name, cur));
                    Ok(())
                }
                other => Err(Abort::Trap(Trap::new(
                    RT,
                    format!("`with` cần bản ghi, được `{}`", other.show()),
                    span,
                ))),
            }
        }
        Work::InterpParts {
            parts,
            idx,
            out,
            span,
        } => {
            if idx >= parts.len() {
                frame.push_val(Value::Str(out));
                return Ok(());
            }
            match &parts[idx] {
                InterpPart::Str(s) => {
                    frame.cur().work.push(Work::InterpParts {
                        parts,
                        idx: idx + 1,
                        out: format!("{out}{s}"),
                        span,
                    });
                }
                InterpPart::Expr(e) => {
                    frame.cur().work.push(Work::InterpPartVal {
                        parts,
                        idx,
                        out,
                        spec: None,
                        span,
                    });
                    frame.cur().work.push(Work::Eval(e));
                }
                InterpPart::ExprSpec { expr, spec } => {
                    frame.cur().work.push(Work::InterpPartVal {
                        parts,
                        idx,
                        out,
                        spec: Some(spec.clone()),
                        span,
                    });
                    frame.cur().work.push(Work::Eval(expr));
                }
            }
            Ok(())
        }
        Work::InterpPartVal {
            parts,
            idx,
            out,
            spec,
            span,
        } => {
            let v = frame.pop_val()?;
            let piece = match spec {
                Some(s) => crate::interp::dinh_dang(&v, &s, span)?,
                None => v.show(),
            };
            frame.cur().work.push(Work::InterpParts {
                parts,
                idx: idx + 1,
                out: format!("{out}{piece}"),
                span,
            });
            Ok(())
        }
        Work::ApplyShortCircuit { op, rhs, span } => {
            let l = frame.pop_val()?;
            let lt = l.truthy(span)?;
            match op {
                BinOp::And if !lt => frame.push_val(Value::Bool(false)),
                BinOp::Or if lt => frame.push_val(Value::Bool(true)),
                _ => {
                    frame.cur().work.push(Work::ApplyBoolRhs { span });
                    frame.cur().work.push(Work::Eval(rhs));
                }
            }
            Ok(())
        }
        Work::ApplyBoolRhs { span } => {
            let r = frame.pop_val()?;
            let rt = r.truthy(span)?;
            frame.push_val(Value::Bool(rt));
            Ok(())
        }
        Work::ApplyCoalesce { rhs } => {
            let l = frame.pop_val()?;
            if l.is_some() {
                frame.push_val(l.into_some().expect("vừa kiểm is_some"));
            } else if l.is_none() {
                frame.cur().work.push(Work::Eval(rhs));
            } else {
                frame.push_val(l);
            }
            Ok(())
        }
        Work::ApplyPipe { rhs, span } => {
            let l = frame.pop_val()?;
            frame.cur().work.push(Work::ApplyPipeCall { lhs: l, span });
            frame.cur().work.push(Work::Eval(rhs));
            Ok(())
        }
        Work::ApplyPipeCall { lhs, span } => {
            let f = frame.pop_val()?;
            call_value(it, frame, f, vec![lhs], Vec::new(), span)
        }
        Work::MatchArm {
            arms,
            idx,
            subj,
            stmt_mode,
            span,
        } => {
            if idx >= arms.len() {
                return Err(Abort::Trap(Trap::new(
                    RT,
                    "`match` không khớp nhánh nào (không xét hết bị lọt)",
                    span,
                )));
            }
            let arm = &arms[idx];
            let env = frame.cur().env.clone();
            let snap = env.borrow().clone();
            if !bind_pat_env(&arm.pat, &subj, &env) {
                *env.borrow_mut() = snap;
                frame.cur().work.push(Work::MatchArm {
                    arms,
                    idx: idx + 1,
                    subj,
                    stmt_mode,
                    span,
                });
                return Ok(());
            }
            if let Some(g) = &arm.guard {
                frame.cur().work.push(Work::MatchGuard {
                    arms,
                    idx,
                    subj,
                    stmt_mode,
                    snapshot: snap,
                    span,
                });
                frame.cur().work.push(Work::Eval(g));
                return Ok(());
            }
            run_arm_body(frame, arm, stmt_mode);
            Ok(())
        }
        Work::MatchGuard {
            arms,
            idx,
            subj,
            stmt_mode,
            snapshot,
            span,
        } => {
            let gv = frame.pop_val()?;
            if gv.truthy(span)? {
                run_arm_body(frame, &arms[idx], stmt_mode);
            } else {
                *frame.cur().env.borrow_mut() = snapshot;
                frame.cur().work.push(Work::MatchArm {
                    arms,
                    idx: idx + 1,
                    subj,
                    stmt_mode,
                    span,
                });
            }
            Ok(())
        }
        Work::MatchSubj {
            arms,
            stmt_mode,
            span,
        } => {
            let subj = frame.pop_val()?;
            frame.cur().work.push(Work::MatchArm {
                arms,
                idx: 0,
                subj,
                stmt_mode,
                span,
            });
            Ok(())
        }
        Work::ApplyNamedCall {
            callee,
            npos,
            names,
            span,
        } => {
            // E_CALL/E_BUILTIN đã charge tại nút Call trong `eval_work` (trước
            // đối số) ⇒ KHÔNG charge lại ở đây, kể cả resume qua `Redispatch`.
            let (pos, named_vals) = take_call_args_names(frame, npos, &names)?;
            match dispatch(it, frame, callee, pos.clone(), named_vals.clone(), span) {
                Ok(()) => Ok(()),
                Err(Abort::Pending(p)) => {
                    frame.cur().work.push(Work::Redispatch {
                        callee,
                        args: pos,
                        named: named_vals,
                        span,
                    });
                    Err(Abort::Pending(p))
                }
                Err(e) => Err(e),
            }
        }
        Work::ApplyValueCall { npos, names, span } => {
            // E_CALL đã charge tại nút Call — resume qua `RedispatchValue` không charge lại.
            let (pos, named) = take_call_args_names(frame, npos, &names)?;
            let f = frame.pop_val()?;
            match call_value(it, frame, f.clone(), pos.clone(), named.clone(), span) {
                Ok(()) => Ok(()),
                Err(Abort::Pending(p)) => {
                    frame.cur().work.push(Work::RedispatchValue {
                        f,
                        args: pos,
                        named,
                        span,
                    });
                    Err(Abort::Pending(p))
                }
                Err(e) => Err(e),
            }
        }
        Work::RedispatchValue {
            f,
            args,
            named,
            span,
        } => match call_value(it, frame, f.clone(), args.clone(), named.clone(), span) {
            Ok(()) => Ok(()),
            Err(Abort::Pending(p)) => {
                frame.cur().work.push(Work::RedispatchValue {
                    f,
                    args,
                    named,
                    span,
                });
                Err(Abort::Pending(p))
            }
            Err(e) => Err(e),
        },
        Work::ApplyMethodOrField {
            name,
            args,
            named,
            base,
            span,
        } => {
            let recv = frame.pop_val()?;
            if matches!(
                recv,
                Value::List(_) | Value::Str(_) | Value::Map(_) | Value::Chan(_)
            ) {
                let base_var = if let HExpr::Var { sym: Some(id), .. } = base {
                    Some(*id)
                } else {
                    None
                };
                frame.cur().work.push(args_works(
                    args,
                    &[],
                    span,
                    vec![Work::ApplyMethod {
                        name,
                        recv,
                        nargs: args.len(),
                        base_var,
                        span,
                    }],
                ));
            } else {
                let f = field_of(recv, &name, span)?;
                frame.push_val(f);
                frame.cur().work.push(args_works(
                    args,
                    named,
                    span,
                    vec![Work::ApplyValueCall {
                        npos: args.len(),
                        names: named.iter().map(|(n, _)| n.clone()).collect(),
                        span,
                    }],
                ));
            }
            Ok(())
        }
        Work::ApplyMethod {
            name,
            recv,
            nargs,
            base_var,
            span,
        } => {
            // E_BUILTIN đã charge tại nút Call (dạng Field) — resume qua
            // `RedispatchMethod` không charge lại.
            let mut got = pop_n(frame, nargs)?;
            got.reverse();
            match crate::builtin::method(it, recv.clone(), &name, got.clone(), span) {
                Ok(kq) => {
                    method_writeback(frame, &name, base_var, &kq);
                    frame.push_val(kq);
                    Ok(())
                }
                // Kênh đầy/rỗng ⇒ `chay_mot` báo Pending. Giữ **đối số đã eval** trong
                // continuation ⇒ resume thử lại méthode, KHÔNG eval lại đối số/side-effect.
                Err(Abort::Pending(p)) => {
                    frame.cur().work.push(Work::RedispatchMethod {
                        name,
                        recv,
                        args: got,
                        base_var,
                        span,
                    });
                    Err(Abort::Pending(p))
                }
                Err(e) => Err(e),
            }
        }
        Work::RedispatchMethod {
            name,
            recv,
            args,
            base_var,
            span,
        } => match crate::builtin::method(it, recv.clone(), &name, args.clone(), span) {
            Ok(kq) => {
                method_writeback(frame, &name, base_var, &kq);
                frame.push_val(kq);
                Ok(())
            }
            Err(Abort::Pending(p)) => {
                frame.cur().work.push(Work::RedispatchMethod {
                    name,
                    recv,
                    args,
                    base_var,
                    span,
                });
                Err(Abort::Pending(p))
            }
            Err(e) => Err(e),
        },
        Work::MakeLambda {
            params,
            ret_depth,
            body,
        } => {
            // Chụp snapshot vào Rc MỚI (khớp interp mặc định) — tránh chu trình Rc tự tham chiếu.
            let env = Rc::new(RefCell::new(frame.cur().env.borrow().clone()));
            frame.push_val(Value::Fn(Rc::new(crate::value::Closure {
                name: String::new(),
                params,
                body,
                env,
                ret_depth,
            })));
            Ok(())
        }
        Work::ApplyAssignVar { sym, op, span } => {
            if let Some(a) = crate::rt::fee::frame_charge(it, FeeKind::Assign, span) {
                return Err(a);
            }
            let rhs = frame.pop_val()?;
            if !frame.mutable.borrow().contains(&sym) {
                return Err(Abort::Trap(Trap::new(
                    crate::interp::MISMATCH,
                    "không gán được cho `let`/tham số (chỉ `var` mới gán được, §4.1)",
                    span,
                )));
            }
            let new =
                if matches!(op, AssignOp::Set) {
                    rhs
                } else {
                    let cur =
                        frame.cur().env.borrow().get(&sym).cloned().ok_or_else(|| {
                            Abort::Trap(Trap::new(RT, "tên chưa có giá trị", span))
                        })?;
                    let bop = match op {
                        AssignOp::Add => BinOp::Add,
                        AssignOp::Sub => BinOp::Sub,
                        AssignOp::Mul => BinOp::Mul,
                        AssignOp::Div => BinOp::Div,
                        AssignOp::FloorDiv => BinOp::FloorDiv,
                        _ => BinOp::Rem,
                    };
                    crate::ops::bin(bop, cur, rhs, span)?
                };
            frame.cur().env.borrow_mut().insert(sym, new);
            Ok(())
        }
        Work::ApplyAssignField {
            sym,
            name,
            op,
            span,
        } => {
            if let Some(a) = crate::rt::fee::frame_charge(it, FeeKind::Assign, span) {
                return Err(a);
            }
            let rhs = frame.pop_val()?;
            if !matches!(op, AssignOp::Set) {
                return Err(Abort::Trap(Trap::new(
                    RT,
                    "gán ghép trên trường chưa được đường frame test hỗ trợ",
                    span,
                )));
            }
            let cur = frame
                .cur()
                .env
                .borrow()
                .get(&sym)
                .cloned()
                .ok_or_else(|| Abort::Trap(Trap::new(RT, "tên chưa có giá trị", span)))?;
            if !frame.mutable.borrow().contains(&sym) {
                return Err(Abort::Trap(Trap::new(
                    crate::interp::MISMATCH,
                    "không gán được trường của `let`/tham số — chỉ `var` mới gán được (§4.1)",
                    span,
                )));
            }
            let moi = crate::gan::gan_truong(cur, &name, rhs, span)?;
            frame.cur().env.borrow_mut().insert(sym, moi);
            Ok(())
        }
        Work::ApplyAssignIndex { sym, op, span } => {
            if let Some(a) = crate::rt::fee::frame_charge(it, FeeKind::Assign, span) {
                return Err(a);
            }
            if !matches!(op, AssignOp::Set) {
                return Err(Abort::Trap(Trap::new(
                    RT,
                    "gán ghép trên phần tử chưa được đường frame test hỗ trợ",
                    span,
                )));
            }
            let rhs = frame.pop_val()?;
            let idx = frame.pop_val()?;
            let cur = frame
                .cur()
                .env
                .borrow()
                .get(&sym)
                .cloned()
                .ok_or_else(|| Abort::Trap(Trap::new(RT, "tên chưa có giá trị", span)))?;
            if !frame.mutable.borrow().contains(&sym) {
                return Err(Abort::Trap(Trap::new(
                    crate::interp::MISMATCH,
                    "không gán được phần tử của `let`/tham số — chỉ `var` mới gán được (§4.1)",
                    span,
                )));
            }
            let moi = crate::gan::gan_chi_so(cur, idx, rhs, span)?;
            frame.cur().env.borrow_mut().insert(sym, moi);
            Ok(())
        }

        // ================= VD-257 / stream A =================
        Work::ApplyTry { span } => {
            let v = frame.pop_val()?;
            match v {
                Value::Enum(ref ea, ref va, ref ag) if ea == "Option" && va == "Some" => {
                    frame.push_val(ag.first().cloned().unwrap_or(Value::Unit));
                }
                Value::Enum(ref ea, ref va, ref ag) if ea == "Result" && va == "Ok" => {
                    frame.push_val(ag.first().cloned().unwrap_or(Value::Unit));
                }
                Value::Enum(ea, va, ag)
                    if (ea == "Option" && va == "None") || (ea == "Result" && va == "Err") =>
                {
                    // Trả sớm từ khung hiện tại (khớp `Instr::Try` của VM): bỏ mọi
                    // continuation còn lại của khung ⇒ side-effect sau `?` KHÔNG chạy.
                    let cf = frame.cur();
                    cf.last = Some(Value::Enum(ea, va, ag));
                    cf.work.clear();
                    cf.loops.clear();
                }
                other => {
                    return Err(Abort::Trap(Trap::new(
                        RT,
                        format!("`?` cần `T?`/`Result`, được `{}`", other.show()),
                        span,
                    )))
                }
            }
            Ok(())
        }
        Work::ApplyTryIndex { span } => {
            let i = frame.pop_val()?;
            let b = frame.pop_val()?;
            let v = match (&b, &i) {
                (Value::List(xs), Value::Int(n)) if *n >= 0 => {
                    Value::opt(xs.get(*n as usize).cloned())
                }
                (Value::Bytes(bs), Value::Int(n)) if *n >= 0 => {
                    Value::opt(bs.get(*n as usize).map(|x| Value::Int(*x as i64)))
                }
                (Value::Map(kv), k) => Value::opt(
                    kv.iter()
                        .find(|(kk, _)| kk.eq_value(k))
                        .map(|(_, v)| v.clone()),
                ),
                _ => Value::none(),
            };
            let _ = span;
            frame.push_val(v);
            Ok(())
        }
        Work::BuildChan { span } => {
            let cap = frame.pop_val()?;
            // Dùng đúng builtin nội bộ `$kenh` của runtime mặc định — tự created value
            // ở đây sẽ lệch quản lý `sched.kenh` (ownership/snapshot).
            let v = crate::builtin::call(it, "$kenh", vec![cap], span)?;
            frame.push_val(v);
            Ok(())
        }
    }
}

/// Ghi ngược kết quả methode `push` vào biến gốc (giữ nguyên hành vi cũ của
/// `ApplyMethod`), dùng chung cho lần chạy đầu và lần retry sau `Pending`.
fn method_writeback(frame: &mut IFrame<'_>, name: &str, base_var: Option<u32>, kq: &Value) {
    if name == "push" {
        if let Some(id) = base_var {
            if frame.mutable.borrow().contains(&id) {
                frame.cur().env.borrow_mut().insert(id, kq.clone());
            }
        }
    }
}
