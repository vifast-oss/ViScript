// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-315d (VD-291 §3.4)** — kiểm **tĩnh** chu trình phụ thuộc `const` (`VS0304`).
//!
//! Đồ thị cạnh `const A → const B` khi biểu thức giá trị của `A` **đánh giá ngay** một tham chiếu
//! tới `B`. Thân `lambda` bị bỏ qua (đánh giá sau, không phải phụ thuộc lúc tính hằng). Đây là
//! **xấp xỉ trên** (over-approximation) theo hướng an toàn: nhánh chết như `if false: B` vẫn tính;
//! tham chiếu `const` **bên trong thân hàm** được gọi từ giá trị hằng có thể lọt (false negative) —
//! hàng rào runtime `ConstCycle`/trap interp vẫn bắt.
//!
//! Dùng **Tarjan lặp** (stack tường minh) — chuỗi phụ thuộc 100k không tràn stack. Với SCC nhỏ
//! (≤ 8 hằng) báo **một chẩn đoán mỗi `const`**; SCC lớn chỉ báo **một chẩn đoán** có trần tên
//! (tránh O(N²) bộ nhớ/thời gian khi in). Thứ tự chẩn đoán theo `span` nguồn.

use std::collections::HashMap;

use vis_hir::hir::{HCond, HExpr, HItem, HStmt, Hir, InterpPart};
use vis_syntax::Diagnostic;

/// Số tên tối đa in trong một chẩn đoán SCC lớn.
const TEN_TOI_DA: usize = 8;

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
    let sccs = tinh_scc(&canh);
    let mut them: Vec<Diagnostic> = Vec::new();
    for scc in sccs {
        let tu_thanh = scc.len() > 1 || canh[scc[0]].contains(&scc[0]);
        if !tu_thanh {
            continue;
        }
        let mut ds_scc = scc;
        ds_scc.sort_unstable();
        let ten: Vec<&str> = ds_scc.iter().map(|&i| ds[i].name.as_str()).collect();
        if ten.len() <= TEN_TOI_DA {
            let ds_ten = ten.join(", ");
            for i in ds_scc {
                them.push(Diagnostic {
                    code: super::CONST_CYCLE,
                    message: format!("chu trình phụ thuộc hằng số (`const`): {ds_ten}"),
                    span: ds[i].span,
                    line: 0,
                    col: 0,
                });
            }
        } else {
            // SCC lớn: **một** chẩn đoán, tên có trần — không nhân bản message N lần.
            let dau = ten[..TEN_TOI_DA].join(", ");
            let i = ds_scc[0];
            them.push(Diagnostic {
                code: super::CONST_CYCLE,
                message: format!(
                    "chu trình phụ thuộc hằng số (`const`) gồm {} hằng: {dau} …",
                    ten.len()
                ),
                span: ds[i].span,
                line: 0,
                col: 0,
            });
        }
    }
    // Thứ tự theo nguồn — ổn định, không phụ thuộc thứ tự hoàn tất SCC.
    them.sort_by_key(|d| (d.span.start, d.span.end));
    diags.extend(them);
}

/// Tarjan **lặp** (stack tường minh): trả các SCC; không đệ quy theo độ sâu phụ thuộc.
fn tinh_scc(canh: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let n = canh.len();
    let mut chi_so = vec![usize::MAX; n];
    let mut low = vec![0usize; n];
    let mut tren_ngan_xep = vec![false; n];
    let mut ngan_xep: Vec<usize> = Vec::new();
    let mut dem = 0usize;
    let mut sccs: Vec<Vec<usize>> = Vec::new();
    let mut goi: Vec<(usize, usize)> = Vec::new();
    for start in 0..n {
        if chi_so[start] != usize::MAX {
            continue;
        }
        chi_so[start] = dem;
        low[start] = dem;
        dem += 1;
        ngan_xep.push(start);
        tren_ngan_xep[start] = true;
        goi.push((start, 0));
        while let Some(&(u, i_con)) = goi.last() {
            if i_con < canh[u].len() {
                if let Some(top) = goi.last_mut() {
                    top.1 += 1;
                }
                let v = canh[u][i_con];
                if chi_so[v] == usize::MAX {
                    chi_so[v] = dem;
                    low[v] = dem;
                    dem += 1;
                    ngan_xep.push(v);
                    tren_ngan_xep[v] = true;
                    goi.push((v, 0));
                } else if tren_ngan_xep[v] {
                    low[u] = low[u].min(chi_so[v]);
                }
            } else {
                goi.pop();
                if let Some(&(p, _)) = goi.last() {
                    low[p] = low[p].min(low[u]);
                }
                if low[u] == chi_so[u] {
                    let mut scc = Vec::new();
                    loop {
                        let w = ngan_xep.pop().expect("ngăn xếp Tarjan không rỗng");
                        tren_ngan_xep[w] = false;
                        scc.push(w);
                        if w == u {
                            break;
                        }
                    }
                    sccs.push(scc);
                }
            }
        }
    }
    sccs
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

#[cfg(test)]
mod tests {
    use super::tinh_scc;

    /// **ST-315d review F1** — Tarjan lặp không được tràn stack theo độ sâu phụ thuộc.
    #[test]
    fn chuoi_100k_canh_sau_khong_tran_stack() {
        let n = 100_000usize;
        let canh: Vec<Vec<usize>> = (0..n)
            .map(|i| if i + 1 < n { vec![i + 1] } else { Vec::new() })
            .collect();
        let scc = tinh_scc(&canh);
        assert_eq!(scc.len(), n, "chuỗi 100k phải là 100k SCC đơn");
    }

    /// Vòng 100k đỉnh ⇒ **một** SCC (đường lặp, không đệ quy).
    #[test]
    fn vong_100k_dinh_la_mot_scc() {
        let n = 100_000usize;
        let canh: Vec<Vec<usize>> = (0..n).map(|i| vec![(i + 1) % n]).collect();
        let scc = tinh_scc(&canh);
        assert_eq!(scc.len(), 1);
        assert_eq!(scc[0].len(), n);
    }
}
