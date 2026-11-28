// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-285 (VD-272)** — nhánh **gọi hàm/closure** của interp: `eval_call`, `eval_args`,
//! `call_value`, `call_closure`/`call_closure_trong`, `call_hfn`/`call_hfn_trong` — tách
//! nguyên văn khỏi `interp/goi.rs` (R0 refactor, 0 đổi hành vi).
//!
//! Mô-đun con của `interp::goi` nên vẫn thấy `Interp`/`Closure`/`Args`/`Abort` và các helper
//! private của cha; API `pub(crate)` giữ nguyên.

use super::*;

impl<'a> Interp<'a> {
    pub(crate) fn eval_call(
        &mut self,
        callee: &HExpr,
        args: &[HExpr],
        named: &[(String, HExpr)],
        span: Span,
    ) -> R<Value> {
        // **VD-269**: `recv?.m(…)` — base `Option`, đoản mạch `None` (không đánh giá đối số).
        if let HExpr::Field {
            base,
            name,
            optional: true,
            ..
        } = callee
        {
            return self.goi_tuy_chon(base, name, args, named, span);
        }
        // 1) Phương thức: `xs.push(1)`, `s.upper()` — **mọi** `recv` đi cùng một hiện thực
        // `builtin::method`, đúng như opcode `CallMethod` của VM. Ca `recv` không phải
        // list/str/map/chan (vd `str.join` thiếu `use std.str`) vì thế cũng trả **cùng mã +
        // cùng thông điệp** `không có phương thức` ở hai engine (trước đây interp rơi vào
        // đường truy cập trường ⇒ `không có trường`, lệch thông điệp với VM).
        if let HExpr::Field {
            base,
            name,
            optional: false,
            ..
        } = callee
        {
            // **ST-316a**: `xs.push(v)` trên `var` — lấy hẳn giá trị ra khỏi ô để `push` nối
            // **tại chỗ** (không `Rc::make_mut` chép cả list); kết quả được ghi lại bên dưới.
            let recv = match (&**base, name.as_str()) {
                (HExpr::Var { sym: Some(id), .. }, "push") if self.mutable.contains(id) => {
                    self.lay_o(*id, span)?
                }
                _ => self.eval(base)?,
            };
            // **VD-258**: `xs.push(v)` với `xs: list[T?]` — nâng `v` plain ở biên phương
            // thức (mirror codegen), tránh list chứa giá trị plain trong ô `T?`.
            let push_depth = if name == "push" {
                match self.expr_ty(base, &recv) {
                    Some(vis_hir::Type::List(el)) if el.is_opt() => opt_depth_ty(&el),
                    _ => 0,
                }
            } else {
                0
            };
            let mut vs = Vec::with_capacity(args.len());
            for a in args {
                let mut vv = self.eval(a)?;
                if push_depth > 0 {
                    vv = crate::lift::lift_opt(vv, push_depth);
                }
                vs.push(vv);
            }
            let kq = crate::builtin::method(self, recv, name, vs, span)?;
            // ST-15b (mục 1, §7.1 ngữ nghĩa giá trị + CoW): `push` trên **`var`** phải **ghi lại**
            // biến (bản thân list đổi), không chỉ trả list mới. `push` trên `let`/biểu thức tạm
            // vẫn "trả list mới" (không có lvalue để ghi) — ghi rõ trong JOURNAL.
            if name == "push" {
                if let HExpr::Var { sym: Some(id), .. } = &**base {
                    if self.mutable.contains(id) {
                        self.env.borrow_mut().insert(*id, kq.clone());
                    }
                }
            }
            return Ok(kq);
        }
        // 2) Gọi thẳng hàm khai trong tệp (kể cả đệ quy) — theo `sym`.
        if let HExpr::Var { sym: Some(id), .. } = callee {
            if let Some(f) = self.fns.get(id).copied() {
                let (pos, nm) = self.eval_args(args, named)?;
                // **ST-60 (VD-90)**: gọi `async fn` **không** chạy thân — trả `Task[T]` (cùng một
                // đường với codegen: `$tac_vu` + handle, ở đây handle = `sym`).
                if f.is_async {
                    let t = self.sched.get_mut().tao(f.sym, pos, f.name.clone(), span);
                    return Ok(Value::Task(t));
                }
                return self.call_hfn(f, pos, nm, span);
            }
        }
        // 3) Hàm dựng sẵn.
        if let HExpr::Var {
            name, sym: None, ..
        } = callee
        {
            if builtin_name(name).is_some() || name.starts_with("$json:") {
                let (pos, _nm) = self.eval_args(args, named)?;
                return crate::builtin::call(self, name, pos, span);
            }
        }
        // 4) Còn lại: giá trị hàm (closure/lambda/builtin value).
        let f = self.eval(callee)?;
        let (pos, nm) = self.eval_args(args, named)?;
        self.call_value(f, pos, nm, span)
    }

    pub(crate) fn eval_args(&mut self, args: &[HExpr], named: &[(String, HExpr)]) -> R<Args> {
        let mut pos = Vec::with_capacity(args.len());
        for a in args {
            pos.push(self.eval(a)?);
        }
        let mut nm = Vec::with_capacity(named.len());
        for (n, a) in named {
            nm.push((n.clone(), self.eval(a)?));
        }
        Ok((pos, nm))
    }

    pub(crate) fn call_value(
        &mut self,
        f: Value,
        pos: Vec<Value>,
        named: Vec<(String, Value)>,
        span: Span,
    ) -> R<Value> {
        match f {
            Value::Fn(c) => self.call_closure(&c, pos, span),
            Value::Builtin(name) => crate::builtin::call(self, name, pos, span),
            Value::Record(r) if r.fields.is_empty() => {
                // `User(name: "An", age: 20)` — khởi tạo bản ghi theo tên kiểu.
                let _ = pos;
                Ok(Value::record(r.name.clone(), named))
            }
            other => {
                let _ = named;
                Err(Abort::Trap(loi_khong_phai_ham(&other, span)))
            }
        }
    }

    pub(crate) fn call_closure(&mut self, c: &Closure, pos: Vec<Value>, span: Span) -> R<Value> {
        self.depth += 1;
        if self.depth > MAX_CALL_DEPTH {
            self.depth -= 1;
            return Err(Abort::Trap(Trap::new(
                RT,
                format!(
                    "gọi hàm quá sâu (>{} khung) — đệ quy không có điểm dừng?",
                    MAX_CALL_DEPTH
                ),
                span,
            )));
        }
        self.fn_depth += 1;
        let kq = self.call_closure_trong(c, pos, span);
        self.fn_depth -= 1;
        self.depth -= 1;
        kq
    }

    fn call_closure_trong(&mut self, c: &Closure, pos: Vec<Value>, span: Span) -> R<Value> {
        if pos.len() != c.params.len() {
            return Err(Abort::Trap(loi_arity(
                &c.name,
                c.params.len(),
                pos.len(),
                span,
            )));
        }
        let saved_env = self.env.clone();
        // **ST-319e (#210)**: closure **ngoài cùng** cho biết env module thật — non-frame là env
        // cấp tệp, frame là `globals` (frame gán `it.env`). Closure lồng nhau **không** ghi đè:
        // `saved_env` lúc đó là env của closure ngoài, không phải module.
        if self.closure_dong == 0 {
            self.env_module = saved_env.clone();
        }
        self.closure_dong += 1;
        self.env = c.env.clone();
        let mut old = Vec::with_capacity(c.params.len());
        for (i, p) in c.params.iter().enumerate() {
            old.push((*p, self.env.borrow().get(p).cloned()));
            let depth = c.param_depths.get(i).copied().unwrap_or(0);
            self.env
                .borrow_mut()
                .insert(*p, opt_lift_maybe(pos[i].clone(), depth));
        }
        let r = match &c.body {
            HBody::Expr(e) => self.eval(e),
            HBody::Stmts(st) => self.exec_body(st),
        };
        for (p, v) in old {
            match v {
                Some(v) => {
                    self.env.borrow_mut().insert(p, v);
                }
                None => {
                    self.env.borrow_mut().remove(&p);
                }
            }
        }
        self.env = saved_env;
        self.closure_dong -= 1;
        match r {
            Ok(v) => {
                if c.ret_depth > 0 {
                    Ok(opt_lift_maybe(v, c.ret_depth))
                } else {
                    Ok(v)
                }
            }
            Err(Abort::Ret(v)) => {
                if c.ret_depth > 0 {
                    Ok(opt_lift_maybe(v, c.ret_depth))
                } else {
                    Ok(v)
                }
            }
            // **VD-263**: thân closure dạng **biểu thức** (không phải suite) cũng không cho
            // `break`/`continue` xuyên biên hàm — trap thật thay vì để vòng lặp của người gọi
            // bắt nhầm (VD-259).
            Err(t) => match Self::tach_ctrl(t) {
                Ok(_) => Err(Self::trap_nhay(span)),
                Err(t) => Err(t),
            },
        }
    }

    pub(crate) fn call_hfn(
        &mut self,
        f: &'a HFn,
        pos: Vec<Value>,
        named: Vec<(String, Value)>,
        span: Span,
    ) -> R<Value> {
        self.depth += 1;
        if self.depth > MAX_CALL_DEPTH {
            self.depth -= 1;
            return Err(Abort::Trap(Trap::new(
                RT,
                format!(
                    "gọi hàm quá sâu (>{} khung) — đệ quy không có điểm dừng?",
                    MAX_CALL_DEPTH
                ),
                span,
            )));
        }
        self.fn_depth += 1;
        let kq = self.call_hfn_trong(f, pos, named, span);
        self.fn_depth -= 1;
        self.depth -= 1;
        kq
    }

    /// **ST-319e (#210)** — hàm tên **luôn** chạy trên env module: closure đang gọi chỉ là
    /// caller, không truyền snapshot của nó sang thân hàm. Ở độ sâu closure `0`, `self.env`
    /// **chính là** env module (non-frame: env cấp tệp; frame: `globals`) nên chỉ đồng bộ
    /// handle; ở độ sâu `> 0`, tạm đổi `env` sang `env_module` và khôi phục trên **mọi** nhánh
    /// thoát (`Ok`, `Abort::Ret`, trap/abort).
    fn call_hfn_trong(
        &mut self,
        f: &'a HFn,
        pos: Vec<Value>,
        named: Vec<(String, Value)>,
        span: Span,
    ) -> R<Value> {
        let saved_env = if self.closure_dong > 0 {
            Some(std::mem::replace(&mut self.env, self.env_module.clone()))
        } else {
            self.env_module = self.env.clone();
            None
        };
        let kq = self.call_hfn_than(f, pos, named, span);
        if let Some(env) = saved_env {
            self.env = env;
        }
        kq
    }

    /// **ST-323d (#214)** — ghi tham số vào env module, ghi giá trị cũ vào `old`.
    ///
    /// Tách riêng để vòng restore được gọi **ngay cả khi** `self.eval(d)?` (default) hoặc
    /// nhánh "thiếu đối số" trả `Err` giữa chừng — trước bản vá các nhánh đó nhảy thẳng qua
    /// vòng restore của [`Self::call_hfn_than`] và để tham số lọt lại env module.
    fn gan_tham_so(
        &mut self,
        f: &'a HFn,
        pos: &[Value],
        named: &[(String, Value)],
        span: Span,
        old: &mut Vec<(u32, Option<Value>)>,
    ) -> R<()> {
        // ST-27: hàm có `...rest` nhận **thêm** đối số vị trí.
        let bien_di = f.params.last().map(|p| p.variadic).unwrap_or(false);
        if !bien_di && pos.len() + named.len() > f.params.len() {
            return Err(Abort::Trap(Trap::new(
                RT,
                format!(
                    "hàm `{}` nhận {} đối số, được {}",
                    f.name,
                    f.params.len(),
                    pos.len() + named.len()
                ),
                span,
            )));
        }
        for (i, p) in f.params.iter().enumerate() {
            old.push((p.sym, self.env.borrow().get(&p.sym).cloned()));
            // ST-27: `...rest` gom **mọi** đối số vị trí còn lại thành `list[str]`.
            if p.variadic {
                let con: Vec<Value> = pos.iter().skip(i).cloned().collect();
                self.env.borrow_mut().insert(p.sym, Value::list(con));
                continue;
            }
            let v = if let Some(v) = pos.get(i) {
                Some(v.clone())
            } else {
                named
                    .iter()
                    .find(|(n, _)| *n == p.name)
                    .map(|(_, v)| v.clone())
            };
            let v = match v {
                Some(v) => v,
                None => match &p.default {
                    Some(d) => self.eval(d)?,
                    None => {
                        return Err(Abort::Trap(Trap::new(
                            RT,
                            format!("thiếu đối số `{}` của `{}`", p.name, f.name),
                            span,
                        )))
                    }
                },
            };
            // **VD-228b**: tham số annot `T?` — nâng giá trị plain ở biên hàm (giống `let` + `-> T?`).
            let v = match &p.ty {
                Type::Opt(_) => opt_lift_maybe(v, opt_depth_ty(&p.ty)),
                _ => v,
            };
            self.env.borrow_mut().insert(p.sym, v);
            if p.is_var_self {
                self.mutable.insert(p.sym);
            }
        }
        Ok(())
    }

    /// **ST-323d (#214)** — đưa env module về trạng thái trước lời gọi (ô nào cũ thì khôi
    /// phục, ô nào chưa có thì xoá). Gọi ở **mọi** nhánh thoát của `call_hfn_than`.
    fn khoi_phuc_tham_so(&mut self, old: &[(u32, Option<Value>)]) {
        for (p, v) in old {
            match v {
                Some(v) => {
                    self.env.borrow_mut().insert(*p, v.clone());
                }
                None => {
                    self.env.borrow_mut().remove(p);
                }
            }
        }
    }

    /// Thân [`Self::call_hfn_trong`] — chạy trên env **đã chọn** (env module, #210).
    fn call_hfn_than(
        &mut self,
        f: &'a HFn,
        pos: Vec<Value>,
        named: Vec<(String, Value)>,
        span: Span,
    ) -> R<Value> {
        let mut old = Vec::with_capacity(f.params.len());
        // **ST-323d (#214)**: chỉ restore **trước** thân khi setup lỗi (default trap/Ret,
        // thiếu đối số, arity). Nếu setup Ok thì thân hàm cần tham số — restore sau `exec_body`.
        if let Err(e) = self.gan_tham_so(f, &pos, &named, span, &mut old) {
            self.khoi_phuc_tham_so(&old);
            return Err(e);
        }
        let r = self.exec_body(&f.body);
        self.khoi_phuc_tham_so(&old);
        match r {
            Ok(v) => match &f.ret {
                Type::Opt(_) => Ok(opt_lift_maybe(v, opt_depth_ty(&f.ret))),
                _ => Ok(v),
            },
            Err(Abort::Ret(v)) => match &f.ret {
                Type::Opt(_) => Ok(opt_lift_maybe(v, opt_depth_ty(&f.ret))),
                _ => Ok(v),
            },
            Err(t) => Err(t),
        }
    }
}

/// **ST-317b** — thông điệp arity **một nguồn** cho interp + frame evaluator.
pub(crate) fn loi_arity(ten: &str, can: usize, got: usize, span: Span) -> Trap {
    Trap::new(
        RT,
        format!(
            "hàm `{}` cần {} đối số, được {}",
            if ten.is_empty() { "<fn>" } else { ten },
            can,
            got
        ),
        span,
    )
}

/// **ST-317b** — thông điệp "không phải hàm" **một nguồn** cho interp + frame evaluator.
pub(crate) fn loi_khong_phai_ham(v: &Value, span: Span) -> Trap {
    Trap::new(RT, format!("`{}` không phải hàm", v.show()), span)
}

#[cfg(test)]
mod tests_st323d {
    use super::*;

    fn ha(src: &str) -> vis_types::HirOutput {
        let src = format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}");
        let lex = vis_syntax::lex(&src, vis_syntax::Mode::Script);
        let out = vis_syntax::parse(&src, vis_syntax::Mode::Script);
        vis_types::lower(&out.cst, &lex)
    }

    /// `sym` của tham số đầu (`a`) của hàm tên `h` — ô bị rò nếu restore bị bỏ qua.
    fn sym_a(h: &vis_types::HirOutput) -> u32 {
        h.hir
            .items
            .iter()
            .find_map(|it| match it {
                HItem::Fn(f) if f.name == "h" => {
                    f.params.iter().find(|p| p.name == "a").map(|p| p.sym)
                }
                _ => None,
            })
            .expect("hàm h(a, …)")
    }

    fn co_o(it: &Interp<'_>, sym: u32) -> bool {
        it.env.borrow().contains_key(&sym)
    }

    /// **#214 / trap sớm**: default `1 / 0` ⇒ `Abort::Trap` ngay trong vòng gán tham số,
    /// trước vòng restore. Tham số đã ghi (`a=7`) không được lọt lại env module.
    #[test]
    fn st323d_khong_ro_tham_so_khi_trap_default() {
        let h = ha("fn h(a: int, --n: float = 1 / 0) -> float:\n    1.0\n\nfn main() -> float:\n    h(7)\n");
        let a = sym_a(&h);
        let mut it = Interp::new(&h.hir, &h.symbols, &h.types);
        assert!(!co_o(&it, a), "trước chạy: ô tham số chưa có");
        assert!(
            matches!(it.run_program(&[]), Err(Abort::Trap(_))),
            "default `1 / 0` phải trap VS0502"
        );
        assert!(
            !co_o(&it, a),
            "#214: ô `a` của h vẫn còn trong env module sau trap sớm"
        );
    }

    /// **#214 / `Ret` sớm**: `None?` trong default ⇒ `Abort::Ret(None)` thoát hàm tên trước
    /// vòng restore; closure bắt `Ret` và trả `None` như thường, nhưng env module phải sạch.
    #[test]
    fn st323d_khong_ro_tham_so_khi_ret_default() {
        let h = ha("fn boom() -> int?:\n    None\n\nfn h(a: int, --n: int = boom()?) -> int:\n    a\n\nfn main():\n    let f = fn() -> int: h(7)\n    print(f())\n");
        let a = sym_a(&h);
        let mut it = Interp::new(&h.hir, &h.symbols, &h.types);
        assert!(!co_o(&it, a), "trước chạy: ô tham số chưa có");
        assert!(
            it.run_program(&[]).is_ok(),
            "`?` trong default trả sớm `None`, không trap"
        );
        assert_eq!(it.take_stdout(), "None\n");
        assert!(
            !co_o(&it, a),
            "#214: ô `a` của h vẫn còn trong env module sau `Ret` sớm"
        );
    }
}
