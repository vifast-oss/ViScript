// SPDX-License-Identifier: MIT OR Apache-2.0
//! Pattern HIR: [`HPat`] + `span`/`sexp`/`else_binds_err`. Tách nguyên văn từ `hir.rs` (ST-318d).

use vis_syntax::Span;

use super::lit::Lit;

/// Pattern (§A.4).
#[derive(Debug, Clone, PartialEq)]
pub enum HPat {
    Wild {
        span: Span,
    },
    Bind {
        name: String,
        span: Span,
        sym: u32,
    },
    Lit {
        lit: Lit,
        span: Span,
    },
    /// Biến thể enum / constructor: `Circle(r)`, `Some(x)`, `Ok(v)`, `Empty`, `None`.
    Ctor {
        path: Vec<String>,
        args: Vec<HPat>,
        span: Span,
    },
    Tuple {
        items: Vec<HPat>,
        span: Span,
    },
    List {
        items: Vec<HPat>,
        rest: Option<Box<HPat>>,
        span: Span,
    },
    Record {
        name: Option<String>,
        fields: Vec<(String, Option<HPat>)>,
        open: bool,
        span: Span,
    },
    Range {
        lo: Option<Box<HPat>>,
        hi: Option<Box<HPat>>,
        inclusive: bool,
        span: Span,
    },
    Or {
        alts: Vec<HPat>,
        span: Span,
    },
    /// `..` / `..rest` trong pattern list/bản ghi.
    Rest {
        name: Option<String>,
        span: Span,
    },
}

impl HPat {
    /// **VD-263** — binder của `let … else` (`else e`) chỉ nhận payload lỗi khi nhánh **không
    /// khớp** là `Err`:
    /// - tên trần (`let x = r else e`) trên `Result` ⇒ `Err` (payload E);
    /// - `Ok(x)`/`Ok(_)`/`Ok()` ⇒ nhánh không khớp chắc chắn là `Err` ⇒ payload E;
    /// - `Err(e)`, `Some(x)`, `None`, `Ok(Some(x))` (có thể thất bại vì `Ok(None)`) và mọi pattern
    ///   khác ⇒ `()`.
    ///
    /// Pattern tên trần trên `Option` cũng là `()` — checker quyết theo kiểu tĩnh; runtime dùng
    /// hàm này rồi chỉ lấy payload khi giá trị thật là `Err` (nên `None` vẫn ra `()`).
    pub fn else_binds_err(&self) -> bool {
        match self {
            HPat::Bind { .. } => true,
            // Chỉ `Ok(x)`/`Ok(_)`/`Ok()` (pattern con **không bác được**) mới chắc chắn thất bại
            // vì `Err`. `Ok(Some(x))` có thể thất bại vì `Ok(None)` ⇒ binder phải là `()` —
            // nếu lấy payload E trong trường hợp đó sẽ gán sai kiểu.
            HPat::Ctor { path, args, .. } => {
                path.last().map(String::as_str) == Some("Ok")
                    && (args.is_empty()
                        || (args.len() == 1
                            && matches!(args[0], HPat::Bind { .. } | HPat::Wild { .. })))
            }
            _ => false,
        }
    }

    pub fn span(&self) -> Span {
        match self {
            HPat::Wild { span }
            | HPat::Bind { span, .. }
            | HPat::Lit { span, .. }
            | HPat::Ctor { span, .. }
            | HPat::Tuple { span, .. }
            | HPat::List { span, .. }
            | HPat::Record { span, .. }
            | HPat::Range { span, .. }
            | HPat::Or { span, .. }
            | HPat::Rest { span, .. } => *span,
        }
    }

    pub fn sexp(&self) -> String {
        match self {
            HPat::Wild { .. } => "_".into(),
            HPat::Bind { name, .. } => name.clone(),
            HPat::Lit { lit, .. } => lit.text.clone(),
            HPat::Ctor { path, args, .. } => {
                if args.is_empty() {
                    path.join(".")
                } else {
                    let a: Vec<String> = args.iter().map(HPat::sexp).collect();
                    format!("({} {})", path.join("."), a.join(" "))
                }
            }
            HPat::Tuple { items, .. } => {
                let a: Vec<String> = items.iter().map(HPat::sexp).collect();
                format!("(tuple {})", a.join(" "))
            }
            HPat::List { items, rest, .. } => {
                let mut a: Vec<String> = items.iter().map(HPat::sexp).collect();
                if let Some(r) = rest {
                    a.push(r.sexp());
                }
                format!("(list {})", a.join(" "))
            }
            HPat::Record { fields, open, .. } => {
                let mut a: Vec<String> = fields
                    .iter()
                    .map(|(n, p)| match p {
                        Some(p) => format!("{n}: {}", p.sexp()),
                        None => n.clone(),
                    })
                    .collect();
                if *open {
                    a.push("..".into());
                }
                format!("(record {})", a.join(" "))
            }
            HPat::Range {
                lo, hi, inclusive, ..
            } => {
                let l = lo.as_ref().map(|b| b.sexp()).unwrap_or_default();
                let h = hi.as_ref().map(|b| b.sexp()).unwrap_or_default();
                format!("({} {l} {h})", if *inclusive { "..=" } else { ".." })
            }
            HPat::Or { alts, .. } => {
                let a: Vec<String> = alts.iter().map(HPat::sexp).collect();
                format!("(or {})", a.join(" "))
            }
            HPat::Rest { name, .. } => match name {
                Some(n) => format!("..{n}"),
                None => "..".into(),
            },
        }
    }
}
