// SPDX-License-Identifier: MIT OR Apache-2.0
//! Kiểu của **biểu thức** (ST-12) — số học, so sánh, gọi hàm, trường, chỉ số, bản ghi/enum.
//!
//! **ST-318c** — tách tệp theo nhóm biểu thức (0 đổi hành vi/API): `toan_tu` (toán tử nhị phân),
//! `builtin` (`std.*` + tên dựng sẵn); `goi`/`kenh`/`phu` đã tách từ trước.

use vis_hir::hir::{HCond, HExpr, HPat, HStmt, UnOp};
use vis_hir::ty::{EnumId, Type};

use super::{Checker, ARG_COUNT, MISMATCH, NO_FIELD, NO_TYPE};

mod builtin;
mod goi;
/// **ST-61**: phần kiểu của **kênh** — tách ra để `tyck/expr.rs` giữ **R1 ≤ 1.500 dòng** (tiền lệ
/// ST-60 tách `tyck/sig.rs`). Mô-đun con của `expr` nên dùng chung `diag`/`coerce`/`arity`.
mod kenh;
/// **ST-61**: phụ trợ thuần (tách cùng lý do R1).
pub(super) mod phu;
mod toan_tu;
use phu::{builtin_value_ty, static_key};

impl<'a> Checker<'a> {
    /// Kiểu một biểu thức (ghi vào `types_of_expr`).
    pub(crate) fn kiem_expr(&mut self, e: &HExpr) -> Type {
        let t = self.kiem_expr_trong(e);
        self.note(t.clone())
    }

    fn kiem_expr_trong(&mut self, e: &HExpr) -> Type {
        match e {
            HExpr::Lit { lit, ty, span } => {
                self.kiem_int_lit(lit, *span);
                ty.clone()
            }
            HExpr::Var { name, sym, span } => match sym {
                Some(id) => {
                    // **ST-60 (VD-95 (4))**: `async fn` **không** dùng làm giá trị (chưa có
                    // `Task` như hàm hạng nhất) — gọi `f(…)` vẫn được vì `kiem_call` bắt trước.
                    if self.sig_of(*id).map(|s| s.is_async).unwrap_or(false) {
                        self.diag(
                            MISMATCH,
                            format!("`{name}` là `async fn` — không dùng làm **giá trị** (nợ R10)"),
                            *span,
                        );
                        Type::Error
                    } else if self.symbols.kind_of(*id) == Some(crate::lower::SymKind::Builtin) {
                        // **VD-258**: `None` trần là **giá trị** `Option[?]` (không phải biến
                        // chưa khai báo). Trước đây `ty_of` của ký hiệu dựng sẵn là `None` ⇒
                        // `{error}` nên `if None`, `1 + None`, `xs.push(None)`, `E.A(None)` lọt
                        // checker. Các builtin khác vẫn `{error}` như cũ.
                        builtin_value_ty(name)
                    } else {
                        self.ty_of(*id)
                    }
                }
                None => {
                    // **VD-260**: tên giá trị chưa phân giải không còn im lặng thành `{error}`.
                    // Tên dựng sẵn/module/tiền tố kiểu (`Json`, `std.str.split`, …) được miễn.
                    if !phu::ten_duoc_mien(self, name) {
                        self.diag(
                            NO_TYPE,
                            format!("tên không tồn tại: `{name}` chưa được khai báo (VD-260)"),
                            *span,
                        );
                    }
                    builtin_value_ty(name)
                }
            },
            HExpr::Bin { op, lhs, rhs, span } => {
                let l = self.kiem_expr(lhs);
                let r = self.kiem_expr(rhs);
                let (l_lit, r_lit) = (super::vd238::is_int_lit(lhs), super::vd238::is_int_lit(rhs));
                self.kiem_bin(
                    *op,
                    (&l, &r),
                    (l_lit, r_lit),
                    (lhs.span(), rhs.span()),
                    *span,
                )
            }
            HExpr::Un { op, expr, span } => {
                let t = self.kiem_expr(expr);
                match op {
                    UnOp::Neg => {
                        let ok = matches!(
                            t,
                            Type::Int
                                | Type::Float
                                | Type::Size
                                | Type::Duration
                                | Type::Percent
                                | Type::Error
                        );
                        if ok {
                            t
                        } else {
                            self.diag(
                                MISMATCH,
                                format!("`-` một ngôi cần kiểu số, được `{}`", self.show(&t)),
                                *span,
                            );
                            Type::Error
                        }
                    }
                    UnOp::Not => {
                        if matches!(t, Type::Bool | Type::Error) {
                            Type::Bool
                        } else {
                            self.diag(
                                MISMATCH,
                                format!("`not` cần `bool`, được `{}`", self.show(&t)),
                                *span,
                            );
                            Type::Error
                        }
                    }
                }
            }
            HExpr::Call {
                callee,
                args,
                named,
                span,
            } => self.kiem_call(callee, args, named, *span),
            HExpr::Field {
                base,
                name,
                optional,
                span,
            } => self.kiem_field(base, name, *optional, *span),
            HExpr::Index { base, index, span } => self.kiem_index(base, index, *span),
            HExpr::List {
                items,
                elem: _,
                span,
            } => {
                let elem = self.unify_all(items, "phần tử `list`", *span);
                Type::List(Box::new(elem))
            }
            HExpr::Set {
                items,
                elem: _,
                span,
            } => {
                let elem = self.unify_all(items, "phần tử `set`", *span);
                Type::List(Box::new(elem))
            }
            HExpr::Map {
                entries,
                key: _,
                value: _,
                span,
            } => {
                let mut kt = Type::Error;
                let mut vt = Type::Error;
                // **ST-239 (VD-242 Q4a + §3.4, ca 170)**: literal có khoá trùng **xác định
                // được tĩnh** ⇒ lỗi checker (không gọi mã người dùng để tìm khoá). Khoá do
                // biểu thức/biến tính ⇒ hợp lệ, runtime giữ last-wins mỗi khoá một entry.
                let mut khoa_da_thay: std::collections::HashSet<String> =
                    std::collections::HashSet::new();
                for (i, (k, v)) in entries.iter().enumerate() {
                    let kt2 = self.kiem_expr(k);
                    let vt2 = self.kiem_expr(v);
                    if let Some(khoa) = static_key(k) {
                        if !khoa_da_thay.insert(khoa) {
                            self.diag(
                                MISMATCH,
                                "khoá map literal trùng xác định được tĩnh (VD-242 §3.4)",
                                k.span(),
                            );
                        }
                    }
                    if i == 0 {
                        kt = kt2;
                        vt = vt2;
                    } else {
                        // **VD-258**: như literal `list` — không trộn `Option` với plain.
                        phu::hop_ve_literal(self, &mut kt, k, &kt2, "khoá `map` literal");
                        phu::hop_ve_literal(self, &mut vt, v, &vt2, "giá trị `map` literal");
                    }
                }
                let _ = span;
                Type::Map(Box::new(kt), Box::new(vt))
            }
            HExpr::Record { fields, ty, .. } => {
                // **VD-258**: `U(age: "hello")` / `U(age: None)` / `U(age: 1, bogus: 2)` từng
                // lọt checker vì chỉ kiểm biểu thức mà không so với trường của `Type::Record(id)`.
                // Helper kiểm **từng trường theo tên** + default (xem `expr/phu.rs`).
                phu::kiem_truong_ban_ghi(self, fields, ty);
                ty.clone()
            }
            HExpr::EnumLit {
                enum_id,
                variant,
                args,
                span,
                ..
            } => self.kiem_enum_lit(*enum_id, variant, args, *span),
            HExpr::Opt { value, ty, .. } => match value {
                None => Type::Opt(Box::new(Type::Error)),
                Some(v) => {
                    let t = self.kiem_expr(v);
                    let _ = ty;
                    Type::Opt(Box::new(t))
                }
            },
            HExpr::Try { expr, span } => super::expr::phu::kiem_try(self, expr, *span),
            HExpr::With { base, fields, span } => {
                phu::kiem_with_trung(self, fields, *span);
                let bt = self.kiem_expr(base);
                match bt {
                    Type::Record(id) => {
                        for (n, v) in fields {
                            let vt = self.kiem_expr(v);
                            match self.types.field(id, n) {
                                Some(f) => {
                                    let want = f.ty.clone();
                                    self.coerce_e(v, &vt, &want, "`with` cập nhật trường");
                                }
                                None => self.diag(
                                    NO_FIELD,
                                    format!("bản ghi không có trường `{n}` (VD-43 #6)"),
                                    *span,
                                ),
                            }
                        }
                        Type::Record(id)
                    }
                    Type::Error => Type::Error,
                    other => {
                        self.diag(
                            MISMATCH,
                            format!("`with` cần bản ghi, được `{}`", self.show(&other)),
                            *span,
                        );
                        Type::Error
                    }
                }
            }
            HExpr::If {
                cond,
                then,
                elifs,
                else_,
                span,
                ..
            } => {
                // **VD-260**: binding `if let` chỉ sống trong biểu thức `if`.
                let snap = self.luu_env();
                self.kiem_cond(cond, then);
                let mut acc = self.than_value(then);
                for (c, body) in elifs {
                    self.kiem_cond(c, body);
                    let t = self.than_value(body);
                    self.unify_branch(&mut acc, &t, *span);
                }
                if !else_.is_empty() {
                    let t = self.than_value(else_);
                    self.unify_branch(&mut acc, &t, *span);
                } else {
                    acc = Type::Unit;
                }
                self.tra_env(snap);
                acc
            }
            HExpr::Match {
                subject,
                arms,
                span,
                ..
            } => {
                let subj = self.kiem_expr(subject);
                let mut acc: Option<Type> = None;
                for arm in arms {
                    // **VD-260**: binding của nhánh chỉ sống trong nhánh.
                    let snap = self.luu_env();
                    // **ST-239 / VD-259 (VD-242 §3.1)**: chặn or-pattern bind không đầy đủ
                    // (kể cả lồng) và or-pattern bind khác kiểu trước khi kiểm thân nhánh —
                    // interp in `2`, VM in `()` đều là chương trình sai.
                    let uses = super::pat::arm_uses(arm.guard.as_ref(), &arm.body);
                    self.kiem_or_bind(&arm.pat, &subj, &uses);
                    self.bind_pat(&arm.pat, &subj);
                    if let Some(g) = &arm.guard {
                        let gt = self.kiem_expr(g);
                        self.coerce(&gt, &Type::Bool, g.span(), "guard `if`");
                    }
                    let t = self.than_value(&arm.body);
                    self.tra_env(snap);
                    match &mut acc {
                        None => acc = Some(t),
                        Some(a) => self.unify_branch(a, &t, arm.span),
                    }
                }
                self.xet_het(&subj, arms, *span);
                acc.unwrap_or(Type::Unit)
            }
            HExpr::Lambda {
                params,
                ret,
                body,
                span,
            } => {
                // **VD-260**: tham số lambda chỉ sống trong thân lambda.
                let snap = self.luu_env();
                let mut pts: Vec<Type> = Vec::new();
                for p in params {
                    let t = self.resolve_ann(&p.ty, p.span);
                    self.set_ty(p.sym, t.clone());
                    pts.push(t);
                }
                let saved = self.cur_ret.clone();
                self.cur_ret = None;
                // **VD-259**: `break`/`continue` không xuyên biên lambda; use-set riêng.
                let saved_loop = std::mem::replace(&mut self.loop_depth, 0);
                let saved_value = std::mem::replace(&mut self.in_value, false);
                let saved_gan = std::mem::replace(&mut self.bo_qua_gan_pt, false);
                let saved_uses = match body {
                    vis_hir::hir::HBody::Stmts(st) => {
                        std::mem::replace(&mut self.fn_uses, super::pat::than_uses(st))
                    }
                    vis_hir::hir::HBody::Expr(_) => std::mem::take(&mut self.fn_uses),
                };
                let body_ty = match body {
                    vis_hir::hir::HBody::Expr(e) => self.kiem_expr(e),
                    vis_hir::hir::HBody::Stmts(st) => self.than_value(st),
                };
                self.fn_uses = saved_uses;
                self.in_value = saved_value;
                self.loop_depth = saved_loop;
                self.bo_qua_gan_pt = saved_gan;
                self.cur_ret = saved;
                let declared = self.resolve_ann(ret, *span);
                let out = if declared.is_error() {
                    body_ty
                } else {
                    self.coerce(&body_ty, &declared, *span, "thân lambda");
                    declared
                };
                self.tra_env(snap);
                Type::Fn(pts.into_boxed_slice(), Box::new(out))
            }
            HExpr::Tuple { items, .. } => {
                let tys: Vec<Type> = items.iter().map(|i| self.kiem_expr(i)).collect();
                // **VD-241 §6/B-A (A0, 2026-11-19)** — `()` là **một** kiểu (`Unit` ngay trong checker,
                // khớp lower "Unit"); tuple chỉ tồn tại khi có phần tử.
                if items.is_empty() {
                    Type::Unit
                } else {
                    Type::TypeRef {
                        name: "tuple".into(),
                        args: tys,
                    }
                }
            }
            // **ST-56 (VD-92)**: `ref(x)` **giữ nguyên kiểu** của `x` — hạ **identity** ở cả hai
            // đường (interp/codegen). **Chưa có mượn thật**: `&T`, kiểm vòng đời là nợ **VS2+**.
            HExpr::Ref { expr, .. } => self.kiem_expr(expr),
            // **ST-60 (VD-90/VD-95)**: `await` bỏ **một** lớp `Task[T]` ⇒ `T`; không phải `Task`
            // ⇒ `VS0412` (một quy tắc cho cả hai đường).
            // **ST-61 (VD-96)**: `chan[T](n)` — phần kiểu ở `kenh.rs` (`n` phải `int`, kết quả
            // `Chan[T]`); tách tệp để giữ R1.
            HExpr::ChanNew { elem, cap, span } => self.kiem_chan_new(elem, cap, *span),
            HExpr::Await { expr, span } => {
                let t = self.kiem_expr(expr);
                match t {
                    Type::TypeRef { name, args } if name == "Task" && args.len() == 1 => {
                        args[0].clone()
                    }
                    Type::Error => Type::Error,
                    other => {
                        self.diag(
                            MISMATCH,
                            format!("`await` cần `Task[T]`, được `{}`", self.show(&other)),
                            *span,
                        );
                        Type::Error
                    }
                }
            }
            // **ST-60**: `spawn f(args)` — chỉ **tên** hàm `async` cấp tệp, đối số **vị trí**.
            HExpr::Spawn {
                callee,
                args,
                named,
                span,
            } => {
                let sig = match &**callee {
                    HExpr::Var { sym: Some(id), .. } => self.sig_of(*id).cloned(),
                    _ => None,
                };
                match sig {
                    Some(sig) if sig.is_async => {
                        if !named.is_empty() {
                            self.diag(
                                MISMATCH,
                                "`spawn` chỉ nhận đối số **vị trí** (không `--tên`) — VD-95 (4)",
                                *span,
                            );
                        }
                        // Vẫn kiểm **đủ** đối số (kể cả `--tên` đã báo lỗi ở trên) để không sinh
                        // thêm `VS0401` nhiễu.
                        self.kiem_args(&sig, args, named, *span);
                        Type::TypeRef {
                            name: "Task".into(),
                            args: vec![sig.ret],
                        }
                    }
                    Some(_) => {
                        for a in args {
                            self.kiem_expr(a);
                        }
                        // **ST-302b**: phân biệt phương thức `impl` (tên hạ `Ten.m`) với hàm
                        // cấp tệp để thông điệp đúng chỗ.
                        let la_phuong_thuc = matches!(
                            &**callee,
                            HExpr::Var { name, .. } if name.contains('.')
                        );
                        let msg = if la_phuong_thuc {
                            "`spawn` chỉ nhận hàm/phương thức `async` (phương thức này không `async`)"
                        } else {
                            "`spawn` chỉ nhận hàm `async` (hàm này không `async`)"
                        };
                        self.diag(MISMATCH, msg, *span);
                        Type::Error
                    }
                    None => {
                        for a in args {
                            self.kiem_expr(a);
                        }
                        self.diag(
                            MISMATCH,
                            "`spawn` cần **tên** hàm `async` cấp tệp (không nhận biểu thức)",
                            *span,
                        );
                        Type::Error
                    }
                }
            }
            // ST-14: chuỗi nội suy — kiểm **từng** phần biểu thức, kiểu của cả chuỗi là `str`.
            HExpr::Interp { parts, .. } => {
                super::expr::phu::kiem_interp(self, parts);
                Type::Str
            }
            HExpr::CmdBlock { .. } | HExpr::Error { .. } => Type::Error,
        }
    }

    pub(crate) fn show(&self, t: &Type) -> String {
        self.types.show(t)
    }

    /// Duyệt `node` của `if let`: binding thuộc **nhánh** `then` (đã đủ vì binding nằm trong HIR).
    /// **VD-259**: `then_body` là use-set của or-pattern trong `if let`/`while let`.
    pub(crate) fn kiem_cond(&mut self, cond: &HCond, then_body: &[HStmt]) {
        match cond {
            HCond::Expr(e) => {
                let t = self.kiem_expr(e);
                self.coerce_cond(e, &t, "điều kiện `if`");
            }
            HCond::Let { pat, value, .. } => {
                let vt = self.kiem_expr(value);
                // **R3a (VD-178)**: `if let` nhận **pattern** kiểm theo **kiểu đầy đủ** (`T?`),
                // không bóc sẵn — `if let Some(v) = o:` mới đúng; `if let v = o:` (luôn khớp) ⇒ lỗi.
                if matches!(pat, vis_hir::hir::HPat::Bind { .. })
                    && matches!(vt, Type::Opt(_) | Type::Result(..))
                {
                    self.diag(
                        MISMATCH,
                        "`if let` cần **pattern** — dùng `if let Some(v) = o:` (hoặc `Ok(v)`/`Err(e)`)",
                        pat.span(),
                    );
                }
                let uses = super::pat::than_uses(then_body);
                self.kiem_or_bind(pat, &vt, &uses);
                self.bind_pat(pat, &vt);
            }
        }
    }

    /// Kiểu chung của một dãy biểu thức (`list`/`set`) — VD-258 tách sang `expr/phu.rs`.
    fn unify_all(&mut self, items: &[HExpr], what: &str, span: vis_syntax::Span) -> Type {
        phu::unify_all(self, items, what, span)
    }

    pub(crate) fn unify_branch(&mut self, acc: &mut Type, t: &Type, span: vis_syntax::Span) {
        phu::unify_branch(self, acc, t, span)
    }
}

impl<'a> Checker<'a> {
    fn arity(&mut self, args: &[HExpr], want: usize, name: &str, span: vis_syntax::Span) {
        if args.len() != want {
            self.diag(
                ARG_COUNT,
                format!("`{name}` cần {want} đối số, được {}", args.len()),
                span,
            );
        }
    }

    fn arg_ty(
        &mut self,
        got: &[Type],
        i: usize,
        want: &Type,
        name: &str,
        args: &[HExpr],
        span: vis_syntax::Span,
    ) {
        if let Some(g) = got.get(i) {
            let g = g.clone();
            let w = want.clone();
            let at = args.get(i).map(|a| a.span()).unwrap_or(span);
            self.coerce(&g, &w, at, &format!("đối số `{name}`"));
        }
    }

    /// `?` có hợp lệ không: hàm đang kiểm phải trả `U?` (với `T?`) hoặc `Result[…, …]` (với
    /// `Result`); ngoài hàm (hoặc trong lambda) `cur_ret = None` ⇒ **cho phép** (không có ngữ cảnh).
    fn try_hop_le(&self, opt: bool) -> bool {
        match &self.cur_ret {
            None => true,
            Some(Type::Error) => true,
            Some(Type::Opt(_)) => opt,
            Some(Type::Result(..)) => !opt,
            Some(_) => false,
        }
    }

    fn ham_tra(&self) -> Type {
        self.cur_ret.clone().unwrap_or(Type::Unit)
    }
}

impl<'a> Checker<'a> {
    fn kiem_enum_lit(
        &mut self,
        enum_id: Option<EnumId>,
        variant: &str,
        args: &[HExpr],
        span: vis_syntax::Span,
    ) -> Type {
        let Some(id) = enum_id else {
            for a in args {
                let _ = self.kiem_expr(a);
            }
            return Type::Error;
        };
        let Some(def) = self.types.enum_def(id) else {
            return Type::Error;
        };
        let fields = def
            .variants
            .iter()
            .find(|v| v.name == variant)
            .map(|v| v.fields.clone());
        let Some(fields) = fields else {
            self.diag(
                NO_FIELD,
                format!("enum `{}` không có biến thể `{variant}`", def.name),
                span,
            );
            return Type::Error;
        };
        if args.len() != fields.len() {
            self.diag(
                ARG_COUNT,
                format!(
                    "biến thể `{variant}` cần {} đối số, được {}",
                    fields.len(),
                    args.len()
                ),
                span,
            );
        }
        for (i, a) in args.iter().enumerate() {
            let t = self.kiem_expr(a);
            if let Some(f) = fields.get(i) {
                let want = f.ty.clone();
                self.coerce(&t, &want, a.span(), "đối số biến thể");
            }
        }
        Type::Enum(id)
    }
}

/// Giữ chỗ: `HPat`/`HStmt` dùng ở module khác (tránh import thừa).
#[allow(dead_code)]
fn _giu(_: &HPat, _: &HStmt) {}
