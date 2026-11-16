// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-122 (VD-127)** — audit: **mọi node HIR** của một đơn vị **đa tệp** phải mang `Span.file` của
//! **tệp sinh ra nó**. Node do lower **tổng hợp** (không copy từ CST: `HExpr::Error`, `lit()` của chế
//! độ lệnh, thân `cmd` re-lex, `where`/closure qua Lowerer con, `get`/`select`…) trước ST-122 dựng
//! `Span::new(a, b)` ⇒ `file = 0`, tức **tệp nạp đầu tiên** (hậu duyệt DFS), không phải "tệp chính":
//! xem `modules.rs` — `file_id` cấp theo **thứ tự nạp**, tệp chính **cuối** cùng.
//!
//! Test **tự động duyệt hết fixture**, không hardcode danh sách: quét `test/**`, lấy mọi tệp có
//! `use ./…`/`use pkg.…` (chỉ hai dạng này mới nạp nhiều tệp), nạp bằng **đúng đường của sản phẩm**
//! ([`crate::modules::load_full`]), rồi so `span.file` của từng node với chỉ số của tệp khai item.
//!
//! Chạy trong **bin crate** (không phải `tests/`) vì `modules` là module nội bộ của binary `vis`.

use std::path::{Path, PathBuf};

use vis_syntax::Span;
use vis_types::{HArm, HBody, HCond, HExpr, HItem, HPat, HStmt};

use crate::modules;

/// Gốc `viscript/` (test chạy với `CARGO_MANIFEST_DIR = viscript/crates/vis-cli`).
fn goc() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Mọi tệp `.vis`/`.vish` dưới `dir` (đệ quy), sắp xếp để **tất định**.
fn quet(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut ds: Vec<PathBuf> = rd.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    ds.sort();
    for p in ds {
        let ten = p.file_name().map(|s| s.to_string_lossy().to_string());
        if ten.as_deref() == Some("target") {
            continue;
        }
        if p.is_dir() {
            quet(&p, out);
        } else if matches!(
            p.extension().and_then(|x| x.to_str()),
            Some("vis") | Some("vish")
        ) {
            out.push(p);
        }
    }
}

/// Tệp **không** thể nạp nhiều tệp nếu văn bản không có `use ./`/`use pkg.` — lọc rẻ trước khi nạp.
fn co_the_da_tep(text: &str) -> bool {
    text.lines().any(|l| {
        let t = l.trim_start();
        let Some(rest) = t.strip_prefix("use ") else {
            return false;
        };
        rest.trim_start().starts_with("./") || rest.trim_start().starts_with("pkg.")
    })
}

// ----- thu span của mọi node HIR -----

type Nut = (&'static str, Span);

fn them(v: &mut Vec<Nut>, ten: &'static str, sp: Span) {
    v.push((ten, sp));
}

fn thu_expr(e: &HExpr, v: &mut Vec<Nut>) {
    them(v, "HExpr", e.span());
    match e {
        HExpr::Lit { .. } | HExpr::Var { .. } | HExpr::CmdBlock { .. } | HExpr::Error { .. } => {}
        HExpr::Bin { lhs, rhs, .. } => {
            thu_expr(lhs, v);
            thu_expr(rhs, v);
        }
        HExpr::Un { expr, .. } | HExpr::Try { expr, .. } | HExpr::Ref { expr, .. } => {
            thu_expr(expr, v)
        }
        HExpr::Await { expr, .. } => thu_expr(expr, v),
        HExpr::Call {
            callee,
            args,
            named,
            ..
        } => {
            thu_expr(callee, v);
            for a in args {
                thu_expr(a, v);
            }
            for (_, a) in named {
                thu_expr(a, v);
            }
        }
        HExpr::Field { base, .. } => thu_expr(base, v),
        HExpr::Index { base, index, .. } => {
            thu_expr(base, v);
            thu_expr(index, v);
        }
        HExpr::List { items, .. } | HExpr::Set { items, .. } | HExpr::Tuple { items, .. } => {
            for i in items {
                thu_expr(i, v);
            }
        }
        HExpr::Map { entries, .. } => {
            for (k, val) in entries {
                thu_expr(k, v);
                thu_expr(val, v);
            }
        }
        HExpr::Record { fields, .. } => {
            for (_, x) in fields {
                thu_expr(x, v);
            }
        }
        HExpr::EnumLit { args, .. } => {
            for a in args {
                thu_expr(a, v);
            }
        }
        HExpr::Opt { value, .. } => {
            if let Some(x) = value {
                thu_expr(x, v);
            }
        }
        HExpr::With { base, fields, .. } => {
            thu_expr(base, v);
            for (_, x) in fields {
                thu_expr(x, v);
            }
        }
        HExpr::If {
            cond,
            then,
            elifs,
            else_,
            ..
        } => {
            thu_cond(cond, v);
            for s in then {
                thu_stmt(s, v);
            }
            for (c, ss) in elifs {
                thu_cond(c, v);
                for s in ss {
                    thu_stmt(s, v);
                }
            }
            for s in else_ {
                thu_stmt(s, v);
            }
        }
        HExpr::Match { subject, arms, .. } => {
            thu_expr(subject, v);
            for a in arms {
                thu_arm(a, v);
            }
        }
        HExpr::Lambda { params, body, .. } => {
            for p in params {
                them(v, "HParam", p.span);
                if let Some(d) = &p.default {
                    thu_expr(d, v);
                }
            }
            match body {
                HBody::Expr(x) => thu_expr(x, v),
                HBody::Stmts(ss) => {
                    for s in ss {
                        thu_stmt(s, v);
                    }
                }
            }
        }
        HExpr::Spawn {
            callee,
            args,
            named,
            ..
        } => {
            thu_expr(callee, v);
            for a in args {
                thu_expr(a, v);
            }
            for (_, a) in named {
                thu_expr(a, v);
            }
        }
        HExpr::ChanNew { cap, .. } => thu_expr(cap, v),
        HExpr::Interp { parts, .. } => {
            for p in parts {
                if let vis_types::InterpPart::Expr(x) = p {
                    thu_expr(x, v);
                }
            }
        }
    }
}

fn thu_cond(c: &HCond, v: &mut Vec<Nut>) {
    match c {
        HCond::Expr(e) => thu_expr(e, v),
        HCond::Let { pat, value, span } => {
            them(v, "HCond", *span);
            thu_pat(pat, v);
            thu_expr(value, v);
        }
    }
}

fn thu_pat(p: &HPat, v: &mut Vec<Nut>) {
    them(v, "HPat", p.span());
    match p {
        HPat::Wild { .. } | HPat::Bind { .. } | HPat::Lit { .. } | HPat::Rest { .. } => {}
        HPat::Ctor { args, .. } => {
            for a in args {
                thu_pat(a, v);
            }
        }
        HPat::Tuple { items, .. } | HPat::Or { alts: items, .. } => {
            for i in items {
                thu_pat(i, v);
            }
        }
        HPat::List { items, rest, .. } => {
            for i in items {
                thu_pat(i, v);
            }
            if let Some(r) = rest {
                thu_pat(r, v);
            }
        }
        HPat::Record { fields, .. } => {
            for (_, f) in fields {
                if let Some(f) = f {
                    thu_pat(f, v);
                }
            }
        }
        HPat::Range { lo, hi, .. } => {
            if let Some(x) = lo {
                thu_pat(x, v);
            }
            if let Some(x) = hi {
                thu_pat(x, v);
            }
        }
    }
}

fn thu_arm(a: &HArm, v: &mut Vec<Nut>) {
    them(v, "HArm", a.span);
    thu_pat(&a.pat, v);
    if let Some(g) = &a.guard {
        thu_expr(g, v);
    }
    for s in &a.body {
        thu_stmt(s, v);
    }
}

fn thu_stmt(s: &HStmt, v: &mut Vec<Nut>) {
    them(v, "HStmt", s.span());
    match s {
        HStmt::Let {
            pat, init, else_, ..
        } => {
            thu_pat(pat, v);
            thu_expr(init, v);
            if let Some(e) = else_ {
                for s in &e.body {
                    thu_stmt(s, v);
                }
            }
        }
        HStmt::Var { init, .. } => thu_expr(init, v),
        HStmt::Assign { target, value, .. } => {
            thu_expr(target, v);
            thu_expr(value, v);
        }
        HStmt::Expr(e) => thu_expr(e, v),
        HStmt::If {
            cond,
            then,
            elifs,
            else_,
            ..
        } => {
            thu_cond(cond, v);
            for s in then {
                thu_stmt(s, v);
            }
            for (c, ss) in elifs {
                thu_cond(c, v);
                for s in ss {
                    thu_stmt(s, v);
                }
            }
            if let Some(ss) = else_ {
                for s in ss {
                    thu_stmt(s, v);
                }
            }
        }
        HStmt::For {
            pat, iter, body, ..
        } => {
            thu_pat(pat, v);
            thu_expr(iter, v);
            for s in body {
                thu_stmt(s, v);
            }
        }
        HStmt::While { cond, body, .. } => {
            thu_expr(cond, v);
            for s in body {
                thu_stmt(s, v);
            }
        }
        HStmt::Loop { body, .. } | HStmt::Scope { body, .. } | HStmt::Block { body, .. } => {
            for s in body {
                thu_stmt(s, v);
            }
        }
        HStmt::Return { value, .. } => {
            if let Some(x) = value {
                thu_expr(x, v);
            }
        }
        HStmt::Break { .. } | HStmt::Continue { .. } => {}
        HStmt::Match { subject, arms, .. } => {
            thu_expr(subject, v);
            for a in arms {
                thu_arm(a, v);
            }
        }
    }
}

fn thu_item(it: &HItem, v: &mut Vec<Nut>) {
    them(v, "HItem", it.span());
    match it {
        HItem::Fn(f) => {
            for p in &f.params {
                them(v, "HParam", p.span);
                if let Some(d) = &p.default {
                    thu_expr(d, v);
                }
            }
            for s in &f.body {
                thu_stmt(s, v);
            }
        }
        HItem::Rec(r) => {
            for f in &r.fields {
                them(v, "HField", f.span);
                if let Some(d) = &f.default {
                    thu_expr(d, v);
                }
            }
        }
        HItem::Enum(e) => {
            for var in &e.variants {
                them(v, "HVariant", var.span);
                for f in &var.fields {
                    them(v, "HField", f.span);
                    if let Some(d) = &f.default {
                        thu_expr(d, v);
                    }
                }
            }
        }
        HItem::Const(c) => thu_expr(&c.value, v),
        HItem::Trait(t) => {
            for m in &t.methods {
                for p in &m.params {
                    them(v, "HParam", p.span);
                }
                for s in &m.body {
                    thu_stmt(s, v);
                }
            }
        }
        HItem::Impl(i) => {
            for m in &i.methods {
                for p in &m.params {
                    them(v, "HParam", p.span);
                }
                for s in &m.body {
                    thu_stmt(s, v);
                }
            }
        }
        HItem::Extern(x) => {
            for p in &x.params {
                them(v, "HParam", p.span);
            }
        }
        HItem::Alias(_) | HItem::Use(_) => {}
    }
}

/// Kết quả audit một đơn vị đa tệp.
#[derive(Default, Debug)]
pub struct Kq {
    /// Số tệp của đơn vị (> 1 ⇒ đa tệp).
    pub so_tep: usize,
    /// Số node đã kiểm.
    pub so_node: usize,
    /// Node có `file` **khác** tệp khai item — vi phạm VD-127.
    pub sai: Vec<String>,
    /// Node có `file == 0` mà tệp khai item **không** phải tệp nạp đầu — đúng dạng nợ R22 §5 #103.
    pub file_0_sai: usize,
}

/// `rel → file_id` của một đơn vị đã nạp.
///
/// `file_id` cấp **tiền tự** (tệp chính = 0, rồi phụ thuộc theo thứ tự `use`) trong
/// `modules.rs` (khối `let file_id = self.file_id; … parse_file(&text, mode, file_id)`), còn
/// `Loaded::files` xếp **hậu duyệt** (phụ thuộc trước, chính cuối) ⇒ phải dựng lại bản đồ từ
/// `Loaded::deps` — chính là đồ thị `use` mà loader đã duyệt đúng thứ tự.
fn ban_do_file(l: &modules::Loaded) -> Vec<(String, usize)> {
    let mut con: std::collections::HashMap<&str, &Vec<String>> = std::collections::HashMap::new();
    for (rel, ds) in &l.deps {
        con.insert(rel.as_str(), ds);
    }
    fn tham(
        rel: &str,
        con: &std::collections::HashMap<&str, &Vec<String>>,
        da: &mut Vec<String>,
        out: &mut Vec<(String, usize)>,
    ) {
        if da.iter().any(|x| x == rel) {
            return;
        }
        da.push(rel.to_string());
        out.push((rel.to_string(), out.len()));
        if let Some(ds) = con.get(rel) {
            for d in ds.iter() {
                tham(d, con, da, out);
            }
        }
    }
    let mut out = Vec::new();
    if let Some(chinh) = l.files.last() {
        let mut da = Vec::new();
        tham(&chinh.rel, &con, &mut da, &mut out);
    }
    out
}

/// Audit **một** đơn vị đã nạp: mỗi node phải mang `file` = chỉ số tệp khai item chứa nó.
fn audit(l: &modules::Loaded) -> Kq {
    let mut kq = Kq {
        so_tep: l.files.len(),
        ..Kq::default()
    };
    let bd = ban_do_file(l);
    let idx = |rel: &str| bd.iter().find(|(r, _)| r == rel).map(|(_, i)| *i);
    let kiem = |mo_ta: &str, rel: &str, mong: Option<usize>, v: &Vec<Nut>, kq: &mut Kq| {
        let Some(mong) = mong else { return };
        for (ten, sp) in v {
            kq.so_node += 1;
            if sp.file as usize != mong {
                if sp.file == 0 && mong != 0 {
                    kq.file_0_sai += 1;
                }
                kq.sai.push(format!(
                    "{rel}: {ten} ({mo_ta}) có file={} (mong {mong}), span {}..{}",
                    sp.file, sp.start, sp.end
                ));
            }
        }
    };
    for (i, it) in l.hir.items.iter().enumerate() {
        let Some(rel) = l.item_rel.get(i) else {
            continue;
        };
        let mut v: Vec<Nut> = Vec::new();
        thu_item(it, &mut v);
        kiem("item", rel, idx(rel), &v, &mut kq);
    }
    // **`hir.stmts` chỉ của tệp chính** (`modules.rs`: `self.stmts = out.hir.stmts` khi `rel == chính`)
    // ⇒ tệp chính là tệp **nạp cuối** (hậu duyệt), không phải tệp 0.
    if let Some((chinh, so)) = bd.first() {
        let mut v: Vec<Nut> = Vec::new();
        for s in &l.hir.stmts {
            thu_stmt(s, &mut v);
        }
        kiem("stmt cấp tệp", chinh, Some(*so), &v, &mut kq);
    }
    kq
}

/// Nạp + audit một tệp chính; `None` khi không đa tệp (hoặc không nạp được).
fn audit_tep(chinh: &Path) -> Option<Kq> {
    let text = std::fs::read_to_string(chinh).ok()?;
    if !co_the_da_tep(&text) {
        return None;
    }
    let s = chinh.to_string_lossy().to_string();
    // `kiem_kieu = false`: audit **HIR**, không cần tyck (nhanh + không phụ thuộc chẩn đoán).
    let l = modules::load_full(&s, true, false).ok()?;
    if l.files.len() < 2 {
        return None;
    }
    Some(audit(&l))
}

/// **Ca chính (VD-127)**: duyệt **mọi** fixture đa tệp dưới `test/` ⇒ **0** node mang sai `file`.
#[test]
fn moi_node_hir_mang_dung_file_cua_tep_sinh_ra_no() {
    let mut ds: Vec<PathBuf> = Vec::new();
    quet(&goc().join("test"), &mut ds);
    assert!(
        ds.len() >= 400,
        "phải quét được bộ fixture (thấy {} tệp) — sai đường dẫn gốc?",
        ds.len()
    );
    let mut so_don_vi = 0usize;
    let mut so_node = 0usize;
    let mut file_0_sai = 0usize;
    let mut sai: Vec<String> = Vec::new();
    let mut ds_don_vi: Vec<String> = Vec::new();
    for p in &ds {
        let Some(kq) = audit_tep(p) else {
            continue;
        };
        so_don_vi += 1;
        so_node += kq.so_node;
        file_0_sai += kq.file_0_sai;
        ds_don_vi.push(format!(
            "{} ({} tệp, {} node, {} sai)",
            p.strip_prefix(goc()).unwrap_or(p).display(),
            kq.so_tep,
            kq.so_node,
            kq.sai.len()
        ));
        for s in kq.sai {
            sai.push(format!(
                "{}: {s}",
                p.strip_prefix(goc()).unwrap_or(p).display()
            ));
        }
    }
    eprintln!(
        "ST-122 audit: {so_don_vi} đơn vị đa tệp, {so_node} node HIR, \
         {} node sai tệp (trong đó {file_0_sai} node file=0 sai tệp)",
        sai.len()
    );
    for d in &ds_don_vi {
        eprintln!("  - {d}");
    }
    assert!(
        so_don_vi >= 10,
        "phải audit được ≥10 đơn vị đa tệp (thấy {so_don_vi}) — fixture thiếu?"
    );
    assert!(
        sai.is_empty(),
        "{} node HIR mang **sai tệp** (VD-127: node do lower sinh phải mang file của node nguồn):\n{}",
        sai.len(),
        sai.join("\n")
    );
}
