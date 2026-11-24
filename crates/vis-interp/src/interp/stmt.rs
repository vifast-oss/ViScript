// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310a (VD-285)** — thi hành câu lệnh (`exec_stmt`), tách khỏi `interp.rs`.
//! Chuyển **nguyên văn**, không đổi ngữ nghĩa.

use super::*;
impl<'a> Interp<'a> {
    pub(crate) fn exec_stmt(&mut self, s: &HStmt) -> R<Ctrl> {
        self.steps += 1;
        if let Some(a) = self.worker_tick(s.span()) {
            return Err(a);
        }
        if let Some(a) = self.charge(crate::rt::fee::FeeKind::Stmt, s.span()) {
            return Err(a);
        }
        match s {
            HStmt::Let {
                pat,
                ty,
                init,
                else_,
                span,
            } => {
                let v = self.eval(init)?;
                if let Some(a) = self.charge(crate::rt::fee::FeeKind::Assign, *span) {
                    return Err(a);
                }
                // **VD-226 §TEST (A0-proxy)**: tự nâng `T → T?` ở vế phải `let` khi chú thích
                // là `T?` — đồng nhất với codegen (codegen.rs) và spec §4.4 ("tự nâng ở ba vị trí:
                // gán let x: int? = …, trả về → int? …, đối số"). `Some(_)`/`None` giữ nguyên,
                // `Result` giữ nguyên; khác đi thì bọc `Value::some`.
                // **VD-241 §3 (A0, 2026-11-19)** — tự nâng `T → T? → …` tới đủ lớp của annot
                // (một lớp/biên chuyển đổi; không bọc thêm giá trị đã Option/Result).
                let v = if ty_là_opt(ty.as_ref()) {
                    let target = ty.as_ref().map(opt_depth_ty).unwrap_or(0);
                    opt_lift_maybe(v, target)
                } else {
                    v
                };
                // **R3b (VD-178)**: `let x = r else:` — bóc `Ok`/`Some`; ngược lại chạy khối `else`.
                // **VD-260**: pattern **không phải** tên trần (`Some(a)`, `Ok(v)`, …) khớp trên
                // **kiểu đầy đủ** của giá trị; chỉ tên trần mới bóc một lớp như R3b.
                // **VD-263** — cùng luật với frame VM/codegen:
                // - tên trần: `Ok`/`Some` khớp (bind payload), còn lại là nhánh lỗi;
                // - pattern bác được: khớp toàn bộ giá trị; thất bại thì hoàn nguyên binding
                //   một phần trước khi chạy `else`;
                // - binder `else e`: tên trần/`Ok(...)` ⇒ payload `Err`; pattern khác ⇒ `()`.
                if let Some(e) = else_ {
                    let matched = if matches!(pat, HPat::Bind { .. }) {
                        match &v {
                            Value::Enum(ea, va, ag)
                                if ((ea == "Result" && va == "Ok")
                                    || (ea == "Option" && va == "Some"))
                                    && ag.len() == 1 =>
                            {
                                let payload = ag[0].clone();
                                self.bind_pat(pat, &payload);
                                true
                            }
                            _ => false,
                        }
                    } else {
                        let snap = self.env.borrow().clone();
                        if self.bind_pat_bool(pat, &v) {
                            true
                        } else {
                            *self.env.borrow_mut() = snap;
                            false
                        }
                    };
                    if !matched {
                        if let Some(sym) = e.bind_sym {
                            let binder = crate::value::let_else_binder(pat, &v);
                            self.env.borrow_mut().insert(sym, binder);
                        }
                        return self.exec_block(&e.body);
                    }
                } else {
                    self.bind_pat(pat, &v);
                }
                Ok(Ctrl::Normal)
            }
            HStmt::Var {
                sym,
                init,
                ty,
                span,
                ..
            } => {
                let v = self.eval(init)?;
                if let Some(a) = self.charge(crate::rt::fee::FeeKind::Assign, *span) {
                    return Err(a);
                }
                // **VD-241 §3+ (A0, 2026-11-19)** — biên **khai `var`** có annot Optional nâng
                // giá trị như `let` (mirror codegen `HStmt::Var`): `var x: int?? = Some(7)` ⇒ `x`
                // bằng `Some(Some(7))`. `None` trần giữ nguyên (bước 1 — `lift_opt`).
                let v = if ty_là_opt(ty.as_ref()) {
                    let target_d = ty.as_ref().map(opt_depth_ty).unwrap_or(0);
                    opt_lift_maybe(v, target_d)
                } else {
                    v
                };
                self.env.borrow_mut().insert(*sym, v);
                self.mutable.insert(*sym);
                Ok(Ctrl::Normal)
            }
            HStmt::Assign {
                target,
                op,
                value,
                span,
            } => {
                // **ST-56b (VD-97)**: gán **trường/phần tử** của biến `var` — **cùng** ngữ nghĩa
                // codegen (xem `gan_truong`/`gan_chi_so`): thứ tự đánh giá **giống** codegen
                // (nạp ô → chỉ số → giá trị) và kết quả **ghi lại ô** (giá trị, không aliasing).
                // Chỉ `=`; gán ghép trên trường/phần tử đã bị tyck chặn trước.
                if matches!(op, AssignOp::Set) {
                    match target {
                        HExpr::Field { base, name, .. } => {
                            if let HExpr::Var { sym: Some(id), .. } = &**base {
                                let cu = self.doc_o(*id, target.span())?;
                                let dep = self.assign_depth(target, &cu); // ST-248: đích trường Opt
                                let rhs = crate::lift::lift_opt(self.eval(value)?, dep);
                                let moi = gan_truong(cu, name, rhs, *span)?;
                                if let Some(a) = self.charge(crate::rt::fee::FeeKind::Assign, *span)
                                {
                                    return Err(a);
                                }
                                self.ghi_o(*id, moi, *span)?;
                                return Ok(Ctrl::Normal);
                            }
                        }
                        HExpr::Index { base, index, .. } => {
                            if let HExpr::Var { sym: Some(id), .. } = &**base {
                                let cu = self.doc_o(*id, target.span())?;
                                let dep = self.assign_depth(target, &cu); // ST-248: đích phần tử Opt
                                let i = self.eval(index)?;
                                let rhs = crate::lift::lift_opt(self.eval(value)?, dep);
                                let moi = gan_chi_so(cu, i, rhs, *span)?;
                                if let Some(a) = self.charge(crate::rt::fee::FeeKind::Assign, *span)
                                {
                                    return Err(a);
                                }
                                self.ghi_o(*id, moi, *span)?;
                                return Ok(Ctrl::Normal);
                            }
                            // **ST-60b**: `p.xs[0] = v` (một tầng trường, gốc `var`) — đọc trường,
                            // gán phần tử, rồi **ghi lại trường** rồi ghi lại biến: đúng chuỗi
                            // `p = p with xs: (p.xs[i] = v)` mà codegen phát (`Field`/`Set`/`With`).
                            if let HExpr::Field {
                                base: goc, name, ..
                            } = &**base
                            {
                                if let HExpr::Var { sym: Some(id), .. } = &**goc {
                                    let cu = self.doc_o(*id, target.span())?;
                                    let dep = self.assign_depth(target, &cu); // ST-248: `x.f[i]`
                                    let truong = self.field_of(cu, name, *span)?;
                                    let i = self.eval(index)?;
                                    let rhs = crate::lift::lift_opt(self.eval(value)?, dep);
                                    let moi_truong = gan_chi_so(truong, i, rhs, *span)?;
                                    let cu2 = self.doc_o(*id, target.span())?;
                                    let moi = gan_truong(cu2, name, moi_truong, *span)?;
                                    if let Some(a) =
                                        self.charge(crate::rt::fee::FeeKind::Assign, *span)
                                    {
                                        return Err(a);
                                    }
                                    self.ghi_o(*id, moi, *span)?;
                                    return Ok(Ctrl::Normal);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                // **VD-268**: đích **đường** (`a.b.f`, `xs[0].f`, `a.bs[i].f`, …) hoặc gán ghép
                // trên **lá** trường/phần tử. Ba hình dạng một tầng ở trên giữ nguyên đường cũ.
                if !matches!(target, HExpr::Var { .. }) {
                    if let Some(id) = gan_long::goc_var(target) {
                        self.gan_long(id, target, *op, value, *span)?;
                        return Ok(Ctrl::Normal);
                    }
                }
                let cur = self.lvalue_get(target)?;
                let rhs = self.eval(value)?;
                // **VD-241 §3+ (A0, 2026-11-19)** — biên **gán lại `=`** nâng một lớp khi đích là
                // biến `var` có annot Optional (`x = Some(7)` với `x: int??` ⇒ `x` bằng
                // `Some(Some(7))`). Đích `let` không gán được (tyck chặn trước). `None` trần giữ
                // nguyên (bước 1). **Chưa** nâng ở đích trường/chỉ số (`x.f = …`, `x[i] = …`) —
                // xem báo cáo.
                let rhs = match op {
                    AssignOp::Set => match target {
                        HExpr::Var { sym: Some(id), .. } => {
                            // **VD-260**: kiểu đích là kiểu **suy ra** của biến (annot hoặc từ
                            // initializer) — `var x = Some(5); x = 7` ⇒ `Some(7)` như VM.
                            let want = self
                                .sym_ty
                                .get(id)
                                .filter(|t| matches!(t, vis_hir::Type::Opt(_)))
                                .cloned();
                            match want {
                                Some(t) => opt_lift_maybe(rhs, opt_depth_ty(&t)),
                                None => rhs,
                            }
                        }
                        _ => rhs,
                    },
                    _ => rhs,
                };
                let new = match op {
                    AssignOp::Set => rhs,
                    _ => {
                        let bop = match op {
                            AssignOp::Add => BinOp::Add,
                            AssignOp::Sub => BinOp::Sub,
                            AssignOp::Mul => BinOp::Mul,
                            AssignOp::Div => BinOp::Div,
                            // **R7 (VD-178)**: `//=` — chia lấy phần nguyên.
                            AssignOp::FloorDiv => BinOp::FloorDiv,
                            _ => BinOp::Rem,
                        };
                        crate::builtin::bin(bop, cur, rhs, *span)?
                    }
                };
                if let Some(a) = self.charge(crate::rt::fee::FeeKind::Assign, *span) {
                    return Err(a);
                }
                self.lvalue_set(target, new, *span)?;
                Ok(Ctrl::Normal)
            }
            HStmt::Expr(e) => {
                let v = self.eval(e)?;
                self.last = Some(v);
                Ok(Ctrl::Normal)
            }
            HStmt::If {
                cond,
                then,
                elifs,
                else_,
                ..
            } => {
                // VD-47: `if` **có** `else` cho giá trị của nhánh đã chạy; **không** `else` ⇒ `()`.
                let has_else = matches!(else_, Some(b) if !b.is_empty());
                let (min, max) = core_types::goi::do_if(
                    then,
                    elifs,
                    else_.as_deref(),
                    &self.sym_ty,
                    self.types,
                    &self.fn_rets,
                    &self.mret,
                );
                let mut ctrl = Ctrl::Normal;
                let mut ran = false;
                if self.eval_cond(cond)? {
                    ctrl = self.exec_block(then)?;
                    ran = true;
                }
                if !ran {
                    for (c, body) in elifs {
                        if self.eval_cond(c)? {
                            ctrl = self.exec_block(body)?;
                            ran = true;
                            break;
                        }
                    }
                }
                if !ran {
                    if let Some(b) = else_ {
                        if !b.is_empty() {
                            ctrl = self.exec_block(b)?;
                        }
                    }
                }
                if !has_else {
                    self.last = None;
                } else if let Some(v) = self.last.take() {
                    self.last = Some(core_types::goi::boc_nhanh(v, min, max));
                }
                Ok(ctrl)
            }
            HStmt::For {
                pat, iter, body, ..
            } => {
                let it = self.eval(iter)?;
                let items = self.iter_items(it, iter.span())?;
                self.loop_depth += 1;
                let ctrl = self.exec_for(pat, items, body, s.span());
                self.loop_depth -= 1;
                self.last = None;
                ctrl
            }
            HStmt::While { cond, body, .. } => {
                let mut guard = 0usize;
                self.loop_depth += 1;
                // **ST-226 (R18b)**: `while let` dùng chung bộ máy `eval_cond` với `if let`.
                loop {
                    if let Some(a) = self.charge(crate::rt::fee::FeeKind::Loop, s.span()) {
                        self.loop_depth -= 1;
                        return Err(a);
                    }
                    if !self.eval_cond(cond)? {
                        break;
                    }
                    guard += 1;
                    if guard > crate::rt::fee::MAX_LOOP_ITERATIONS as usize {
                        return Err(Abort::Trap(Trap::new(
                            RT,
                            "vòng lặp quá lâu (>10^7 bước)",
                            s.span(),
                        )));
                    }
                    let c = self.exec_block(body)?;
                    if c == Ctrl::Break {
                        break;
                    }
                }
                self.loop_depth -= 1;
                self.last = None;
                Ok(Ctrl::Normal)
            }
            HStmt::Loop { body, .. } => {
                self.loop_depth += 1;
                for _ in 0..crate::rt::fee::MAX_LOOP_ITERATIONS as usize {
                    if let Some(a) = self.charge(crate::rt::fee::FeeKind::Loop, s.span()) {
                        self.loop_depth -= 1;
                        return Err(a);
                    }
                    if self.exec_block(body)? == Ctrl::Break {
                        self.loop_depth -= 1;
                        self.last = None;
                        return Ok(Ctrl::Normal);
                    }
                }
                self.loop_depth -= 1;
                Err(Abort::Trap(Trap::new(
                    RT,
                    "`loop` quá lâu (>10^7 bước)",
                    s.span(),
                )))
            }
            HStmt::Scope { body, .. } => self.exec_block(body),
            HStmt::Return { value, .. } => {
                let v = match value {
                    Some(e) => self.eval(e)?,
                    None => Value::Unit,
                };
                Err(Abort::Ret(v))
            }
            HStmt::Break { .. } => {
                if self.loop_depth == 0 {
                    return Err(Abort::Trap(Trap::new(
                        RT,
                        "`break` ngoài vòng lặp (chỉ dùng trong `for`/`while`/`loop`)",
                        s.span(),
                    )));
                }
                Ok(Ctrl::Break)
            }
            HStmt::Continue { .. } => {
                if self.loop_depth == 0 {
                    return Err(Abort::Trap(Trap::new(
                        RT,
                        "`continue` ngoài vòng lặp",
                        s.span(),
                    )));
                }
                Ok(Ctrl::Continue)
            }
            HStmt::Match { subject, arms, .. } => {
                let (min, max) = core_types::goi::do_match(
                    arms,
                    &self.sym_ty,
                    self.types,
                    &self.fn_rets,
                    &self.mret,
                );
                let subj = self.eval(subject)?;
                let Some((arm, binds)) = self.pick_arm(arms, &subj)? else {
                    return Err(Abort::Trap(Trap::new(
                        RT,
                        "`match` không khớp nhánh nào (không xét hết bị lọt)",
                        s.span(),
                    )));
                };
                for (sym, v) in binds {
                    self.env.borrow_mut().insert(sym, v);
                }
                let r = self.exec_block(&arm.body)?;
                if let Some(v) = self.last.take() {
                    self.last = Some(core_types::goi::boc_nhanh(v, min, max));
                }
                Ok(r)
            }
            HStmt::Block { body, .. } => self.exec_block(body),
        }
    }
}
