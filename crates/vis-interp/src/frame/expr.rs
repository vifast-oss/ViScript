// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-274 / VD-255** — đánh giá HExpr/HStmt + gọi hàm của frame evaluator.
//! Tách từ `frame.rs` cho luật R1 (≤1500 dòng/file). Mọi item dùng chung lấy qua `super`.
use super::*;
use crate::rt::fee::FeeKind;

/// Đánh giá một `HExpr` (đẩy kết quả vào `vals` hoặc continuation).
pub(super) fn eval_work<'a>(it: &mut Interp<'a>, frame: &mut IFrame<'a>, e: &'a HExpr) -> R<()> {
    // **VD-257/D** — mirror `interp.rs:628`: E_EXPR cho mọi nút thực sự chạy, TRỪ
    // Call/Spawn/Await (ba nhánh đó mang sự kiện riêng — VD-252 quy tắc 1/2).
    // Charge nằm ở đầu execution ⇒ budget check đúng trước side-effect; hook tắt
    // (mặc định) ⇒ no-op, frame path không đổi (test f2 của stream A vẫn pass).
    if !matches!(
        e,
        HExpr::Call { .. } | HExpr::Spawn { .. } | HExpr::Await { .. }
    ) {
        if let Some(a) = crate::rt::fee::frame_charge(it, FeeKind::Expr, e.span()) {
            return Err(a);
        }
    }
    match e {
        HExpr::Lit { lit, .. } => {
            let v = lit_value(lit).map_err(|m| Abort::Trap(Trap::new(RT, m, e.span())))?;
            frame.push_val(v);
        }
        HExpr::Var { sym, name, .. } => {
            let cf = frame.cur();
            let v = sym
                .and_then(|s| cf.env.borrow().get(&s).cloned())
                // ST-283: thẻ `enum`/`type` phải được ưu tiên TRƯỚC argv fallback (`u32::MAX`),
                // nếu không `E` bị lấy nhầm thành danh sách đối số `[]`.
                .or_else(|| {
                    sym.and_then(|s| it.tags.get(&s).cloned())
                        .map(|(n, is_enum)| {
                            if is_enum {
                                Value::Enum(n, String::new(), Vec::new())
                            } else {
                                Value::Record(n, Vec::new())
                            }
                        })
                })
                .or_else(|| {
                    cf.env
                        .borrow()
                        .iter()
                        .find(|(s, _)| **s == u32::MAX)
                        .map(|(_, v)| v.clone())
                })
                .or_else(|| it.env.borrow().get(&sym.unwrap_or(u32::MAX)).cloned());
            if let Some(v) = v {
                frame.push_val(v);
            } else if let Some(f) = sym.and_then(|s| it.fns.get(&s).copied()) {
                let cenv = frame.cur().env.clone();
                frame.push_val(Value::Fn(Rc::new(crate::value::Closure {
                    name: f.name.clone(),
                    params: f.params.iter().map(|p| p.sym).collect(),
                    body: HBody::Stmts(f.body.clone()),
                    env: cenv,
                    ret_depth: opt_depth_ty(&f.ret),
                })));
            } else if name == "None" {
                frame.push_val(Value::none());
            } else if let Some(sig) = crate::builtin::SIGS.iter().find(|s| s.name == name) {
                frame.push_val(Value::Builtin(sig.name));
            } else {
                return Err(Abort::Trap(Trap::new(
                    RT,
                    format!("biến `{name}` chưa định nghĩa trong frame test"),
                    e.span(),
                )));
            }
        }
        HExpr::Bin { op, lhs, rhs, .. } => match op {
            BinOp::And | BinOp::Or => {
                frame.cur().work.push(Work::ApplyShortCircuit {
                    op: *op,
                    rhs,
                    span: e.span(),
                });
                frame.cur().work.push(Work::Eval(lhs));
            }
            BinOp::Coalesce => {
                frame.cur().work.push(Work::ApplyCoalesce { rhs });
                frame.cur().work.push(Work::Eval(lhs));
            }
            BinOp::Pipe => {
                frame.cur().work.push(Work::ApplyPipe {
                    rhs,
                    span: e.span(),
                });
                frame.cur().work.push(Work::Eval(lhs));
            }
            _ => {
                frame.cur().work.push(Work::ApplyBin {
                    op: *op,
                    span: e.span(),
                });
                frame.cur().work.push(Work::Eval(rhs));
                frame.cur().work.push(Work::Eval(lhs));
            }
        },
        HExpr::Un { op, expr, .. } => {
            frame.cur().work.push(Work::ApplyUn {
                op: *op,
                span: e.span(),
            });
            frame.cur().work.push(Work::Eval(expr));
        }
        HExpr::Call {
            callee,
            args,
            named,
            span,
        } => {
            // **VD-257/D** — mirror `interp.rs:735-751` + codegen: charge sự kiện
            // gọi **tại nút Call, TRƯỚC đối số** (giữ thứ tự f(g(1))). `host` =
            // method trên giá trị (Field) hoặc builtin theo tên không phải hàm tệp.
            let host = match &**callee {
                HExpr::Field { .. } => true,
                HExpr::Var { name, sym, .. } => {
                    sym.and_then(|id| it.fns.get(&id)).is_none() && is_builtin_name(name)
                }
                _ => false,
            };
            let call_kind = if host {
                FeeKind::Builtin
            } else {
                FeeKind::Call
            };
            if let Some(a) = crate::rt::fee::frame_charge(it, call_kind, *span) {
                return Err(a);
            }
            // Interp đánh giá callee dạng Var/Field như một GIÁ TRỊ ⇒ có E_EXPR
            // riêng (codegen cũng charge thêm để parity). Các dạng callee khác đã
            // được `Work::Eval(callee)` ở dưới charge — không cộng đôi.
            if matches!(&**callee, HExpr::Var { .. } | HExpr::Field { .. }) {
                if let Some(a) = crate::rt::fee::frame_charge(it, FeeKind::Expr, callee.span()) {
                    return Err(a);
                }
            }
            if let Some(c) = classify_callee(it, callee) {
                frame.cur().work.push(args_works(
                    args,
                    named,
                    *span,
                    vec![Work::ApplyNamedCall {
                        callee: c,
                        npos: args.len(),
                        names: named.iter().map(|(n, _)| n.clone()).collect(),
                        span: *span,
                    }],
                ));
            } else if let HExpr::Field {
                base,
                name,
                optional: false,
                ..
            } = &**callee
            {
                frame.cur().work.push(Work::ApplyMethodOrField {
                    name: name.clone(),
                    args,
                    named,
                    base,
                    span: *span,
                });
                frame.cur().work.push(Work::Eval(base));
            } else {
                frame.cur().work.push(args_works(
                    args,
                    named,
                    *span,
                    vec![Work::ApplyValueCall {
                        npos: args.len(),
                        names: named.iter().map(|(n, _)| n.clone()).collect(),
                        span: *span,
                    }],
                ));
                frame.cur().work.push(Work::Eval(callee));
            }
        }
        HExpr::Spawn {
            callee,
            args,
            named: _,
            span,
        } => {
            // VD-252/VD-256: `$spawn` nội bộ chỉ mang E_SPAWN, KHÔNG cộng E_BUILTIN.
            if let Some(a) = crate::rt::fee::frame_charge(it, FeeKind::Spawn, *span) {
                return Err(a);
            }
            match &**callee {
                HExpr::Var { sym: Some(id), .. } => {
                    let f = it.fns.get(id).copied().ok_or_else(|| {
                        Abort::Trap(Trap::new(
                            RT,
                            "frame test: spawn hàm không tồn tại",
                            e.span(),
                        ))
                    })?;
                    let c = Callee::Spawn(*id, &f.name);
                    frame.cur().work.push(args_works(
                        args,
                        &[],
                        *span,
                        vec![Work::ApplyNamedCall {
                            callee: c,
                            npos: args.len(),
                            names: Vec::new(),
                            span: *span,
                        }],
                    ));
                }
                _ => {
                    return Err(Abort::Trap(Trap::new(
                        RT,
                        "frame test: spawn cần tên hàm async cấp tệp",
                        e.span(),
                    )))
                }
            }
        }
        HExpr::Await { expr, .. } => {
            // VD-252 quy tắc 5: charge TRƯỚC suspend; pc/continuation lưu SAU điểm
            // charge ⇒ resume (ApplyAwait) không charge lại.
            if let Some(a) = crate::rt::fee::frame_charge(it, FeeKind::Await, e.span()) {
                return Err(a);
            }
            frame.cur().work.push(Work::ApplyAwait(e.span()));
            frame.cur().work.push(Work::Eval(expr));
        }
        HExpr::Field {
            base,
            name,
            optional,
            span,
        } => {
            frame.cur().work.push(Work::ApplyField {
                name: name.clone(),
                optional: *optional,
                span: *span,
            });
            frame.cur().work.push(Work::Eval(base));
        }
        HExpr::Index { base, index, span } => {
            frame.cur().work.push(Work::ApplyIndex { span: *span });
            frame.cur().work.push(Work::Eval(index));
            frame.cur().work.push(Work::Eval(base));
        }
        HExpr::List { items, .. } => {
            frame.cur().work.push(args_works(
                items,
                &[],
                e.span(),
                vec![Work::BuildList { n: items.len() }],
            ));
        }
        HExpr::Set { items, .. } => {
            frame.cur().work.push(args_works(
                items,
                &[],
                e.span(),
                vec![Work::BuildSet { n: items.len() }],
            ));
        }
        HExpr::Tuple { items, .. } => {
            frame.cur().work.push(args_works(
                items,
                &[],
                e.span(),
                vec![Work::BuildTuple { n: items.len() }],
            ));
        }
        HExpr::Map { entries, .. } => {
            let mut w = Vec::new();
            for (k, v) in entries.iter() {
                w.push(Work::Eval(k));
                w.push(Work::Eval(v));
            }
            w.push(Work::BuildMap { n: entries.len() });
            append_work(frame.cur(), w);
        }
        HExpr::Record { fields, ty, span } => {
            frame.cur().work.push(Work::BuildRecord {
                name: type_name(it, ty),
                fields,
                span: *span,
            });
            frame.cur().work.push(Work::EvalRecordFields {
                fields,
                idx: 0,
                span: *span,
            });
        }
        HExpr::EnumLit {
            path,
            variant,
            args,
            ty,
            span,
            ..
        } => {
            let mut name = type_name(it, ty);
            if name.is_empty() {
                name = path.first().cloned().unwrap_or_default();
            }
            frame.cur().work.push(args_works(
                args,
                &[],
                *span,
                vec![Work::BuildEnum {
                    name,
                    variant: variant.clone(),
                    n: args.len(),
                    span: *span,
                }],
            ));
        }
        HExpr::Opt { value, .. } => match value {
            None => frame.push_val(Value::none()),
            Some(v) => {
                frame.cur().work.push(Work::ApplySome);
                frame.cur().work.push(Work::Eval(v));
            }
        },
        HExpr::With { base, fields, span } => {
            let mut w = vec![Work::Eval(base)];
            for (_, v) in fields.iter() {
                w.push(Work::Eval(v));
            }
            w.push(Work::WithCollect {
                names: fields.iter().map(|(n, _)| n.clone()).collect(),
                n: fields.len(),
                span: *span,
            });
            append_work(frame.cur(), w);
        }
        HExpr::If {
            cond,
            then,
            elifs,
            else_,
            span,
            ..
        } => {
            append_work(
                frame.cur(),
                if_work(cond, then, elifs, Some(else_), *span, false),
            );
        }
        HExpr::Match {
            subject,
            arms,
            span,
            ..
        } => {
            frame.cur().work.push(Work::MatchSubj {
                arms,
                stmt_mode: false,
                span: *span,
            });
            frame.cur().work.push(Work::Eval(subject));
        }
        HExpr::Lambda {
            params, ret, body, ..
        } => {
            frame.cur().work.push(Work::MakeLambda {
                params: params.iter().map(|p| p.sym).collect(),
                ret_depth: opt_depth_ty(ret),
                body: body.clone(),
            });
        }
        HExpr::Interp { parts, span, .. } => {
            frame.cur().work.push(Work::InterpParts {
                parts,
                idx: 0,
                out: String::new(),
                span: *span,
            });
        }
        HExpr::Ref { expr, .. } => {
            frame.cur().work.push(Work::Eval(expr));
        }
        // **VD-257 / stream A**: `?` — hai dạng. `xs[i]?` là chỉ số có kiểm tra (VD-178 R8);
        // còn lại là `Option`/`Result` unwrap với trả sớm `None`/`Err` (§4.4).
        HExpr::Try { expr, span } => {
            if let HExpr::Index { base, index, .. } = &**expr {
                frame.cur().work.push(Work::ApplyTryIndex { span: *span });
                frame.cur().work.push(Work::Eval(index));
                frame.cur().work.push(Work::Eval(base));
            } else {
                frame.cur().work.push(Work::ApplyTry { span: *span });
                frame.cur().work.push(Work::Eval(expr));
            }
        }
        // **VD-257 / stream A**: `chan[T](n)` — eval dung lượng rồi gọi `$kenh` thật
        // (cùng builtin với runtime mặc định, không tự dựng `Value::Chan`).
        HExpr::ChanNew { cap, span, .. } => {
            frame.cur().work.push(Work::BuildChan { span: *span });
            frame.cur().work.push(Work::Eval(cap));
        }
        // `cmd { … }` là VS1 — runtime mặc định và codegen VM đều từ chối. Frame path
        // phải từ chối **cùng thông điệp** (không dùng cụm "chưa được đường frame test
        // hỗ trợ" gây hiểu lầm), ngay tại điểm gặp, trước khi đẩy operand.
        HExpr::CmdBlock { span, .. } => {
            return Err(Abort::Trap(Trap::new(
                RT,
                "khối `cmd` là VS1 — chưa hỗ trợ ở VS0",
                *span,
            )))
        }
        HExpr::Error { .. } => {
            return Err(Abort::Trap(Trap::new(
                RT,
                format!(
                    "frame test: HExpr {:?} chưa được đường frame test hỗ trợ",
                    std::mem::discriminant(e)
                ),
                e.span(),
            )))
        }
    }
    Ok(())
}

/// Đánh giá một `HStmt` (đẩy continuation tương ứng).
pub(super) fn exec_work<'a>(it: &mut Interp<'a>, frame: &mut IFrame<'a>, s: &'a HStmt) -> R<()> {
    // **VD-257/D** — mirror `interp.rs:232`: E_STMT một lần mỗi câu, trước khi
    // đẩy continuation. Resume không chạy lại `exec_work` của câu đã xong.
    if let Some(a) = crate::rt::fee::frame_charge(it, FeeKind::Stmt, s.span()) {
        return Err(a);
    }
    match s {
        HStmt::Let {
            pat,
            ty,
            init,
            else_,
            span,
        } => {
            let depth = ty.as_ref().map(opt_depth_ty).unwrap_or(0);
            if let Some(e) = else_ {
                frame.cur().work.push(Work::ApplyLetElse {
                    pat,
                    else_body: &e.body,
                    bind_sym: e.bind_sym,
                    opt_depth: depth,
                    span: *span,
                });
            } else {
                frame.cur().work.push(Work::Bind {
                    pat,
                    opt_depth: depth,
                    span: *span,
                });
            }
            frame.cur().work.push(Work::Eval(init));
            Ok(())
        }
        HStmt::Var {
            sym,
            ty,
            init,
            span,
            ..
        } => {
            let depth = ty.as_ref().map(opt_depth_ty).unwrap_or(0);
            frame.cur().work.push(Work::SetVar {
                sym: *sym,
                opt_depth: depth,
                span: *span,
            });
            frame.cur().work.push(Work::Eval(init));
            Ok(())
        }
        HStmt::Assign {
            target,
            op,
            value,
            span,
        } => {
            match target {
                HExpr::Var { sym: Some(id), .. } => {
                    frame.cur().work.push(Work::ApplyAssignVar {
                        sym: *id,
                        op: *op,
                        span: *span,
                    });
                    frame.cur().work.push(Work::Eval(value));
                }
                HExpr::Field {
                    base: goc, name, ..
                } => {
                    if let HExpr::Var { sym: Some(id), .. } = &**goc {
                        frame.cur().work.push(Work::ApplyAssignField {
                            sym: *id,
                            name: name.clone(),
                            op: *op,
                            span: *span,
                        });
                        frame.cur().work.push(Work::Eval(value));
                    } else {
                        return Err(Abort::Trap(Trap::new(
                            RT,
                            "frame test: gán trường chỉ hỗ trợ gốc là `var`",
                            *span,
                        )));
                    }
                }
                HExpr::Index { base, .. } => {
                    if let HExpr::Var { sym: Some(id), .. } = &**base {
                        frame.cur().work.push(Work::ApplyAssignIndex {
                            sym: *id,
                            op: *op,
                            span: *span,
                        });
                        frame.cur().work.push(Work::Eval(value));
                        if let HExpr::Index { index, .. } = target {
                            frame.cur().work.push(Work::Eval(index.as_ref()));
                        }
                    } else {
                        return Err(Abort::Trap(Trap::new(
                            RT,
                            "frame test: gán phần tử chỉ hỗ trợ gốc là `var`",
                            *span,
                        )));
                    }
                }
                _ => {
                    return Err(Abort::Trap(Trap::new(
                        RT,
                        "frame test: đích gán chưa được hỗ trợ",
                        *span,
                    )))
                }
            }
            let _ = it;
            Ok(())
        }
        HStmt::Expr(e) => {
            frame.cur().work.push(Work::SetLast);
            frame.cur().work.push(Work::Eval(e));
            Ok(())
        }
        HStmt::If {
            cond,
            then,
            elifs,
            else_,
            span,
        } => {
            append_work(
                frame.cur(),
                if_work(cond, then, elifs, else_.as_deref(), *span, true),
            );
            Ok(())
        }
        HStmt::For {
            pat,
            iter,
            body,
            span,
        } => {
            frame.cur().work.push(Work::LoopInit {
                kind: LoopKindInit::For { pat, iter },
                body,
                span: *span,
            });
            frame.cur().work.push(Work::Eval(iter));
            Ok(())
        }
        HStmt::While { cond, body, span } => {
            frame.cur().work.push(Work::LoopInit {
                kind: LoopKindInit::While { cond },
                body,
                span: *span,
            });
            Ok(())
        }
        HStmt::Loop { body, span } => {
            frame.cur().work.push(Work::LoopInit {
                kind: LoopKindInit::Loop,
                body,
                span: *span,
            });
            Ok(())
        }
        HStmt::Scope { body, .. } | HStmt::Block { body, .. } => {
            frame.cur().work.push(Work::ExecBlock(body));
            Ok(())
        }
        HStmt::Return { value, .. } => {
            frame.cur().work.push(Work::DoReturn);
            if let Some(e) = value {
                frame.cur().work.push(Work::Eval(e));
            } else {
                frame.push_val(Value::Unit);
            }
            Ok(())
        }
        HStmt::Break { span } => {
            frame.cur().work.push(Work::DoBreak { span: *span });
            Ok(())
        }
        HStmt::Continue { span } => {
            frame.cur().work.push(Work::DoContinue { span: *span });
            Ok(())
        }
        HStmt::Match {
            subject,
            arms,
            span,
        } => {
            frame.cur().work.push(Work::MatchSubj {
                arms,
                stmt_mode: true,
                span: *span,
            });
            frame.cur().work.push(Work::Eval(subject));
            Ok(())
        }
    }
}

/// Gọi một giá trị hàm (closure/builtin value/record ctor) — mirror `call_value`.
pub(super) fn call_value<'a>(
    it: &mut Interp<'a>,
    frame: &mut IFrame<'a>,
    f: Value,
    pos: Vec<Value>,
    named: Vec<(String, Value)>,
    span: Span,
) -> R<()> {
    match f {
        Value::Fn(c) => {
            if pos.len() != c.params.len() {
                return Err(Abort::Trap(Trap::new(
                    RT,
                    format!(
                        "hàm `{}` cần {} đối số, được {}",
                        if c.name.is_empty() { "<fn>" } else { &c.name },
                        c.params.len(),
                        pos.len()
                    ),
                    span,
                )));
            }
            let env = c.env.clone();
            let mut restore = Vec::new();
            for (i, p) in c.params.iter().enumerate() {
                restore.push((*p, env.borrow().get(p).cloned()));
                env.borrow_mut().insert(*p, pos[i].clone());
            }
            // `c.body` là HBody sở hữu trong Value::Fn — leak một bản sao nhỏ để có
            // tham chiếu 'static ⊆ 'a (test-only), work stack giữ thân closure qua suspend.
            let body: &'static HBody = Box::leak(Box::new(c.body.clone()));
            let work = match body {
                HBody::Expr(e) => vec![Work::PushLast, Work::Eval(e.as_ref())],
                HBody::Stmts(st) => initial_work(st.as_slice()),
            };
            frame.calls.push(CallFrame {
                env,
                ret_depth: c.ret_depth,
                work,
                last: None,
                loops: Vec::new(),
                restore,
            });
            Ok(())
        }
        Value::Builtin(name) => {
            let v = crate::builtin::call(it, name, pos, span)?;
            let _ = named;
            frame.push_val(v);
            Ok(())
        }
        Value::Record(name, fields) if fields.is_empty() => {
            frame.push_val(Value::Record(name, named));
            Ok(())
        }
        other => {
            let _ = named;
            Err(Abort::Trap(Trap::new(
                RT,
                format!("`{}` không phải hàm", other.show()),
                span,
            )))
        }
    }
}

/// Tên builtin được frame evaluator chấp nhận: 8 tên ngắn (`SIGS`) **và** `std.*`.
pub(super) fn is_builtin_name(name: &str) -> bool {
    crate::builtin::SIGS.iter().any(|s| s.name == name)
        || name
            .strip_prefix("std.")
            .and_then(vis_types::stdlib::sig_of)
            .is_some()
}

pub(super) fn classify_callee<'a>(it: &Interp<'a>, callee: &'a HExpr) -> Option<Callee<'a>> {
    match callee {
        HExpr::Var {
            sym: Some(id),
            name,
            ..
        } => it
            .fns
            .get(id)
            .copied()
            .map(|f| {
                if f.is_async {
                    Callee::Named(*id, true, &f.name)
                } else {
                    Callee::Named(*id, false, &f.name)
                }
            })
            .or_else(|| {
                if is_builtin_name(name) {
                    Some(Callee::Builtin(name))
                } else {
                    None
                }
            }),
        HExpr::Var {
            sym: None, name, ..
        } => {
            if is_builtin_name(name) {
                Some(Callee::Builtin(name))
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(super) fn dispatch<'a>(
    it: &mut Interp<'a>,
    frame: &mut IFrame<'a>,
    callee: Callee<'a>,
    args: Vec<Value>,
    named: Vec<(String, Value)>,
    span: Span,
) -> R<()> {
    match callee {
        Callee::Builtin(name) => {
            let v = crate::builtin::call(it, name, args, span)?;
            let _ = named;
            frame.push_val(v);
            Ok(())
        }
        Callee::Spawn(sym, ten) => {
            let t = it
                .sched
                .borrow_mut()
                .spawn(sym, args, ten.to_string(), span);
            let _ = named;
            frame.push_val(Value::Task(t));
            Ok(())
        }
        Callee::Named(sym, is_async, ten) => {
            if is_async {
                let t = it.sched.borrow_mut().tao(sym, args, ten.to_string(), span);
                let _ = named;
                frame.push_val(Value::Task(t));
                return Ok(());
            }
            let f =
                it.fns.get(&sym).copied().ok_or_else(|| {
                    Abort::Trap(Trap::new(RT, "frame test: hàm không tồn tại", span))
                })?;
            if args.len() + named.len() > f.params.len()
                && !f.params.last().map(|p| p.variadic).unwrap_or(false)
            {
                return Err(Abort::Trap(Trap::new(
                    RT,
                    format!(
                        "hàm `{ten}` nhận {} đối số, được {}",
                        f.params.len(),
                        args.len() + named.len()
                    ),
                    span,
                )));
            }
            let env = Rc::new(RefCell::new(HashMap::new()));
            for (i, p) in f.params.iter().enumerate() {
                let v = args.get(i).cloned().or_else(|| {
                    named
                        .iter()
                        .find(|(n, _)| *n == p.name)
                        .map(|(_, v)| v.clone())
                });
                let v = match v {
                    Some(v) => v,
                    None => match &p.default {
                        Some(d) => eval_expr_in(it, d, &env, &frame.mutable)?,
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
                    frame.mutable.borrow_mut().insert(p.sym);
                }
                env.borrow_mut().insert(p.sym, v);
            }
            frame.calls.push(CallFrame {
                env,
                ret_depth: opt_depth_ty(&f.ret),
                work: initial_work(&f.body),
                last: None,
                loops: Vec::new(),
                restore: Vec::new(),
            });
            Ok(())
        }
    }
}
