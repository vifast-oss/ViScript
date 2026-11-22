// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-285 (VD-272)** — nhánh **gọi** của `tyck/expr.rs`: `kiem_call`, `kiem_args`,
//! `kiem_method` — tách nguyên văn (R0 refactor, 0 đổi hành vi).
//!
//! Mô-đun con của `tyck::expr` nên dùng chung `diag`/`coerce`/`kiem_expr`/`phu` của cha.
//! `kiem_args`/`kiem_method` giữ `pub(crate)` vì `tyck/tuy_chon.rs` gọi trực tiếp.

use crate::hir::HExpr;
use crate::ty::Type;

use super::super::sig::FnSig;
use super::super::{Checker, ARG_COUNT, MISMATCH};
use super::phu::{self, builtin_is_value};

impl<'a> Checker<'a> {
    pub(super) fn kiem_call(
        &mut self,
        callee: &HExpr,
        args: &[HExpr],
        named: &[(String, HExpr)],
        span: vis_syntax::Span,
    ) -> Type {
        // Hàm dựng sẵn: kiểm **chữ ký** (ST-12b BLOCKER #3) — cả khi `sym = None` lẫn khi `sym`
        // trỏ tới ký hiệu `SymKind::Builtin` (ST-11 khai 8 tên dựng sẵn).
        if let HExpr::Var { name, sym, .. } = callee {
            // ST-26: `std.<module>.<fn>` — kiểm chữ ký theo bảng `vis_types::stdlib`.
            if let Some(t) = self.kiem_std(name, args, named, span) {
                return t;
            }
            let la_builtin = match sym {
                None => builtin_is_value(name),
                Some(id) => self.symbols.kind_of(*id) == Some(crate::lower::SymKind::Builtin),
            };
            // ST-50: **lệnh shell** dựng bởi bộ hạ cũng kiểm chữ ký (`where`/`each`/`select`/
            // `get`/`sort_by` không nằm trong 8 tên giá trị dựng sẵn nhưng là Call có tên).
            let la_lenh = matches!(
                name.as_str(),
                "where" | "each" | "select" | "get" | "sort_by"
            );
            if la_builtin || la_lenh {
                if let Some(t) = phu::kiem_builtin_sig(self, name, args, span) {
                    return t;
                }
                if let Some(t) = self.kiem_builtin(name, args, span) {
                    return t;
                }
            }
        }
        // 0) Phương thức lõi (ST-13, VD-47) + **VD-269** `recv?.m(…)`.
        if let Some(t) = self.kiem_goi_field(callee, args, named, span) {
            return t;
        }
        // Hàm **khai trong tệp**: dùng chữ ký đầy đủ (có tên + mặc định).
        let sig = match callee {
            HExpr::Var { sym: Some(id), .. } => self.sig_of(*id).cloned(),
            _ => None,
        };
        if let Some(sig) = sig {
            self.kiem_args(&sig, args, named, span);
            // **ST-60 (VD-90)**: gọi `async fn` ⇒ `Task[ret]`; `--tên` chưa hỗ trợ cho async.
            if sig.is_async {
                if !named.is_empty() {
                    self.diag(
                        MISMATCH,
                        "gọi `async fn` chỉ nhận đối số **vị trí** (không `--tên`) — nợ R10",
                        span,
                    );
                }
                return Type::TypeRef {
                    name: "Task".into(),
                    args: vec![sig.ret],
                };
            }
            if sig.generics.is_empty() {
                return sig.ret;
            }
            // ST-28: **hợp nhất** kiểu khai báo (còn `T`) với kiểu đối số thật ⇒ suy `T`; mâu thuẫn
            // ⇒ `VS0412`. Kiểu trả sau khi thay `T` ⇒ giữ được kiểm tĩnh ở ngữ cảnh
            // (`let n: int = lay("x")` ⇒ `VS0412`).
            let arg_tys: Vec<Type> = args.iter().map(|a| self.kiem_expr(a)).collect();
            let mut bang: Vec<(String, Type)> = Vec::new();
            for (i, t) in arg_tys.iter().enumerate() {
                if let Some(raw) = sig.raw_params.get(i) {
                    let raw = raw.clone();
                    self.hop_nhat(&raw, t, &sig.generics, &mut bang, &sig.name, args[i].span());
                }
            }
            return crate::tyck::thay_gen(&sig.ret_raw(), &bang);
        }
        let ct = self.kiem_expr(callee);
        let arg_tys: Vec<Type> = args.iter().map(|a| self.kiem_expr(a)).collect();
        let named_tys: Vec<(String, Type, vis_syntax::Span)> = named
            .iter()
            .map(|(n, a)| (n.clone(), self.kiem_expr(a), a.span()))
            .collect();
        match ct {
            Type::Fn(params, ret) => {
                let total = arg_tys.len() + named_tys.len();
                if total != params.len() {
                    self.diag(
                        ARG_COUNT,
                        format!("hàm cần {} đối số, được {total}", params.len()),
                        span,
                    );
                }
                for (i, a) in arg_tys.iter().enumerate() {
                    if let Some(p) = params.get(i) {
                        let p = p.clone();
                        self.coerce_e(&args[i], a, &p, "đối số");
                    }
                }
                for (n, t, sp) in &named_tys {
                    let _ = (n, t, sp);
                }
                (*ret).clone()
            }
            Type::Record(id) => {
                // **VD-260**: bản ghi chỉ dựng được bằng **trường có tên** (`U(a: 1)`).
                phu::kiem_goi_ban_ghi(self, id, args, named, span);
                Type::Record(id)
            }
            Type::Error => Type::Error,
            other => {
                self.diag(
                    MISMATCH,
                    format!("`{}` không phải hàm", self.show(&other)),
                    span,
                );
                Type::Error
            }
        }
    }

    pub(crate) fn kiem_args(
        &mut self,
        sig: &FnSig,
        args: &[HExpr],
        named: &[(String, HExpr)],
        span: vis_syntax::Span,
    ) -> Vec<Type> {
        let total = args.len() + named.len();
        let required = sig.params.iter().filter(|p| !p.2 && !sig.variadic).count();
        let toi_da = if sig.variadic {
            usize::MAX
        } else {
            sig.params.len()
        };
        if total > toi_da || total < required {
            self.diag(
                ARG_COUNT,
                format!(
                    "hàm `{}` cần {} đối số (tối thiểu {required}), được {total}",
                    sig.name,
                    sig.params.len()
                ),
                span,
            );
        }
        let arg_tys: Vec<Type> = args.iter().map(|a| self.kiem_expr(a)).collect();
        for (i, t) in arg_tys.iter().enumerate() {
            if let Some(p) = sig.params.get(i) {
                // ST-27: ô `...rest` — mọi đối số từ đây trở đi là **phần tử**, không phải list.
                let want = match (&p.1, sig.variadic && i + 1 == sig.params.len()) {
                    (Type::List(elem), true) => (**elem).clone(),
                    _ => p.1.clone(),
                };
                self.coerce_e(&args[i], t, &want, "đối số");
            } else if sig.variadic {
                // ST-27: đối số **thừa** rơi vào `...rest` — kiểm theo **kiểu phần tử** của nó.
                if let Some(p) = sig.params.last() {
                    if let Type::List(elem) = &p.1 {
                        let want = (**elem).clone();
                        self.coerce_e(&args[i], t, &want, "đối số");
                    }
                }
            }
        }
        let mut da_gap: Vec<&str> = Vec::new();
        for (n, a) in named {
            let t = self.kiem_expr(a);
            match sig.params.iter().find(|p| &p.0 == n) {
                // **R5a (VD-178)**: `f(1, b: 5)` — đối số **có tên** khớp tham số theo **tên**.
                Some(p) => {
                    if da_gap.contains(&n.as_str()) {
                        self.diag(ARG_COUNT, format!("đối số tên `{n}` bị lặp"), a.span());
                    }
                    da_gap.push(n);
                    let want = p.1.clone();
                    self.coerce_e(a, &t, &want, "đối số có tên");
                }
                None => self.diag(
                    ARG_COUNT,
                    format!("hàm `{}` không có tham số tên `{n}`", sig.name),
                    a.span(),
                ),
            }
        }
        arg_tys
    }

    /// Phương thức lõi trên `list`/`str`/`map` (ST-13): trả `None` nếu `recv` không phải kiểu có
    /// phương thức (để nhánh bản ghi/enum xử lý). Phương thức lạ ⇒ `VS0403` (tyck chặn trước khi chạy).
    pub(crate) fn kiem_method(
        &mut self,
        recv: &Type,
        name: &str,
        args: &[HExpr],
        span: vis_syntax::Span,
    ) -> Option<Type> {
        let arg_tys: Vec<Type> = args.iter().map(|a| self.kiem_expr(a)).collect();
        let elem = |t: &Type| match t {
            Type::List(e) => (**e).clone(),
            _ => Type::Error,
        };
        let out = match recv {
            // **ST-61 (VD-96)**: kênh — phần phương thức ở `kenh.rs` (`send`/`recv`/`close`), tách
            // tệp để giữ R1.
            Type::TypeRef {
                name: k,
                args: c_args,
            } if k == "Chan" && c_args.len() == 1 => {
                return self.kiem_chan_method(c_args, name, &arg_tys, args, span);
            }
            Type::List(_) => {
                let e = elem(recv);
                match name {
                    "len" => {
                        self.arity(args, 0, name, span);
                        Type::Int
                    }
                    "get" => {
                        self.arity(args, 1, name, span);
                        self.arg_ty(&arg_tys, 0, &Type::Int, name, args, span);
                        Type::Opt(Box::new(e))
                    }
                    "first" | "last" => {
                        self.arity(args, 0, name, span);
                        Type::Opt(Box::new(e))
                    }
                    "push" => {
                        self.arity(args, 1, name, span);
                        self.arg_ty(&arg_tys, 0, &e, name, args, span);
                        Type::List(Box::new(e))
                    }
                    "map" => {
                        self.arity(args, 1, name, span);
                        let f = arg_tys.first().cloned().unwrap_or(Type::Error);
                        match f {
                            Type::Fn(_, ret) => Type::List(ret),
                            Type::Error => Type::List(Box::new(Type::Error)),
                            other => {
                                self.diag(
                                    MISMATCH,
                                    format!("`map` cần hàm, được `{}`", self.show(&other)),
                                    span,
                                );
                                Type::List(Box::new(Type::Error))
                            }
                        }
                    }
                    "filter" => {
                        self.arity(args, 1, name, span);
                        // ST-13b (mục 6): `filter` phải nhận **hàm 1 tham số** trả `bool`.
                        match arg_tys.first() {
                            Some(Type::Fn(params, ret)) => {
                                if params.len() != 1 {
                                    self.diag(MISMATCH, "`filter` cần hàm **1 tham số**", span);
                                }
                                if !matches!(**ret, Type::Bool | Type::Error) {
                                    self.diag(MISMATCH, "`filter` cần hàm trả `bool`", span);
                                }
                            }
                            Some(Type::Error) | None => {}
                            Some(other) => {
                                let o = other.clone();
                                self.diag(
                                    MISMATCH,
                                    format!("`filter` cần **hàm**, được `{}`", self.show(&o)),
                                    span,
                                );
                            }
                        }
                        Type::List(Box::new(e))
                    }
                    // **R20b (VD-178)**: `enumerate`/`any`/`all`/`find`.
                    "enumerate" => {
                        self.arity(args, 0, name, span);
                        Type::List(Box::new(Type::TypeRef {
                            name: "tuple".into(),
                            args: vec![Type::Int, e],
                        }))
                    }
                    "any" | "all" => {
                        self.arity(args, 1, name, span);
                        if let Some(Type::Fn(params, ret)) = arg_tys.first() {
                            if params.len() != 1 {
                                self.diag(
                                    MISMATCH,
                                    format!("`{name}` cần hàm **1 tham số**"),
                                    span,
                                );
                            }
                            if !matches!(**ret, Type::Bool | Type::Error) {
                                self.diag(MISMATCH, format!("`{name}` cần hàm trả `bool`"), span);
                            }
                        }
                        Type::Bool
                    }
                    "find" => {
                        self.arity(args, 1, name, span);
                        if let Some(Type::Fn(params, ret)) = arg_tys.first() {
                            if params.len() != 1 {
                                self.diag(MISMATCH, "`find` cần hàm **1 tham số**", span);
                            }
                            if !matches!(**ret, Type::Bool | Type::Error) {
                                self.diag(MISMATCH, "`find` cần hàm trả `bool`", span);
                            }
                        }
                        Type::Opt(Box::new(e))
                    }
                    "each" => {
                        self.arity(args, 1, name, span);
                        // ST-13b (mục 6): `each` phải nhận **hàm 1 tham số**.
                        match arg_tys.first() {
                            Some(Type::Fn(params, _)) => {
                                if params.len() != 1 {
                                    let n = params.len();
                                    self.diag(
                                        MISMATCH,
                                        format!("`each` cần hàm **1 tham số**, được {n}"),
                                        span,
                                    );
                                }
                            }
                            Some(Type::Error) | None => {}
                            Some(other) => {
                                let o = other.clone();
                                self.diag(
                                    MISMATCH,
                                    format!("`each` cần **hàm**, được `{}`", self.show(&o)),
                                    span,
                                );
                            }
                        }
                        Type::Unit
                    }
                    // **R17 (VD-178)**: `is_empty()` cho list/map/str — gợi ý của điều kiện.
                    "is_empty" => phu::is_empty(self, args, name, span),
                    "sum" => {
                        self.arity(args, 0, name, span);
                        // ST-13b (mục 6): `sum` chỉ trên list **số**.
                        if !matches!(e, Type::Int | Type::Float | Type::Error) {
                            let t = e.clone();
                            self.diag(
                                MISMATCH,
                                format!(
                                    "`sum` chỉ dùng cho list số, phần tử là `{}`",
                                    self.show(&t)
                                ),
                                span,
                            );
                            return Some(Type::Error);
                        }
                        e
                    }
                    "join" => {
                        self.arity(args, 1, name, span);
                        self.arg_ty(&arg_tys, 0, &Type::Str, name, args, span);
                        Type::Str
                    }
                    // **R5c (VD-178)**: thử `std.list.<name>` trước khi báo lỗi.
                    _ => return None,
                }
            }
            Type::Str => match name {
                "len" => {
                    self.arity(args, 0, name, span);
                    Type::Int
                }
                "upper" | "lower" => {
                    self.arity(args, 0, name, span);
                    Type::Str
                }
                "is_empty" => phu::is_empty(self, args, name, span),
                "chars" => {
                    self.arity(args, 0, name, span);
                    Type::List(Box::new(Type::Str))
                }
                "contains" => {
                    self.arity(args, 1, name, span);
                    self.arg_ty(&arg_tys, 0, &Type::Str, name, args, span);
                    Type::Bool
                }
                "split" => {
                    self.arity(args, 1, name, span);
                    self.arg_ty(&arg_tys, 0, &Type::Str, name, args, span);
                    Type::List(Box::new(Type::Str))
                }
                // **R5c (VD-178)**: thử `std.str.<name>` trước khi báo lỗi.
                _ => return None,
            },
            Type::Map(k, v) => {
                let kk = (**k).clone();
                let vv = (**v).clone();
                match name {
                    "len" => {
                        self.arity(args, 0, name, span);
                        Type::Int
                    }
                    "get" => {
                        self.arity(args, 1, name, span);
                        self.arg_ty(&arg_tys, 0, &kk, name, args, span);
                        Type::Opt(Box::new(vv))
                    }
                    "has" => {
                        self.arity(args, 1, name, span);
                        self.arg_ty(&arg_tys, 0, &kk, name, args, span);
                        Type::Bool
                    }
                    "is_empty" => phu::is_empty(self, args, name, span),
                    "keys" => {
                        self.arity(args, 0, name, span);
                        Type::List(Box::new(kk))
                    }
                    "values" => {
                        self.arity(args, 0, name, span);
                        Type::List(Box::new(vv))
                    }
                    // **R5c (VD-178)**: thử `std.map.<name>` trước khi báo lỗi.
                    _ => return None,
                }
            }
            _ => return None,
        };
        Some(out)
    }
}
