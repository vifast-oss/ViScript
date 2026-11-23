// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-274 / VD-255** — continuation stack (`process_work`) của frame evaluator.
//! Tách từ `frame.rs` cho luật R1 (≤1500 dòng/file). Mọi item dùng chung lấy qua `super`.
use super::*;
use crate::rt::fee::FeeKind;

#[path = "proc_expr.rs"]
mod proc_expr;

/// **VD-270** — bước đích gán đường (định nghĩa cạnh helper `thu_duong` ở `frame/expr.rs`).
use super::expr::NutGan;

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
            // **VD-263 + VD-265** — cùng luật với `interp.rs` (interp mặc định) và codegen:
            // - tên trần: chỉ `Ok`/`Some` khớp (bóc **một** lớp payload), còn lại là nhánh lỗi;
            // - pattern bác được: khớp trên **toàn** giá trị; thất bại thì hoàn nguyên binding
            //   một phần trước khi chạy `else`;
            // - thất bại ⇒ chạy khối `else` (không rơi xuống nhánh khớp), binder `else e` theo
            //   `HPat::else_binds_err` (tên trần/`Ok(...)` ⇒ payload `Err`; còn lại ⇒ `()`).
            let env = frame.cur().env.clone();
            let matched = if matches!(pat, HPat::Bind { .. }) {
                match &v {
                    Value::Enum(ea, va, ag)
                        if ((ea == "Result" && va == "Ok") || (ea == "Option" && va == "Some"))
                            && ag.len() == 1 =>
                    {
                        bind_pat_env(pat, &ag[0], &env);
                        true
                    }
                    _ => false,
                }
            } else {
                let snap = env.borrow().clone();
                if bind_pat_env(pat, &v, &env) {
                    true
                } else {
                    *env.borrow_mut() = snap;
                    false
                }
            };
            if !matched {
                if let Some(sym) = bind_sym {
                    let binder = crate::value::let_else_binder(pat, &v);
                    env.borrow_mut().insert(sym, binder);
                }
                frame.cur().work.push(Work::ExecBlock(else_body));
            }
            Ok(())
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
            // **VD-258**: `ApplyNamedCall` ĐÃ pop đối số khỏi `vals` trước khi dispatch;
            // đẩy lại ở đây sẽ chèn rác vào ngăn xếp toán hạng của biểu thức bao ngoài
            // (vd `"x" == net.read(...)` ⇒ so sánh nhầm với đối số cuối).
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
                    LoopKindState::For { pat, items }
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
                    LoopKindState::For { pat, items } => match items.next() {
                        Some(v) => Step::For(pat, Some(v)),
                        None => Step::For(pat, None),
                    },
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

        w => proc_expr::process(it, frame, w),
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

pub(super) fn purge_terminal_frames<V>(
    frames: &mut HashMap<u32, V>,
    s: &rt::Scheduler,
    keep: Option<u32>,
) -> Vec<u32> {
    let dead: Vec<u32> = frames
        .keys()
        .copied()
        .filter(|id| *id != 0 && Some(*id) != keep)
        .filter(|id| {
            s.tasks
                .iter()
                .find(|t| t.id == *id)
                .map(|t| matches!(t.trang_thai.get(), rt::TrangThai::Xong | rt::TrangThai::Huy))
                .unwrap_or(true)
        })
        .collect();
    for id in &dead {
        frames.remove(id);
    }
    dead
}
