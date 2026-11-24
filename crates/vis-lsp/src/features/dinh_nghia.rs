// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-269** — `textDocument/definition` (go-to-definition) cho LSP.
//!
//! Nguồn: HIR của tệp đang mở — item cấp tệp (fn/type/enum/alias/const/trait), tham số, trường,
//! biến thể enum, và binder của `let`/`var`/`for`/`if let`/`while let`/`match`. Trả **mảng**
//! `Location` (nhiều khai báo cùng tên ⇒ nhiều vị trí, sắp theo khoảng cách tới con trỏ, cap 20);
//! không tìm thấy ⇒ `result:null`.
//!
//! Vị trí dùng [`super::vi_tri::khoang`] (0-based UTF-16, tab = 1) trên văn bản **đã chuẩn hoá**
//! (`KetQua::text`), đúng quy ước completion/hover.

use vis_core::value::Value;
use vis_hir::hir::{HCond, HItem, HPat, HStmt, Hir};
use vis_syntax::Span;

use super::jsonrpc::{m, phan_hoi, s};
use super::phan_tich::KetQua;
use super::vi_tri::khoang;

/// Một khai báo có thể nhảy tới.
#[derive(Clone)]
pub struct DinhNghia {
    pub ten: String,
    pub span: Span,
}

fn them(out: &mut Vec<DinhNghia>, ten: &str, span: Span) {
    if ten.is_empty() || span.start >= span.end {
        return;
    }
    out.push(DinhNghia {
        ten: ten.to_string(),
        span,
    });
}

/// Binder trong pattern (đệ quy tuple/list/record/ctor/or).
fn pat(p: &HPat, out: &mut Vec<DinhNghia>) {
    match p {
        HPat::Bind { name, span, .. } => them(out, name, *span),
        HPat::Tuple { items, .. } | HPat::List { items, .. } => {
            for x in items {
                pat(x, out);
            }
        }
        HPat::Ctor { args, .. } => {
            for x in args {
                pat(x, out);
            }
        }
        HPat::Record { fields, .. } => {
            for (_, x) in fields {
                if let Some(x) = x {
                    pat(x, out);
                }
            }
        }
        HPat::Or { alts, .. } => {
            for x in alts {
                pat(x, out);
            }
        }
        _ => {}
    }
}

fn cond(c: &HCond, out: &mut Vec<DinhNghia>) {
    if let HCond::Let { pat: p, .. } = c {
        pat(p, out);
    }
}

fn fn_decl(f: &vis_hir::HFn, out: &mut Vec<DinhNghia>) {
    them(out, &f.name, f.span);
    for p in &f.params {
        them(out, &p.name, p.span);
    }
    // Biến cục bộ nằm trong thân hàm, không ở `hir.stmts`.
    for st in &f.body {
        stmt(st, out);
    }
}

fn stmt(st: &HStmt, out: &mut Vec<DinhNghia>) {
    match st {
        HStmt::Let { pat: p, else_, .. } => {
            pat(p, out);
            if let Some(e) = else_ {
                for x in &e.body {
                    stmt(x, out);
                }
            }
        }
        HStmt::Var { name, span, .. } => them(out, name, *span),
        HStmt::If {
            cond: c,
            then,
            elifs,
            else_,
            ..
        } => {
            cond(c, out);
            for x in then {
                stmt(x, out);
            }
            for (c, b) in elifs {
                cond(c, out);
                for x in b {
                    stmt(x, out);
                }
            }
            if let Some(b) = else_ {
                for x in b {
                    stmt(x, out);
                }
            }
        }
        HStmt::For { pat: p, body, .. } => {
            pat(p, out);
            for x in body {
                stmt(x, out);
            }
        }
        HStmt::While { cond: c, body, .. } => {
            cond(c, out);
            for x in body {
                stmt(x, out);
            }
        }
        HStmt::Loop { body, .. } | HStmt::Scope { body, .. } | HStmt::Block { body, .. } => {
            for x in body {
                stmt(x, out);
            }
        }
        HStmt::Match { arms, .. } => {
            for a in arms {
                pat(&a.pat, out);
                for x in &a.body {
                    stmt(x, out);
                }
            }
        }
        _ => {}
    }
}

/// Gom mọi khai báo của một HIR (giữ thứ tự nguồn).
pub fn gom(hir: &Hir) -> Vec<DinhNghia> {
    let mut out = Vec::new();
    for it in &hir.items {
        match it {
            HItem::Fn(f) => fn_decl(f, &mut out),
            HItem::Rec(r) => {
                them(&mut out, &r.name, r.span);
                for f in &r.fields {
                    them(&mut out, &f.name, f.span);
                }
            }
            HItem::Enum(e) => {
                them(&mut out, &e.name, e.span);
                for v in &e.variants {
                    them(&mut out, &v.name, v.span);
                    for f in &v.fields {
                        them(&mut out, &f.name, f.span);
                    }
                }
            }
            HItem::Alias(a) => them(&mut out, &a.name, a.span),
            HItem::Const(c) => them(&mut out, &c.name, c.span),
            HItem::Trait(t) => {
                them(&mut out, &t.name, t.span);
                for me in &t.methods {
                    fn_decl(me, &mut out);
                }
            }
            HItem::Impl(i) => {
                for me in &i.methods {
                    fn_decl(me, &mut out);
                }
            }
            HItem::Extern(x) => {
                them(&mut out, &x.name, x.span);
                for p in &x.params {
                    them(&mut out, &p.name, p.span);
                }
            }
            HItem::Use(_) => {}
        }
    }
    for st in &hir.stmts {
        stmt(st, &mut out);
    }
    out
}

/// Thu hẹp span khai báo về **đúng tên** (span HIR của `fn`/`type`/câu lệnh thường phủ cả khai
/// báo; LSP client sẽ chọn cả khối nếu trả span rộng). Tìm lần xuất hiện đầu tiên của tên như một
/// từ trọn vẹn trong span; không thấy ⇒ giữ span gốc.
pub(crate) fn thu_hep_ten(text: &str, ten: &str, span: Span) -> Span {
    let seg = &text[span.start..span.end];
    let mut start = 0;
    while let Some(i) = seg[start..].find(ten) {
        let a = start + i;
        let b = a + ten.len();
        let truoc = seg[..a].chars().next_back();
        let sau = seg[b..].chars().next();
        let ria = |c: Option<char>| c.is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
        if ria(truoc) && ria(sau) {
            return Span::new(span.start + a, span.start + b);
        }
        start = b;
    }
    span
}

/// Thu hẹp span khai báo về **đúng tên** (span HIR của `fn`/`type`/câu lệnh thường phủ cả khai
/// báo; LSP client sẽ chọn cả khối nếu trả span rộng). Tìm lần xuất hiện đầu tiên của tên như một
/// từ trọn vẹn trong span; không thấy ⇒ giữ span gốc.
fn thu_hep(text: &str, d: &DinhNghia) -> Span {
    thu_hep_ten(text, &d.ten, d.span)
}

/// Phản hồi `textDocument/references`: mọi lần xuất hiện **trọn từ** trong tệp (kể cả khai báo),
/// sắp theo vị trí, cap 200; không thấy ⇒ mảng rỗng.
pub fn tra_loi_references(kq: &KetQua, uri: &str, byte: usize, id: &Value) -> Vec<u8> {
    let (start, end) = super::goi_y::tu_tai(&kq.text, byte);
    if start == end {
        return phan_hoi(id, Value::List(Vec::new()));
    }
    let ten = &kq.text[start..end];
    let mut spans: Vec<Span> = Vec::new();
    let mut at = 0usize;
    while let Some(i) = kq.text[at..].find(ten) {
        let a = at + i;
        let b = a + ten.len();
        let truoc = kq.text[..a].chars().next_back();
        let sau = kq.text[b..].chars().next();
        let ria = |c: Option<char>| c.is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
        if ria(truoc) && ria(sau) {
            spans.push(Span::new(a, b));
            if spans.len() >= 200 {
                break;
            }
        }
        at = b;
    }
    let pos = |line: u32, character: u32| {
        m(vec![
            ("line", Value::Int(line as i64)),
            ("character", Value::Int(character as i64)),
        ])
    };
    let locs: Vec<Value> = spans
        .iter()
        .map(|span| {
            let ((l0, c0), (l1, c1)) = khoang(&kq.text, *span);
            m(vec![
                ("uri", s(uri)),
                (
                    "range",
                    m(vec![("start", pos(l0, c0)), ("end", pos(l1, c1))]),
                ),
            ])
        })
        .collect();
    phan_hoi(id, Value::List(locs))
}

pub fn tra_loi_definition(
    kq: &KetQua,
    uri: &str,
    byte: usize,
    root: Option<&std::path::Path>,
    id: &Value,
) -> Vec<u8> {
    let (start, end) = super::goi_y::tu_tai(&kq.text, byte);
    if start == end {
        return phan_hoi(id, Value::none());
    }
    let ten = &kq.text[start..end];
    let mut ds: Vec<&DinhNghia> = kq.dinh_nghia.iter().filter(|d| d.ten == ten).collect();
    if ds.is_empty() {
        return super::workspace::tra_loi_definition(root, ten, id);
    }
    // Gần con trỏ trước; cùng khoảng cách thì theo vị trí khai báo.
    ds.sort_by_key(|d| (d.span.start.abs_diff(start), d.span.start));
    ds.truncate(20);
    let pos = |line: u32, character: u32| {
        m(vec![
            ("line", Value::Int(line as i64)),
            ("character", Value::Int(character as i64)),
        ])
    };
    let locs: Vec<Value> = ds
        .iter()
        .map(|d| {
            let ((l0, c0), (l1, c1)) = khoang(&kq.text, thu_hep(&kq.text, d));
            m(vec![
                ("uri", s(uri)),
                (
                    "range",
                    m(vec![("start", pos(l0, c0)), ("end", pos(l1, c1))]),
                ),
            ])
        })
        .collect();
    phan_hoi(id, Value::List(locs))
}

/// **ST-282**: `textDocument/documentHighlight` — mọi token định danh trùng từ tại con trỏ trong
/// tệp (cùng lexer với rename nên bỏ chuỗi/chú thích), tối đa 200, `kind: 1` (Text).
pub fn tra_loi_highlights(kq: &KetQua, byte: usize, mode: vis_syntax::Mode, id: &Value) -> Vec<u8> {
    let (start, end) = super::goi_y::tu_tai(&kq.text, byte);
    if start == end {
        return phan_hoi(id, Value::List(Vec::new()));
    }
    let ten = &kq.text[start..end];
    let lexed = vis_syntax::lex(&kq.text, mode);
    let pos = |line: u32, character: u32| {
        m(vec![
            ("line", Value::Int(line as i64)),
            ("character", Value::Int(character as i64)),
        ])
    };
    let mut locs: Vec<Value> = Vec::new();
    for t in &lexed.tokens {
        if t.kind != vis_syntax::Kind::Ident || t.text != ten {
            continue;
        }
        let ((l0, c0), (l1, c1)) = khoang(&kq.text, t.span);
        locs.push(m(vec![
            (
                "range",
                m(vec![("start", pos(l0, c0)), ("end", pos(l1, c1))]),
            ),
            ("kind", Value::Int(1)),
        ]));
        if locs.len() >= 200 {
            break;
        }
    }
    phan_hoi(id, Value::List(locs))
}
