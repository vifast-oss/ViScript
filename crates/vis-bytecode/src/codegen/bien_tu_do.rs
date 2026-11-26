// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-319a (#206)** — quét `sym` **được tham chiếu** trong thân một closure.
//!
//! Codegen dùng danh sách này để chụp **biến cấp tệp** theo **giá trị tại điểm tạo closure**
//! (SPEC §4.7): chỉ chụp global **thật sự** xuất hiện trong thân (kể cả trong lambda lồng), nhờ
//! vậy closure tạo trước khi một global **không liên quan** được khởi tạo không bị trap oan khi
//! thân chưa từng đọc biến đó.
//!
//! Quét **thô** theo `sym`: lower cấp id duy nhất cho mỗi khai báo trong toàn chương trình, nên
//! không cần theo dõi miền ràng buộc — một `Var { sym }` trùng `sym` global **luôn** là tham chiếu
//! global, không thể là biến cục bộ/param trùng tên (chúng có `sym` khác).

use std::collections::BTreeSet;

use vis_hir::{HBody, HCond, HExpr, HStmt, InterpPart};

/// Gom mọi `sym` xuất hiện trong thân `b` (đi xuyên lambda lồng).
pub(super) fn quet_body(b: &HBody, ra: &mut BTreeSet<u32>) {
    match b {
        HBody::Expr(e) => quet_expr(e, ra),
        HBody::Stmts(st) => quet_stmts(st, ra),
    }
}

fn quet_stmts(st: &[HStmt], ra: &mut BTreeSet<u32>) {
    for s in st {
        quet_stmt(s, ra);
    }
}

fn quet_stmt(s: &HStmt, ra: &mut BTreeSet<u32>) {
    match s {
        HStmt::Let { init: e, else_, .. } => {
            quet_expr(e, ra);
            if let Some(el) = else_ {
                quet_stmts(&el.body, ra);
            }
        }
        HStmt::Var { init, .. } => quet_expr(init, ra),
        HStmt::Assign { target, value, .. } => {
            quet_expr(target, ra);
            quet_expr(value, ra);
        }
        HStmt::Expr(e) => quet_expr(e, ra),
        HStmt::If {
            cond,
            then,
            elifs,
            else_,
            ..
        } => {
            quet_cond(cond, ra);
            quet_stmts(then, ra);
            for (c, b) in elifs {
                quet_cond(c, ra);
                quet_stmts(b, ra);
            }
            if let Some(b) = else_ {
                quet_stmts(b, ra);
            }
        }
        HStmt::For { iter, body, .. } => {
            quet_expr(iter, ra);
            quet_stmts(body, ra);
        }
        HStmt::While { cond, body, .. } => {
            quet_cond(cond, ra);
            quet_stmts(body, ra);
        }
        HStmt::Loop { body, .. } | HStmt::Scope { body, .. } | HStmt::Block { body, .. } => {
            quet_stmts(body, ra);
        }
        HStmt::Return { value, .. } => {
            if let Some(e) = value {
                quet_expr(e, ra);
            }
        }
        HStmt::Break { .. } | HStmt::Continue { .. } => {}
        HStmt::Match { subject, arms, .. } => {
            quet_expr(subject, ra);
            quet_arms(arms, ra);
        }
    }
}

fn quet_cond(c: &HCond, ra: &mut BTreeSet<u32>) {
    match c {
        HCond::Expr(e) => quet_expr(e, ra),
        HCond::Let { value, .. } => quet_expr(value, ra),
    }
}

fn quet_arms(arms: &[vis_hir::HArm], ra: &mut BTreeSet<u32>) {
    for a in arms {
        if let Some(g) = &a.guard {
            quet_expr(g, ra);
        }
        quet_stmts(&a.body, ra);
    }
}

fn quet_expr(e: &HExpr, ra: &mut BTreeSet<u32>) {
    match e {
        HExpr::Lit { .. } | HExpr::CmdBlock { .. } | HExpr::Error { .. } => {}
        HExpr::Var { sym, .. } => {
            if let Some(id) = sym {
                ra.insert(*id);
            }
        }
        HExpr::Bin { lhs, rhs, .. } => {
            quet_expr(lhs, ra);
            quet_expr(rhs, ra);
        }
        HExpr::Un { expr, .. }
        | HExpr::Try { expr, .. }
        | HExpr::Ref { expr, .. }
        | HExpr::Await { expr, .. }
        | HExpr::Opt {
            value: Some(expr), ..
        } => quet_expr(expr, ra),
        HExpr::Opt { value: None, .. } => {}
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
            quet_expr(callee, ra);
            for a in args {
                quet_expr(a, ra);
            }
            for (_, a) in named {
                quet_expr(a, ra);
            }
        }
        HExpr::Field { base, .. } => quet_expr(base, ra),
        HExpr::Index { base, index, .. } => {
            quet_expr(base, ra);
            quet_expr(index, ra);
        }
        HExpr::List { items, .. } | HExpr::Set { items, .. } | HExpr::Tuple { items, .. } => {
            for x in items {
                quet_expr(x, ra);
            }
        }
        HExpr::Map { entries, .. } => {
            for (k, v) in entries {
                quet_expr(k, ra);
                quet_expr(v, ra);
            }
        }
        HExpr::Record { fields, .. } => {
            for (_, v) in fields {
                quet_expr(v, ra);
            }
        }
        HExpr::EnumLit { args, .. } => {
            for x in args {
                quet_expr(x, ra);
            }
        }
        HExpr::With { base, fields, .. } => {
            quet_expr(base, ra);
            for (_, v) in fields {
                quet_expr(v, ra);
            }
        }
        HExpr::If {
            cond,
            then,
            elifs,
            else_,
            ..
        } => {
            quet_cond(cond, ra);
            quet_stmts(then, ra);
            for (c, b) in elifs {
                quet_cond(c, ra);
                quet_stmts(b, ra);
            }
            quet_stmts(else_, ra);
        }
        HExpr::Match { subject, arms, .. } => {
            quet_expr(subject, ra);
            quet_arms(arms, ra);
        }
        // Lambda **lồng**: biến nó tham chiếu vẫn phải được closure ngoài chụp trước, để lambda
        // trong chụp lại **đúng bản snapshot** của closure ngoài (không đọc global live).
        HExpr::Lambda { body, .. } => quet_body(body, ra),
        HExpr::ChanNew { cap, .. } => quet_expr(cap, ra),
        HExpr::Interp { parts, .. } => {
            for p in parts {
                match p {
                    InterpPart::Str(_) => {}
                    InterpPart::Expr(x) | InterpPart::ExprSpec { expr: x, .. } => quet_expr(x, ra),
                }
            }
        }
    }
}
