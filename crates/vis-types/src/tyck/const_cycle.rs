// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-315d (VD-291 §3.4)** — kiểm **tĩnh** chu trình phụ thuộc `const` (`VS0304`).
//!
//! Đồ thị cạnh `const A → const B` khi biểu thức giá trị của `A` **đánh giá ngay** một tham chiếu
//! tới `B`. Thân `lambda` bị bỏ qua (đánh giá sau, không phải phụ thuộc lúc tính hằng). Chu trình
//! được báo **một lần cho mỗi `const`** trong chu trình, tại `span` khai báo; hàng rào runtime
//! (`ConstCycle`/trap của interp) vẫn giữ nguyên cho artifact không qua `vis check`.

use std::collections::HashMap;

use vis_hir::hir::{HCond, HExpr, HItem, HStmt, Hir, InterpPart};
use vis_syntax::Diagnostic;

/// Thêm chẩn đoán `VS0304` cho mọi `const` nằm trong một chu trình phụ thuộc.
pub(super) fn kiem(hir: &Hir, diags: &mut Vec<Diagnostic>) {
    let ds: Vec<&vis_hir::hir::HConst> = hir
        .items
        .iter()
        .filter_map(|i| match i {
            HItem::Const(c) => Some(c),
            _ => None,
        })
        .collect();
    if ds.is_empty() {
        return;
    }
    let chi_so: HashMap<u32, usize> = ds.iter().enumerate().map(|(i, c)| (c.sym, i)).collect();
    let canh: Vec<Vec<usize>> = ds
        .iter()
        .map(|c| {
            let mut v = Vec::new();
            thu_expr(&c.value, &chi_so, &mut v);
            v.sort_unstable();
            v.dedup();
            v
        })
        .collect();
    let mut mau = vec![0u8; ds.len()];
    let mut ngan_xep: Vec<usize> = Vec::new();
    let mut da_bao = vec![false; ds.len()];
    for i in 0..ds.len() {
        if mau[i] == 0 {
            dfs(i, &canh, &mut mau, &mut ngan_xep, &mut da_bao, &ds, diags);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn dfs(
    u: usize,
    canh: &[Vec<usize>],
    mau: &mut [u8],
    ngan_xep: &mut Vec<usize>,
    da_bao: &mut [bool],
    ds: &[&vis_hir::hir::HConst],
    diags: &mut Vec<Diagnostic>,
) {
    mau[u] = 1;
    ngan_xep.push(u);
    for &v in &canh[u] {
        match mau[v] {
            0 => dfs(v, canh, mau, ngan_xep, da_bao, ds, diags),
            1 => {
                // Cạnh ngược: đoạn `v..=u` trên ngăn xếp DFS chính là một chu trình.
                let dau = ngan_xep
                    .iter()
                    .position(|&x| x == v)
                    .expect("đỉnh xám phải có trên ngăn xếp");
                let chu_trinh = &ngan_xep[dau..];
                let ten: Vec<&str> = chu_trinh.iter().map(|&i| ds[i].name.as_str()).collect();
                let duong = ten.join(" → ");
                for &i in chu_trinh {
                    if !da_bao[i] {
                        da_bao[i] = true;
                        diags.push(Diagnostic {
                            code: super::CONST_CYCLE,
                            message: format!("chu trình phụ thuộc hằng số (`const`): {duong}"),
                            span: ds[i].span,
                            line: 0,
                            col: 0,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    ngan_xep.pop();
    mau[u] = 2;
}

fn thu_cond(c: &HCond, chi_so: &HashMap<u32, usize>, ra: &mut Vec<usize>) {
    match c {
        HCond::Expr(e) => thu_expr(e, chi_so, ra),
        HCond::Let { value, .. } => thu_expr(value, chi_so, ra),
    }
}

fn thu_stmt(s: &HStmt, chi_so: &HashMap<u32, usize>, ra: &mut Vec<usize>) {
    match s {
        HStmt::Let { init, else_, .. } => {
            thu_expr(init, chi_so, ra);
            if let Some(e) = else_ {
                for s in &e.body {
                    thu_stmt(s, chi_so, ra);
                }
            }
        }
        HStmt::Var { init, .. } => thu_expr(init, chi_so, ra),
        HStmt::Assign { target, value, .. } => {
            thu_expr(target, chi_so, ra);
            thu_expr(value, chi_so, ra);
        }
        HStmt::Expr(e) => thu_expr(e, chi_so, ra),
        HStmt::If {
            cond,
            then,
            elifs,
            else_,
            ..
        } => {
            thu_cond(cond, chi_so, ra);
            for s in then {
                thu_stmt(s, chi_so, ra);
            }
            for (c, ss) in elifs {
                thu_cond(c, chi_so, ra);
                for s in ss {
                    thu_stmt(s, chi_so, ra);
                }
            }
            if let Some(ss) = else_ {
                for s in ss {
                    thu_stmt(s, chi_so, ra);
                }
            }
        }
        HStmt::For { iter, body, .. } => {
            thu_expr(iter, chi_so, ra);
            for s in body {
                thu_stmt(s, chi_so, ra);
            }
        }
        HStmt::While { cond, body, .. } => {
            thu_cond(cond, chi_so, ra);
            for s in body {
                thu_stmt(s, chi_so, ra);
            }
        }
        HStmt::Loop { body, .. } | HStmt::Scope { body, .. } | HStmt::Block { body, .. } => {
            for s in body {
                thu_stmt(s, chi_so, ra);
            }
        }
        HStmt::Return { value: Some(e), .. } => thu_expr(e, chi_so, ra),
        HStmt::Return { value: None, .. } | HStmt::Break { .. } | HStmt::Continue { .. } => {}
        HStmt::Match { subject, arms, .. } => {
            thu_expr(subject, chi_so, ra);
            for a in arms {
                if let Some(g) = &a.guard {
                    thu_expr(g, chi_so, ra);
                }
                for s in &a.body {
                    thu_stmt(s, chi_so, ra);
                }
            }
        }
    }
}

fn thu_expr(e: &HExpr, chi_so: &HashMap<u32, usize>, ra: &mut Vec<usize>) {
    match e {
        HExpr::Var { sym: Some(s), .. } => {
            if let Some(&i) = chi_so.get(s) {
                ra.push(i);
            }
        }
        HExpr::Bin { lhs, rhs, .. } => {
            thu_expr(lhs, chi_so, ra);
            thu_expr(rhs, chi_so, ra);
        }
        HExpr::Un { expr, .. }
        | HExpr::Try { expr, .. }
        | HExpr::Ref { expr, .. }
        | HExpr::Await { expr, .. }
        | HExpr::Field { base: expr, .. } => thu_expr(expr, chi_so, ra),
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
            thu_expr(callee, chi_so, ra);
            for a in args {
                thu_expr(a, chi_so, ra);
            }
            for (_, a) in named {
                thu_expr(a, chi_so, ra);
            }
        }
        HExpr::Index { base, index, .. } => {
            thu_expr(base, chi_so, ra);
            thu_expr(index, chi_so, ra);
        }
        HExpr::List { items, .. } | HExpr::Set { items, .. } | HExpr::Tuple { items, .. } => {
            for x in items {
                thu_expr(x, chi_so, ra);
            }
        }
        HExpr::Map { entries, .. } => {
            for (k, v) in entries {
                thu_expr(k, chi_so, ra);
                thu_expr(v, chi_so, ra);
            }
        }
        HExpr::Record { fields, .. } => {
            for (_, v) in fields {
                thu_expr(v, chi_so, ra);
            }
        }
        HExpr::EnumLit { args, .. } => {
            for a in args {
                thu_expr(a, chi_so, ra);
            }
        }
        HExpr::Opt { value, .. } => {
            if let Some(v) = value {
                thu_expr(v, chi_so, ra);
            }
        }
        HExpr::With { base, fields, .. } => {
            thu_expr(base, chi_so, ra);
            for (_, v) in fields {
                thu_expr(v, chi_so, ra);
            }
        }
        HExpr::If {
            cond,
            then,
            elifs,
            else_,
            ..
        } => {
            thu_cond(cond, chi_so, ra);
            for s in then {
                thu_stmt(s, chi_so, ra);
            }
            for (c, ss) in elifs {
                thu_cond(c, chi_so, ra);
                for s in ss {
                    thu_stmt(s, chi_so, ra);
                }
            }
            for s in else_ {
                thu_stmt(s, chi_so, ra);
            }
        }
        HExpr::Match { subject, arms, .. } => {
            thu_expr(subject, chi_so, ra);
            for a in arms {
                if let Some(g) = &a.guard {
                    thu_expr(g, chi_so, ra);
                }
                for s in &a.body {
                    thu_stmt(s, chi_so, ra);
                }
            }
        }
        HExpr::Interp { parts, .. } => {
            for p in parts {
                match p {
                    InterpPart::Str(_) => {}
                    InterpPart::Expr(x) | InterpPart::ExprSpec { expr: x, .. } => {
                        thu_expr(x, chi_so, ra)
                    }
                }
            }
        }
        // Thân `lambda` đánh giá **sau** khi hằng đã có giá trị ⇒ không phải phụ thuộc tĩnh.
        HExpr::Lambda { .. } => {}
        HExpr::ChanNew { cap, .. } => thu_expr(cap, chi_so, ra),
        HExpr::Lit { .. }
        | HExpr::Var { sym: None, .. }
        | HExpr::CmdBlock { .. }
        | HExpr::Error { .. } => {}
    }
}
