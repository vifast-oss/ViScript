// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-311d (VD-286)** — test đơn vị của HIR (`hir.rs`), tách ra cùng thư mục.

use super::*;

fn var(n: &str) -> HExpr {
    HExpr::Var {
        name: n.into(),
        sym: None,
        span: Span::new(0, 0),
    }
}

fn lit(k: LitKind, t: &str) -> HExpr {
    let l = Lit {
        kind: k,
        text: t.into(),
    };
    HExpr::Lit {
        ty: l.ty(),
        lit: l,
        span: Span::new(0, 0),
    }
}

#[test]
fn sexp_theo_quy_uoc_vd34_khong_paren() {
    let e = HExpr::Bin {
        op: BinOp::Mul,
        lhs: Box::new(HExpr::Bin {
            op: BinOp::Add,
            lhs: Box::new(lit(LitKind::Int, "1")),
            rhs: Box::new(lit(LitKind::Int, "2")),
            span: Span::new(0, 0),
        }),
        rhs: Box::new(lit(LitKind::Int, "3")),
        span: Span::new(0, 0),
    };
    assert_eq!(e.sexp(), "(* (+ 1 2) 3)");
    assert_eq!(var("a").sexp(), "a");
    let call = HExpr::Call {
        callee: Box::new(var("f")),
        args: vec![lit(LitKind::Str, "\"a\"")],
        named: vec![],
        span: Span::new(0, 0),
    };
    assert_eq!(call.sexp(), "(call f \"a\")");
    let fld = HExpr::Field {
        base: Box::new(var("a")),
        name: "b".into(),
        optional: true,
        span: Span::new(0, 0),
    };
    assert_eq!(fld.sexp(), "(?field a b)");
    let rec = HExpr::Record {
        fields: vec![("age".into(), lit(LitKind::Int, "20"))],
        ty: Type::Error,
        span: Span::new(0, 0),
    };
    assert_eq!(rec.sexp(), "(record age: 20)");
    let m = HExpr::Map {
        entries: vec![(lit(LitKind::Str, "\"a\""), lit(LitKind::Int, "1"))],
        key: Type::Str,
        value: Type::Int,
        span: Span::new(0, 0),
    };
    assert_eq!(m.sexp(), "(map \"a\" 1)");
    assert_eq!(
        HExpr::Opt {
            value: None,
            ty: Type::Error,
            span: Span::new(0, 0)
        }
        .sexp(),
        "None"
    );
}

#[test]
fn toan_tu_va_pattern_co_chu_viet() {
    for op in [
        BinOp::Add,
        BinOp::Range,
        BinOp::Coalesce,
        BinOp::And,
        BinOp::Pipe,
    ] {
        assert_eq!(BinOp::parse(op.text()), Some(op));
    }
    assert_eq!(BinOp::parse("!"), None);
    assert_eq!(AssignOp::Add.text(), "+=");
    assert_eq!(UnOp::Neg.text(), "-");
    assert_eq!(
        HPat::Or {
            alts: vec![
                HPat::Ctor {
                    path: vec!["Circle".into()],
                    args: vec![HPat::Bind {
                        name: "r".into(),
                        span: Span::new(0, 0),
                        sym: 0
                    }],
                    span: Span::new(0, 0)
                },
                HPat::Wild {
                    span: Span::new(0, 0)
                }
            ],
            span: Span::new(0, 0)
        }
        .sexp(),
        "(or (Circle r) _)"
    );
}

#[test]
fn literal_dien_kieu() {
    for (k, t) in [
        (LitKind::Int, Type::Int),
        (LitKind::Float, Type::Float),
        (LitKind::Size, Type::Size),
        (LitKind::Duration, Type::Duration),
        (LitKind::Percent, Type::Percent),
        (LitKind::Str, Type::Str),
        (LitKind::Bool, Type::Bool),
        (LitKind::Bytes, Type::Bytes),
        (LitKind::Path, Type::Path),
        (LitKind::Char, Type::Char),
    ] {
        assert_eq!(lit(k, "x").sexp(), "x");
        let l = Lit {
            kind: k,
            text: "x".into(),
        };
        assert_eq!(l.ty(), t);
    }
}
