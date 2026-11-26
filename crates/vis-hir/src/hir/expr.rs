// SPDX-License-Identifier: MIT OR Apache-2.0
//! Biểu thức HIR: [`HExpr`], [`InterpPart`], [`HBody`] + toán tử [`BinOp`]/[`UnOp`]. Tách
//! nguyên văn từ `hir.rs` (ST-318d).

use vis_syntax::Span;

use crate::ty::{EnumId, Type};

use super::item::HParam;
use super::lit::Lit;
use super::stmt::{HArm, HCond, HStmt};

/// Biểu thức.
#[derive(Debug, Clone, PartialEq)]
pub enum HExpr {
    /// Literal **đã điền kiểu** (int/size/duration/percent/str/bool/bytes/path/char/float).
    Lit { lit: Lit, ty: Type, span: Span },
    /// Tên giá trị; `sym = None` khi chưa phân giải được (VD-44 #2: hạ permissive).
    Var {
        name: String,
        sym: Option<u32>,
        span: Span,
    },
    Bin {
        op: BinOp,
        lhs: Box<HExpr>,
        rhs: Box<HExpr>,
        span: Span,
    },
    Un {
        op: UnOp,
        expr: Box<HExpr>,
        span: Span,
    },
    Call {
        callee: Box<HExpr>,
        args: Vec<HExpr>,
        /// `--ten: giá_trị` (§A.3) — giữ thứ tự viết.
        named: Vec<(String, HExpr)>,
        span: Span,
    },
    Field {
        base: Box<HExpr>,
        name: String,
        /// `?.` (khác `?` postfix).
        optional: bool,
        span: Span,
    },
    Index {
        base: Box<HExpr>,
        index: Box<HExpr>,
        span: Span,
    },
    List {
        items: Vec<HExpr>,
        elem: Type,
        span: Span,
    },
    Set {
        items: Vec<HExpr>,
        elem: Type,
        span: Span,
    },
    Map {
        entries: Vec<(HExpr, HExpr)>,
        key: Type,
        value: Type,
        span: Span,
    },
    Record {
        fields: Vec<(String, HExpr)>,
        ty: Type,
        span: Span,
    },
    /// `Shape.Circle(1.0)` / `Circle(1.0)` khi tên là biến thể của enum **đã khai trong tệp**.
    EnumLit {
        enum_id: Option<EnumId>,
        path: Vec<String>,
        variant: String,
        args: Vec<HExpr>,
        ty: Type,
        span: Span,
    },
    /// `None` (value = `None`) và tự nâng `T → T?` (§4.4).
    Opt {
        value: Option<Box<HExpr>>,
        ty: Type,
        span: Span,
    },
    /// `expr?` — `?` postfix.
    Try { expr: Box<HExpr>, span: Span },
    /// `expr with ten: giá_trị …`.
    With {
        base: Box<HExpr>,
        fields: Vec<(String, HExpr)>,
        span: Span,
    },
    If {
        cond: Box<HCond>,
        then: Vec<HStmt>,
        elifs: Vec<(HCond, Vec<HStmt>)>,
        else_: Vec<HStmt>,
        ty: Type,
        span: Span,
    },
    Match {
        subject: Box<HExpr>,
        arms: Vec<HArm>,
        ty: Type,
        span: Span,
    },
    Lambda {
        params: Vec<HParam>,
        ret: Type,
        body: HBody,
        span: Span,
    },
    Tuple {
        items: Vec<HExpr>,
        ty: Type,
        span: Span,
    },
    /// `ref(expr)` — `ref[T]` là VS1, ST-11 chỉ giữ nút.
    Ref { expr: Box<HExpr>, span: Span },
    /// `await postfix` — async là VS1.
    Await { expr: Box<HExpr>, span: Span },
    /// **ST-60**: `spawn f(args)` — tạo tác vụ + đưa vào **hàng đợi FIFO**; callee phải là **tên**
    /// hàm `async` cấp tệp (tyck `VS0412` nếu không).
    /// **ST-61 (VD-96)**: `chan[T](n)` — tạo kênh đệm `n` (`n ≥ 0`), phần tử kiểu `T`.
    ChanNew {
        elem: Type,
        cap: Box<HExpr>,
        span: Span,
    },
    Spawn {
        callee: Box<HExpr>,
        args: Vec<HExpr>,
        named: Vec<(String, HExpr)>,
        span: Span,
    },
    /// `cmd { … }` — **chưa định kiểu** (VD-22: `cmd_lower` là VS1); giữ nguyên văn.
    CmdBlock { raw: String, span: Span },
    /// Chuỗi **có nội suy** `{expr}` (ST-14, VD-47(a)): các mảnh chữ + biểu thức đã hạ **trong cùng
    /// ngữ cảnh giải tên** của điểm dùng ⇒ chạy chỉ cần đánh giá, **không** parse lại.
    Interp { parts: Vec<InterpPart>, span: Span },
    /// Biểu thức không hạ được (giữ chỗ, **không** panic — VD-44 #6).
    Error { span: Span },
}

/// Một mảnh của chuỗi nội suy (ST-14).
#[derive(Debug, Clone, PartialEq)]
pub enum InterpPart {
    /// Chữ thường, **đã giải escape** (`\n`, `{{` → `{`, …).
    Str(String),
    /// `{expr}` — hạ như biểu thức bình thường.
    Expr(Box<HExpr>),
    /// **R6c (VD-178)**: `{expr:spec}` — định dạng lúc chạy (`.N` = N chữ số thập phân).
    ExprSpec { expr: Box<HExpr>, spec: String },
}

impl HExpr {
    pub fn span(&self) -> Span {
        match self {
            HExpr::Lit { span, .. }
            | HExpr::Var { span, .. }
            | HExpr::Bin { span, .. }
            | HExpr::Un { span, .. }
            | HExpr::Call { span, .. }
            | HExpr::Field { span, .. }
            | HExpr::Index { span, .. }
            | HExpr::List { span, .. }
            | HExpr::Set { span, .. }
            | HExpr::Map { span, .. }
            | HExpr::Record { span, .. }
            | HExpr::EnumLit { span, .. }
            | HExpr::Opt { span, .. }
            | HExpr::Try { span, .. }
            | HExpr::With { span, .. }
            | HExpr::If { span, .. }
            | HExpr::Match { span, .. }
            | HExpr::Lambda { span, .. }
            | HExpr::Tuple { span, .. }
            | HExpr::Ref { span, .. }
            | HExpr::Await { span, .. }
            | HExpr::Spawn { span, .. }
            | HExpr::ChanNew { span, .. }
            | HExpr::Interp { span, .. }
            | HExpr::CmdBlock { span, .. }
            | HExpr::Error { span } => *span,
        }
    }

    /// Sexp theo quy ước ở đầu tệp (VD-34 **không** nút `paren`).
    pub fn sexp(&self) -> String {
        match self {
            HExpr::Lit { lit, .. } => lit.text.clone(),
            HExpr::Var { name, .. } => name.clone(),
            HExpr::Bin { op, lhs, rhs, .. } => {
                format!("({} {} {})", op.text(), lhs.sexp(), rhs.sexp())
            }
            HExpr::Un { op, expr, .. } => format!("({} {})", op.text(), expr.sexp()),
            HExpr::Call {
                callee,
                args,
                named,
                ..
            } => {
                let mut s = format!("(call {}", callee.sexp());
                for a in args {
                    s.push(' ');
                    s.push_str(&a.sexp());
                }
                for (n, v) in named {
                    s.push_str(&format!(" --{n}: {}", v.sexp()));
                }
                s.push(')');
                s
            }
            HExpr::Field {
                base,
                name,
                optional,
                ..
            } => {
                let tag = if *optional { "?field" } else { "field" };
                format!("({tag} {} {name})", base.sexp())
            }
            HExpr::Index { base, index, .. } => {
                format!("(index {} {})", base.sexp(), index.sexp())
            }
            HExpr::List { items, .. } => tagged_items("list", items),
            HExpr::Set { items, .. } => tagged_items("set", items),
            HExpr::Map { entries, .. } => {
                let mut s = String::from("(map");
                for (k, v) in entries {
                    s.push(' ');
                    s.push_str(&k.sexp());
                    s.push(' ');
                    s.push_str(&v.sexp());
                }
                s.push(')');
                s
            }
            HExpr::Record { fields, .. } => {
                let mut s = String::from("(record");
                for (n, v) in fields {
                    s.push_str(&format!(" {n}: {}", v.sexp()));
                }
                s.push(')');
                s
            }
            HExpr::EnumLit {
                path,
                variant,
                args,
                ..
            } => {
                let mut name = path.clone();
                if name.last().map(String::as_str) != Some(variant.as_str()) {
                    name.push(variant.clone());
                }
                let mut s = format!("(enumlit {}", name.join("."));
                for a in args {
                    s.push(' ');
                    s.push_str(&a.sexp());
                }
                s.push(')');
                s
            }
            HExpr::Opt { value, .. } => match value {
                None => "None".into(),
                Some(v) => format!("(opt {})", v.sexp()),
            },
            HExpr::Try { expr, .. } => format!("(try {})", expr.sexp()),
            HExpr::With { base, fields, .. } => {
                let mut s = format!("(with {}", base.sexp());
                for (n, v) in fields {
                    s.push_str(&format!(" {n}: {}", v.sexp()));
                }
                s.push(')');
                s
            }
            HExpr::If { cond, .. } => format!("(if-expr {})", cond.sexp()),
            HExpr::Match { subject, .. } => format!("(match-expr {})", subject.sexp()),
            HExpr::Lambda { params, body, .. } => {
                let p: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
                format!("(lambda ({}) {})", p.join(" "), body.sexp())
            }
            HExpr::Tuple { items, .. } => tagged_items("tuple", items),
            HExpr::Ref { expr, .. } => format!("(ref {})", expr.sexp()),
            HExpr::Await { expr, .. } => format!("(await {})", expr.sexp()),
            HExpr::ChanNew { elem, cap, .. } => {
                format!("(chan {} {})", elem.name(), cap.sexp())
            }
            HExpr::Spawn {
                callee, args, span, ..
            } => {
                let _ = span;
                let a: Vec<String> = args.iter().map(HExpr::sexp).collect();
                format!(
                    "(spawn {}{})",
                    callee.sexp(),
                    if a.is_empty() {
                        String::new()
                    } else {
                        format!(" {}", a.join(" "))
                    }
                )
            }
            HExpr::CmdBlock { raw, .. } => format!("(cmd-block {})", raw.trim()),
            HExpr::Interp { parts, .. } => {
                let mut out = String::from("(interp");
                for p in parts {
                    match p {
                        InterpPart::Str(t) => out.push_str(&format!(" \"{t}\"")),
                        InterpPart::Expr(e) => out.push_str(&format!(" {}", e.sexp())),
                        InterpPart::ExprSpec { expr, spec } => {
                            out.push_str(&format!(" (spec \"{spec}\" {})", expr.sexp()))
                        }
                    }
                }
                out.push(')');
                out
            }
            HExpr::Error { .. } => "{error}".into(),
        }
    }
}

fn tagged_items(tag: &str, items: &[HExpr]) -> String {
    let mut s = format!("({tag}");
    for i in items {
        s.push(' ');
        s.push_str(&i.sexp());
    }
    s.push(')');
    s
}

/// Thân lambda: biểu thức (`x => x * 2`) hoặc khối (`fn(x: int) -> int: …`).
#[derive(Debug, Clone, PartialEq)]
pub enum HBody {
    Expr(Box<HExpr>),
    Stmts(Vec<HStmt>),
}

impl HBody {
    pub fn sexp(&self) -> String {
        match self {
            HBody::Expr(e) => e.sexp(),
            HBody::Stmts(_) => "(stmts)".into(),
        }
    }
}

/// Toán tử nhị phân (đúng 12 mức ưu tiên A.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    /// **R7 (VD-178)**: `//` — chia lấy phần nguyên (làm tròn **xuống**).
    FloorDiv,
    Rem,
    Pow,
    Range,
    RangeIncl,
    Coalesce,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Pipe,
}

impl BinOp {
    pub const fn text(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::FloorDiv => "//",
            BinOp::Rem => "%",
            BinOp::Pow => "**",
            BinOp::Range => "..",
            BinOp::RangeIncl => "..=",
            BinOp::Coalesce => "??",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::And => "and",
            BinOp::Or => "or",
            BinOp::Pipe => "|>",
        }
    }

    /// Từ chữ viết nguồn (§A.5).
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "+" => BinOp::Add,
            "-" => BinOp::Sub,
            "*" => BinOp::Mul,
            "/" => BinOp::Div,
            "//" => BinOp::FloorDiv,
            "%" => BinOp::Rem,
            "**" => BinOp::Pow,
            ".." => BinOp::Range,
            "..=" => BinOp::RangeIncl,
            "??" => BinOp::Coalesce,
            "==" => BinOp::Eq,
            "!=" => BinOp::Ne,
            "<" => BinOp::Lt,
            "<=" => BinOp::Le,
            ">" => BinOp::Gt,
            ">=" => BinOp::Ge,
            "and" => BinOp::And,
            "or" => BinOp::Or,
            "|>" => BinOp::Pipe,
            _ => return None,
        })
    }
}

/// Toán tử một ngôi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
}

impl UnOp {
    pub const fn text(self) -> &'static str {
        match self {
            UnOp::Neg => "-",
            UnOp::Not => "not",
        }
    }
}
