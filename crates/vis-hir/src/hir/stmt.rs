// SPDX-License-Identifier: MIT OR Apache-2.0
//! Câu lệnh HIR: [`HStmt`], [`HElse`], [`AssignOp`], [`HArm`], [`HCond`]. Tách nguyên văn từ
//! `hir.rs` (ST-318d).

use vis_syntax::Span;

use crate::ty::Type;

use super::expr::HExpr;
use super::pat::HPat;

/// Câu lệnh.
#[derive(Debug, Clone, PartialEq)]
pub enum HStmt {
    Let {
        pat: HPat,
        ty: Option<Type>,
        init: HExpr,
        else_: Option<HElse>,
        span: Span,
    },
    Var {
        name: String,
        span: Span,
        sym: u32,
        ty: Option<Type>,
        init: HExpr,
        is_self: bool,
    },
    Assign {
        target: HExpr,
        op: AssignOp,
        value: HExpr,
        span: Span,
    },
    Expr(HExpr),
    If {
        cond: HCond,
        then: Vec<HStmt>,
        elifs: Vec<(HCond, Vec<HStmt>)>,
        else_: Option<Vec<HStmt>>,
        span: Span,
    },
    For {
        pat: HPat,
        iter: HExpr,
        body: Vec<HStmt>,
        span: Span,
    },
    While {
        cond: HCond,
        body: Vec<HStmt>,
        span: Span,
    },
    Loop {
        body: Vec<HStmt>,
        span: Span,
    },
    Scope {
        label: Option<String>,
        body: Vec<HStmt>,
        span: Span,
    },
    Return {
        value: Option<HExpr>,
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
    Match {
        subject: HExpr,
        arms: Vec<HArm>,
        span: Span,
    },
    /// Khối lồng không gắn với cấu trúc điều khiển (giữ chỗ cho ST-12/13).
    Block {
        body: Vec<HStmt>,
        span: Span,
    },
}

impl HStmt {
    pub fn span(&self) -> Span {
        match self {
            HStmt::Let { span, .. }
            | HStmt::Var { span, .. }
            | HStmt::Assign { span, .. }
            | HStmt::If { span, .. }
            | HStmt::For { span, .. }
            | HStmt::While { span, .. }
            | HStmt::Loop { span, .. }
            | HStmt::Scope { span, .. }
            | HStmt::Return { span, .. }
            | HStmt::Break { span }
            | HStmt::Continue { span }
            | HStmt::Match { span, .. }
            | HStmt::Block { span, .. } => *span,
            HStmt::Expr(e) => e.span(),
        }
    }

    pub fn sexp(&self) -> String {
        match self {
            HStmt::Let { pat, init, .. } => format!("(let {} {})", pat.sexp(), init.sexp()),
            HStmt::Var { name, init, .. } => format!("(var {name} {})", init.sexp()),
            HStmt::Assign {
                target, op, value, ..
            } => format!("({} {} {})", op.text(), target.sexp(), value.sexp()),
            HStmt::Expr(e) => e.sexp(),
            HStmt::If { cond, .. } => format!("(if {})", cond.sexp()),
            HStmt::For { pat, iter, .. } => format!("(for {} {})", pat.sexp(), iter.sexp()),
            HStmt::While { cond, .. } => format!("(while {})", cond.sexp()),
            HStmt::Loop { .. } => "(loop)".into(),
            HStmt::Scope { label, .. } => match label {
                Some(l) => format!("(scope {l})"),
                None => "(scope)".into(),
            },
            HStmt::Return { value, .. } => match value {
                Some(v) => format!("(return {})", v.sexp()),
                None => "(return)".into(),
            },
            HStmt::Break { .. } => "(break)".into(),
            HStmt::Continue { .. } => "(continue)".into(),
            HStmt::Match { subject, .. } => format!("(match {})", subject.sexp()),
            HStmt::Block { .. } => "(block)".into(),
        }
    }
}

/// Nhánh `else` của `let` (giá trị dự phòng khi lỗi, §A.2).
#[derive(Debug, Clone, PartialEq)]
pub struct HElse {
    /// `else err: …` — tên nhận lỗi (khai trong khối `else`).
    pub bind: Option<String>,
    pub bind_sym: Option<u32>,
    pub body: Vec<HStmt>,
    pub span: Span,
}

/// Toán tử gán.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Set,
    Add,
    Sub,
    Mul,
    Div,
    /// **R7 (VD-178)**: `//` — chia lấy phần nguyên (làm tròn **xuống**).
    FloorDiv,
    Rem,
}

impl AssignOp {
    pub const fn text(self) -> &'static str {
        match self {
            AssignOp::Set => "=",
            AssignOp::Add => "+=",
            AssignOp::Sub => "-=",
            AssignOp::Mul => "*=",
            AssignOp::Div => "/=",
            AssignOp::FloorDiv => "//=",
            AssignOp::Rem => "%=",
        }
    }
}

/// Một arm của `match` (§A.2 `match_arm := pattern ['if' expr] ':' suite`).
#[derive(Debug, Clone, PartialEq)]
pub struct HArm {
    pub pat: HPat,
    pub guard: Option<HExpr>,
    pub body: Vec<HStmt>,
    pub span: Span,
}

/// Điều kiện `if`/`elif`: `if let PAT = EXPR` hoặc biểu thức thường (§A.2 `if_cond`).
#[derive(Debug, Clone, PartialEq)]
pub enum HCond {
    Expr(HExpr),
    Let { pat: HPat, value: HExpr, span: Span },
}

impl HCond {
    pub fn sexp(&self) -> String {
        match self {
            HCond::Expr(e) => e.sexp(),
            HCond::Let { pat, value, .. } => format!("(if-let {} {})", pat.sexp(), value.sexp()),
        }
    }
}
