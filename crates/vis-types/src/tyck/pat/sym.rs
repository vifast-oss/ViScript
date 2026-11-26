// SPDX-License-Identifier: MIT OR Apache-2.0
//! Duyệt `sym` được dùng trong biểu thức/thân (use-set cho luật or-pattern) — tách khỏi `pat.rs`
//! ở `ST-317b`.

use std::collections::BTreeSet;

use vis_hir::hir::{HBody, HCond, HExpr, HStmt, InterpPart};

/// Mọi `sym` được **tham chiếu** (đọc/ghi) trong một biểu thức — để biết binding or-pattern
/// nào thực sự được dùng ở thân nhánh. Không đếm binding của pattern lồng (khác `sym`).
fn expr_syms(e: &HExpr, out: &mut BTreeSet<u32>) {
    match e {
        HExpr::Lit { .. } | HExpr::CmdBlock { .. } | HExpr::Error { .. } => {}
        HExpr::Var { sym, .. } => {
            if let Some(id) = sym {
                out.insert(*id);
            }
        }
        HExpr::Bin { lhs, rhs, .. } => {
            expr_syms(lhs, out);
            expr_syms(rhs, out);
        }
        HExpr::Index { base, index, .. } => {
            expr_syms(base, out);
            expr_syms(index, out);
        }
        HExpr::Un { expr, .. }
        | HExpr::Try { expr, .. }
        | HExpr::Ref { expr, .. }
        | HExpr::Await { expr, .. } => expr_syms(expr, out),
        HExpr::Call {
            callee,
            args,
            named,
            ..
        }
        | HExpr::Spawn {
            callee,
            args,
            named,
            ..
        } => {
            expr_syms(callee, out);
            for a in args {
                expr_syms(a, out);
            }
            for (_, a) in named {
                expr_syms(a, out);
            }
        }
        HExpr::Field { base, .. } => expr_syms(base, out),
        HExpr::List { items, .. } | HExpr::Set { items, .. } | HExpr::Tuple { items, .. } => {
            for it in items {
                expr_syms(it, out);
            }
        }
        HExpr::EnumLit { args, .. } => {
            for it in args {
                expr_syms(it, out);
            }
        }
        HExpr::Map { entries, .. } => {
            for (k, v) in entries {
                expr_syms(k, out);
                expr_syms(v, out);
            }
        }
        HExpr::Record { fields, .. } | HExpr::With { fields, .. } => {
            for (_, v) in fields {
                expr_syms(v, out);
            }
            if let HExpr::With { base, .. } = e {
                expr_syms(base, out);
            }
        }
        HExpr::Opt { value, .. } => {
            if let Some(v) = value {
                expr_syms(v, out);
            }
        }
        HExpr::If {
            cond,
            then,
            elifs,
            else_,
            ..
        } => {
            cond_syms(cond, out);
            stmts_syms(then, out);
            for (c, b) in elifs {
                cond_syms(c, out);
                stmts_syms(b, out);
            }
            stmts_syms(else_, out);
        }
        HExpr::Match { subject, arms, .. } => {
            expr_syms(subject, out);
            for arm in arms {
                if let Some(g) = &arm.guard {
                    expr_syms(g, out);
                }
                stmts_syms(&arm.body, out);
            }
        }
        HExpr::Lambda { body, .. } => body_syms(body, out),
        HExpr::ChanNew { cap, .. } => expr_syms(cap, out),
        HExpr::Interp { parts, .. } => {
            for part in parts {
                match part {
                    InterpPart::Str(_) => {}
                    InterpPart::Expr(e) => expr_syms(e, out),
                    InterpPart::ExprSpec { expr, .. } => expr_syms(expr, out),
                }
            }
        }
    }
}

fn cond_syms(c: &HCond, out: &mut BTreeSet<u32>) {
    match c {
        HCond::Expr(e) => expr_syms(e, out),
        HCond::Let { value, .. } => expr_syms(value, out),
    }
}

fn body_syms(b: &HBody, out: &mut BTreeSet<u32>) {
    match b {
        HBody::Expr(e) => expr_syms(e, out),
        HBody::Stmts(ss) => stmts_syms(ss, out),
    }
}

fn stmts_syms(ss: &[HStmt], out: &mut BTreeSet<u32>) {
    for s in ss {
        match s {
            HStmt::Let { init, else_, .. } => {
                expr_syms(init, out);
                if let Some(e) = else_ {
                    stmts_syms(&e.body, out);
                }
            }
            HStmt::Var { init, .. } => expr_syms(init, out),
            HStmt::Assign { target, value, .. } => {
                expr_syms(target, out);
                expr_syms(value, out);
            }
            HStmt::Expr(e) => expr_syms(e, out),
            HStmt::If {
                cond,
                then,
                elifs,
                else_,
                ..
            } => {
                cond_syms(cond, out);
                stmts_syms(then, out);
                for (c, b) in elifs {
                    cond_syms(c, out);
                    stmts_syms(b, out);
                }
                if let Some(b) = else_ {
                    stmts_syms(b, out);
                }
            }
            HStmt::For { iter, body, .. } => {
                expr_syms(iter, out);
                stmts_syms(body, out);
            }
            HStmt::While { cond, body, .. } => {
                cond_syms(cond, out);
                stmts_syms(body, out);
            }
            HStmt::Loop { body, .. } | HStmt::Scope { body, .. } | HStmt::Block { body, .. } => {
                stmts_syms(body, out)
            }
            HStmt::Return { value, .. } => {
                if let Some(v) = value {
                    expr_syms(v, out);
                }
            }
            HStmt::Match { subject, arms, .. } => {
                expr_syms(subject, out);
                for arm in arms {
                    if let Some(g) = &arm.guard {
                        expr_syms(g, out);
                    }
                    stmts_syms(&arm.body, out);
                }
            }
            HStmt::Break { .. } | HStmt::Continue { .. } => {}
        }
    }
}

/// Mọi `sym` được dùng trong một **thân** (đọc/ghi) — use-set cho luật or-pattern.
pub(crate) fn than_uses(body: &[HStmt]) -> BTreeSet<u32> {
    let mut out = BTreeSet::new();
    stmts_syms(body, &mut out);
    out
}

/// Mọi `sym` được dùng trong guard + thân một nhánh `match`.
pub(crate) fn arm_uses(guard: Option<&HExpr>, body: &[HStmt]) -> BTreeSet<u32> {
    let mut out = BTreeSet::new();
    if let Some(g) = guard {
        expr_syms(g, &mut out);
    }
    stmts_syms(body, &mut out);
    out
}
